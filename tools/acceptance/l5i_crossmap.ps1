# l5i_crossmap.ps1 — ② 跨图传送：传送 NPC 两层菜单（Service/@tele → 目的地 @moveN）→ 真换图
#
# 脚本依据（ServerRust/Daneo1989/Envir/NPCs/BichonProvince/BorderVillage/Border_Transport-0.txt）：
#   [@Main-1] 里 `I'll use this <Service/@tele>`；[@tele] 里
#   `<CastleBichon/@move1>(500) <SerpentValley/@move2>(1000) <MudWall/@move3>(2000)
#    <TaoistSchool/@move4>(3000) <CastleGi-Ryoong/@move5>(2000)`；
#   @move2 = `MOVE 2 500 485` + `TAKEGOLD 1000` → 地图名 '2'(SerpentValley) @ (500,485)。
#   @move1 的 `MOVE 0` 是**同图**（'0'=BichonProvince，就是该 NPC 所在图），不构成跨图判据，故用 @move2。
#
# 判据（取状态，不解析日志）：A) 传送前 state.map='0' 且 gold 足够
#                            B) 点完两层菜单后 state.map 变成 '2'（换图真的发生）
#                            C) 金币恰好扣 1000（服务费）+ 落点接近 (500,485)
#
# 等待全是「轮询到条件」不是固定 sleep：NPC 对话从 npc_call 到客户端收到「NPC 对话: N 行」
# 实测可超 2s（debug 服务端冷脚本 ~3s），固定 sleep 2 会读在菜单到达之前 → 假 FAIL
# （links= 空但菜单其实随后到了，客户端日志可核）。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$NpcMap = '0',
    [int]$NpcX = 287,
    [int]$NpcY = 615,
    [string]$DestLabel = 'SerpentValley',
    [string]$DestKey = '[@move2]',
    [string]$ExpectMap = '2',
    [int]$ExpectGold = 1000,
    [int]$ExpectX = 500,
    [int]$ExpectY = 485,
    [int]$TileTolerance = 6,
    # 客户端构建根（其 Client-Bevy\target\debug\client_bevy.exe）；默认 wt-p3 保持原约定。
    # wt-p3 构建不含 #3044（换图 panic），本夹具有 @mapmove，建议指向含修复的构建。
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

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5i_client.log" -RedirectStandardError "$acc\l5i_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 到传送 NPC 旁
Rpc 'chat' @{ message = "@mapmove $NpcMap $NpcX $NpcY" } | Out-Null
Start-Sleep 4
$gold0 = [int](Rpc 'bag_probe').gold
$before = Rpc 'state'
Write-Host ("[A] 传送前 map={0} tile=({1},{2}) gold={3}" -f $before.map, $before.tile_x, $before.tile_y, $gold0)
$npc = (Rpc 'nearby' @{ radius = 3000 }).entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
if (-not $npc) { Write-Host 'FAIL: 传送 NPC 附近没有 NPC'; exit 1 }
Write-Host ("[A] 传送 NPC={0} id={1} dist={2}" -f $npc.name, $npc.object_id, $npc.dist)

# 第一层：开菜单 → 点 Service/@tele（轮询到链接出现，上限 15s）
Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
$rows1 = $null; $tele = $null
foreach ($i in 1..15) {
    Start-Sleep 1
    $rows1 = Rpc 'npc_rows'
    $tele = $rows1.links | Where-Object { $_.key -ieq '[@tele]' } | Select-Object -First 1
    if ($tele) { break }
}
if (-not $tele) { Write-Host ('FAIL: 第一层菜单没有 [@tele]；links=' + (($rows1.links | ForEach-Object { $_.key }) -join ',')); exit 2 }
Write-Host ("[A] 点 {0} @ ({1},{2})" -f $tele.key, $tele.cx, $tele.cy)
Rpc 'click' @{ x = $tele.cx; y = $tele.cy } | Out-Null

# 第二层：目的地菜单 → 点 @moveN（同样轮询，上限 15s）
$rows2 = $null; $dest = $null
foreach ($i in 1..15) {
    Start-Sleep 1
    $rows2 = Rpc 'npc_rows'
    $dest = $rows2.links | Where-Object { $_.key -ieq $DestKey } | Select-Object -First 1
    if (-not $dest) {
        $dest = $rows2.links | Where-Object { $_.text -ieq $DestLabel } | Select-Object -First 1
    }
    if ($dest) { break }
}
Write-Host ("[B] 目的地菜单 links={0}" -f (($rows2.links | ForEach-Object { $_.key }) -join ','))
if (-not $dest) { Write-Host ('FAIL: 目的地菜单没有 ' + $DestKey); exit 3 }
Write-Host ("[B] 点目的地 {0}({1}) @ ({2},{3})" -f $dest.key, $dest.text, $dest.cx, $dest.cy)
Rpc 'click' @{ x = $dest.cx; y = $dest.cy } | Out-Null

# 换图与扣费：轮询到 state.map 变了（上限 20s；MOVE+TAKEGOLD 是服务端动作）
$after = $null
foreach ($i in 1..20) {
    Start-Sleep 1
    $after = Rpc 'state'
    if ("$($after.map)" -eq $ExpectMap) { break }
}
# 扣费轮询：GoldChanged 包可能晚于换图包几帧到达（同 #ACT 内 MOVE 先 TAKEGOLD 后），
# 一次读会假 FAIL（实跑抓到过：DB 已扣 1000、客户端探针还是旧值）。上限 10s。
$gold1 = $gold0
foreach ($i in 1..10) {
    $gold1 = [int](Rpc 'bag_probe').gold
    if (($gold0 - $gold1) -eq $ExpectGold) { break }
    Start-Sleep 1
}
$goldDelta = $gold0 - $gold1
$mapChanged = ("$($after.map)" -eq $ExpectMap)
$near = ([Math]::Abs([int]$after.tile_x - $ExpectX) -le $TileTolerance) -and ([Math]::Abs([int]$after.tile_y - $ExpectY) -le $TileTolerance)
Write-Host ("[C] 传送后 map={0} (期望 {1}) tile=({2},{3}) (期望≈{4},{5})；gold {6}->{7} (扣 {8}, 期望 {9})" -f `
    $after.map, $ExpectMap, $after.tile_x, $after.tile_y, $ExpectX, $ExpectY, $gold0, $gold1, $goldDelta, $ExpectGold)
$okB = $mapChanged
$okC = ($goldDelta -eq $ExpectGold) -and $near
Write-Host ("VERDICT map_changed={0} fee_and_landing={1}" -f `
    $(if ($okB) { 'PASS' } else { 'FAIL' }), $(if ($okC) { 'PASS' } else { 'FAIL' }))
if (-not ($okB -and $okC)) { exit 5 }
