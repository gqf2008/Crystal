# golden_kbd_windows.ps1 — 用**键盘**把原版客户端的每一扇「有键位」窗口逐扇打开并截图（逐窗像素 A/B 的原版侧）。
#
# 为什么走键盘：README §3.1 实测原版镜像 UI 的按钮**点不动**（WinForms 不为注入的 WM_LBUTTONDOWN/UP 触发
# MouseClick，真实 SetCursorPos+左键也被当成标题栏拖动）⇒ 需要「点页签/点行」才能到的窗不能靠驱动原版。
# 但 `KeyBinds.ini` 把所有主要窗口都绑了键，而键盘通道**实测可用**（§2.1）——于是逐窗 A/B 走这条路。
#
# 用法（前置：客户端已由 csharp_kbd_login.ps1 登录进游戏、仍在运行）：
#   pwsh -NoProfile -File .\golden_kbd_windows.ps1 -SandboxRoot <dir>
#
# 每扇窗前后都先发 `Escape`（C# `KeybindOptions.Closeall`）把状态归零，避免"上一扇窗还开着"污染这一张。

param(
    [string]$SandboxRoot = $env:CRYSTAL_CSHARP_SANDBOX,
    [string]$OutJson = ''
)
if (-not $SandboxRoot) { throw 'pass -SandboxRoot <dir>' }
. "$PSScriptRoot\csharp_client_driver.ps1" -SandboxRoot $SandboxRoot

if (-not $OutJson) { $OutJson = Join-Path $script:CS 'shots\kbd_windows.json' }

# KeyBinds.ini 的**段作用域**读取（`make_sandbox.ps1` 里那份是给 Setup/Mir2Config 用的，
# 这里自带一份，免得为了一个读取函数去 dot-source 一个会建沙箱的脚本）。
function Get-IniSectionValue {
    param([string]$Text, [string]$Section, [string]$Key)
    $s = [regex]::Escape($Section); $k = [regex]::Escape($Key)
    $m = [regex]::Match($Text, "(?ms)^\[$s\]\s*$.*?(?=^\[|\z)")
    if (-not $m.Success) { return $null }
    $km = [regex]::Match($m.Value, "(?m)^\s*$k\s*=\s*(.*?)\s*$")
    if ($km.Success) { return $km.Groups[1].Value.Trim() }
    return $null
}

# 键位表 = 从**沙箱那份 KeyBinds.ini** 读出来的实际绑定（[动作] 段里的 RequireKey）。
# 刻意**不写死**在脚本里：换一份 KeyBinds.ini（例如原版默认键位）时这张表跟着走。
# 只取「单击即开窗/切 HUD」的那些动作（带修饰键的语义动作如 Ctrl+H 改攻击模式不在本表）。
$keyVk = @{
    'F1' = 0x70; 'F2' = 0x71; 'F3' = 0x72; 'F4' = 0x73; 'F5' = 0x74; 'F6' = 0x75
    'F7' = 0x76; 'F8' = 0x77; 'F9' = 0x78; 'F10' = 0x79; 'F11' = 0x7A; 'F12' = 0x7B
    'Escape' = 0x1B
}
foreach ($c in [char[]]'ABCDEFGHIJKLMNOPQRSTUVWXYZ') { $keyVk["$c"] = [byte][char]$c }

# 动作 → 我方 DialogKind（逐窗 A/B 的配对键；空串表示 HUD/无对应窗，只留档不比对）
$kindOf = @{
    'Inventory'     = 'inventory'
    'Equipment'     = 'character'
    'Skills'        = 'character_skill_page'
    'Quests'        = 'quest_log'
    'Options'       = 'settings'
    'Group'         = 'group'
    'Friends'       = 'friend'
    'Relationship'  = 'relationship'
    'Guilds'        = 'guild'
    'Ranking'       = 'ranking'
    'Help'          = 'help'
    'Keybind'       = 'keyboard_layout'
    'Creature'      = 'creature'
    'MountWindow'   = 'mount'
    'Fishing'       = 'fishing'
    'GameShop'      = 'game_shop'
    'Bigmap'        = 'big_map'
    'Minimap'       = 'minimap'
    'Belt'          = 'hud_belt'
    'Skillbar'      = 'hud_skillbar'
}

# 只做「按一下键就能看到」的动作（RequireCtrl=0 / RequireAlt=0 的那些，见 KeyBinds.ini）
$actions = @(
    'Inventory', 'Equipment', 'Skills', 'Quests', 'Options', 'Group', 'Friends', 'Relationship',
    'Guilds', 'Ranking', 'Help', 'Keybind', 'Creature', 'MountWindow', 'Fishing', 'GameShop',
    'Bigmap', 'Minimap', 'Belt', 'Skillbar'
)

Init-CsClient

# **必须走窗口消息**，不能走 `Key-Cs`（它用全局 `keybd_event`）：那条路要求客户端是**前台窗口**，
# 而原版客户端是 owner/别的 agent 也在用的共享窗口，抢前台既不可靠也会打断别人。实测对照：
# 登录脚本用 `SendMessage(WM_KEYDOWN/UP)` 直达窗口 → F9/F10/F11 生效；本脚本第一版用 `Key-Cs` →
# 20 张全一样（窗没开）。判据也据此改：每扇窗都要与「基线帧」比出**可见差异**，否则记为 no_effect。
function Send-KeyMsg([byte]$vk, [int]$holdMs = 120) {
    [void][CsUi]::SendMessage($global:csHwnd, 0x0100, [IntPtr]$vk, [IntPtr]::Zero)  # WM_KEYDOWN
    Start-Sleep -Milliseconds $holdMs
    # **必须补 WM_CHAR**：C# 的 `MirMessageBox.OnKeyPress`（Escape 关模态框）与一堆键位语义
    # 都挂在 KeyPress 上，只发 DOWN/UP 的话模态框关不掉——它会一直吞掉后面的键，
    # 实测表现是「基线帧里那个 'You are not in a guild.' 一直不退，后续 20 张全等于基线」。
    # 登录脚本里那条能用的 `Send-Key` 正是 DOWN + **CHAR** + UP 三连，这里与它对齐。
    [void][CsUi]::SendMessage($global:csHwnd, 0x0102, [IntPtr]$vk, [IntPtr]::Zero)  # WM_CHAR
    Start-Sleep -Milliseconds $holdMs
    [void][CsUi]::SendMessage($global:csHwnd, 0x0101, [IntPtr]$vk, [IntPtr]0xC0000001)  # WM_KEYUP
}

# 采样亮度差（够用且快）：用来判「这一键到底有没有让画面变」
function Frame-Diff([string]$a, [string]$b, [int]$step = 6) {
    if (-not (Test-Path -LiteralPath $a) -or -not (Test-Path -LiteralPath $b)) { return $null }
    $ia = [System.Drawing.Bitmap]::FromFile($a); $ib = [System.Drawing.Bitmap]::FromFile($b)
    try {
        $sum = 0.0; $n = 0
        for ($y = 0; $y -lt $ia.Height; $y += $step) {
            for ($x = 0; $x -lt $ia.Width; $x += $step) {
                $ca = $ia.GetPixel($x, $y); $cb = $ib.GetPixel($x, $y)
                $la = 0.299 * $ca.R + 0.587 * $ca.G + 0.114 * $ca.B
                $lb = 0.299 * $cb.R + 0.587 * $cb.G + 0.114 * $cb.B
                $sum += [Math]::Abs($la - $lb); $n++
            }
        }
        return [Math]::Round($sum / [Math]::Max($n, 1), 2)
    } finally { $ia.Dispose(); $ib.Dispose() }
}

# **干净基线**：先 Escape（C# `KeybindOptions.Closeall`）把所有窗关掉再拍。
# 不能借用 `kbd_login` 那张 `orig_kbd_02_ingame.png`——那张是在 F9/F10 之后拍的，
# 里面已经开着背包+角色窗，用它当基线会把"某扇窗没出"判成"出窗了"（逐窗判据会整排失真）。
$baseline = Join-Path $script:CS 'shots\orig_baseline_none.png'
# 多按两次 Escape：第一次关模态框（WM_CHAR），第二次关其它窗
Send-KeyMsg 0x1B; Start-Sleep -Milliseconds 500
Send-KeyMsg 0x1B; Start-Sleep -Milliseconds 700
if (Test-Path -LiteralPath $baseline) { Remove-Item -LiteralPath $baseline -Force }
Shot-Cs 'baseline_none'

$rows = @()
foreach ($a in $actions) {
    $key = Get-IniSectionValue -Text ([IO.File]::ReadAllText((Join-Path $script:CS 'Client\KeyBinds.ini'))) -Section $a -Key 'RequireKey'
    if (-not $key -or $key -eq 'None' -or -not $keyVk.ContainsKey($key)) {
        Write-Host ("跳过 {0}：KeyBinds.ini 里 RequireKey={1}（无键或不认识）" -f $a, $key)
        continue
    }
    # 归零：先 Escape（关模态框 + Closeall），再按这一扇窗的键
    Send-KeyMsg 0x1B; Start-Sleep -Milliseconds 400
    Send-KeyMsg 0x1B; Start-Sleep -Milliseconds 600
    Send-KeyMsg ([byte]$keyVk[$key]); Start-Sleep -Milliseconds 900
    $label = "win_$a"
    Shot-Cs $label
    $png = Join-Path $script:CS "shots\orig_$label.png"
    $diff = Frame-Diff $baseline $png
    $rows += [pscustomobject]@{
        action = $a; key = $key; kind = $kindOf[$a]; png = $png
        exists = (Test-Path -LiteralPath $png); diff_vs_baseline = $diff
        no_effect = ($null -ne $diff -and $diff -lt 0.5)
    }
    Write-Host ("{0,-14} 键={1,-4} kind={2,-12} 与基线差={3} {4}" -f `
        $a, $key, $kindOf[$a], $diff, $(if ($null -ne $diff -and $diff -lt 0.5) { '<-- 画面没变（键没生效？）' } else { '' }))
}
$json = $rows | ConvertTo-Json -Depth 4
[IO.File]::WriteAllText($OutJson, $json)
$bad = @($rows | Where-Object { $_.no_effect }).Count
Write-Host ("逐窗截图写完：{0}（{1} 扇，其中画面没变的 {2} 扇）" -f $OutJson, $rows.Count, $bad)
