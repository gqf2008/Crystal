# l5t_minimize_survives.ps1 — 「最小化窗口」不得再把客户端带走（实机夹具，默认离线，约 25s）
#
# 缺陷（修复前的 master，实测 3/3 复现）：最小化窗口 → winit 上报
#   `WM_SIZE(SIZE_MINIMIZED)`→`Resized(0,0)` → bevy 用新尺寸重配表面 →
#   dx12 `ResizeBuffers` 失败（DXGI 调试层：`Swapchain cannot be resized unless all
#   outstanding buffer references have been released`）→ wgpu 归类为 Validation
#   RenderError → bevy **默认** `RenderErrorHandler` 对任何 RenderError 一律
#   `AppExit::error()` → 客户端 7ms 内退进程（无 WER、无转储）。
#   定性/证据见 `Client-Bevy/src/render_error.rs` 文件头与 owner 提供的实测日志。
#
# 判据（全部取「进程存活」+「客户端日志真值」，不做像素比对）：
#   A) 起客户端静置 $WaitSec 秒仍存活（**离线也能跑**：停在开场画面即可，不需要服务端）
#   B) SW_MINIMIZE 后 $PostMinimizeSec 秒仍存活，且日志无
#      `Quitting the application due to`
#   B2) 最小化**确实**触发了那次表面配置失败（日志出现 `ResizeBuffers failed`
#       或 `[render-error]`）——否则夹具是空转，判 exit 2 而不是假绿
#   B3) 最小化期间「暂停渲染」提示**恰好 1 次**（去抖生效；每帧刷屏算 FAIL）
#   D) 最小化期间 control RPC 仍能应答——**暂停的是渲染，不是整个应用**
#       （`StopRendering` 只停渲染图；主世界 Update/网络必须照跑，否则最小化=卡死）
#   C) SW_RESTORE 后 $RestoreSec 秒仍存活，且日志出现
#      `[render-error] 窗口已恢复：渲染继续`（证明渲染真的被放回来了，不是"卡死不报错"）
#
# 退出码：
#   0 = 全 PASS
#   1 = 缺陷存在（进程退出 / 出现 `Quitting the application due to` / 恢复后渲染没回来 /
#       最小化期间刷屏）
#   2 = 不确定（启动即退、拿不到窗口句柄、最小化没触发表面失败、control RPC 不通、
#       被外部 Stop-Process）
#
# 阳性对照（自带红检，**必跑**，证明"绿"是本策略带来的）：
#   pwsh tools/acceptance/l5t_minimize_survives.ps1 -NoHandler     # 期望 exit 1
#   -NoHandler 给客户端设 `CRYSTAL_NO_RENDER_ERROR_HANDLER=1`（不装本策略、保留 bevy
#   默认），同一二进制、同一流程必须复现缺陷；这里若没红，说明夹具或策略没接上线。
#
# 多 agent 并行（本机常态）：**禁止**按进程名批量 `Stop-Process client_bevy.exe`
# （别的 agent 正在用客户端做实验，2026-09-24 已踩过：实验被批量清进程杀成假阴）。
# 本夹具默认把 exe 复制成唯一名字（`$OutDir\client_l5t.exe`）再跑、只杀自己那个名字；
# `-NoCopy` 可跳过复制（多 agent 环境下不建议）。
#
# 用法：
#   pwsh tools/acceptance/l5t_minimize_survives.ps1                  # 修后应 exit 0
#   pwsh tools/acceptance/l5t_minimize_survives.ps1 -NoHandler       # 阳性对照，应 exit 1
param(
    [string]$ClientHome = '',
    [string]$ExePath = '',
    [string]$OutDir = "$env:TEMP\l5t_minimize",
    [string]$ControlPort = '9317',
    [int]$WaitSec = 12,
    [int]$PostMinimizeSec = 5,
    [int]$RestoreSec = 5,
    [switch]$NoHandler,
    [switch]$NoCopy
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5t_minimize_survives' -TimeoutSec 1800)) { exit 2 }
$ErrorActionPreference = 'Continue'

# 客户端链了 libpinyin DLL：PATH 不带这两个目录时进程会**静默秒退**（本会话踩过）
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'

if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not $ExePath) { $ExePath = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class L5TWin32 {
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
}
'@

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$log = Join-Path $OutDir "l5t_$stamp.err"
$outLog = Join-Path $OutDir "l5t_$stamp.log"
$runExe = $ExePath
$uniqProc = 'client_bevy'

if (-not $NoCopy) {
    $runExe = Join-Path $OutDir 'client_l5t.exe'
    $uniqProc = 'client_l5t'   # 唯一进程名：免疫其它 agent 的批量 Stop-Process
    $needCopy = $true
    if (Test-Path $runExe) {
        $src = Get-Item $ExePath
        $dst = Get-Item $runExe
        $needCopy = ($dst.LastWriteTime -lt $src.LastWriteTime) -or ($dst.Length -ne $src.Length)
    }
    if ($needCopy) {
        Write-Host "[l5t] 复制被测 exe → $runExe（避开别的 agent 的批量清进程）"
        Copy-Item -LiteralPath $ExePath -Destination $runExe -Force
    }
    # 只清自己那份唯一命名的残留（绝不按进程名批量杀——别的 agent 可能正在用同名副本路径之外
    # 的客户端做实验，批量 Stop-Process 会把他们的实验杀成假阴）
    Get-Process -Name $uniqProc -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $runExe } |
        ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
    Start-Sleep -Milliseconds 500
} else {
    Write-Host "[l5t] -NoCopy：直接跑 $runExe（多 agent 并行环境下请自行确认没有残留同名实例）"
}

if ($NoHandler) {
    $env:CRYSTAL_NO_RENDER_ERROR_HANDLER = '1'
    Write-Host '[l5t] 阳性对照：CRYSTAL_NO_RENDER_ERROR_HANDLER=1（不装表面错误降级策略）'
} else {
    Remove-Item Env:CRYSTAL_NO_RENDER_ERROR_HANDLER -ErrorAction SilentlyContinue
}

function Get-LogText {
    if (Test-Path $log) { Get-Content -LiteralPath $log -Raw -Encoding UTF8 } else { '' }
}

$args = @('--control-port', $ControlPort, '--window-title', 'L5T-MINIMIZE')
Write-Host "[l5t] 启动 $runExe  ws=$ClientHome\Client-Bevy  at $(Get-Date -Format HH:mm:ss)"
$p = Start-Process -FilePath $runExe -ArgumentList $args -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput $outLog -RedirectStandardError $log -PassThru

# --- A) 静置：进程本身就得活着（对照组：不是"自己死的"） ---
Start-Sleep -Seconds $WaitSec
$p.Refresh()
if ($p.HasExited) {
    Write-Host "[l5t] [A] 最小化前进程已退出 exit=$($p.ExitCode) → 不确定"
    Get-Content -LiteralPath $log -Tail 5 -ErrorAction SilentlyContinue
    exit 2
}
Write-Host "[l5t] [A] 最小化前存活 → PASS"

$hwnd = [IntPtr]::Zero
for ($i = 0; $i -lt 10 -and $hwnd -eq [IntPtr]::Zero; $i++) {
    $p.Refresh()
    if ($p.MainWindowHandle -ne 0) { $hwnd = [IntPtr]$p.MainWindowHandle } else { Start-Sleep -Milliseconds 500 }
}
if ($hwnd -eq [IntPtr]::Zero -or -not [L5TWin32]::IsWindow($hwnd)) {
    Write-Host '[l5t] 拿不到窗口句柄 → 不确定'
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 2
}

# --- B) 最小化：这正是把进程带走的动作 ---
Write-Host "[l5t] [B] hwnd=$hwnd 发 SW_MINIMIZE（SW_SHOWMINIMIZED）"
[void][L5TWin32]::ShowWindow($hwnd, 6)
Start-Sleep -Seconds $PostMinimizeSec
$p.Refresh()
$text = Get-LogText
$quitSig = $text -match 'Quitting the application due to'
$surfSig = $text -match 'ResizeBuffers failed|\[render-error\]'

if ($p.HasExited -or $quitSig) {
    Write-Host "[l5t] [B] 最小化后进程死亡 (exited=$($p.HasExited) exit=$($p.ExitCode) quiting签名=$quitSig)"
    Select-String -LiteralPath $log -Pattern 'ResizeBuffers failed|surface configuration failed|Quitting the application due to' -ErrorAction SilentlyContinue |
        Select-Object -Last 4 | ForEach-Object { Write-Host ('    ' + $_.Line) }
    if ($surfSig -or $quitSig) {
        Write-Host "[l5t] VERDICT: DEFECT PRESENT（最小化把进程带走了）exit 1  log=$log"
        exit 1
    }
    Write-Host '[l5t] 退出但无缺陷签名（疑似被外部 Stop-Process 杀掉）→ 不确定 exit 2'
    exit 2
}
Write-Host "[l5t] [B] 最小化后存活 → PASS"

if (-not $surfSig) {
    Write-Host '[l5t] [B2] 日志里没有表面配置失败签名 → 最小化没触发判据，夹具空转 → 不确定 exit 2'
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 2
}
Write-Host '[l5t] [B2] 最小化确实触发过表面配置失败 → PASS'

$suspendHits = ([regex]::Matches($text, '表面配置失败且窗口已最小化')).Count
Write-Host "[l5t] [B3] 最小化期间「暂停渲染」提示 $suspendHits 次（期望 1）"
if ($suspendHits -ne 1) {
    Write-Host '[l5t] [B3] 去抖失效（每帧刷屏）→ FAIL exit 1'
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 1
}
Write-Host '[l5t] [B3] 去抖生效 → PASS'

# --- D) 暂停的是渲染，不是整个应用：最小化期间 control RPC 仍须应答 ---
$rpcLine = ''
try {
    $tcp = New-Object Net.Sockets.TcpClient
    $tcp.ReceiveTimeout = 3000
    $tcp.Connect('127.0.0.1', [int]$ControlPort)
    $stream = $tcp.GetStream()
    $req = "{`"jsonrpc`":`"2.0`",`"id`":1,`"method`":`"state`",`"params`":{}}`n"
    $bytes = [Text.Encoding]::UTF8.GetBytes($req)
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Flush()
    $rpcLine = (New-Object IO.StreamReader($stream)).ReadLine()
    $tcp.Close()
} catch {
    $rpcLine = ''
}
if ($rpcLine -notmatch '"jsonrpc"') {
    Write-Host "[l5t] [D] 最小化期间 control RPC 无应答（端口 $ControlPort 可能被占）→ 不确定 exit 2"
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 2
}
Write-Host "[l5t] [D] 最小化期间 RPC 仍应答（主世界没被冻住）→ PASS  $rpcLine"

# --- C) 恢复：渲染必须回来（不然只是"卡死不报错"） ---
Write-Host '[l5t] [C] 发 SW_RESTORE'
[void][L5TWin32]::ShowWindow($hwnd, 9)
Start-Sleep -Seconds $RestoreSec
$p.Refresh()
$text = Get-LogText
$resumed = $text -match '\[render-error\] 窗口已恢复：渲染继续'

if ($p.HasExited) {
    Write-Host "[l5t] [C] 恢复后进程已退出 exit=$($p.ExitCode) → FAIL"
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 1
}
if (-not $resumed) {
    Write-Host '[l5t] [C] 恢复后未见「窗口已恢复：渲染继续」→ 渲染没被放回来 → FAIL exit 1'
    Select-String -LiteralPath $log -Pattern '\[render-error\]' -ErrorAction SilentlyContinue |
        Select-Object -Last 3 | ForEach-Object { Write-Host ('    ' + $_.Line) }
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 1
}
Write-Host '[l5t] [C] 恢复后存活且渲染已继续 → PASS'

Select-String -LiteralPath $log -Pattern '\[render-error\]|ResizeBuffers failed' -ErrorAction SilentlyContinue |
    Select-Object -Last 4 | ForEach-Object { Write-Host ('    ' + $_.Line) }
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Write-Host '=== 全部 PASS ==='
exit 0
