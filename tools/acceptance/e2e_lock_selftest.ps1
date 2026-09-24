#Requires -Version 5.1
<#
.SYNOPSIS
  `e2e_lock.ps1`（实机资源跨进程互斥锁）的自证夹具 —— 不起客户端、不连服务端、不登录账号，秒级。

.DESCRIPTION
  为什么需要它：`e2e_lock.ps1` 是"多 agent 并行跑实机"的**唯一**互斥点，它错了就是两个客户端
  同时跑 —— 现象正是 `login 失败 result=4 密码错误`（服务端实为 `Account already online`）那种
  **假红**。而它的语义（僵尸回收 / PID 复用 / 超龄 / 继承）此前只有一次性探针验过，探针跑完就删了，
  下一个人改脚本时没有任何可复跑的回归网。

  隔离：本夹具把 `TEMP`/`TMP` 指向临时目录后再 dot-source 锁脚本，所以**绝不会碰真实锁**
  （`%TEMP%\crystal_e2e_client_test.lock`，别的 agent 正在用）。开头会断言锁路径确实落在沙箱里，
  否则直接 exit 2（宁可前置失败，也不冒险去动真锁）。

  用法：
    pwsh tools/acceptance/e2e_lock_selftest.ps1                       # 验当前脚本
    pwsh tools/acceptance/e2e_lock_selftest.ps1 -LockScriptPath <旧版> # 阳性/阴性对照（旧版应挂）

  退出码：0 = 全过；1 = 有用例 FAIL；2 = 前置失败（锁路径不在沙箱 / 脚本文件缺失）。
#>
param(
    [string]$LockScriptPath = '',
    [int]$GraceSec = 5
)
$ErrorActionPreference = 'Continue'

$pass = @()
$fail = @()
function Check([string]$name, [bool]$cond, [string]$detail = '') {
    if ($cond) {
        $script:pass += $name
        Write-Host ("  [PASS] {0}" -f $name) -ForegroundColor Green
    } else {
        $script:fail += $name
        $msg = "  [FAIL] $name"
        if ($detail) { $msg += " —— $detail" }
        Write-Host $msg -ForegroundColor Red
    }
}

if (-not $LockScriptPath) { $LockScriptPath = Join-Path $PSScriptRoot 'e2e_lock.ps1' }
$LockScriptPath = (Resolve-Path -LiteralPath $LockScriptPath -EA Stop).Path
Write-Host ("被测脚本：{0}" -f $LockScriptPath)

# ---- 沙箱：把 TEMP 指到临时目录，真实锁文件绝不会被本夹具读到/删掉 ----
$sandbox = Join-Path ([IO.Path]::GetTempPath()) ("crystal_e2e_lock_selftest_" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $sandbox -Force | Out-Null
$env:TEMP = $sandbox
$env:TMP = $sandbox
Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue

. $LockScriptPath

$lock = Get-E2eLockPath
if ($lock -notlike "$sandbox*") {
    Write-Host ("前置失败：锁路径不在沙箱内（{0}）—— 拒绝在真实锁上做实验" -f $lock) -ForegroundColor Red
    exit 2
}
Write-Host ("沙箱锁路径：{0}" -f $lock)

function Reset-Lock {
    Remove-Item -LiteralPath $lock -Force -EA SilentlyContinue
    Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue
    $script:E2eLockHeld = $false
    $script:E2eLockInherited = $false
}
function Write-FakeLock([int]$HolderPid, [datetime]$Started, [string]$Script = 'selftest-holder') {
    $payload = @{
        pid     = $HolderPid
        script  = $Script
        host    = $env:COMPUTERNAME
        started = $Started.ToString('o')
    } | ConvertTo-Json -Compress
    [System.IO.File]::WriteAllText($lock, $payload, [Text.Encoding]::UTF8)
}
function Get-LockRaw { if (Test-Path -LiteralPath $lock) { Get-Content -LiteralPath $lock -Raw } else { $null } }
function Get-OwnPid { $PID }
function New-DeadPid {
    foreach ($cand in 999999, 987654, 876543, 765432) {
        if (-not (Get-Process -Id $cand -EA SilentlyContinue)) { return $cand }
    }
    return 999999
}
function Get-LongLivedProcess {
    foreach ($p in (Get-Process -EA SilentlyContinue)) {
        try {
            if ($p.Id -ne $PID -and $p.StartTime -and $p.StartTime -lt (Get-Date).AddSeconds(-120)) { return $p }
        } catch {}
    }
    return $null
}
function Timer([scriptblock]$blk) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $r = & $blk
    $sw.Stop()
    return @{ ok = $r; ms = [int]$sw.Elapsed.TotalMilliseconds }
}

$me = Get-OwnPid
$myStart = (Get-Process -Id $PID).StartTime

Write-Host 'T1 正常获取 / 释放'
Reset-Lock
$got = Enter-E2eLock -ScriptName 'selftest-basic' -TimeoutSec 3 -PollSec 1
$h = $null; try { $h = Get-LockRaw | ConvertFrom-Json } catch {}
Check 'T1.1 拿到锁返回 true' ($got -eq $true)
Check 'T1.2 锁文件已建立且 pid 是自己' ($null -ne $h -and [int]$h.pid -eq $me) ("payload=" + (Get-LockRaw))
Exit-E2eLock
Check 'T1.3 释放后锁文件消失' (-not (Test-Path -LiteralPath $lock))

Write-Host 'T2 有人持锁（活着的进程）→ 排队到超时，且不许删持有者的锁'
Reset-Lock
Write-FakeLock -HolderPid $me -Started (Get-Date) -Script 'selftest-live-holder'
$before = Get-LockRaw
$t = Timer { Enter-E2eLock -ScriptName 'selftest-contender' -TimeoutSec 2 -PollSec 1 }
Check 'T2.1 抢不到 → 返回 false（不假成功）' ($t.ok -eq $false)
Check 'T2.2 超时用时 ≥2s（确实排队了，不是立刻放弃）' ($t.ms -ge 1800) ("ms=" + $t.ms)
Check 'T2.3 持有者的锁文件原样保留（没被抢）' ((Get-LockRaw) -eq $before)

Write-Host 'T3 持有者 PID 已死 → 僵尸锁立即回收'
Reset-Lock
Write-FakeLock -HolderPid (New-DeadPid) -Started (Get-Date) -Script 'selftest-dead'
$t = Timer { Enter-E2eLock -ScriptName 'selftest-zombie' -TimeoutSec 5 -PollSec 1 }
Check 'T3.1 返回 true' ($t.ok -eq $true)
Check 'T3.2 回收很快（<3s，不等满 5s 超时）' ($t.ms -lt 3000) ("ms=" + $t.ms)
$h = $null; try { $h = Get-LockRaw | ConvertFrom-Json } catch {}
Check 'T3.3 锁已归自己' ($null -ne $h -and [int]$h.pid -eq $me)
Exit-E2eLock

Write-Host 'T4 PID 被复用护栏（锁创建时刻早于同 PID 进程的启动时刻）'
Reset-Lock
Write-FakeLock -HolderPid $me -Started $myStart.AddSeconds(-600) -Script 'selftest-pidreuse'
$t = Timer { Enter-E2eLock -ScriptName 'selftest-pidreuse' -TimeoutSec 5 -PollSec 1 }
Check 'T4.1 判定为复用 PID → 回收并拿到锁' ($t.ok -eq $true -and $t.ms -lt 3000) ("ms=" + $t.ms)
Exit-E2eLock

Write-Host 'T5 超龄锁回收（隔离复用判据：用一个「启动时刻早于锁创建时刻」的长寿进程）'
Reset-Lock
$long = Get-LongLivedProcess
if ($null -eq $long) {
    Write-Host '  [SKIP] T5 —— 找不到存活 >120s 的进程来做隔离' -ForegroundColor Yellow
} else {
    Write-FakeLock -HolderPid $long.Id -Started $long.StartTime -Script 'selftest-stale'
    $t = Timer { Enter-E2eLock -ScriptName 'selftest-stale' -TimeoutSec 5 -StaleSec 60 -PollSec 1 }
    Check 'T5.1 超龄 → 回收并拿到锁' ($t.ok -eq $true -and $t.ms -lt 3000) ("ms=" + $t.ms + " holder=$($long.Id)")
    Exit-E2eLock
}

Write-Host 'T6 锁文件在、内容读不全 → 宽限期内不回收（回归：原实现会删掉正在写入的锁）'
Reset-Lock
[System.IO.File]::WriteAllText($lock, '', [Text.Encoding]::UTF8)
$t = Timer { try { Enter-E2eLock -ScriptName 'selftest-partial' -TimeoutSec 2 -PollSec 1 -GraceSec $GraceSec } catch { Write-Host ("  [note] Enter-E2eLock 抛错（旧版没有 GraceSec 参数？）：" + $_.Exception.Message) -ForegroundColor Yellow; $false } }
Check 'T6.1 宽限期内返回 false（没有假成功）' ($t.ok -eq $false)
# 注意判据要用**内容**，不能用 Test-Path：原实现会先删再自己 CreateNew，
# 于是"文件还在"这条会假绿 —— 真正该问的是"那份写了一半的锁有没有被人夺走"。
$rawNow = Get-LockRaw
Check 'T6.2 那份写了一半的锁仍原样（没被删、也没被夺走）' ([string]::IsNullOrEmpty($rawNow)) ("实际内容=" + $rawNow)
[System.IO.File]::Delete($lock)
Write-Host 'T6b 同一份不可解析的锁「超龄」→ 才当残骸回收'
[System.IO.File]::WriteAllText($lock, '', [Text.Encoding]::UTF8)
[System.IO.File]::SetLastWriteTime($lock, (Get-Date).AddSeconds(-($GraceSec + 30)))
$t = Timer { try { Enter-E2eLock -ScriptName 'selftest-partial-old' -TimeoutSec 5 -PollSec 1 -GraceSec $GraceSec } catch { Write-Host ("  [note] Enter-E2eLock 抛错（旧版没有 GraceSec 参数？）：" + $_.Exception.Message) -ForegroundColor Yellow; $false } }
Check 'T6b.1 超龄残骸 → 回收并拿到锁' ($t.ok -eq $true -and $t.ms -lt 3000) ("ms=" + $t.ms)
Exit-E2eLock

Write-Host 'T7 继承标记要复核（回归：残留环境变量不许"假持锁"）'
Reset-Lock
Write-FakeLock -HolderPid $me -Started (Get-Date) -Script 'selftest-parent'
$parentRaw = Get-LockRaw
$env:CRYSTAL_E2E_LOCK_HELD_BY = "pid=$me script=selftest-parent"
$t = Timer { Enter-E2eLock -ScriptName 'selftest-child' -TimeoutSec 3 -PollSec 1 }
Check 'T7.1 父进程真持锁 → 复用成功' ($t.ok -eq $true)
Check 'T7.2 复用是"立刻"的（<1s，没排队）' ($t.ms -lt 1000) ("ms=" + $t.ms)
Exit-E2eLock
Check 'T7.3 子进程释放时不许删父进程的锁' ((Get-LockRaw) -eq $parentRaw)

Reset-Lock
Write-FakeLock -HolderPid $me -Started (Get-Date) -Script 'selftest-other-holder'
$env:CRYSTAL_E2E_LOCK_HELD_BY = "pid=$(New-DeadPid) script=selftest-dead-parent"
$t = Timer { Enter-E2eLock -ScriptName 'selftest-orphan-child' -TimeoutSec 2 -PollSec 1 }
Check 'T7.4 父进程已死 → 忽略继承标记、正常排队（返回 false）' ($t.ok -eq $false)
Check 'T7.5 也没有顺手把别人的锁删掉' ((Get-LockRaw) -ne $null)

Reset-Lock
$env:CRYSTAL_E2E_LOCK_HELD_BY = "pid=$(New-DeadPid) script=selftest-dead-parent"
$t = Timer { Enter-E2eLock -ScriptName 'selftest-orphan-acquire' -TimeoutSec 3 -PollSec 1 }
$h = $null; try { $h = Get-LockRaw | ConvertFrom-Json } catch {}
Check 'T7.6 无锁可继承时 → 正常拿到锁' ($t.ok -eq $true -and $null -ne $h -and [int]$h.pid -eq $me)
Exit-E2eLock

Write-Host 'T8 跨进程串行性（3 个真子进程抢同一把锁，临界区不许重叠）'
# 为什么必须有这条：T1–T7 都在同一个进程里，证明不了"两个**进程**不会同时进临界区"。
# 建锁的原子性（tmp+Move）与回收判据只有在真并发下才被压到。子脚本由本夹具现场生成，
# 免得手抄出两份逻辑。
Reset-Lock
$childScript = Join-Path $sandbox 'child.ps1'
$childBody = @'
param([string]$LockScript, [int]$HoldMs, [string]$LogPath)
. $LockScript
if (-not (Enter-E2eLock -ScriptName 'selftest-child' -TimeoutSec 30 -PollSec 1)) {
    [System.IO.File]::AppendAllText($LogPath, "timeout`n"); exit 2
}
[System.IO.File]::AppendAllText($LogPath, ("enter " + [DateTime]::UtcNow.Ticks + "`n"))
Start-Sleep -Milliseconds $HoldMs
[System.IO.File]::AppendAllText($LogPath, ("exit " + [DateTime]::UtcNow.Ticks + "`n"))
Exit-E2eLock
'@
[System.IO.File]::WriteAllText($childScript, $childBody, (New-Object System.Text.UTF8Encoding($false)))
$serialLog = Join-Path $sandbox 'serial.log'
$pwshPath = (Get-Process -Id $PID).Path
$kids = @()
foreach ($i in 1..3) {
    $kidErr = Join-Path $sandbox ("kid{0}.err" -f $i)
    $kids += Start-Process -FilePath $pwshPath -WindowStyle Hidden -PassThru `
        -RedirectStandardError $kidErr `
        -ArgumentList '-NoProfile', '-File', $childScript, $LockScriptPath, '350', $serialLog
}
foreach ($k in $kids) { $k.WaitForExit(120000) | Out-Null }
$lines = @(Get-Content -LiteralPath $serialLog -EA SilentlyContinue)
# 子进程 stderr 只在用例失败时才有用（成功时是空的）；失败信息里带上它，省得再猜
$kidErrs = @(Get-ChildItem -LiteralPath $sandbox -Filter 'kid*.err' -EA SilentlyContinue |
    Where-Object { $_.Length -gt 0 } | ForEach-Object { $_.Name + ':' + (Get-Content -LiteralPath $_.FullName -Raw) })
$events = @()
foreach ($l in $lines) {
    $p = $l -split ' '
    if ($p.Count -eq 2) { $events += [pscustomobject]@{ k = $p[0]; t = [long]$p[1] } }
}
$depth = 0; $maxDepth = 0
foreach ($e in ($events | Sort-Object t)) {
    if ($e.k -eq 'enter') { $depth++; if ($depth -gt $maxDepth) { $maxDepth = $depth } } else { $depth-- }
}
Check 'T8.1 三个子进程都进过临界区（3 进 3 出、无超时）' `
    ($events.Count -eq 6 -and @($lines | Where-Object { $_ -eq 'timeout' }).Count -eq 0) `
    ("events=" + $events.Count + " lines=" + ($lines -join '|') + " err=" + ($kidErrs -join ' / '))
Check 'T8.2 任一时刻最多一个持有者（临界区不重叠）' ($maxDepth -eq 1) ("maxDepth=" + $maxDepth)
Check 'T8.3 三个子进程都正常退出了' (@($kids | Where-Object { -not $_.HasExited }).Count -eq 0)

# ---------------- T9 接入覆盖面（「有锁但某个入口没接」也要变红） ----------------
# 这一条是「别再用 for(i=1..8) 撞干净窗口」能不能兑现的关键：只要有一个会起客户端的脚本没走锁，
# 它和走了锁的脚本并行跑就照样撞 result=4。判据与批量接入器共用 e2e_lock.ps1 里的同一对函数。
if (-not (Get-Command Get-E2eClientScripts -EA SilentlyContinue)) {
    Write-Host '  [SKIP] T9 —— 被测锁脚本没提供 Get-E2eClientScripts（A/B 对照旧版时的正常情况）' -ForegroundColor DarkGray
} else {
    $repoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
    $targets = @(Get-E2eClientScripts -RepoRoot $repoRoot)
    $missing = @()
    foreach ($t in $targets) {
        $c = Test-E2eLockEnrollment -Path $t.Path
        if (-not $c.ok) { $missing += ("{0}（{1}）" -f $t.Name, ($c.reasons -join '、')) }
    }
    Check 'T9.1 每个实机入口都 dot-source 了锁、有 Enter 且有 Exit' ($targets.Count -gt 0 -and $missing.Count -eq 0) `
        ("扫描 {0} 个；缺锁定接入：{1}" -f $targets.Count, ($missing -join ' | '))

    # 阳性对照：造一个「会起客户端但没接入锁」的脚本，覆盖面判据必须把它判为不合规
    $fakeRepo = Join-Path $sandbox 'fake_repo'
    $fakeAcc = Join-Path $fakeRepo 'tools\acceptance'
    New-Item -ItemType Directory -Path $fakeAcc -Force | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $fakeAcc 'no_lock.ps1'),
        "Start-Process client.exe -ArgumentList '--e2e-user','test','--e2e-pass','123456'`n",
        (New-Object System.Text.UTF8Encoding($false)))
    $fakeFound = @(Get-E2eClientScripts -RepoRoot $fakeRepo)
    $fakeCheck = if ($fakeFound.Count -gt 0) { Test-E2eLockEnrollment -Path $fakeFound[0].Path } else { $null }
    Check 'T9.2 阳性对照：没接入锁的实机入口必须被判不合规' `
        ($fakeFound.Count -eq 1 -and $null -ne $fakeCheck -and -not $fakeCheck.ok) `
        ("扫到 {0} 个；判语={1}" -f $fakeFound.Count, $(if ($null -ne $fakeCheck) { $fakeCheck.reasons -join '、' } else { '（没扫到）' }))
}

Reset-Lock
Remove-Item -LiteralPath $sandbox -Recurse -Force -EA SilentlyContinue

Write-Host ''
Write-Host ("结果：{0} passed / {1} failed（沙箱 {2} 已清理）" -f $pass.Count, $fail.Count, $sandbox)
if ($fail.Count -gt 0) {
    Write-Host ("失败项：" + ($fail -join '; ')) -ForegroundColor Red
    exit 1
}
exit 0
