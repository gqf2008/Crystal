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
# 仪器口径（2026-09-24 定稿）：**只用自然刷新的怪**，不做 GM 召唤。
#   为什么不用 `@recallmob`（实测结论，别再试）：它只能把怪召到玩家**自己那一格**（带坐标的形式
#   不生效）；同格怪近战方向差值是 (0,0)→退化成 Up，`@kill` 又只结算"正前方一格"，两条路都杀不掉。
#   自然刷新的怪在客户端/服务端两端坐标一致、方向明确，才是可靠判据来源；找不到目标就换随机落点再来。
#
# ==== 2026-09-24 实测状态（④ ItemTasks 端到端：**本机已人工跑通**）====
#   quest 30（JadeRing x1 ← Currish p=0.33 @ map 2；接取/交付 NPC Merchant_Bradley @ map 0120）：
#     [A] 接取 PASS      taken 1,28,46,48,76,90,142 -> …142,30
#     [B] 任务物品 PASS  击杀 Currish → **剥皮两次** → 服务端 `rolled item=1117 quest_required=true`
#                        → 客户端任务格 `{"cell":0,"count":1,"name":"JadeRing"}`
#     [C] 交付 PASS      finish_quest → taken 不再含 30、任务格清空
#     [D] 奖励 PASS      gold 1024474 -> 1025274（+800 = quest_infos.gold_reward）
#   —— 两处关键修复都已在 master：① 击杀路径的 Q 行交付顺序（`drop_should_land`）；
#      ② **剥皮路径**（`roll_harvest_drops` 不再跳过 quest_required，改交 `try_give_quest_item_at`）。
#
#   夹具自动跑仍受两件**非链路**因素干扰（各自记账）：
#     · 本机客户端**偶发进场崩溃**（Bevy 在 AppState enter 里 spawn dura_status UI 的命令 panic，
#       栈里是 `dura_status::DuraToggleBtn`）→ 崩了就没有本地玩家；夹具已内置"进场失败重启客户端×3"。
#     · `@clearquests` 之后客户端任务日志/probe 不刷新（`quest_probe.taken` 停在旧值），
#       默认不要加 `-ResetQuests`；重跑用干净库或重启服务端即可。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [int]$QuestId = 0,
    [int]$KillCap = 60,
    [switch]$ResetQuests,
    [int]$HarvestTimeoutSec = 900
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5g3_quest_item_tasks' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
# 判据脚本与客户端构建都必须取自**本脚本所在的 worktree**：写死别的 worktree 会静默跑旧构建、
# 给出假红/假绿（本轮前几版就硬编码过 Crystal-wt-p3）。
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$acc = Join-Path $wt 'tools\acceptance'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName 'l5g3_quest_item_tasks'
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5g3_client.exe'
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
# 站得住吗？判据是"真走一步"：四个邻格轮流 walk_to，位置变了才算可走（问地图不算数）。
function TestMobile {
    $st = Rpc 'state'
    $tx = [int]$st.tile_x; $ty = [int]$st.tile_y
    foreach ($off in @(@(1, 0), @(-1, 0), @(0, 1), @(0, -1))) {
        $nx = $tx + $off[0]; $ny = $ty + $off[1]
        Rpc 'walk_to' @{ x = [double]($nx * 48 + 24); y = [double](-($ny * 32 + 32)) } | Out-Null
        Start-Sleep 2
        $st2 = Rpc 'state'
        if ("$($st2.tile_x),$($st2.tile_y)" -ne "$tx,$ty") { return $true }
    }
    return $false
}

# 落到 (x,y) 并确认站得住（先等换图完成）
function LandAt([string]$mapName, [int]$x, [int]$y) {
    Rpc 'chat' @{ message = "@mapmove $mapName $x $y" } | Out-Null
    foreach ($i in 1..12) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$mapName") { break } }
    return (TestMobile)
}

# 找一块能站的地：① 先试刷点本身 ② 再试刷点四邻格（**刷点常落在封闭口袋里，而邻格往往能走**——
# 实测 map '2' 的 Currish 刷点 (400,200) 四邻格全不可达，但 (399,199)/(401,199) 一带能走）
# ③ 都不行才退回不带坐标的随机落点。
function EnsureMobile([string]$mapName, [int]$baseX = -1, [int]$baseY = -1, [int]$tries = 4) {
    if ($baseX -ge 0 -and $baseY -ge 0) {
        foreach ($off in @(@(0, 0), @(1, 0), @(-1, 0), @(0, 1), @(0, -1))) {
            $x = $baseX + $off[0]; $y = $baseY + $off[1]
            if (LandAt $mapName $x $y) {
                Write-Host ("      · 落点 ({0},{1}) 可走" -f $x, $y)
                return $true
            }
            Write-Host ("      · 落点 ({0},{1}) 不可走 → 试下一个" -f $x, $y)
        }
    }
    foreach ($t in 1..$tries) {
        Rpc 'chat' @{ message = "@mapmove $mapName" } | Out-Null
        Start-Sleep 4
        if (TestMobile) { return $true }
        $st = Rpc 'state'
        Write-Host ("      · 随机落点 ({0},{1}) 不可走 → 再试" -f $st.tile_x, $st.tile_y)
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
# 2026-09-24 追加第三条（本轮 626 个 attack 包零命中的直接原因）：
#   3) **必须等 `state.in_sync == true` 再挥砍**。客户端本地预测天然领先服务端一步（移动包在
#      "到达那一步"时才发），方向却是客户端用**自己**的格差算的，而命中由服务端按
#      「服务端玩家格 + 方向」结算（`world/combat.rs`）。原点差一格 = 落在空地。
#      另外目标**贴在自己这一格**（dist=0）时方向差值是 (0,0)→退化成 Up，也永远打不到。
# 判据是 `combat_probe.events` 里 id == 目标的 `died`（服务端事件流），不是"血条看起来掉了"。
function KillOne($mon, [int]$timeoutSec = 30) {
    if (-not $mon) { return $false }
    $id = [uint32]$mon.object_id
    # **先自己贴到邻格再锁目标**：靠 auto_attack 追击时本地一直在动，`UserLocation` 校正在 LocalMove
    # 活动期间被推迟 ⇒ `in_sync` 永远不成立、但客户端仍按自己的格算方向 → 空挥（实测 targets 一直
    # "最近贴到 1 格, in_sync=False"）。walk_to 到位后路径结束、校正落地，`in_sync` 才成立，此时开打才准。
    $target = $mon
    for ($a = 1; $a -le 6; $a++) {
        $stA = Rpc 'state'
        $mtx = [int][math]::Round(([double]$target.x - 24) / 48.0)
        $mty = [int][math]::Round((-[double]$target.y - 32) / 32.0)
        $dx = [math]::Sign([int]$stA.tile_x - $mtx)
        $dy = [math]::Sign([int]$stA.tile_y - $mty)
        # 目标与自己同格（呼叫/追击都会造成）：必须先走开一格，否则方向差值是 (0,0)、
        # 服务端按"正前方一格"结算永远打不到（实测 dist=0 的目标能贴 20s 不死）。
        if ($dx -eq 0 -and $dy -eq 0) { $dx = 1 }
        $near = @(($stA.tile_x + $dx), ($stA.tile_y + $dy))
        if (($near[0] -ne [int]$stA.tile_x) -or ($near[1] -ne [int]$stA.tile_y)) {
            Rpc 'walk_to' @{ x = [double]($near[0] * 48 + 24); y = [double](-($near[1] * 32 + 32)); run = $true } | Out-Null
            Start-Sleep 3
        }
        if (WaitInSync 6) { break }
        $next = (Monsters (Rpc 'nearby' @{ radius = 1500 }) | Where-Object { $_.object_id -eq $id } | Select-Object -First 1)
        if (-not $next) { return $false }
        $target = $next
    }
    Rpc 'attack' @{ object_id = $id } | Out-Null
    $deadline = (Get-Date).AddSeconds($timeoutSec)
    $lastTile = $null; $stable = 0; $minDist = $null; $onOwnTile = 0
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $stNow = Rpc 'state'
        $tileKey = "$($stNow.tile_x),$($stNow.tile_y)"
        if ($tileKey -eq $lastTile) { $stable++ } else { $stable = 0; $lastTile = $tileKey }
        $cp = Rpc 'combat_probe'
        if ($null -ne $cp.target_tile) { $script:lastTargetTile = $cp.target_tile }
        foreach ($e in @($cp.events)) { if ($e.kind -eq 'died' -and $e.id -eq $id) { return $true } }
        if ($null -eq $cp.attack_target) { return $true }   # 目标已从场景消失
        if ($null -ne $cp.target_dist_tiles) {
            $d = [int]$cp.target_dist_tiles
            if ($null -eq $minDist -or $d -lt $minDist) { $minDist = $d }
            # 目标与自己同格 → 方向 (0,0) 退化，永远打不到，别在这只上耗时间
            if ($d -eq 0) { $onOwnTile++ } else { $onOwnTile = 0 }
            if ($onOwnTile -ge 16) {
                Write-Host ("      · 目标 {0} 一直贴在自己这一格（方向退化），放弃这只" -f $id)
                return $false
            }
        }
        if ($stNow.in_sync -and $stable -ge 2 -and $null -ne $cp.target_dist_tiles `
                -and [int]$cp.target_dist_tiles -eq 1) {
            Rpc 'attack' @{ object_id = $id } | Out-Null
        }
    }
    Write-Host ("      · 目标 {0} 超时（最近贴到 {1} 格，in_sync={2}）→ 换下一只/改召唤" -f $id, $minDist, $stNow.in_sync)
    return $false
}

# 等「客户端这一格 == 服务端权威那一格」：移动刚停下时本地预测可能还领先一步，
# 这一步内挥砍必空（见 KillOne 注释第 3 条）。
function WaitInSync([int]$timeoutSec = 8) {
    $deadline = (Get-Date).AddSeconds($timeoutSec)
    $nudged = 0
    while ((Get-Date) -lt $deadline) {
        $st = Rpc 'state'
        if ($st.in_sync) { return $true }
        # **卡在"没同步"多半是没人在动**：服务端只在移动/拒绝时发 UserLocation，
        # 玩家一静下来就再没有新权威位置可校正。这时主动朝**服务端那一格**走一小步
        # （产生一次移动 → 一次 UserLocation），客户端随即收敛。实测这一步能把
        # "8s 内没等到 in_sync" 的僵局打开（此前会一路卡到 B 阶段全废）。
        if ($nudged -lt 3 -and $null -ne $st.server_tile_x) {
            $tx = [int]$st.server_tile_x; $ty = [int]$st.server_tile_y
            if ($tx -ne [int]$st.tile_x -or $ty -ne [int]$st.tile_y) {
                Rpc 'walk_to' @{ x = [double]($tx * 48 + 24); y = [double](-($ty * 32 + 32)); run = $true } | Out-Null
                $nudged++
                Start-Sleep 2
                continue
            }
        }
        Start-Sleep -Milliseconds 400
    }
    return $false
}

# 击杀后**剥皮**：可采集怪（Currish/SpittingSpider/Deer 系…）的 Q 物品只能从尸体上拿——
# 服务端 `HarvestMonster.Harvest` 的两次剥皮后摇掉落、再由任务系统交付（2026-09-24 实机确认：
# 击杀 Currish → 两次剥皮 → `quest_occupied` 出现 JadeRing）。普通怪没有尸体，这一步无副作用。
# 朝向必须对准尸体：服务端 `try_harvest_corpse` 只看「玩家格 + 方向」的正前方 3×3。
function HarvestCorpse([int]$times = 3) {
    for ($h = 1; $h -le $times; $h++) {
        $st = Rpc 'state'
        $dir = 0
        if ($script:lastTargetTile) {
            $dx = [math]::Sign([int]$script:lastTargetTile[0] - [int]$st.tile_x)
            $dy = [math]::Sign([int]$script:lastTargetTile[1] - [int]$st.tile_y)
            $dir = switch ("$dx,$dy") {
                "-1,0" { 6 } "1,0" { 2 } "0,-1" { 0 } "0,1" { 4 }
                "-1,-1" { 7 } "1,-1" { 1 } "-1,1" { 5 } "1,1" { 3 } default { 0 }
            }
        }
        Rpc 'harvest' @{ direction = $dir } | Out-Null
        Start-Sleep -Milliseconds 800
    }
}

function KillNamed([string]$monsterName) {
    foreach ($m in (FindNamedList $monsterName 3)) { if (KillOne $m) { return $true } }
    # 打不到就算了，交给调用方换落点再来——**不做 GM 召唤**。
    # 为什么不用 `@recallmob`（2026-09-24 实测结论，别再试）：它只能召到玩家**自己那一格**
    # （带坐标的形式实测不生效），而同格怪近战方向差值是 (0,0)→退化成 Up、`@kill` 又只打
    # 「正前方一格」，两条路都杀不掉。自然刷新的怪两端坐标一致、方向明确，才是可靠判据来源。
    return $false
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }

Get-CimInstance Win32_Process -Filter "Name='l5g3_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
# 进场需要重试：本机客户端**偶发**在进场时崩（Bevy 在 AppState enter 里 spawn dura_status UI 的命令 panic，
# 栈里是 `dura_status::DuraToggleBtn` 那条 spawn），崩了就没有本地玩家、后面全空。夹具自己重试，
# 免得把"环境偶发"记成"功能失败"。
$st = $null; $entered = $false
foreach ($attempt in 1..3) {
    # 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5g3_client.log" -RedirectStandardError "$acc\l5g3_client.err.log" | Out-Null
    foreach ($i in 1..45) {
        Start-Sleep 1
        try {
            $st = Rpc 'state'
            if ($null -ne $st.tile_x) {
                $bp0 = Rpc 'bag_probe'
                if ($bp0.ok) { $entered = $true; break }
            }
        } catch {}
    }
    if ($entered) { break }
    Write-Host ("[0] 客户端第 {0} 次进场失败（无本地玩家）→ 重启客户端" -f $attempt)
    Get-CimInstance Win32_Process -Filter "Name='l5g3_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep 2
}
if (-not $entered) { Write-Host 'FAIL: 客户端三次都没进场（见 l5g3_client.err.log）'; exit 9 }
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
if ($ResetQuests) {
    # 保证 [A] 是"干净的一次接取"：本机反复跑会把 quest 打成 completed（`@CLEARQUESTS` 清空
    # 客户端在内存里的任务记录，含 completed 标记）。链路口径本身仍是真实的 接取→打怪→交付。
    Rpc 'chat' @{ message = "@clearquests" } | Out-Null
    Start-Sleep 3
}
$taken0 = Taken
# 前置任务：服务端会以「需要先完成前置任务」拒绝（**只走系统消息**，RPC 仍回 ok:true）。
# 前置任务本身不是本夹具要验的链路，按验收夹具惯例用 GM `@setquest <id> 1` 把它标成已完成。
# （踩坑记录：之前用 `@clearquests` 清任务，连带把 completed 标记清掉 → 之后本任务再也接不上。）
$rqA = QuestReq $QuestId
if ($rqA -and $rqA.req -gt 0 -and -not $completed.ContainsKey($rqA.req)) {
    Rpc 'chat' @{ message = "@setquest $($rqA.req) 1" } | Out-Null
    Start-Sleep 2
    Write-Host ("[A] 前置任务 {0} 用 GM 标记完成（链路外前提）" -f $rqA.req)
}
$rA = Rpc 'accept_quest' @{ npc_index = $accNpc.npc_index; quest_index = $QuestId }
$taken1 = $taken0
foreach ($i in 1..15) { Start-Sleep 1; $taken1 = Taken; if ($taken1 -contains $QuestId) { break } }
$okA = $taken1 -contains $QuestId
Write-Host ("[A] accept_quest -> {0}；taken {1} -> {2}（含 {3} = {4}）" -f `
    ($rA | ConvertTo-Json -Compress), ($taken0 -join ','), ($taken1 -join ','), $QuestId, $okA)

# B) 打怪凑任务物品（判据 = **任务格**里出现该物品且数量达标）
# 落到**刷点附近**（刷点本身或它的四邻格）：这样自然刷新的目标怪大概率就在视野里，
# 也就不必走"召唤"那条路（实测召唤出来的怪在客户端/服务端的位置口径上还有坑）。
if (EnsureMobile $drop.map $drop.x $drop.y) {
    $stB = Rpc 'state'
    Write-Host ("[B] 落点可走（已走到 tile ({0},{1})）" -f $stB.tile_x, $stB.tile_y)
} else {
    Write-Host ("[B] 警告：{0} 上落点始终不可走——贴上去打会失败（如实记录，不硬造）" -f $drop.map)
}
if (WaitInSync 8) { Write-Host "[B] 客户端与服务端已同步（可以开打）" } else { Write-Host "[B] 警告：8s 内没等到 in_sync" }
$have = 0; $kills = 0; $miss = 0; $deadline = (Get-Date).AddSeconds($HarvestTimeoutSec)
while ($have -lt $task.need -and $kills -lt $KillCap -and (Get-Date) -lt $deadline) {
    $ok = KillNamed $drop.monster
    if ($ok) {
        $kills++
        $miss = 0
        HarvestCorpse 3   # 可采集怪：Q 物品靠剥皮交付（普通怪无尸体，空跑）
    } else {
        $miss++
        # 连续打不到 → 换一块随机落点再来（地图大、刷点分散；不召唤，理由见 KillNamed 注释）
        if ($miss -ge 3) {
            Write-Host ("      · 连续 {0} 次附近没有 {1} → 换随机落点再来" -f $miss, $drop.monster)
            Rpc 'chat' @{ message = "@mapmove $($drop.map)" } | Out-Null
            Start-Sleep 4
            if (TestMobile) { Write-Host "      · 新落点可走" } else { Write-Host "      · 新落点不可走（继续，下一轮还会换）" }
            if (-not (WaitInSync 8)) { Write-Host "      · 警告：新落点 8s 内没等到 in_sync" }
            $miss = 0
        }
    }
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
# 判据口径（2026-09-24 实机标定）：**金币 delta 必须为正**（且与 quest_infos.gold_reward 相符），
# 经验 delta 为正 **或** 等级上升即算到账——实测本机客户端 `bag_probe.exp/level` 会滞后/串档
# （同一时刻客户端报 level=54 而库里是 46、exp 交付前后都是 80），只认 exp 会给出假红。
$b0 = Rpc 'bag_probe'
$gold0 = [int]$b0.gold; $exp0 = [int]$b0.exp; $lv0 = [int]$b0.level
Rpc 'chat' @{ message = "@mapmove $($finNpc.map) $($finNpc.x) $($finNpc.y)" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($finNpc.map)") { break } }
$rC = Rpc 'finish_quest' @{ quest_index = $QuestId; selected_item_index = -1 }
$taken2 = $taken1
foreach ($i in 1..20) { Start-Sleep 1; $taken2 = Taken; if (-not ($taken2 -contains $QuestId)) { break } }
$okC = -not ($taken2 -contains $QuestId)
$gold1 = $gold0; $exp1 = $exp0; $lv1 = $lv0
foreach ($i in 1..20) {
    Start-Sleep 1
    $b1 = Rpc 'bag_probe'
    $gold1 = [int]$b1.gold; $exp1 = [int]$b1.exp; $lv1 = [int]$b1.level
    if (($gold1 -gt $gold0) -and (($exp1 -gt $exp0) -or ($lv1 -gt $lv0))) { break }
}
$okD = (($gold1 -gt $gold0) -and (($exp1 -gt $exp0) -or ($lv1 -gt $lv0)))
Write-Host ("[C] finish_quest -> {0}；taken {1} -> {2}（已移除={3}）" -f ($rC | ConvertTo-Json -Compress), ($taken1 -join ','), ($taken2 -join ','), $okC)
Write-Host ("[D] 奖励：gold {0}->{1}；exp {2}->{3}；level {4}->{5}" -f $gold0, $gold1, $exp0, $exp1, $lv0, $lv1)

Write-Host ("VERDICT accept={0} quest_items_in_quest_bag={1} finish={2} reward={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 5 }

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5g3_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
