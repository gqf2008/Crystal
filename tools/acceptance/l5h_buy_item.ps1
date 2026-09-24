# l5h_buy_sell.ps1 — ③ 买卖闭环（购买段 + **出售段**）：开买卖页 → 买一件 → 卖一件
#
# 为什么不能只发 BuyItem/SellItem：服务端有原版门槛（`world/item.rs`）
#   1) `npc_page_allows([@BUYSELL]/[@BUY]/...)` —— **必须先打开买卖页**，否则回「请先打开购买页」；
#   2) 商品必须在该 NPC 的销售列表里（包体不含 npc_id，服务端按当前会话的 NPC+页解析）；
#   3) 出售还要「与该买卖 NPC 对话中 + 在其 DataRange(16) 内」。
# 所以夹具走真实顺序：npc_call [@MAIN] → 点菜单里的买卖链接（默认 [@BuySell]）→ 读商品行 → 买 → 卖。
#
# 客户端构建根用 `-ClientHome` 指定（与 l5e/l5g/l5i/l5j 同款）。**必须**能指向含修复的构建：
# 本夹具此前把 `$exe` 硬编码成 wt-p3 的构建，跑的是旧客户端（同类坑见
# `LESSON_运行目标分支e2e前需重建二进制避免陈旧target误报` 的 2026-09-24 变体）。
#
# 判据（全部取状态，不解析像素）：
#   A) 打开买卖页后 npc_goods_probe 有商品行（含价格）
#   B) 购买后 gold 恰好减少「单价 × 数量」
#   C) 该商品真的进了背包（同名数量增加）
#   D) 出售后 gold **增加**且**该实例**（unique_id）离开背包——按实例判，避免"格位被别的物品顶上"的假 PASS
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [string]$MapName = '0106',
    [int]$NpcX = 19,
    [int]$NpcY = 6,
    [string]$BuyKey = '[@BuySell]',
    [int]$BuyCount = 1
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5h_buy_item' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
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
function NameCount($probe, [string]$name) {
    @($probe.occupied | Where-Object { $_.name -eq $name }).Count
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5h_client.log" -RedirectStandardError "$acc\l5h_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 到商人旁 → 开菜单 → 点买卖链接
#   选 NPC 不按「最近」：同图可能有别的 NPC；判据是「这个 NPC 的菜单里真的有买卖链接」，
#   并且整段轮询到条件（换图后 nearby 头几秒可能还是上一张图的实体列表）。
Rpc 'chat' @{ message = "@mapmove $MapName $NpcX $NpcY" } | Out-Null
$npc = $null; $rows = $null; $link = $null
foreach ($i in 1..20) {
    Start-Sleep 1
    $cands = (Rpc 'nearby' @{ radius = 3000 }).entities |
        Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 3
    foreach ($c in $cands) {
        Rpc 'npc_call' @{ object_id = $c.object_id; key = '[@MAIN]' } | Out-Null
        Start-Sleep -Milliseconds 900
        $r = Rpc 'npc_rows'
        $l = $r.links | Where-Object { $_.key -ieq $BuyKey } | Select-Object -First 1
        if (-not $l) { $l = $r.links | Where-Object { $_.key -imatch 'buy|buysell' } | Select-Object -First 1 }
        if ($l) { $npc = $c; $rows = $r; $link = $l; break }
    }
    if ($link) { break }
}
if (-not $link) {
    Write-Host ('FAIL: 20s 内没有找到带买卖链接的 NPC；最后 links=' + (($rows.links | ForEach-Object { $_.key }) -join ','))
    exit 2
}
Write-Host ("商人={0} id={1} dist={2}；点 {3} @ ({4},{5})" -f $npc.name, $npc.object_id, $npc.dist, $link.key, $link.cx, $link.cy)
Rpc 'click' @{ x = $link.cx; y = $link.cy } | Out-Null

# A) 商品行（轮询到商品窗真有行，不用固定 sleep）
$probe = $null
foreach ($i in 1..10) {
    Start-Sleep 1
    $p = Rpc 'npc_goods_probe'
    if ($p.count -gt 0) { $probe = $p; break }
    if ($null -eq $probe) { $probe = $p }
}
Write-Host ("[A] 商品窗 visible={0} title={1} 行数={2}" -f $probe.visible, $probe.title, $probe.count)
if (-not $probe.count -or $probe.count -eq 0) { Write-Host 'FAIL(A): 商品列表为空'; exit 3 }
$item = $probe.goods | Where-Object { $_.price -gt 0 } | Sort-Object price | Select-Object -First 1
Write-Host ("[A] 选最便宜: row={0} item_index={1} unique_id={2} name={3} price={4} 库存={5}" -f `
    $item.row, $item.item_index, $item.unique_id, $item.name, $item.price, $item.count)

$bag0 = Rpc 'bag_probe'
$gold0 = [int]$bag0.gold
$nameBefore = NameCount $bag0 $item.name

# B/C) 购买（轮询到「钱扣够 + 货到手」）
$uid = $item.unique_id
if (-not $uid -or $uid -eq 0) { $uid = $item.item_index }
$r = Rpc 'buy_item' @{ item_index = $uid; count = $BuyCount }
Write-Host ("[B] buy_item -> {0}" -f ($r | ConvertTo-Json -Compress))
$expect = [int]$item.price * $BuyCount
$bag1 = $null; $gold1 = $gold0; $nameAfter = $nameBefore
foreach ($i in 1..10) {
    Start-Sleep 1
    $b = Rpc 'bag_probe'
    if ($null -eq $b) { continue }
    $bag1 = $b; $gold1 = [int]$b.gold; $nameAfter = NameCount $b $item.name
    if ((($gold0 - $gold1) -eq $expect) -and ($nameAfter -gt $nameBefore)) { break }
}
$goldDelta = $gold0 - $gold1
Write-Host ("[B] gold {0}->{1} (扣 {2}, 期望 {3})；[C] 背包 used {4}->{5}，同名 {6} {7}->{8}" -f `
    $gold0, $gold1, $goldDelta, $expect, $bag0.used, $bag1.used, $item.name, $nameBefore, $nameAfter)
$okA = ($probe.count -gt 0)
$okB = ($goldDelta -eq $expect)
$okC = ($nameAfter -gt $nameBefore)

# D) 出售：把刚买到的**那个实例**卖回（买卖页仍开着时服务端才会受理）
#    判据按 unique_id 判实例是否离开背包——按「同名数量减少」会被"别的同名牌顶上"骗过。
$sellBag = $bag1
$sellTarget = $null
# 目标必须挑「uid 在背包里**唯一**」的实例：
#   服务端按 uid 定位（`inventory.get_item(uid)` 取第一个匹配），而**存量历史数据里 uid 是重复的**
#   （旧计数器每进程从 1 起，实测背包内 uid=1 有 4 件、uid=3 有 3 件、uid=4 有 3 件）。
#   挑重复 uid 会卖到"另一件同号物品"上——判据（该 uid 是否离开背包）依然成立，
#   但**卖给谁**不可控，夹具会给出误导性的输出。根治在服务端（启动时把 uid 计数器种子推到
#   存量最大值之上）；本夹具只保证自己这一轮挑的实例是明确的。
$uidCounts = @{}
foreach ($o in $sellBag.occupied) { $uidCounts[[string]$o.unique_id] = 1 + [int]($uidCounts[[string]$o.unique_id]) }
$uniqueInstances = $sellBag.occupied | Where-Object { $_.unique_id -ne 0 -and $uidCounts[[string]$_.unique_id] -eq 1 }
$bought = $uniqueInstances | Where-Object { $_.name -eq $item.name } | Select-Object -First 1
if ($bought) { $sellTarget = $bought } else { $sellTarget = $uniqueInstances | Select-Object -First 1 }
$okD = $false
$goldDeltaSell = 0
$sellLine = ''
if (-not $sellTarget) {
    Write-Host 'FAIL(D): 背包里没有带 unique_id 的实例可卖（bag_probe 无 unique_id？）'
} else {
    Write-Host ("[D] 卖格 {0} 实例 uid={1} name={2} count={3}" -f `
        $sellTarget.cell, $sellTarget.unique_id, $sellTarget.name, 1)
    $goldSell0 = [int]$sellBag.gold
    $r2 = Rpc 'sell_item' @{ unique_id = $sellTarget.unique_id; count = 1 }
    Write-Host ("[D] sell_item -> {0}" -f ($r2 | ConvertTo-Json -Compress))
    $sellBag2 = $null
    foreach ($i in 1..10) {
        Start-Sleep 1
        $b2 = Rpc 'bag_probe'
        if ($null -eq $b2) { continue }
        $sellBag2 = $b2
        $stillThere = @($b2.occupied | Where-Object { $_.unique_id -eq $sellTarget.unique_id }).Count
        if (([int]$b2.gold -gt $goldSell0) -and ($stillThere -eq 0)) { break }
    }
    if ($sellBag2) {
        $stillThere = @($sellBag2.occupied | Where-Object { $_.unique_id -eq $sellTarget.unique_id }).Count
        $goldDeltaSell = [int]$sellBag2.gold - $goldSell0
        $okD = (($goldDeltaSell -gt 0) -and ($stillThere -eq 0))
        Write-Host ("[D] gold {0}->{1} (增 {2})，实例仍在背包={3}" -f `
            $goldSell0, [int]$sellBag2.gold, $goldDeltaSell, ($stillThere -ne 0))
    }
}
# 服务端侧的确切成交金额（交叉核对用，不作为判据）：`SellItem: ... sold item=... for N gold`
$srvLog = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance\e2e_server.log'
if (Test-Path $srvLog) {
    $sellLine = (Select-String -Path $srvLog -Pattern 'SellItem: .* sold .* for \d+ gold' | Select-Object -Last 1).Line
    if ($sellLine) { Write-Host ('[D] 服务端成交行: ' + $sellLine.Substring([Math]::Max(0, $sellLine.Length - 90))) }
}

Write-Host ("VERDICT open_page={0} buy_gold_delta={1} buy_item_received={2} sell_gold_gain={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 5 }

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
