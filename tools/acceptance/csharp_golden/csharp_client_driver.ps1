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
  $p = Get-Process Client -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $p) { throw 'original client not running' }
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

function Shot-Cs([string]$label) {
  $h = $global:csHwnd
  $dir = "$script:CS\Client\Screenshots"
  $before = @(Get-ChildItem $dir -Filter *.png -ErrorAction SilentlyContinue).Count
  $lp = [IntPtr]1
  [void][CsUi]::SendMessage($h, 0x0100, [IntPtr]0x2C, $lp)
  Start-Sleep -Milliseconds 90
  [void][CsUi]::SendMessage($h, 0x0101, [IntPtr]0x2C, [IntPtr]0xC0000001)
  Start-Sleep -Milliseconds 1800
  $files = Get-ChildItem $dir -Filter *.png -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
  if (-not $files) { return 'no screenshot produced' }
  $newest = $files[0]
  $out = "$script:CS\shots\orig_$label.png"
  Copy-Item $newest.FullName $out -Force
  $img=[System.Drawing.Image]::FromFile($out); $dim="$($img.Width)x$($img.Height)"; $img.Dispose()
  return "$out ($dim, from $($newest.Name))"
}

function Get-CsEdits {
  foreach ($k in [CsUi]::Kids($global:csHwnd.ToInt64())) {
    if ($k -match 'WindowsForms10\.Edit') { $k }
  }
}
