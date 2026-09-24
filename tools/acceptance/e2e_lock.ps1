<#
.SYNOPSIS
  实机验收的**跨进程互斥锁**：客户端 + e2e 账号 + 本地服务端 是「一次只能一组」的资源。

.DESCRIPTION
  背景（2026-09-25 实测）：多个 agent/夹具并行跑实机时，谁先登录谁占住账号，其余客户端登录得到
  `result=4 密码错误`（服务端日志实为 `Account already online`）——这是**资源互斥假红**，不是产品缺陷；
  靠 `for(i=1..8){ 跑夹具; sleep 60 }` 撞"干净窗口"只会把交付时间耗在等待上（实测连撞 7 次全红）。

  用法（dot-source 后成对调用，务必放在 try/finally 里）：

  ```powershell
  . "$PSScriptRoot\e2e_lock.ps1"
  if (-not (Enter-E2eLock -ScriptName 'l5x_xxx' -TimeoutSec 1800)) { exit 2 }
  try {
      ... 起客户端、跑断言 ...
  } finally {
      Exit-E2eLock
  }
  ```

  语义：
  - 锁文件 `%TEMP%\crystal_e2e_client_test.lock`（**跨 worktree、跨 agent 全局唯一**）；
  - 用 `[System.IO.File]::Open(..., FileMode::CreateNew)` 原子创建 —— 已存在即失败；
  - 锁里记 `{pid, script, host, started}`，回收判据三条（都打显式 WARN，防"看起来像占着"）：
    1. **持有者 PID 已死** → 立即回收（防僵尸锁）；
    2. **PID 被复用**：同 PID 的进程**晚于**锁创建时刻才启动 ⇒ 那是另一个进程，不算持有者
       （Windows 会复用 PID，缺这条会让死锁看起来"仍被占着"，白白等到 `StaleSec`）；
    3. 锁龄超过 `StaleSec`（默认 1800s）→ 回收（持有者可能卡死或拿到锁后没跑到释放）。
  - 没拿到就每 5s 重试，直到 `TimeoutSec`（默认 1800s）；返回 `$false` 表示超时（调用方应 exit 2）。

#>

<#
  ===== 接入覆盖面（这才是「别再用 for(i=1..8) 撞干净窗口」的兑现方式）=====

  光有锁不够：**没接入的脚本与接入的脚本并行跑，照样撞假红**（实测 2026-09-25：
  `tools/acceptance/` 下 26 个会起客户端的脚本里只有 l5u/l5v 接了锁，连每次 UI 改动
  都要跑的硬门禁 `ui_interact_sweep.ps1` 都没接）。

  所以配套三件事，缺一把就退化成「有锁但没用」：

  1. `enroll_e2e_lock.ps1`：批量接入器（幂等，只插不删）——给每个实机入口插入
     dot-source + `Enter-E2eLock`，并在每条 `exit` 之前插入 `Exit-E2eLock`；
  2. `e2e_lock_selftest.ps1`：**门禁**——扫描「会起客户端的脚本」（判据见 `Get-E2eClientScripts`），
     少一把锁即红；同时用子进程实测串行性与僵尸锁回收，并带阳性对照（去掉锁必须红）；
  3. 本文件里的 `Get-E2eClientScripts` / `Test-E2eLockEnrollment`：上面两个脚本共用同一份
     判据，避免「接入器认为接了、门禁认为没接」这种漂移。

 约定：**任何要起客户端或登录 e2e 账号的脚本/agent 都必须先拿这把锁**（含 `*.ps1` 夹具与临时取数脚本）。

 释放：接入器给每个入口整段包了 `try/finally` + `Exit-E2eLock`（PowerShell 的 `finally` 在 `exit` 下
 也会执行——`-File` 与会话内 `& 夹具.ps1` 两种调用都实测过），正常/早退/异常路径都会及时释放。
 **即使漏了释放也不会卡住队列**：脚本进程一退出，下一个调用者立刻按判据 1（或 2）回收
 （实测：两个 `l5w` 并发，后者排队 30s，前者进程退出后它即接管）。释放后锁文件残留是正常的，
 它只是一份"最近一个持有者"的记录，不是"仍被占用"的信号。

 嵌套调用（父脚本 → 子脚本）：拿到锁的进程会设环境变量 `CRYSTAL_E2E_LOCK_HELD_BY`，子进程
 （继承环境）`Enter-E2eLock` 直接**复用父进程的锁**、不重复抢——否则会自锁到超时。
 现实例子：`scripts/run_real_e2e.ps1 -IncludeInteractSweep` 会调用 `ui_interact_sweep.ps1`。
 **但复用前必须验标记**：环境变量会沿进程树继承，父进程释放/退出后残留的标记会让后来者"假持锁"
 （以为自己持着、实际没有任何串行化）。所以只有当「锁文件里的 pid == 标记里的 pid 且该进程仍活着」
 才复用；否则打黄字忽略标记、改走正常抢锁（`e2e_lock_selftest.ps1` 的 T2d 钉这条）。

 新增夹具的机械做法：`pwsh tools/acceptance/enroll_e2e_lock.ps1 -Apply`，或照抄已接入夹具的写法。
#>

$script:E2eLockPath = Join-Path $env:TEMP 'crystal_e2e_client_test.lock'
$script:E2eLockHeld = $false
$script:E2eLockInherited = $false
$script:E2eLockPid = $PID
$script:E2eLockEnvVar = 'CRYSTAL_E2E_LOCK_HELD_BY'
$script:E2eLockAcquiredAt = $null

function Get-E2eLockPath { $script:E2eLockPath }

function Enter-E2eLock {
    param(
        [string]$ScriptName = 'unknown',
        [int]$TimeoutSec = 1800,
        [int]$StaleSec = 1800,
        [int]$PollSec = 5
    )
    # 父进程已持有（本进程是它拉起的子脚本）：复用，不重复抢。
    # 但**必须先证明这个标记仍然有效**：环境变量沿进程树继承，父进程早就释放/退出之后，
    # 残留的标记会让后来者「假持锁」（读不到锁文件却以为自己持着，实际并没有串行化）。
    # 判据：锁文件里记的 pid == 标记里的 pid，且那个进程仍然活着。
    if ($env:CRYSTAL_E2E_LOCK_HELD_BY) {
        $token = [string]$env:CRYSTAL_E2E_LOCK_HELD_BY
        $tokenPid = 0
        if ($token -match 'pid=(\d+)') { $tokenPid = [int]$Matches[1] }
        $filePid = 0
        $fileOk = $false
        try {
            $h = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop | ConvertFrom-Json
            if ($null -ne $h.pid) { $filePid = [int]$h.pid; $fileOk = $true }
        } catch {}
        $tokenAlive = ($tokenPid -gt 0) -and [bool](Get-Process -Id $tokenPid -EA SilentlyContinue)
        if ($fileOk -and $tokenAlive -and ($filePid -eq $tokenPid)) {
            $script:E2eLockInherited = $true
            Write-Host ("[e2e-lock] 复用父进程已持有的锁（{0}）：{1}" -f $token, $ScriptName)
            return $true
        }
        Write-Host ("[e2e-lock] 忽略残留的继承标记（{0}；锁文件 pid={1}、标记进程存活={2}）——改为正常抢锁" -f `
            $token, $filePid, $tokenAlive) -ForegroundColor Yellow
        Remove-Item Env:CRYSTAL_E2E_LOCK_HELD_BY -ErrorAction SilentlyContinue
    }
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $waited = $false
    while ($true) {
        try {
            # 先把 payload 写进临时文件，再 Move 到锁路径：Move 在目标已存在时失败（= CreateNew 语义），
            # 且**锁文件一出现就是完整内容**。早前写法是 CreateNew 后再 Write，中间有一段
            # 「文件存在但为空」的窗口——等待者读不出持有者，会把它当僵尸锁删掉，于是两个进程同时进
            # 临界区（2026-09-25 被 e2e_lock_selftest 的 T1 抓到：两个子进程在同一 20ms 内都"持锁"）。
            $payload = @{
                pid     = $PID
                script  = $ScriptName
                host    = $env:COMPUTERNAME
                started = (Get-Date).ToString('o')
            } | ConvertTo-Json -Compress
            $tmp = "$script:E2eLockPath.$PID.tmp"
            [System.IO.File]::WriteAllText($tmp, $payload, (New-Object System.Text.UTF8Encoding($false)))
            try {
                [System.IO.File]::Move($tmp, $script:E2eLockPath)
            } catch {
                Remove-Item -LiteralPath $tmp -Force -EA SilentlyContinue
                throw
            }
            $script:E2eLockHeld = $true
            # 传给子进程：让嵌套调用（run_real_e2e → ui_interact_sweep）复用同一把锁
            $env:CRYSTAL_E2E_LOCK_HELD_BY = "pid=$PID script=$ScriptName"
            $script:E2eLockAcquiredAt = Get-Date
            if ($waited) { Write-Host ("[e2e-lock] 拿到锁（等了 {0}s）：{1}" -f [int]$waitedSeconds, $ScriptName) }
            else { Write-Host ("[e2e-lock] 拿到锁：{0}" -f $ScriptName) }
            return $true
        } catch {
            # 锁已存在：看持有者是否还活着 / 是否超龄
            $holder = $null
            try {
                $holder = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop | ConvertFrom-Json
            } catch {}
            # 同进程重入：夹具里再 dot-source / 再 Enter 一把时不要自锁
            # （否则持有者就是自己，会一路等到 TimeoutSec 才失败）
            if ($null -ne $holder -and [int]$holder.pid -eq $PID) {
                $script:E2eLockHeld = $true
                return $true
            }
            $ownerAlive = $false
            $ageSec = 0
            $pidReused = $false
            if ($null -ne $holder -and $null -ne $holder.pid) {
                $proc = Get-Process -Id ([int]$holder.pid) -EA SilentlyContinue
                $ownerAlive = [bool]$proc
                if ($null -ne $holder.started) {
                    try { $ageSec = [int]((Get-Date) - [datetime]::Parse($holder.started)).TotalSeconds } catch {}
                }
                # PID 复用护栏：锁建于 T0，若同 PID 的进程**在 T0 之后**才启动，那是另一个进程
                # （Windows 复用 PID）——它不是持有者，不能让队列白等 30 分钟。
                if ($ownerAlive -and $null -ne $holder.started -and $null -ne $proc.StartTime) {
                    try {
                        $pidReused = ($proc.StartTime -gt ([datetime]::Parse($holder.started).AddSeconds(2)))
                    } catch {}
                }
            } else {
                # 锁文件存在但读不出持有者：可能是别人正在创建/写入（旧实现的空窗），也可能是文件坏了。
                # 一律按**文件 mtime** 判龄，且默认当作「有人持锁」——只有确实超龄才回收。
                # 反例（修前）：这里把「读不出」当僵尸直接删，于是把持有者刚建的锁删掉，两个进程同时进。
                $mt = (Get-Item -LiteralPath $script:E2eLockPath -EA SilentlyContinue).LastWriteTime
                if ($null -ne $mt) { $ageSec = [int]((Get-Date) - $mt).TotalSeconds }
                $ownerAlive = ($ageSec -le $StaleSec)
                Write-Host ("[e2e-lock] 锁文件暂不可解析（锁龄 {0}s，可能是持有者正在写入）——按持锁处理" -f $ageSec) -ForegroundColor DarkGray
            }
            if ((-not $ownerAlive) -or $pidReused -or ($ageSec -gt $StaleSec)) {
                Write-Host ("[e2e-lock] 回收僵尸/复用PID/超龄锁（pid={0} 存活={1} 复用PID={2} 锁龄={3}s）：{4}" -f `
                    $holder.pid, $ownerAlive, $pidReused, $ageSec, $holder.script) -ForegroundColor Yellow
                try { [System.IO.File]::Delete($script:E2eLockPath) } catch {}
                continue
            }
            if ((Get-Date) -gt $deadline) {
                Write-Host ("[e2e-lock] 等锁超时 {0}s（持有者 pid={1} 脚本={2} 锁龄={3}s）" -f `
                    $TimeoutSec, $holder.pid, $holder.script, $ageSec) -ForegroundColor Yellow
                return $false
            }
            if (-not $waited) {
                Write-Host ("[e2e-lock] 排队等锁（持有者 pid={0} 脚本={1}）..." -f $holder.pid, $holder.script)
                $waited = $true
                $waitedSeconds = 0
            }
            Start-Sleep -Seconds $PollSec
            $waitedSeconds = ([int]$waitedSeconds) + $PollSec
        }
    }
}

function Exit-E2eLock {
    # 子进程复用的是父进程的锁：子进程不碰锁文件
    if ($script:E2eLockInherited) { $script:E2eLockInherited = $false; return }
    if (-not $script:E2eLockHeld) { return }
    try {
        $holder = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop | ConvertFrom-Json
        # 只删自己的锁（避免把后来者的锁删掉）
        if ($null -ne $holder -and [int]$holder.pid -eq $PID) {
            [System.IO.File]::Delete($script:E2eLockPath)
        } else {
            Write-Host ("[e2e-lock] 警告：锁的持有者不是本进程（pid={0}），不删" -f $holder.pid) -ForegroundColor Yellow
        }
    } catch {
        # 读不出持有者：只有「文件确实是在本次持锁期间出现的」才认为是我们自己的（写坏了），
        # 否则保留——盲目删会把别人的锁删掉（那正是 T1 抓到的"两个进程同时进"的另一半）。
        $mt = (Get-Item -LiteralPath $script:E2eLockPath -EA SilentlyContinue).LastWriteTime
        if ($null -ne $mt -and $null -ne $script:E2eLockAcquiredAt -and $mt -ge $script:E2eLockAcquiredAt.AddSeconds(-2)) {
            try { [System.IO.File]::Delete($script:E2eLockPath) } catch {}
        } else {
            Write-Host '[e2e-lock] 警告：锁文件不可解析且不像本次持锁产生的，保留不删' -ForegroundColor Yellow
        }
    }
    $script:E2eLockHeld = $false
    Remove-Item Env:$script:E2eLockEnvVar -ErrorAction SilentlyContinue
    Write-Host '[e2e-lock] 已释放'
}

function Get-E2eLockInfo {
    <#
      只读诊断：谁在持锁 / 锁龄多少 / 持有者是否还活着。
      排查「result=4 密码错误」时先看这个——先确认是不是没走锁的会话占着账号。
    #>
    if (-not (Test-Path -LiteralPath $script:E2eLockPath)) {
        return [pscustomobject]@{ held = $false; pid = $null; script = ''; age_sec = 0; owner_alive = $false }
    }
    $h = $null
    try { $h = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop | ConvertFrom-Json } catch {}
    $age = 0
    if ($null -ne $h -and $null -ne $h.started) {
        try { $age = [int]((Get-Date) - [datetime]::Parse($h.started)).TotalSeconds } catch {}
    }
    $alive = $false
    if ($null -ne $h -and $null -ne $h.pid) { $alive = [bool](Get-Process -Id ([int]$h.pid) -EA SilentlyContinue) }
    [pscustomobject]@{
        held        = $true
        pid         = if ($null -ne $h) { $h.pid } else { $null }
        script      = if ($null -ne $h) { [string]$h.script } else { '' }
        age_sec     = $age
        owner_alive = $alive
    }
}

function Get-E2eClientScripts {
    <#
      「会起客户端并登录 e2e 账号的脚本」的单一判据（接入器与门禁共用，避免两边漂移）。

      判据：脚本正文里出现 `--e2e-user`（= 起客户端时带自动化登录账号）。
      扫描面：<RepoRoot>\tools\acceptance\*.ps1 与 <RepoRoot>\scripts\*.ps1
      （临时取数脚本不在此列——那种脚本归「谁写谁拿锁」，见本文件头部约定）。
    #>
    param([string]$RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path)
    $selfNames = @('e2e_lock.ps1', 'e2e_lock_selftest.ps1', 'enroll_e2e_lock.ps1')
    $dirs = @((Join-Path $RepoRoot 'tools\acceptance'), (Join-Path $RepoRoot 'scripts'))
    $out = @()
    foreach ($d in $dirs) {
        if (-not (Test-Path -LiteralPath $d)) { continue }
        foreach ($f in (Get-ChildItem -LiteralPath $d -File -Filter *.ps1 -EA SilentlyContinue)) {
            if ($selfNames -contains $f.Name) { continue }
            $text = Get-Content -LiteralPath $f.FullName -Raw -EA SilentlyContinue
            if ($null -eq $text -or $text -notmatch '--e2e-user') { continue }
            $out += [pscustomobject]@{
                Name       = $f.Name
                Path       = $f.FullName
                Dir        = $f.DirectoryName
                HasLock    = [bool]($text -match 'e2e_lock\.ps1')
                EnterCount = ([regex]::Matches($text, 'Enter-E2eLock')).Count
                ExitCount  = ([regex]::Matches($text, 'Exit-E2eLock')).Count
            }
        }
    }
    $out
}

function Test-E2eLockEnrollment {
    <#
      单个脚本的接入是否完整：① dot-source 了 e2e_lock.ps1；② 有 Enter-E2eLock；
      ③ 至少有 Exit-E2eLock（没持锁时它是 no-op，所以宁多勿少）。
      返回 @{ ok = $bool; reasons = @() }。
    #>
    param([Parameter(Mandatory)][string]$Path)
    $text = Get-Content -LiteralPath $Path -Raw -EA Stop
    $reasons = @()
    if ($text -notmatch 'e2e_lock\.ps1') { $reasons += '未 dot-source e2e_lock.ps1' }
    if ($text -notmatch 'Enter-E2eLock') { $reasons += '未调用 Enter-E2eLock' }
    if ($text -notmatch 'Exit-E2eLock') { $reasons += '未调用 Exit-E2eLock（会泄漏锁：持有者进程还活着时别人只能等到 StaleSec）' }
    [pscustomobject]@{ ok = ($reasons.Count -eq 0); reasons = $reasons }
}
