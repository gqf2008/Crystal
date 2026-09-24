# Keyboard-only driver for the ORIGINAL C# client: login -> character select -> in-game,
# then open the keybind windows (F9 inventory / F10 equipment / F11 skills) for A/B capture.
#
# Why keyboard-only: while the Windows session is locked, real mouse input goes to the lock screen and
# WinForms never raises MouseClick for injected WM_LBUTTONDOWN/UP, so the mirror UI's buttons cannot be
# clicked (see README §3). Keyboard works because LoginDialog.TextBox_KeyPress (Enter) calls
# OKButton.InvokeMouseClick, and SelectScene_KeyPress (Enter) calls StartGame() with character index 0.
#
# Usage:
#   pwsh -NoProfile -File .\csharp_kbd_login.ps1 -SandboxRoot <dir> -Account 333 -Password abbtest123

param(
  [string]$SandboxRoot = $env:CRYSTAL_CSHARP_SANDBOX,
  [string]$Account = '333',
  [string]$Password = 'abbtest123',
  [int]$LoginWaitSeconds = 8,
  [int]$StartWaitSeconds = 10
)

if (-not $SandboxRoot) { throw 'pass -SandboxRoot <dir> (or set CRYSTAL_CSHARP_SANDBOX)' }
. "$PSScriptRoot\csharp_client_driver.ps1" -SandboxRoot $SandboxRoot

Add-Type @"
using System;using System.Runtime.InteropServices;
public class CsKbd {
  [DllImport("user32.dll", CharSet=CharSet.Unicode, EntryPoint="SendMessageW")]
  public static extern IntPtr SendMessageStr(IntPtr h, uint msg, IntPtr w, string l);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint tid, ref GTI g);
  public struct GTI { public int cbSize; public int flags;
    public IntPtr hwndActive, hwndFocus, hwndCapture, hwndMenuOwner, hwndMoveSize, hwndCaret; public RECT rcCaret; }
  public struct RECT { public int Left,Top,Right,Bottom; }
  public static long Focus(uint tid){ var g=new GTI(); g.cbSize=Marshal.SizeOf(g); GetGUIThreadInfo(tid, ref g); return (long)g.hwndFocus; }
}
"@

function Send-Key([IntPtr]$target,[byte]$vk,[int]$holdMs=120){
  [void][CsKbd]::SendMessage($target,0x0100,[IntPtr]$vk,[IntPtr]::Zero)   # WM_KEYDOWN
  Start-Sleep -Milliseconds $holdMs
  [void][CsKbd]::SendMessage($target,0x0102,[IntPtr]$vk,[IntPtr]::Zero)   # WM_CHAR (mirror TextBox KeyPress)
  Start-Sleep -Milliseconds $holdMs
  [void][CsKbd]::SendMessage($target,0x0101,[IntPtr]$vk,[IntPtr]0xC0000001)  # WM_KEYUP
  Start-Sleep -Milliseconds $holdMs
}

function Get-LoginBoxes {
  $edits = @()
  for ($i = 0; $i -lt 15; $i++) { $edits = @(Get-CsEdits); if ($edits.Count -ge 2) { break }; Start-Sleep -Seconds 2 }
  $parsed = $edits | ForEach-Object {
    $m = [regex]::Match($_, 'hwnd=0x([0-9A-F]+) vis=\S+ rect=\((\d+),(\d+)\)')
    if ($m.Success) { [pscustomobject]@{ Hwnd = [Convert]::ToInt64($m.Groups[1].Value, 16); Y = [int]$m.Groups[3].Value } }
  } | Sort-Object Y
  return $parsed
}

Write-Host '=== start original client ==='
Get-Process Client -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Start-Process -FilePath "$script:CS\Client\Client.exe" -WorkingDirectory "$script:CS\Client"
Start-Sleep -Seconds 22
Init-CsClient

$boxes = Get-LoginBoxes
if ($boxes.Count -lt 2) { throw "login text boxes not found (got $($boxes.Count))" }
$idBox = [IntPtr]$boxes[0].Hwnd      # smaller Y = account box
$pwBox = [IntPtr]$boxes[-1].Hwnd
Write-Host ("login boxes: id=0x{0:X} pw=0x{1:X}" -f $idBox.ToInt64(), $pwBox.ToInt64())

[void][CsKbd]::SendMessageStr($idBox, 0x000C, [IntPtr]::Zero, $Account)    # WM_SETTEXT
Start-Sleep -Milliseconds 500
[void][CsKbd]::SendMessageStr($pwBox, 0x000C, [IntPtr]::Zero, $Password)
Start-Sleep -Milliseconds 700

$pid2 = 0
$tid = [CsKbd]::GetWindowThreadProcessId($global:csHwnd, [ref]$pid2)
$focus = [CsKbd]::Focus($tid)
$target = if ($focus -ne 0) { [IntPtr]$focus } else { $pwBox }

Write-Host '=== Enter #1: login (LoginDialog.TextBox_KeyPress -> OKButton.InvokeMouseClick) ==='
Send-Key $target 0x0D
Start-Sleep -Seconds $LoginWaitSeconds
Shot-Cs 'kbd_01_select'

Write-Host '=== Enter #2: start game (SelectScene_KeyPress -> StartGame, character index 0) ==='
Send-Key $global:csHwnd 0x0D
Start-Sleep -Seconds $StartWaitSeconds
Shot-Cs 'kbd_02_ingame'

Write-Host '=== keybind windows: F9 inventory / F10 equipment / F11 skills ==='
foreach ($kb in @(@(0x78,'F9_inventory'), @(0x79,'F10_equipment'), @(0x7A,'F11_skills'))) {
  [void][CsKbd]::SendMessage($global:csHwnd, 0x0100, [IntPtr][byte]$kb[0], [IntPtr]::Zero)
  Start-Sleep -Milliseconds 120
  [void][CsKbd]::SendMessage($global:csHwnd, 0x0101, [IntPtr][byte]$kb[0], [IntPtr]0xC0000001)
  Start-Sleep -Seconds 1
  Shot-Cs "ingame_$($kb[1])"
}

Write-Host '=== server log tail (expect "User logged in" + "<char> has connected") ==='
Get-ChildItem "$script:CS\Server\Logs\Server" -File | Sort-Object LastWriteTime -Descending |
  Select-Object -First 1 | ForEach-Object { Get-Content $_.FullName -Tail 8 }
