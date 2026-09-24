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
    [switch]$NoLock
)
$ErrorActionPreference = 'Stop'
. $LockScript
$got = $true
if (-not $NoLock) {
    $got = [bool](Enter-E2eLock -ScriptName $Name -TimeoutSec 120)
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
