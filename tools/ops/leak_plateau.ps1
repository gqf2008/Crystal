# leak_plateau.ps1 — 「登出后不归还」门禁：连登连退后必须**收敛到平台**，而不是线性增长
#
# 背景（CAPACITY.md §4.1 残留「登出后不归还」）：早期观测到 release 20 会话 × 5 轮，
# 每轮 idle RSS 都往上抬一点（46.3→48.3→49.4→50.5→51.0），当时的记法像"每轮线性泄漏"。
# 2026-09-24 用更高采样密度（RSS + **线程数 + 句柄数 + admin 计数器**）复核后定性为：
#   **前 1~4 轮的预热台阶**（heap/懒初始化），之后 RSS/线程/句柄/计数全部**平台化**。
# 本脚本把这条定性做成可复跑门禁：预热若干轮，再测若干轮，断言平台而非增长。
#
# 判据（缺一不可）：
#   J1 每个 idle 采样 `online_players == 0`（玩家记录真被回收，不是靠时间掩盖）
#   J2 每个 idle 采样 `tasks.running` 回到基线（无后台任务泄漏；基线=预热前 idle 的值）
#   J3 测量轮 idle **线程数**与预热后基线一致（±2）；句柄数一致（±8）
#   J4 测量轮 idle RSS 斜率 ≤ `-MaxRssSlopePerCycleMb`（默认 0.5 MB/轮，20 会话/轮）
#
# 用法：
#   pwsh tools/ops/leak_plateau.ps1 -DeployDir <deploy> -ExePath <mir2_server.exe> `
#        -Sessions 20 -WarmCycles 3 -MeasureCycles 5 -OutFile tools/ops/out/leak_plateau.json
#
# 前置：deploy 库里有 `-AccountPrefix` 前缀的账号+角色（用 `seed_load_accounts.py` 播种；
#       admin 端口 = gate 端口 + 1，脚本按 `-Port + 1` 取，别硬编码 7001）。
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Port = 7100,
    [string]$AccountPrefix = 'opsload',
    [int]$Sessions = 20,
    [int]$WarmCycles = 3,
    [int]$MeasureCycles = 5,
    [int]$HoldSec = 8,
    [int]$IdleSec = 12,
    [double]$MaxRssSlopePerCycleMb = 0.5,
    [string]$OutFile = '',
    # 每轮连登连退 bot 的超时（秒）：超时按本轮失败处理并**立刻**返回（2026-09-25 修）
    [int]$BotTimeoutSec = 180
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $ops '_run_bot.ps1')
New-Item -ItemType Directory -Force -Path (Join-Path $ops 'out') | Out-Null
$log = Join-Path $ops 'out/leak_plateau.log'
$env:RUST_LOG = 'crystal_server=info'

Get-CimInstance Win32_Process -Filter "Name='mir2_server.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Seconds 2
$proc = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $ops 'out/leak_plateau.err.log') -PassThru
$ready = $false
for ($i = 0; $i -lt 90; $i++) { Start-Sleep 1; if ((Get-Content $log -EA SilentlyContinue) -match 'Gate listening') { $ready = $true; break } }
if (-not $ready) { Write-Host 'server not ready'; exit 9 }

function Sample([string]$tag) {
    $proc.Refresh()
    $health = $null
    try {
        $health = (Invoke-WebRequest -UseBasicParsing -TimeoutSec 5 ("http://127.0.0.1:{0}/" -f ($Port + 1)) -EA Stop).Content | ConvertFrom-Json
    } catch { $health = $null }
    [pscustomobject]@{
        tag            = $tag
        rss_mb         = [Math]::Round($proc.WorkingSet64 / 1MB, 2)
        threads        = $proc.Threads.Count
        handles        = $proc.HandleCount
        online_players = if ($health) { [int]$health.online_players } else { -1 }
        tasks_running  = if ($health) { [int]$health.tasks.running } else { -1 }
    }
}
function RunCycle([int]$n) {
    # 2026-09-25：改走带超时的共用 helper（原先 `& python bot.py …` 同步无超时）
    $r = Invoke-BotJson -OpsDir $ops -BotArgs @('--host', '127.0.0.1', '--port', "$Port",
        '--account-prefix', $AccountPrefix, '--sessions', "$Sessions", '--hold', "$HoldSec", '--password', '123456') `
        -TimeoutSec $BotTimeoutSec -Tag "leak_plateau_cycle$n"
    if ($r.timedOut) {
        Write-Host ("WARN: 连登连退 bot 超过 {0}s 未退出（port={1}）——本轮按失败处理，见 {2}" -f `
                $BotTimeoutSec, $Port, $r.errFile)
        return $null
    }
    if ($null -eq $r.json) { return $null }
    return $r.json.summary
}

$rows = @(); $baseline = Sample 'idle_before'
$rows += $baseline
$cycleNo = 0
foreach ($phase in @(@('warm', $WarmCycles), @('measure', $MeasureCycles))) {
    foreach ($i in 1..$phase[1]) {
        $cycleNo++
        $sum = RunCycle $cycleNo
        if ($null -eq $sum -or $sum.failed -ne 0) {
            Write-Host ("FAIL(J0): 第 {0} 轮会话失败（ok={1} failed={2}）——标定无效，不产出报告" -f $cycleNo, $sum.ok, $sum.failed)
            Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
            exit 3
        }
        Start-Sleep -Seconds $IdleSec
        $rows += Sample ("{0}{1}_idle" -f $phase[0], $i)
    }
}
$proc.Refresh()
$alive = -not $proc.HasExited
Stop-Process -Id $proc.Id -Force -EA SilentlyContinue

$measured = @($rows | Where-Object { $_.tag -like 'measure*' })
$j1 = -not (@($measured | Where-Object { $_.online_players -ne 0 }).Count -gt 0)
$j2 = -not (@($measured | Where-Object { $_.tasks_running -ne $baseline.tasks_running }).Count -gt 0)
$j3thr = if ($measured.Count -gt 0) { ($measured | Measure-Object threads -Maximum).Maximum - ($measured | Measure-Object threads -Minimum).Minimum } else { 99 }
$j3hnd = if ($measured.Count -gt 0) { ($measured | Measure-Object handles -Maximum).Maximum - ($measured | Measure-Object handles -Minimum).Minimum } else { 999 }
$j3 = ($j3thr -le 2) -and ($j3hnd -le 8)
$slope = if ($measured.Count -ge 2) {
    ($measured[-1].rss_mb - $measured[0].rss_mb) / ($measured.Count - 1)
} else { 99 }
$j4 = ($slope -le $MaxRssSlopePerCycleMb)
$ok = $j1 -and $j2 -and $j3 -and $j4

$report = [ordered]@{
    ok                        = $ok
    sessions                  = $Sessions
    warm_cycles               = $WarmCycles
    measure_cycles            = $MeasureCycles
    baseline_idle_rss_mb      = $baseline.rss_mb
    measured_idle_rss_mb      = @($measured | ForEach-Object { $_.rss_mb })
    rss_slope_mb_per_cycle    = [Math]::Round($slope, 3)
    max_slope_allowed         = $MaxRssSlopePerCycleMb
    J1_online_players_zero    = $j1
    J2_tasks_running_back     = $j2
    J3_threads_handles_stable = $j3
    J3_thread_span            = $j3thr
    J3_handle_span            = $j3hnd
    J4_plateau_not_leak       = $j4
    samples                   = $rows
    server_alive_at_end       = $alive
}
$jsonOut = $report | ConvertTo-Json -Depth 5
if ($OutFile) { $jsonOut | Set-Content -Encoding utf8 $OutFile }
Write-Host $jsonOut
if ($ok) { exit 0 } else { exit 10 }
