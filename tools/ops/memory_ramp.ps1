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

foreach ($n in $Steps) {
    $log = Join-Path $ops "out/mem_${Tag}_$n.log"
    $env:RUST_LOG = 'crystal_server=info'
    $proc = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $ops "out/mem_${Tag}_$n.err.log") -PassThru
    $ready = $false
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
    }
    if (-not $ready) { $rows += [pscustomobject]@{ sessions = $n; ready = $false }; Stop-Process -Id $proc.Id -Force -EA SilentlyContinue; continue }
    # 进图前（无会话）——注意：地图是懒加载的，这里通常还没加载地图
    # 注意：Process 对象会缓存属性值，读数前必须 Refresh，否则整轮阶梯都读到同一个陈旧值
    $proc.Refresh()
    $rssIdle = [Math]::Round($proc.WorkingSet64 / 1MB, 1)

    $acc = (1..$n | ForEach-Object { "$AccountPrefix$_" }) -join ','
    $json = Join-Path $ops "out/mem_${Tag}_bot_$n.json"
    $job = Start-Job -ScriptBlock {
        param($ops, $acc, $n, $Port, $HoldSec, $json)
        & python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --accounts $acc `
            --sessions $n --hold $HoldSec --password 123456 > $json 2>&1
    } -ArgumentList $ops, $acc, $n, $Port, $HoldSec, $json
    $samples = @()
    $deadline = (Get-Date).AddSeconds($HoldSec + 15)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep 2
        $proc.Refresh()
        if (-not $proc.HasExited) { $samples += [Math]::Round($proc.WorkingSet64 / 1MB, 1) }
        if ((Get-Job -Id $job.Id).State -eq 'Completed' -and $samples.Count -ge 3) { break }
    }
    Wait-Job $job | Out-Null
    Remove-Job $job -Force
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
