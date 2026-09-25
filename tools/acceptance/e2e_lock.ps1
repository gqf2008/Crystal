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
  - **原子建锁**：payload 先写自己 PID 的临时文件，再 `File.Move` 到锁路径（目标已存在即失败，
    与 `FileMode::CreateNew` 同样的互斥语义）。这样**锁文件一出现就是完整内容**，等待者不会
    看到"存在但读不出"的半截锁（2026-09-25 修，见下面两条判据）；
  - 锁里记 `{pid, script, host, started}`，回收判据三条（都打显式 WARN，防"看起来像占着"）：
    1. **持有者 PID 已死** → 立即回收（防僵尸锁）；
    2. **PID 被复用**：同 PID 的进程**晚于**锁创建时刻才启动 ⇒ 那是另一个进程，不算持有者
       （Windows 会复用 PID，缺这条会让死锁看起来"仍被占着"，白白等到 `StaleSec`）；
    3. 锁龄超过 `StaleSec`（默认 1800s）→ 回收（持有者可能卡死或拿到锁后没跑到释放）。
  - 没拿到就每 5s 重试，直到 `TimeoutSec`（默认 1800s）；返回 `$false` 表示超时（调用方应 exit 2）。
  - **锁文件在、内容却读不全**（空 / 半截 / 被写入句柄独占）→ **不回收**，只等它落地；
    超过 `GraceSec`（默认 5s）仍不可解析才当残骸回收。2026-09-25 修：原实现把
    「持有者刚 CreateNew、payload 还没写完」当成垃圾删掉 → 两个客户端可能同时跑，
    正是这套锁要消除的那种假红。
  - **继承标记要复核**（2026-09-25 修）：`CRYSTAL_E2E_LOCK_HELD_BY` 只在「标记里的父进程
    仍活着 **且** 锁文件确实由它持有」时才算数，否则忽略并正常抢锁。原实现凭残留环境变量
    就能"假持锁"：父脚本退出/提前释放后，同一个 shell 里后续的夹具会跳过排队直接起客户端。

  自证：`pwsh tools/acceptance/e2e_lock_selftest.ps1`（不需要客户端/服务端/账号，秒级；
  它把 `TEMP` 指向临时目录，**不会碰真实锁**）。

  约定：**任何要起客户端或登录 e2e 账号的脚本/agent 都必须先拿这把锁**（含 `*.ps1` 夹具与临时取数脚本）。

 释放：能包 `try/finally` 的（`l5u`/`l5v`）显式 `Exit-E2eLock`；结构不便于包一层的夹具只调用
  `Enter-E2eLock`——**脚本进程一退出，下一个调用者立刻按判据 1（或 2）回收**，不会卡住队列
  （实测：两个 `l5w` 并发，后者排队 30s，前者进程退出后它即接管）。释放后锁文件残留是正常的，
  它只是一份"最近一个持有者"的记录，不是"仍被占用"的信号。

  嵌套调用（父脚本 → 子脚本）：拿到锁的进程会设环境变量 `CRYSTAL_E2E_LOCK_HELD_BY`，子进程
  （继承环境）`Enter-E2eLock` 直接**复用父进程的锁**、不重复抢——否则会自锁到超时。
  现实例子：`scripts/run_real_e2e.ps1 -IncludeInteractSweep` 会调用 `ui_interact_sweep.ps1`。
#>

$script:E2eLockPath = Join-Path $env:TEMP 'crystal_e2e_client_test.lock'
$script:E2eLockHeld = $false
$script:E2eLockInherited = $false
$script:E2eLockPid = $PID
$script:E2eLockEnvVar = 'CRYSTAL_E2E_LOCK_HELD_BY'

function Get-E2eLockPath { $script:E2eLockPath }

function Enter-E2eLock {
    param(
        [string]$ScriptName = 'unknown',
        [int]$TimeoutSec = 1800,
        [int]$StaleSec = 1800,
        [int]$PollSec = 5,
        [int]$GraceSec = 5
    )
    # 父进程已持有（本进程是它拉起的子脚本）：复用，不重复抢。
    # 但**必须复核**这个标记还成立 —— 只用环境变量当判据时，父脚本退出/提前释放后，
    # 同一个 shell 里后续的夹具会凭残留标记"假持锁"，两个客户端就同时跑了。
    if ($env:CRYSTAL_E2E_LOCK_HELD_BY) {
        $inheritParent = $null
        if ($env:CRYSTAL_E2E_LOCK_HELD_BY -match 'pid=(\d+)') { $inheritParent = [int]$Matches[1] }
        $inheritOk = $false
        if ($null -ne $inheritParent -and (Get-Process -Id $inheritParent -EA SilentlyContinue)) {
            try {
                $inh = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop | ConvertFrom-Json
                if ($null -ne $inh -and [int]$inh.pid -eq $inheritParent) { $inheritOk = $true }
            } catch {}
        }
        if ($inheritOk) {
            $script:E2eLockInherited = $true
            Write-Host ("[e2e-lock] 复用父进程已持有的锁（{0}）：{1}" -f $env:CRYSTAL_E2E_LOCK_HELD_BY, $ScriptName)
            return $true
        }
        Write-Host ("[e2e-lock] 忽略失效的继承标记（父进程已退出或锁已不在）：{0} → 改为正常抢锁" -f $env:CRYSTAL_E2E_LOCK_HELD_BY) -ForegroundColor Yellow
        Remove-Item Env:$script:E2eLockEnvVar -ErrorAction SilentlyContinue
    }
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $waited = $false
    while ($true) {
        try {
            $payload = @{
                pid     = $PID
                script  = $ScriptName
                host    = $env:COMPUTERNAME
                started = (Get-Date).ToString('o')
            } | ConvertTo-Json -Compress
            # 原子建锁：payload 先落到**自己 PID 的临时文件**，再 `Move` 到锁路径。
            # `File.Move` 在目标已存在时**直接失败**，所以互斥语义与原来的 `FileMode::CreateNew` 一样；
            # 但好处是**锁文件一出现就是完整内容** —— 旧写法（CreateNew 之后再 Write）中间有一段
            # 「文件存在、内容为空/半截」的窗口，等待者读不出持有者就会当僵尸删掉它，两个进程同时
            # 进临界区（下面的宽限判据是第二道防线，第一道是这里的原子性）。
            $tmp = "$script:E2eLockPath.$PID.tmp"
            [System.IO.File]::WriteAllText($tmp, $payload, (New-Object System.Text.UTF8Encoding($false)))
            try {
                [System.IO.File]::Move($tmp, $script:E2eLockPath)
            } catch {
                Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
                throw
            }
            $script:E2eLockHeld = $true
            # 传给子进程：让嵌套调用（run_real_e2e → ui_interact_sweep）复用同一把锁
            $env:CRYSTAL_E2E_LOCK_HELD_BY = "pid=$PID script=$ScriptName"
            if ($waited) { Write-Host ("[e2e-lock] 拿到锁（等了 {0}s）：{1}" -f [int]$waitedSeconds, $ScriptName) }
            else { Write-Host ("[e2e-lock] 拿到锁：{0}" -f $ScriptName) }
            return $true
        } catch {
            # 锁已存在：看持有者是否还活着 / 是否超龄
            $holder = $null
            if (-not (Test-Path -LiteralPath $script:E2eLockPath)) {
                # 竞态：持有者恰在这几微秒里释放了 → 不睡觉，立刻重试抢锁
                continue
            }
            try {
                $raw = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop
                if (-not [string]::IsNullOrWhiteSpace($raw)) { $holder = $raw | ConvertFrom-Json }
            } catch {}
            if ($null -eq $holder) {
                # 文件在、内容读不全（空 / 半截 / 被独占句柄挡住 / 外部工具写坏）。
                # 正常路径下不会走到这里（建锁是 tmp+Move，锁一出现就是完整内容），但**不能**因为
                # "读不到"就删它 —— 删了就等于两个客户端同时跑。只有超过宽限期 GraceSec
                # 仍不可解析，才认定是残骸并回收。
                $rawAge = 0
                try { $rawAge = [int]((Get-Date) - (Get-Item -LiteralPath $script:E2eLockPath -EA Stop).LastWriteTime).TotalSeconds } catch {}
                if ($rawAge -le $GraceSec) {
                    if ((Get-Date) -gt $deadline) {
                        Write-Host ("[e2e-lock] 等锁超时 {0}s（锁文件正在写入中，锁龄 {1}s）" -f $TimeoutSec, $rawAge) -ForegroundColor Yellow
                        return $false
                    }
                    if (-not $waited) {
                        Write-Host '[e2e-lock] 锁文件刚建立、内容尚未写完 → 等它落地（不回收）...'
                        $waited = $true
                        $waitedSeconds = 0
                    }
                    Start-Sleep -Seconds 1
                    $waitedSeconds = ([int]$waitedSeconds) + 1
                    continue
                }
                Write-Host ("[e2e-lock] 回收不可解析的锁文件（锁龄 {0}s > 宽限 {1}s）" -f $rawAge, $GraceSec) -ForegroundColor Yellow
                try { [System.IO.File]::Delete($script:E2eLockPath) } catch {}
                continue
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
        }
    } catch {
        try { [System.IO.File]::Delete($script:E2eLockPath) } catch {}
    }
    $script:E2eLockHeld = $false
    Remove-Item Env:$script:E2eLockEnvVar -ErrorAction SilentlyContinue
    Write-Host '[e2e-lock] 已释放'
}

function Get-E2eClientScripts {
    <#
      「会起客户端并登录 e2e 账号的脚本」的**单一判据**（接入器与门禁共用，避免两边漂移）。

      判据：正文里出现任一特征即算「会起客户端」——
        `--e2e-user`（自动化登录账号）/ `client_bevy.exe` / `--real-net` / `--auto-enter`。
      不只看 `--e2e-user`：`l5r_ranged_projectile`、`l5t_minimize_survives` 这类不传 e2e 账号
      但照样起客户端的脚本，也必须接入锁（#3129 的覆盖口径就是「会起客户端」）。
      扫描面：**整个仓库**的 *.ps1（排除 .git / target / node_modules 与锁自身的三个脚本）。
      不要退回「写死目录清单」——原先只扫 `tools\acceptance` 与 `scripts`，结果
      `tools\ops\package_windows_rehearsal.ps1` 明明起客户端（`client_bevy.exe --e2e-user`）
      却扫不进来；它一旦被改掉那把锁，门禁照样绿（假绿盲区）。整仓扫描让任何新目录里的
      实机入口自动纳入判据，不必再维护目录清单。
      临时取数脚本不在此列——那种脚本归「谁写谁拿锁」，见本文件头部的约定。
    #>
    param([string]$RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path)
    $selfNames = @('e2e_lock.ps1', 'e2e_lock_selftest.ps1', 'enroll_e2e_lock.ps1')
    $skipDirs = '\\(\.git|target|node_modules)\\'
    $out = @()
    $files = @(Get-ChildItem -LiteralPath $RepoRoot -Recurse -File -Filter *.ps1 -EA SilentlyContinue |
        Where-Object { $_.FullName -notmatch $skipDirs })
    foreach ($f in $files) {
        if ($selfNames -contains $f.Name) { continue }
        $text = Get-Content -LiteralPath $f.FullName -Raw -EA SilentlyContinue
        if ($null -eq $text) { continue }
        if ($text -notmatch '--e2e-user|client_bevy\.exe|--real-net|--auto-enter') { continue }
        $out += [pscustomobject]@{
            Name       = $f.Name
            Path       = $f.FullName
            Dir        = $f.DirectoryName
            EnterCount = ([regex]::Matches($text, 'Enter-E2eLock')).Count
            ExitCount  = ([regex]::Matches($text, 'Exit-E2eLock')).Count
        }
    }
    $out
}

function Test-E2eLockEnrollment {
    <#
      单个实机入口的接入是否完整：① dot-source 了 e2e_lock.ps1；② 有 `Enter-E2eLock`；
      ③ 至少有 `Exit-E2eLock`（没持锁时它是 no-op，所以宁多勿少——缺它，早退路径会把锁
      留到下一个调用者才发现要回收）。
      返回 @{ ok = $bool; reasons = @() }。
    #>
    param([Parameter(Mandatory)][string]$Path)
    $text = Get-Content -LiteralPath $Path -Raw -EA Stop
    $reasons = @()
    if ($text -notmatch 'e2e_lock\.ps1') { $reasons += '未 dot-source e2e_lock.ps1' }
    if ($text -notmatch 'Enter-E2eLock') { $reasons += '未调用 Enter-E2eLock' }
    if ($text -notmatch 'Exit-E2eLock') { $reasons += '未调用 Exit-E2eLock（早退路径会把锁留到下一次进入才发现）' }
    [pscustomobject]@{ ok = ($reasons.Count -eq 0); reasons = $reasons }
}
