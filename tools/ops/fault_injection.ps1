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
    # **默认 0**：`latency_proxy.py` 的 `--drop-pct` 是在字节流中间直接丢且**不重传**，
    # 会破坏 TCP 语义（下游收到拼接坏的帧）——它自己的文件头就写明「想测丢包不要用本开关、
    # 想测延迟用 --drop-pct 0」。此前默认 5 ⇒ jitter 用例**必然** session 超时假红
    # （2026-09-27 实测：login_sent 后 TimeoutError、frames=0）。要探丢包/断流请另做
    # 「按连接丢/阻断再放行」的模型，别用这个开关。
    [double]$JitterDropPct = 0,
    [string]$OutFile = '',
    # 单次 bot 会话超时（秒）：超时按该次采样失败处理并**立刻**返回（2026-09-25 修）
    [int]$BotTimeoutSec = 90,
    # 客户端可见报错的文案判据（与 storage_degrade_drill 同口径；可调便于做「断言本身会红」的阳性对照）
    [string]$NoticePattern = '存档失败',
    # 只读用例里会话的保持秒数：必须跨过「每 300 ticks 的自动存档」那一拍。
    # **实测口径**：本服 tick≈100ms（heartbeat: 300 ticks / 30s）⇒ 自动存档**每 30s**一次，
    # 12s 的会话根本打不到它（第一版就是 12s，日志里连 `Auto-save` 行都没有）。
    # 默认 40s：保证跨过至少一次自动存档，才能验到「玩家在线时落库失败也看得到」。
    [int]$RoHoldSec = 40
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $ops '_run_bot.ps1')
$exe = Join-Path $DeployDir 'mir2_server.exe'
$results = [ordered]@{}
# 本脚本启动过的服务端（**按 PID 管控**）。2026-09-25：原先 `Stop-All` 按公共名
# `Get-Process -Name mir2_server | Stop-Process` 清场，会把同机别人的服务端（7000 上的常驻开发服、
# 别的 agent 的演练）一起带走。现在只停自己启动的实例 +（兜底）从**本 deploy 目录的 exe** 起的残留。
$script:FaultProcs = @()

function Start-Srv([string]$tag, [int]$port) {
    $env:RUST_LOG = 'crystal_server=info'
    $log = Join-Path $DeployDir "fault.$tag.log"
    $p = Start-Process -FilePath $exe -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $DeployDir "fault.$tag.err.log") -PassThru
    $script:FaultProcs += $p
    for ($i = 0; $i -lt $ReadyTimeoutSec; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { return @{ proc = $p; log = $log; ready = $true } }
    }
    return @{ proc = $p; log = $log; ready = $false }
}
function Stop-All {
    # 只停自己启动过的（PID），再按 **exe 路径**兜底清理本 deploy 目录的残留；
    # 绝不按公共名清场（见文件头/上方注释）。
    foreach ($p in @($script:FaultProcs)) {
        if ($p -and -not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    }
    $script:FaultProcs = @()
    Get-CimInstance Win32_Process -Filter "Name='mir2_server.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $exe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Start-Sleep 3
}
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
    # 故障注入：**无优雅关闭**地杀掉本次场景自己起的那个实例（按 PID，不按公共名）
    Stop-Process -Id $srv.proc.Id -Force -ErrorAction SilentlyContinue
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
    # **必须先建 out 目录**：`health_report.ps1` 写不出报告时这里会静默变成 `$null`，
    # 而判据 `($jitterErrors -eq 0)` 对 `$null` 不成立 ⇒ 整条 jitter 用例**假红**。
    # 2026-09-27 实测：fresh worktree 里 `tools/ops/out/` 不存在（该目录不入库），
    # 于是 `jitter.ok=false`、drill exit 5；同一条日志手工跑 health_report 却是 `errors=0 / PASS`。
    $hrOut = Join-Path $ops 'out/fault_jitter_health.json'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $hrOut) | Out-Null
    & pwsh (Join-Path $ops 'health_report.ps1') -LogFile $srvJ.log -OutFile $hrOut | Out-Null
    $hr = $null
    try { $hr = (Get-Content -Raw $hrOut | ConvertFrom-Json) } catch {}
    if ($null -eq $hr) {
        # 报告都产不出来 = 这条判据**没判成**，要说清楚，别让人以为"服务端有真错误"
        Write-Host ('WARN：health_report 未产出可解析报告（{0}）——jitter 判据按"未判成"记 false' -f $hrOut)
    }
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
# **判据补齐（2026-09-27）**：只验「服务端没 panic + 日志有错误行」是不够的——owner 已拍板
# 「落库失败一律**直接反馈到客户端**」，所以只读 DB 这条故障路径也必须断言**客户端看得到**
# （与 storage_degrade_drill 的 J5 同口径、同文案正则）。此前这里只 grep 日志，等于把
# 「服务端降级可见」当成了「玩家可见」，而两者的实现路径不同（一个是 persist_report 打日志，
#  一个是往会话发 S.Chat/ChatType::System）。
$roSession = if ($srvR.ready) { Bot $Port $RoHoldSec } else { $null }
$roMsgs = @()
if ($null -ne $roSession) {
    foreach ($s in @($roSession.sessions)) {
        if ($null -ne $s -and $null -ne $s.system_messages) { $roMsgs += @($s.system_messages) }
    }
}
$roHit = @($roMsgs | Where-Object { $_ -match $NoticePattern })
$clientNotice = $roHit.Count -gt 0
$roLog = Get-Content $srvR.log -ErrorAction SilentlyContinue
$panicked = @($roLog | Where-Object { $_ -match 'panicked|thread .* panicked' }).Count
$sawError = @($roLog | Where-Object { $_ -match 'ERROR|Failed|read-only|readonly|PERSIST_LOST' }).Count
Stop-All
attrib -R $db | Out-Null
$results.db_readonly = [ordered]@{
    db_set_readonly = [bool]$dbRo
    server_ready = $srvR.ready
    session_failed = if ($roSession) { $roSession.summary.failed } else { $null }
    client_notice_seen = $clientNotice
    client_notice_messages = @($roHit | Select-Object -First 3)
    client_system_messages_total = $roMsgs.Count
    notice_pattern = $NoticePattern
    panicked = $panicked
    error_lines = $sawError
    # 不 panic **且** 玩家看得到失败提示，才算这条故障路径真的按拍板语义降级
    ok = (-not $dbRo) -or (($panicked -eq 0) -and $clientNotice)
}

$allOk = ($results.kill_restart.ok -and $results.jitter.ok -and $results.db_readonly.ok)
$report = [ordered]@{
    ok = $allOk
    scenarios = $results
    criteria = '①杀进程可感知+可恢复 ②抖动下会话可用且零真错误 ③DB 只读时不 panic 且落库失败对客户端可见'
}
$json = $report | ConvertTo-Json -Depth 8
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if (-not $allOk) { exit 5 } else { exit 0 }
