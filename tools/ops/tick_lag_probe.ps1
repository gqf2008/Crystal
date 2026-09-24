#Requires -Version 5.1
<#
.SYNOPSIS
  高负载 tick 滞后**长窗口**标定：把 CAPACITY.md §6 第 4 项「拉长窗口（≥2 分钟）在 10/20/50 会话
  各测一次心跳滞后」变成一条可复跑的命令。

.DESCRIPTION
  为什么单独做：`capacity_ramp.ps1` 的 `-HoldSec` 默认 20s，而心跳每 `HEARTBEAT_TICKS`(=300 tick,
  100ms/tick ⇒ 30s) 才打一条 —— 20s 窗口内**根本采不到几条心跳**，所以 CAPACITY.md 里
  「20–50 会话下 tick 滞后」一直挂着"待测"。本工具把保持期拉到 ≥2 分钟，让每档拿到 ≥4 条心跳。

  判据（每档独立起服；缺一不可）：
    J0 ready      ：窗口开始前日志出现 `Gate listening`
    J1 载荷成立   ：**这一档必须真的有 N 条会话在场**——`bot.py` 的 `summary.ok == N` **且**
                    心跳里出现 `online >= N`。没有这条，机器人在下面没连上时会得到"零滞后"的假绿。
    J2 样本够     ：窗口内心跳条数 ≥ `MinHeartbeats`（默认 4），否则"A 没劣化"没有证据力
    J3 滞后判据   ：窗口内 `|lag_pct|` 最大值 ≤ `MaxLagPct`（默认 5.0）
    另记录（不作判据）：`interval_ms` 的 min/max、背压计数（`gate mailbox full` / `kicking slow reader`
    / `broadcast deferred to outbox` / `broadcast outbox full`）。

  与 `capacity_ramp.ps1` 的两处刻意不同：
    - **绝不按进程名杀 `mir2_server`**：同机可能有别的 agent 的开发服（7000）与其它 deploy 实例。
      本工具只 `Stop-Process` 它自己启动的那个 PID；
    - 端口从 `<DeployDir>\config\server.toml` 的 `listen_addr` 读，不额外传参（读错端口会静默连不上）。

.PARAMETER DeployDir
  部署副本目录（含 `mir2_server.exe`、`config/server.toml`、`data/crystal.db`）。

.PARAMETER StepsCsv
  会话档位，默认 `10,20,50`（`pwsh -File` 传数组会被拼成字符串，故用 CSV）。

.PARAMETER HoldSec
  每档保持秒数，默认 150（≥2 分钟，够 4~5 条心跳）。

.EXAMPLE
  pwsh tools/ops/tick_lag_probe.ps1 -DeployDir C:\Users\gxh\AppData\Local\Temp\ops_drill_deploy `
      -OutFile tools/ops/out/tick_lag.json
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [string]$ExePath = '',
    [string]$StepsCsv = '10,20,50',
    [int]$HoldSec = 150,
    [string]$AccountPrefix = 'opsload',
    [string]$Password = '123456',
    [double]$MaxLagPct = 5.0,
    [int]$MinHeartbeats = 4,
    # 前置：同机 CPU 必须基本空闲，否则测的是"别人抢 CPU"而不是服务端自己的 tick 能力
    [double]$MaxIdleCpuPct = 20.0,
    [int]$IdleSampleSec = 8,
    [switch]$AllowBusy,
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = Join-Path $ops 'out'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
if (-not $ExePath) { $ExePath = Join-Path $DeployDir 'mir2_server.exe' }

# 端口从部署副本自己的配置读：少一个参数就少一处"读错端口 → 机器人连不上 → 零滞后假绿"
$cfg = Join-Path $DeployDir 'config/server.toml'
$port = 7100
if (Test-Path $cfg) {
    $m = Select-String -Path $cfg -Pattern 'listen_addr\s*=\s*"[^"]*:(\d+)"' | Select-Object -First 1
    if ($m -and $m.Matches.Count -gt 0) { $port = [int]$m.Matches[0].Groups[1].Value }
}
Write-Host ("部署：{0}" -f $DeployDir)
Write-Host ("可执行：{0}" -f $ExePath)
Write-Host ("端口：{0}（来自 config/server.toml）" -f $port)

# ---------------- 前置 J-1：同机必须基本空闲 ----------------
# 为什么必须有：多 agent 共机，别人的 cargo build/压测会让本进程被抢占，测出来的 lag_pct
# 是"主机争用"而不是服务端 tick 能力（本机实测：一次 release 构建并发时 11 个 rustc 在跑）。
# 采 `Win32_PerfFormattedData_PerfOS_Processor` 的 `_Total`（不依赖计数器英文名），取若干次均值。
$cpuLabel = @()
for ($i = 0; $i -lt $IdleSampleSec; $i++) {
    $pct = $null
    try {
        $p = Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -EA Stop |
            Where-Object { $_.Name -eq '_Total' } | Select-Object -First 1
        if ($p) { $pct = [double]$p.PercentProcessorTime }
    } catch {}
    if ($null -ne $pct) { $cpuLabel += $pct }
    Start-Sleep 1
}
$cpuAvg = if ($cpuLabel.Count) { [Math]::Round((($cpuLabel | Measure-Object -Average).Average), 1) } else { $null }
if ($null -eq $cpuAvg) {
    Write-Host '警告：拿不到 CPU 采样（性能计数器不可用），跳过静默前置' -ForegroundColor Yellow
} elseif ($cpuAvg -gt $MaxIdleCpuPct) {
    $busy = @(Get-Process cargo, rustc -EA SilentlyContinue | Select-Object -ExpandProperty ProcessName -Unique)
    Write-Host ("前置失败：{0}s 平均 CPU {1}% > 阈值 {2}%（并发编译：{3}）——此时测 tick 滞后得到的是主机争用，不是服务端能力。" -f `
        $IdleSampleSec, $cpuAvg, $MaxIdleCpuPct, ($(if ($busy) { $busy -join '/' } else { '无 cargo/rustc' }))) -ForegroundColor Red
    Write-Host '等机器空闲后重跑；确要在忙机上测（结果必须标注并发负载）加 -AllowBusy。' -ForegroundColor Yellow
    if (-not $AllowBusy) { exit 2 }
    Write-Host '（-AllowBusy：继续，但结论里必须写明当时 CPU 占用）' -ForegroundColor Yellow
} else {
    Write-Host ("前置 J-1 通过：{0}s 平均 CPU {1}%（阈值 {2}%）" -f $IdleSampleSec, $cpuAvg, $MaxIdleCpuPct) -ForegroundColor Green
}

$steps = @($StepsCsv.Split(',') | ForEach-Object { [int]$_.Trim() } | Where-Object { $_ -gt 0 })
$rows = @()
$serverProc = $null

foreach ($n in $steps) {
    # 上一档的服务端只按**我们自己记录的 PID** 停
    if ($serverProc -and -not $serverProc.HasExited) {
        Stop-Process -Id $serverProc.Id -Force -EA SilentlyContinue
        Start-Sleep 2
    }
    $log = Join-Path $outDir ("ticklag_{0}.log" -f $n)
    $errLog = Join-Path $outDir ("ticklag_{0}.err.log" -f $n)
    Remove-Item -LiteralPath $log, $errLog -Force -EA SilentlyContinue
    $env:RUST_LOG = 'crystal_server=info'
    $serverProc = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $log -RedirectStandardError $errLog
    $ready = $false
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -EA SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
    }
    if (-not $ready) {
        Write-Host ("[{0} 会话] J0 失败：90s 内没看到 Gate listening（端口 {1} 可能被占）" -f $n, $port) -ForegroundColor Red
        $rows += [pscustomobject]@{ sessions = $n; ready = $false; ok = $false }
        continue
    }

    Write-Host ("[{0} 会话] 起压测：{1}s 保持（心跳每 30s 一条）..." -f $n, $HoldSec)
    $acc = (1..$n | ForEach-Object { "$AccountPrefix$_" }) -join ','
    $botJson = Join-Path $outDir ("ticklag_bot_{0}.json" -f $n)
    $bot = Start-Process -FilePath 'python' -PassThru -WindowStyle Hidden -ArgumentList @(
        (Join-Path $ops 'bot.py'), '--host', '127.0.0.1', '--port', "$port", '--accounts', $acc,
        '--sessions', "$n", '--hold', "$HoldSec", '--password', $Password
    ) -RedirectStandardOutput $botJson -RedirectStandardError ($botJson + '.err')
    $null = $bot.WaitForExit(($HoldSec + 120) * 1000)
    if (-not $bot.HasExited) { Stop-Process -Id $bot.Id -Force -EA SilentlyContinue }
    Start-Sleep 2

    $botRes = $null
    try { $botRes = (Get-Content $botJson -Raw | ConvertFrom-Json) } catch {}
    $botOk = if ($botRes) { [int]$botRes.summary.ok } else { 0 }
    $botFail = if ($botRes) { [int]$botRes.summary.failed } else { $n }

    $text = @(Get-Content $log -EA SilentlyContinue)
    $hb = @($text | Where-Object { $_ -match 'heartbeat: tick=' })
    $hbParsed = @()
    foreach ($h in $hb) {
        if ($h -match 'tick=(\d+) online=(\d+) monsters=(\d+) interval_ms=(\d+) lag_pct=(-?[\d.]+)') {
            $hbParsed += [pscustomobject]@{
                tick = [int]$Matches[1]; online = [int]$Matches[2]; monsters = [int]$Matches[3]
                interval_ms = [int]$Matches[4]; lag_pct = [double]$Matches[5]
            }
        }
    }
    $onlineMax = if ($hbParsed.Count) { ($hbParsed | Measure-Object online -Maximum).Maximum } else { 0 }
    $lagMax = if ($hbParsed.Count) { ($hbParsed | ForEach-Object { [Math]::Abs($_.lag_pct) } | Measure-Object -Maximum).Maximum } else { $null }
    $ivMin = if ($hbParsed.Count) { ($hbParsed | Measure-Object interval_ms -Minimum).Minimum } else { $null }
    $ivMax = if ($hbParsed.Count) { ($hbParsed | Measure-Object interval_ms -Maximum).Maximum } else { $null }

    $j1 = ($botOk -eq $n) -and ($onlineMax -ge $n)
    $j2 = ($hbParsed.Count -ge $MinHeartbeats)
    $j3 = ($null -ne $lagMax) -and ($lagMax -le $MaxLagPct)
    $stepOk = $ready -and $j1 -and $j2 -and $j3

    $rows += [pscustomobject]@{
        sessions                = $n
        ready                   = $ready
        bot_ok                  = $botOk
        bot_failed              = $botFail
        heartbeats              = $hbParsed.Count
        online_max              = $onlineMax
        interval_ms_min         = $ivMin
        interval_ms_max         = $ivMax
        lag_pct_max_abs         = $lagMax
        gate_mailbox_full       = @($text | Where-Object { $_ -match 'gate mailbox full' }).Count
        slow_reader_kicks       = @($text | Where-Object { $_ -match 'kicking slow reader' }).Count
        broadcast_deferred      = @($text | Where-Object { $_ -match 'broadcast deferred to outbox' }).Count
        broadcast_outbox_full   = @($text | Where-Object { $_ -match 'broadcast outbox full' }).Count
        j1_load_present         = $j1
        j2_samples_enough       = $j2
        j3_lag_within_limit     = $j3
        ok                      = $stepOk
    }
    $verdict = if ($stepOk) { 'PASS' } else { 'FAIL' }
    Write-Host ("[{0} 会话] {1}：ok={2}/{3} 心跳={4} online_max={5} lag%≤{6} interval=[{7},{8}]ms 丢包={9} 踢线={10}" -f `
        $n, $verdict, $botOk, $n, $hbParsed.Count, $onlineMax, $lagMax, $ivMin, $ivMax, `
        @($text | Where-Object { $_ -match 'gate mailbox full' }).Count, `
        @($text | Where-Object { $_ -match 'kicking slow reader' }).Count) -ForegroundColor $(if ($stepOk) { 'Green' } else { 'Red' })
}

if ($serverProc -and -not $serverProc.HasExited) { Stop-Process -Id $serverProc.Id -Force -EA SilentlyContinue }

$bad = @($rows | Where-Object { -not $_.ok })
$report = [ordered]@{
    ok           = ($bad.Count -eq 0)
    deploy_dir   = $DeployDir
    exe          = $ExePath
    port         = $port
    cpu_avg_pct_before = $cpuAvg
    hold_sec     = $HoldSec
    max_lag_pct  = $MaxLagPct
    min_heartbeats = $MinHeartbeats
    steps        = $rows
    criteria     = 'J0 起服 / J1 载荷成立（bot.ok==N 且心跳 online≥N）/ J2 心跳≥MinHeartbeats / J3 |lag_pct|≤MaxLagPct'
}
$json = $report | ConvertTo-Json -Depth 6
if ($OutFile) { $json | Set-Content -LiteralPath $OutFile -Encoding UTF8 }
Write-Host $json
if ($bad.Count -gt 0) {
    Write-Host ("FAIL：{0} 档未过判据：{1}" -f $bad.Count, (($bad | ForEach-Object { $_.sessions }) -join ',')) -ForegroundColor Red
    exit 1
}
Write-Host '全部档位 PASS'
exit 0
