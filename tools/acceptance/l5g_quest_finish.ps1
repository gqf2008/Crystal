# l5g_quest_finish.ps1 — ④ 任务闭环（交付段）：接取 → 到交付 NPC 处 → FinishQuest → 奖励到账
#
# 选任务 27（Errands）的理由（都能对着数据核）：
#   * Envir/Quests/27.txt 的目标段为空（无 KillTasks/ItemTasks/CarryItems/FlagTasks），
#     而服务端 `QuestLog::is_progress_complete()` 是 `progress.iter().all(current>=target)`
#     —— 空进度**恒真**，所以它是"走到 Scout 面前即可交付"的任务。
#   * 奖励明确：[@GoldReward] 1200（判据用金币 delta，比 exp 更直观）。
#   * 交付 NPC 由 NPC 脚本 [QUESTS] 段决定（`-27` = 交付）：DB npc_scripts 里 npc_index=52
#     的 [QUESTS] = ["-27","35","-37"] → npc_infos idx=52 = MongchonScout_Brian，
#     map_infos idx=14 = '0100'(Kitchen) @ (4,10)。
#
# 判据（取状态）：A) 接取前 journal 无 27 → B) accept_quest 后 taken 含 27
#               C) 到交付 NPC 处 finish_quest → taken 不再含 27（服务端 CompleteQuest 生效）
#               D) 奖励：gold +1200（bag_probe.gold delta）
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$QuestId = 27,
    # 交付点默认**从数据推导**（不手填）：NPC 脚本 [QUESTS] 段里 `-<quest>` 即"交付"，
    # 由 npc_index → npc_infos(地图/坐标) → map_infos(地图名) 三步换算。
    # 手填过一次，结果填到了「接取 NPC」上，服务端正确地回了「请到对应 NPC 处交付任务」。
    [string]$FinishMap = '',
    [int]$FinishX = -1,
    [int]$FinishY = -1,
    # 奖励判据：不写死数值——**任务实例自身带奖励快照**（DB `quests` 行里就存着 exp/gold），
    # 实测 quest 27 的实例是 1000/500，而它的任务文件 Envir/Quests/27.txt 写的是 1104/1200，
    # 服务端按**实例**发放。写死文件值会得到假 FAIL，所以判据取「奖励真的到账」= 金币与经验同时为正。
    [int]$MinGold = 1,
    [int]$MinExp = 1
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
function Taken { @((Rpc 'quest_probe').taken | ForEach-Object { [int]$_.id }) }
function Gold { [int](Rpc 'bag_probe').gold }
function Db([string]$sql) { (& python "$wt\tools\acceptance\dbq.py" $sql) }
function FinishNpcFor([int]$qid) {
    foreach ($row in Db "select npc_index, lines_json from npc_scripts where page_name='[QUESTS]'") {
        if ($row -notmatch '^\((\d+), ''(.+)''\)$') { continue }
        $npcIndex = [int]$Matches[1]
        $lines = $Matches[2] -replace "''", "'"
        $arr = $lines | ConvertFrom-Json
        if ($arr -contains "-$qid") {
            $info = Db "select map_index, name, x, y from npc_infos where idx=$npcIndex" | Select-Object -First 1
            if ($info -notmatch '^\((\d+), ''(.+)'', (\d+), (\d+)\)$') { continue }
            $mapIdx = [int]$Matches[1]; $npcName = $Matches[2]; $nx = [int]$Matches[3]; $ny = [int]$Matches[4]
            # dbq.py 单列输出裸值（如 0106）；老版本输出元组 ('0106',)，两种都吃
            $mapName = (Db "select file_name from map_infos where idx=$mapIdx" | Select-Object -First 1)
            if ($mapName -match "^\( '(.*)',? \)$") { $mapName = $Matches[1] }
            $mapName = "$mapName".Trim()
            return [pscustomobject]@{ npc_index = $npcIndex; name = $npcName; map = $mapName; x = $nx; y = $ny }
        }
    }
    return $null
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }

if ($FinishMap -eq '') {
    $fin = FinishNpcFor $QuestId
    if (-not $fin) { Write-Host ("FAIL: 数据里找不到任务 {0} 的交付 NPC（[QUESTS] 段无 -{0}）" -f $QuestId); exit 4 }
    $FinishMap = $fin.map; $FinishX = $fin.x; $FinishY = $fin.y
    Write-Host ("[0] 交付点推导: quest {0} → NPC {1}(idx {2}) @ {3} ({4},{5})" -f $QuestId, $fin.name, $fin.npc_index, $fin.map, $fin.x, $fin.y)
}
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$wt\Client-Bevy" `
    -RedirectStandardOut "$acc\l5g_client.log" -RedirectStandardError "$acc\l5g_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)

$taken0 = Taken
Write-Host ("[A] 当前已接任务: {0}" -f ($taken0 -join ','))
if ($taken0 -contains $QuestId) {
    Write-Host ("[A] 任务 {0} 已在日志里（上一轮接过），跳过接取，直接走交付" -f $QuestId)
    $accepted = $true
} else {
    # 接取：任务 27 的接取 NPC 由 [QUESTS] 正数项决定（npc 52 的 [QUESTS] 含 -27，接取在别的 NPC/同 NPC 的 @MAIN）
    Rpc 'accept_quest' @{ npc_index = 0; quest_index = $QuestId } | Out-Null
    Start-Sleep -Seconds 2
    $taken1 = Taken
    Write-Host ("[A] accept_quest 后: {0}" -f ($taken1 -join ','))
    $accepted = $taken1 -contains $QuestId
}

$gold0 = Gold
$lv0 = (Rpc 'bag_probe').level; $exp0 = (Rpc 'bag_probe').exp
Write-Host ("[B] 交付前 gold={0} level={1} exp={2}" -f $gold0, $lv0, $exp0)

# 走到交付 NPC
Rpc 'chat' @{ message = "@mapmove $FinishMap $FinishX $FinishY" } | Out-Null
Start-Sleep 4
$st2 = Rpc 'state'
Write-Host ("[B] @mapmove {0} {1} {2} -> tile=({3},{4})" -f $FinishMap, $FinishX, $FinishY, $st2.tile_x, $st2.tile_y)
$npc = (Rpc 'nearby' @{ radius = 3000 }).entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
if ($npc) { Write-Host ("[B] 交付 NPC={0} id={1} dist={2}" -f $npc.name, $npc.object_id, $npc.dist) } else { Write-Host '[B] WARN: 附近没有 NPC' }
if ($npc) { Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null; Start-Sleep 2 }
$rows = Rpc 'npc_rows'
Write-Host ("[B] NPC 窗可见={0} 行数={1} 链接={2}" -f $rows.visible, ($rows.lines | Measure-Object).Count, (($rows.links | ForEach-Object { $_.key }) -join ','))

# 交付
Rpc 'finish_quest' @{ quest_index = $QuestId; selected_item_index = -1 } | Out-Null
Start-Sleep -Seconds 3
$taken2 = Taken
$gold1 = Gold
$p = Rpc 'bag_probe'
$goldDelta = $gold1 - $gold0
$expDelta = [int]$p.exp - [int]$exp0
$removed = -not ($taken2 -contains $QuestId)
Write-Host ("[C] finish_quest 后 已接={0}（任务 {1} 已移除={2}）" -f ($taken2 -join ','), $QuestId, $removed)
Write-Host ("[D] 奖励: gold {0}->{1} (delta={2}, 期望 >= {3})；exp {4}->{5} (delta={6}, 期望 >= {7})；level {8}->{9}" -f `
    $gold0, $gold1, $goldDelta, $MinGold, $exp0, $p.exp, $expDelta, $MinExp, $lv0, $p.level)

$okA = [bool]$accepted
$okC = [bool]$removed
$okD = (($goldDelta -ge $MinGold) -and ($expDelta -ge $MinExp))
Write-Host ("VERDICT accept={0} state_flip={1} reward_gold={2}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okC -and $okD)) { exit 5 }
