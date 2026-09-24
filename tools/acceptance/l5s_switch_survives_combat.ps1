# l5s_switch_survives_combat.ps1 — 换图不再打爆客户端（#3089：StruckTimer 插入打到失效实体）
#
# 复现/判据（全部取**进程与状态真值**，不看像素）：
#   A) 进场成功（`state` 有 tile）
#   B) 同图先制造战斗事件：拉起若干怪（`@recallmob`）并攻击一次，让 ObjectStruck/
#      PlayerStruck 事件在**换图同帧**仍可能排队（真实故障形态）。
#      **前置断言（防空转）**：必须以 `combat_probe.applied.{struck,player_struck}`
#      相对基线的**增量**证明事件真的到达并被应用；没增量 = 前置不成立 = exit 3，
#      绝不产出 PASS（否则"没打到怪"也会假绿）。
#   C) `@mapmove` 切到另一张图（地图重建会 despawn 并以新 generation 重建对象实体）
#   D) 判据：切图后 ① 客户端进程仍存活；② `state` RPC 仍能读到 tile（进程没卡死/没退出）
#
# 阳性对照（**实机原始证据**）：修复前同一套操作下客户端 panic 并退出，日志为
#   `insert<client_bevy::game::combat::StruckTimer> ... Entity despawned ...`
#   → `Encountered a panic when applying buffers for system client_bevy::game::combat::apply_combat_events`
#   → `Encountered a panic in system bevy_app::main_schedule::Main::run_main`（进程退出、9000 拒连）。
#   夹具在那种状态下必然 FAIL（D 两条都不成立）。
#
# 退出码：0 = PASS；10 = 换图后客户端未存活/状态不可读；9 = 前置未就绪；
#         3 = 前置未成立（同图/战斗事件未到达 → 本轮不产出 PASS）
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    # 目标图必须**与进场图不同**（同图移动不触发地图重建 → 复现不出崩溃 → 夹具会假绿）。
    # 留空 = **自动挑一张与进场图不同的图**（角色存档是持久的：上一次跑完会把角色留在目标图上，
    # 写死 '2' 的话下一次进场就在 '2' 上 → 同图 → 夹具永远 exit 3，这是夹具自身的腐烂）。
    # 无论自动还是显式，脚本都会自证「换图后 map 变了」，没变直接 exit 3。
    [string]$ToMap = '',
    [int]$ToX = 0,
    [int]$ToY = 0
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5s_switch_survives_combat' -TimeoutSec 1800)) { exit 2 }
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
$err = "$PSScriptRoot\l5s_client.err.log"

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
    } catch { return $null }
}

Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
if (Test-Path $err) { Remove-Item $err -Force }
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$PSScriptRoot\l5s_client.log" -RedirectStandardError $err | Out-Null

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[A] 进场 map={0} tile=({1},{2}) PASS" -f $st.map, $st.tile_x, $st.tile_y)
$fromMap = $st.map

# 目标图：留空则自动挑一张与进场图不同的（'0' BichonProvince ↔ '2' SerpentValley，
# 两个落点都是既有夹具（l5a/l5i）验证过可站的坐标）。
if (-not $ToMap) {
    if ("$fromMap" -eq '0') { $ToMap = '2'; $ToX = 500; $ToY = 485 }
    else { $ToMap = '0'; $ToX = 287; $ToY = 615 }
    Write-Host ("[A] 自动选目标图: {0} ({1},{2})" -f $ToMap, $ToX, $ToY)
}
$probe0 = Rpc 'combat_probe'
if ($null -eq $probe0 -or $null -eq $probe0.applied) {
    Write-Host 'FAIL(9): combat_probe.applied 不可读——无法证明战斗事件真的到达（拒绝空转）'
    exit 9
}
$struckBase = [int]$probe0.applied.struck + [int]$probe0.applied.player_struck

# B) 制造战斗事件：**必须真的挥砍到怪**（Struck 事件由服务端回）。
#    两条 l5a 实测教训照抄（别改回去）：
#      ① 不要自己 walk_to 追怪——`auto_attack` 自带 #1817 追击，而 control 的 `attack`
#         会 `remove::<LocalMove>()`，在走位循环里反复 attack 会把刚起步的走位取消；
#      ② 只在「本地位置连续两拍不变」时挥砍——本地移动是预测的、服务端位置滞后，
#         近战由服务端按它自己视角的「正前方一格」结算，边追边打全是空挥（实测 56 次 attack 零伤害）。
#    首版这里用的是「@recallmob + attack_mode + click 屏幕点」：实测 applied 增量恒为 0
#    （Deer 拉过来了但一次都没结算），所以 B2 前置断言当场判 FAIL(3)——那次假绿就是这么来的。
$target = $null
foreach ($i in 1..10) {
    Start-Sleep 1
    $mons = @((Rpc 'nearby' @{ radius = 5000 }).entities | Where-Object { $_.kind -eq 'monster' })
    $target = $mons | Select-Object -First 1
    if ($null -ne $target) { break }
    # 附近没怪才补拉一批（Deer 在多数地图刷新）
    Rpc 'chat' @{ message = '@recallmob Deer 6' } | Out-Null
}
if ($null -eq $target) {
    Write-Host 'FAIL(3): 5000px 内找不到任何怪——前置不成立（换图后「没崩」证明不了本夹具要证的路径）'
    exit 3
}
Write-Host ("[B] 目标怪: {0} id={1} dist={2}" -f $target.name, $target.object_id, $target.dist)
Rpc 'attack' @{ object_id = $target.object_id } | Out-Null

# B2) 前置断言：战斗事件必须**真的应用过**（applied 增量 > 0），否则本夹具证明不了
#     「换图同帧仍有 Struck 排队」这条路径 → 前置不成立，exit 3（不产出 PASS）。
$deadline = (Get-Date).AddSeconds(25)
$lastTile = $null; $stable = 0; $struckAfter = $struckBase; $probe1 = $null
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
    $stNow = Rpc 'state'
    $tileKey = "$($stNow.tile_x),$($stNow.tile_y)"
    if ($tileKey -eq $lastTile) { $stable++ } else { $stable = 0; $lastTile = $tileKey }
    $probe1 = Rpc 'combat_probe'
    if ($null -ne $probe1 -and $null -ne $probe1.applied) {
        $struckAfter = [int]$probe1.applied.struck + [int]$probe1.applied.player_struck
    }
    if ($struckAfter -gt $struckBase) { break }
    if ($stable -ge 2 -and $null -ne $probe1.target_dist_tiles -and [int]$probe1.target_dist_tiles -le 1) {
        Rpc 'attack' @{ object_id = $target.object_id } | Out-Null
    }
}
$delta = $struckAfter - $struckBase
Write-Host ("[B] combat_probe.applied struck+player_struck 基线={0} 攻击后={1} 增量={2}" -f $struckBase, $struckAfter, $delta)
if ($delta -le 0) {
    Write-Host 'FAIL(3): 战斗事件未到达（applied 增量=0）——前置不成立，本轮不产出 PASS'
    exit 3
}

# C) 换图（地图重建）
Rpc 'chat' @{ message = "@mapmove $ToMap $ToX $ToY" } | Out-Null
Start-Sleep -Seconds 6

# D) 判据：进程活着 + 状态可读
$alive = [bool](Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue)
$st2 = Rpc 'state'
$stateOk = ($null -ne $st2 -and $null -ne $st2.tile_x)
# 自证：必须真的换了图（否则本夹具证明不了"换图重建"这条路径，属假绿）
if ($stateOk -and "$fromMap" -eq "$($st2.map)") {
    Write-Host ("FAIL(J0): 目标图与进场图相同（map={0}）——本夹具必须在**真换图**上跑（用 -ToMap 指定另一张图）" -f $st2.map)
    exit 3
}
# 自证 2：客户端日志里必须出现**换图重建**（MapChanged + 清理旧世界并重建），
# 只有坐标动了但没重建 = 没走到 despawn 路径 = 前置不成立（不产出 PASS）。
$log = "$PSScriptRoot\l5s_client.log"
$mapChanged = @(Select-String -Path $log, $err -Pattern ("MapChanged: " + [regex]::Escape("$ToMap")) -EA SilentlyContinue).Count
$rebuilt = @(Select-String -Path $log, $err -Pattern ("检测到换图 " + [regex]::Escape("$ToMap") + "，清理旧世界并重建") -EA SilentlyContinue).Count
Write-Host ("[C] 日志自证：MapChanged 命中={0}，清理旧世界并重建命中={1}" -f $mapChanged, $rebuilt)
if ($stateOk -and ($mapChanged -eq 0 -or $rebuilt -eq 0)) {
    Write-Host 'FAIL(3): 日志里没有换图重建证据（未走到 despawn 路径）——前置不成立，本轮不产出 PASS'
    exit 3
}
Write-Host ("[D] 换图后进程存活={0}；state 可读={1}（map={2} tile=({3},{4})）" -f `
    $alive, $stateOk, $st2.map, $st2.tile_x, $st2.tile_y)

$panic = @(Select-String -Path $err -Pattern 'panic|Entity despawned|apply_combat_events' -EA SilentlyContinue).Count
Write-Host ("[D] 日志里的 panic/失效实体行数={0}" -f $panic)

if ($alive -and $stateOk -and $panic -eq 0) {
    Write-Host '=== 全部 PASS ==='
    exit 0
}
Write-Host '=== 有 FAIL ==='
exit 10
