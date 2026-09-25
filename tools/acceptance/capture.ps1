# OS 级窗口客户区截屏（用于 Login/Select 等控制接口截图不可用的状态）
# 用法: .\capture.ps1 标签名   → shots/<标签名>.png
# 方案：PrintWindow(PW_RENDERFULLCONTENT) 走 DWM 合成纹理，窗口被遮挡/无前台也能截。
#       失败（黑图风险）时回退 SetForegroundWindow + CopyFromScreen。
param(
    [Parameter(Mandatory = $true)][string]$Label,
    # 多 agent 并行时公共名 `client_bevy` 可能是别人的会话（BATCH #3181 起夹具都用唯一命名副本）；
    # 默认值保持兼容，但建议显式传自己的唯一名。
    [string]$ProcessName = 'client_bevy'
)

$ErrorActionPreference = 'Stop'
# **必须在任何窗口度量之前**声明 DPI aware（2026-09-26 实测）：本机 150% DPI 下，
# DPI-unaware 的宿主拿到的窗口矩形是**虚拟化**的（真实 1536×1152 物理窗口只报 1039×805），
# PrintWindow 也只截到被缩小/裁剪的那张 —— 实测**底部整条 UI（聊天面板、底栏）都不在截图里**，
# 而客户端自报 `physical=1536x1152 logical=1024x768 scale=1.5`（什么都没裁）。
# 这会让人误判成"UI 被窗口裁了"，把截图工具的问题算到产品头上（线程
# `crystal-chat-panel-screenshot-mismatch` 就是被它带偏的）。
if (-not ('Win32DpiAware' -as [type])) {
    Add-Type -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();' -Name Win32DpiAware -Namespace Probe
}
[void][Probe.Win32DpiAware]::SetProcessDPIAware()

Add-Type -AssemblyName System.Drawing
if (-not ('Win32Cap3' -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Cap3 {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  public struct RECT { public int L, T, R, B; }
  public struct POINT { public int X, Y; }
}
"@
}
$p = Get-Process -Name $ProcessName -ErrorAction Stop | Select-Object -First 1
$h = $p.MainWindowHandle
if ($h -eq [IntPtr]::Zero) { throw "client_bevy 无主窗口" }
$cr = New-Object Win32Cap3+RECT
[Win32Cap3]::GetClientRect($h, [ref]$cr) | Out-Null
$w = $cr.R - $cr.L; $ht = $cr.B - $cr.T
if ($w -le 0 -or $ht -le 0) { throw "客户区尺寸异常 ${w}x${ht}" }
$outDir = Join-Path $PSScriptRoot 'shots'
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory $outDir | Out-Null }
$out = Join-Path $outDir "$Label.png"

$bmp = New-Object System.Drawing.Bitmap($w, $ht)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
# PW_CLIENTONLY(0x1) | PW_RENDERFULLCONTENT(0x2)
$ok = [Win32Cap3]::PrintWindow($h, $hdc, 0x3)
$g.ReleaseHdc($hdc)

if (-not $ok) {
  # 回退：前台 + 屏幕拷贝（有遮挡窗口风险，写 WARNING）
  $fg = [IntPtr]::Zero
  foreach ($i in 1..6) {
    [Win32Cap3]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 350
    $fg = [Win32Cap3]::GetForegroundWindow()
    if ($fg -eq $h) { break }
  }
  if ($fg -ne $h) { Write-Warning "PrintWindow 失败且未能置前台——截图可能含遮挡窗口" }
  $pt = New-Object Win32Cap3+POINT
  [Win32Cap3]::ClientToScreen($h, [ref]$pt) | Out-Null
  $g.CopyFromScreen($pt.X, $pt.Y, 0, 0, $bmp.Size)
}
$g.Dispose()
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Host "🖼️ $Label -> $out (${w}x$ht)"
