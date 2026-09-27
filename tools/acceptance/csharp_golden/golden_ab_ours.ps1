# golden_ab_ours.ps1 — 逐窗像素 A/B 的我方一侧：按 golden_kbd_windows.ps1 产出的窗口清单，
# 在自己（Rust/Bevy）客户端上开**同样的窗**并截图，随后与沙箱那版原版帧逐窗比。
#
# 用法：pwsh -NoProfile -File .\golden_ab_ours.ps1 -SandboxRoot <dir> [-ClientHome <检出>] [-Port 9105]
param(
    [string]$SandboxRoot = $env:CRYSTAL_CSHARP_SANDBOX,
    [string]$ClientHome = '',
    [int]$Port = 9105,
    [int]$MapX = 278,
    [int]$MapY = 609,
    # 2026-09-27 加：账号/密码可传。首次完整跑通时发现**逐窗差异的主项是"角色状态"**（原版是全新
    # 1 级女道士：空背包/无宠物/无行会/无坐骑/无技能；我方当时用的高等级角色 51/51 格满、5 只宠物、
    # 已在测试行会）——原版侧那几扇窗直接弹 MirMessageBox（"You do not own any creatures."），
    # 于是 diff 98% 全是状态差。要出**可判的绘制差异**必须先把状态对齐：用这个参数指向一个
    # 与金标准同职业/同等级/同空背包的测试角色再跑同一条链。
    [string]$User = 'test',
    [string]$Password = '123456',
    # 原版帧恒 1024x768@scale1；我方默认跟系统 DPI（本机 1.5）⇒ 传 1 让两边同尺度，
    # 否则逐窗 diff 的主项是重采样噪声（见 `--ui-scale` 的注释）。
    [string]$UiScale = '1'
)
if (-not $SandboxRoot) { throw 'pass -SandboxRoot <dir>' }
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\..\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'golden_ab_ours'

$manifest = Get-Content (Join-Path $SandboxRoot 'shots\kbd_windows.json') -Raw | ConvertFrom-Json
$shots = Join-Path $SandboxRoot 'shots'

. "$PSScriptRoot\..\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'golden_ab_ours' -TimeoutSec 1800)) { exit 2 }
try {
$exe = Join-Path (Split-Path -Parent $exeSrc) 'client_bevy_ab.exe'
Get-CimInstance Win32_Process -Filter "Name='client_bevy_ab.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -EA Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
$log = Join-Path $env:TEMP 'golden_ab_ours.err.log'
$scaleArgs = if ($UiScale) { @('--ui-scale', $UiScale) } else { @() }
Start-Process -FilePath $exe `
    -ArgumentList (@('--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Password,'--control-port',"$Port") + $scaleArgs) `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput (Join-Path $env:TEMP 'golden_ab_ours.out.log') -RedirectStandardError $log | Out-Null

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', $Port)
        $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}

$st = $null
foreach ($i in 1..90) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st.tile_x) { Write-Host 'FAIL(2): 我方客户端未进场'; exit 2 }
Write-Host ("我方进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)
# 对齐到原版帧所在的地图/格（BichonProvince 278,609），并等落位。
# 2026-09-27 改：金标准 A/B 现在用**与金标准同状态的角色**（1 级女道士，非 GM）⇒ `@mapmove`
# 会被服务端按权限拒绝（实测 tile 停在出生点 (288,616) 不动，脚本却照旧打印"对齐后"）。
# 所以：先试 @mapmove，落点不对就退回 `walk_to`（玩家验收能力，走真实寻路，不需要 GM 权限），
# 并把**最终落点**打出来（对不上就是对齐失败，别让后续截图在错误位置比）。
Rpc 'chat' @{ message = "@mapmove 0 $MapX $MapY" } | Out-Null
Start-Sleep -Seconds 2
$st2 = Rpc 'state'
$near = ($null -ne $st2.tile_x) -and ([Math]::Abs($st2.tile_x - $MapX) -le 1) -and ([Math]::Abs($st2.tile_y - $MapY) -le 1)
if (-not $near) {
    Write-Host ("@mapmove 未生效（tile=({0},{1})，目标 ({2},{3})）→ 改用 walk_to 走过去" -f $st2.tile_x, $st2.tile_y, $MapX, $MapY)
    Rpc 'walk_to' @{ tx = $MapX; ty = $MapY; run = $true } | Out-Null
    foreach ($i in 1..30) {
        Start-Sleep 1
        $st2 = Rpc 'state'
        if (($null -ne $st2.tile_x) -and ([Math]::Abs($st2.tile_x - $MapX) -le 1) -and ([Math]::Abs($st2.tile_y - $MapY) -le 1)) { break }
    }
}
$aligned = ($null -ne $st2.tile_x) -and ([Math]::Abs($st2.tile_x - $MapX) -le 1) -and ([Math]::Abs($st2.tile_y - $MapY) -le 1)
Write-Host ("对齐后 map={0} tile=({1},{2}) aligned={3}" -f $st2.map, $st2.tile_x, $st2.tile_y, $aligned)

# 帧差（判「这一张到底有没有开出窗」）——`screenshot` 是**下一帧**才落盘（bevy `Screenshot::primary_window()`
# + `save_to_disk`），实测 0.9s 不够：第一版 20 张全是"没窗"的样子。所以每扇窗都做**重试直到画面变化**。
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
Add-Type -AssemblyName System.Drawing

# 基线帧（无任何窗口）
$basePng = Join-Path $shots 'ours_kbd_02_ingame.png'
if (Test-Path -LiteralPath $basePng) { Remove-Item -LiteralPath $basePng -Force }
foreach ($k in @('inventory', 'character', 'quest_log', 'settings', 'group', 'friend', 'relationship',
                 'guild', 'ranking', 'help', 'keyboard_layout', 'creature', 'mount', 'fishing',
                 'game_shop', 'big_map')) {
    Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null
}
Start-Sleep -Milliseconds 1200
Rpc 'screenshot' @{ path = $basePng } | Out-Null
Start-Sleep -Seconds 2

$rows = @()
foreach ($r in $manifest) {
    $kind = $r.kind
    if (-not $kind) { continue }
    # 归零：先关掉上一扇窗与两扇常开窗，再开目标窗（对齐原版「Escape 后按这一扇窗的键」）
    foreach ($k in @('inventory', 'character', 'quest_log', 'settings', 'group', 'friend', 'relationship',
                     'guild', 'ranking', 'help', 'keyboard_layout', 'creature', 'mount', 'fishing',
                     'game_shop', 'big_map', $kind)) {
        Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null
    }
    Start-Sleep -Milliseconds 700
    $open = Rpc 'dialog' @{ kind = $kind; action = 'open' }
    $png = Join-Path $shots ("ours_win_{0}.png" -f $r.action)
    if (Test-Path -LiteralPath $png) { Remove-Item -LiteralPath $png -Force }
    # 重试：直到这一张与基线**明显不同**（= 窗真的开出来了），最多 8 轮
    $diff = $null
    foreach ($try in 1..8) {
        $open = Rpc 'dialog' @{ kind = $kind; action = 'open' }   # 幂等：open 已开时保持开
        Start-Sleep -Milliseconds 700
        Rpc 'screenshot' @{ path = $png } | Out-Null
        Start-Sleep -Milliseconds 900
        $diff = Frame-Diff $basePng $png
        if ($null -ne $diff -and $diff -ge 0.5) { break }
    }
    $rows += [pscustomobject]@{
        action = $r.action; kind = $kind; key = $r.key
        orig = $r.png; ours = $png
        open_ok = [bool]$open.ok
        exists = (Test-Path -LiteralPath $png)
        diff_vs_baseline = $diff
        no_effect = ($null -eq $diff -or $diff -lt 0.5)
    }
    Write-Host ("{0,-14} kind={1,-12} open_ok={2} 与基线差={3} {4}" -f `
        $r.action, $kind, $open.ok, $diff, $(if ($null -eq $diff -or $diff -lt 0.5) { '<-- 窗口没开出来？' } else { '' }))
}
$out = Join-Path $shots 'ab_windows.json'
[IO.File]::WriteAllText($out, ($rows | ConvertTo-Json -Depth 4))
Write-Host ("我方逐窗截图写完：{0}（{1} 扇）" -f $out, $rows.Count)
Get-CimInstance Win32_Process -Filter "Name='client_bevy_ab.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
} finally {
    Exit-E2eLock
}
