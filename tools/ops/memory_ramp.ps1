# memory_ramp.ps1 — 内存阶梯（CAPACITY.md §4「7.5MB/会话」拆解的测量工具）
#
# 与 capacity_ramp.ps1 的区别：**只按 PID 管控自己启动的服务端**（不按进程名 Stop-Process），
# 因此可以在同一台机器上并行保留其它服务端实例（例如开发用 7000 端口那个）。
# 采样：每 2s 采一次 RSS，取保持期内的**最大值**（避免抓到瞬时低谷）。
#
# 用法（A/B 对比同一份代码的两种状态时，两次分别用不同 -ExePath / -Tag 跑）：
#   pwsh tools/ops/memory_ramp.ps1 -DeployDir <deploy> -ExePath <exe> -StepsCsv 10,20 `
#        -Tag before -OutFile tools/ops/out/memory_before.json
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Port = 7100,
    [string]$StepsCsv = '10,20',
    [string]$AccountPrefix = 'opsload',
    [int]$HoldSec = 20,
    [string]$Tag = 'run',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$Steps = @($StepsCsv.Split(',') | ForEach-Object { [int]$_.Trim() } | Where-Object { $_ -gt 0 })
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$rows = @()

# 日志目录必须先存在：`Start-Process -RedirectStandardOutput` 指向不存在的目录会直接启动失败，
# 而失败只表现为一屏 PowerShell 报错 + 每档 ready=false（2026-09-24 实测踩到）
New-Item -ItemType Directory -Force -Path (Join-Path $ops 'out') | Out-Null

# 端口单一真源：把部署副本的 gate 端口对齐到 -Port。
# 不对齐就会出现「服务端去抢 7000（开发机在用时 bind 失败）→ 日志里没有 'Gate listening'
# → 本脚本记 ready=false」这种**静默作废**的标定。
$cfgPath = Join-Path $DeployDir 'config/server.toml'
if (-not (Test-Path $cfgPath)) {
    Write-Host "FATAL: 部署副本缺少 $cfgPath，无法对齐 gate 端口（拒绝产出无效标定）"
    exit 2
}
$cfgText = Get-Content $cfgPath -Raw
$aligned = [regex]::Replace($cfgText, 'listen_addr\s*=\s*"[^"]*"', "listen_addr = `"0.0.0.0:$Port`"")
if ($aligned -ne $cfgText) { Set-Content -Path $cfgPath -Value $aligned -NoNewline -Encoding utf8 }

foreach ($n in $Steps) {
    $log = Join-Path $ops "out/mem_${Tag}_$n.log"
    $env:RUST_LOG = 'crystal_server=info'
    $proc = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $ops "out/mem_${Tag}_$n.err.log") -PassThru -WindowStyle Hidden
    $ready = $false
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
    }
    if (-not $ready) {
        Write-Host "FATAL: 服务端 90s 内未就绪（未见 'Gate listening'）——本轮标定作废，日志：$log"
        if ($proc) { Stop-Process -Id $proc.Id -Force -EA SilentlyContinue }
        exit 2
    }
    # 进图前（无会话）——注意：地图是懒加载的，这里通常还没加载地图
    # 注意：Process 对象会缓存属性值，读数前必须 Refresh，否则整轮阶梯都读到同一个陈旧值
    $proc.Refresh()
    $rssIdle = [Math]::Round($proc.WorkingSet64 / 1MB, 1)

    $acc = (1..$n | ForEach-Object { "$AccountPrefix$_" }) -join ','
    $json = Join-Path $ops "out/mem_${Tag}_bot_$n.json"
    $job = Start-Job -ScriptBlock {
        param($ops, $acc, $n, $Port, $HoldSec, $json)
        # job 的 runspace 不继承父作用域函数 ⇒ 内部自己 dot-source helper（父线程 Wait-Job -Timeout 是第二层）
        . (Join-Path $ops '_run_bot.ps1')
        $r = Invoke-BotJson -OpsDir $ops -BotArgs @('--host', '127.0.0.1', '--port', "$Port", '--accounts', $acc,
            '--sessions', "$n", '--hold', "$HoldSec", '--password', '123456') `
            -TimeoutSec ($HoldSec + 90) -Tag 'memory_ramp'
        if ($r.json) { $r.json | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $json }
    } -ArgumentList $ops, $acc, $n, $Port, $HoldSec, $json
    $samples = @()
    $deadline = (Get-Date).AddSeconds($HoldSec + 15)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep 2
        $proc.Refresh()
        if (-not $proc.HasExited) { $samples += [Math]::Round($proc.WorkingSet64 / 1MB, 1) }
        if ((Get-Job -Id $job.Id).State -eq 'Completed' -and $samples.Count -ge 3) { break }
    }
    # 2026-09-25：Wait-Job 原先没有超时——bot 卡住就整轮永久等下去（同类"静默挂死"）
    if (-not (Wait-Job $job -Timeout ($HoldSec + 120))) {
        Write-Host ("WARN: 内存阶梯 bot 超过 {0}s 未结束（port={1}）——终止该轮采样" -f ($HoldSec + 120), $Port)
        Stop-Job $job -ErrorAction SilentlyContinue
    }
    Remove-Job $job -Force -ErrorAction SilentlyContinue
    $rssLoaded = if ($samples.Count) { ($samples | Measure-Object -Maximum).Maximum } else { 0 }
    $bot = $null
    try { $bot = (Get-Content $json -Raw | ConvertFrom-Json) } catch {}
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    Start-Sleep 2
    $rows += [pscustomobject]@{
        sessions = $n
        ready = $true
        ok = if ($bot) { $bot.summary.ok } else { 0 }
        failed = if ($bot) { $bot.summary.failed } else { $n }
        rss_idle_mb = $rssIdle
        rss_loaded_mb = $rssLoaded
        rss_per_session_mb = [Math]::Round(($rssLoaded - $rssIdle) / [Math]::Max(1, $n), 2)
        samples = $samples.Count
    }
}

$report = [ordered]@{
    tag = $Tag
    exe = $ExePath
    steps = $Steps
    hold_sec = $HoldSec
    step_table = $rows
}
$jsonOut = $report | ConvertTo-Json -Depth 6
if ($OutFile) { New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutFile) | Out-Null; $jsonOut | Set-Content -Encoding utf8 $OutFile }
Write-Host $jsonOut
