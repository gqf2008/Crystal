# l5i_crossmap.ps1 — ② 跨图传送：传送 NPC 两层菜单（Service/@tele → 目的地 @moveN）→ 真换图
#
# 脚本依据（ServerRust/Daneo1989/Envir/NPCs/BichonProvince/BorderVillage/Border_Transport-0.txt）：
#   [@Main-1] 里 `I'll use this <Service/@tele>`；[@tele] 里五个目的地，每条 = `MOVE <图> <x> <y>` + `TAKEGOLD <fee>`：
#     [@move1] MOVE 0 296 221  TAKEGOLD 500     ← **同图**（'0' 就是 NPC 所在图），不作换图判据
#     [@move2] MOVE 2 500 485  TAKEGOLD 1000
#     [@move3] MOVE 3 361 342  TAKEGOLD 2000
#     [@move4] MOVE 11 164 337 TAKEGOLD 3000
#     [@move5] MOVE 4 264 257  TAKEGOLD 2000
#
# 判据（取状态，不解析日志）：
#   每个目的地：A) 传送前 gold 足够；B) 点完两层菜单后 state.map 变成期望图（@move1 同图 → 只判落点+扣费）；
#               C) 金币恰好扣掉该地点的 TAKEGOLD；D) 落点与脚本坐标一致（±TileTolerance）。
#   默认跑**全部五个**目的地（此前只跑 @move2 一个）；`-DestKey` 非空时只跑那一个（向后兼容旧用法）。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [string]$NpcMap = '0',
    [int]$NpcX = 287,
    [int]$NpcY = 615,
    [string]$NpcNamePattern = 'Teleport_*',
    [string]$DestKey = '',
    [string]$DestLabel = '',
    [string]$ExpectMap = '',
    [int]$ExpectGold = 0,
    [int]$ExpectX = 0,
    [int]$ExpectY = 0,
    [int]$TileTolerance = 6
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5i_crossmap' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5i_client.exe'

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}

# 目的地表（与脚本逐条对齐；SameMap=同图传送，不作换图判据）
$dests = @(
    [pscustomobject]@{ Key = '[@move1]'; Label = 'BichonProvince(同图)'; Map = '0';  Gold = 500;  X = 296; Y = 221; SameMap = $true },
    [pscustomobject]@{ Key = '[@move2]'; Label = 'SerpentValley';        Map = '2';  Gold = 1000; X = 500; Y = 485; SameMap = $false },
    [pscustomobject]@{ Key = '[@move3]'; Label = '@move3';              Map = '3';  Gold = 2000; X = 361; Y = 342; SameMap = $false },
    [pscustomobject]@{ Key = '[@move4]'; Label = '@move4';              Map = '11'; Gold = 3000; X = 164; Y = 337; SameMap = $false },
    [pscustomobject]@{ Key = '[@move5]'; Label = '@move5';              Map = '4';  Gold = 2000; X = 264; Y = 257; SameMap = $false }
)
if ($DestKey) {
    # 向后兼容：只跑指定目的地（旧调用形态）
    $dests = @([pscustomobject]@{ Key = $DestKey; Label = $DestLabel; Map = $ExpectMap; Gold = $ExpectGold;
        X = $ExpectX; Y = $ExpectY; SameMap = ("$ExpectMap" -eq "$NpcMap") })
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='l5i_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
# 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5i_client.log" -RedirectStandardError "$acc\l5i_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

$results = @()
foreach ($d in $dests) {
    Write-Host ("===== 目的地 {0}（图 {1} @ ({2},{3})，费 {4}）=====" -f $d.Key, $d.Map, $d.X, $d.Y, $d.Gold)
    # 1) 回到传送 NPC 旁（上一条目的地可能不在地图 0）
    Rpc 'chat' @{ message = "@mapmove $NpcMap $NpcX $NpcY" } | Out-Null
    $back = $null
    foreach ($i in 1..20) {
        Start-Sleep 1
        $back = Rpc 'state'
        if ("$($back.map)" -eq "$NpcMap") { break }
    }
    # 2) 找传送 NPC（按名字筛，不按最近）并开 [@MAIN]
    $npc = $null
    foreach ($i in 1..15) {
        Start-Sleep 1
        $npc = (Rpc 'nearby' @{ radius = 3000 }).entities |
            Where-Object { $_.kind -eq 'npc' -and $_.name -like $NpcNamePattern } |
            Sort-Object dist | Select-Object -First 1
        if ($npc) { break }
    }
    if (-not $npc) { Write-Host ('FAIL: 没找到传送 NPC（pattern={0}）' -f $NpcNamePattern); $results += @{ Key = $d.Key; Map = 'FAIL'; Fee = 'FAIL'; Land = 'FAIL' }; continue }
    Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
    # 3) 第一层：点 [@tele]（轮询到链接出现）
    $rows1 = $null; $tele = $null
    foreach ($i in 1..15) {
        Start-Sleep 1
        $rows1 = Rpc 'npc_rows'
        $tele = $rows1.links | Where-Object { $_.key -ieq '[@tele]' } | Select-Object -First 1
        if ($tele) { break }
    }
    if (-not $tele) { Write-Host ('FAIL: 第一层没有 [@tele]；links=' + (($rows1.links | ForEach-Object { $_.key }) -join ',')); $results += @{ Key = $d.Key; Map = 'FAIL'; Fee = 'FAIL'; Land = 'FAIL' }; continue }
    Rpc 'click' @{ x = $tele.cx; y = $tele.cy } | Out-Null
    # 4) 第二层：点目的地图
    $rows2 = $null; $dest = $null
    foreach ($i in 1..15) {
        Start-Sleep 1
        $rows2 = Rpc 'npc_rows'
        $dest = $rows2.links | Where-Object { $_.key -ieq $d.Key } | Select-Object -First 1
        if ($dest) { break }
    }
    if (-not $dest) { Write-Host ('FAIL: 目的地菜单没有 ' + $d.Key); $results += @{ Key = $d.Key; Map = 'FAIL'; Fee = 'FAIL'; Land = 'FAIL' }; continue }
    $gold0 = [int](Rpc 'bag_probe').gold
    $before = Rpc 'state'
    Write-Host ("[A] 传送前 map={0} tile=({1},{2}) gold={3}" -f $before.map, $before.tile_x, $before.tile_y, $gold0)
    Rpc 'click' @{ x = $dest.cx; y = $dest.cy } | Out-Null
    # 5) 换图 / 落点 / 扣费（都轮询到条件）
    $after = $null
    foreach ($i in 1..20) {
        Start-Sleep 1
        $after = Rpc 'state'
        if ("$($after.map)" -eq "$($d.Map)") { break }
    }
    $gold1 = $gold0
    foreach ($i in 1..10) {
        $gold1 = [int](Rpc 'bag_probe').gold
        if (($gold0 - $gold1) -eq $d.Gold) { break }
        Start-Sleep 1
    }
    $goldDelta = $gold0 - $gold1
    $mapChanged = ("$($after.map)" -eq "$($d.Map)")
    $near = ([Math]::Abs([int]$after.tile_x - $d.X) -le $TileTolerance) -and ([Math]::Abs([int]$after.tile_y - $d.Y) -le $TileTolerance)
    $okMap = if ($d.SameMap) { $true } else { $mapChanged }
    $okFee = ($goldDelta -eq $d.Gold)
    Write-Host ("[B] 传送后 map={0}（期望 {1}{2}）tile=({3},{4})（期望≈{5},{6}）；gold {7}->{8}（扣 {9}，期望 {10}）" -f `
        $after.map, $d.Map, $(if ($d.SameMap) { ' 同图' } else { '' }), $after.tile_x, $after.tile_y, $d.X, $d.Y, `
        $gold0, $gold1, $goldDelta, $d.Gold)
    $results += @{ Key = $d.Key; Map = $(if ($okMap) { 'PASS' } else { 'FAIL' }); Fee = $(if ($okFee) { 'PASS' } else { 'FAIL' }); Land = $(if ($near) { 'PASS' } else { 'FAIL' }) }
}

$allOk = $true
Write-Host "===== 逐目的地结果 ====="
foreach ($r in $results) {
    Write-Host ("{0,-9} map={1,-4} fee={2,-4} landing={3,-4}" -f $r.Key, $r.Map, $r.Fee, $r.Land)
    if ($r.Map -ne 'PASS' -or $r.Fee -ne 'PASS' -or $r.Land -ne 'PASS') { $allOk = $false }
}
$passCount = @($results | Where-Object { $_.Map -eq 'PASS' -and $_.Fee -eq 'PASS' -and $_.Land -eq 'PASS' }).Count
Write-Host ("VERDICT destinations={0}/{1}" -f $passCount, $results.Count)
if (-not $allOk) { exit 5 }

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5i_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
