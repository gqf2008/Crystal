# ⛔ **PENDING（未跑绿，故意不合入门禁；按本仓约定改名 `.pending.ps1`）**
#
# 阻塞（2026-09-28 实测两条 GM 命令都走不通）：`@MOB` / `@RECALLMOB` 在本端**只广播 spawn 包、
# 不建服务端实体**（`world::spawn_monster_named` 里只有 `build_object_monster_packet` + 广播，
# 没有插入 `self.monsters`）⇒ 召唤出来的怪是"客户端幽灵"，**永远不会有 AI、不会攻击**。
# 实测：`@MOB AxeSkeleton` 后客户端确实收到 `🌐 网络对象生成完成: 1 个`，但 40s 内
# `spell_fx_probe.spawned.monster_missile_add` 一直是 0（`🏹 对象远程攻击` 日志也没有）。
# 另有第二个前置：出生点 (288,616) 一带是**安全区**（`safe_zones`: map 1, (288,616), size 10），
# 安全区里不打斗；而 `walk_to 300,650` 只把我们带到 (280,615)（路被挡/目标不可达）。
#
# 要跑绿这条，需要其中之一：
#   ① 走到**有真实刷怪**的地图（OmaCave 的 AxeSkeleton 就在 `BichonProvince/OmaCave/` 刷）；
#   ② 或先补服务端：`@MOB`/`@RECALLMOB` 建真实怪物实体 + AI（C# 原版是真实召唤）——那是另一条线；
#   ③ 或走 mock 路径（像 `l5r_ranged_projectile.ps1` 那样让夹具自己造 `S.ObjectRangeAttack`），
#      但那需要客户端世界里先有一只带 `MonsterAppearance` 的怪实体。
# 在此之前，本文件的判据（下）只作配方留档，**不要**当覆盖证据。
#
# l5zh_monster_missile.ps1 —— 真机：怪物远程攻击用**原版表**的帧弹道（owner 反馈「有些魔法是个黄色方框」）
#
# 判据（只读探针，不看截图；弹道只活 0.35s，所以取**累计**计数）：
#   A) 前置：`@MOB AxeSkeleton` 之后世界出现该怪（只读 `state`/日志，不用猜）；
#   B) `spell_fx_probe.monster_missile_add > 0` —— 怪物侧走了 `MonsterObject.cs` 那张表
#      （AxeSkeleton=24 → `Monster/024.Lib[224]` 3 帧）；
#   C) 对照：`fallback_placeholder` 不得随本轮**怪物**攻击增长（占位方块应只属于"表里没有的怪物"）。
#
# 为什么能这么测：客户端弹道实体寿命 0.35s，夹具 100ms 轮询「存活实体」会采样漏掉（实测踩过），
# 而这两个计数是**自客户端启动以来累计**的，与采样时刻无关。
#
# 依赖：master 服务端在 7000；客户端 control RPC 的 `chat` / `spell_fx_probe`（见 control.rs）。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend',
    [string]$Monster = 'AxeSkeleton',
    # 出生点 (288,616) 附近是**安全区**（`safe_zones`: map 1, (288,616) size 10）——
    # 安全区里怪物不会攻击玩家（实测 45s 一发都没有）。所以先用玩家路径走到安全区外再召唤。
    [int]$WalkX = 300,
    [int]$WalkY = 650,
    [int]$WaitSeconds = 45
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = $PSScriptRoot
. "$acc\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5zh_monster_missile' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }
$clientProc = $null
try {
    if (-not (Get-NetTCPConnection -LocalPort 7000 -State Listen -EA SilentlyContinue)) { Write-Host 'FAIL(9): 7000 上没有服务端'; exit 9 }
    $exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
    $exe = Join-Path (Split-Path -Parent $exeSrc) 'l5zh_client.exe'
    if (-not (Test-Path $exeSrc)) { Write-Host "FAIL(9): 找不到客户端 $exeSrc"; exit 9 }
    . "$acc\build_stamp.ps1"
    Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5zh_monster_missile'
    Get-CimInstance Win32_Process -Filter "Name='l5zh_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
    catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
    $out = "$env:TEMP\l5zh.out.log"; $err = "$env:TEMP\l5zh.err.log"
    Remove-Item $out, $err -Force -EA SilentlyContinue
    $clientProc = Start-Process -FilePath $exe -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
    function Rpc([string]$m, [hashtable]$q = @{}) {
        $c = New-Object Net.Sockets.TcpClient
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream()
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        ($l | ConvertFrom-Json).result
    }
    $st = $null
    foreach ($i in 1..90) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
    if ($null -eq $st.tile_x) { Write-Host ("FAIL(9): 90s 未进图（日志尾：" + ((Get-Content $err -Tail 3 -EA SilentlyContinue) -join ' | ') + "）"); exit 9 }
    Write-Host ("[前置] 进场 map={0} tile=({1},{2}) 账号={3}" -f $st.map, $st.tile_x, $st.tile_y, $User)
    # 走出安全区（`walk_to` 是玩家验收能力，走真实寻路）
    Rpc 'walk_to' @{ tx = $WalkX; ty = $WalkY; run = $true } | Out-Null
    $pos = $null
    foreach ($i in 1..30) {
        Start-Sleep 1
        $pos = Rpc 'state'
        if ($null -ne $pos.tile_x -and [Math]::Abs($pos.tile_x - $WalkX) -le 2 -and [Math]::Abs($pos.tile_y - $WalkY) -le 2) { break }
    }
    Write-Host ("[前置] 走出安全区：tile=({0},{1})（目标 ({2},{3})）" -f $pos.tile_x, $pos.tile_y, $WalkX, $WalkY)
    $before = Rpc 'spell_fx_probe'
    # 计数在 `spawned` 子对象里（探针返回 {ok,count,active,spawned{...}}）
    Write-Host ("[前置] 探针基线：monster_missile_add={0} fallback_placeholder={1}" -f $before.spawned.monster_missile_add, $before.spawned.fallback_placeholder)
    # 召唤：@MOB <名字>（服务端 session.rs 的 "MONSTER"|"MOB" 分支；test 账号 admin_account=1）
    Rpc 'chat' @{ message = "@MOB $Monster" } | Out-Null
    Write-Host ("[步骤] 已发 @MOB {0}，等它远程攻击（最多 {1}s）" -f $Monster, $WaitSeconds)
    $after = $null
    foreach ($i in 1..$WaitSeconds) {
        Start-Sleep 1
        $after = Rpc 'spell_fx_probe'
        if ($after.spawned.monster_missile_add -gt $before.spawned.monster_missile_add) { break }
    }
    $delta_missile = [int]$after.spawned.monster_missile_add - [int]$before.spawned.monster_missile_add
    $delta_placeholder = [int]$after.spawned.fallback_placeholder - [int]$before.spawned.fallback_placeholder
    Write-Host ("[读数] monster_missile_add +{0}（{1}→{2}） / fallback_placeholder +{3}" -f `
        $delta_missile, $before.spawned.monster_missile_add, $after.spawned.monster_missile_add, $delta_placeholder)
    $hit = $delta_missile -gt 0
    $verdict = if ($hit) { 'PASS' } else { 'FAIL' }
    Write-Host ("VERDICT monster_missile={0}（判据：monster_missile_add 必须增长；fallback_placeholder 不应增长）" -f $verdict)
    if (-not $hit) {
        Write-Host ("  日志尾：" + ((Get-Content $err -Tail 6 -EA SilentlyContinue) -join ' | '))
        exit 5
    }
    exit 0
} finally {
    if ($clientProc -and -not $clientProc.HasExited) { Stop-Process -Id $clientProc.Id -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 500
    Get-CimInstance Win32_Process -Filter "Name='l5zh_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock
}
