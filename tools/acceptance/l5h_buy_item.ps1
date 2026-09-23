# l5h_buy_item.ps1 — ③ 买卖闭环（购买段）：开购买页 → 买一件 → 金币扣减 + 商品真的到手
#
# 为什么不能只发 BuyItem：服务端有两道原版门槛（`world/item.rs` BuyItemRequest）
#   1) `npc_page_allows([@BUYSELL]/[@BUY]/[@BUYBACK]/…)` —— **必须先打开购买页**，否则回「请先打开购买页」；
#   2) 商品必须在该 NPC 的销售列表里（包体不含 npc_id，服务端按当前会话的 NPC+页解析）。
# 所以夹具走真实顺序：npc_call [@MAIN] → 点菜单里的购买链接（默认 [@BuySell]）→ 读商品行 → 买。
#
# 判据（取状态）：A) 打开购买页后 npc_goods_probe 有商品行（含价格）
#               B) 购买后 gold 恰好减少「单价 × 数量」
#               C) 该商品真的进了背包（背包格数/同名单品数量增加）
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$MapName = '0106',
    [int]$NpcX = 19,
    [int]$NpcY = 6,
    [string]$BuyKey = '[@BuySell]',
    [int]$BuyCount = 1
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
$exe = "$wt\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$wt\Client-Bevy" `
    -RedirectStandardOut "$acc\l5h_client.log" -RedirectStandardError "$acc\l5h_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)

# 到商人旁 → 开菜单 → 点购买链接
Rpc 'chat' @{ message = "@mapmove $MapName $NpcX $NpcY" } | Out-Null
Start-Sleep 4
$npc = (Rpc 'nearby' @{ radius = 3000 }).entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
if (-not $npc) { Write-Host 'FAIL: 附近没有商人 NPC'; exit 1 }
Write-Host ("商人={0} id={1} dist={2}" -f $npc.name, $npc.object_id, $npc.dist)
Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
Start-Sleep 2
$rows = Rpc 'npc_rows'
$link = $rows.links | Where-Object { $_.key -ieq $BuyKey } | Select-Object -First 1
if (-not $link) {
    $link = $rows.links | Where-Object { $_.key -imatch 'buy|buysell' } | Select-Object -First 1
}
if (-not $link) { Write-Host ('FAIL: 菜单里没有购买链接；links=' + (($rows.links | ForEach-Object { $_.key }) -join ',')); exit 2 }
Write-Host ("点购买链接 {0} @ ({1},{2})" -f $link.key, $link.cx, $link.cy)
Rpc 'click' @{ x = $link.cx; y = $link.cy } | Out-Null
Start-Sleep -Seconds 2

# A) 商品行
$probe = Rpc 'npc_goods_probe'
Write-Host ("[A] 商品窗 visible={0} title={1} 行数={2}" -f $probe.visible, $probe.title, $probe.count)
if (-not $probe.count -or $probe.count -eq 0) { Write-Host 'FAIL(A): 商品列表为空'; exit 3 }
$item = $probe.goods | Where-Object { $_.price -gt 0 } | Sort-Object price | Select-Object -First 1
Write-Host ("[A] 选最便宜: row={0} item_index={1} unique_id={2} name={3} price={4} 库存={5}" -f `
    $item.row, $item.item_index, $item.unique_id, $item.name, $item.price, $item.count)

$bag0 = Rpc 'bag_probe'
$gold0 = [int]$bag0.gold
$before = @{}
foreach ($o in $bag0.occupied) { $before[[string]$o.name] = 1 + [int]($before[[string]$o.name]) }

# B/C) 购买
$uid = $item.unique_id
if (-not $uid -or $uid -eq 0) { $uid = $item.item_index }
$r = Rpc 'buy_item' @{ item_index = $uid; count = $BuyCount }
Write-Host ("[B] buy_item -> {0}" -f ($r | ConvertTo-Json -Compress))
Start-Sleep -Seconds 2
$bag1 = Rpc 'bag_probe'
$gold1 = [int]$bag1.gold
$after = @{}
foreach ($o in $bag1.occupied) { $after[[string]$o.name] = 1 + [int]($after[[string]$o.name]) }
$goldDelta = $gold0 - $gold1
$expect = [int]$item.price * $BuyCount
$nameBefore = [int]($before[[string]$item.name]); $nameAfter = [int]($after[[string]$item.name])
Write-Host ("[B] gold {0}->{1} (扣 {2}, 期望 {3})；[C] 背包 used {4}->{5}，同名 {6} {7}->{8}" -f `
    $gold0, $gold1, $goldDelta, $expect, $bag0.used, $bag1.used, $item.name, $nameBefore, $nameAfter)

$okA = ($probe.count -gt 0)
$okB = ($goldDelta -eq $expect)
$okC = (($bag1.used -gt $bag0.used) -or ($nameAfter -gt $nameBefore))
Write-Host ("VERDICT open_page={0} gold_delta={1} item_received={2}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), $(if ($okC) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC)) { exit 5 }
