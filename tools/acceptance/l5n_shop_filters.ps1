# l5n_shop_filters.ps1 — 游戏商城三段筛选（C# ClassFilter/TypeFilter/SectionFilter）实机验收
# （owner 队列 `shop-class-tabs`）
#
# 原版口径（`Client/MirScenes/Dialogs/GameshopDialog.cs`）：
#   - 职业：六个按钮 `Title[751..768]` @ `ALL(539,37)`、`War(568,38)`+23px/档（`:276-377`）
#     → `ClassFilter`；谓词是**字符串**比较：`Class == ClassFilter || Class == "All" || ClassFilter == "Show All"`（`:720`）
#   - 区段：四个按钮 @ `(138|209|280|351, 68)`（`:212-273`）→ `SectionFilter`；
#     `Show All` / `TopItems&&TopItem` / `DealItems&&Deal` / `NewItems&&Date>Now-7d`（`:723`）
#     （第 4 档 `New` 在原版 `Visible=false` 且从未置 true → 本端同样隐藏，故本夹具不点它）
#   - 开窗 `Show()` 会把 `ClassFilter` 设为**玩家自己的职业**、`SectionFilter` 回 `Show All`（`:504-513`）
#
# 判据（全部取 `shop_probe` 状态真值，不解析像素）：
#   A) 开窗后 `class_filter` == 玩家职业名，`section_filter` == "Show All"，且 `filtered == total`
#   B) 点职业 `Show All` → `class_filter` 变 "Show All" 且 `filtered == total`（库里商品 class 全是 "All"）
#   C) 点区段 `DealItems` → `section_filter` 变 "DealItems" 且 `0 < filtered < total`（只有特价）
#   D) 点区段 `TopItems` → `0 < filtered < total`（只有置顶），且与 C 的行集不同
#   E) 点区段 `Show All` → 回到 `filtered == total`
#   并校验三个点击点确实落在**对应按钮**的矩形里（`ui_nodes_at`：rect 与 C# 常量对齐）
#
# 退出码：0 = 全 PASS；10 = 判据未达成；9 = 服务端/客户端未就绪
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    # 面板原点 = C# GameObject 居中 ((1024-696)/2, (768-476)/2) = (164,146)
    [int]$PanelX = 164,
    [int]$PanelY = 146,
    [double]$BtnW = 28.0,
    [double]$BtnH = 20.0,
    [switch]$NoRestart
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5n_shop_filters' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$acc = "$PSScriptRoot"
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName 'l5n_shop_filters'
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5n_client.exe'

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
# C# 面板内坐标 → 屏幕坐标（按钮中心）
function Btn([double]$x, [double]$y) { @([int]($PanelX + $x + $BtnW / 2), [int]($PanelY + $y + $BtnH / 2)) }
function Probe() { Rpc 'shop_probe' }

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
if (-not $NoRestart) {
    Get-CimInstance Win32_Process -Filter "Name='l5n_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    # 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5n_client.log" -RedirectStandardError "$acc\l5n_client.err.log" | Out-Null
}
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2}) 职业={3}" -f $st.map, $st.tile_x, $st.tile_y, $st.class)

Rpc 'dialog' @{ kind = 'game_shop'; action = 'open' } | Out-Null
$p = $null
foreach ($i in 1..15) { Start-Sleep 1; $p = Probe; if ($null -ne $p.total_items -and [int]$p.total_items -gt 0) { break } }
if ($null -eq $p -or [int]$p.total_items -le 0) { Write-Host '[A] FAIL：商城目录没有到达（shop_probe.total_items=0）'; exit 10 }
$total = [int]$p.total_items
Write-Host ("[A] 开窗：class_filter={0} section_filter={1} category='{2}' filtered={3} total={4}" -f `
    $p.class_filter, $p.section_filter, $p.category, $p.filtered, $total)
$okA = ("$($p.section_filter)" -eq 'Show All') -and ([int]$p.filtered -eq $total) -and ("$($p.class_filter)" -ne '')
Write-Host ("[A] 开着的是「自己的职业 + Show All」且过滤后=全量 → {0}" -f $(if ($okA) { 'PASS' } else { 'FAIL' }))
$verdict = $okA

# 三个点击点：职业 Show All（539,37）、区段 DealItems（280,68）、区段 TopItems（209,68）
$classAll = Btn 539 37
$deal = Btn 280 68
$top = Btn 209 68
$secAll = Btn 138 68
foreach ($pt in @(@($classAll, '职业 Show All', 539, 37), @($deal, '区段 DealItems', 280, 68), @($top, '区段 TopItems', 209, 68))) {
    $n = Rpc 'ui_nodes_at' @{ x = $pt[0][0]; y = $pt[0][1] }
    $hit = @($n.nodes)
    Write-Host ("  [{0}] 点({1},{2}) 命中 {3} 个节点；首节点 rect={4}" -f `
        $pt[1], $pt[0][0], $pt[0][1], $hit.Count, ($hit | Select-Object -First 1).rect -join ',')
}

# B) 职业切到 Show All
Rpc 'click' @{ x = $classAll[0]; y = $classAll[1] } | Out-Null
Start-Sleep -Milliseconds 800
$p = Probe
$okB = ("$($p.class_filter)" -eq 'Show All') -and ([int]$p.filtered -eq $total)
Write-Host ("[B] 点职业 Show All 后 class_filter={0} filtered={1} → {2}" -f `
    $p.class_filter, $p.filtered, $(if ($okB) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okB

# C) 区段切到 DealItems
Rpc 'click' @{ x = $deal[0]; y = $deal[1] } | Out-Null
Start-Sleep -Milliseconds 800
$pDeal = Probe
$nDeal = [int]$pDeal.filtered
$rowsDeal = @($pDeal.rows | ForEach-Object { $_.item_index }) -join ','
Write-Host ("[C] 点区段 DealItems 后 section_filter={0} filtered={1}（全量 {2}）分类表={3}" -f `
    $pDeal.section_filter, $nDeal, $total, (@($pDeal.categories) -join '|'))
$okC = ("$($pDeal.section_filter)" -eq 'DealItems') -and ($nDeal -gt 0) -and ($nDeal -lt $total) -and `
    (@($pDeal.rows | Where-Object { -not $_.deal }).Count -eq 0)
Write-Host ("[C] 特价段：只出 deal=true 且少于全量 → {0}" -f $(if ($okC) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okC

# D) 区段切到 TopItems
Rpc 'click' @{ x = $top[0]; y = $top[1] } | Out-Null
Start-Sleep -Milliseconds 800
$pTop = Probe
$nTop = [int]$pTop.filtered
$rowsTop = @($pTop.rows | ForEach-Object { $_.item_index }) -join ','
Write-Host ("[D] 点区段 TopItems 后 section_filter={0} filtered={1}；首页行=[{2}]" -f `
    $pTop.section_filter, $nTop, $rowsTop)
$okD = ("$($pTop.section_filter)" -eq 'TopItems') -and ($nTop -gt 0) -and ($nTop -lt $total) -and `
    (@($pTop.rows | Where-Object { -not $_.top_item }).Count -eq 0)
Write-Host ("[D] 置顶段：只出 top_item=true 且少于全量 → {0}" -f $(if ($okD) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okD

# E) 区段回 Show All
Rpc 'click' @{ x = $secAll[0]; y = $secAll[1] } | Out-Null
Start-Sleep -Milliseconds 800
$p = Probe
$okE = ("$($p.section_filter)" -eq 'Show All') -and ([int]$p.filtered -eq $total)
Write-Host ("[E] 区段回 Show All 后 section_filter={0} filtered={1} → {2}" -f `
    $p.section_filter, $p.filtered, $(if ($okE) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okE

Write-Host ("VERDICT: {0}（三段筛选：职业/区段都在筛选、特价 {1} 项、置顶 {2} 项，全量 {3} 项）" -f `
    $(if ($verdict) { 'PASS' } else { 'FAIL' }), $nDeal, $nTop, $total)
if (-not $verdict) { exit 10 }
exit 0

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5n_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
