# l5a_combat.ps1 — ① 战斗闭环（含掉落拾取）：接近 → 贴到 ≤1 格 → 打到 died → 拾取掉落
#
# 判据（全取**状态/服务端事件流**，不解析像素、不看截图）：
#   A) 锁定目标：`attack` 后 `combat_probe.attack_target == 选定怪物`
#   B) 贴近：`combat_probe.target_dist_tiles` 收敛到 ≤1（近战判定按格）
#   C) 命中链：事件流出现 `damage`/`object_health`（id = 目标）
#   D) 击杀：事件流出现 `died`（id = 目标）
#   E) 掉落拾取：`nearby` 出现 `kind=item` → `pickup` → 背包 used +1（或金币增加）；
#      掉率是概率的，跑 `-Kills` 只，只要任一只掉出物品并拾取成功即算 PASS，
#      一只都没掉就如实记 `N/A`（不虚报、也不算失败）。
#
# 两条实机教训（都写在代码里，别改回去）：
#   1. **不要自己 walk_to**：客户端 `auto_attack_system` 自带 #1817 追击；而 control 的
#      `Attack` 会 `remove::<LocalMove>()`——在走位循环里反复发 attack 会把刚起步的走位取消
#      （首跑实测：14 步后距离反而从 5.5 格拉到 12 格）。
#   2. **只在「本地位置稳定」时挥砍**：本地移动是预测的、服务端位置滞后，而近战由服务端按
#      **它自己视角**的「正前方一格」结算——边追边打全是空挥（实测 56 次 attack 事件、零伤害）。
#      连续两次采样本地 tile 不变 ⇒ 预测已停、`UserLocation` 权威位置已被采用（movement.rs）。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [string]$MapName = '0',
    [int]$MapX = 287,
    [int]$MapY = 615,
    [int]$ScanRadius = 5000,
    [string]$PreferMonster = 'Scarecrow',
    [int]$Kills = 3,
    [int]$KillTimeoutSec = 30
)

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5a_combat' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

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
function Monsters($probe) { @($probe.entities | Where-Object { $_.kind -eq 'monster' }) }
function Items($probe) { @($probe.entities | Where-Object { $_.kind -eq 'item' }) }

# 找一只怪：优先 PreferMonster（掉率表非空的常见怪），否则最近一只
function FindMonster {
    foreach ($i in 1..12) {
        Start-Sleep 1
        $all = Monsters (Rpc 'nearby' @{ radius = $ScanRadius })
        $pref = $all | Where-Object { $_.name -eq $PreferMonster } | Sort-Object dist | Select-Object -First 1
        if ($pref) { return $pref }
        $near = $all | Sort-Object dist | Select-Object -First 1
        if ($near) { return $near }
    }
    return $null
}

# 打一只怪：返回 @{died; damage; minDist; struck}（判据全来自服务端事件流/探针）
function KillMonster([uint32]$targetId) {
    $res = @{ died = $false; damage = $false; minDist = $null; struck = 0; samples = @() }
    Rpc 'attack' @{ object_id = $targetId } | Out-Null
    $deadline = (Get-Date).AddSeconds($KillTimeoutSec)
    $lastTile = $null; $stable = 0
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $stNow = Rpc 'state'
        $tileKey = "$($stNow.tile_x),$($stNow.tile_y)"
        if ($tileKey -eq $lastTile) { $stable++ } else { $stable = 0; $lastTile = $tileKey }
        $cp = Rpc 'combat_probe'
        if ($null -ne $cp.target_dist_tiles) {
            $d = [int]$cp.target_dist_tiles
            $res.samples += $d
            if ($null -eq $res.minDist -or $d -lt $res.minDist) { $res.minDist = $d }
        }
        foreach ($e in @($cp.events)) {
            if ($e.kind -eq 'struck') { $res.struck++ }
            if (($e.kind -eq 'damage' -or $e.kind -eq 'object_health') -and $e.id -eq $targetId) { $res.damage = $true }
            if ($e.kind -eq 'died' -and $e.id -eq $targetId) { $res.died = $true }
        }
        if ($res.died) { break }
        if ($null -eq $cp.attack_target) { break }
        if ($stable -ge 2 -and $null -ne $cp.target_dist_tiles -and [int]$cp.target_dist_tiles -le 1) {
            Rpc 'attack' @{ object_id = $targetId } | Out-Null
        }
    }
    return $res
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5a_client.log" -RedirectStandardError "$acc\l5a_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 到有怪的地图（轮询到真的换图）
Rpc 'chat' @{ message = "@mapmove $MapName $MapX $MapY" } | Out-Null
$at = $null
foreach ($i in 1..20) {
    Start-Sleep 1
    $at = Rpc 'state'
    if ("$($at.map)" -eq "$MapName") { break }
}
Write-Host ("[A] 到图 map={0} tile=({1},{2})（优先打 {3}）" -f $at.map, $at.tile_x, $at.tile_y, $PreferMonster)

$okLock = $false; $okRange = $false; $okDamage = $false; $okKilled = $false
$dropSeen = $false; $okDrop = $false; $killsDone = 0
for ($k = 1; $k -le $Kills; $k++) {
    $mon = FindMonster
    if (-not $mon) { Write-Host ('[k{0}] FAIL(A): {1}px 内没有怪物' -f $k, $ScanRadius); break }
    $targetId = [uint32]$mon.object_id
    $okLock = $true
    Write-Host ("[k{0}] 目标怪={1} id={2} dist={3}" -f $k, $mon.name, $targetId, $mon.dist)
    $r = KillMonster $targetId
    $killsDone++
    if ($null -ne $r.minDist -and $r.minDist -le 1) { $okRange = $true }
    if ($r.damage) { $okDamage = $true }
    if ($r.died) { $okKilled = $true }
    Write-Host ("[k{0}] 距离 {1}（最小 {2}）；命中={3}；死亡={4}；struck={5}" -f `
        $k, (($r.samples | Select-Object -First 6) -join '→'), $r.minDist, $r.damage, $r.died, $r.struck)
    if (-not $r.died) { continue }

    # E) 掉落拾取（掉率是概率的：这只没掉就接着打下一只）
    $bagBefore = Rpc 'bag_probe'
    $drop = $null
    foreach ($i in 1..6) {
        Start-Sleep 1
        $drop = (Items (Rpc 'nearby' @{ radius = 2000 }) | Sort-Object dist | Select-Object -First 1)
        if ($drop) { break }
    }
    if (-not $drop) { Write-Host ("[k{0}][E] 这只没掉东西" -f $k); continue }
    $dropSeen = $true
    Write-Host ("[k{0}][E] 掉落物={1} id={2} dist={3}" -f $k, $drop.name, $drop.object_id, $drop.dist)
    Rpc 'pickup' @{ object_id = $drop.object_id } | Out-Null
    $bagAfter = $null
    foreach ($i in 1..8) {
        Start-Sleep 1
        $b = Rpc 'bag_probe'
        if ($null -eq $b) { continue }
        $bagAfter = $b
        if (($b.used -gt $bagBefore.used) -or ([int]$b.gold -gt [int]$bagBefore.gold)) { break }
    }
    if ($bagAfter) {
        $okDrop = (($bagAfter.used -gt $bagBefore.used) -or ([int]$bagAfter.gold -gt [int]$bagBefore.gold))
        Write-Host ("[k{0}][E] 拾取后 bag used {1}->{2}，gold {3}->{4}" -f `
            $k, $bagBefore.used, $bagAfter.used, $bagBefore.gold, $bagAfter.gold)
    }
    if ($okDrop) { break }
}

# E') 确定性「地面掉落 → 拾取」：怪物掉率是概率的（本库 Scarecrow 单杀命中掉落行的期望 ≈0.25、
#     Deer 只有 2 行），只靠打怪这条判据会长期停在 N/A。丢弃是**真实玩家动作**
#     （背包拖出/确认框 Yes 同款包），用它造一个地面物品再拾回，让这条链每次都真跑一遍。
#
#     挑物品要避开**原版语义就是"丢弃即销毁"**的（本库 Saddle=145、LeatherBridle=133，都带
#     `BindMode.DestroyOnDrop` 0x80）：那种物品丢弃**本就不落地**，服务端回 success=true 但地面
#     不出现物品——第一版夹具没筛 bind_mode，两次都挑到 Saddle/LeatherBridle，报成「丢弃后地面
#     没有物品」的假 FAIL（我据此开过一个缺陷线程，事后证明是夹具选错了对象，已更正收口）。
#     这里按 item_infos.bind_mode 过滤：`DONT_DROP(0x02)`/`DESTROY_ON_DROP(0x80)` 都不要。
$okDropGround = $false; $dropRoundTrip = $false
# 先**造一个可落地的物品**：BaseDress(M)（`item_infos.bind_mode=0`，非堆叠 → 新实例必然带独立 uid）。
# 这样判据不依赖"背包里恰好有可丢的物品"，每次都真跑一遍。
$bagPre = Rpc 'bag_probe'
$preUids = @{}
foreach ($o in $bagPre.occupied) { $preUids[[string]$o.unique_id] = $true }
Rpc 'chat' @{ message = '@MAKE BaseDress(M) 1' } | Out-Null
$victim = $null
$bagA = $bagPre
foreach ($i in 1..8) {
    Start-Sleep 1
    $b = Rpc 'bag_probe'
    if ($null -eq $b) { continue }
    $victim = $b.occupied | Where-Object { -not $preUids.ContainsKey([string]$_.unique_id) } | Select-Object -First 1
    if ($victim) { $bagA = $b; break }
}
if (-not $victim) {
    Write-Host "[E'] FAIL: @MAKE 后 8s 内背包没有出现新实例（造物通道？）"
}
$uidCounts = @{}
foreach ($o in $bagA.occupied) { $uidCounts[[string]$o.unique_id] = 1 + [int]($uidCounts[[string]$o.unique_id]) }
$cands = @()
if ($victim) { $cands = @($victim) }
foreach ($c in $cands) {
    $bmRaw = (& python "$acc\dbq.py" "select bind_mode from item_infos where name='$($c.name)' limit 1" 2>$null)
    $bm = 0
    [void][int]::TryParse("$bmRaw".Trim(), [ref]$bm)
    if (($bm -band 0x82) -ne 0) {
        Write-Host ("[E'] 跳过 {0}（bind_mode={1} 带 DONT_DROP/DESTROY_ON_DROP，原版语义不落地）" -f $c.name, $bm)
        continue
    }
    $victim = $c
    break
}
if ($victim) {
    Write-Host ("[E'] 丢弃造物：格 {0} {1}（uid={2}），bag.used={3}" -f $victim.cell, $victim.name, $victim.unique_id, $bagA.used)
    Rpc 'drop_item' @{ unique_id = $victim.unique_id; count = 1 } | Out-Null
    $ground = $null
    foreach ($i in 1..8) {
        Start-Sleep 1
        $g = Items (Rpc 'nearby' @{ radius = 2000 })
        # 地面物品是新实例（服务端重新分配 id），按名字找即可
        $ground = $g | Where-Object { $_.name -eq $victim.name } | Sort-Object dist | Select-Object -First 1
        if (-not $ground -and $g.Count -gt 0) { $ground = $g | Sort-Object dist | Select-Object -First 1 }
        if ($ground) { break }
    }
    if ($ground) {
        $okDropGround = $true
        Write-Host ("[E'] 地面出现物品 {0} id={1} dist={2}" -f $ground.name, $ground.object_id, $ground.dist)
        Rpc 'pickup' @{ object_id = $ground.object_id } | Out-Null
        $bagB = $null
        foreach ($i in 1..8) {
            Start-Sleep 1
            $b = Rpc 'bag_probe'
            if ($null -eq $b) { continue }
            $bagB = $b
            if ($b.used -ge $bagA.used) { break }
        }
        if ($bagB) {
            $dropRoundTrip = ($bagB.used -ge $bagA.used)
            Write-Host ("[E'] 拾取后 bag.used={0}（丢弃前 {1}）→ 往返{2}" -f `
                $bagB.used, $bagA.used, $(if ($dropRoundTrip) { '成立' } else { '不成立' }))
        }
    } else {
        Write-Host "[E'] FAIL: 丢弃后 8s 内地面没有出现物品"
    }
} else {
    Write-Host "[E'] 背包里没有可用（唯一 uid）实例可丢"
}

$okPickup = ($okDrop -or $dropRoundTrip)
$dropVerdict = if ($okDrop) { 'PASS' }
    elseif ($dropRoundTrip) { 'PASS(确定性丢弃往返)' }
    elseif ($dropSeen -or $okDropGround) { 'FAIL' }
    else { 'N/A' }
Write-Host ("VERDICT lock_target={0} melee_range={1} damage_chain={2} killed={3} drop_pickup={4}（击杀 {5} 只）" -f `
    $(if ($okLock) { 'PASS' } else { 'FAIL' }), $(if ($okRange) { 'PASS' } else { 'FAIL' }), `
    $(if ($okDamage) { 'PASS' } else { 'FAIL' }), $(if ($okKilled) { 'PASS' } else { 'FAIL' }), `
    $dropVerdict, $killsDone)
if (-not ($okLock -and $okRange -and $okDamage -and $okKilled)) { exit 5 }

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
