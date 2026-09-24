# l5l_npc_scroll_hitrect.ps1 — NPC 对话框 / 商品窗「滚轮命中区」实机验收
# （owner 队列 `scroll-hitrect-npc` / `scroll-hitrect-npcgoods`）
#
# 原版口径（C#）：
#   - NPC 对话框：`MouseWheel += NPCDialog_MouseWheel` 挂在**对话框自身**（`NPCDialogs.cs:64`），
#     文本行/链接标签上那几处（`:502/547/566/604`）只是重复挂载 → 整个对话框都能滚。
#   - 商品窗：只给 `Cells[i]` 挂了 `MouseWheel`（`:1101`）→ 命中区 = 8 个 cell 的并集；
#     `Cells[i] = MirGoodsCell(205x32) @ (10, 34 + i*33)`（`:1074-1084`、`MirGoodsCell.cs:20`）
#     → 面板内 x 10..215、y 34..297（绝对 y 258..521，面板原点 (0,224)）。
#
# 判据（全部取 `scroll` RPC 真值 + 实机滚轮，不靠像素）：
#   A1) NPC 对话框命中区 == 面板 (0,0,440,224)
#   A2) 在**旧命中区之外**（标题栏 (30,12)；旧 rect=(8,34,400,144)）滚一格 → offset 变大
#   B1) 商品窗命中区 == 8 个 Cells 的并集 (10,258,205x263)
#   B2) 在**底行**（(60,500)；旧 rect 绝对 y 240..416）滚一格 → offset 变大
#   + 校验两个探针点确实落在旧命中区之外（否则判据没有区分力，直接判 FAIL）
#
# 测试对象（DB 选定，坐标写死便于复现）：
#   - Teleport_Gilbert @ BichonProvince（地图文件名 '0'）(287,615)：[@MAIN-1] 16 行 → 可滚
#   - Merchant_Carratt  @ 'UMM' (125,169)：70 件商品 → 可滚
#   （坐标/地图取自库：`npc_infos.map_index` 连 `map_infos.file_name`，不是直接用 map_index）
#
# 退出码：0 = 全 PASS；10 = 判据未达成；9 = 服务端/客户端未就绪
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [string]$NpcMap = '0',
    [int]$NpcX = 287,
    [int]$NpcY = 615,
    [string]$ShopMap = 'UMM',
    [int]$ShopX = 125,
    [int]$ShopY = 169,
    [string]$BuyKey = '[@BuySell]',
    # 本轮之前的两个命中区（面板内相对坐标），只用于打印「探针点当初在体外」的算术证据
    [string]$OldNpcRect = '8,34,400,144',
    [string]$OldGoodsRect = '10,16,230,176',
    # 找列表用的 rw（A/B 对照时传旧值：修复前 NPC 窗 rw=400、商品窗 rw=230）
    [double]$NpcListW = 440.0,
    [double]$ShopNpcListW = 440.0,
    [double]$GoodsListW = 205.0,
    [switch]$NoRestart
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5l_npc_scroll_hitrect' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$acc = "$PSScriptRoot"
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { Write-Host ("  [Rpc $m 失败] " + $_.Exception.Message); return $null }
}
function ListByW([double]$w) { (Rpc 'scroll').lists | Where-Object { $_.rw -eq $w } | Select-Object -First 1 }
function InRect([double]$x, [double]$y, [double]$rx, [double]$ry, [double]$rw, [double]$rh) {
    ($x -ge $rx) -and ($x -le ($rx + $rw)) -and ($y -ge $ry) -and ($y -le ($ry + $rh))
}
# 走到 NPC 旁 → 逐个候选试 key（页名真值在 DB `npc_scripts.page_name`，
# 实测 Teleport_Gilbert 的页名是 [@MAIN-1]，商人是 [@MAIN]）→ 返回「列表总量 >= NeedTotal」的那个窗
function OpenNpc([string]$map, [int]$x, [int]$y, [string[]]$keys, [double]$listW, [int]$NeedTotal) {
    Rpc 'chat' @{ message = "@mapmove $map $x $y" } | Out-Null
    Start-Sleep 2
    foreach ($i in 1..15) {
        Start-Sleep 1
        $cands = (Rpc 'nearby' @{ radius = 500 }).entities |
            Where-Object { $_.kind -eq 'npc' } | Sort-Object dist
        foreach ($c in $cands) {
            foreach ($k in $keys) {
                Rpc 'npc_call' @{ object_id = $c.object_id; key = $k } | Out-Null
                Start-Sleep -Milliseconds 800
                $l = ListByW $listW
                if ($null -ne $l -and [int]$l.total -ge $NeedTotal) {
                    return @{ npc = $c; list = $l; key = $k }
                }
            }
        }
    }
    return $null
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
if (-not $NoRestart) {
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5l_client.log" -RedirectStandardError "$acc\l5l_client.err.log" | Out-Null
}
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

$verdict = $true

# ============================ A) NPC 对话框 ============================
$hit = OpenNpc $NpcMap $NpcX $NpcY @('[@MAIN]', '[@MAIN-1]') $NpcListW 9
if ($null -eq $hit) { Write-Host '[A] FAIL：找不到可滚（行数 > 8）的 NPC 对话页'; exit 10 }
$nl = $hit.list
Write-Host ("[A] {0} key={1} 命中区 rect=({2},{3},{4}x{5}) offset={6} total={7} visible={8}" -f `
    $hit.npc.name, $hit.key, $nl.rx, $nl.ry, $nl.rw, $nl.rh, $nl.offset, $nl.total, $nl.visible)
$okA1 = ($nl.rx -eq 0.0 -and $nl.ry -eq 0.0 -and $nl.rw -eq 440.0 -and $nl.rh -eq 224.0)
Write-Host ("[A1] 命中区 == 面板 (0,0,440,224) → {0}" -f $(if ($okA1) { 'PASS' } else { 'FAIL' }))

$oldNpc = $OldNpcRect.Split(',') | ForEach-Object { [double]$_ }
$ax = 30; $ay = 12
$oldExclA = -not (InRect $ax $ay $oldNpc[0] $oldNpc[1] $oldNpc[2] $oldNpc[3])
Write-Host ("[A2] 探针 ({0},{1}) 在旧命中区 ({2}) 之外 → {3}" -f $ax, $ay, $OldNpcRect, $(if ($oldExclA) { '是' } else { '否' }))
$beforeA = (ListByW $NpcListW).offset
Rpc 'wheel' @{ x = $ax; y = $ay; delta = 1 } | Out-Null
Start-Sleep -Milliseconds 900
$afterA = (ListByW $NpcListW).offset
$okA2 = ($afterA -gt $beforeA)
Write-Host ("[A2] 在标题栏滚一格：offset {0} -> {1} → {2}（旧实现此处体外 ⇒ 恒 0）" -f `
    $beforeA, $afterA, $(if ($okA2) { 'PASS' } else { 'FAIL' }))
Rpc 'screenshot' @{ path = '../tools/acceptance/l5l_npc.png' } | Out-Null
$verdict = $verdict -and $okA1 -and $okA2 -and $oldExclA

# ============================= B) 商品窗 =============================
Rpc 'dialog' @{ kind = 'npc'; action = 'close' } | Out-Null
Start-Sleep -Milliseconds 400
$shop = OpenNpc $ShopMap $ShopX $ShopY @('[@MAIN]', '[@MAIN-1]') $ShopNpcListW 1
if ($null -eq $shop) { Write-Host '[B] FAIL：找不到商人的 [@MAIN]'; exit 10 }
$links = @((Rpc 'npc_rows').links)
$link = $links | Where-Object { $_.key -ieq $BuyKey } | Select-Object -First 1
if (-not $link) { $link = $links | Where-Object { $_.key -imatch 'buy|buysell' } | Select-Object -First 1 }
if (-not $link) {
    Write-Host ("[B] FAIL：{0} 的菜单里没有买卖链接（links={1}）" -f $shop.npc.name, ($links | ForEach-Object { $_.key }) -join ',')
    exit 10
}
Rpc 'click' @{ x = $link.cx; y = $link.cy } | Out-Null
$probe = $null
foreach ($i in 1..10) { Start-Sleep 1; $p = Rpc 'npc_goods_probe'; if ($p.count -gt 0) { $probe = $p; break } }
if ($null -eq $probe) { Write-Host '[B] FAIL：商品窗没有开'; exit 10 }
$gl = ListByW $GoodsListW
Write-Host ("[B] {0} 商品 {1} 行；命中区 rect=({2},{3},{4}x{5}) offset={6} total={7} visible={8}" -f `
    $shop.npc.name, $probe.count, $gl.rx, $gl.ry, $gl.rw, $gl.rh, $gl.offset, $gl.total, $gl.visible)
$okB1 = ($gl.rx -eq 10.0 -and $gl.ry -eq 258.0 -and $gl.rw -eq 205.0 -and $gl.rh -eq 263.0)
Write-Host ("[B1] 命中区 == 8 个 Cells 的并集 (10,258,205x263) → {0}" -f $(if ($okB1) { 'PASS' } else { 'FAIL' }))

$oldGoods = $OldGoodsRect.Split(',') | ForEach-Object { [double]$_ }
$bx = 60; $by = 500
$oldExclB = -not (InRect $bx $by $oldGoods[0] (224.0 + $oldGoods[1]) $oldGoods[2] $oldGoods[3])
Write-Host ("[B2] 探针 ({0},{1}) 在旧命中区 (绝对 y 240..416) 之外 → {2}" -f $bx, $by, $(if ($oldExclB) { '是' } else { '否' }))
$beforeB = (ListByW $GoodsListW).offset
Rpc 'wheel' @{ x = $bx; y = $by; delta = 1 } | Out-Null
Start-Sleep -Milliseconds 900
$afterB = (ListByW $GoodsListW).offset
$okB2 = ($afterB -gt $beforeB)
Write-Host ("[B2] 在底行滚一格：offset {0} -> {1} → {2}（旧实现末行体外 ⇒ 恒 0）" -f `
    $beforeB, $afterB, $(if ($okB2) { 'PASS' } else { 'FAIL' }))
Rpc 'screenshot' @{ path = '../tools/acceptance/l5l_goods.png' } | Out-Null
$verdict = $verdict -and $okB1 -and $okB2 -and $oldExclB

if (-not ($oldExclA -and $oldExclB)) {
    Write-Host '注意：探针点没落在旧命中区之外 → 本判据对「改没改」没有区分力'
    $verdict = $false
}
Write-Host ("VERDICT: {0}" -f $(if ($verdict) { 'PASS（NPC 对话框=整窗、商品窗=8 格并集，两处都真能滚）' } else { 'FAIL' }))
if (-not $verdict) { exit 10 }
exit 0

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
