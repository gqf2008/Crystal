# l5t_chat_dialog4.ps1 — 底部对话框（聊天窗）四缺陷的**实机**判据
#
# 为什么需要它：四缺陷（①滚动 ②透明 ③对齐 ④尺寸还原）此前只有**离线单测**
# （几何常量 + 滚动反查互逆函数）。离线测试证明不了"鼠标点得动、像素上真不透明"——
# `chat_scroll_drag_system` 的**命中路径**在单测里根本不执行。本夹具补这条端到端证据。
#
# 判据（全部取客户端状态真值 `chat_probe` + 像素对照，不看"看起来像"）：
#   A) 进场：`state` 可读
#   B) 前置：灌够聊天行使 `max_scroll > 0`（否则滚动类断言全是空转 → exit 3）
#   C) ① 滚轮：0 档面板中心 `wheel +3` → `scroll_up` 增加；`wheel -999` → 回 0
#   D) ① 拖动/点轨道：在轨道上（面板顶 +16 +40）`click` → `scroll_up` 变化
#   E) ③ 对齐（逐档）：对 0/1/2 三档分别用**该档**的轨道坐标点击 → 每档都命中
#      （旧 bug：命中区按 0 档硬编码 → 展开档点不动）
#   F) ① 展开档滚轮：size=2 时在**展开后的面板区域**（0 档矩形之外）滚轮仍生效（同一旧 bug 的另一半）
#   G) ④ 升档/降档几何还原：0→1→2→1→0，每步 `panel_top` 等于原版底边锚定值，回到 0 档必须还原
#   H) ② 不透明（像素）：面板内部区域在角色走动两帧间**几乎不变**（地图透不过来），
#      同一时刻**面板外**的对照区域必须明显变化——后者是本判据的阳性对照（证明度量灵敏）；
#      对照区没变化就说明"人没走动/地图没变"，本项记 N/A 并 exit 3，不虚报 PASS。
#
# 退出码：0 = 全部 PASS；1 = 有用例 FAIL；3 = 前置不成立（不产出 PASS）；9 = 未进场/探针不可读
#
# 与其它 check 并存的注意事项（多 agent 同机）：本夹具**不按进程名杀进程**，
# 而是复制成独立名字 `client_bevy_l5t.exe` + 独立控制端口（默认 9099），
# 因此不会打掉别人正在跑的客户端实验。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [int]$Port = 9099,
    [int]$Lines = 40
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$srcExe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy_l5t.exe"
$err = "$PSScriptRoot\l5t_client.err.log"
$shotA = "$PSScriptRoot\l5t_shot_a.png"
$shotB = "$PSScriptRoot\l5t_shot_b.png"

$script:Fail = 0
function Check([string]$name, [bool]$ok, [string]$detail = '') {
    if ($ok) { Write-Host ("  [PASS] {0} {1}" -f $name, $detail) }
    else { Write-Host ("  [FAIL] {0} {1}" -f $name, $detail); $script:Fail++ }
}

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', $Port)
        $s = $c.GetStream(); $s.ReadTimeout = 5000; $s.WriteTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}

# 面板内区域的平均绝对亮度差（每 3 像素抽样，够用且快）
function RegionDiff([string]$f1, [string]$f2, [int]$x, [int]$y, [int]$w, [int]$h) {
    Add-Type -AssemblyName System.Drawing
    $a = [System.Drawing.Bitmap]::FromFile($f1); $b = [System.Drawing.Bitmap]::FromFile($f2)
    try {
        if ($x + $w -gt $a.Width -or $y + $h -gt $a.Height) { return $null }
        $sum = 0.0; $n = 0
        for ($py = $y; $py -lt $y + $h; $py += 3) {
            for ($px = $x; $px -lt $x + $w; $px += 3) {
                $ca = $a.GetPixel($px, $py); $cb = $b.GetPixel($px, $py)
                $la = 0.299 * $ca.R + 0.587 * $ca.G + 0.114 * $ca.B
                $lb = 0.299 * $cb.R + 0.587 * $cb.G + 0.114 * $cb.B
                $sum += [math]::Abs($la - $lb); $n++
            }
        }
        if ($n -eq 0) { return $null }
        return $sum / $n
    } finally { $a.Dispose(); $b.Dispose() }
}

Get-CimInstance Win32_Process -Filter "Name='client_bevy_l5t.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 1200
# 复制可能因上一次实验的进程还没完全退出而失败（文件仍被占用）→ 重试几次，
# 失败就明确报错退出，别拿旧二进制跑（旧二进制可能没有本次的探针字段 → 判据失真）。
$copied = $false
foreach ($i in 1..5) {
    try { Copy-Item $srcExe $exe -Force -EA Stop; $copied = $true; break }
    catch { Start-Sleep -Milliseconds 1500 }
}
if (-not $copied) { Write-Host 'FAIL(9): 无法复制客户端二进制（可能仍被占用），本轮不跑'; exit 9 }
if (Test-Path $err) { Remove-Item $err -Force }
$proc = Start-Process -FilePath $exe `
    -ArgumentList '--real-net','--auto-enter','--control-port',"$Port",'--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$PSScriptRoot\l5t_client.log" -RedirectStandardError $err -PassThru

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场/控制端口不通'; exit 9 }
Write-Host ("[A] 进场 map={0} tile=({1},{2}) PASS" -f $st.map, $st.tile_x, $st.tile_y)

# B) 前置：灌够行
foreach ($i in 1..$Lines) { Rpc 'chat' @{ message = "l5t-line-$i" } | Out-Null }
$p = $null
foreach ($i in 1..20) {
    Start-Sleep -Milliseconds 500
    $p = Rpc 'chat_probe' @{ limit = 5 }
    if ($null -ne $p.max_scroll -and [int]$p.max_scroll -gt 0) { break }
}
if ($null -eq $p -or $null -eq $p.max_scroll) { Write-Host 'chat_probe 不可读'; exit 9 }
Write-Host ("[B] total_lines={0} visible={1} max_scroll={2} size={3}" -f $p.total_lines, $p.visible_lines, $p.max_scroll, $p.size)
if ([int]$p.max_scroll -le 0) {
    Write-Host 'FAIL(3): 聊天行不足（max_scroll=0）——滚动类断言会空转，本轮不产出 PASS'
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    exit 3
}

# C) ① 滚轮
Rpc 'chat_size' @{ size = 0 } | Out-Null; Start-Sleep -Milliseconds 400
$centerX = [int]$p.panel.x + [int]($p.panel.w / 2)
$centerY = [int]$p.panel.y + [int]($p.panel.h / 2)
Rpc 'wheel' @{ x = $centerX; y = $centerY; delta = 3 } | Out-Null
Start-Sleep -Milliseconds 600
$p2 = Rpc 'chat_probe' @{ limit = 1 }
Check '①滚轮上滚' ([int]$p2.scroll_up -ge 1) ("scroll_up {0} -> {1}" -f $p.scroll_up, $p2.scroll_up)
Rpc 'wheel' @{ x = $centerX; y = $centerY; delta = -999 } | Out-Null
Start-Sleep -Milliseconds 600
$p3 = Rpc 'chat_probe' @{ limit = 1 }
Check '①滚轮回底' ([int]$p3.scroll_up -eq 0) ("scroll_up={0}" -f $p3.scroll_up)

# D) ① 拖轨道（命中区在 面板x+622、顶+16）。用 `click {drag_to}` 真拖动而不是单击：
#    单击的按下/抬起可能落在同一帧，而拖动系统的语义是「按下后每帧按 y 反查 scroll_up」；
#    首跑单点实测 3 档里只有 1 档命中（探针撤销时序），拖动能稳定覆盖这条路径。
function TrackPoint($probe, [int]$dy) {
    return @{ x = [int]$probe.panel.x + 622; y = [int]$probe.panel.y + 16 + $dy }
}
# 轨道自身的像素高度没有对外暴露（`ChatScrollTrack` 是私有组件），所以按下点在一个小网格上
# 试几次：命中判定要求「按下点落在 轨道矩形 ±2px」内，网格覆盖常见的几档轨道高度。
function DragTrack($probe, [int]$dy) {
    $down = TrackPoint $probe $dy
    $up = TrackPoint $probe ([math]::Max(2, $dy - 18))
    Rpc 'click' @{ x = $down.x; y = $down.y; drag_to = @{ x = $up.x; y = $up.y } } | Out-Null
    Start-Sleep -Milliseconds 700
    return (Rpc 'chat_probe' @{ limit = 1 }).scroll_up
}
function TryDrag([int]$base) {
    foreach ($dy in 8, 20, 32, 44) {
        Rpc 'wheel' @{ x = $centerX; y = $base + 30; delta = -999 } | Out-Null
        Start-Sleep -Milliseconds 350
        $v = DragTrack $p3 $dy
        if ([int]$v -gt 0) { return @{ ok = $true; dy = $dy; value = [int]$v } }
    }
    return @{ ok = $false; dy = -1; value = 0 }
}
$drag = TryDrag ([int]$p3.panel.y)
Check '①拖滚动条改变 scroll_up' $drag.ok ("命中按下点 dy={0} -> scroll_up={1}" -f $drag.dy, $drag.value)

# E) ③ 逐档命中：每档用该档面板顶边推出的轨道坐标拖动，都应命中
foreach ($tier in 0, 1, 2) {
    Rpc 'chat_size' @{ size = $tier } | Out-Null
    Start-Sleep -Milliseconds 500
    Rpc 'wheel' @{ x = $centerX; y = [int]$p.panel.y + 30; delta = -999 } | Out-Null
    Start-Sleep -Milliseconds 400
    $pt = Rpc 'chat_probe' @{ limit = 1 }
    $res = $null
    foreach ($dy in 8, 20, 32, 44) {
        Rpc 'wheel' @{ x = $centerX; y = [int]$pt.panel.y + 30; delta = -999 } | Out-Null
        Start-Sleep -Milliseconds 350
        $v = DragTrack $pt $dy
        if ([int]$v -gt 0) { $res = @{ ok = $true; dy = $dy; value = [int]$v }; break }
    }
    if ($null -eq $res) { $res = @{ ok = $false; dy = -1; value = 0 } }
    Check ("③{0}档轨道命中" -f $tier) $res.ok `
        ("panel_top={0} 命中按下点 dy={1} -> scroll_up={2}" -f $pt.panel_top, $res.dy, $res.value)
}

# F) ① 展开档滚轮（在 0 档矩形之外的展开区域）
Rpc 'chat_size' @{ size = 2 } | Out-Null
Start-Sleep -Milliseconds 500
Rpc 'wheel' @{ x = $centerX; y = [int]$p.panel.y + 20; delta = -999 } | Out-Null
Start-Sleep -Milliseconds 400
$p5 = Rpc 'chat_probe' @{ limit = 1 }
$expandedY = [int]$p5.panel_top + 20   # 展开档顶部附近：小于 0 档顶边 671
Rpc 'wheel' @{ x = $centerX; y = $expandedY; delta = 2 } | Out-Null
Start-Sleep -Milliseconds 600
$p6 = Rpc 'chat_probe' @{ limit = 1 }
Check '①展开档滚轮生效' ([int]$p6.scroll_up -gt [int]$p5.scroll_up) `
    ("y={0}（0档顶边=671）scroll_up {1} -> {2}" -f $expandedY, $p5.scroll_up, $p6.scroll_up)

# G) ④ 升档/降档几何还原（原版底边锚定）
$expect = @{ 0 = 671.0; 1 = 623.0; 2 = 575.0 }
foreach ($tier in 0, 1, 2, 1, 0) {
    Rpc 'chat_size' @{ size = $tier } | Out-Null
    Start-Sleep -Milliseconds 400
    $pg = Rpc 'chat_probe' @{ limit = 1 }
    Check ("④档位{0} panel_top" -f $tier) ([math]::Abs([double]$pg.panel_top - $expect[$tier]) -lt 0.5) `
        ("panel_top={0} 期望={1}" -f $pg.panel_top, $expect[$tier])
}

# H) ② 不透明：走动前后，面板内几乎不变 + 面板外必须明显变化（阳性对照）
Rpc 'screenshot' @{ path = $shotA } | Out-Null
Start-Sleep -Milliseconds 800
$before = Rpc 'state'
# 让屏幕内容变化：走到**另一张图**（同图内小位移在镜头被地图边界钳住时像素几乎不变——
# 实测 +5 格只换来 0.64 的平均差，达不到判据所需的"对照区明显变化"）。
# 换图必然整屏重绘；聊天面板是 UI，不受换图影响，正好用来判"面板是不是不透明"。
# 注意：`@mapmove` 是 GM 命令——普通账号（bevy2）用不了，而本机 `test` 账号当前登录失败
# （实测 `⛔ 登录失败 result=4 密码错误`）。所以这里用**走路**让镜头位移，四个方向逐个试
# （角色可能被地形挡住），一个都不动就如实判前置不成立 exit 3。
$dirs = @(@{ dx = 3; dy = 0 }, @{ dx = -3; dy = 0 }, @{ dx = 0; dy = 3 }, @{ dx = 0; dy = -3 })
foreach ($d in $dirs) {
    Rpc 'walk_to' @{ tx = [int]$before.tile_x + $d.dx; ty = [int]$before.tile_y + $d.dy; run = $false } | Out-Null
    Start-Sleep -Milliseconds 1500
    $mid = Rpc 'state'
    if ($mid.tile_x -ne $before.tile_x -or $mid.tile_y -ne $before.tile_y) {
        Write-Host ("[H] 走动生效: ({0},{1}) -> ({2},{3})" -f $before.tile_x, $before.tile_y, $mid.tile_x, $mid.tile_y)
        break
    }
}
Rpc 'screenshot' @{ path = $shotB } | Out-Null
Start-Sleep -Milliseconds 800
$after = Rpc 'state'
$moved = ($before.tile_x -ne $after.tile_x) -or ($before.tile_y -ne $after.tile_y)
Write-Host ("[H] 走动前后 tile=({0},{1}) -> ({2},{3})" -f $before.tile_x, $before.tile_y, $after.tile_x, $after.tile_y)
if (-not (Test-Path $shotA) -or -not (Test-Path $shotB)) {
    Write-Host 'FAIL(3): 截图未落盘——②的像素判据无法评估'; Stop-Process -Id $proc.Id -Force -EA SilentlyContinue; exit 3
}
$ph = Rpc 'chat_probe' @{ limit = 1 }
# 截图是**物理像素**，面板矩形是**逻辑坐标** → 必须按 scale 换算，否则采样区根本不是面板
# （首跑就是踩这个：物理 1536×1152、逻辑 1024×768、scale=1.5，采样却按逻辑值取，采到了地图）。
$scale = if ($null -ne $ph.window -and $ph.window.scale) { [double]$ph.window.scale } else { 1.0 }
$insideX = [int](([double]$ph.panel.x + 4) * $scale); $insideW = [int](([double]$ph.panel.w - 8) * $scale)
$insideY = [int](([double]$ph.panel.y + 4) * $scale); $insideH = [int](([double]$ph.panel.h - 8) * $scale)
$dIn = RegionDiff $shotA $shotB $insideX $insideY $insideW $insideH
# 对照区：面板正上方 120px 处的地图（同宽），走动时地图必然位移
$ctlY = [int](([double]$ph.panel.y - 120) * $scale)
$dCtl = RegionDiff $shotA $shotB $insideX $ctlY $insideW ([int](80 * $scale))
Write-Host ("[H] scale={0} 面板内平均差={1:N2}；面板外对照区平均差={2:N2}（逻辑→物理已换算）" -f $scale, $dIn, $dCtl)
if ($null -eq $dIn -or $null -eq $dCtl -or -not $moved -or $dCtl -lt 3.0) {
    Write-Host 'FAIL(3): 对照区没有变化（人没走动/地图没变）——②的像素判据不成立，本轮不产出 PASS'
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    exit 3
}
Check '②面板不透明（内部像素稳定）' ($dIn -le 2.0) ("面板内平均差={0:N2}（阈值<=2.0，对照区={1:N2}）" -f $dIn, $dCtl)

$alive = -not $proc.HasExited
Check '客户端存活' $alive ("pid={0}" -f $proc.Id)
Stop-Process -Id $proc.Id -Force -EA SilentlyContinue

if ($script:Fail -eq 0) { Write-Host '=== 全部 PASS ==='; exit 0 }
Write-Host ("=== 有 FAIL：{0} 项 ===" -f $script:Fail)
exit 1
