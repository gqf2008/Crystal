# l5q_shop_viewer.ps1 — 商城「试穿预览」实机验收（C# `GameShopViewer`，`Client/MirControls/MirGameShopCell.cs:312-595`）
#
# 判据（全部取 `shop_probe.viewer` 状态真值，不解析像素）：
#   A) 未点试穿钮时 `viewer == null`
#   B) 在可试穿格（ItemType ∈ {Weapon, Armour, Mount, Transform}）点「试穿」钮后
#      `viewer != null`、`viewer.item_index` == 该格商品 idx、`viewer.direction == 6`（C# 初值）、
#      且 `viewer.pos` == 面板该侧的 (416,115)/(151,115)（`cell_x < 350` 判据）
#   C) 左转 → direction 6→5；连点右转两次 → 6→7（1..8 环绕）
#   D) 点关闭钮 → `viewer == null`
#   并校验点击点确实落在对应控件矩形里（`ui_nodes_at`）
#
# 坐标（C# 常量 → 面板原点 (164,146)）：
#   格子 `cell_pos(i)`：i<4 → (152+132i,115)，否则 (152+132(i-4),275)
#   格子内「试穿」钮 `Title[781..783]` @(8,122) 42x22 → 中心 +（29,133)
#   预览面板 `Title[785]` @(416,115) 或 (151,115)；关闭 `Prguse[361..363]` @(230,8) 24x21
#   左转 `Prguse2[240..242]` @(81,282) 16x14；右转 `[243..245]` @(160,282) 16x14
#
# 退出码：0 = 全 PASS；10 = 判据未达成；9 = 服务端/客户端未就绪
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [int]$PanelX = 164,
    [int]$PanelY = 146,
    [switch]$NoRestart
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5q_shop_viewer' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
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
function Cell([int]$i) {
    if ($i -lt 4) { @(($PanelX + 152 + 132 * $i), ($PanelY + 115)) }
    else { @(($PanelX + 152 + 132 * ($i - 4)), ($PanelY + 275)) }
}
function Probe() { Rpc 'shop_probe' }
function ClickAt([int]$x, [int]$y) { Rpc 'click' @{ x = $x; y = $y } | Out-Null; Start-Sleep -Milliseconds 700 }

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
if (-not $NoRestart) {
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5q_client.log" -RedirectStandardError "$acc\l5q_client.err.log" | Out-Null
}
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2}) 职业={3}" -f $st.map, $st.tile_x, $st.tile_y, $st.class)

Rpc 'dialog' @{ kind = 'game_shop'; action = 'open' } | Out-Null
$p = $null
foreach ($i in 1..20) { Start-Sleep 1; $p = Probe; if ($null -ne $p.total_items -and [int]$p.total_items -gt 0) { break } }
if ($null -eq $p -or [int]$p.total_items -le 0) { Write-Host '[A] FAIL：商城目录没到（total_items=0）'; exit 10 }
$rows = @($p.rows)
Write-Host ("[A] 开窗：rows={0} filtered={1} viewer={2}" -f $rows.Count, $p.filtered, $(if ($null -eq $p.viewer) { 'null' } else { '非空' }))
$verdict = $true
if ($null -ne $p.viewer) { Write-Host '[A] FAIL：未点试穿钮时 viewer 应为 null'; $verdict = $false }
else { Write-Host '[A] PASS：未点试穿钮时 viewer=null' }

# 找**任一页**上的第一个可试穿格（ItemInfo 按需回包后 previewable 才为真；
# 商城首页是药水类，武器/护甲/坐骑/变形在后面的页 → 用「下一页」按钮翻页扫描）
$nextX = $PanelX + 660 + 8; $nextY = $PanelY + 448 + 7   # Prguse2[243..245] @(660,448) 16x14
$idx = -1; $row = $null; $page = 1
$pages = [int]$p.pages
if ($pages -lt 1) { $pages = 1 }
foreach ($pg in 1..$pages) {
    foreach ($try in 1..10) {
        $rows = @((Probe).rows)
        for ($k = 0; $k -lt $rows.Count; $k++) {
            if ($rows[$k].previewable -eq $true) { $idx = $k; $row = $rows[$k]; break }
        }
        if ($idx -ge 0) { break }
        Start-Sleep -Milliseconds 600
    }
    if ($idx -ge 0) { break }
    ClickAt $nextX $nextY
    $page++
}
if ($idx -lt 0) { Write-Host '[B] FAIL：当前页没有可试穿商品（previewable 全为 false）'; exit 10 }
Write-Host ("[B] 第 {0}/{1} 页找到可试穿格 #{2}" -f $page, $pages, $idx)
$cell = Cell $idx
$btnX = $cell[0] + 29; $btnY = $cell[1] + 133
Write-Host ("[B] 可试穿格 #{0}: item_index={1} type={2} shape={3} 试穿钮点=({4},{5})" -f $idx, $row.item_index, $row.item_type, $row.shape, $btnX, $btnY)
$nodes = @((Rpc 'ui_nodes_at' @{ x = $btnX; y = $btnY }).nodes)
Write-Host ("   ui_nodes_at 命中 {0} 个节点；首节点 rect={1}" -f $nodes.Count, (($nodes | Select-Object -First 1).rect -join ','))
ClickAt $btnX $btnY
$p2 = Probe
$v = $p2.viewer
if ($null -eq $v) {
    Write-Host '[B] FAIL：点了试穿钮 viewer 仍为 null'
    $verdict = $false
} else {
    $expSide = @(151, 115)
    if ([double]$cell[0] -lt 350) { $expSide = @(416, 115) }
    $okItem = ([int]$v.item_index -eq [int]$row.item_index)
    $okDir = ([int]$v.direction -eq 6)
    $okPos = ([double]$v.pos[0] -eq $expSide[0]) -and ([double]$v.pos[1] -eq $expSide[1])
    Write-Host ("[B] viewer: item_index={0}（期望 {1}）direction={2}（期望 6）pos=({3},{4})（期望 ({5},{6})）" -f `
        $v.item_index, $row.item_index, $v.direction, $v.pos[0], $v.pos[1], $expSide[0], $expSide[1])
    $verdict = $verdict -and $okItem -and $okDir -and $okPos
    Write-Host ("[B] {0}" -f $(if ($okItem -and $okDir -and $okPos) { 'PASS：面板打开 + 商品正确 + 初值方向 6 + 位置按左右半选边' } else { 'FAIL' }))

    # C) 转身（面板原点 = 面板坐标 + viewer.pos）
    $vx = $PanelX + [int]$v.pos[0]; $vy = $PanelY + [int]$v.pos[1]
    $prevX = $vx + 81 + 8; $prevY = $vy + 282 + 7     # 16x14 中心
    $nextX = $vx + 160 + 8; $nextY = $vy + 282 + 7
    ClickAt $prevX $prevY
    $d1 = (Probe).viewer.direction
    ClickAt $nextX $nextY
    $d2 = (Probe).viewer.direction
    ClickAt $nextX $nextY
    $d3 = (Probe).viewer.direction
    Write-Host ("[C] 左转→{0}（期望 5）；右转→{1}（期望 6）；右转→{2}（期望 7）" -f $d1, $d2, $d3)
    $okTurn = ([int]$d1 -eq 5) -and ([int]$d2 -eq 6) -and ([int]$d3 -eq 7)
    Write-Host ("[C] {0}" -f $(if ($okTurn) { 'PASS：转身 1..8 环绕与 C# 一致' } else { 'FAIL' }))
    $verdict = $verdict -and $okTurn

    $shot = "$acc\l5q_shop_viewer.png"
    # 截图路径按客户端 CWD（Client-Bevy/）解析，故用相对路径（同 l5l/l5o）
    Rpc 'screenshot' @{ path = '../tools/acceptance/l5q_shop_viewer.png' } | Out-Null
    Start-Sleep -Milliseconds 900
    Write-Host ("[C] 截图 {0}（存在={1}，{2} 字节）" -f $shot, (Test-Path $shot), $(if (Test-Path $shot) { (Get-Item $shot).Length } else { 0 }))

    # D) 关闭
    $closeX = $vx + 230 + 12; $closeY = $vy + 8 + 10
    ClickAt $closeX $closeY
    $p3 = Probe
    $okClose = ($null -eq $p3.viewer)
    Write-Host ("[D] 点关闭钮后 viewer={0} → {1}" -f $(if ($okClose) { 'null' } else { '非空' }), $(if ($okClose) { 'PASS' } else { 'FAIL' }))
    $verdict = $verdict -and $okClose
}

if ($verdict) { Write-Host '=== 全部 PASS ==='; exit 0 } else { Write-Host '=== 有 FAIL ==='; exit 10 }

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
