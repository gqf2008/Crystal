# l5e_storage_roundtrip.ps1 — ⑤ 邮件仓库闭环（仓库半段）：造物 → 存 → 取，用格数增减判成交
#
# 判据（缺一不可，全部取状态而非像素）：
#   A) 开仓库前置达成：storage_probe.total != 0（窗口真开、服务端真发了 UserStorage）
#   B) 存入：bag.used 减 1 且 storage.used 加 1，且物品出现在仓库的 occupied 里
#   C) 取回：storage.used 减 1 且 bag.used 加 1
# 判据仪器：bag_probe / storage_probe 的 occupied（格号→名称），动作侧 storage_store/
# storage_take 发的是与点击路径同一个包（C.StoreItem=15 / C.TakeBackItem=16）。
#
# 客户端构建根可用 `-ClientHome` 指定（与 l5g/l5i/l5j 同款）。**必须**能指向含修复的构建：
# 本夹具此前把 `$exe` 硬编码成主工作区的构建，实机跑出来的是**旧客户端**（主工作区落后
# master 数十个提交，缺 #3058 的 npc_object_id 边沿清零修复），于是「第一次 npc_call 后
# npc_object_id 读回 0、第二次才正常」被当成本端竞态追了一轮——真因是夹具指向了旧构建。
# 夹具自身的构建来源必须显式、可覆盖，否则测的根本不是当前代码。
param(
    [string]$ClientHome = ''
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Shot([string]$n) { Rpc 'screenshot' @{ path = "$acc\player_shots\l5e_$n.png" } | Out-Null; Start-Sleep -Milliseconds 700 }

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5e_client.log" -RedirectStandardError "$acc\l5e_client.err.log" -PassThru
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)

# 造物（GM 通道）：一件可堆叠的普通消耗品，避免装备栏/重量干扰
Rpc 'chat' @{ message = '@MAKE Saddle 1' } | Out-Null
Start-Sleep -Seconds 2
$bag0 = Rpc 'bag_probe'
Write-Host ("bag before: used={0}/{1} occupied={2}" -f $bag0.used, $bag0.total, (($bag0.occupied | ForEach-Object { "$($_.cell):$($_.name)" }) -join ','))
$item = $bag0.occupied | Where-Object { $_.name -eq 'Saddle' } | Select-Object -First 1
if (-not $item) { Write-Host 'FAIL: @MAKE 后背包里没有 Saddle'; exit 1 }
$srcCell = [int]$item.cell

# 开仓库：走到仓库 NPC（Storage_Jake，map 40 / 文件名 D002 @ 174,216）
#   → npc_call [@MAIN] → 点 <Access/@Storage> 链接
#
# 夹具自身硬化（2026-09-24，实机踩到假 FAIL）：
#   ① **按名字选 NPC**（Storage_* / Warehouse_*），不按「最近」——同图可能有别的 NPC
#      （D002 上还有 StrangeMan），而且换图后 `nearby` 头几秒可能仍是上一张图的实体列表，
#      「取最近的」会把上一图的 NPC（如传送 NPC）当成仓库 NPC 去 npc_call，服务端找不到该
#      object_id 就静默丢弃 → 探针读回 npc_object_id=0，被记成「仓库窗没开」的假 FAIL。
#   ② 轮询到「NPC 出现在本图」与「[@MAIN] 真的开窗（npc_object_id 非 0）」两个条件，
#      不用固定 sleep（冷图客户端首次生成对象要数秒）。
# 开仓库是**重试到判据成立**的步骤，不是「call 一次点一下」（2026-09-24 实机踩到）：
#   换图后地图重建期间发出的 CallNPC，客户端可能已经渲染出行（npc_rows.links 有 [@Storage]）
#   但 `npc_object_id` 仍为 0——此时点行发出的 CallNPC{object_id:0} 会被服务端静默丢弃，
#   表现成「点了没反应」。判据必须是 storage_probe.total != 0（窗真开），而不是中间态字段；
#   未成立就重新按名字定位 NPC 并重发 npc_call（object_id 可能随地图重建变化）。
Rpc 'chat' @{ message = '@mapmove D002 174 217' } | Out-Null
$st0 = $null
$npc = $null
$opened = $false
foreach ($attempt in 1..3) {
    # ① 按名字定位仓库 NPC（Storage_* / Warehouse_*），轮询到它出现在本图
    $npc = $null
    foreach ($i in 1..20) {
        Start-Sleep 1
        $near = Rpc 'nearby' @{ radius = 2000 }
        $npc = $near.entities |
            Where-Object { $_.kind -eq 'npc' -and ($_.name -like 'Storage_*' -or $_.name -like 'Warehouse_*') } |
            Sort-Object dist | Select-Object -First 1
        if ($npc) { break }
    }
    if (-not $npc) {
        Write-Host ('FAIL: 20s 内本图没有仓库 NPC（Storage_*/Warehouse_*）；nearby npcs=' +
            (($near.entities | Where-Object { $_.kind -eq 'npc' } | ForEach-Object { $_.name }) -join ','))
        exit 1
    }
    Write-Host ("[attempt {0}] 仓库 NPC={1} id={2} dist={3}" -f $attempt, $npc.name, $npc.object_id, [int]$npc.dist)

    # ② 开菜单：npc_call [@MAIN] → 轮询到 npc_object_id 非 0（未开窗就重试）
    Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
    $rows = $null
    $rowOpen = $false
    foreach ($i in 1..10) {
        Start-Sleep 1
        $rows = Rpc 'npc_rows'
        if ($rows.npc_object_id -ne 0) { $rowOpen = $true; break }
    }
    if (-not $rowOpen) {
        Write-Host ("[attempt {0}] npc_object_id 仍为 0（links={1}）——重试" -f $attempt,
            (($rows.links | ForEach-Object { $_.key }) -join ','))
        continue
    }
    $link = $rows.links | Where-Object { $_.key -eq '[@Storage]' } | Select-Object -First 1
    if (-not $link) {
        # 2026-09-24 实测：换图/重建后 object_id 过期时，`npc_object_id` 可能读回非 0 而 `links` 为空——
        # 这与「窗没开」同属**可重试**状态，不能一读就判死（首跑就在 attempt 1 上因此假红，单独重跑即绿；
        # 独立探针证明产品侧 [@Storage]/[@exit] 两条链接与 visible=true 都正常）。
        Write-Host ("[attempt {0}] npc_rows 有对象但 links 为空（links={1}）——重试" -f $attempt,
            (($rows.links | ForEach-Object { $_.key }) -join ','))
        continue
    }

    # ③ 点 <Access/@Storage> → 轮询到仓库窗真开（total 非 0 才算达成前置）
    Rpc 'click' @{ x = $link.cx; y = $link.cy } | Out-Null
    foreach ($i in 1..10) {
        Start-Sleep 1
        $st0 = Rpc 'storage_probe'
        if ($st0.total -ne 0) { $opened = $true; break }
    }
    if ($opened) { break }
    Write-Host ("[attempt {0}] 点了 [@Storage] 但 storage_probe.total 仍为 0——重试" -f $attempt)
}
if (-not $opened) {
    Write-Host ('FAIL(A): 3 次尝试后仓库窗仍未开（storage_probe.total=0）；npc=' + $npc.name)
    exit 4
}
Write-Host ("storage open: total={0} used={1} visible={2}" -f $st0.total, $st0.used, $st0.visible)
Shot '1_storage_open'

# 存入：背包格 srcCell → 仓库空格（第一个 None）。仓库格号从 occupied 反推空格。
$dstCell = 0
while (($st0.occupied | Where-Object { $_.cell -eq $dstCell })) { $dstCell++ }
Write-Host ("store: bag[{0}] -> storage[{1}]" -f $srcCell, $dstCell)
Rpc 'storage_store' @{ from = $srcCell; to = $dstCell } | Out-Null
# 轮询到判据成立（服务端 StoreItem 生效 + 两个探针都真的有回包）——单次「sleep 2 后读一次」
# 会踩两类假 FAIL：① 包晚到；② 探针偶发空回包（debug 客户端帧重时实测出现过）。
$bag1 = $null; $st1 = $null
foreach ($i in 1..10) {
    Start-Sleep 1
    $b = Rpc 'bag_probe'; $s = Rpc 'storage_probe'
    if ($null -ne $b -and $null -ne $s) {
        $bag1 = $b; $st1 = $s
        if ($s.used -eq ($st0.used + 1) -and $b.used -eq ($bag0.used - 1)) { break }
    }
}
Write-Host ("after store: bag.used={0} storage.used={1} storage.occupied={2}" -f $bag1.used, $st1.used, (($st1.occupied | ForEach-Object { "$($_.cell):$($_.name)" }) -join ','))
$c1 = ($st1.used -eq ($st0.used + 1))
$c2 = ($bag1.used -eq ($bag0.used - 1))
$c3 = (@($st1.occupied | Where-Object { $_.cell -eq $dstCell }).Count -eq 1)
Write-Host ("  store sub-checks: storage+1={0} bag-1={1} item-at-{2}={3}" -f $c1, $c2, $dstCell, $c3)
$stored = $c1 -and $c2 -and $c3
Shot '2_stored'

# 取回：仓库格 dstCell → 原背包格
Rpc 'storage_take' @{ from = $dstCell; to = $srcCell } | Out-Null
$bag2 = $null; $st2 = $null
foreach ($i in 1..10) {
    Start-Sleep 1
    $b = Rpc 'bag_probe'; $s = Rpc 'storage_probe'
    if ($null -ne $b -and $null -ne $s) {
        $bag2 = $b; $st2 = $s
        if ($s.used -eq $st0.used -and $b.used -eq $bag0.used) { break }
    }
}
Write-Host ("after take: bag.used={0} storage.used={1}" -f $bag2.used, $st2.used)
$t1 = ($st2.used -eq $st0.used)
$t2 = ($bag2.used -eq $bag0.used)
Write-Host ("  take sub-checks: storage回0={0} bag回满={1}" -f $t1, $t2)
$taken = $t1 -and $t2
Shot '3_taken'

Write-Host ("VERDICT store={0} take={1}" -f $(if ($stored) { 'PASS' } else { 'FAIL' }), $(if ($taken) { 'PASS' } else { 'FAIL' }))
if (-not ($stored -and $taken)) { exit 5 }
