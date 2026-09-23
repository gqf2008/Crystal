# l5e_storage_roundtrip.ps1 — ⑤ 邮件仓库闭环（仓库半段）：造物 → 存 → 取，用格数增减判成交
#
# 判据（缺一不可，全部取状态而非像素）：
#   A) 开仓库前置达成：storage_probe.total != 0（窗口真开、服务端真发了 UserStorage）
#   B) 存入：bag.used 减 1 且 storage.used 加 1，且物品出现在仓库的 occupied 里
#   C) 取回：storage.used 减 1 且 bag.used 加 1
# 判据仪器：bag_probe / storage_probe 的 occupied（格号→名称），动作侧 storage_store/
# storage_take 发的是与点击路径同一个包（C.StoreItem=15 / C.TakeBackItem=16）。
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
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
    -WorkingDirectory 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy' `
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

# 开仓库：走到 Storage_Jake（D002 @ 174,216）→ npc_call [@MAIN] → 点 <Access/@Storage> 链接
Rpc 'chat' @{ message = '@mapmove D002 174 217' } | Out-Null
Start-Sleep 4
$near = Rpc 'nearby' @{ radius = 2000 }
$npc = $near.entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
if (-not $npc) { Write-Host 'FAIL: nearby 里没有仓库 NPC'; exit 1 }
Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
Start-Sleep 2
$rows = Rpc 'npc_rows'
if ($rows.npc_object_id -eq 0) { Write-Host 'FAIL: npc_object_id=0'; exit 3 }
$link = $rows.links | Where-Object { $_.key -eq '[@Storage]' } | Select-Object -First 1
if (-not $link) { Write-Host 'FAIL: 没有 [@Storage] 链接'; exit 2 }
Rpc 'click' @{ x = $link.cx; y = $link.cy } | Out-Null
Start-Sleep -Milliseconds 1800
$st0 = Rpc 'storage_probe'
Write-Host ("storage open: total={0} used={1} visible={2}" -f $st0.total, $st0.used, $st0.visible)
if ($st0.total -eq 0) { Write-Host 'FAIL(A): storage_probe.total=0 —— 仓库窗没开'; exit 4 }
Shot '1_storage_open'

# 存入：背包格 srcCell → 仓库空格（第一个 None）。仓库格号从 occupied 反推空格。
$dstCell = 0
while (($st0.occupied | Where-Object { $_.cell -eq $dstCell })) { $dstCell++ }
Write-Host ("store: bag[{0}] -> storage[{1}]" -f $srcCell, $dstCell)
Rpc 'storage_store' @{ from = $srcCell; to = $dstCell } | Out-Null
Start-Sleep -Seconds 2
$bag1 = Rpc 'bag_probe'; $st1 = Rpc 'storage_probe'
Write-Host ("after store: bag.used={0} storage.used={1} storage.occupied={2}" -f $bag1.used, $st1.used, (($st1.occupied | ForEach-Object { "$($_.cell):$($_.name)" }) -join ','))
$c1 = ($st1.used -eq ($st0.used + 1))
$c2 = ($bag1.used -eq ($bag0.used - 1))
$c3 = (@($st1.occupied | Where-Object { $_.cell -eq $dstCell }).Count -eq 1)
Write-Host ("  store sub-checks: storage+1={0} bag-1={1} item-at-{2}={3}" -f $c1, $c2, $dstCell, $c3)
$stored = $c1 -and $c2 -and $c3
Shot '2_stored'

# 取回：仓库格 dstCell → 原背包格
Rpc 'storage_take' @{ from = $dstCell; to = $srcCell } | Out-Null
Start-Sleep -Seconds 2
$bag2 = Rpc 'bag_probe'; $st2 = Rpc 'storage_probe'
Write-Host ("after take: bag.used={0} storage.used={1}" -f $bag2.used, $st2.used)
$t1 = ($st2.used -eq $st0.used)
$t2 = ($bag2.used -eq $bag0.used)
Write-Host ("  take sub-checks: storage回0={0} bag回满={1}" -f $t1, $t2)
$taken = $t1 -and $t2
Shot '3_taken'

Write-Host ("VERDICT store={0} take={1}" -f $(if ($stored) { 'PASS' } else { 'FAIL' }), $(if ($taken) { 'PASS' } else { 'FAIL' }))
if (-not ($stored -and $taken)) { exit 5 }
