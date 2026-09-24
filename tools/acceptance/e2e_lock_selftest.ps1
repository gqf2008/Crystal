#Requires -Version 5.1
<#
.SYNOPSIS
  e2e 实机互斥锁的**门禁自检**：串行性、僵尸锁回收、接入覆盖面，各带阳性对照。

.DESCRIPTION
  实机资源（客户端 + e2e 账号 + 本地服务端）是单例，靠 `e2e_lock.ps1` 串行化。但锁本身也会坏：
    A) 锁没生效（两个进程同时进）→ 登录假红照旧，只是换了个地方偶发；
    B) 持有者被强杀后锁不释放 → 后续谁都得等 30 分钟 StaleSec，比假红更糟；
    C) 某个实机入口忘了接入 → 与接入的脚本并行跑，照样撞 result=4。
  本脚本就是钉这三条，并且**带阳性对照**（去掉守卫必须变红），否则「全绿」可能只是断言写空了。

  用例（退出码：0 全过，1 有用例红）：
    T1 串行性       ：N 个独立子进程各自 Enter→hold 2s→Exit，断言**时间区间两两不重叠**；
    T2 僵尸锁回收   ：子进程持锁后被强杀（不释放）→ 另一个子进程必须在 ~PollSec×3 内拿到锁；
    T2b PID 复用    ：同 PID 的进程**晚于**锁创建才启动 ⇒ 不算持有者，必须立即回收
                      （缺这条：死锁看起来"仍被占着"，队列白等到 StaleSec=1800s）；
                      阳性对照＝活持有者且未超龄必须**不**回收、也不许删它的锁；
    T2c 嵌套继承    ：父脚本持锁时，子脚本必须**复用**父进程的锁（`CRYSTAL_E2E_LOCK_HELD_BY`）——
                      缺这条 `scripts/run_real_e2e.ps1 -IncludeInteractSweep` 会自锁到超时；
    T2d 残留标记    ：继承标记指向的持有者已退出/与锁文件不一致时必须**忽略标记、照常抢锁**——
                      环境变量会沿进程树继承，残留标记若被当真就会「假持锁」（没有任何串行化）；
    T3 接入覆盖面   ：`Get-E2eClientScripts` 认出来的每个实机入口都必须 dot-source 并有 Enter/Exit；
    T4 阳性对照     ：T4a 子进程**不拿锁**时 T1 的串行性断言必须变红；T4b 「会起客户端但没接入」的
                      临时脚本必须被判为不合规。

.PARAMETER Rounds
  T1 的子进程个数，默认 3（每个 hold 2s，总耗时约 6s 起）。

.PARAMETER HoldSec
  子进程持锁时长（秒），默认 2。

.PARAMETER SkipPositiveControl
  跳过 T4（不推荐：没有阳性对照的绿说明不了断言有效）。

.EXAMPLE
  pwsh tools/acceptance/e2e_lock_selftest.ps1
#>
[CmdletBinding()]
param(
    [string]$RepoRoot = '',
    [int]$Rounds = 3,
    [int]$HoldSec = 2,
    [string]$WorkDir = '',
    [switch]$SkipPositiveControl
)
$ErrorActionPreference = 'Stop'
if (-not $RepoRoot) { $RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path }
. "$PSScriptRoot\e2e_lock.ps1"
# 本自检必须从「没有持锁」的状态出发：若它自己被一个持锁的父进程拉起来
# （CRYSTAL_E2E_LOCK_HELD_BY 已被父进程设上），子进程会走复用路径，T1/T4 的串行性断言会假绿。
Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue

$lockPath = Get-E2eLockPath
$lockScript = Join-Path $PSScriptRoot 'e2e_lock.ps1'
$pwshExe = (Get-Command pwsh -EA SilentlyContinue).Source
if (-not $pwshExe) { $pwshExe = (Get-Process -Id $PID).Path }
if (-not $WorkDir) { $WorkDir = Join-Path $env:TEMP ('crystal_e2e_lock_selftest_' + (Get-Date -Format 'HHmmss')) }
New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

$failures = @()
$notes = @()
function Add-Fail([string]$what, [string]$why) { $script:failures += ("[{0}] {1}" -f $what, $why) }

# ---------------- 子进程脚本（只在本自检里用；父进程生成，避免手抄） ----------------
$childPath = Join-Path $WorkDir 'lock_child.ps1'
$childSrc = @'
param(
    [string]$LockScript,
    [string]$OutFile,
    [int]$HoldSec = 2,
    [string]$Name = 'child',
    [int]$TimeoutSec = 120,
    [switch]$NoLock
)
$ErrorActionPreference = 'Stop'
. $LockScript
$got = $true
if (-not $NoLock) {
    $got = [bool](Enter-E2eLock -ScriptName $Name -TimeoutSec $TimeoutSec)
}
$t0 = [DateTime]::UtcNow
Start-Sleep -Seconds $HoldSec
$t1 = [DateTime]::UtcNow
if ($got -and -not $NoLock) { Exit-E2eLock }
(@{ name = $Name; got = $got; enter = $t0.ToString('o'); exit = $t1.ToString('o') } |
    ConvertTo-Json -Compress) | Set-Content -LiteralPath $OutFile -Encoding UTF8
if (-not $got) { exit 4 }
exit 0
'@
[System.IO.File]::WriteAllText($childPath, $childSrc, (New-Object System.Text.UTF8Encoding($false)))

function Invoke-ChildBatch {
    <# 并行起 $N 个子进程抢同一把锁，回收它们的区间读数。#>
    param([int]$N, [int]$Hold, [string]$Tag, [switch]$NoLock)
    $procs = @()
    for ($i = 1; $i -le $N; $i++) {
        $out = Join-Path $WorkDir ('{0}_{1}.json' -f $Tag, $i)
        Remove-Item -LiteralPath $out -Force -EA SilentlyContinue
        # 注意：不要用 $args 当局部变量名（它是 PowerShell 的自动变量）
        $childArgs = @('-NoProfile', '-NoLogo', '-File', $childPath, '-LockScript', $lockScript,
            '-OutFile', $out, '-HoldSec', "$Hold", '-Name', ("$Tag-$i"))
        if ($NoLock) { $childArgs += '-NoLock' }
        $procs += [pscustomobject]@{
            i   = $i
            out = $out
            p   = Start-Process -FilePath $pwshExe -ArgumentList $childArgs -PassThru -WindowStyle Hidden
        }
    }
    foreach ($x in $procs) {
        $null = $x.p.WaitForExit(180000)
    }
    $rows = @()
    foreach ($x in $procs) {
        if (-not (Test-Path -LiteralPath $x.out)) { continue }
        $j = Get-Content -LiteralPath $x.out -Raw | ConvertFrom-Json
        $rows += [pscustomobject]@{
            name  = $j.name
            got   = [bool]$j.got
            enter = [datetime]::Parse($j.enter, [cultureinfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::RoundtripKind)
            exit  = [datetime]::Parse($j.exit, [cultureinfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::RoundtripKind)
        }
    }
    , $rows
}

function Measure-Serialization {
    <# 返回重叠（秒）与缺口说明；0 表示严格串行。#>
    param($Rows)
    $sorted = @($Rows | Sort-Object enter)
    $overlap = 0.0
    $detail = @()
    for ($i = 1; $i -lt $sorted.Count; $i++) {
        $ov = ($sorted[$i - 1].exit - $sorted[$i].enter).TotalSeconds
        if ($ov -gt 0.05) {
            $detail += ("{0} 与 {1} 重叠 {2:N2}s" -f $sorted[$i - 1].name, $sorted[$i].name, $ov)
            if ($ov -gt $overlap) { $overlap = $ov }
        }
    }
    [pscustomobject]@{ overlap_sec = $overlap; detail = $detail; rows = $sorted }
}

# ---------------- T0 语法解析（批量接入是机械插入，最容易插出语法错） ----------------
Write-Host '== T0 语法解析：所有实机入口 + 锁本体必须能被 PowerShell 解析 =='
$parseTargets = @(Get-E2eClientScripts -RepoRoot $RepoRoot | ForEach-Object { $_.Path })
$parseTargets += @(
    (Join-Path $PSScriptRoot 'e2e_lock.ps1'),
    (Join-Path $PSScriptRoot 'e2e_lock_selftest.ps1'),
    (Join-Path $PSScriptRoot 'enroll_e2e_lock.ps1')
)
$parseBad = @()
foreach ($p in $parseTargets) {
    if (-not (Test-Path -LiteralPath $p)) { continue }
    $errs = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile($p, [ref]$null, [ref]$errs)
    if ($errs.Count -gt 0) { $parseBad += ("{0}: {1}" -f (Split-Path -Leaf $p), (($errs | ForEach-Object { $_.Message }) -join '; ')) }
}
if ($parseBad.Count -gt 0) {
    Add-Fail 'T0' ("{0} 个文件解析失败：`n      {1}" -f $parseBad.Count, ($parseBad -join "`n      "))
} else {
    Write-Host ("   T0 通过：{0} 个文件解析无错" -f $parseTargets.Count)
}

# ---------------- T1 串行性 ----------------
Write-Host ("== T1 串行性：{0} 个子进程抢同一把锁（各持 {1}s） ==" -f $Rounds, $HoldSec)
$t1 = Invoke-ChildBatch -N $Rounds -Hold $HoldSec -Tag 't1'
if ($t1.Count -ne $Rounds) { Add-Fail 'T1' ("只回收 {0}/{1} 个子进程的读数（读数缺失时「不重叠」不算通过）" -f $t1.Count, $Rounds) }
$notGot = @($t1 | Where-Object { -not $_.got })
if ($notGot.Count -gt 0) { Add-Fail 'T1' ("有 {0} 个子进程没拿到锁（超时）" -f $notGot.Count) }
$m1 = Measure-Serialization -Rows $t1
foreach ($r in $m1.rows) { Write-Host ("   {0,-8} 持锁 {1:HH:mm:ss.fff} → {2:HH:mm:ss.fff}" -f $r.name, $r.enter.ToLocalTime(), $r.exit.ToLocalTime()) }
if ($m1.overlap_sec -gt 0.05) {
    Add-Fail 'T1' ("区间重叠 {0:N2}s —— 锁没有串行化：{1}" -f $m1.overlap_sec, ($m1.detail -join '; '))
} elseif (($t1.Count -eq $Rounds) -and ($notGot.Count -eq 0)) {
    Write-Host '   T1 通过：区间两两不重叠（严格串行）'
}

# ---------------- T2 僵尸锁回收 ----------------
Write-Host '== T2 僵尸锁回收：持锁者被强杀后，下一个必须能拿到锁 =='
$zOut = Join-Path $WorkDir 't2_zombie.json'
Remove-Item -LiteralPath $zOut -Force -EA SilentlyContinue
$zArgs = @('-NoProfile', '-NoLogo', '-File', $childPath, '-LockScript', $lockScript, '-OutFile', $zOut,
    '-HoldSec', '120', '-Name', 't2-zombie')
$z = Start-Process -FilePath $pwshExe -ArgumentList $zArgs -PassThru -WindowStyle Hidden
$held = $false
for ($i = 1; $i -le 20; $i++) {
    Start-Sleep -Milliseconds 500
    $info = Get-E2eLockInfo
    if ($info.held -and [int]$info.pid -eq $z.Id) { $held = $true; break }
}
if (-not $held) { Add-Fail 'T2' '子进程 10s 内没拿到锁（前置不成立）'; }
Stop-Process -Id $z.Id -Force -EA SilentlyContinue
$infoAfter = Get-E2eLockInfo
Write-Host ("   已强杀持锁子进程 PID={0}；锁文件仍在（pid={1} 存活={2}）——这是要验的场景" -f `
    $z.Id, $infoAfter.pid, $infoAfter.owner_alive)
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$t2 = Invoke-ChildBatch -N 1 -Hold 1 -Tag 't2'
$sw.Stop()
if ($t2.Count -eq 0 -or -not $t2[0].got) {
    Add-Fail 'T2' '僵尸锁没被回收：下一个子进程拿不到锁'
} elseif ($sw.Elapsed.TotalSeconds -gt 15) {
    Add-Fail 'T2' ("回收太慢：{0:N1}s（应 ≤ PollSec×3 = 15s）" -f $sw.Elapsed.TotalSeconds)
} else {
    Write-Host ("   T2 通过：僵尸锁在 {0:N1}s 内被回收" -f $sw.Elapsed.TotalSeconds)
}
Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue

# ---------------- T2b PID 复用护栏 ----------------
Write-Host '== T2b PID 复用：同 PID 的进程晚于锁创建才启动 ⇒ 不算持有者（缺这条会白等 StaleSec=1800s） =='
$reuseSleeper = Start-Process -FilePath $pwshExe -ArgumentList @('-NoProfile', '-NoLogo', '-Command', 'Start-Sleep 120') -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
try {
    # (a) 伪造「PID 被复用」：pid 是**活着的**新进程，但锁创建时间远早于它的启动时刻
    Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue
    @{ pid = $reuseSleeper.Id; script = 't2b-fake-reused'; host = $env:COMPUTERNAME
       started = (Get-Date).AddSeconds(-600).ToString('o') } | ConvertTo-Json -Compress |
        Set-Content -LiteralPath $lockPath -NoNewline
    $swb = [System.Diagnostics.Stopwatch]::StartNew()
    $gotB = [bool](Enter-E2eLock -ScriptName 't2b' -TimeoutSec 5 -PollSec 1)
    $swb.Stop()
    if ($gotB -and $swb.Elapsed.TotalSeconds -le 3) {
        Write-Host ("   T2b-a 通过：PID 复用锁 {0:N1}s 内被回收（活进程 pid={1}，但它的启动时刻晚于锁创建）" -f `
            $swb.Elapsed.TotalSeconds, $reuseSleeper.Id)
        Exit-E2eLock
    } else {
        Add-Fail 'T2b-a' ("PID 复用锁没被回收（等 {0:N1}s，got={1}）——死锁会看起来仍被占着" -f $swb.Elapsed.TotalSeconds, $gotB)
    }

    # (b) 阳性对照：**活持有者且未超龄**（锁创建晚于该进程启动）必须不回收、排队到超时，且不许删它的锁
    Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue
    @{ pid = $reuseSleeper.Id; script = 't2b-live-holder'; host = $env:COMPUTERNAME
       started = (Get-Date).ToString('o') } | ConvertTo-Json -Compress |
        Set-Content -LiteralPath $lockPath -NoNewline
    $swb2 = [System.Diagnostics.Stopwatch]::StartNew()
    $gotB2 = [bool](Enter-E2eLock -ScriptName 't2b-ctl' -TimeoutSec 4 -PollSec 1)
    $swb2.Stop()
    $stillThere = Test-Path -LiteralPath $lockPath
    if ((-not $gotB2) -and $stillThere) {
        Write-Host ("   T2b-b 通过：活持有者（未超龄）没被抢（等 {0:N1}s 后按超时返回 $false），锁文件未被删" -f `
            $swb2.Elapsed.TotalSeconds)
    } else {
        Add-Fail 'T2b-b' ("活持有者被误抢或锁被误删（got={0}，锁文件仍在={1}）" -f $gotB2, $stillThere)
    }
} finally {
    Stop-Process -Id $reuseSleeper.Id -Force -EA SilentlyContinue
    Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue
    Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue
}

# ---------------- T2c 嵌套继承（父持锁 → 子脚本复用） ----------------
Write-Host '== T2c 嵌套继承：父脚本持锁时子脚本必须复用（否则 run_real_e2e -IncludeInteractSweep 会自锁到超时） =='
$nestedChild = Join-Path $WorkDir 'nested_child.ps1'
$nestedSrc = @'
param(
    [string]$LockScript,
    [string]$OutFile,
    [string]$GrandChildScript,
    [string]$ScratchDir
)
$ErrorActionPreference = 'Stop'
# 自己在进程内解析 pwsh 路径：不要把它当参数从外面传进来——`Start-Process -ArgumentList`
# **不会**给含空格的值加引号（`C:\Program Files\...` 会被拆成两个参数），本用例踩过这个坑。
$pwshExe = (Get-Process -Id $PID).Path
if (-not $pwshExe) { $pwshExe = (Get-Command pwsh).Source }
. $LockScript
$got = [bool](Enter-E2eLock -ScriptName 't2c-parent' -TimeoutSec 60)
$gcOut = Join-Path $ScratchDir 't2c_grandchild.json'
Remove-Item -LiteralPath $gcOut -Force -EA SilentlyContinue
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$gcErr = Join-Path $ScratchDir 't2c_grandchild.err'
$p = Start-Process -FilePath $pwshExe -WindowStyle Hidden -PassThru `
    -RedirectStandardError $gcErr -ArgumentList @(
        '-NoProfile', '-NoLogo', '-File', $GrandChildScript, '-LockScript', $LockScript,
        '-OutFile', $gcOut, '-HoldSec', '1', '-Name', 't2c-grandchild', '-TimeoutSec', '20')
$null = $p.WaitForExit(90000)
$sw.Stop()
$gcGot = $false
if (Test-Path -LiteralPath $gcOut) {
    $j = Get-Content -LiteralPath $gcOut -Raw | ConvertFrom-Json
    $gcGot = [bool]$j.got
}
if ($got) { Exit-E2eLock }
(@{ child_got = $got; grandchild_got = $gcGot; elapsed_sec = [math]::Round($sw.Elapsed.TotalSeconds, 2) } |
    ConvertTo-Json -Compress) | Set-Content -LiteralPath $OutFile -Encoding UTF8
exit 0
'@
[System.IO.File]::WriteAllText($nestedChild, $nestedSrc, (New-Object System.Text.UTF8Encoding($false)))
$nOut = Join-Path $WorkDir 't2c_parent.json'
Remove-Item -LiteralPath $nOut -Force -EA SilentlyContinue
$nErr = Join-Path $WorkDir 't2c_parent.err'
$nLog = Join-Path $WorkDir 't2c_parent.log'
$nProc = Start-Process -FilePath $pwshExe -WindowStyle Hidden -PassThru `
    -RedirectStandardError $nErr -RedirectStandardOutput $nLog -ArgumentList @(
        '-NoProfile', '-NoLogo', '-File', $nestedChild, '-LockScript', $lockScript, '-OutFile', $nOut,
        '-GrandChildScript', $childPath, '-ScratchDir', $WorkDir)
$null = $nProc.WaitForExit(120000)
$nested = $null
if (Test-Path -LiteralPath $nOut) { $nested = Get-Content -LiteralPath $nOut -Raw | ConvertFrom-Json }
if ($null -eq $nested -or -not $nested.child_got) {
    $why = ''
    if ($null -eq $nested) {
        $why = '；子进程没写出结论文件，stderr 末尾：'
        if (Test-Path -LiteralPath $nErr) { $why += ((Get-Content -LiteralPath $nErr | Select-Object -Last 3) -join ' / ') }
    }
    Add-Fail 'T2c' ("嵌套场景的父（持锁）子进程没拿到锁（前置不成立）" + $why)
} elseif (-not $nested.grandchild_got) {
    Add-Fail 'T2c' ("子脚本没复用父进程的锁：它排队等到自己超时（等 {0:N1}s，got=False）——" -f $nested.elapsed_sec +
        'run_real_e2e.ps1 -IncludeInteractSweep 会因此自锁')
} elseif ($nested.elapsed_sec -gt 10) {
    Add-Fail 'T2c' ("子脚本拿到锁了但等太久（{0:N1}s）——不像复用（期望 ≈ 子进程启动耗时），像是在等别人释放" -f $nested.elapsed_sec)
} else {
    Write-Host ("   T2c 通过：父持锁期间子脚本 {0:N1}s 内复用成功（未排队、未自锁）" -f $nested.elapsed_sec)
}
Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue
Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue

# ---------------- T2d 残留继承标记不得造成假持锁 ----------------
Write-Host '== T2d 残留继承标记：标记指向的持有者已退出/不匹配时，必须改为正常抢锁（不能假持锁） =='
$ghostChild = Join-Path $WorkDir 'ghost_token_child.ps1'
$ghostSrc = @'
param(
    [string]$LockScript,
    [string]$OutFile,
    [string]$Token,
    [int]$TimeoutSec = 4
)
$ErrorActionPreference = 'Stop'
# 模拟"父进程留下的继承标记"（会沿进程树继承的那种）
$env:CRYSTAL_E2E_LOCK_HELD_BY = $Token
. $LockScript
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$got = [bool](Enter-E2eLock -ScriptName 't2d-ghost' -TimeoutSec $TimeoutSec -PollSec 1)
$sw.Stop()
if ($got) { Exit-E2eLock }
(@{ got = $got; elapsed_sec = [math]::Round($sw.Elapsed.TotalSeconds, 2) } |
    ConvertTo-Json -Compress) | Set-Content -LiteralPath $OutFile -Encoding UTF8
exit 0
'@
[System.IO.File]::WriteAllText($ghostChild, $ghostSrc, (New-Object System.Text.UTF8Encoding($false)))
function Invoke-Ghost([string]$Token, [int]$Timeout) {
    $out = Join-Path $WorkDir ('t2d_{0}.json' -f ([guid]::NewGuid().ToString('N').Substring(0, 6)))
    $p = Start-Process -FilePath $pwshExe -WindowStyle Hidden -PassThru -ArgumentList @(
        '-NoProfile', '-NoLogo', '-File', $ghostChild, '-LockScript', $lockScript, '-OutFile', $out,
        '-Token', $Token, '-TimeoutSec', "$Timeout")
    $null = $p.WaitForExit(90000)
    if (Test-Path -LiteralPath $out) { Get-Content -LiteralPath $out -Raw | ConvertFrom-Json } else { $null }
}
# (a) 真持有者在场：残留标记（死 pid）不得让子进程"假持锁"
$ghHolder = Start-Process -FilePath $pwshExe -WindowStyle Hidden -PassThru -ArgumentList @(
    '-NoProfile', '-NoLogo', '-File', $childPath, '-LockScript', $lockScript,
    '-OutFile', (Join-Path $WorkDir 't2d_holder.json'), '-HoldSec', '120', '-Name', 't2d-holder')
$heldNow = $false
for ($i = 1; $i -le 20; $i++) {
    Start-Sleep -Milliseconds 500
    $info = Get-E2eLockInfo
    if ($info.held -and [int]$info.pid -eq $ghHolder.Id) { $heldNow = $true; break }
}
if (-not $heldNow) {
    Add-Fail 'T2d' '前置不成立：造不出「真持有者在场」的局面'
    Stop-Process -Id $ghHolder.Id -Force -EA SilentlyContinue
} else {
    $g1 = Invoke-Ghost -Token 'pid=999999 script=ghost-of-dead-holder' -Timeout 4
    if ($null -eq $g1) {
        Add-Fail 'T2d-a' 'Ghost 子进程没写出结论'
    } elseif ($g1.got) {
        Add-Fail 'T2d-a' ("残留继承标记让子进程假持锁了（{0:N1}s 内就返回成功）——真持有者还在，它必须排队" -f $g1.elapsed_sec)
    } else {
        Write-Host ("   T2d-a 通过：残留标记被忽略，子进程照常排队（等 {0:N1}s 后超时返回 false）" -f $g1.elapsed_sec)
    }
    # (b) 真持有者被强杀（锁文件成僵尸）后，带残留标记的子进程应能正常回收并拿到锁
    Stop-Process -Id $ghHolder.Id -Force -EA SilentlyContinue
    $g2 = Invoke-Ghost -Token 'pid=999999 script=ghost-of-dead-holder' -Timeout 10
    if ($null -eq $g2) {
        Add-Fail 'T2d-b' 'Ghost 子进程（僵尸锁场景）没写出结论'
    } elseif (-not $g2.got) {
        Add-Fail 'T2d-b' '僵尸锁 + 残留标记时子进程拿不到锁：忽略标记后没走回正常抢锁路径'
    } else {
        Write-Host ("   T2d-b 通过：忽略残留标记后照常回收僵尸锁并拿到锁（{0:N1}s）" -f $g2.elapsed_sec)
    }
}
Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue
Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue

# ---------------- T3 接入覆盖面 ----------------
Write-Host '== T3 接入覆盖面：每个会起客户端的脚本都必须走锁 =='
$targets = @(Get-E2eClientScripts -RepoRoot $RepoRoot)
$violations = @()
foreach ($t in $targets) {
    $c = Test-E2eLockEnrollment -Path $t.Path
    if (-not $c.ok) { $violations += ("{0}: {1}" -f $t.Name, ($c.reasons -join '、')) }
}
Write-Host ("   扫描面 {0} 个脚本：已接入 {1}、缺锁 {2}" -f $targets.Count, ($targets.Count - $violations.Count), $violations.Count)
if ($violations.Count -gt 0) {
    Add-Fail 'T3' ("有 {0} 个实机入口没接入锁：`n      {1}" -f $violations.Count, ($violations -join "`n      "))
} else {
    Write-Host '   T3 通过：所有实机入口都 dot-source 了 e2e_lock.ps1 并有 Enter/Exit'
}

# ---------------- T4 阳性对照 ----------------
if ($SkipPositiveControl) {
    Write-Host '== T4 阳性对照：跳过（-SkipPositiveControl） =='
} else {
    Write-Host '== T4 阳性对照：去掉守卫必须变红 =='
    $t4 = Invoke-ChildBatch -N $Rounds -Hold $HoldSec -Tag 't4' -NoLock
    $m4 = Measure-Serialization -Rows $t4
    if (@($t4).Count -ne $Rounds) {
        Add-Fail 'T4a' ("阳性对照子进程读数不全（{0}/{1}）——对照无效，不能据此说 T1 有效" -f @($t4).Count, $Rounds)
    } elseif ($m4.overlap_sec -gt 0.05) {
        Write-Host ("   T4a 通过：不拿锁时区间确实重叠 {0:N2}s（{1}）——说明 T1 的断言不是空的" -f `
            $m4.overlap_sec, ($m4.detail -join '; '))
    } else {
        Add-Fail 'T4a' '不拿锁时也没有重叠 —— T1 的串行性断言无效（子进程没真并行 / hold 太短）'
    }
    $fakeRoot = Join-Path $WorkDir 'fake_repo'
    $fakeAcc = Join-Path $fakeRoot 'tools\acceptance'
    New-Item -ItemType Directory -Path $fakeAcc -Force | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $fakeAcc 'fake_no_lock.ps1'),
        "Start-Process -FilePath client.exe -ArgumentList '--e2e-user','test','--e2e-pass','123456'`n", (New-Object System.Text.UTF8Encoding($false)))
    $fakeFound = @(Get-E2eClientScripts -RepoRoot $fakeRoot)
    $fakeCheck = if ($fakeFound.Count -gt 0) { Test-E2eLockEnrollment -Path $fakeFound[0].Path } else { $null }
    if ($fakeFound.Count -eq 1 -and $null -ne $fakeCheck -and -not $fakeCheck.ok) {
        Write-Host ("   T4b 通过：没接入的实机脚本被判为不合规（{0}）" -f ($fakeCheck.reasons -join '、'))
    } else {
        Add-Fail 'T4b' '「会起客户端但没接入锁」的脚本没被判为不合规 —— 覆盖检查无效'
    }
}

# ---------------- 结论 ----------------
$exitCode = 0
if ($failures.Count -gt 0) { $exitCode = 1 }
$result = [ordered]@{
    gate         = [ordered]@{
        exit_code = $exitCode
        failures  = $failures
        t1  = [ordered]@{ rounds = $Rounds; hold_sec = $HoldSec; overlap_sec = $m1.overlap_sec
                          windows = @($m1.rows | ForEach-Object { [ordered]@{ name = $_.name; got = $_.got
                              enter = $_.enter.ToString('o'); exit = $_.exit.ToString('o') } }) }
        t2  = [ordered]@{ zombie_reclaim_sec = [math]::Round($sw.Elapsed.TotalSeconds, 2) }
        t2b = [ordered]@{ pid_reuse_reclaim_sec = [math]::Round($swb.Elapsed.TotalSeconds, 2)
                          live_holder_stolen = $gotB2 }
        t2c = if ($null -ne $nested) { [ordered]@{ child_got = $nested.child_got
                          grandchild_got = $nested.grandchild_got; reuse_sec = $nested.elapsed_sec } } else { $null }
        t2d = [ordered]@{ ghost_token_while_held = if ($null -ne $g1) { $g1.got } else { $null }
                          ghost_token_zombie_reclaim = if ($null -ne $g2) { $g2.got } else { $null } }
        t3  = [ordered]@{ scanned = $targets.Count; missing = $violations }
        t4  = if ($SkipPositiveControl) { $null } else { [ordered]@{ nolock_overlap_sec = $m4.overlap_sec } }
    }
    lock_file    = $lockPath
    generated_at = (Get-Date).ToString('o')
}
$jsonOut = Join-Path $PSScriptRoot 'e2e_lock_selftest_results.json'
$result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $jsonOut -Encoding UTF8

Write-Host ''
Write-Host ("===== e2e_lock_selftest: exit={0} fail={1} =====" -f $exitCode, $failures.Count)
if ($failures.Count -gt 0) { $failures | ForEach-Object { Write-Host ("  FAIL  " + $_) -ForegroundColor Red } }
Write-Host ("结论 JSON: {0}" -f $jsonOut)
Remove-Item -LiteralPath $lockPath -Force -EA SilentlyContinue
exit $exitCode
