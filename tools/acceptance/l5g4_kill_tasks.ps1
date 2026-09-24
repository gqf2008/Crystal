# l5g4_kill_tasks.ps1 — ④ 任务闭环（**KillTasks 类型**，端到端）：接取 → 打够 N 只 → 交付 → 奖励
#
# 为什么单开一条：l5g 覆盖「空目标」、l5g2 覆盖 `CarryItems`、l5g3 覆盖 `ItemTasks`（含剥皮交付）；
# **KillTasks**（`[@KillTasks]` 段：杀够 N 只指定怪）是五闭环「任务」一环最后一块没覆盖的形状。
#
# 判据（全取状态 / 服务端事件流）：
#   A) 接取后 `quest_probe.taken` 含该任务
#   B) 每只目标怪的击杀在 `combat_probe.events` 里有 `died`（服务端事件流），且
#      `quest_probe.taken[].tasks` 的进度行走到 `N/N`（该文本由服务端 ChangeQuest 下发，与 taken 同源）
#   C) 到交付 NPC 处 finish_quest → taken 不再含它（服务端对**进度未满**会拒绝，所以 C 成立反过来证明 B）
#   D) gold delta 为正（与 l5g3 同口径：exp/level 字段本机客户端会滞后，只作记录）
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
# 判据脚本与客户端构建都取自**本脚本所在的 worktree**（写死别的 worktree 会静默跑旧构建）
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

# 任务文件的 KillTasks → @( @{ name; need } )（杀够 N 只）
function QuestKillTasks([int]$qid) {
    $f = Join-Path $questDir "$qid.txt"
    if (-not (Test-Path $f)) { return @() }
    $sec = $null; $out = @()
    foreach ($ln in (Get-Content $f)) {
        if ($ln -match '^\[@(.+)\]') { $sec = $Matches[1]; continue }
        if ($sec -eq 'KillTasks' -and $ln.Trim()) {
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
# 目标怪 → 刷得最多的那张图（KillTasks 只要找得到怪，不依赖掉落）
function KillMapFor([string]$monsterName) {
    $m = Db "select if2.file_name from map_respawns mr join monster_infos mi on mi.idx=mr.monster_index join map_infos if2 on if2.idx=mr.map_index where lower(mi.name)=lower('$monsterName') group by if2.file_name order by sum(mr.count) desc limit 1" | Select-Object -First 1
    if ("$m" -match '^\(\s*''(.*)''\s*,?\s*\)$') { $m = $Matches[1] }
    return "$m".Trim()
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

# 进度读数（服务端真值经 ChangeQuest 下发到客户端 tasks 文本，形如 "9/10"）。
# **按条目**返回 @( @(current,target), ... )：一个任务可以有多条 KillTasks（q33 = TigerSnake 10 + RedSnake 10），
# 顺序与 [@KillTasks] 一致。本地"击杀计数"不能当判据——它会把"目标离开视野"误记成击杀
# （实测 10 次里 1 次是这种，服务端只认 9 次，交付被"进度未满"挡下）。
function TaskPairs([int]$qid) {
    $text = ''
    foreach ($e in (Rpc 'quest_probe').taken) {
        if ([int]$e.id -eq $qid) { $text = (@($e.tasks) -join ' ') }
    }
    $out = @()
    foreach ($m in [regex]::Matches($text, '(\d+)\s*/\s*(\d+)')) {
        $out += ,@([int]$m.Groups[1].Value, [int]$m.Groups[2].Value)
    }
    return $out
}
function TaskProgressAt([int]$qid, [int]$idx) {
    $p = TaskPairs $qid
    if ($idx -lt $p.Count) { return $p[$idx][0] }
    return -1
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

# 启动客户端（进场失败重启 ×3：本机客户端偶发在 AppState enter 崩，见 crystal-client-entry-crash）
$st = $null; $entered = $false
foreach ($attempt in 1..3) {
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5g4_client.log" -RedirectStandardError "$acc\l5g4_client.err.log" | Out-Null
    foreach ($i in 1..45) {
        Start-Sleep 1
        try {
            $st = Rpc 'state'
            if ($null -ne $st.tile_x) { $bp0 = Rpc 'bag_probe'; if ($bp0.ok) { $entered = $true; break } }
        } catch {}
    }
    if ($entered) { break }
    Write-Host ("[0] 客户端第 {0} 次进场失败 → 重启客户端" -f $attempt)
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep 2
}
if (-not $entered) { Write-Host 'FAIL: 客户端三次都没进场'; exit 9 }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 目标怪的刷点（用于落点；KillTasks 不依赖掉落，只要找得到怪）
function KillSpot([string]$monsterName, [string]$mapName) {
    $row = Db "select mr.x, mr.y, mr.count from map_respawns mr join monster_infos mi on mi.idx=mr.monster_index join map_infos if2 on if2.idx=mr.map_index where lower(mi.name)=lower('$monsterName') and if2.file_name='$mapName' order by mr.count desc limit 1" | Select-Object -First 1
    if ("$row" -match '^\((\d+), (\d+), (\d+)\)$') { return @([int]$Matches[1], [int]$Matches[2]) }
    return @(-1, -1)
}

# [0] 选任务（KillTasks 非空 + +/- NPC 链接齐 + 等级/职业/前置过；池子按「刷得多 × 期望击杀少」）
$Pool = @(33, 5, 8, 6, 101, 115, 139, 90, 48)
if ($QuestId -eq 0) {
    $live = @{}
    foreach ($e in (Rpc 'quest_probe').taken) { $live[[int]$e.id] = $true }
    $lv = [int](Rpc 'bag_probe').level
    foreach ($q in $Pool) {
        if ($live.ContainsKey($q)) { continue }
        if (-not (NpcLinkFor $q $false) -or -not (NpcLinkFor $q $true)) {
            Write-Host ("[0] 跳过 quest {0}：没有 +{0}/-{0} 的 [QUESTS] 链接" -f $q); continue
        }
        $rq = QuestReq $q
        if ($rq) {
            if ($rq.min -and $lv -lt $rq.min) { Write-Host ("[0] 跳过 quest {0}：等级不足" -f $q); continue }
            if ($rq.max -and $lv -gt $rq.max) { Write-Host ("[0] 跳过 quest {0}：等级过高" -f $q); continue }
        }
        $QuestId = $q
        break
    }
    if ($QuestId -eq 0) { Write-Host 'FAIL: KillTasks 候选池已耗尽'; exit 4 }
    Write-Host ("[0] 自动选任务: quest {0}（池 {1}）" -f $QuestId, ($Pool -join ','))
}
$tasks = QuestKillTasks $QuestId
if ($tasks.Count -eq 0) { Write-Host ("FAIL: 任务 {0} 的 [@KillTasks] 为空（选错任务类型？）" -f $QuestId); exit 4 }
$task = $tasks[0]
$killMap = KillMapFor $task.name
$spot = KillSpot $task.name $killMap
Write-Host ("[0] 任务要求: 杀 {0} x{1}；刷图 {2} 刷点 ({3},{4})" -f $task.name, $task.need, $killMap, $spot[0], $spot[1])
$accNpc = NpcLinkFor $QuestId $false
$finNpc = NpcLinkFor $QuestId $true
if (-not $accNpc -or -not $finNpc) { Write-Host ("FAIL: 找不到接取/交付 NPC（+/-{0}）" -f $QuestId); exit 4 }

# 复位（可重复跑的关键）：上一轮可能把该任务留在 taken / completed 里（本轮就是这样：进度 9/10 被服务端
# 挡下交付，任务仍挂在日志里）。`@setquest <id> 0` 会把它从服务端任务日志移除并清 completed 标记，
# 但**客户端的任务日志要重新登录才会刷新**，所以复位后重启一次客户端（判据仍是"clean 接取"）。
$already = (Taken) -contains $QuestId
if ($already) {
    Write-Host ("[A] quest {0} 已在 taken 里（上轮残留）→ GM @setquest {0} 0 复位 + 重启客户端" -f $QuestId)
    Rpc 'chat' @{ message = "@setquest $QuestId 0" } | Out-Null
    Start-Sleep 3
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep 2
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5g4_client.log" -RedirectStandardError "$acc\l5g4_client.err.log" | Out-Null
    for ($i = 1; $i -le 45; $i++) {
        Start-Sleep 1
        try { $stR = Rpc 'state'; if ($null -ne $stR.tile_x) { $bpR = Rpc 'bag_probe'; if ($bpR.ok) { break } } } catch {}
    }
    Write-Host ("[A] 复位后 taken = {0}" -f ((Taken) -join ','))
}

# A) 接取（前置任务用 GM 标记完成——链路外前提）
Rpc 'chat' @{ message = "@mapmove $($accNpc.map) $($accNpc.x) $($accNpc.y)" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($accNpc.map)") { break } }
$rqA = QuestReq $QuestId
if ($rqA -and $rqA.req -gt 0) {
    Rpc 'chat' @{ message = "@setquest $($rqA.req) 1" } | Out-Null
    Start-Sleep 2
    Write-Host ("[A] 前置任务 {0} 用 GM 标记完成（链路外前提）" -f $rqA.req)
}
$taken0 = Taken
$rA = Rpc 'accept_quest' @{ npc_index = $accNpc.npc_index; quest_index = $QuestId }
$taken1 = $taken0
foreach ($i in 1..15) { Start-Sleep 1; $taken1 = Taken; if ($taken1 -contains $QuestId) { break } }
$okA = $taken1 -contains $QuestId
Write-Host ("[A] accept_quest -> {0}；taken {1} -> {2}（含 {3} = {4}）" -f ($rA | ConvertTo-Json -Compress), ($taken0 -join ','), ($taken1 -join ','), $QuestId, $okA)

# B) 逐条 KillTasks 打够（进度读数取服务端真值；每条独立落点 + 贴近 + 等同步）
$deadline = (Get-Date).AddSeconds($HarvestTimeoutSec)
$okB = $true
for ($ti = 0; $ti -lt $tasks.Count; $ti++) {
    $tk = $tasks[$ti]
    $kmap = KillMapFor $tk.name
    $kspot = KillSpot $tk.name $kmap
    Write-Host ("[B] 条目 {0}/{1}：杀 {2} x{3} @ 图 {4}（刷点 {5},{6}）" -f ($ti + 1), $tasks.Count, $tk.name, $tk.need, $kmap, $kspot[0], $kspot[1])
    if (EnsureMobile $kmap $kspot[0] $kspot[1]) {
        $stB = Rpc 'state'
        Write-Host ("      · 落点可走（已走到 tile ({0},{1})）" -f $stB.tile_x, $stB.tile_y)
    } else {
        Write-Host ("      · 警告：{0} 上落点始终不可走" -f $kmap)
    }
    if (-not (WaitInSync 8)) { Write-Host "      · 警告：8s 内没等到 in_sync" }
    $kills = 0; $miss = 0
    # 双重边界：进度读数达标 **或** 本地击杀够数就收手。
    # 为什么不能只等进度读数：**客户端展示的进度会滞后**（实测服务端/库里已是 {67:10/10, 64:10/10}，
    # 而客户端 tasks 文本还停在 9/10 ⇒ 只看文本会一直空杀）；为什么不能只看本地击杀：
    # 本地计数会把"目标离开视野"误记成击杀（实测 10 次里 1 次）。两条合起来 + 用 [C] 交付是否被
    # 服务端放行作为**权威**判据（服务端对进度未满会拒绝）。
    while ((TaskProgressAt $QuestId $ti) -lt $tk.need -and $kills -lt $tk.need -and (Get-Date) -lt $deadline) {
        $ok = KillNamed $tk.name
        if ($ok) {
            $kills++; $miss = 0
            Write-Host ("      · 击杀 {0} 次（条目进度 {1}/{2}）" -f $kills, (TaskProgressAt $QuestId $ti), $tk.need)
        } else {
            $miss++
            if ($miss -ge 3) {
                Write-Host ("      · 连续 {0} 次附近没有 {1} → 换随机落点再来" -f $miss, $tk.name)
                Rpc 'chat' @{ message = "@mapmove $kmap" } | Out-Null
                Start-Sleep 4
                [void](TestMobile)
                [void](WaitInSync 8)
                $miss = 0
            }
        }
    }
    $cur = TaskProgressAt $QuestId $ti
    Write-Host ("[B] 条目 {0}：进度读数 {1}/{2}（本地击杀 {3} 次）" -f $tk.name, $cur, $tk.need, $kills)
    if ($cur -lt $tk.need) { $script:progressDisplayShort = $true }
}
$prog = ''
foreach ($e in (Rpc 'quest_probe').taken) { if ([int]$e.id -eq $QuestId) { $prog = (@($e.tasks) -join ' | ') } }
Write-Host ("[B] 进度文本: {0}" -f $prog)

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

# [B] 的权威判据：进度文本达标，**或** 服务端放行交付（进度未满会被拒 ⇒ C 成立即 B 成立）
if ($progressDisplayShort -and -not $okC) { $okB = $false }
Write-Host ("[B] 判据：进度文本{0} / 交付被服务端放行={1} ⇒ kills_counted={2}" -f `
    $(if ($progressDisplayShort) { '滞后（未读到 N/N）' } else { '读到 N/N' }), $okC, $okB)

Write-Host ("VERDICT accept={0} kills_counted={1} finish={2} reward={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 5 }
