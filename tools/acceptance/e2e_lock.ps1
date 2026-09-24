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
  - 锁里记 `{pid, script, host, started}`：**持有者 PID 已死** → 立即回收（防僵尸锁）；
    锁龄超过 `StaleSec`（默认 1800s）→ 也回收，但打一条显式 WARN（防"看起来像占着"）；
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
  新增夹具的机械做法：`pwsh tools/acceptance/enroll_e2e_lock.ps1 -Apply`，或照抄已接入夹具的写法。
#>

$script:E2eLockPath = Join-Path $env:TEMP 'crystal_e2e_client_test.lock'
$script:E2eLockHeld = $false
$script:E2eLockPid = $PID
$script:E2eLockAcquiredAt = $null

function Get-E2eLockPath { $script:E2eLockPath }

function Enter-E2eLock {
    param(
        [string]$ScriptName = 'unknown',
        [int]$TimeoutSec = 1800,
        [int]$StaleSec = 1800,
        [int]$PollSec = 5
    )
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
            if ($null -ne $holder -and $null -ne $holder.pid) {
                $ownerAlive = [bool](Get-Process -Id ([int]$holder.pid) -EA SilentlyContinue)
                if ($null -ne $holder.started) {
                    try { $ageSec = [int]((Get-Date) - [datetime]::Parse($holder.started)).TotalSeconds } catch {}
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
            if ((-not $ownerAlive) -or ($ageSec -gt $StaleSec)) {
                Write-Host ("[e2e-lock] 回收僵尸/超龄锁（pid={0} 存活={1} 锁龄={2}s）：{3}" -f `
                    $holder.pid, $ownerAlive, $ageSec, $holder.script) -ForegroundColor Yellow
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
