#Requires -Version 5.1
<#
.SYNOPSIS
  地面物品拾取的**用户路径**实机夹具（owner 反馈「血瓶怎么捡起来？」）。

.DESCRIPTION
  目的：把「地面上的物品到底怎么捡」按用户真实入口逐条验证，而不是只验控制 RPC。
  三条入口（判据都取**状态真值**，不取日志）：
    A) **左键点地面物品**：`nearby` 给出该物品的视口坐标 `vp` → 控制 RPC `click {x,y}`
       （走的是与真人同一套左键处理：`player_control` 的世界左键 → 物品命中 → PickUp）。
    B) **拾取键**：控制 RPC `key {key:'tab'}`（默认绑定「拾取」= Tab，见
       `keyboard_layout::default_bindings`；C# 同为 `KeybindOptions.Pickup`）。
    C) **控制 RPC**：`pickup {object_id}`（既有夹具用的确定性入口，用来区分
       「UI 链路坏」还是「服务端链路坏」）。
  判据：拾取成功 = 该地面物品从 `nearby` 消失 **且** 背包里该物品的件数回到掉落前。

  前置：本夹具**自造**掉落——用 `@MAKE (HP)DrugSmall 1` 造一瓶**血瓶**再丢（owner 的原话就是「血瓶」）。

  ⚠️ 选物踩坑（2026-09-27 实测，别再踩）：第一版图省事丢了背包里的 `Saddle`，结果连跑 4 轮
  「服务端回执 success=true、背包里该 uid 已扣、客户端却一条 `📦 地面物品` 都没有」。
  根因不是丢弃/广播坏，而是 **`Saddle` 的 `bind_mode=145` 含 `DestroyOnDrop(0x80)`**：
  C#/服务端对这种物品**丢弃即销毁、本来就不落地**（`item.rs` 的 destroy_on_drop 分支同样回 success=true）。
  换成 `(HP)DrugSmall`（`bind_mode=0`）后地面物品正常工作。
  ⇒ 夹具必须丢**可落地**物品，否则量的是绑定语义而不是拾取链路。

  退出码：0 = A 或 B 通过（用户路径可用）；1 = A/B 都失败但 C 通过（= UI 链路缺陷，
          正是本夹具要抓的东西）；2 = 前置失败（未进场/探针不可用）；3 = 前置不成立（造不出掉落）。
  实机资源（客户端 + e2e 账号）必须走 `e2e_lock.ps1` 串行，否则会撞 `result=4` 假红。
#>
param(
    [string]$ClientHome = '',
    [int]$ControlPort = 9049,
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$Tag = 'pickupui'
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5za_pickup_ui'
# 唯一进程名（多 agent 并行时不要按公共名清进程）
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5za_client.exe'
$log = "$PSScriptRoot\l5za_pickup_ui.$Tag.log"

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5za_pickup_ui' -TimeoutSec 1800)) {
    Write-Host 'FAIL(2): 等 e2e 锁超时'
    exit 2
}

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 2000; $c.SendTimeout = 2000
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 2000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}

function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='l5za_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

$bagCountOf = {
    param($probe, $name)
    ($probe.occupied | Where-Object { $_.name -eq $name } | Measure-Object -Property count -Sum).Sum
}
$groundItem = {
    param($near)
    $near.entities | Where-Object { $_.kind -eq 'item' } | Select-Object -First 1
}

Get-CimInstance Win32_Process -Filter "Name='l5za_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
if (Test-Path $log) { [System.IO.File]::Delete($log) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, `
        '--control-port', "$ControlPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput "$PSScriptRoot\l5za_pickup_ui.$Tag.out" -RedirectStandardError $log | Out-Null

$st = $null
foreach ($i in 1..90) {
    Start-Sleep 1
    $st = Rpc 'state'
    if ($null -ne $st.tile_x) { break }
}
if ($null -eq $st -or $null -eq $st.tile_x) {
    $why = (Select-String -Path $log -Pattern '登录失败|already online' -EA SilentlyContinue | Select-Object -Last 1).Line
    Write-Host ("FAIL(2): 未进场 - " + $why)
    Stop-Client; Exit-E2eLock; exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# ---- 造掉落（自造，不依赖怪物掉率） --------------------------------------
$POTION = '(HP)DrugSmall'
$bag0 = Rpc 'bag_probe'
if ($null -eq $bag0.occupied) { Write-Host 'FAIL(2): bag_probe 不可用'; Stop-Client; Exit-E2eLock; exit 2 }
# 先看背包里有没有血瓶；没有就 @MAKE 一瓶（bind_mode=0，可落地）
$victim = $bag0.occupied | Where-Object { $_.name -eq $POTION } | Select-Object -First 1
if ($null -eq $victim) {
    Rpc 'chat' @{ message = "@MAKE $POTION 1" } | Out-Null
    Start-Sleep -Seconds 2
    $bag0 = Rpc 'bag_probe'
    $victim = $bag0.occupied | Where-Object { $_.name -eq $POTION } | Select-Object -First 1
}
if ($null -eq $victim) { Write-Host ("FAIL(3): 造不出可丢的血瓶（@MAKE {0} 未进包）" -f $POTION); Stop-Client; Exit-E2eLock; exit 3 }
$name = $victim.name
$before = & $bagCountOf $bag0 $name
Write-Host ("掉落物 = {0} x{1}（格 {2}，掉落前背包计数 {3}）" -f $name, $victim.count, $victim.cell, $before)
Rpc 'drop_item' @{ unique_id = [int64]$victim.unique_id; count = 1 } | Out-Null
Start-Sleep -Seconds 2

$near = Rpc 'nearby' @{ radius = 4000 }
$item = & $groundItem $near
$broadcast_missing = $false
if ($null -eq $item) {
    # 判别实验（这一步把两个完全不同的根因分开）：
    #   ① 掉落没落地（服务端没建 GroundItem）→ 重进图也看不到；
    #   ② 掉落落地了，但**丢弃时的 ObjectItem 广播没送到丢弃者**→ 重进图随地图同步补发就能看到。
    # 重进图走 `@mapmove <同图> <同坐标>`（服务端 MAPMOVE 分支），只用于触发一次地图同步。
    Write-Host 'WARN(1): 丢弃后地面物品**没有出现在本客户端**（客户端未收到 ObjectItem）→ 换图往返复核'
    # 必须**真的换图**才会重发地图对象（同图 @mapmove 不触发同步，实测）：先切到 D002 再切回来。
    Rpc 'chat' @{ message = '@mapmove D002 174 217' } | Out-Null
    Start-Sleep -Seconds 5
    $mid = Rpc 'state'
    Write-Host ("    （换图核对：map={0}）" -f $mid.map)
    Rpc 'chat' @{ message = "@mapmove $($st.map) $($st.tile_x) $($st.tile_y)" } | Out-Null
    Start-Sleep -Seconds 6
    $back = Rpc 'state'
    Write-Host ("    （换回核对：map={0} tile=({1},{2})）" -f $back.map, $back.tile_x, $back.tile_y)
    $near = Rpc 'nearby' @{ radius = 4000 }
    $item = & $groundItem $near
    if ($null -eq $item) {
        Write-Host 'FAIL(3): 掉落没有落地（重进图后 nearby 仍无 item）'
        Stop-Client; Exit-E2eLock; exit 3
    }
    $broadcast_missing = $true
    Write-Host '⇒ 根因定性：地面物品**存在**（重进图后随地图同步可见），坏的是**丢弃时那条广播**没送到本客户端'
}
Write-Host ("地面物品 object_id={0} name={1} @ ({2},{3}) vp={4}" -f $item.object_id, $item.name, $item.x, $item.y, ($item.vp | ConvertTo-Json -Compress))

function Test-Picked {
    param([string]$name, [int]$objectId, [int]$before)
    Start-Sleep -Seconds 2
    $near2 = Rpc 'nearby' @{ radius = 4000 }
    $still = $near2.entities | Where-Object { $_.kind -eq 'item' -and $_.object_id -eq $objectId }
    $bag2 = Rpc 'bag_probe'
    $after = & $bagCountOf $bag2 $name
    [pscustomobject]@{
        gone = ($null -eq $still)
        after = $after
        ok = (($null -eq $still) -and ($after -ge $before))
        hits = $null
    }
}

# ---- A) 左键点地面物品（用户路径） --------------------------------------
$vp = $item.vp
$a = $null
if ($null -ne $vp) {
    $hit = Rpc 'click' @{ x = [double]$vp.x; y = [double]$vp.y }
    $a = Test-Picked -name $name -objectId $item.object_id -before $before
    Write-Host ("[A] 左键点地面物品 → {0}（点击命中栈 {1}；拾取后背包计数 {2}）" -f $(if ($a.ok) { 'PASS' } else { 'FAIL' }), ($hit.hits | ConvertTo-Json -Compress), $a.after)
} else {
    Write-Host '[A] 左键点地面物品 → SKIP（nearby 没给 vp 视口坐标）'
}
if ($a -and $a.ok -and -not $broadcast_missing) { Write-Host '=== 全部 PASS（用户路径 A 可用）==='; Stop-Client; Exit-E2eLock; exit 0 }
if ($a -and $a.ok -and $broadcast_missing) {
    Write-Host 'FAIL(1): 拾取链路可用，但**玩家丢弃的物品在丢弃瞬间对客户端不可见**（重进图才出现）⇒ 用户路径根因'
    Stop-Client; Exit-E2eLock; exit 1
}

# ---- B) 拾取键（Tab） ---------------------------------------------------
$nearB = Rpc 'nearby' @{ radius = 4000 }
$itemB = & $groundItem $nearB
if ($null -ne $itemB) {
    $null = Rpc 'key' @{ key = 'tab' }
    $b = Test-Picked -name $name -objectId $itemB.object_id -before $before
    Write-Host ("[B] 拾取键 Tab → {0}（拾取后背包计数 {1}）" -f $(if ($b.ok) { 'PASS' } else { 'FAIL' }), $b.after)
    if ($b.ok -and -not $broadcast_missing) { Write-Host '=== 全部 PASS（用户路径 B 可用）==='; Stop-Client; Exit-E2eLock; exit 0 }
    if ($b.ok -and $broadcast_missing) {
        Write-Host 'FAIL(1): 拾取链路可用（Tab），但**丢弃瞬间对客户端不可见**（重进图才出现）⇒ 用户路径根因'
        Stop-Client; Exit-E2eLock; exit 1
    }
} else {
    Write-Host '[B] 拾取键 Tab → SKIP（物品已被上一步拾走）'
}

# ---- C) 控制 RPC（区分 UI 链路 vs 服务端链路） ---------------------------
$nearC = Rpc 'nearby' @{ radius = 4000 }
$itemC = & $groundItem $nearC
if ($null -ne $itemC) {
    Rpc 'pickup' @{ object_id = $itemC.object_id } | Out-Null
    $c = Test-Picked -name $name -objectId $itemC.object_id -before $before
    Write-Host ("[C] 控制 RPC pickup → {0}（拾取后背包计数 {1}）" -f $(if ($c.ok) { 'PASS' } else { 'FAIL' }), $c.after)
    if ($broadcast_missing -and ($c.ok -or ($a -and $a.ok) -or ($b -and $b.ok))) {
        Write-Host 'FAIL(1): 拾取链路本身可用，但**玩家丢弃的物品在丢弃瞬间对客户端不可见**（重进图才出现）'
        Write-Host '          ⇒ 用户路径上「地上有个东西却点不到/Tab 也拾不到」的根因就是这条广播缺失'
        Stop-Client; Exit-E2eLock; exit 1
    }
    if ($c.ok) {
        Write-Host 'FAIL(1): 用户路径 A/B 都捡不起来，但控制 RPC 可以 ⇒ **UI 链路缺陷**（正是 owner 反馈的现象）'
        Stop-Client; Exit-E2eLock; exit 1
    }
} else {
    Write-Host '[C] 控制 RPC pickup → SKIP（物品已消失）'
}

Write-Host 'FAIL(1): 三条入口都没能把这件地面物品捡回来（服务端/客户端链路都待查）'
Stop-Client; Exit-E2eLock; exit 1
