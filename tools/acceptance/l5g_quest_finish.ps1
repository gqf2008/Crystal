# l5g_quest_finish.ps1 — ④ 任务闭环（交付段）：接取 → 到交付 NPC 处 → FinishQuest → 奖励到账
#
# 任务怎么选（都能对着数据核）：候选池里的任务满足——
#   * 目标段为空（Envir/Quests/<id>.txt 无 KillTasks/ItemTasks/CarryItems/FlagTasks 行），
#     而服务端 `QuestLog::is_progress_complete()` 是 `progress.iter().all(current>=target)`
#     —— 空进度**恒真**，所以它们是"走到交付 NPC 面前即可交付"的任务。
#   * 文件奖励 [@GoldReward]/[@ExpReward] 均 > 0（判据用 delta 不写死数值——服务端按**实例**发）。
#   * quest_infos.required_quest = 0（无前置链）、required_class = 31（全职业）、required_min_level ≤ 31。
#   * NPC 脚本 [QUESTS] 段有 `-<id>`（交付 NPC 可推导）。
# 池（2026-09-23 扫 Daneo1989/Envir/Quests + DB 得出）：43,51,63,79,93,97,102,110,117
#
# 为什么不能写死一个任务：交付是**永久态**（completed_quests 落行后服务端拒再接，
# 「该任务已完成」）——写死 27 的初版跑绿一次后第二轮起必然 accept=FAIL。
# 所以默认 -QuestId 0 = 自动选：按池顺序挑「本角色未完成」的第一个；
# 已接未交的（上一轮中断残留）直接续跑交付段。池耗尽会 exit 4 并提示扩池。
#
# 等待全是「轮询到条件」不是固定 sleep：冷服务端/密集图客户端首次生成对象要数秒
# （实测 map 2 生成 717 个对象期间 finish_quest 完成用了 ~4s），固定 3s 会让探针
# 读在奖励到达之前 → 假 FAIL（奖励其实到了，DB 与客户端日志可核）。
#
# 判据（取状态）：A) 接取后 taken 含该任务（轮询 ≤15s）
#               C) 到交付 NPC 处 finish_quest → taken 不再含它（轮询 ≤20s，服务端 CompleteQuest 生效）
#               D) 奖励：gold 与 exp delta 同时为正（轮询 ≤20s，bag_probe）
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    # 0 = 自动选（默认）：按候选池挑本角色未完成的第一个；显式给任务号则用它。
    [int]$QuestId = 0,
    # 交付点默认**从数据推导**（不手填）：NPC 脚本 [QUESTS] 段里 `-<quest>` 即"交付"，
    # 由 npc_index → npc_infos(地图/坐标) → map_infos(地图名) 三步换算。
    # 手填过一次，结果填到了「接取 NPC」上，服务端正确地回了「请到对应 NPC 处交付任务」。
    [string]$FinishMap = '',
    [int]$FinishX = -1,
    [int]$FinishY = -1,
    # 奖励判据：不写死数值——**任务实例自身带奖励快照**（DB `quests` 行里就存着 exp/gold），
    # 服务端按实例发放。写死文件值会得到假 FAIL，所以判据取「奖励真的到账」= 金币与经验同时为正。
    [int]$MinGold = 1,
    [int]$MinExp = 1,
    # 客户端构建根（其 Client-Bevy\target\debug\client_bevy.exe）；默认 wt-p3 保持原约定。
    [string]$ClientHome = '',
    # **受测服务端**的工作目录（其 Data\crystal.db 即判据来源）。非空时夹具的 DB 判据
    # 走 `dbq.py --db <ServerWorkDir>\Data\crystal.db`，不再默认读仓库 dev 库——
    # 实测：服务端起在 %TEMP%\e2e_workdir 时，默认 dev 库里的「已完成任务」集合与受测库不一致，
    # 夹具会选中一个已交付的任务（受测服务端正确拒绝接取），报成假 FAIL。
    [string]$ServerWorkDir = ''
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5g_quest_finish' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
# 客户端可用 -ClientHome 换到别的构建根：wt-p3 的构建不含 #3044（换图重建时对已
# despawn 实体排队 insert/remove → apply_net_motions panic），跑含 @mapmove 的用例会崩；
# 默认取本夹具所在仓库（而不是写死某个 worktree——写死会在换 worktree 时拿旧构建出假红）。
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Taken { @((Rpc 'quest_probe').taken | ForEach-Object { [int]$_.id }) }
function Gold { [int](Rpc 'bag_probe').gold }
# DB 判据来源：指定了 -ServerWorkDir 就用它的库（受测服务端实际在写的那个），否则用 dbq.py 默认库。
$script:dbArgs = @()
if ($ServerWorkDir) {
    $dbPath = Join-Path $ServerWorkDir 'Data\crystal.db'
    if (-not (Test-Path -LiteralPath $dbPath)) {
        Write-Host ("FAIL: -ServerWorkDir {0} 下没有 Data\crystal.db（判据来源缺失，拒绝用别的库代替）" -f $ServerWorkDir)
        exit 2
    }
    $script:dbArgs = @('--db', $dbPath)
    Write-Host ("[db] 判据来源={0}" -f $dbPath)
} else {
    Write-Host "[db] 判据来源=dbq.py 默认库（未指定 -ServerWorkDir）"
}
function Db([string]$sql) { (& python (Join-Path $wt 'tools\acceptance\dbq.py') @script:dbArgs $sql) }
# [QUESTS] 段语义（C# NPCScript.ParseQuests）：正数=该 NPC 可接，负数=可交。
# 按符号精确匹配（"143" 不能误中 "-143"，反之亦然），返回 NPC 的地图/坐标。
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

# 自动选任务：池内挑「本角色未完成」的第一个（角色名从 account_username 反查）。
$Pool = @(43, 51, 63, 79, 93, 97, 102, 110, 117)
if ($QuestId -eq 0) {
    $charName = (Db "select name from characters where account_username='$User'") | Select-Object -First 1
    if ("$charName" -match "^\( '(.*)',? \)$") { $charName = $Matches[1] }
    $charName = "$charName".Trim()
    $completed = @{}
    foreach ($r in (Db "select quest_index from completed_quests where character_name='$charName'")) {
        if ("$r" -match '(\d+)') { $completed[[int]$Matches[1]] = $true }
    }
    foreach ($q in $Pool) { if (-not $completed.ContainsKey($q)) { $QuestId = $q; break } }
    if ($QuestId -eq 0) { Write-Host 'FAIL: 候选池已耗尽（全部完成）——按头部注释口径扫新任务扩池'; exit 4 }
    Write-Host ("[0] 自动选任务: quest {0}（角色 {1} 未完成；池 {2}）" -f $QuestId, $charName, ($Pool -join ','))
}

if ($FinishMap -eq '') {
    $fin = NpcLinkFor $QuestId $true
    if (-not $fin) { Write-Host ("FAIL: 数据里找不到任务 {0} 的交付 NPC（[QUESTS] 段无 -{0}）" -f $QuestId); exit 4 }
    $FinishMap = $fin.map; $FinishX = $fin.x; $FinishY = $fin.y
    Write-Host ("[0] 交付点推导: quest {0} → NPC {1}(idx {2}) @ {3} ({4},{5})" -f $QuestId, $fin.name, $fin.npc_index, $fin.map, $fin.x, $fin.y)
}
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
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
    # 有接取 NPC 关联（[QUESTS] 正数项）的任务必须先站到它旁边——服务端 #2014 校验
    # 同图 DataRange(16)，不在范围回「请到对应 NPC 处接取任务」；无关联的（数据未配置）随处可接。
    $accNpc = NpcLinkFor $QuestId $false
    if ($accNpc) {
        Write-Host ("[A] 接取 NPC={0}(idx {1}) @ {2} ({3},{4})，先传送" -f $accNpc.name, $accNpc.npc_index, $accNpc.map, $accNpc.x, $accNpc.y)
        Rpc 'chat' @{ message = "@mapmove $($accNpc.map) $($accNpc.x) $($accNpc.y)" } | Out-Null
        foreach ($i in 1..15) {
            Start-Sleep 1; $sp = Rpc 'state'
            if ($null -ne $sp.tile_x -and [math]::Abs([int]$sp.tile_x - $accNpc.x) -le 16 -and [math]::Abs([int]$sp.tile_y - $accNpc.y) -le 16) { break }
        }
    }
    Rpc 'accept_quest' @{ npc_index = 0; quest_index = $QuestId } | Out-Null
    $taken1 = @()
    foreach ($i in 1..15) { Start-Sleep 1; $taken1 = Taken; if ($taken1 -contains $QuestId) { break } }
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
if ($npc) { Write-Host ("[B] 交付 NPC={0} id={1} dist={2}" -f $npc.name, $npc.object_id, $npc.dist) } else { Write-Host '[B] WARN: 附近没有 NPC（客户端对象未生成完不影响服务端交付校验）' }
if ($npc) { Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null; Start-Sleep 2 }
$rows = Rpc 'npc_rows'
Write-Host ("[B] NPC 窗可见={0} 行数={1} 链接={2}" -f $rows.visible, ($rows.lines | Measure-Object).Count, (($rows.links | ForEach-Object { $_.key }) -join ','))

# 交付
Rpc 'finish_quest' @{ quest_index = $QuestId; selected_item_index = -1 } | Out-Null
# 轮询而非固定 sleep：冷图首次生成对象时服务端完成处理要数秒（实测 ~4s），固定 3s 会假 FAIL
$removed = $false
foreach ($i in 1..20) { Start-Sleep 1; if (-not ((Taken) -contains $QuestId)) { $removed = $true; break } }
$taken2 = Taken
$p = $null
foreach ($i in 1..20) {
    Start-Sleep 1; $p = Rpc 'bag_probe'
    if (([int]$p.gold - $gold0) -ge $MinGold -and ([int]$p.exp - [int]$exp0) -ge $MinExp) { break }
}
$gold1 = [int]$p.gold
$goldDelta = $gold1 - $gold0
$expDelta = [int]$p.exp - [int]$exp0
Write-Host ("[C] finish_quest 后 已接={0}（任务 {1} 已移除={2}）" -f ($taken2 -join ','), $QuestId, $removed)
Write-Host ("[D] 奖励: gold {0}->{1} (delta={2}, 期望 >= {3})；exp {4}->{5} (delta={6}, 期望 >= {7})；level {8}->{9}" -f `
    $gold0, $gold1, $goldDelta, $MinGold, $exp0, $p.exp, $expDelta, $MinExp, $lv0, $p.level)

# C/D 以 A 为前提：没接上时「日志里本来就没有它」不算交付成功（空真 PASS 会把接取失败洗绿）
$okA = [bool]$accepted
$okC = ($okA -and $removed)
$okD = ($okA -and ($goldDelta -ge $MinGold) -and ($expDelta -ge $MinExp))
Write-Host ("VERDICT accept={0} state_flip={1} reward_gold={2}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okC -and $okD)) { exit 5 }

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
