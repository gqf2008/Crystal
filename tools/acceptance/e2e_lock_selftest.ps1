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

# ---------------- T9 接入覆盖面（防"新增夹具忘拿锁"再回来） ----------------
Write-Host 'T9 接入覆盖面：每个会起客户端的脚本都必须走同一把锁'
function Get-E2eClientScript {
    <#
      判据：正文里出现任一特征即算「会起客户端」——
        `--e2e-user`（自动化登录账号）/ `client_bevy.exe` / `--real-net` / `--auto-enter`。
      不只看 `--e2e-user`：`l5r_ranged_projectile`、`l5t_minimize_survives` 这类不传 e2e 账号
      但照样起客户端的脚本，也必须接入锁（#3129 的覆盖口径就是"会起客户端"）。

      扫描面＝整仓 *.ps1 / *.bat / *.cmd（2026-09-25 扩：只认 *.ps1 时，一个
      `launch_client.bat` 起客户端能整体绕开锁与门禁，见 T9.3c/T9.3d），判据分档同
      e2e_lock.ps1 的 Get-E2eClientScripts，两份必须给同一结论（T9.4 钉真仓、T9.4b 钉假仓）。
    #>
    param([string]$Root)
    # 这三个是"锁自己的脚本"，不算实机入口（接入器正文里就写着 `--e2e-user` 之类的示例，
    # 不排除它就会被自己的判据扫进来）——与 e2e_lock.ps1 里的 Get-E2eClientScripts 保持同一份名单。
    # 同 e2e_lock.ps1 的共享判据（T9.4 要求两份名单一致）：工具/判据脚本只是**提到**这些字样，不是实机入口。
    $selfNames = @('e2e_lock.ps1', 'e2e_lock_selftest.ps1', 'enroll_e2e_lock.ps1', 'check_process_scope.ps1')
    $out = @()
    # 扫描面与 e2e_lock.ps1 的共享判据一致：**整个仓库**的 *.ps1/.bat/.cmd（排除 .git/target/node_modules）。
    # 写死目录清单会让新目录里的实机入口静默漏网（见 T9.1b/T9.3b）。
    $skipDirs = '\\(\.git|target|node_modules)\\'
    $exts = @('.ps1', '.bat', '.cmd')
    $files = @(Get-ChildItem -LiteralPath $Root -Recurse -File -EA SilentlyContinue |
        Where-Object { $exts -contains $_.Extension.ToLowerInvariant() -and $_.FullName -notmatch $skipDirs })
    $texts = @{}
    foreach ($f in $files) {
        if ($selfNames -contains $f.Name) { continue }
        $text = Get-Content -LiteralPath $f.FullName -Raw -EA SilentlyContinue
        if ($null -ne $text) { $texts[$f.FullName] = $text }
    }
    # 判据 A（直连入口）：正文自己带「起客户端」字样（与 e2e_lock.ps1 的共享判据同口径）。
    $direct = @()
    foreach ($f in $files) {
        if (-not $texts.ContainsKey($f.FullName)) { continue }
        $text = $texts[$f.FullName]
        if ($text -notmatch '--e2e-user|client_bevy\.exe|--real-net|--auto-enter') { continue }
        $kind = $f.Extension.ToLowerInvariant().TrimStart('.')
        $direct += [pscustomobject]@{
            Name    = $f.Name
            Path    = $f.FullName
            Kind    = $kind
            Direct  = $true
            Armed   = $(if ($kind -eq 'ps1') { ($text -match 'e2e_lock\.ps1') -and ($text -match 'Enter-E2eLock') } else { $false })
            # 有 Enter 还不够：早退路径（if (...) { exit 5 }）会把锁留到下一个调用者才发现要回收，
            # 所以接入必须**成对**——有 Enter 就要有 Exit（缺它即红，见 T9.2）。
            HasExit = $(if ($kind -eq 'ps1') { $text -match 'Exit-E2eLock' } else { $false })
            Locked  = (($text -match 'e2e_lock\.ps1') -and ($text -match 'Enter-E2eLock') -and
                       ($text -match 'Exit-E2eLock'))
        }
    }
    $out += $direct
    # 判据 B（包装入口）：正文没有客户端字样，但**调用了**某个直连入口（典型：.bat 只写
    # `call pwsh -File ...\l5h_buy_item.ps1`）。只认调用形态，不认「只是提到」——否则把
    # run_real_e2e.ps1 写进默认参数值的 flag_coverage_check.ps1 会被误判成实机入口。
    $wrappers = @()
    foreach ($f in $files) {
        if (-not $texts.ContainsKey($f.FullName)) { continue }
        if (@($direct | Where-Object { $_.Path -eq $f.FullName }).Count -gt 0) { continue }
        $text = $texts[$f.FullName]
        $callsEntry = $false
        foreach ($d in $direct) {
            $esc = [regex]::Escape($d.Name)
            foreach ($ln in ($text -split "`r`n|`n")) {
                if ($ln -notmatch $esc) { continue }
                if ($ln -match '(?i)\bcall\b|\bpwsh\b|\bpowershell\b|Start-Process|Invoke-|-File\b|&') {
                    $callsEntry = $true
                    break
                }
            }
            if ($callsEntry) { break }
        }
        if (-not $callsEntry) { continue }
        $wrappers += [pscustomobject]@{
            Name    = $f.Name
            Path    = $f.FullName
            Kind    = $f.Extension.ToLowerInvariant().TrimStart('.')
            Direct  = $false
            Armed   = $false
            HasExit = $false
            Locked  = $false
        }
    }
    $out += $wrappers
    # 统一判定（非 *.ps1 直连入口 + 所有包装入口）：合规 = 点名 e2e_lock，或点名调用某个已接入入口。
    $enrolled = @($direct | Where-Object { $_.Locked } | ForEach-Object { $_.Name })
    foreach ($e in @($direct + $wrappers)) {
        if ($e.Direct -and $e.Kind -eq 'ps1') { continue }
        $text = $texts[$e.Path]
        $locked = (($text -match 'e2e_lock') -or
                   (@($enrolled | Where-Object { $text -match [regex]::Escape($_) }).Count -gt 0))
        # 非 PowerShell 入口没有 Enter/Exit 字样：合规与否统一由 Armed/HasExit/Locked 三处表达，
        # 这样 T9.2/T9.2b 的「必须成对」断言对它同样成立（缺委托即红）。
        $e.Armed = $locked
        $e.HasExit = $locked
        $e.Locked = $locked
    }
    $out
}
# 本自检没有 -RepoRoot 参数（它只认 -LockScriptPath），这里自己定位仓库根
$scanRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$clientScripts = @(Get-E2eClientScript -Root $scanRoot)

# 扫描面自证用的探针：优先用**被测锁脚本里的共享判据**（A/B 对照旧版锁脚本时它会缺失，回退本自检副本）。
# 这样用 `-LockScriptPath <旧版>` 跑，就能把「扫描面退回写死两目录」这种回归直接打成红。
function Get-E2eScriptSurfaceProbe {
    param([string]$Root)
    if (Get-Command Get-E2eClientScripts -EA SilentlyContinue) {
        return @(Get-E2eClientScripts -RepoRoot $Root | ForEach-Object {
            # 非 PowerShell 启动器（.bat/.cmd）正文里没有 Enter/Exit 字样，合规与否由 Locked 表达；
            # 旧版锁脚本没有这个字段，回退到 Enter/Exit 计数（A/B 对照旧版时的正常路径）。
            $hasLock = if ($_.PSObject.Properties['Locked']) { [bool]$_.Locked }
                       else { ([int]$_.EnterCount -gt 0 -and [int]$_.ExitCount -gt 0) }
            [pscustomobject]@{ Name = $_.Name; HasLock = $hasLock }
        })
    }
    @(Get-E2eClientScript -Root $Root | ForEach-Object {
        [pscustomobject]@{ Name = $_.Name; HasLock = ($_.Armed -and $_.HasExit) }
    })
}
$surfaceNow = @(Get-E2eScriptSurfaceProbe -Root $scanRoot)
$opsEntry = @($surfaceNow | Where-Object { $_.Name -eq 'package_windows_rehearsal.ps1' })
Check 'T9.1b 回归锁：旧清单之外的目录（tools\ops）里的实机入口也必须在扫描面内（退回写死两目录即红）' `
    ($opsEntry.Count -eq 1)

$notArmed = @($clientScripts | Where-Object { -not $_.Armed })
Check 'T9.1 判据没写空（真源至少认出 24 个会起客户端的脚本）' `
    ($clientScripts.Count -ge 24) ("count=" + $clientScripts.Count)
Check 'T9.2 每个实机入口都 dot-source 了 e2e_lock.ps1 且有 Enter-E2eLock' `
    ($notArmed.Count -eq 0) ("缺锁：" + (($notArmed | ForEach-Object { $_.Name }) -join ','))
$noExit = @($clientScripts | Where-Object { -not $_.HasExit })
Check 'T9.2b 每个实机入口都有 Exit-E2eLock（早退路径也要释放，不能只靠「持有者已死」兜底）' `
    ($noExit.Count -eq 0) ("缺 Exit：" + (($noExit | ForEach-Object { $_.Name }) -join ','))
$fakeRoot = Join-Path $sandbox 'fake_repo'
New-Item -ItemType Directory -Path (Join-Path $fakeRoot 'tools\acceptance') -Force | Out-Null
[System.IO.File]::WriteAllText((Join-Path $fakeRoot 'tools\acceptance\fake_no_lock.ps1'),
    "Start-Process -FilePath client.exe -ArgumentList '--e2e-user','test'`n", (New-Object System.Text.UTF8Encoding($false)))
$fakeFound = @(Get-E2eClientScript -Root $fakeRoot)
Check 'T9.3 阳性对照：临时造一个「起客户端但没走锁」的脚本必须被判为不合规' `
    ($fakeFound.Count -eq 1 -and -not $fakeFound[0].Armed)

# T9.3b 盲区阳性对照：把「起客户端但没走锁」的脚本放进**旧清单之外**的目录（tools\ops），
# 扫描面必须照样认出它 —— 写死两目录的旧实现会漏掉它，该用例即红。
New-Item -ItemType Directory -Path (Join-Path $fakeRoot 'tools\ops') -Force | Out-Null
[System.IO.File]::WriteAllText((Join-Path $fakeRoot 'tools\ops\fake_ops_no_lock.ps1'),
    "Start-Process -FilePath client_bevy.exe -ArgumentList '--e2e-user','test'`n", (New-Object System.Text.UTF8Encoding($false)))
$fakeSurface = @(Get-E2eScriptSurfaceProbe -Root $fakeRoot)
$fakeUnlocked = @($fakeSurface | Where-Object { -not $_.HasLock })
Check 'T9.3b 盲区阳性对照：旧清单外目录（tools\ops）里「起客户端但没走锁」的脚本必须被认出且判不合规' `
    ($fakeSurface.Count -eq 2 -and $fakeUnlocked.Count -eq 2) `
    ("认出：" + (($fakeSurface | ForEach-Object { $_.Name }) -join ',') + "；判不合规：" + $fakeUnlocked.Count)

# T9.3c 扩展名盲区阳性对照（2026-09-25 补）：`.bat`/`.cmd` 一样能起客户端。实测只认 *.ps1 的
# 实现对这个文件返回 0 条 ⇒ 用 .bat 起客户端就能整体绕开锁与门禁（假绿通道），而资源互斥
# 假红（result=4 密码错误）照旧发生。所以它必须被认出来、且判不合规。
New-Item -ItemType Directory -Path (Join-Path $fakeRoot 'scripts') -Force | Out-Null
[System.IO.File]::WriteAllText((Join-Path $fakeRoot 'scripts\launch_client.bat'),
    "@echo off`r`nstart `"`" client_bevy.exe --e2e-user bevychar --real-net`r`n",
    (New-Object System.Text.ASCIIEncoding))
$surfaceBat = @(Get-E2eScriptSurfaceProbe -Root $fakeRoot)
$batEntry = @($surfaceBat | Where-Object { $_.Name -eq 'launch_client.bat' })
Check 'T9.3c 扩展名盲区阳性对照：.bat 起客户端也必须被认出来且判不合规（扫描面退回只认 *.ps1 即红）' `
    ($batEntry.Count -eq 1 -and -not $batEntry[0].HasLock) `
    ("认出：" + (($surfaceBat | ForEach-Object { $_.Name }) -join ','))

# T9.3d 阴性对照：`.bat` 点名调用**已接入的 .ps1 夹具**（把拿锁委托出去）不许判成不合规——
# 否则扩展名判据只会制造假红，逼人给 .bat 也照抄一份 Enter/Exit（.bat 根本没有那种写法）。
[System.IO.File]::WriteAllText((Join-Path $fakeRoot 'tools\acceptance\fake_enrolled.ps1'),
    ". `"`$PSScriptRoot\e2e_lock.ps1`"`nEnter-E2eLock`nclient.exe --e2e-user test`nExit-E2eLock`n",
    (New-Object System.Text.UTF8Encoding($false)))
[System.IO.File]::WriteAllText((Join-Path $fakeRoot 'scripts\run_enrolled.bat'),
    "call pwsh -NoProfile -File tools\acceptance\fake_enrolled.ps1`r`n",
    (New-Object System.Text.ASCIIEncoding))
$surfaceDeleg = @(Get-E2eScriptSurfaceProbe -Root $fakeRoot)
$delegEntry = @($surfaceDeleg | Where-Object { $_.Name -eq 'run_enrolled.bat' })
Check 'T9.3d 阴性对照：.bat 点名调用已接入的 .ps1 入口时不许判不合规（委托拿锁是合法形态，别造假红）' `
    ($delegEntry.Count -eq 1 -and $delegEntry[0].HasLock) `
    ("认出：" + (($surfaceDeleg | ForEach-Object { $_.Name }) -join ','))

# T9.3e 阴性对照：**只是提到**某个实机入口名（没调用）的文件不算入口 —— 否则
# `flag_coverage_check.ps1` 这种把 `scripts\run_real_e2e.ps1` 当默认参数值的静态门禁会被
# 误判成实机入口、凭空变红。判据：包装入口只认调用形态（call/&/pwsh/Start-Process/-File）。
[System.IO.File]::WriteAllText((Join-Path $fakeRoot 'tools\acceptance\fake_mentions_only.ps1'),
    "`$gate = Join-Path `$Root 'tools\acceptance\fake_no_lock.ps1'`n",
    (New-Object System.Text.UTF8Encoding($false)))
$surfaceMentions = @(Get-E2eScriptSurfaceProbe -Root $fakeRoot)
$mentionsEntry = @($surfaceMentions | Where-Object { $_.Name -eq 'fake_mentions_only.ps1' })
Check 'T9.3e 阴性对照：只提到入口名（没调用）的文件不算实机入口（包装判据不许误伤静态门禁）' `
    ($mentionsEntry.Count -eq 0) `
    ("认出：" + (($surfaceMentions | ForEach-Object { $_.Name }) -join ','))

# T9.4/T9.5：两处判据不许漂移 + 接入器与门禁必须同口径
# （本自检里的 Get-E2eClientScript 是为了 A/B 对照旧版锁脚本才自带的副本；锁脚本里另有一份
#  Get-E2eClientScripts 供批量接入器使用——两份口径一旦漂移，就会出现「接入器说都接了、门禁说没接」。）
if (Get-Command Get-E2eClientScripts -EA SilentlyContinue) {
    $shared = @(Get-E2eClientScripts -RepoRoot $scanRoot | ForEach-Object { $_.Name } | Sort-Object)
    $local = @($clientScripts | ForEach-Object { $_.Name } | Sort-Object)
    $diff = @(Compare-Object -ReferenceObject $local -DifferenceObject $shared)
    Check 'T9.4 自检自带判据与 e2e_lock.ps1 的共享判据识别同一批脚本（防两处漂移）' `
        ($diff.Count -eq 0) ("仅自检认得：" + (($diff | Where-Object SideIndicator -eq '<=' | ForEach-Object InputObject) -join ',') +
                             "；仅共享判据认得：" + (($diff | Where-Object SideIndicator -eq '=>' | ForEach-Object InputObject) -join ','))
} else {
    Write-Host '  [SKIP] T9.4 —— 被测锁脚本没提供 Get-E2eClientScripts（A/B 对照旧版时的正常情况）' -ForegroundColor DarkGray
}
# T9.4b：两份判据在**扩展名与委托判定**上也必须同结论。T9.4 只对真仓比名字，而真仓里此刻
# 没有 .bat/.cmd，所以「只扩了一份扫描面」这种漂移它挡不住 —— 这里拿上面那个含 .bat 的假仓再比一次。
if (Get-Command Get-E2eClientScripts -EA SilentlyContinue) {
    $sharedFake = @(Get-E2eClientScripts -RepoRoot $fakeRoot |
        ForEach-Object { "{0}|{1}" -f $_.Name, [bool]$_.Locked } | Sort-Object)
    $localFake = @(Get-E2eClientScript -Root $fakeRoot |
        ForEach-Object { "{0}|{1}" -f $_.Name, [bool]$_.Locked } | Sort-Object)
    $diffFake = @(Compare-Object -ReferenceObject $localFake -DifferenceObject $sharedFake)
    Check 'T9.4b 两份判据在扩展名（.bat/.cmd）与「委托拿锁」判定上必须同结论（防只扩一份）' `
        ($diffFake.Count -eq 0) ("仅自检认得：" + (($diffFake | Where-Object SideIndicator -eq '<=' | ForEach-Object InputObject) -join ',') +
                                 "；仅共享判据认得：" + (($diffFake | Where-Object SideIndicator -eq '=>' | ForEach-Object InputObject) -join ','))
} else {
    Write-Host '  [SKIP] T9.4b —— 被测锁脚本没提供 Get-E2eClientScripts（A/B 对照旧版时的正常情况）' -ForegroundColor DarkGray
}
$enroller = Join-Path $PSScriptRoot 'enroll_e2e_lock.ps1'
if (Test-Path -LiteralPath $enroller) {
    $enrollOut = (& (Get-Process -Id $PID).Path -NoProfile -NoLogo -File $enroller 2>&1) -join "`n"
    Check 'T9.5 批量接入器与门禁同口径（dry-run 应为「需改 0 个」）' `
        ($enrollOut -match '需改\s*0\s*个') ("接入器输出末段：" + (($enrollOut -split "`n" | Select-Object -Last 2) -join ' / '))
} else {
    Write-Host "  [SKIP] T9.5 —— 没有 $enroller（新增夹具时可以没有接入器，但要手工照抄已接入夹具的写法）" -ForegroundColor DarkGray
}

# T9.6：接入器对非 PowerShell 启动器只许「报人工」，不许假装能自动插入——它是文本启动器，
# 往里插 Enter/Exit 只会把 .bat/.cmd 改坏。判据：输出里有「需人工接入」，且没有对该 .bat 的插入行。
if (Test-Path -LiteralPath $enroller) {
    $enrollFake = (& (Get-Process -Id $PID).Path -NoProfile -NoLogo -File $enroller -RepoRoot $fakeRoot 2>&1) -join "`n"
    Check 'T9.6 接入器不会去改非 PowerShell 启动器（只报「需人工接入」，不出现对 .bat 的自动插入行）' `
        (($enrollFake -match '需人工接入') -and ($enrollFake -notmatch 'DRY\s+launch_client\.bat')) `
        ("接入器输出末段：" + (($enrollFake -split "`n" | Select-Object -Last 2) -join ' / '))
} else {
    Write-Host "  [SKIP] T9.6 —— 没有 $enroller" -ForegroundColor DarkGray
}

# ---------------- T10 语法解析（接入是插入式改动，最容易插出语法错） ----------------
Write-Host 'T10 语法解析：所有实机入口 + 锁本体 + 本自检都必须能被 PowerShell 解析'
$parseTargets = @($clientScripts | Where-Object { $_.Path -match '\.ps1$' } | ForEach-Object { $_.Path }) + @(
    $LockScriptPath,
    (Join-Path $PSScriptRoot 'e2e_lock_selftest.ps1')
)
$parseBad = @()
foreach ($p in $parseTargets) {
    if (-not (Test-Path -LiteralPath $p)) { continue }
    $errs = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile($p, [ref]$null, [ref]$errs)
    if ($errs.Count -gt 0) {
        $parseBad += ("{0}: {1}" -f (Split-Path -Leaf $p), (($errs | ForEach-Object { $_.Message }) -join '; '))
    }
}
Check 'T10.1 全部 PowerShell 文件解析无错（.bat/.cmd 不是 PowerShell 语法，不在此列）' `
    ($parseBad.Count -eq 0) ("解析失败 " + $parseBad.Count + " 个：" + ($parseBad -join ' | '))

Reset-Lock
Remove-Item -LiteralPath $sandbox -Recurse -Force -EA SilentlyContinue

Write-Host ''
Write-Host ("结果：{0} passed / {1} failed（沙箱 {2} 已清理）" -f $pass.Count, $fail.Count, $sandbox)
if ($fail.Count -gt 0) {
    Write-Host ("失败项：" + ($fail -join '; ')) -ForegroundColor Red
    exit 1
}
exit 0
