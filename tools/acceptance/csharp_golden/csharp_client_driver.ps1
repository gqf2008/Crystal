# Driver for the ORIGINAL C# Crystal client (see README: sandbox copy on a free port).
# Usage: . .\csharp_client_driver.ps1 -SandboxRoot <dir>; Init-CsClient
param([string]$SandboxRoot = $env:CRYSTAL_CSHARP_SANDBOX)
if (-not $SandboxRoot) { throw 'pass -SandboxRoot <dir> (or set CRYSTAL_CSHARP_SANDBOX)' }
$script:CS = $SandboxRoot

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;using System.Text;using System.Collections.Generic;using System.Runtime.InteropServices;
public class CsUi {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,IntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr wp, IntPtr lp);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr wp, IntPtr lp);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumWindowsProc f, IntPtr l);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x,int y,int cx,int cy,uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  public struct RECT { public int Left,Top,Right,Bottom; }
  public struct POINT { public int X,Y; }
  public static int[] WRect(long h){ RECT r; GetWindowRect((IntPtr)h, out r); return new int[]{r.Left,r.Top,r.Right-r.Left,r.Bottom-r.Top}; }
  public static List<string> Kids(long p){
    var res=new List<string>();
    EnumChildWindows((IntPtr)p,(h,l)=>{ var cn=new StringBuilder(256); GetClassName(h,cn,256); var sb=new StringBuilder(512); GetWindowText(h,sb,512);
      RECT r; GetWindowRect(h,out r);
      res.Add(string.Format("hwnd=0x{0:X} vis={1} rect=({2},{3})-({4},{5}) class={6} text='{7}'",(long)h,IsWindowVisible(h),r.Left,r.Top,r.Right,r.Bottom,cn,sb)); return true;}, IntPtr.Zero);
    return res; }
  public static bool Topmost(long h){ return (GetWindowLongSafe(h) & 0x8) != 0; }
  [DllImport("user32.dll", EntryPoint="GetWindowLongPtr")] static extern IntPtr GetWindowLongPtr64(IntPtr h,int i);
  [DllImport("user32.dll", EntryPoint="GetWindowLong")] static extern IntPtr GetWindowLong32(IntPtr h,int i);
  static int GetWindowLongSafe(long h){ var v = IntPtr.Size==8 ? GetWindowLongPtr64((IntPtr)h,-20) : GetWindowLong32((IntPtr)h,-20); return (int)v.ToInt64(); }
}
"@

function Get-CsHwnd {
  # 认**本沙盒自己那份**原版客户端：按 exe 路径过滤，不要"按名取第一个"——
  # 同机可能有别人/owner 的原版客户端在做 A/B 对照，取错了会把别人的窗口当成我们的测。
  $csExe = Join-Path $script:CS 'Client\Client.exe'
  $p = Get-CimInstance Win32_Process -Filter "Name='Client.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.ExecutablePath -eq $csExe } | Select-Object -First 1
  if (-not $p) { throw "original client not running (expected $csExe)" }
  $p = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
  if (-not $p) { throw "original client not running (expected $csExe)" }
  if (-not ('CsUiNs.FindW' -as [type])) {
    [void](Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc f, IntPtr l); public delegate bool EnumWindowsProc(IntPtr h, IntPtr l); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid); [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);' -Name 'FindW' -Namespace 'CsUiNs' -PassThru)
  }
  $global:csHwnd = [IntPtr]::Zero
  $cb = [CsUiNs.FindW+EnumWindowsProc]{
    param($h,$l)
    $pid2 = 0
    [void][CsUiNs.FindW]::GetWindowThreadProcessId($h,[ref]$pid2)
    if ($pid2 -eq $p.Id -and [CsUiNs.FindW]::IsWindowVisible($h)) {
      $r = [CsUi]::WRect($h.ToInt64())
      if ($r[2] -ge 800 -and $r[3] -ge 600) { $global:csHwnd = $h; return $false }
    }
    return $true
  }
  [void][CsUiNs.FindW]::EnumWindows($cb,[IntPtr]::Zero)
  if ($global:csHwnd -eq [IntPtr]::Zero) { throw 'game window not found' }
  return $global:csHwnd
}

function Init-CsClient {
  $h = Get-CsHwnd
  # keep the game window above the terminals so real input lands on it
  [void][CsUi]::SetWindowPos($h, [IntPtr](-1), 0,0,0,0, (0x2 -bor 0x1 -bor 0x40))
  Start-Sleep -Milliseconds 400
  Write-Host "client hwnd=0x$($h.ToInt64().ToString('X')) topmost=$([CsUi]::Topmost($h.ToInt64())) rect=$([CsUi]::WRect($h.ToInt64()) -join ',')"
}

function Click-Image([int]$ix,[int]$iy,[int]$n=1) {
  $h = $global:csHwnd
  $r = [CsUi]::WRect($h.ToInt64())
  $sx = $r[0] + $ix; $sy = $r[1] + $iy
  for ($k=0; $k -lt $n; $k++) {
    [void][CsUi]::SetCursorPos($sx,$sy)
    Start-Sleep -Milliseconds 250
    [CsUi]::mouse_event(0x0002,0,0,0,[IntPtr]::Zero)   # LEFTDOWN
    Start-Sleep -Milliseconds 90
    [CsUi]::mouse_event(0x0004,0,0,0,[IntPtr]::Zero)   # LEFTUP
    Start-Sleep -Milliseconds 250
  }
}

function Move-Image([int]$ix,[int]$iy) {
  $h = $global:csHwnd
  $r = [CsUi]::WRect($h.ToInt64())
  [void][CsUi]::SetCursorPos(($r[0]+$ix),($r[1]+$iy))
  Start-Sleep -Milliseconds 300
}

function Msg-Click([int]$ix,[int]$iy,[int]$holdMs=250) {
  $h = $global:csHwnd
  $lp=[IntPtr](($iy -shl 16) -bor ($ix -band 0xFFFF))
  [void][CsUi]::SendMessage($h,0x0200,[IntPtr]::Zero,$lp)
  Start-Sleep -Milliseconds 200
  [void][CsUi]::SendMessage($h,0x0201,[IntPtr]1,$lp)
  Start-Sleep -Milliseconds $holdMs
  [void][CsUi]::SendMessage($h,0x0202,[IntPtr]0,$lp)
  Start-Sleep -Milliseconds 300
}

function Key-Cs([byte]$vk,[int]$holdMs=90) {
  [CsUi]::keybd_event($vk,0,0,[IntPtr]::Zero)
  Start-Sleep -Milliseconds $holdMs
  [CsUi]::keybd_event($vk,0,2,[IntPtr]::Zero)
  Start-Sleep -Milliseconds 200
}

function Msg-Key([byte]$vk,[int]$holdMs=90) {
  # 与 `Key-Cs` 的区别：`keybd_event` 发给**前台窗口**（脚本一退出、焦点回终端，
  # 按键就打到终端上了 —— 实测同一脚本里 F9 有时"生效"有时"没反应"，就是焦点在谁那）。
  # 这条把 WM_KEYDOWN/WM_KEYUP **直接 SendMessage 给客户端 hwnd**，不依赖焦点，
  # 与 `csharp_kbd_login.ps1` 的按键路径同源（同为 `MirControl`/`MirScene` 的消息入口）。
  $h = $global:csHwnd
  [void][CsUi]::SendMessage($h, 0x0100, [IntPtr][int]$vk, [IntPtr]::Zero)
  Start-Sleep -Milliseconds $holdMs
  [void][CsUi]::SendMessage($h, 0x0101, [IntPtr][int]$vk, [IntPtr]0xC0000001)
  Start-Sleep -Milliseconds 250
}

function Msg-Wheel([int]$x,[int]$y,[int]$delta=-3) {
  # 注入滚轮：`WM_MOUSEWHEEL(0x020A)`，wParam 高位是 delta（一格 = 120；正 = 向上），
  # lParam 是**屏幕坐标**（所以要把窗口摆到 (0,0,1024,768)，屏幕坐标才等于客户坐标）。
  #
  # **调用前必须先把光标移到目标上**（`Move-Image $x $y`）：C# 的滚轮派发链是
  # `CMain_MouseWheel`（Forms 事件）→ `MirScene.OnMouseWheel` → **`MouseControl.OnMouseWheel`**
  # （`Client/MirControls/MirScene.cs:136-144`、`Client/Forms/CMain.cs:65/312`），
  # 而 `MouseControl` 是"光标当前所在控件"——不先移光标，滚轮就没有落点（实测：同一坐标先移光标
  # 则大地图 NPC 列表会滚，不移则一动不动）。
  #
  # 阳性对照（2026-09-28 实测）：大地图（B 键）的 NPC 列表随滚轮变化（窗区 408 px 不同，
  # `Client/MirScenes/Dialogs/BigMapDialog.cs:368-390`）；阴性：NPC 窗文本行数 ≤ `MaximumLines(8)`
  # 时按 C# 守卫不动。
  $h = $global:csHwnd
  $wp = [IntPtr](($delta -band 0xFFFF) -shl 16)
  $lp = [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
  [void][CsUi]::SendMessage($h, 0x020A, $wp, $lp)
  Start-Sleep -Milliseconds 200
}

function Msg-Drag([int]$x,[int]$y,[int]$dx,[int]$dy,[int]$steps=6,[int]$holdMs=120) {
  # 注入"按下-移动-抬起"（滑条/滚动条这类取 `OnMoving` 的控件要用它，`Msg-Click` 只按一下不动）：
  #   WM_MOUSEMOVE(0x0200) → WM_LBUTTONDOWN(0x0201, wParam=1) →
  #   中间若干步 WM_MOUSEMOVE（插值）→ WM_LBUTTONUP(0x0202, wParam=0)
  # 与 `Msg-Click` 同坐标系：**客户区坐标**（窗口在 (0,0,1024,768) 时等于屏幕坐标）。
  # 实证（2026-09-28）：商城窗 `GameShopDialog.PositionBar`（面板 (164,146) + 面板内 (120,117)
  # ⇒ 屏幕 (284,263)）拖 60px 后，商品列表/滑条位移 —— C# 侧处理器是 `PositionBar_OnMoving`
  # （`Client/MirScenes/Dialogs/GameshopDialog.cs:143-155`）。
  $h = $global:csHwnd
  $pt = { param($px, $py) [IntPtr](([int]$py -shl 16) -bor ([int]$px -band 0xFFFF)) }
  [void][CsUi]::SendMessage($h, 0x0200, [IntPtr]::Zero, (& $pt $x $y))
  Start-Sleep -Milliseconds 120
  [void][CsUi]::SendMessage($h, 0x0201, [IntPtr]1, (& $pt $x $y))
  Start-Sleep -Milliseconds $holdMs
  for ($i = 1; $i -le $steps; $i++) {
    $px = [int]($x + $dx * $i / $steps)
    $py = [int]($y + $dy * $i / $steps)
    [void][CsUi]::SendMessage($h, 0x0200, [IntPtr]1, (& $pt $px $py))
    Start-Sleep -Milliseconds 120
  }
  [void][CsUi]::SendMessage($h, 0x0202, [IntPtr]0, (& $pt ($x + $dx) ($y + $dy)))
  Start-Sleep -Milliseconds 250
}

function Shot-Cs([string]$label) {
  $h = $global:csHwnd
  $dir = "$script:CS\Client\Screenshots"
  # 原版截图目录与我们的落盘目录都**先建**：`Copy-Item` 到不存在的 `shots\` 会让后面
  # `Image::FromFile` 抛 `InvalidOperation`（"shots/ 根本没建"就是实测踩到的那个坑）。
  if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $shots = "$script:CS\shots"
  if (-not (Test-Path -LiteralPath $shots)) { New-Item -ItemType Directory -Force -Path $shots | Out-Null }
  $beforeNames = @(Get-ChildItem $dir -Filter *.png -ErrorAction SilentlyContinue | ForEach-Object Name)
  $lp = [IntPtr]1
  [void][CsUi]::SendMessage($h, 0x0100, [IntPtr]0x2C, $lp)
  Start-Sleep -Milliseconds 90
  [void][CsUi]::SendMessage($h, 0x0101, [IntPtr]0x2C, [IntPtr]0xC0000001)
  # **断言源图已落盘**：D3D 的 F12 落盘有延迟，旧实现只看"最新一个 png"，会在新图还没写出来时
  # 把上一张当成这一张（或读到半截文件）。改判据为「**按文件名新增**的那一张，且长度 > 0」，
  # 最多等 ~4s；等不到就明确 FAIL，别返回一个看起来成功的旧图。
  $newest = $null
  foreach ($try in 1..10) {
    Start-Sleep -Milliseconds 400
    $newest = Get-ChildItem $dir -Filter *.png -ErrorAction SilentlyContinue |
      Where-Object { $beforeNames -notcontains $_.Name -and $_.Length -gt 0 } |
      Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($newest) { break }
  }
  if (-not $newest) { return "FAIL: 4s 内未出现新截图（$dir）——原版 F12 未生效或落盘更慢" }
  $out = Join-Path $shots "orig_$label.png"
  Copy-Item -LiteralPath $newest.FullName -Destination $out -Force
  if (-not (Test-Path -LiteralPath $out)) { return "FAIL: 复制后 $out 不存在" }
  $img=[System.Drawing.Image]::FromFile($out); $dim="$($img.Width)x$($img.Height)"; $img.Dispose()
  return "$out ($dim, from $($newest.Name))"
}

function Get-CsEdits {
  foreach ($k in [CsUi]::Kids($global:csHwnd.ToInt64())) {
    if ($k -match 'WindowsForms10\.Edit') { $k }
  }
}
