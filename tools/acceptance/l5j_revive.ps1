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
    # B2：**死亡态下第二次换图**的目标（默认换到另一张图，专门覆盖"死着换图"这条序列）
    [string]$DieMap2 = '3',
    [int]$DieX2 = 361,
    [int]$DieY2 = 342,
    # 客户端构建根（默认 wt-p3 旧约定；其构建不含 #3044 换图 panic 修复，
    # 本夹具全程换图，实跑必须指到含修复的构建根，如 -ClientHome <worktree>）
    [string]$ClientHome = ''
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5j_revive' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
# -ClientHome 未给时保持 wt-p3 旧约定
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5j_client.exe'

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
Get-CimInstance Win32_Process -Filter "Name='l5j_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
# 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
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

# B) 死亡：GM @die（C# case "DIE"：自杀）——**重试到判据成立**。
#
# 两处实机定性（2026-09-24，都在本夹具里踩到）：
#   ① 判据只取客户端 `dead`，**不**要求 hp<=0。C# 客户端 `GameScene.Death(S.Death)` 只置
#      `User.Dead = true`（`Client/MirScenes/GameScene.cs:3781-3789`），服务端 `PlayerObject.Die()`
#      是直接赋值 `HP = 0`（不是 `SetHP(0)`，不发 HealthChanged），所以客户端探针的 hp 会停在
#      死前值——把 hp<=0 写进判据会把「C# 同款行为」判成假 FAIL（实测 @die 后 dead=True、hp 仍 50005）。
#   ② `@mapmove` 与 `@die` 是两条独立聊天包，服务端按邮箱先后处理；若传送在 @die **之后**落地，
#      传送的状态重建会把死亡态抹掉——实测 @die 后 dead 仍 False，50s 后再发一次立即生效。
#      所以判据要「重试到 dead」，不能发一次就当结论。
$died = $false
$dead = $null
foreach ($attempt in 1..3) {
    Rpc 'chat' @{ message = '@die' } | Out-Null
    foreach ($i in 1..8) {
        Start-Sleep 1
        $dead = Rpc 'state'
        if ([bool]$dead.dead) {
            $died = $true
            break
        }
    }
    if ($died) { break }
    Write-Host ("[attempt {0}] @die 未生效（dead 仍为 False）——重试" -f $attempt)
}
Write-Host ("[B] @die 后 hp={0}/{1} dead={2} map={3} tile=({4},{5})" -f $dead.hp, $dead.max_hp, $dead.dead, $dead.map, $dead.tile_x, $dead.tile_y)
$died = [bool]$died
$dieMap = "$($dead.map)"

# B2) **死亡态下换图**：换图后本地玩家实体必须仍在、状态可读、dead 仍为 true。
#
# 这条钉的是历史缺陷「死亡态+换图丢本地玩家实体」：#3044（master 904a82f0）把换图/重建路径上
# 6 处捕获式 `commands.entity(e).insert/remove` 换成**落地时复查**的 safe_*，封掉了根源路径；
# 这里在**端到端**再钉一层——2026-09-24 复核实测三种序列（死→换图 / 死→换图→复活 / 复活→再换图）
# 都不再复现：`combat_probe.players=1`、`state` 全程可读、revive 后位置几秒内收敛到服务端权威位置。
Rpc 'chat' @{ message = "@mapmove $DieMap2 $DieX2 $DieY2" } | Out-Null
$dead2 = $null
foreach ($i in 1..20) {
    Start-Sleep 1
    $dead2 = Rpc 'state'
    if ("$($dead2.map)" -eq "$DieMap2") { break }
}
$cp2 = Rpc 'combat_probe'
$okB2 = ($null -ne $dead2 -and $null -ne $dead2.tile_x -and [bool]$dead2.dead -and ($cp2.players -ge 1))
Write-Host ("[B2] 死亡态换图到 {0}：map={1}（期望 {2}）tile=({3},{4}) dead={5} players={6} → {7}" -f `
    $DieMap2, $dead2.map, $DieMap2, $dead2.tile_x, $dead2.tile_y, $dead2.dead, $cp2.players, `
    $(if ($okB2) { 'PASS' } else { 'FAIL（本地玩家实体丢失或状态不可读）' }))
$dieMap = "$($dead2.map)"

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
Write-Host ("VERDICT alive_before={0} died={1} dead_mapswitch_entity_kept={2} revived={3} back_to_bind_map_and_spot={4}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okB2) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okB2 -and $okC -and $okD)) { exit 5 }

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5j_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
