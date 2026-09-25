# l5k_ranking_scroll.ps1 — 排行榜滚动（服务端分页）实机验收
#
# 背景（owner 队列 `ranking-scroll-noop`）：本端排行榜的滚轮/位置条此前"只记录不滚动"，
# 因为客户端只拿到**前 20 名一个窗口**、把「窗口行数」当 total → `max_offset = 20 - 20 = 0`。
# 原版 C# 是**服务端分页**：客户端发 `C.GetRanking{RankType, RankIndex=RowOffset}`，
# 服务端回该窗口的 20 行 + 总条数（`S.Rankings.Count`），滚动上限 = `Count - 20`。
#
# 判据（全部取**客户端状态真值** `scroll` RPC，不靠像素、不靠日志猜测）：
#   A) 打开排行榜后 `total > 20`（服务端回的是该榜总条数，不是窗口行数）
#   B) 在列表矩形内滚轮一格 → `offset` 由 0 变大（滚轮真的驱动了 RowOffset）
#   C) 连滚到边界 → `offset == total - visible`（钳位与原版 `[0, RankCount-20]` 一致）
#   D) 滚动确实走了服务端分页：客户端日志出现「请求排行榜 榜=0 窗口起点=N(N>0)」
#      （只把 offset 记在本地、不发包 = 假滚动，这条判据专门拦它）
#   E) 职业页签也走服务端选榜：点「战士」页签 → 请求 `榜=1`，回的 total 是该职业子集
#      （< 全榜且 ≥1）。改动前是客户端从「前 20 名窗口」里本地过滤 → 榜内人不在前 20 就显示不出来。
#
# 用法：
#   pwsh -File tools/acceptance/l5k_ranking_scroll.ps1 -ClientHome <构建根>
# 退出码：0 = A–D 全 PASS；10 = 判据未达成（详见输出）；9 = 服务端未运行
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    # 客户端构建根（默认本 worktree）；日志写在该根下的 tools/acceptance
    [string]$ClientHome = '',
    [int]$WheelDelta = 1,
    [int]$SettleMs = 1400,
    # A 判据门槛：必须 > 一屏（20 行），否则滚动根本没有空间
    [int]$MinTotal = 21
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5k_ranking_scroll' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$acc = "$ClientHome\tools\acceptance"
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5k_client.exe'
$clientLog = "$acc\l5k_client.log"
$clientErr = "$acc\l5k_client.err.log"

# RPC 必须自带超时：客户端控制线程若卡住，无超时的 ReadLine 会把夹具挂死
# （首次实跑就踩到——C 循环第 6 个 wheel 后再也读不到回包）。
function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 5000
        $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch {
        Write-Host ("  [Rpc $m 失败] " + $_.Exception.Message)
        return $null
    }
}

# 排行榜列表实体（面板内 z=39 的那个；按 rect 尺寸 + visible 行数识别，不写死 entity id）
function RankList($scroll) {
    $scroll.lists | Where-Object { $_.visible -eq 20 -and $_.rh -gt 200 } | Select-Object -First 1
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='l5k_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Remove-Item $clientLog, $clientErr -Force -EA SilentlyContinue
# 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut $clientLog -RedirectStandardError $clientErr | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '[前置] 客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 打开排行榜（走真实 dialog 路径，不是直接改状态）
Rpc 'dialog' @{ kind = 'ranking'; action = 'open' } | Out-Null
# 等服务端回榜（第一次实跑读太早 → total=0 会被误读成"不可滚"）：轮询到 total>0
$before = $null
foreach ($i in 1..12) {
    Start-Sleep -Milliseconds 900
    $before = RankList (Rpc 'scroll')
    if ($null -ne $before -and [int]$before.total -gt 0) { break }
}
if ($null -eq $before) { Write-Host '[A] FAIL：找不到排行榜列表（对话框没打开？）'; exit 10 }
Write-Host ("[A] 打开后 total={0} offset={1} visible={2} 矩形=({3},{4},{5},{6})" -f `
    $before.total, $before.offset, $before.visible, $before.rx, $before.ry, $before.rw, $before.rh)
$okA = ([int]$before.total -ge $MinTotal)
Write-Host ("[A] total({0}) >= {1} → {2}（旧实现只回前 20 名 → total=20 ⇒ max_offset=0 滚不动）" -f `
    $before.total, $MinTotal, $(if ($okA) { 'PASS' } else { 'FAIL' }))

# B) 滚轮一格 → offset 必须真的动
$cx = [int]([double]$before.rx + [double]$before.rw / 2)
$cy = [int]([double]$before.ry + [double]$before.rh / 2)
Rpc 'wheel' @{ x = $cx; y = $cy; delta = $WheelDelta } | Out-Null
Start-Sleep -Milliseconds $SettleMs
$after = RankList (Rpc 'scroll')
Write-Host ("[B] 滚轮({0},{1}) delta={2} 后 offset={3}（滚前 {4}）" -f $cx, $cy, $WheelDelta, $after.offset, $before.offset)
$okB = ([int]$after.offset -gt [int]$before.offset)
Write-Host ("[B] offset 增大 → {0}" -f $(if ($okB) { 'PASS' } else { 'FAIL' }))

# C) 一步滚到末页（`rows = delta * step` 后在系统内 clamp）→ 钳在 total - visible
#    （原版 `[0, RankCount-20]`；不是"滚到哪算哪"）
$maxOff = [int]$after.total - [int]$after.visible
if ($maxOff -le 0) {
    Write-Host '[C] N/A：total <= 一屏 → 无可滚空间（旧实现恒为此态）'
    $okC = $true
} else {
    Rpc 'wheel' @{ x = $cx; y = $cy; delta = $maxOff } | Out-Null
    Start-Sleep -Milliseconds $SettleMs
    $last = RankList (Rpc 'scroll')
    Write-Host ("[C] 滚到底（delta={0}）后 offset={1}，上限 total-visible={2}" -f $maxOff, $last.offset, $maxOff)
    $okC = ($null -ne $last -and [int]$last.offset -eq $maxOff)
}
Write-Host ("[C] 钳位一致 → {0}" -f $(if ($okC) { 'PASS' } else { 'FAIL' }))

# D) 证据：客户端真的按窗口起点发了请求（本地假滚动不会有这行）
#    注意：tracing 走 **stderr**（stdout 是空文件），两个都扫
$reqFiles = @($clientLog, $clientErr) | Where-Object { Test-Path $_ }
$reqLines = @(Select-String -Path $reqFiles -Pattern '请求排行榜 榜=0 窗口起点=(\d+)' -EA SilentlyContinue)
$nonZero = @($reqLines | Where-Object { $_.Matches[0].Groups[1].Value -ne '0' })
$okD = ($nonZero.Count -gt 0)
Write-Host ("[D] 客户端请求行 {0} 条，其中窗口起点>0 的 {1} 条" -f $reqLines.Count, $nonZero.Count)
if ($nonZero.Count -gt 0) { Write-Host ("    例：" + $nonZero[-1].Line.Trim()) }
Write-Host ("[D] 滚动走了服务端分页 → {0}" -f $(if ($okD) { 'PASS' } else { 'FAIL' }))

$verdict = $okA -and $okB -and $okC -and $okD

# E) 职业页签 = 服务端选榜（面板原点 350,163 + War 页签 TAB_POS[2]=(60,38) 24x20 的中心）
$warX = [int](350 + 60 + 12)
$warY = [int](163 + 38 + 10)
Rpc 'click' @{ x = $warX; y = $warY } | Out-Null
Start-Sleep -Milliseconds 1500
$war = RankList (Rpc 'scroll')
$warReq = @(Select-String -Path $reqFiles -Pattern '请求排行榜 榜=1 ' -EA SilentlyContinue)
Write-Host ("[E] 点战士页签({0},{1}) 后 total={2}（全榜 {3}），请求榜=1 的行数={4}" -f `
    $warX, $warY, $war.total, $before.total, $warReq.Count)
$okE = ($warReq.Count -gt 0) -and ([int]$war.total -ge 1) -and ([int]$war.total -lt [int]$before.total)
Write-Host ("[E] 职业榜 = 服务端按职业选榜（total 缩到子集）→ {0}" -f $(if ($okE) { 'PASS' } else { 'FAIL' }))

$verdict = $verdict -and $okE
Write-Host ("VERDICT: {0}" -f $(if ($verdict) { 'PASS（排行榜滚动 = 服务端分页，真滚动）' } else { 'FAIL' }))
if (-not $verdict) { exit 10 }
exit 0

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5k_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
