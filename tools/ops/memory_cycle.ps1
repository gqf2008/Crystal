# memory_cycle.ps1 — **单进程多轮**内存标定（CAPACITY.md §4.5 「release 复测 + 双轮 cycle」用）
#
# 与 memory_ramp.ps1 的差别：memory_ramp 每个阶梯都重启服务端（只能测「idle → 载入」
# 的每会话增量）；本工具**一个进程跑完所有轮**，于是多出两类此前测不到的量：
#   ① 每轮「全部登出后」的 RSS —— 用来判「登出后不归还」是否真的成立；
#   ② 同会话数相邻两轮的峰值差（cycle2 − cycle1）—— §4.1b/§4.3 的 13MB/轮 结论要用它复核。
# 同时按 PID 管控自己启动的实例（不按进程名 Stop-Process），可与其他服务端实例共存。
#
# 计算口径与既有工具一致：每 2s 采样、取保持期**最大值**（Process 必须 Refresh，否则读到陈旧值）。
#
# 用法：
#   pwsh tools/ops/memory_cycle.ps1 -DeployDir <deploy> -ExePath <release exe> `
#        -RoundsCsv '10,20,20' -HoldSec 20 -Tag release-post3062 `
#        -OutFile tools/ops/out/memory_cycle_release.json
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Port = 7300,
    [string]$RoundsCsv = '10,20,20',
    [string]$AccountPrefix = 'opsload',
    [int]$HoldSec = 20,
    [int]$IdleSec = 25,
    [string]$Tag = 'cycle',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$Steps = @($RoundsCsv.Split(',') | ForEach-Object { [int]$_.Trim() } | Where-Object { $_ -gt 0 })
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
New-Item -ItemType Directory -Force -Path (Join-Path $ops 'out') | Out-Null

# 端口单一真源：把部署副本的 gate 端口对齐到 -Port（不对齐会静默去抢 7000 → 绑定失败 → ready=false）
$cfgPath = Join-Path $DeployDir 'config/server.toml'
if (-not (Test-Path $cfgPath)) {
    Write-Host "FATAL: 部署副本缺少 $cfgPath，无法对齐 gate 端口（拒绝产出无效标定）"
    exit 2
}
$cfgText = Get-Content $cfgPath -Raw
$aligned = [regex]::Replace($cfgText, 'listen_addr\s*=\s*"[^"]*"', "listen_addr = `"0.0.0.0:$Port`"")
if ($aligned -ne $cfgText) { Set-Content -Path $cfgPath -Value $aligned -NoNewline -Encoding utf8 }

$log = Join-Path $ops "out/memcycle_${Tag}.log"
$env:RUST_LOG = 'crystal_server=info'
$proc = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $ops "out/memcycle_${Tag}.err.log") `
    -PassThru -WindowStyle Hidden

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
$proc.Refresh()
$rssIdle = [Math]::Round($proc.WorkingSet64 / 1MB, 1)

$rows = @()
$round = 0
foreach ($n in $Steps) {
    $round++
    $acc = (1..$n | ForEach-Object { "$AccountPrefix$_" }) -join ','
    $json = Join-Path $ops "out/memcycle_${Tag}_r${round}_$n.json"
    $job = Start-Job -ScriptBlock {
        param($ops, $acc, $n, $Port, $HoldSec, $json)
        # job 的 runspace 不继承父作用域函数 ⇒ 内部自己 dot-source helper（父线程 Wait-Job -Timeout 是第二层）
        . (Join-Path $ops '_run_bot.ps1')
        $r = Invoke-BotJson -OpsDir $ops -BotArgs @('--host', '127.0.0.1', '--port', "$Port", '--accounts', $acc,
            '--sessions', "$n", '--hold', "$HoldSec", '--password', '123456') `
            -TimeoutSec ($HoldSec + 90) -Tag 'memory_cycle'
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
        Write-Host ("WARN: 连登连退 bot 超过 {0}s 未结束（port={1}）——终止该轮采样" -f ($HoldSec + 120), $Port)
        Stop-Job $job -ErrorAction SilentlyContinue
    }
    Remove-Job $job -Force -ErrorAction SilentlyContinue
    $peak = if ($samples.Count) { ($samples | Measure-Object -Maximum).Maximum } else { 0 }
    $bot = $null
    try { $bot = (Get-Content $json -Raw | ConvertFrom-Json) } catch {}

    # 全部登出后的 idle：等 IdleSec 再读（等它把会话与生成物释放完）
    Start-Sleep $IdleSec
    $proc.Refresh()
    $rssAfter = if ($proc.HasExited) { -1 } else { [Math]::Round($proc.WorkingSet64 / 1MB, 1) }

    $rows += [pscustomobject]@{
        round = $round
        sessions = $n
        ok = if ($bot) { $bot.summary.ok } else { 0 }
        failed = if ($bot) { $bot.summary.failed } else { $n }
        rss_loaded_peak_mb = $peak
        rss_after_logout_mb = $rssAfter
        samples = $samples.Count
    }
    Write-Host ("[round {0}] n={1} ok={2} peak={3}MB after_logout={4}MB" -f $round, $n, $rows[-1].ok, $peak, $rssAfter)
}

# 机制计数（与 §4.4/§4.5 的日志口径一致）：整图生成物物化 / 复用 / 清理各几次
$text = @(Get-Content $log -ErrorAction SilentlyContinue)
$spawnLines = @($text | Where-Object { $_ -match 'Spawned \d+ NPCs and \d+ monsters' })
$monsterTotal = 0
foreach ($l in $spawnLines) {
    if ($l -match 'and (\d+) monsters') { $monsterTotal += [int]$Matches[1] }
}
$reuseLines = @($text | Where-Object { $_ -match 'spawns reused' })
$cleanLines = @($text | Where-Object { $_ -match 'spawns cleaned' })

if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -EA SilentlyContinue }

$report = [ordered]@{
    tag = $Tag
    exe = $ExePath
    port = $Port
    hold_sec = $HoldSec
    idle_sec = $IdleSec
    rss_idle_mb = $rssIdle
    rounds = $rows
    spawn_materialize_log_lines = $spawnLines.Count
    spawn_materialize_monsters = $monsterTotal
    spawn_reuse_log_lines = $reuseLines.Count
    spawn_clean_log_lines = $cleanLines.Count
}
$jsonOut = $report | ConvertTo-Json -Depth 6
if ($OutFile) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutFile) | Out-Null
    $jsonOut | Set-Content -Encoding utf8 $OutFile
}
Write-Host $jsonOut
