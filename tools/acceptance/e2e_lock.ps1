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

  约定：**任何要起客户端或登录 e2e 账号的脚本/agent 都必须先拿这把锁**（含 `*.ps1` 夹具与临时取数脚本）。
#>

$script:E2eLockPath = Join-Path $env:TEMP 'crystal_e2e_client_test.lock'
$script:E2eLockHeld = $false
$script:E2eLockPid = $PID

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
            $fs = [System.IO.File]::Open(
                $script:E2eLockPath,
                [System.IO.FileMode]::CreateNew,
                [System.IO.FileAccess]::Write,
                [System.IO.FileShare]::None
            )
            $payload = @{
                pid     = $PID
                script  = $ScriptName
                host    = $env:COMPUTERNAME
                started = (Get-Date).ToString('o')
            } | ConvertTo-Json -Compress
            $bytes = [Text.Encoding]::UTF8.GetBytes($payload)
            $fs.Write($bytes, 0, $bytes.Length)
            $fs.Flush()
            $fs.Close()
            $script:E2eLockHeld = $true
            if ($waited) { Write-Host ("[e2e-lock] 拿到锁（等了 {0}s）：{1}" -f [int]$waitedSeconds, $ScriptName) }
            else { Write-Host ("[e2e-lock] 拿到锁：{0}" -f $ScriptName) }
            return $true
        } catch {
            # 锁已存在：看持有者是否还活着 / 是否超龄
            $holder = $null
            try {
                $holder = Get-Content -LiteralPath $script:E2eLockPath -Raw -EA Stop | ConvertFrom-Json
            } catch {}
            $ownerAlive = $false
            $ageSec = 0
            if ($null -ne $holder -and $null -ne $holder.pid) {
                $ownerAlive = [bool](Get-Process -Id ([int]$holder.pid) -EA SilentlyContinue)
                if ($null -ne $holder.started) {
                    try { $ageSec = [int]((Get-Date) - [datetime]::Parse($holder.started)).TotalSeconds } catch {}
                }
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
        }
    } catch {
        try { [System.IO.File]::Delete($script:E2eLockPath) } catch {}
    }
    $script:E2eLockHeld = $false
    Write-Host '[e2e-lock] 已释放'
}
