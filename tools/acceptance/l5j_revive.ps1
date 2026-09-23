# l5j_revive.ps1 — ② 复活闭环：死亡（GM @die）→ 回城复活（C.TownRevive）→ 状态与位置恢复
#
# 判据（取状态）：A) 死亡前 dead=false 且 hp>0
#               B) `@die` 后 dead=true 且 hp<=0（真的死了，不是"看起来死了"）
#               C) `revive_town` 后 dead=false 且 hp>0（复活真的生效）
#               D) 复活后位置回到**绑定点**（服务端 TownRevive 会传回 bind map/坐标；
#                  这里只断言"位置与死亡点不同或等于绑定点"，避免把绑定点写死）
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$DieWaitSec = 5,
    # 复活必须回到**绑定点地图**（C# TownRevive = Teleport(bindMap,bindX,bindY)）。
    # 2026-09-23 实测缺陷：只搬坐标不换图 → 人留在死亡地图上。判据因此显式断言 map 与坐标。
    # bevychar 的绑定：bind_map_index=1 → map 文件名 '0'(BichonProvince) @ (288,616)。
    [string]$ExpectReviveMap = '0',
    [int]$ExpectReviveX = 288,
    [int]$ExpectReviveY = 616,
    [int]$LandingTolerance = 3,
    # 死亡点强制换到**与绑定点不同的地图**再自杀——否则"人已经站在绑定点上"，
    # 复活就算不切图也会通过（判据失去意义）。默认 map 文件名 '2' = SerpentValley。
    [string]$DieMap = '2',
    [int]$DieX = 500,
    [int]$DieY = 485,
    # 客户端构建根（默认 wt-p3 旧约定；其构建不含 #3044 换图 panic 修复，
    # 本夹具全程换图，实跑必须指到含修复的构建根，如 -ClientHome <worktree>）
    [string]$ClientHome = ''
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
# -ClientHome 未给时保持 wt-p3 旧约定
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Wait-State([int]$TimeoutSec = 20) {
    # 换图后客户端会重建场景（本地玩家实体短暂不存在 → state 返回空），判据必须等它稳定，
    # 否则会把"正在加载"读成"复活失败"（假红）。
    for ($i = 0; $i -lt $TimeoutSec; $i++) {
        $s = Rpc 'state'
        if ($null -ne $s -and $null -ne $s.tile_x -and "$($s.map)" -ne '') { return $s }
        Start-Sleep 1
    }
    return (Rpc 'state')
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5j_client.log" -RedirectStandardError "$acc\l5j_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("[A] 进图 map={0} tile=({1},{2}) hp={3}/{4} dead={5}" -f $st.map, $st.tile_x, $st.tile_y, $st.hp, $st.max_hp, $st.dead)
$alive = (-not $st.dead) -and ([int]$st.hp -gt 0)

# 先把死亡点挪到与绑定点**不同的地图**（判据才有意义）
Rpc 'chat' @{ message = "@mapmove $DieMap $DieX $DieY" } | Out-Null
# 轮询到真的到了死亡图（上限 20s）：冷图/高 lag 时固定 sleep 会读在传送生效之前
# （实跑抓到过：服务端 62% lag，命令积压在邮箱，客户端探针一直是旧图旧坐标）。
$at = $null
foreach ($i in 1..20) {
    Start-Sleep 1
    $at = Rpc 'state'
    if ("$($at.map)" -eq $DieMap) { break }
}
Write-Host ("[A2] 到死亡地图: map={0} tile=({1},{2})（绑定图期望 {3}）" -f $at.map, $at.tile_x, $at.tile_y, $ExpectReviveMap)
$dieTile = @($at.tile_x, $at.tile_y)
$diedOffBindMap = ("$($at.map)" -ne $ExpectReviveMap)

# B) 死亡：GM @die（C# case "DIE"：自杀）
Rpc 'chat' @{ message = '@die' } | Out-Null
# 轮询到真死（上限 20s，DieWaitSec 参数保留兼容但不再固定等待）：
# 高 lag 下 TakeDamage 落到玩家邮箱会延迟，固定 5s 会假 FAIL。
$dead = $null
foreach ($i in 1..20) {
    Start-Sleep 1
    $dead = Rpc 'state'
    if ([bool]$dead.dead -and [int]$dead.hp -le 0) { break }
}
Write-Host ("[B] @die 后 hp={0}/{1} dead={2} map={3} tile=({4},{5})" -f $dead.hp, $dead.max_hp, $dead.dead, $dead.map, $dead.tile_x, $dead.tile_y)
$died = [bool]$dead.dead
$dieMap = "$($dead.map)"

# C/D) 复活
Rpc 'revive_town' | Out-Null
# 轮询到复活生效且回到绑定图（上限 25s；Wait-State 只等「状态非空」
# 会把「还没复活」读成复活失败/复活图不符——假红）
$rev = $null
foreach ($i in 1..25) {
    Start-Sleep 1
        $rev = Rpc 'state'
    if ($null -eq $rev -or $null -eq $rev.tile_x) { continue }
    if ((-not $rev.dead) -and [int]$rev.hp -gt 0 -and "$($rev.map)" -eq $ExpectReviveMap) { break }
}
Write-Host ("[C] revive_town 后 hp={0}/{1} dead={2} map={3} tile=({4},{5})" -f $rev.hp, $rev.max_hp, $rev.dead, $rev.map, $rev.tile_x, $rev.tile_y)
$revived = ((-not $rev.dead) -and ([int]$rev.hp -gt 0))
$moved = ([int]$rev.tile_x -ne [int]$dieTile[0]) -or ([int]$rev.tile_y -ne [int]$dieTile[1])
$backToBindMap = ("$($rev.map)" -eq $ExpectReviveMap)
$backToBindSpot = ([Math]::Abs([int]$rev.tile_x - $ExpectReviveX) -le $LandingTolerance) -and ([Math]::Abs([int]$rev.tile_y - $ExpectReviveY) -le $LandingTolerance)
Write-Host ("[D] 死亡图={0} → 复活图={1}（期望 {2}）；落点=({3},{4})（期望≈{5},{6}）；与死亡点不同={7}" -f `
    $dieMap, $rev.map, $ExpectReviveMap, $rev.tile_x, $rev.tile_y, $ExpectReviveX, $ExpectReviveY, $moved)

$okA = [bool]$alive
$okB = [bool]($died -and $diedOffBindMap)
$okC = [bool]$revived
$okD = [bool]($backToBindMap -and $backToBindSpot)
Write-Host ("VERDICT alive_before={0} died={1} revived={2} back_to_bind_map_and_spot={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 5 }
