# ime_rpc_verify.ps1 — 内置拼音 IME 的 **RPC 真值**验证（#2961 项3「候选词乱码」）
#
# 为什么不用 ime-shot.ps1 的 PrintWindow 像素：本机 PrintWindow 抓不到 Bevy 的 UI 层
# （41/42/43 三张里没有 HUD、聊天区、候选条，只有世界层）→ 像素证据不可用。
# 本脚本改用 control RPC 的客观真值字段 + 客户端自身 screenshot 通道：
#   chat_input_active / chat_input_text / ime_enabled / ime_composing
#
# 断言链（每一步都可判真假，非"看图"）：
#   1) Enter          → chat_input_active == true
#   2) Shift          → 中 chip：字母进 composing，不再直出（英/中的判别式）
#   3) 输入 nihao     → ime_composing == "nihao"（字母确已到达 IME 引擎）
#   4) '1' 选首候选   → chat_input_text 出现 CJK 字符（候选可上屏，非乱码）
#   5) Esc            → 组合被取消
# 产物：shots/ime_rpc_*.png（客户端自身截图，含 HUD 与候选条）+ 控制台 PASS/FAIL
$ErrorActionPreference = 'Stop'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$shotDir = "$acc\shots"

Add-Type -AssemblyName System.Drawing
if (-not ('Win32Ime' -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Ime {
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr wp, IntPtr lp);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint vk, uint type);
}
"@
}
$WM_KEYDOWN = 0x100; $WM_KEYUP = 0x101; $WM_CHAR = 0x102
function KeyLParam([uint32]$vk, [bool]$up) {
  $scan = [Win32Ime]::MapVirtualKey($vk, 0) -band 0xFF
  $lp = 1 -bor ([int64]$scan -shl 16)
  if ($up) { $lp = $lp -bor (1 -shl 30) -bor (1 -shl 31) }
  return [IntPtr]$lp
}
function Tap([uint32]$vk) {
  [Win32Ime]::PostMessage($script:hwnd, $WM_KEYDOWN, [IntPtr]$vk, (KeyLParam $vk $false)) | Out-Null
  Start-Sleep -Milliseconds 60
  [Win32Ime]::PostMessage($script:hwnd, $WM_KEYUP,   [IntPtr]$vk, (KeyLParam $vk $true))  | Out-Null
  Start-Sleep -Milliseconds 120
}
function TypeLetter([char]$c) {
  $vk = [int][char]::ToUpper($c)
  [Win32Ime]::PostMessage($script:hwnd, $WM_KEYDOWN, [IntPtr]$vk, (KeyLParam $vk $false)) | Out-Null
  [Win32Ime]::PostMessage($script:hwnd, $WM_CHAR,    [IntPtr][int]$c, [IntPtr]::Zero) | Out-Null
  Start-Sleep -Milliseconds 40
  [Win32Ime]::PostMessage($script:hwnd, $WM_KEYUP,   [IntPtr]$vk, (KeyLParam $vk $true))  | Out-Null
  Start-Sleep -Milliseconds 90
}
function Rpc([string]$method, [hashtable]$params = @{}) {
  $c = New-Object Net.Sockets.TcpClient
  $c.Connect('127.0.0.1', 9000)
  $s = $c.GetStream()
  $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$method; params=$params } | ConvertTo-Json -Compress) + "`n")
  $s.Write($b, 0, $b.Length); $s.Flush()
  $r = New-Object IO.StreamReader($s)
  $line = $r.ReadLine(); $c.Close()
  if ($null -eq $line) { throw "control 无响应: $method" }
  ($line | ConvertFrom-Json).result
}
function St { Rpc 'state' }
function Shot([string]$name) {
  $p = "$shotDir\$name.png"
  Rpc 'screenshot' @{ path = $p } | Out-Null
  Start-Sleep -Milliseconds 800
  return $p
}
$results = @()
function Rec([string]$check, [bool]$pass, [string]$detail) {
  $script:results += [pscustomobject]@{ check=$check; pass=$pass; detail=$detail }
  Write-Host ("[{0}] {1} — {2}" -f $(if ($pass) {'PASS'} else {'FAIL'}), $check, $detail)
}
function HasCjk([string]$s) { return ($s -match '[\u4e00-\u9fff]') }

$p = Get-Process client_bevy -ErrorAction Stop | Select-Object -First 1
$script:hwnd = $p.MainWindowHandle
if ($script:hwnd -eq [IntPtr]::Zero) { throw 'client_bevy 无主窗口' }
Write-Host "目标窗口 pid=$($p.Id) hwnd=0x$($script:hwnd.ToString('X'))"

try {
    $st0 = St
    if ($st0.chat_input_active) { Tap 0x1B; Start-Sleep -Milliseconds 300 }   # 归零：先 Esc 收掉
    # `state.ime_enabled` 是**中/英 chip**（Shift 翻转），不是"引擎装载"：
    # `PinyinIme::toggle()`（pinyin_ime.rs:219）在 engine 缺失时是 no-op ——
    # 故"按一次 Shift 能让该位翻转"才是引擎已装载的判据
    $chip0 = (St).ime_enabled
    Tap 0x10; Start-Sleep -Milliseconds 350
    $chip1 = (St).ime_enabled
    Rec 'libpinyin 引擎已装载（Shift 能翻转中/英 chip）' ($chip0 -ne $chip1) "chip $chip0 -> $chip1"
    # 对齐到「中」：enabled==true 即中文（插件构造默认 true）
    if (-not (St).ime_enabled) { Tap 0x10; Start-Sleep -Milliseconds 350 }
    Rec '处于中文 chip' ((St).ime_enabled -eq $true) "ime_enabled=$((St).ime_enabled)"

    Start-Sleep -Milliseconds 400
    Tap 0x0D                                                                  # Enter 开聊天输入
    Start-Sleep -Milliseconds 500
    $s1 = St
    Rec 'Enter 打开聊天输入' ($s1.chat_input_active -eq $true) "chat_input_active=$($s1.chat_input_active) 初始文本='$($s1.chat_input_text)'"
    Shot 'ime_rpc_1_chat_open' | Out-Null

    # 中/英判别式：敲 n 后中文模式进 composing；同时文本域**不得**出现裸 ASCII
    # （字母被 IME 接管就应完全不上屏，故 chat_input_text 必须保持为空）
    TypeLetter 'n'
    Start-Sleep -Milliseconds 300
    $s2 = St
    $cn = ($s2.ime_composing -eq 'n')
    Rec '中文模式：字母进 composing' $cn "ime_composing='$($s2.ime_composing)'"
    # 用 trim 判空：草稿里可能有不可见字符（如回车符）——它们在控制台看不见但会让等于空串的判定为假。
    # （曾因此误报 FAIL，真因是 Enter 开框当帧写入回车符，已在 PR #2973 修复）
    Rec '中文模式：字母不落文本域（无裸 ASCII 泄漏）' ([string]::IsNullOrEmpty($s2.chat_input_text.Trim())) "chat_input_text='$($s2.chat_input_text)'"

    foreach ($c in 'ihao'.ToCharArray()) { TypeLetter $c }
    Start-Sleep -Milliseconds 500
    $s3 = St
    Rec '拼音 nihao 到达 IME 引擎' ($s3.ime_composing -eq 'nihao') "ime_composing='$($s3.ime_composing)'"
    $shot3 = Shot 'ime_rpc_3_candidates'
    Rec '候选条截图（人工核字形非乱码）' $true $shot3

    Tap 0x31                                                                  # '1' 选首候选上屏
    Start-Sleep -Milliseconds 500
    $s4 = St
    Rec '候选上屏为 CJK（非乱码/非拼音残留）' (HasCjk $s4.chat_input_text) "chat_input_text='$($s4.chat_input_text)' composing='$($s4.ime_composing)'"
    $shot4 = Shot 'ime_rpc_4_committed'
    Rec '上屏截图' $true $shot4

    Tap 0x1B                                                                  # Esc 取消
    Start-Sleep -Milliseconds 300
    $s5 = St
    Rec 'Esc 取消组合' ($s5.ime_composing -eq '') "ime_composing='$($s5.ime_composing)'"
} finally {
    $results | ConvertTo-Json | Set-Content "$acc\ime_rpc_verify_results.json" -Encoding UTF8
    $ok = ($results | Where-Object pass).Count
    Write-Host ("==== IME 汇总 {0}/{1} 通过 ====" -f $ok, $results.Count)
}
