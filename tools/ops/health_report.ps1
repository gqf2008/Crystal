# health_report.ps1 — 可观测性最小集：把服务端日志 + 进程指标折成可判定的 JSON
#
# 判据（PASS/WARN/FAIL 由 -MaxErrors / -MaxTickJitterPct 控制）：
#   tick 节拍抖动（服务端半死最先在这里现形）
#   ERROR 行数、WARN 分类计数
#   已知坏味道：慢读者被踢 / 邮箱背压丢包 / 帧解码失败 / 读取错误 / 未知 NPC object_id
#   进程 RSS / CPU 时间 / 句柄数
param(
    [Parameter(Mandatory = $true)][string]$LogFile,
    [string]$ProcessName = 'mir2_server',
    [string]$OutFile = '',
    [int]$MaxErrors = 0,
    [double]$MaxTickJitterPct = 50.0
)
$ErrorActionPreference = 'Continue'
if (-not (Test-Path $LogFile)) { Write-Host "FAIL: 日志不存在：$LogFile"; exit 2 }
$lines = Get-Content $LogFile -ErrorAction SilentlyContinue

# ---- tick 节拍（服务端每 ~10s 打一行 "World tick #N"）----
$tickTs = @()
foreach ($l in $lines) {
    if ($l -match 'World tick #(\d+)') {
        if ($l -match '^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?)Z') {
            # 日志时间戳是 UTC：显式指定 Kind，避免被当本地时间（抖动计算虽然只看差值，
            # 但跨时区/夏令时的比较要靠这个口径）
            $tickTs += [datetime]::SpecifyKind(
                [datetime]::Parse($Matches[1], $null, [System.Globalization.DateTimeStyles]::RoundtripKind),
                [System.DateTimeKind]::Utc)
        }
    }
}
$tickCount = $tickTs.Count
$gaps = @()
for ($i = 1; $i -lt $tickTs.Count; $i++) { $gaps += ($tickTs[$i] - $tickTs[$i - 1]).TotalSeconds }
$gapAvg = if ($gaps.Count) { ($gaps | Measure-Object -Average).Average } else { 0 }
$gapMax = if ($gaps.Count) { ($gaps | Measure-Object -Maximum).Maximum } else { 0 }
$jitter = if ($gapAvg -gt 0) { [Math]::Round((($gapMax - $gapAvg) / $gapAvg) * 100, 1) } else { 0 }

# ---- 日志信号 ----
# 区分「良性噪声」与「真错误」：压测/运维里主动踢线、客户端进程被杀都会产生
# `Session N read error: 远程主机强迫关闭了一个现有的连接`——它不是故障，是断连的正常表现。
# 把两类混在一起会让健康判定永远 FAIL（本轮实测就是这个坑）。
$allErrors = @($lines | Where-Object { $_ -match '\sERROR\s' })
$benignPattern = 'read error.*(forced|强迫关闭|os error 10054)|Connection reset'
$errors = @($allErrors | Where-Object { $_ -notmatch $benignPattern })
$benignDisconnects = $allErrors.Count - $errors.Count
$warns = @($lines | Where-Object { $_ -match '\sWARN\s' })
function Count-Match([string]$pattern) { @($lines | Where-Object { $_ -match $pattern }).Count }
$signals = [ordered]@{
    slow_reader_kicked = Count-Match 'kicking slow reader'
    gate_mailbox_full  = Count-Match 'gate mailbox full'
    frame_decode_fail  = Count-Match '帧解码失败|frame decode'
    session_read_error = Count-Match 'read error|Session .* read error'
    unknown_npc_object = Count-Match 'NPC call for unknown object_id'
    disconnected       = Count-Match 'Disconnected|与服务器断开|disconnected'
}

# ---- 进程指标 ----
$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
$procInfo = if ($proc) {
    [ordered]@{
        pid        = $proc.Id
        rss_mb     = [Math]::Round($proc.WorkingSet64 / 1MB, 1)
        cpu_sec    = [Math]::Round($proc.TotalProcessorTime.TotalSeconds, 1)
        handles    = $proc.HandleCount
        threads    = $proc.Threads.Count
        uptime_min = [Math]::Round(((Get-Date) - $proc.StartTime).TotalMinutes, 1)
    }
} else { $null }

$ticksAvailable = ($tickCount -ge 3)
$verdict = 'PASS'
if ($errors.Count -gt $MaxErrors) { $verdict = 'FAIL' }
elseif ($ticksAvailable -and ($jitter -gt $MaxTickJitterPct)) { $verdict = 'WARN' }
elseif ($signals.slow_reader_kicked -gt 0 -or $signals.gate_mailbox_full -gt 0) { $verdict = 'WARN' }

$report = [ordered]@{
    ok        = ($verdict -ne 'FAIL')
    verdict   = $verdict
    log_file  = (Resolve-Path $LogFile).Path
    ticks     = [ordered]@{
        available = $ticksAvailable
        note = if ($ticksAvailable) { '' } else { '日志级别未含 DEBUG（World tick #N 是 debug 行）——tick 指标不可用，不据此判 WARN' }
        count = $tickCount; gap_avg_sec = [Math]::Round($gapAvg, 2); gap_max_sec = [Math]::Round($gapMax, 2)
        jitter_pct = $jitter; max_jitter_pct = $MaxTickJitterPct
    }
    errors    = $errors.Count
    benign_disconnects = $benignDisconnects
    warns     = $warns.Count
    signals   = $signals
    process   = $procInfo
}
$json = $report | ConvertTo-Json -Depth 6
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if ($verdict -eq 'FAIL') { exit 5 } else { exit 0 }
