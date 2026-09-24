# l5g2_quest_carry_items.ps1 — ④ 任务闭环（**带目标的任务类型**）：CarryItems 任务 接取 → 交付 → 奖励
#
# 为什么单开一条：l5g 只覆盖「目标段为空」的任务（空进度恒真，走到交付 NPC 就能交）。
# 本次补的是**带目标的任务**里唯一能确定性跑通的一类——`[@CarryItems]`：
#   * 服务端 `take_quest_carry_items`（`world/quest.rs:123`）在**接取时**就把携带物放进**任务格**；
#   * 进度由 `count_quest_item_by_index`（按任务格计数）判定 → 接取后进度即满 → 可直接交付。
#   （ItemTasks/KillTasks 需要真的拿到任务物品/打怪，属另一类，本夹具不碰——不假装覆盖。）
#
# 选任务（都能对着数据核）：`Envir/Quests/<id>.txt` 有非空 [@CarryItems] + [@GoldReward]>0，
# 且 quest_infos.required_quest=0、required_min_level≤角色等级、required_class=31（全职业）。
# 池（2026-09-24 扫文件 + DB 得出）：29,34,50,57,77,83,95（134 要 37 级，排除）。
#
# 判据（取状态）：
#   A) 接取后 quest_probe.taken 含该任务（轮询 ≤15s）
#   B) 到交付 NPC 处 finish_quest → taken 不再含它（轮询 ≤20s）——**这一步本身就证明携带物进度成立**：
#      服务端 `is_progress_complete()` 是「所有进度 current>=target」，携带物没进任务格就交不了。
#   C) 奖励：gold 与 exp delta 同时为正（轮询 ≤20s）
#   D) 负向对照（门槛）：`required_quest` 未满足的任务必须**接不上**（taken 不得增加）——
#      这是「任务门槛」这条原版语义的门禁，避免把"能接"当默认。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    # 0 = 自动选（池内挑本角色未完成的第一个）
    [int]$QuestId = 0,
    # 负向对照用的任务（required_quest 未满足的那个）；0 = 自动从数据里挑
    [int]$GateQuestId = 0,
    [string]$FinishMap = '',
    [int]$FinishX = -1,
    [int]$FinishY = -1
)

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5g2_quest_carry_items' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁
# （PowerShell 的 finally 在 exit 下也会执行——实测 -File 与会话内 & script.ps1 两种调用都成立），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
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
function Exp { [int](Rpc 'bag_probe').exp }
function Db([string]$sql) { (& python "$wt\tools\acceptance\dbq.py" $sql) }

# [QUESTS] 段：正数=可接、负数=可交（C# NPCScript.ParseQuests），按符号精确匹配
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

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }

# 角色名 / 已完成集合
$charName = (Db "select name from characters where account_username='$User'" | Select-Object -First 1)
if ("$charName" -match "^\( '(.*)',? \)$") { $charName = $Matches[1] }
$charName = "$charName".Trim()
$completed = @{}
foreach ($r in (Db "select quest_index from completed_quests where character_name='$charName'")) {
    if ("$r" -match '(\d+)') { $completed[[int]$Matches[1]] = $true }
}

$Pool = @(29, 34, 50, 57, 77, 83, 95)
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5g2_client.log" -RedirectStandardError "$acc\l5g2_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 自动选任务：**登录后用实时探针选**（taken/completed 都取客户端权威状态）。
# 为什么不在启动前用 DB 选：上一轮就是那样——刚交完的任务 `completed_quests` 要等角色
# 落库（登出/自动存档）才可见，于是又选中了刚交掉的那个，accept 被服务端正确拒绝（"该任务已完成"），
# 报成 accept=FAIL 的假红。
if ($QuestId -eq 0) {
    $live = @{}
    foreach ($e in (Rpc 'quest_probe').taken) { $live[[int]$e.id] = [bool]$e.completed }
    foreach ($q in $Pool) {
        if ($live.ContainsKey($q)) { continue }         # 已接（含已接未交的残留）
        if ($completed.ContainsKey($q)) { continue }     # 库里已完成
        $QuestId = $q; break
    }
    if ($QuestId -eq 0) { Write-Host 'FAIL: CarryItems 候选池已耗尽（全部完成）——按头部注释口径扫新任务扩池'; exit 4 }
    Write-Host ("[0] 自动选任务: quest {0}（角色 {1}；池 {2}；实时 taken={3}）" -f `
        $QuestId, $charName, ($Pool -join ','), (($live.Keys | Sort-Object) -join ','))
}
$accNpc = NpcLinkFor $QuestId $false
if (-not $accNpc) { Write-Host ("FAIL: 数据里找不到任务 {0} 的**接取** NPC（[QUESTS] 段无 +{0}）" -f $QuestId); exit 4 }
if ($FinishMap -eq '') {
    $fin = NpcLinkFor $QuestId $true
    if (-not $fin) { Write-Host ("FAIL: 数据里找不到任务 {0} 的交付 NPC（[QUESTS] 段无 -{0}）" -f $QuestId); exit 4 }
    $FinishMap = $fin.map; $FinishX = $fin.x; $FinishY = $fin.y
    Write-Host ("[0] 接取点 {0} @ {1}({2},{3})；交付点 {4} @ {5}({6},{7})" -f `
        $accNpc.name, $accNpc.map, $accNpc.x, $accNpc.y, $fin.name, $fin.map, $fin.x, $fin.y)
}

# 负向对照任务：required_quest 未满足的那个（默认从数据里挑第一个满足"有前置且前置未完成"的）
if ($GateQuestId -eq 0) {
    $lvl = (Db "select level from characters where name='$charName'" | Select-Object -First 1)
    if ("$lvl" -match '(\d+)') { $lvl = [int]$Matches[1] } else { $lvl = 1 }
    foreach ($r in (Db "select idx, required_quest from quest_infos where required_quest>0 and required_min_level<=$lvl and required_class=31 order by idx")) {
        if ($r -match '^\((\d+), (\d+)\)$') {
            $qid = [int]$Matches[1]; $prq = [int]$Matches[2]
            if ($completed.ContainsKey($prq)) { continue }
            # 必须**有接取 NPC** 才能做这条对照（第一版挑到 quest 2，[QUESTS] 里没有 +2 → 只能跳过）
            if (NpcLinkFor $qid $false) { $GateQuestId = $qid; break }
        }
    }
}

# ---- D) 负向对照：门槛任务必须接不上（先跑，避免被主流程的 taken 变化干扰）----
$okD = $true
$gateRan = $false
if ($GateQuestId -gt 0) {
    $gNpc = NpcLinkFor $GateQuestId $false
    if ($gNpc) {
        $gateRan = $true
        Rpc 'chat' @{ message = "@mapmove $($gNpc.map) $($gNpc.x) $($gNpc.y)" } | Out-Null
        foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($gNpc.map)") { break } }
        $beforeGate = Taken
        $r = Rpc 'accept_quest' @{ npc_index = $gNpc.npc_index; quest_index = $GateQuestId }
        Start-Sleep 3
        $afterGate = Taken
        $entered = $afterGate -contains $GateQuestId
        Write-Host ("[D] 门槛对照 quest {0}（前置未完成）：accept 回执={1}；taken {2} -> {3}；是否被接上={4}" -f `
            $GateQuestId, ($r | ConvertTo-Json -Compress), ($beforeGate -join ','), ($afterGate -join ','), $entered)
        $okD = -not $entered
    } else {
        Write-Host ("[D] 门槛对照任务 {0} 找不到接取 NPC——跳过（记 N/A）" -f $GateQuestId)
    }
} else {
    Write-Host '[D] 数据里没有「有前置且前置未完成」的可用对照任务——跳过（记 N/A）'
}

# ---- A) 接取 ----
Rpc 'chat' @{ message = "@mapmove $($accNpc.map) $($accNpc.x) $($accNpc.y)" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$($accNpc.map)") { break } }
$taken0 = Taken
$gold0 = Gold; $exp0 = Exp
$rA = Rpc 'accept_quest' @{ npc_index = $accNpc.npc_index; quest_index = $QuestId }
$taken1 = $taken0
foreach ($i in 1..15) {
    Start-Sleep 1
    $taken1 = Taken
    if ($taken1 -contains $QuestId) { break }
}
$okA = $taken1 -contains $QuestId
Write-Host ("[A] accept_quest -> {0}；taken {1} -> {2}（含 {3} = {4}）" -f `
    ($rA | ConvertTo-Json -Compress), ($taken0 -join ','), ($taken1 -join ','), $QuestId, $okA)

# ---- B/C) 交付 + 奖励 ----
Rpc 'chat' @{ message = "@mapmove $FinishMap $FinishX $FinishY" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; if ("$((Rpc 'state').map)" -eq "$FinishMap") { break } }
$taken2 = $taken1
$rB = Rpc 'finish_quest' @{ quest_index = $QuestId; selected_item_index = -1 }
foreach ($i in 1..20) {
    Start-Sleep 1
    $taken2 = Taken
    if (-not ($taken2 -contains $QuestId)) { break }
}
$okB = -not ($taken2 -contains $QuestId)
$gold1 = $gold0; $exp1 = $exp0
foreach ($i in 1..20) {
    Start-Sleep 1
    $gold1 = Gold; $exp1 = Exp
    if (($gold1 -gt $gold0) -and ($exp1 -gt $exp0)) { break }
}
$okC = (($gold1 -gt $gold0) -and ($exp1 -gt $exp0))
Write-Host ("[B] finish_quest -> {0}；taken {1} -> {2}（已移除={3}）" -f ($rB | ConvertTo-Json -Compress), ($taken1 -join ','), ($taken2 -join ','), $okB)
Write-Host ("[C] 奖励：gold {0}->{1}（+{2}）；exp {3}->{4}（+{5}）" -f $gold0, $gold1, ($gold1 - $gold0), $exp0, $exp1, ($exp1 - $exp0))

# 注意：没真跑过的对照必须记 N/A，不能记 PASS（第一版就是这样把"跳过"印成了 PASS——判据不许洗白）
$gateVerdict = if ($gateRan) { $(if ($okD) { 'PASS' } else { 'FAIL' }) } else { 'N/A' }
Write-Host ("VERDICT accept={0} state_flip={1} reward={2} gate_block={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $gateVerdict)
if (-not ($okA -and $okB -and $okC -and ($okD -or -not $gateRan))) { exit 5 }

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
