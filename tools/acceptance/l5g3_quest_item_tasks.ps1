# l5g3_quest_item_tasks.ps1 — ④ 任务闭环（**ItemTasks 类型**，端到端）：
#                              接取 → 打对应怪掉「任务物品」进任务格 → 交付 → 奖励
#
# 为什么单开一条：l5g 只覆盖「目标段为空」的任务、l5g2 覆盖 `CarryItems`（接取时服务端发携带物）。
# ItemTasks 要**真的把物品弄进任务格**——链路口径（本轮已核实）：
#   Drops/*.txt 的 `1/6 ItemName Q` = 任务物品掉落（C# `MonsterInfo.cs:359-366`）
#   → 击杀时不落地，直接进击杀者**任务格**并推进进度
#   （`world/mod.rs:5417 try_give_quest_item` → `player.rs:6074 TryQuestItemPickup`）。
#   注意：**物品进的是任务格而不是背包**，所以普通背包物品不算数（本夹具的判据读 `quest_occupied`）。
#
# 选任务（可用候选，按「期望击杀数 = 需求数 ÷ 掉率」排序，2026-09-24 扫文件+DB 得出）：
#   47  OliviasRing x1 ← CaveMaggot      p=0.1   @ D011（50 只刷）   期望 10 杀
#   28  Web         x10← SpittingSpider  p=0.333 @ 2/0（各 90 只刷） 期望 30 杀
#   23  OmaTeeth    x10← Oma/Oma0         p=0.333 @ 0（210 只刷）    期望 30 杀
#   26  PoisonSack  x10← SpittingSpider  p=0.333 @ 2/0（各 90 只刷） 期望 30 杀
# 排除掉的候选（首跑实测踩到，写下来免得别人再试）：quest 76（RedDagger ← HiGreatGhoul p=1.0，
#   看着"期望 1 杀"最香）——但那只怪在 D422 上**实际不刷**（DB 里那条 map_respawns 是 boss 级刷新，
#   进场后 nearby 6000px 内 125 只全是 Zombie*），所以候选要按"刷得多"而不是"掉率高"挑。
#   补充（2026-09-24）：洞穴图（D011 / map 12）里 `walk_to` 会持续回「无可行路径」，玩家被钉在
#   落点原地、只有怪自己靠过来，于是"自己走过去打"这条与掉落无关的环节会吃掉整个时间盒。
#   因此候选优先挑**开阔图**：quest 45（RedSnakeTeeth x1 ← RedSnake p=0.1 @ map '2' x355，
#   接取 NPC Merchant_Sandford @ map '5'、交付 NPC Merchant_Robert @ map '2'，+/- 链接齐全）。
#
# 判据（取状态）：
#   A) 接取后 quest_probe.taken 含该任务
#   B) 打到「任务格里出现该任务物品且数量 ≥ 需求」（`bag_probe.quest_occupied`）——
#      这一步同时证明「Q 掉落 → 任务格」这条链路通了（背包里凑数是**不算**的）
#   C) 到交付 NPC 处 finish_quest → taken 不再含它
#   D) 奖励 gold 与 exp delta 同时为正
#
# 仪器口径（重要）：找不到／走不到目标怪时，夹具用 GM `@recallmob <怪名> 4` 把**同种怪**召到脚边
#   再打。这不是旁路——`spawn_monster_named` 建的是同一种 MonsterState、进同一张 monsters 表，
#   死亡时走同一条「掉落表 → try_give_quest_item → 任务格」链路；变的只是"必须自己寻路过去"这一
#   与掉落无关的环节。若这条链路本身没通，本夹具仍然红。
#
# ==== 2026-09-24 实测状态（如实记录，别把它当"全绿"）====
#   ④ ItemTasks 端到端**尚未拿到证据**，只到 [A]：
#     · [A] 接取 PASS（quest 30：`taken 1,28,46,48,76,90,142 -> …142,30`；accept_quest 返回
#       `{"npc_index":135,"ok":true,"quest_index":30}`）。
#     · [B] 打怪掉任务物品：**0 只**。卡点不是掉落链路，是**攻击打不中**——一次 run 实测
#       626 个 `attack` 包、服务端事件流里 0 个 struck/damage/died。
#     · 卡点已定性（服务端 debug 日志实证）：玩家**客户端位置比服务端多 1 格**。
#       证据一（召唤场景）：服务端 `Attack bevychar at (103,417) dir=0 target=(103,416)` +
#         `Attack nearby: …Currish#2102@(103,417)…`（四只召唤怪在服务端玩家格上），
#         而客户端 `state` 读到的 tile 是 (102,417)。近战由服务端按「服务端玩家格 + 方向」
#         结算，方向又是客户端用**自己**的格差算的 ⇒ 原点差一格 = 永远打空。
#       证据二（无怪干扰的纯位移 A/B）：客户端 `state` = (453,117)/(454,117)，而服务端
#         `Player bevychar moved … to (452,117)` 之后再无 moved 行。
#     · 机制假设（下一步要验的）：客户端 `LocalMove` 在**静置 >700ms 后第一步仍发 Run**，
#       而服务端 `can_run()`（C# HumanObject.CanRun：`_stepCounter>0 || FastRun`）会拒收这一脚，
#       客户端却按预测把这一步算进去了；`UserLocation` 校正又在 `LocalMove` 活动期间被忽略
#       （`movement.rs` 的注释），路径走完也没有再校正 ⇒ 漂移**不会自愈**。
#     · 所以本夹具当前口径：**不许写 PASS**，[B]/[C]/[D] 如实记 GAP，等漂移修掉再复跑。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [int]$QuestId = 0,
    [int]$KillCap = 60,
    [int]$HarvestTimeoutSec = 900
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
# 判据脚本与客户端构建都必须取自**本脚本所在的 worktree**：写死别的 worktree 会静默跑旧构建、
# 给出假红/假绿（本轮前几版就硬编码过 Crystal-wt-p3）。
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$acc = Join-Path $wt 'tools\acceptance'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
$questDir = 'E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Daneo1989\Envir\Quests'

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Taken { @((Rpc 'quest_probe').taken | ForEach-Object { [int]$_.id }) }
function QuestCount([string]$name) {
    $b = Rpc 'bag_probe'
    $sum = 0
    foreach ($o in @($b.quest_occupied)) { if ($o.name -eq $name) { $sum += [int]$o.count } }
    return $sum
}
function Db([string]$sql) { (& python "$wt\tools\acceptance\dbq.py" $sql) }

function NpcLinkFor([int]$qid, [bool]$wantFinish) {
    foreach ($row in Db "select npc_index, lines_json from npc_scripts where page_name='[QUESTS]'") {
        if ($row -notmatch '^\((\d+), ''(.+)''\)$') { continue }
        $npcIndex = [int]$Matches[1]
        $lines = $Matches[2] -replace "''", "'"
        $arr = $lines | ConvertFrom-Json
        $hit = $arr | Where-Object { ([int]$_) -eq $(if ($wantFinish) { -$qid } else { $qid }) }
        if ($hit) {
            $info = Db "select map_index, name, x, y from npc_infos where idx=$npcIndex" | Select-Object -First 1
            if ($info -notmatch '^\((\d+), ''(.+)'', (\d+), (\d+)\)$') { continue }
            $mapIdx = [int]$Matches[1]; $npcName = $Matches[2]; $nx = [int]$Matches[3]; $ny = [int]$Matches[4]
            $mapName = (Db "select file_name from map_infos where idx=$mapIdx" | Select-Object -First 1)
            if ($mapName -match "^\( '(.*)',? \)$") { $mapName = $Matches[1] }
            return [pscustomobject]@{ npc_index = $npcIndex; name = $npcName; map = "$mapName".Trim(); x = $nx; y = $ny }
        }
    }
    return $null
}

# 任务文件的 ItemTasks → @( @{ name; need } )
function QuestItemTasks([int]$qid) {
    $f = Join-Path $questDir "$qid.txt"
    if (-not (Test-Path $f)) { return @() }
    $sec = $null; $out = @()
    foreach ($ln in (Get-Content $f)) {
        if ($ln -match '^\[@(.+)\]') { $sec = $Matches[1]; continue }
        if ($sec -eq 'ItemTasks' -and $ln.Trim()) {
            $p = $ln.Trim() -split '\s+'
            $out += [pscustomobject]@{ name = $p[0]; need = $(if ($p.Count -gt 1) { [int]$p[1] } else { 1 }) }
        }
    }
    return $out
}

# 任务的等级/前置/职业门槛（quest_infos）。**不为空就得先判**：
# 不满足时服务端会收下 accept_quest 但不入 taken——夹具侧不判就会白跑一整轮（本轮 q45 实测）。
function QuestReq([int]$qid) {
    $row = (Db "select required_min_level, required_max_level, required_quest, required_class from quest_infos where idx=$qid" | Select-Object -First 1)
    if ("$row" -notmatch '^\((\d+), (\d+), (\d+), (\d+)\)$') { return $null }
    return [pscustomobject]@{
        min = [int]$Matches[1]; max = [int]$Matches[2]
        req = [int]$Matches[3]; cls = [int]$Matches[4]
    }
}

# Q 掉落：物品名 → (怪名, 掉率, 最佳刷图)
function QDropFor([string]$itemName) {
    $row = Db "select mi.name, d.chance from monster_drops d join monster_infos mi on mi.idx=d.monster_index join item_infos ii on ii.idx=d.item_index where d.quest_required=1 and lower(ii.name)=lower('$itemName') order by d.chance desc limit 1" | Select-Object -First 1
    # dbq.py 多列输出是 Python 元组 repr：('HiGreatGhoul', 1.0)（**单引号**、无多余空格）。
    # 注意在 PowerShell 里用单引号字符串写这个正则，'' 表示一个单引号——别用双引号字符串加空格。
    if ("$row" -notmatch '^\(\s*''(.+)'',\s*([\d.eE+-]+)\s*\)$') { return $null }
    $mon = $Matches[1]; $chance = [double]$Matches[2]
    $m = Db "select mi2.file_name from map_respawns mr join monster_infos mi on mi.idx=mr.monster_index join map_infos mi2 on mi2.idx=mr.map_index where mi.name='$mon' group by mi2.file_name order by sum(mr.count) desc limit 1" | Select-Object -First 1
    if ("$m" -match '^\(\s*''(.*)''\s*,?\s*\)$') { $m = $Matches[1] }
    $m = "$m".Trim()
    # 该怪在这张图的**刷新点坐标**：用它当落点——刷新点必然是可走格，
    # 而 GM 传送到任意坐标（如 (300,300)）可能落在墙里，客户端寻路会一直回「无可行路径」。
    $sp = Db "select mr.x, mr.y from map_respawns mr join monster_infos mi on mi.idx=mr.monster_index join map_infos mi2 on mi2.idx=mr.map_index where mi.name='$mon' and mi2.file_name='$m' order by mr.count desc limit 1" | Select-Object -First 1
    $sx = 0; $sy = 0
    if ("$sp" -match '^\((\d+), (\d+)\)$') { $sx = [int]$Matches[1]; $sy = [int]$Matches[2] }
    return [pscustomobject]@{ monster = $mon; chance = $chance; map = $m; x = $sx; y = $sy }
}

function Monsters($probe) { @($probe.entities | Where-Object { $_.kind -eq 'monster' }) }

# 传送后**必须校验落点可走**——`map_respawns` 的坐标可能是封闭口袋（实测 map '2' 的 Currish 刷点
# (400,200)：玩家落在那儿之后 walk_to 四个邻格全报「无可行路径」，玩家一格都动不了，
# 于是"贴上去打"必然全废、`combat_probe.target_dist_tiles` 永远停在 2-3）。
# 判据用「真走一步」而不是"问地图"：四个邻格轮流 walk_to，位置变了才算可走。
function EnsureMobile([string]$mapName, [int]$tries = 6) {
    foreach ($t in 1..$tries) {
        $st = Rpc 'state'
        $tx = [int]$st.tile_x; $ty = [int]$st.tile_y
        foreach ($off in @(@(1, 0), @(-1, 0), @(0, 1), @(0, -1))) {
            $nx = $tx + $off[0]; $ny = $ty + $off[1]
            Rpc 'walk_to' @{ x = [double]($nx * 48 + 24); y = [double](-($ny * 32 + 32)) } | Out-Null
            Start-Sleep 2
            $st2 = Rpc 'state'
            if ("$($st2.tile_x),$($st2.tile_y)" -ne "$tx,$ty") { return $true }
        }
        Write-Host ("      · 落点 ({0},{1}) 不可走 → 换随机落点重试" -f $tx, $ty)
        Rpc 'chat' @{ message = "@mapmove $mapName" } | Out-Null
        Start-Sleep 3
    }
    return $false
}

# 一批同名怪（按距离升序，最多 $max 个）。**为什么要一批**：同一只怪可能卡在墙/围栏里，
# 玩家寻路到不了它相邻格（实测 `combat_probe.target_dist_tiles` 恒停在 2-3、玩家一格都不动），
# 这时要能换下一只或改成召唤，而不是抱着够不着的目标空耗整轮。
function FindNamedList([string]$monsterName, [int]$max = 3) {
    foreach ($i in 1..10) {
        Start-Sleep 1
        $m = @(Monsters (Rpc 'nearby' @{ radius = 6000 }) | Where-Object { $_.name -eq $monsterName } | Sort-Object dist | Select-Object -First $max)
        if ($m.Count -gt 0) { return $m }
    }
    return @()
}

# 锁一只怪并打到死。**贴上去这件事交给客户端自己的 auto_attack_system（#1817 追击）**——
# l5a 的两条实机教训同样适用，第一版之所以一只都打不死就是因为违反了它们：
#   1) 不要自己 `walk_to`：control 的 `Attack` 会 `remove::<LocalMove>()`，在走位循环里反复发
#      `attack` 会把刚起步的走位取消；
#   2) 只在**本地位置稳定**（连续两次采样 tile 不变）且 `target_dist_tiles ≤ 1` 时挥砍：
#      近战由服务端按它自己视角的「正前方一格」结算，边追边打全是空挥。
# 判据是 `combat_probe.events` 里 id == 目标的 `died`（服务端事件流），不是"血条看起来掉了"。
function KillOne($mon, [int]$timeoutSec = 25) {
    if (-not $mon) { return $false }
    $id = [uint32]$mon.object_id
    Rpc 'attack' @{ object_id = $id } | Out-Null
    $deadline = (Get-Date).AddSeconds($timeoutSec)
    $lastTile = $null; $stable = 0; $minDist = $null
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $stNow = Rpc 'state'
        $tileKey = "$($stNow.tile_x),$($stNow.tile_y)"
        if ($tileKey -eq $lastTile) { $stable++ } else { $stable = 0; $lastTile = $tileKey }
        $cp = Rpc 'combat_probe'
        foreach ($e in @($cp.events)) { if ($e.kind -eq 'died' -and $e.id -eq $id) { return $true } }
        if ($null -eq $cp.attack_target) { return $true }   # 目标已从场景消失
        if ($null -ne $cp.target_dist_tiles) {
            $d = [int]$cp.target_dist_tiles
            if ($null -eq $minDist -or $d -lt $minDist) { $minDist = $d }
        }
        if ($stable -ge 2 -and $null -ne $cp.target_dist_tiles -and [int]$cp.target_dist_tiles -le 1) {
            Rpc 'attack' @{ object_id = $id } | Out-Null
        }
    }
    Write-Host ("      · 目标 {0} 超时（最近贴到 {1} 格）→ 换下一只/改召唤" -f $id, $minDist)
    return $false
}

function KillNamed([string]$monsterName) {
    foreach ($m in (FindNamedList $monsterName 3)) { if (KillOne $m) { return $true } }
    # 自然刷的都够不着（墙/围栏）→ 把同种怪召到**脚边**再打（`@recallmob` → spawn_monster_named，
    # 登进同一张 monsters 表、死亡走同一条掉落链路）。它只去掉"必须自己寻路过去"这个与掉落无关的
    # 环节；掉落/任务格链路本身若不通，本夹具仍然红。
    # 关键一步：**召完要自己走开一格**。
    #   ① `@recallmob <名> <数量>`（不带坐标）实测可用；带坐标的形式实测**不生效**
    #      （发 `@recallmob Currish 2 19272 -6432` 后 nearby 里没有任何新卡在该点）。
    #   ② 不带坐标时怪就叠在玩家**同一格**上，而近战方向由「玩家格 → 目标格」差值算，
    #      差值 (0,0) → 方向退化成 Up → 服务端按"正前方一格"结算 ⇒ 四只同格怪贴到 0 格也 20s 打不死。
    #   ③ 走开一格后，怪（spawn 锚在原格、AI 不追）与玩家正好差 1 格、方向明确 → 正常挥砍。
    Write-Host ("      · 附近 {0} 只都够不着 → @recallmob {0} 4（脚边）后走开一格" -f $monsterName)
    Rpc 'chat' @{ message = "@recallmob $monsterName 4" } | Out-Null
    Start-Sleep 3
    $st0 = Rpc 'state'
    foreach ($off in @(@(-48, 0), @(48, 0), @(0, -32), @(0, 32))) {
        Rpc 'walk_to' @{ x = [double]([int]$st0.x + $off[0]); y = [double]([int]$st0.y + $off[1]) } | Out-Null
        Start-Sleep 2
        $st1 = Rpc 'state'
        if ("$($st1.tile_x),$($st1.tile_y)" -ne "$($st0.tile_x),$($st0.tile_y)") {
            Write-Host ("      · 已走到 ({0},{1})，与脚边怪拉开 1 格" -f $st1.tile_x, $st1.tile_y)
            break
        }
    }
    foreach ($m in (FindNamedList $monsterName 4)) { if (KillOne $m 20) { return $true } }
    return $false
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }

Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5g3_client.log" -RedirectStandardError "$acc\l5g3_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

$charName = (Db "select name from characters where account_username='$User'" | Select-Object -First 1)
if ("$charName" -match "^\( '(.*)',? \)$") { $charName = $Matches[1] }
$charName = "$charName".Trim()
$completed = @{}
foreach ($r in (Db "select quest_index from completed_quests where character_name='$charName'")) {
    if ("$r" -match '(\d+)') { $completed[[int]$Matches[1]] = $true }
}

# 顺序 = 期望击杀数升序、开阔图优先（2026-09-24 用「+/- 链接齐 + 等级/前置满足」筛过一遍）。
# 每条都留着，跳过原因会被打印（缺 [QUESTS] 链接 / 等级不足 / 前置任务没做）。
$Pool = @(30, 28, 52, 90, 76, 55, 152, 48, 45, 104, 91, 47, 23, 26, 38)
if ($QuestId -eq 0) {
    $live = @{}
    foreach ($e in (Rpc 'quest_probe').taken) { $live[[int]$e.id] = $true }
    $lv = [int](Rpc 'bag_probe').level
    foreach ($q in $Pool) {
        if ($live.ContainsKey($q) -or $completed.ContainsKey($q)) { continue }
        # 必须**同时**有接取与交付 NPC 才能被夹具驱动（quest 47 就是没链接、首跑被刷掉）
        if (-not (NpcLinkFor $q $false) -or -not (NpcLinkFor $q $true)) {
            Write-Host ("[0] 跳过 quest {0}：数据里没有 +{0}/-{0} 的 [QUESTS] 链接" -f $q)
            continue
        }
        # 等级/前置：不满足时**服务端会收下 accept_quest 但不入 taken**（本轮 q45 实测：
        # accept 返回 ok=true 而 taken 不变，白跑一整轮）——所以在夹具侧先按 quest_infos 判掉。
        $rq = QuestReq $q
        if ($rq) {
            if ($rq.min -and $lv -lt $rq.min) {
                Write-Host ("[0] 跳过 quest {0}：等级不足（{1} < {2}）" -f $q, $lv, $rq.min)
                continue
            }
            if ($rq.max -and $lv -gt $rq.max) {
                Write-Host ("[0] 跳过 quest {0}：等级超出上限（{1} > {2}）" -f $q, $lv, $rq.max)
                continue
            }
            if ($rq.req -and -not $completed.ContainsKey($rq.req)) {
                Write-Host ("[0] 跳过 quest {0}：前置任务 {1} 未完成" -f $q, $rq.req)
                continue
            }
        }
        $QuestId = $q
        break
    }
    if ($QuestId -eq 0) { Write-Host 'FAIL: ItemTasks 候选池已耗尽（或都没 NPC 链接）——按头部注释口径扫新候选'; exit 4 }
    Write-Host ("[0] 自动选任务: quest {0}（池 {1}）" -f $QuestId, ($Pool -join ','))
}
$tasks = QuestItemTasks $QuestId
if ($tasks.Count -eq 0) { Write-Host ("FAIL: 任务 {0} 的 [@ItemTasks] 为空（选错任务类型？）" -f $QuestId); exit 4 }
$task = $tasks[0]
$drop = QDropFor $task.name
if (-not $drop) { Write-Host ("FAIL: 物品 {0} 没有任何 Q（任务物品）掉落来源" -f $task.name); exit 4 }
Write-Host ("[0] 任务需求: {0} x{1}；来源怪 {2}（p={3}）@ 图 {4}；期望击杀 {5}" -f `
    $task.name, $task.need, $drop.monster, [Math]::Round($drop.chance, 3), $drop.map, [Math]::Round($task.need / $drop.chance, 1))

$accNpc = NpcLinkFor $QuestId $false
$finNpc = NpcLinkFor $QuestId $true
if (-not $accNpc -or -not $finNpc) { Write-Host ("FAIL: 找不到接取/交付 NPC（+/-{0}）" -f $QuestId); exit 4 }

# A) 接取
Rpc 'chat' @{ message = "@mapmove $($accNpc.map) $($accNpc.x) $($accNpc.y)" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($accNpc.map)") { break } }
$taken0 = Taken
$rA = Rpc 'accept_quest' @{ npc_index = $accNpc.npc_index; quest_index = $QuestId }
$taken1 = $taken0
foreach ($i in 1..15) { Start-Sleep 1; $taken1 = Taken; if ($taken1 -contains $QuestId) { break } }
$okA = $taken1 -contains $QuestId
Write-Host ("[A] accept_quest -> {0}；taken {1} -> {2}（含 {3} = {4}）" -f `
    ($rA | ConvertTo-Json -Compress), ($taken0 -join ','), ($taken1 -join ','), $QuestId, $okA)

# B) 打怪凑任务物品（判据 = **任务格**里出现该物品且数量达标）
Rpc 'chat' @{ message = "@mapmove $($drop.map) $($drop.x) $($drop.y)" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($drop.map)") { break } }
if ("$((Rpc 'state').map)" -ne "$($drop.map)") {
    # 刷新点坐标也可能因为落图判定失败 → 退回不带坐标的随机落点（服务端 TeleportRandom 语义）
    Write-Host ("[B] @mapmove {0} {1} {2} 未生效，改用随机落点" -f $drop.map, $drop.x, $drop.y)
    Rpc 'chat' @{ message = "@mapmove $($drop.map)" } | Out-Null
    foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($drop.map)") { break } }
}
if (EnsureMobile $drop.map) {
    $stB = Rpc 'state'
    Write-Host ("[B] 落点可走（已走到 tile ({0},{1})）" -f $stB.tile_x, $stB.tile_y)
} else {
    Write-Host ("[B] 警告：{0} 上落点始终不可走——贴上去打会失败（如实记录，不硬造）" -f $drop.map)
}
$have = 0; $kills = 0; $deadline = (Get-Date).AddSeconds($HarvestTimeoutSec)
while ($have -lt $task.need -and $kills -lt $KillCap -and (Get-Date) -lt $deadline) {
    $ok = KillNamed $drop.monster
    if ($ok) { $kills++ }
    Start-Sleep 1
    $have = QuestCount $task.name
    if (($kills % 5) -eq 0 -or $have -ge $task.need) {
        Write-Host ("[B] 击杀 {0} 只，任务格 {1} x{2}/{3}" -f $kills, $task.name, $have, $task.need)
    }
}
$okB = ($have -ge $task.need)
Write-Host ("[B] 任务格 {0} x{1}（需求 {2}）= {3}；共击杀 {4} 只 {5}" -f `
    $task.name, $have, $task.need, $okB, $kills, $drop.monster)

# C/D) 交付 + 奖励
$gold0 = [int](Rpc 'bag_probe').gold; $exp0 = [int](Rpc 'bag_probe').exp
Rpc 'chat' @{ message = "@mapmove $($finNpc.map) $($finNpc.x) $($finNpc.y)" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($finNpc.map)") { break } }
$rC = Rpc 'finish_quest' @{ quest_index = $QuestId; selected_item_index = -1 }
$taken2 = $taken1
foreach ($i in 1..20) { Start-Sleep 1; $taken2 = Taken; if (-not ($taken2 -contains $QuestId)) { break } }
$okC = -not ($taken2 -contains $QuestId)
$gold1 = $gold0; $exp1 = $exp0
foreach ($i in 1..20) {
    Start-Sleep 1
    $gold1 = [int](Rpc 'bag_probe').gold; $exp1 = [int](Rpc 'bag_probe').exp
    if (($gold1 -gt $gold0) -and ($exp1 -gt $exp0)) { break }
}
$okD = (($gold1 -gt $gold0) -and ($exp1 -gt $exp0))
Write-Host ("[C] finish_quest -> {0}；taken {1} -> {2}（已移除={3}）" -f ($rC | ConvertTo-Json -Compress), ($taken1 -join ','), ($taken2 -join ','), $okC)
Write-Host ("[D] 奖励：gold {0}->{1}；exp {2}->{3}" -f $gold0, $gold1, $exp0, $exp1)

Write-Host ("VERDICT accept={0} quest_items_in_quest_bag={1} finish={2} reward={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 5 }
