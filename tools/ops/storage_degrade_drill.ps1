# storage_degrade_drill.ps1 — 运行中存储故障降级演练
#
# 与 fault_injection.ps1 的 `db_readonly` 的区别：那一项是「**起服前**把 DB 置只读」
# （只验了不 panic 且日志有明确错误）。本项注入的是**运行中**的写故障：
# 服务端已有会话时，另一个进程用 `BEGIN IMMEDIATE` 占住 SQLite 写锁 20s，
# 期间服务端的一次真实落库（玩家下线 save_character）必然撞 busy_timeout(5s) 失败。
#
# 判据（缺一不可）：
#   J1 写失败后服务端**进程仍在**（没有 panic/退出）
#   J2 日志里能看到这次写失败（明确的存储错误，而不是静默吞掉）
#   J3 故障期间的**读路径不受影响**：同一时刻新会话仍能登录成功（登录不依赖写锁）
#   J4 释放写锁后**恢复**：新会话正常下线，日志出现 "saved to database on disconnect"
#   J5 落库失败**玩家看得到**：故障窗口内的会话收到 `S.Chat`+`ChatType::System` 的
#      「存档失败：…」提示（owner 2026-09-24 拍板：失败一律直接反馈到客户端）
#
# 前置（J0）：基线（无故障）会话必须成功——否则后面各判据都是空转（2026-09-24 踩过：
#   基线登录失败时报告只剩 J1=true，其余全 false，看起来像"服务端有问题"，其实是端口被别的东西占着）。
#
# 用法：
#   pwsh tools/ops/storage_degrade_drill.ps1 -DeployDir <deploy> -OutFile tools/ops/out/storage_degrade.json \
#       -Account <有角色的账号> -Password <密码> -NoticePattern '存档失败'
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [string]$ExePath = '',
    [int]$Port = 7100,
    [string]$Account = 'opsload1',
    [string]$Password = '123456',
    [string]$DbPath = '',
    [int]$LockSeconds = 22,
    [string]$OutFile = '',
    # 客户端可见报错的文案判据（正则可调，便于做「断言本身会红」的阳性对照）
    [string]$NoticePattern = '存档失败',
    # 单次 bot 会话的超时（秒）。超时视为该次采样失败并**立刻**返回，不阻塞整轮（2026-09-25 修）
    [int]$BotTimeoutSec = 120
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $ExePath) { $ExePath = Join-Path $DeployDir 'mir2_server.exe' }
if (-not $DbPath) { $DbPath = Join-Path $DeployDir 'data/crystal.db' }
$log = Join-Path $ops 'out/storage_degrade.log'
New-Item -ItemType Directory -Force -Path (Join-Path $ops 'out') | Out-Null

function BotJson([int]$hold, [string[]]$extra = @()) {
    $json = Join-Path $ops ("out/storage_degrade_bot_{0}.json" -f (Get-Random))
    # 2026-09-25：bot 调用必须**有超时**。此前直接 `& python … ` 同步调用：一旦服务端没绑到 -Port
    # （例如 DeployDir 的 config/server.toml 写的是 7000、而 -Port 传的是 7100），bot 会一直等下去，
    # 整个演练**静默挂死**（实测挂了 6 分钟以上、无任何输出，调用方无法分辨"在跑"还是"卡住"）。
    $botArgs = @(
        (Join-Path $ops 'bot.py'), '--host', '127.0.0.1', '--port', "$Port", '--accounts', $Account,
        '--password', $Password, '--sessions', '1', '--hold', "$hold"
    ) + @($extra)
    $p = Start-Process -FilePath 'python' -ArgumentList $botArgs `
        -RedirectStandardOutput $json -RedirectStandardError "$json.err" -PassThru -WindowStyle Hidden
    $deadline = (Get-Date).AddSeconds($BotTimeoutSec)
    while (-not $p.HasExited -and (Get-Date) -lt $deadline) { Start-Sleep 1 }
    if (-not $p.HasExited) {
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        Write-Host ("WARN: bot.py 超过 {0}s 未退出（port={1}）——本轮判据按失败处理，见 {2}.err" -f `
                $BotTimeoutSec, $Port, $json)
        return $null
    }
    try { return (Get-Content $json -Raw | ConvertFrom-Json) } catch { return $null }
}

$env:RUST_LOG = 'crystal_server=info'
# 2026-09-25：**监听口必须与 -Port 一致**。服务端读的是 <DeployDir>/config/server.toml 的
# `[network].listen_addr`；`-Port` 只作用于 bot。两者不一致时以前会静默挂死（见 BotJson 注释）。
# 这里：若配置里的端口 != -Port，就生成一份临时配置（只改 listen_addr）并按仓库既有约定
# `mir2_server <config>` 启动，同时在输出里写明用的是哪份配置。
$cfgIn = Join-Path $DeployDir 'config/server.toml'
$cfgPort = $null
if (Test-Path -LiteralPath $cfgIn) {
    $m = Select-String -Path $cfgIn -Pattern 'listen_addr\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($m -and $m.Matches[0].Groups[1].Value -match ':(\d+)$') { $cfgPort = [int]$Matches[1] }
}
$srvArgs = @()
if ($cfgPort -ne $Port) {
    $cfgOut = Join-Path $ops 'out/storage_degrade_server.toml'
    $cfgText = if (Test-Path -LiteralPath $cfgIn) { Get-Content $cfgIn -Raw } else { "[network]`nlisten_addr = `"0.0.0.0:7000`"`n" }
    if ($cfgText -match 'listen_addr\s*=\s*"[^"]+"') {
        $cfgText = [regex]::Replace($cfgText, 'listen_addr\s*=\s*"[^"]+"', "listen_addr = `"0.0.0.0:$Port`"", 1)
    } else {
        $cfgText = "[network]`nlisten_addr = `"0.0.0.0:$Port`"`n" + $cfgText
    }
    Set-Content -LiteralPath $cfgOut -Value $cfgText -Encoding utf8
    $srvArgs = @($cfgOut)
    Write-Host ("[环境] 部署配置的 listen_addr 是端口 {0}，与本轮 -Port {1} 不一致 → 用临时配置 {2}" -f `
            $cfgPort, $Port, $cfgOut)
} else {
    Write-Host ("[环境] 服务端监听口={0}（与 -Port 一致，直接用部署配置）" -f $Port)
}
$srvParams = @{
    FilePath               = $ExePath
    WorkingDirectory       = $DeployDir
    RedirectStandardOutput = $log
    RedirectStandardError  = (Join-Path $ops 'out/storage_degrade.err.log')
    PassThru               = $true
}
if ($srvArgs.Count -gt 0) { $srvParams.ArgumentList = $srvArgs }
$proc = Start-Process @srvParams
$ready = $false
for ($i = 0; $i -lt 90; $i++) {
    Start-Sleep 1
    if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { $ready = $true; break }
}
if (-not $ready) {
    Write-Host ("server not ready on port {0}（gate 未监听；日志尾巴：{1}）" -f `
            $Port, ((Get-Content $log -EA SilentlyContinue | Select-Object -Last 3) -join ' / '))
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    exit 9
}

# 0) 基线：正常登录一次（进图后 4s 下线 → 正常落库）
$base = BotJson 4
$baseOk = ($null -ne $base -and $base.summary.failed -eq 0)
if (-not $baseOk) {
    Write-Host ("FAIL(J0): 基线会话未成功（{0}）——故障窗口的结论会全部空转，本轮不产出报告。" -f `
        ((@($base.sessions) | ForEach-Object { "$($_.account):$($_.stage):$($_.error)" }) -join ' | '))
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    exit 3
}
$linesBeforeFault = @(Get-Content $log -EA SilentlyContinue).Count

# 1) 先注入写锁（20s 级），确认真拿住了
$lockOut = Join-Path $ops 'out/storage_degrade_lock.txt'
Remove-Item $lockOut -ErrorAction SilentlyContinue
$lockJob = Start-Job -ScriptBlock {
    param($ops, $DbPath, $secs, $out)
    & python (Join-Path $ops 'db_write_lock.py') --db $DbPath --seconds $secs *> $out
} -ArgumentList $ops, $DbPath, $LockSeconds, $lockOut
# 判据必须是**注入了故障**，不是"job 还在跑"——`db_write_lock.py` 拿不到写锁时会打印
# LOCK_FAILED 并以退出码 2 结束，但 job 状态可能仍是 Running/Completed 的假象（2026-09-23 踩过：
# 一次探针里锁根本没拿到，却按"已注入"跑完了整轮，结论全废）。
$lockHeld = $false
for ($i = 0; $i -lt 15; $i++) {
    Start-Sleep 1
    $txt = Get-Content $lockOut -ErrorAction SilentlyContinue
    if ($txt -match 'LOCK_HELD') { $lockHeld = $true; break }
    if ($txt -match 'LOCK_FAILED') { break }
}
if (-not $lockHeld) {
    Write-Host ("FAIL: 写锁未真正拿到（db_write_lock.py 输出：{0}）——本轮结论无效，不产出报告" -f ((Get-Content $lockOut -EA SilentlyContinue) -join ' '))
    Wait-Job $lockJob -Timeout 5 | Out-Null; Remove-Job $lockJob -Force -EA SilentlyContinue
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    exit 3
}

# 2) 锁住期间走一个完整会话（进图 4s 后下线）——它的两次落库都会撞锁：
#    账号保存（login/offline）与角色保存（logout）
$victim = BotJson 4
$victimOk = ($null -ne $victim -and $victim.summary.failed -eq 0)

# 3) J3：故障期间新会话仍能登录（读路径 + 写失败不阻断登录）
$duringLock = BotJson 3 @('--login-only')
$j3 = ($null -ne $duringLock -and $duringLock.summary.failed -eq 0)

# 3b) J5：故障窗口内的会话必须收到**客户端可见**的落库失败提示（S.Chat / ChatType::System）
$noticeMsgs = @()
foreach ($s in @(@($victim.sessions) + @($duringLock.sessions))) {
    if ($null -ne $s -and $null -ne $s.system_messages) { $noticeMsgs += @($s.system_messages) }
}
$hit = @($noticeMsgs | Where-Object { $_ -match $NoticePattern })
$j5 = $hit.Count -gt 0

Wait-Job $lockJob -Timeout ($LockSeconds + 20) | Out-Null
Remove-Job $lockJob -Force
Start-Sleep 2

# 4) J1：进程还活着
$proc.Refresh()
$j1 = -not $proc.HasExited

# 5) J2：这次写失败被明确记录（而不是静默吞掉/panic）
# 2026-09-24：判据曾按旧日志格式 `Failed to (save|set)…`，而实现早在持久化改版时换成了
# 可 grep/可告警的固定前缀 `PERSIST_LOST`（`db::persist_report`），于是 J2 长期假红。
# 现在主判据是前缀本身，旧的两种写法保留为兼容分支。
$text = Get-Content $log -EA SilentlyContinue
$saveFail = @($text | Where-Object {
        $_ -match 'PERSIST_LOST' -or
        $_ -match "Failed to (save|set).*database is locked" -or
        $_ -match "Failed to (save|set).*error returned from database"
    })
$j2 = $saveFail.Count -gt 0

# 6) J4：释放写锁后恢复——新会话下线能正常落库（取故障窗口之后新出现的成功行）
$savedBefore = @(Get-Content $log -EA SilentlyContinue | Where-Object { $_ -match 'saved to database on logout' }).Count
$after = BotJson 4
Start-Sleep 2
$savedAfter = @(Get-Content $log -EA SilentlyContinue | Where-Object { $_ -match 'saved to database on logout' }).Count
$j4 = ($null -ne $after -and $after.summary.failed -eq 0 -and $savedAfter -gt $savedBefore)

$proc.Refresh()
$proc.Refresh()
$aliveAtEnd = -not $proc.HasExited
Stop-Process -Id $proc.Id -Force -EA SilentlyContinue

$report = [ordered]@{
    ok = ($j1 -and $j2 -and $j3 -and $j4 -and $j5)
    db = $DbPath
    lock_seconds = $LockSeconds
    lock_acquired = $lockHeld
    lock_output = ((Get-Content $lockOut -EA SilentlyContinue) -join ' | ')
    baseline_login_ok = $baseOk
    J1_server_alive_after_write_failure = $j1
    J2_write_failure_logged = $j2
    J3_read_path_ok_during_fault = $j3
    J4_recovered_after_unlock = $j4
    J5_client_notice_seen = $j5
    # 报告里放**命中文案**（而不是前 5 条随便什么系统消息），便于人工核对
    client_notice_messages = @($hit | Select-Object -First 5)
    client_system_messages_total = $noticeMsgs.Count
    notice_pattern = $NoticePattern
    save_failure_lines = $saveFail.Count
    baseline_saved_lines = $savedBefore
    recovered_saved_lines = $savedAfter
    victim_session_ok = $victimOk
    fault_window_lines = $linesBeforeFault
    server_alive_at_end = $aliveAtEnd
    excerpt = @($text | Where-Object { $_ -match 'database is locked' } | Select-Object -First 3)
}
$jsonOut = $report | ConvertTo-Json -Depth 5
if ($OutFile) { $jsonOut | Set-Content -Encoding utf8 $OutFile }
Write-Host $jsonOut
exit 0
