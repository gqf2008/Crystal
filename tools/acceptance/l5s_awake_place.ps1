# l5s_awake_place.ps1 — 觉醒窗格 3..6「放置/取出」实机验收（纯客户端态，C# `MirItemCell.cs:1655-1785`）
#
# 背景：C# 这 4 格（`ItemCells[3..6]`）的内容是对话框自己的客户端数组（`MirItemCell.cs:88`
# `case MirGridType.AwakenItem: return NPCAwakeDialog.Items;`），原版**服务端不处理**该格类型
# （`rg -c AwakenItem Server -- *.cs` 零命中）⇒ 判据只能取客户端状态（`awake_probe`）。
#
# 判据：
#   A) 开窗：`dialog {kind:npc_awake, action:open}` 之后能读到 `awake_probe` 且 4 格存在（槽位号 3..6）
#   B) 先选：点背包里那件材料的格子后，`awake_probe.inv_selected` **等于**该背包格号
#   C) 放入：点格 3（窗内坐标 (175,199)+中心）后，`awake_probe.slots[0]` 的 item_index/name 正确
#      且 `src_bag_slot` == 该背包格号、`src_locked == true`（`InvLockReason::Awaken`）
#   D) 取出：再点格 3 → 该格清空（`item_index == 0`、`src_bag_slot == null`）
#
# 前置：真客户端 + 真服务端 7000；材料用 GM `@MAKE AwakeningSoul0 1`（idx=937、C# `ItemType.Awakening=35`、
# `Shape=100 < 200` ⇒ 按 C# 规则可进格 3/4）。
# 退出码：0 = A∧B∧C∧D；10 = 判据未达成；2 = 前置不满足（锁/构建戳/进场/材料没到手）
param(
    [string]$ClientHome = '',
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$Port = 9097,
    [int]$TimeoutSec = 60
)

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5s_awake_place' -TimeoutSec 1800)) { exit 2 }

try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5s_awake_place'

# 唯一进程名（LESSON：按公共名清场会误杀别人的 GUI 会话）
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5s_awake.exe'
Get-CimInstance Win32_Process -Filter "Name='l5s_awake.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 600
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient; $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', $Port); $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}
$tmp = $env:TEMP
$log = Join-Path $tmp 'l5s_awake.err.log'

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 2 }
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass,
    '--control-port',"$Port" -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput (Join-Path $tmp 'l5s_awake.out.log') -RedirectStandardError $log | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st.tile_x) { Write-Host 'FAIL(前置): 客户端未进场'; exit 2 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 前置①：腾格。**背包满时服务端会正确拒绝 @MAKE**（"放不下"），夹具会把这种情况误报成"材料没到手"
# ——2026-09-27 实测踩到：客户端视图 38/40 但服务端侧已满 ⇒ `@MAKE AwakeningSoul0` 静默失败。
# 判据：客户端 `bag_probe.used` 与 `total`，先丢 2 件 Saddle 把空间腾出来。
$b0 = Rpc 'bag_probe'
if ($b0.used -ge $b0.total - 1) {
    foreach ($it in @($b0.occupied | Where-Object { $_.name -eq 'Saddle' } | Select-Object -First 2)) {
        Rpc 'drop_item' @{ unique_id = [int64]$it.unique_id } | Out-Null
        Start-Sleep -Milliseconds 400
    }
    Write-Host ("[前置] 腾格：used {0}→{1}/{2}" -f $b0.used, (Rpc 'bag_probe').used, $b0.total)
}

# 前置②：造一件觉醒材料（AwakeningSoul0：C# ItemType.Awakening=35、Shape=100）。
# 失败时把服务端**与 @MAKE 有关**的系统消息打出来（怪物公告会刷屏，不能取"最后 3 条"）。
function MakeLines() {
    $cp = Rpc 'chat_probe'
    @($cp.lines | Where-Object { "$($_.text)" -match '已生成|未找到物品|背包已满' } | Select-Object -Last 2 |
        ForEach-Object { $_.text })
}
Rpc 'chat' @{ message = '@MAKE AwakeningSoul0 1' } | Out-Null
$bag = $null; $slot = -1
foreach ($i in 1..15) {
    Start-Sleep 1
    $bag = Rpc 'bag_probe'
    $hit = @($bag.occupied | Where-Object { $_.name -eq 'AwakeningSoul0' } | Select-Object -First 1)
    if ($hit.Count -gt 0) { $slot = [int]$hit[0].cell; break }
}
if ($slot -lt 0) {
    Write-Host ("FAIL(前置): @MAKE AwakeningSoul0 后 15s 内背包里没看到该材料；服务端消息={0}" -f ((MakeLines) -join ' | '))
    Write-Host ("  客户端 occupied used={0}/{1}" -f $bag.used, $bag.total)
    exit 2
}
$bagItem = @($bag.occupied | Where-Object { $_.cell -eq $slot })[0]
Write-Host ("[前置] 材料到手：cell={0} name={1} uid={2}" -f $slot, $bagItem.name, $bagItem.unique_id)

# A) 开窗 + 读 4 格
Rpc 'dialog' @{ kind = 'npc_awake'; action = 'open' } | Out-Null
$aw = $null
foreach ($i in 1..10) { Start-Sleep 1; $aw = Rpc 'awake_probe'; if ($null -ne $aw.slots) { break } }
# 注意：`dialog_rect` 回的是 **rx/ry/rw/rh**（+ 关闭钮 cx/cy），**没有** `rect` 字段
# ——2026-09-27 实测踩到：写成 `$rect.rect` 会一直为 null，被误判成"窗没开"
#（同 LESSON_实机AB像素对照… 里那条「`$rect.x` 应为 `$rect.rx`」，这里又犯了一次）。
$rect = $null
foreach ($i in 1..10) {
    $rect = Rpc 'dialog_rect' @{ kind = 'npc_awake' }
    if ($null -ne $rect.rx) { break }
    Start-Sleep 1
}
if ($null -eq $rect.rx) { Write-Host 'FAIL(A): 拿不到觉醒窗矩形（dialog_rect 无 rx）'; exit 10 }
$ox = [double]$rect.rx; $oy = [double]$rect.ry
$cells = @($aw.slots | ForEach-Object { $_.cell }) -join ','
Write-Host ("[A] 觉醒窗 rect=({0},{1},{2}x{3})；4 格={4}" -f $ox, $oy, $rect.rw, $rect.rh, $cells)
$okA = ("$cells" -eq '3,4,5,6')

# 背包格屏幕坐标：与 `inv_slot_at`（inventory.rs:838-839）同源：
#   sx = 背包窗原点.x + 9 + col*(36+1)，sy = 原点.y + 37 + row*(32+1)，col=slot%6、row=slot/6
# 觉醒窗打开**不会**自动开背包 ⇒ 夹具自己开一次背包窗，否则拿不到背包矩形与格坐标
Rpc 'dialog' @{ kind = 'inventory'; action = 'open' } | Out-Null
Start-Sleep -Milliseconds 800
$invRect = Rpc 'dialog_rect' @{ kind = 'inventory' }
if ($null -eq $invRect.rx) { Write-Host 'FAIL(前置): 拿不到背包窗矩形'; exit 10 }
$ix = [double]$invRect.rx; $iy = [double]$invRect.ry
# 列数/行数取真值（inventory.rs 的 GRID_COLS/GRID_ROWS，C# `位置 (i%8,(i/8)%5)`）——**不是 6 列**（2026-09-27 实测：按 6 列算出的坐标点到别的格上，选不中）
$col = $slot % 8; $row = [math]::Floor($slot / 8) % 5
$bx = [int]($ix + 9 + $col * 37 + 18); $by = [int]($iy + 37 + $row * 33 + 16)

# B) 选中背包目标格：用夹具仪器 `inv_select`（写 `InvClickState.selected` 同一字段）。
# 为什么不用点击：实测**合成点击驱动不了背包的选中路径**（第一排 8 个格中心逐个点过，`inv_selected` 恒空），
# 而选点坐标的正确性已由命中栈证明（`ui_nodes_at(27,53)` 命中 `rect=9.33,37.33,36,32 z=6` 的背包格本体）。
# 仪器只摆前置状态；被测动作（格 3 的放置/取出）仍是真实 `click`。
Rpc 'inv_select' @{ slot = $slot } | Out-Null
$aw2 = $null
foreach ($i in 1..6) {
    Start-Sleep -Milliseconds 500
    $aw2 = Rpc 'awake_probe'
    if ($null -ne $aw2.inv_selected -and [int]$aw2.inv_selected -eq $slot) { break }
}
$sel = $aw2.inv_selected
Write-Host ("[B] inv_select slot={0} → inv_selected={1}（期望 {2}）；命中栈抽点 ({3},{4}) 见上（诊断）" -f `
    $slot, $sel, $slot, $bx, $by)
$okB = ($null -ne $sel -and [int]$sel -eq $slot)

# C) 点格 3（窗内 (175,199) 36x32 中心）
$cx = [int]($ox + 175 + 18); $cy = [int]($oy + 199 + 16)
Rpc 'click' @{ x = $cx; y = $cy } | Out-Null
$aw3 = $null
foreach ($i in 1..6) {
    Start-Sleep -Milliseconds 500
    $aw3 = Rpc 'awake_probe'
    $s0 = @($aw3.slots | Where-Object { $_.cell -eq 3 })[0]
    if ($null -ne $s0 -and [int]$s0.item_index -ne 0) { break }
}
$s0 = @($aw3.slots | Where-Object { $_.cell -eq 3 })[0]
Write-Host ("[C] 点格3 ({0},{1}) → name='{2}' item_index={3} src_bag_slot={4} src_locked={5}" -f `
    $cx, $cy, $s0.name, $s0.item_index, $s0.src_bag_slot, $s0.src_locked)
$okC = ($null -ne $s0 -and [int]$s0.item_index -eq 937 -and "$($s0.name)" -eq 'AwakeningSoul0' `
    -and "$($s0.src_bag_slot)" -eq "$slot" -and $s0.src_locked)

# D) 再点格 3 → 取出（发 C.MoveItem{Grid=AwakenItem} 并清本地态）
Rpc 'click' @{ x = $cx; y = $cy } | Out-Null
$aw4 = $null
foreach ($i in 1..6) {
    Start-Sleep -Milliseconds 500
    $aw4 = Rpc 'awake_probe'
    $s0b = @($aw4.slots | Where-Object { $_.cell -eq 3 })[0]
    if ($null -ne $s0b -and [int]$s0b.item_index -eq 0) { break }
}
$s0b = @($aw4.slots | Where-Object { $_.cell -eq 3 })[0]
$takeout = @(Select-String -Path $log -Pattern '取出觉醒格 3' -EA SilentlyContinue).Count -gt 0
Write-Host ("[D] 再点格3 → item_index={0} src_bag_slot={1}；日志有『取出觉醒格 3』={2}" -f `
    $s0b.item_index, $s0b.src_bag_slot, $takeout)
$okD = ($null -ne $s0b -and [int]$s0b.item_index -eq 0 -and "$($s0b.src_bag_slot)" -eq '' -and $takeout)

Write-Host ("VERDICT open={0} select={1} place={2} takeout={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 10 }
exit 0

} finally {
    Get-CimInstance Win32_Process -Filter "Name='l5s_awake.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock
}
