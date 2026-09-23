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
    [int]$TileTolerance = 6
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

# 第一层：开菜单 → 点 Service/@tele
Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
Start-Sleep 2
$rows1 = Rpc 'npc_rows'
$tele = $rows1.links | Where-Object { $_.key -ieq '[@tele]' } | Select-Object -First 1
if (-not $tele) { Write-Host ('FAIL: 第一层菜单没有 [@tele]；links=' + (($rows1.links | ForEach-Object { $_.key }) -join ',')); exit 2 }
Write-Host ("[A] 点 {0} @ ({1},{2})" -f $tele.key, $tele.cx, $tele.cy)
Rpc 'click' @{ x = $tele.cx; y = $tele.cy } | Out-Null
Start-Sleep -Seconds 2

# 第二层：目的地菜单 → 点 @moveN
$rows2 = Rpc 'npc_rows'
Write-Host ("[B] 目的地菜单 links={0}" -f (($rows2.links | ForEach-Object { $_.key }) -join ','))
$dest = $rows2.links | Where-Object { $_.key -ieq $DestKey } | Select-Object -First 1
if (-not $dest) {
    $dest = $rows2.links | Where-Object { $_.text -ieq $DestLabel } | Select-Object -First 1
}
if (-not $dest) { Write-Host ('FAIL: 目的地菜单没有 ' + $DestKey); exit 3 }
Write-Host ("[B] 点目的地 {0}({1}) @ ({2},{3})" -f $dest.key, $dest.text, $dest.cx, $dest.cy)
Rpc 'click' @{ x = $dest.cx; y = $dest.cy } | Out-Null
Start-Sleep -Seconds 6

$after = Rpc 'state'
$gold1 = [int](Rpc 'bag_probe').gold
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
