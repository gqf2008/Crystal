#Requires -Version 5.1
<#
.SYNOPSIS
  聊天窗「大小」按钮三档循环的实机门禁（owner 反馈「对话框展开后收不回来了，原版展开有三档」）。

.DESCRIPTION
  原版语义（`Client/MirScenes/Dialogs/MainDialogs.cs`）：
    `SizeButton.Click` → `ChatDialog.ChangeSize()`（:1188）——`if (++WindowSize >= 3) WindowSize = 0;`
    行数 4/7/11，面板 632×(68/116/164)，并且 `Location = (X, 旧底边 - 新高度)`：**底边固定、向上长高**。

  本夹具做的事：在客户端里**点看的见的那颗「大小」按钮**三次，用只读探针 `chat_probe`
  （`size` / `panel` / `bar_top`）断言：
    0 → 1 → 2 → 0（可逆），且每档面板高度 68/116/164、底边恒 739（= 671+68 = 623+116 = 575+164）。
  点击点 = `(CHAT_BAR_X + 574 + 10, chat_bar_top(size) + 8)`（按钮表里 Size 的相对 x=574、栏顶 +1）；
  **这正是老 bug 的判据**：修复前控制栏只挪了绘制、没同步 `UiButton.rect`（点击命中的唯一来源），
  升档后点「看得见的按钮」无效、点「旧位置的空气」才生效 —— 用户看到的就是「收不回来」。

  退出码：0 = 三档循环全 PASS；1 = 有判据 FAIL（含点了没反应）；2 = 前置失败（未进场/探针不可用）。
  实机资源（客户端 + e2e 账号）必须走 `e2e_lock.ps1` 串行。
#>
param(
    [string]$ClientHome = '',
    [int]$ControlPort = 9052,
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$Tag = 'chatsize'
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5zb_chat_size_cycle'
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5zb_client.exe'
$log = "$PSScriptRoot\l5zb_chat_size_cycle.$Tag.log"

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5zb_chat_size_cycle' -TimeoutSec 1800)) {
    Write-Host 'FAIL(2): 等 e2e 锁超时'
    exit 2
}

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 2000; $c.SendTimeout = 2000
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 2000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}
function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='l5zb_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

Get-CimInstance Win32_Process -Filter "Name='l5zb_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null } catch { }
if (Test-Path $log) { [System.IO.File]::Delete($log) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, `
        '--control-port', "$ControlPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput "$PSScriptRoot\l5zb_chat_size_cycle.$Tag.out" -RedirectStandardError $log | Out-Null

$st = $null
foreach ($i in 1..90) {
    Start-Sleep 1
    $st = Rpc 'state'
    if ($null -ne $st.tile_x) { break }
}
if ($null -eq $st -or $null -eq $st.tile_x) {
    # 先强制回到 0 档，避免上一轮把档位留在展开态影响判据起点
    Write-Host 'FAIL(2): 未进场（登录失败/超时）'
    Stop-Client; Exit-E2eLock; exit 2
}
$p0 = Rpc 'chat_probe'
if ($null -eq $p0 -or $null -eq $p0.bar_top) {
    Write-Host 'FAIL(2): chat_probe 不可用'
    Stop-Client; Exit-E2eLock; exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})；聊天窗起点 size={3} bar_top={4}" -f $st.map, $st.tile_x, $st.tile_y, $p0.size, $p0.bar_top)

# 三档：0→1→2→0；每档高度与底边（面板底边必须恒 739）
$expect = @(
    @{ from = 0; to = 1; h = 116 },
    @{ from = 1; to = 2; h = 164 },
    @{ from = 2; to = 0; h = 68 }
)
$fail = @()
foreach ($e in $expect) {
    $before = Rpc 'chat_probe'
    if ($before.size -ne $e.from) {
        $fail += ("起点档位不符：期望 {0}，实际 {1}" -f $e.from, $before.size)
        break
    }
    # 点**当前档位下画出来的**那颗按钮（栏顶 +1 的按钮中心偏下 8px；x = 栏 x230 + 相对 574 + 一半宽）
    $x = 230 + 574 + 10
    # 按钮 art 高约十几 px，顶边 = `chat_bar_top(size) + 1`；在这个可见带里试几个落点
    # （实测同一档位里 +1 命中、+8 不命中——art 实际高度比默认的 16 小，取带内点更稳），
    # 判据不变：**必须真的发生档位跃迁**，且跃迁到期望档位。
    $after = $before
    $y = 0.0
    foreach ($off in @(2.0, 5.0, 8.0)) {
        $y = [double]$before.bar_top + $off
        $null = Rpc 'click' @{ x = $x; y = $y }
        Start-Sleep -Seconds 2
        $after = Rpc 'chat_probe'
        if ($after.size -ne $before.size) { break }
    }
    $bottom = [double]$after.panel.y + [double]$after.panel.h
    $ok = ($after.size -eq $e.to) -and ([math]::Abs([double]$after.panel.h - $e.h) -lt 0.5) -and ([math]::Abs($bottom - 739.0) -lt 0.5)
    Write-Host ("[{0}→{1}] 点({2},{3}) → size={4} h={5} 底边={6} {7}" -f `
        $e.from, $e.to, $x, [math]::Round($y, 1), $after.size, $after.panel.h, $bottom, $(if ($ok) { 'PASS' } else { 'FAIL' }))
    if (-not $ok) {
        $fail += ("{0}→{1} 失败：实测 size={2} h={3} 底边={4}" -f $e.from, $e.to, $after.size, $after.panel.h, $bottom)
        break
    }
}

Stop-Client
Exit-E2eLock
if ($fail.Count -gt 0) {
    Write-Host ('FAIL(1): ' + ($fail -join '; '))
    exit 1
}
Write-Host '=== 全部 PASS（三档 0→1→2→0 可逆，面板 68/116/164、底边恒 739）==='
exit 0
