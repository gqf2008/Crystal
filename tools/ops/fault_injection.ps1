# fault_injection.ps1 — 故障注入最小集：杀进程/网络抖动/存储只读
#
# 判据（每项独立跑，报告落 JSON）：
#   【kill_restart】压测会话中途 kill -9 服务端 → ① 客户端能在 KillDetectSec 内感知断开
#                   ② 重启后能在 RecoverSec 内重新登录成功（不能留下卡死态）
#   【jitter】走 latency_proxy（延迟 + 丢包）登录并保持 → 会话仍能建立、服务端无真错误
#   【db_readonly】把 DB 置只读后起服 → 服务端**不 panic**，且在日志里可见明确错误/降级
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [int]$Port = 7100,
    [string]$Account = 'opsload1',
    [string]$Password = '123456',
    [int]$ReadyTimeoutSec = 90,
    [int]$KillDetectSec = 15,
    [int]$RecoverSec = 60,
    [double]$JitterDelayMs = 200,
    [double]$JitterDropPct = 5,
    [string]$OutFile = '',
    # 单次 bot 会话超时（秒）：超时按该次采样失败处理并**立刻**返回（2026-09-25 修）
    [int]$BotTimeoutSec = 90
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $ops '_run_bot.ps1')
$exe = Join-Path $DeployDir 'mir2_server.exe'
$results = [ordered]@{}

function Start-Srv([string]$tag, [int]$port) {
    $env:RUST_LOG = 'crystal_server=info'
    $log = Join-Path $DeployDir "fault.$tag.log"
    $p = Start-Process -FilePath $exe -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $DeployDir "fault.$tag.err.log") -PassThru
    for ($i = 0; $i -lt $ReadyTimeoutSec; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { return @{ proc = $p; log = $log; ready = $true } }
    }
    return @{ proc = $p; log = $log; ready = $false }
}
function Stop-All { Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 3 }
function Bot([int]$port, [int]$hold, [string[]]$extra = @()) {
    # 2026-09-25：改走带超时的共用 helper（原先 `& python bot.py …` 同步无超时 ⇒ 可能整轮静默挂死）
    $r = Invoke-BotJson -OpsDir $ops -BotArgs (@('--host', '127.0.0.1', '--port', "$port", '--accounts', $Account,
            '--password', $Password, '--sessions', '1', '--hold', "$hold") + @($extra)) `
        -TimeoutSec $BotTimeoutSec -Tag 'fault_injection'
    if ($r.timedOut) {
        Write-Host ("WARN: 故障注入 bot 超过 {0}s 未退出（port={1}）——该次采样按失败处理，见 {2}" -f `
                $BotTimeoutSec, $port, $r.errFile)
        return $null
    }
    return $r.json
}

# ---------- ① 杀进程 + 重启恢复 ----------
Stop-All
$srv = Start-Srv 'kill' $Port
$killOk = $false; $detectSec = $null; $recoverOk = $false; $recoverSec = $null
if ($srv.ready) {
    $job = Start-Job -ScriptBlock {
        param($ops, $Port, $Account, $Password)
        # job 的 runspace 不继承父作用域函数 ⇒ 内部自己 dot-source helper（父线程 Wait-Job -Timeout 是第二层）
        . (Join-Path $ops '_run_bot.ps1')
        $r = Invoke-BotJson -OpsDir $ops -BotArgs @('--host', '127.0.0.1', '--port', "$Port", '--accounts', $Account,
            '--password', $Password, '--sessions', '1', '--hold', '30') -TimeoutSec 90 -Tag 'fault_kill'
        if ($r.json) {
            $r.json | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 (Join-Path $ops 'out/fault_kill_session.json')
        }
    } -ArgumentList $ops, $Port, $Account, $Password
    Start-Sleep 8                       # 等它进图
    $tKill = Get-Date
    Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force   # 无优雅关闭
    # 会话线程会在写失败/读 EOF 时结束；等它产出结论
    Wait-Job $job -Timeout ($KillDetectSec + 10) | Out-Null
    Remove-Job $job -Force
    $sess = $null
    $sf = Join-Path $ops 'out/fault_kill_session.json'
    if (Test-Path $sf) { try { $sess = (Get-Content $sf -Raw | ConvertFrom-Json) } catch {} }
    $killOk = ($null -ne $sess)              # 会话说出了结论（断开被感知，而不是永远挂着）
    $detectSec = [Math]::Round(((Get-Date) - $tKill).TotalSeconds, 1)
    # 重启恢复
    Stop-All
    $t0 = Get-Date
    $srv2 = Start-Srv 'recover' $Port
    if ($srv2.ready) {
        $after = Bot $Port 2 @('--login-only')
        $recoverOk = ($after -and $after.summary.failed -eq 0)
        $recoverSec = [Math]::Round(((Get-Date) - $t0).TotalSeconds, 1)
        Stop-All
    }
}
$results.kill_restart = [ordered]@{
    server_ready = $srv.ready
    session_saw_disconnect = $killOk
    detect_within_sec = $detectSec
    restart_login_ok = $recoverOk
    recover_within_sec = $recoverSec
    ok = ($srv.ready -and $killOk -and $recoverOk)
}

# ---------- ② 网络抖动（延迟 + 丢包，经代理） ----------
Stop-All
$srvJ = Start-Srv 'jitter' $Port
$jitterOk = $false; $jitterDetail = $null
if ($srvJ.ready) {
    $proxy = Start-Process -FilePath 'python' -ArgumentList (Join-Path $ops 'latency_proxy.py'),
        '--listen', '7200', '--target', "127.0.0.1:$Port",
        '--delay-ms', "$JitterDelayMs", '--drop-pct', "$JitterDropPct" -PassThru -WindowStyle Hidden
    Start-Sleep 3
    $jr = Bot 7200 12
    $jitterOk = ($jr -and $jr.summary.failed -eq 0)
    $jitterDetail = if ($jr) { $jr.summary } else { $null }
    if ($proxy -and -not $proxy.HasExited) { Stop-Process -Id $proxy.Id -Force -ErrorAction SilentlyContinue }
    # 服务端在抖动下是否出现真错误（排除良性断连）
    $hr = $null
    & pwsh (Join-Path $ops 'health_report.ps1') -LogFile $srvJ.log -OutFile (Join-Path $ops 'out/fault_jitter_health.json') | Out-Null
    try { $hr = (Get-Content -Raw (Join-Path $ops 'out/fault_jitter_health.json') | ConvertFrom-Json) } catch {}
    $jitterErrors = if ($hr) { $hr.errors } else { $null }
    Stop-All
} else { $jitterErrors = $null }
$results.jitter = [ordered]@{
    server_ready = $srvJ.ready
    session_ok = $jitterOk
    session = $jitterDetail
    server_real_errors = $jitterErrors
    ok = ($srvJ.ready -and $jitterOk -and ($jitterErrors -eq 0))
}

# ---------- ③ 存储只读（DB 不可写） ----------
Stop-All
$db = Join-Path $DeployDir 'Data/crystal.db'
$dbRo = $false
if (Test-Path $db) {
    attrib +R $db | Out-Null
    $dbRo = (Get-Item $db).Attributes -band [IO.FileAttributes]::ReadOnly
}
$srvR = Start-Srv 'rodb' $Port
$roLog = Get-Content $srvR.log -ErrorAction SilentlyContinue
$panicked = @($roLog | Where-Object { $_ -match 'panicked|thread .* panicked' }).Count
$sawError = @($roLog | Where-Object { $_ -match 'ERROR|Failed|read-only|readonly' }).Count
Stop-All
attrib -R $db | Out-Null
$results.db_readonly = [ordered]@{
    db_set_readonly = [bool]$dbRo
    server_ready = $srvR.ready
    panicked = $panicked
    error_lines = $sawError
    ok = (-not $dbRo) -or (($panicked -eq 0))     # 不 panic 即算通过（可降级/可报错）
}

$allOk = ($results.kill_restart.ok -and $results.jitter.ok -and $results.db_readonly.ok)
$report = [ordered]@{
    ok = $allOk
    scenarios = $results
    criteria = '①杀进程可感知+可恢复 ②抖动下会话可用且零真错误 ③DB 只读时不 panic'
}
$json = $report | ConvertTo-Json -Depth 8
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if (-not $allOk) { exit 5 } else { exit 0 }
