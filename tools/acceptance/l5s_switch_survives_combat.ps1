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
    # 默认去 SerpentValley(map 文件名 '2')；脚本会自证「换图后 map 变了」，没变直接 exit 3。
    [string]$ToMap = '2',
    [int]$ToX = 500,
    [int]$ToY = 485
)
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
$probe0 = Rpc 'combat_probe'
if ($null -eq $probe0 -or $null -eq $probe0.applied) {
    Write-Host 'FAIL(9): combat_probe.applied 不可读——无法证明战斗事件真的到达（拒绝空转）'
    exit 9
}
$struckBase = [int]$probe0.applied.struck + [int]$probe0.applied.player_struck

# B) 制造战斗事件：拉 6 只 Deer 到身边，攻击一次（Struck 事件由服务端回）
Rpc 'chat' @{ message = '@recallmob Deer 6' } | Out-Null
Start-Sleep -Seconds 2
Rpc 'attack_mode' @{ mode = 1 } | Out-Null
Rpc 'click' @{ x = 512; y = 400 } | Out-Null
Start-Sleep -Seconds 2
$near = Rpc 'nearby' @{ radius = 12 }
Write-Host ("[B] 身边实体数={0}（含 Deer 用于产生 Struck）" -f (@($near.entities) | Measure-Object).Count)

# B2) 前置断言：战斗事件必须**真的应用过**（applied 增量 > 0），否则本夹具证明不了
#     「换图同帧仍有 Struck 排队」这条路径 → 前置不成立，exit 3（不产出 PASS）。
$probe1 = Rpc 'combat_probe'
$struckAfter = if ($null -ne $probe1.applied) { [int]$probe1.applied.struck + [int]$probe1.applied.player_struck } else { $struckBase }
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
