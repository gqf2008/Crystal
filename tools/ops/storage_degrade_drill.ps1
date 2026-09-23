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
#
# 用法：
#   pwsh tools/ops/storage_degrade_drill.ps1 -DeployDir <deploy> -OutFile tools/ops/out/storage_degrade.json
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [string]$ExePath = '',
    [int]$Port = 7100,
    [string]$Account = 'opsload1',
    [string]$Password = '123456',
    [string]$DbPath = '',
    [int]$LockSeconds = 22,
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $ExePath) { $ExePath = Join-Path $DeployDir 'mir2_server.exe' }
if (-not $DbPath) { $DbPath = Join-Path $DeployDir 'data/crystal.db' }
$log = Join-Path $ops 'out/storage_degrade.log'
New-Item -ItemType Directory -Force -Path (Join-Path $ops 'out') | Out-Null

function BotJson([int]$hold, [string[]]$extra = @()) {
    $json = Join-Path $ops ("out/storage_degrade_bot_{0}.json" -f (Get-Random))
    & python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --accounts $Account `
        --password $Password --sessions 1 --hold $hold @extra > $json 2>&1
    try { return (Get-Content $json -Raw | ConvertFrom-Json) } catch { return $null }
}

$env:RUST_LOG = 'crystal_server=info'
$proc = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $ops 'out/storage_degrade.err.log') -PassThru
$ready = $false
for ($i = 0; $i -lt 90; $i++) { Start-Sleep 1; if ((Get-Content $log -EA SilentlyContinue) -match 'Gate listening') { $ready = $true; break } }
if (-not $ready) { Write-Host 'server not ready'; exit 9 }

# 0) 基线：正常登录一次（进图后 4s 下线 → 正常落库）
$base = BotJson 4
$baseOk = ($null -ne $base -and $base.summary.failed -eq 0)
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

Wait-Job $lockJob -Timeout ($LockSeconds + 20) | Out-Null
Remove-Job $lockJob -Force
Start-Sleep 2

# 4) J1：进程还活着
$proc.Refresh()
$j1 = -not $proc.HasExited

# 5) J2：这次写失败被明确记录（而不是静默吞掉/panic）
$text = Get-Content $log -EA SilentlyContinue
$saveFail = @($text | Where-Object { $_ -match "Failed to (save|set).*database is locked|Failed to (save|set).*error returned from database" })
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
    ok = ($j1 -and $j2 -and $j3 -and $j4)
    db = $DbPath
    lock_seconds = $LockSeconds
    lock_acquired = $lockHeld
    lock_output = ((Get-Content $lockOut -EA SilentlyContinue) -join ' | ')
    baseline_login_ok = $baseOk
    J1_server_alive_after_write_failure = $j1
    J2_write_failure_logged = $j2
    J3_read_path_ok_during_fault = $j3
    J4_recovered_after_unlock = $j4
    save_failure_lines = $saveFail.Count
    baseline_saved_lines = $savedBefore
    recovered_saved_lines = $savedAfter
    victim_session_ok = $victimOk
    fault_window_lines = $linesBeforeFault
    saved_ok_lines = $savedOk.Count
    server_alive_at_end = $aliveAtEnd
    excerpt = @($text | Where-Object { $_ -match 'database is locked' } | Select-Object -First 3)
}
$jsonOut = $report | ConvertTo-Json -Depth 5
if ($OutFile) { $jsonOut | Set-Content -Encoding utf8 $OutFile }
Write-Host $jsonOut
exit 0
