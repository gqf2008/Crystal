# l5o_settings_options.ps1 — 设置窗（C# OptionDialog）两处实机验收
# （owner 队列 `settings-english-labels`）
#
# 背景：该队列条目说「设置窗首个分页显示 SKILL MODE / EFFECTS … 英文标签，C# 查不到这些字面量」。
# 核对结论（`Client/MirScenes/Dialogs/MainDialogs.cs:2527` `OptionDialog`）：
#   - C# 这些行**没有文字控件**：行标签烘在原版美术帧里（`Prguse2[450..467]` / `Title[848..853]`），
#     `Client/Localization/*.json` 里也没有这些字面量 —— 英文来自原版美术，属「原版如此」；
#   - 但顺着这条线索查出**两处真缺陷**：① 同排两颗按钮被套用同一套帧（实机 SKILL BAR 排显示成
#     `[on][on]`，C# 是各自换帧）；② `SkillMode` 那颗的开关方向写反（点左侧「Ctrl」钮把
#     skill_mode_ctrl 置成了 false）；③ C# 有三档开关会往聊天区发本地化提示，本端没发。
#
# 判据（取状态/聊天区真值，不解析像素）：
#   A) 点 SKILL MODE 左钮（C# `SkillModeOn` @159,68）→ 聊天区出现「[技能模式：Ctrl]」
#   B) 点 SKILL MODE 右钮（C# `SkillModeOff` @201,68）→ 聊天区出现「[技能模式：~]」
#      （A/B 同时证明换帧与方向：左侧=Ctrl、右侧=~）
#   C) 负对照：点 EFFECTS 左钮（原版不发提示）→ 聊天区**不新增**任何 "[…]" 提示行
#   D) 截图留档（可目检同排两颗按钮的标签不再重复）
#
# 退出码：0 = 全 PASS；10 = 判据未达成；9 = 服务端/客户端未就绪
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    # 面板原点 = C# `(1024-259)/2, (768-354)/2` = (382,207)；按钮 36x17 @ x=159/201
    [int]$PanelX = 382,
    [int]$PanelY = 207,
    [int]$SkillModeY = 68,
    [int]$EffectsY = 118,
    [switch]$NoRestart
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5o_settings_options' -TimeoutSec 1800)) { exit 2 }
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$acc = "$PSScriptRoot"
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { Write-Host ("  [Rpc $m 失败] " + $_.Exception.Message); return $null }
}
function ChatLines([int]$n = 8) { @((Rpc 'chat_probe' @{ limit = $n }).lines) }
function HintTexts([int]$n = 8) { @(ChatLines $n | Where-Object { $_.text -match '^\[.*\]$' } | ForEach-Object { $_.text }) }
function InY([double]$y, [int]$rowY) { [int]($PanelY + $rowY + 17 / 2) }
function LeftX { [int]($PanelX + 159 + 36 / 2) }
function RightX { [int]($PanelX + 201 + 36 / 2) }

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
if (-not $NoRestart) {
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5o_client.log" -RedirectStandardError "$acc\l5o_client.err.log" | Out-Null
}
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

Rpc 'dialog' @{ kind = 'settings'; action = 'open' } | Out-Null
Start-Sleep 1
$verdict = $true

# A) 左侧「Ctrl」钮
$before = HintTexts
Rpc 'click' @{ x = LeftX; y = InY 0 $SkillModeY } | Out-Null
Start-Sleep -Milliseconds 800
$after = HintTexts
$okA = ($after -contains '[技能模式：Ctrl]')
Write-Host ("[A] 点 SKILL MODE 左钮({0},{1}) → 聊天提示 {2} → {3}" -f (LeftX), (InY 0 $SkillModeY), `
    (($after | Select-Object -Last 2) -join ' | '), $(if ($okA) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okA

# B) 右侧「~」钮
Rpc 'click' @{ x = RightX; y = InY 0 $SkillModeY } | Out-Null
Start-Sleep -Milliseconds 800
$afterB = HintTexts
$okB = ($afterB -contains '[技能模式：~]')
Write-Host ("[B] 点 SKILL MODE 右钮({0},{1}) → 聊天提示 {2} → {3}" -f (RightX), (InY 0 $SkillModeY), `
    (($afterB | Select-Object -Last 2) -join ' | '), $(if ($okB) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okB

# C) 负对照：EFFECTS（原版不发提示）
$nBefore = (HintTexts).Count
Rpc 'click' @{ x = LeftX; y = InY 0 $EffectsY } | Out-Null
Start-Sleep -Milliseconds 800
$nAfter = (HintTexts).Count
$okC = ($nAfter -eq $nBefore)
Write-Host ("[C] 点 EFFECTS 左钮后提示行数 {0} → {1}（原版不发提示）→ {2}" -f `
    $nBefore, $nAfter, $(if ($okC) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okC

Rpc 'screenshot' @{ path = '../tools/acceptance/l5o_settings.png' } | Out-Null
Write-Host '截图：tools/acceptance/l5o_settings.png（目检同排两颗按钮标签是否仍重复）'
Write-Host ("VERDICT: {0}（三档提示 + 方向 + 负对照）" -f $(if ($verdict) { 'PASS' } else { 'FAIL' }))
if (-not $verdict) { exit 10 }
exit 0
