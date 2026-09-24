# l5m_inventory_z.ps1 — 背包「关闭钮 vs 扩容钮」层序实机验收（owner 队列 inventory-zindex）
#
# 背景：C# `InventoryDialog` 先建 `AddButton`（`:76`）后建 `CloseButton`（`:101`），
# `Sort = true` ⇒ 关闭钮画在扩容钮**之上**。本端生成顺序相反（扩容钮后生成），
# 此前两者 z 同为 8（平局）→ 扩容钮在上，点 X 会被吞（命中区相交那条路已被几何门禁挡住，
# 层序这条由 `INV_CLOSE_Z > INV_ADD_Z` 钉住）。
#
# 判据（全部取状态，不解析像素）：
#   A) 打开背包 → 合成点击关闭钮中心 (301,14) → 背包**关闭**（层序摆正后点得到 X）
#   B) 再开背包 → 先断言确认框不在（`ui_nodes_at` 在确认框中心 0 个节点）
#      → 点扩容钮中心 (259,18) → 确认框出现（1 个节点，且矩形 = Prguse[360] 456x190）
#      **且背包仍开着**（点扩容钮不会关窗 = 没被上层元素吞 / 没误中关闭钮）
#   C) 点确认框 No（面板 (284,289) + 按钮 (360,157) 76x25 的中心 (682,458)）→ 确认框消失
#      —— 本夹具**不改持久状态**（不买扩容），可重复跑
#
# 退出码：0 = 全 PASS；10 = 判据未达成；9 = 服务端/客户端未就绪
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    # C# InventoryDialog：CloseButton @(289,3) 原生 24x21；AddButton @(235,5) 精灵 48x25
    [int]$CloseX = 301,
    [int]$CloseY = 14,
    [int]$AddX = 259,
    [int]$AddY = 18,
    # 确认框（Prguse[360] 456x190 @ (284,289)）与其中的 No 按钮中心
    [int]$BoxX = 512,
    [int]$BoxY = 384,
    [int]$NoX = 682,
    [int]$NoY = 458,
    [switch]$NoRestart
)

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5m_inventory_z' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁
# （PowerShell 的 finally 在 exit 下也会执行——实测 -File 与会话内 & script.ps1 两种调用都成立），
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
function InvOpen() {
    $d = Rpc 'dialogs'
    (@($d.dialogs) -join ',') -match 'Inventory'
}
# 确认框是否在：在框中心取「有效可见」的 UI 节点数（隐藏的框返回 0）
function ConfirmBoxNodes() { @((Rpc 'ui_nodes_at' @{ x = $BoxX; y = $BoxY }).nodes).Count }
function WaitInv([bool]$want, [int]$Tries = 12) {
    foreach ($i in 1..$Tries) { Start-Sleep -Milliseconds 500; if ((InvOpen) -eq $want) { return $true } }
    return ((InvOpen) -eq $want)
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
if (-not $NoRestart) {
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5m_client.log" -RedirectStandardError "$acc\l5m_client.err.log" | Out-Null
}
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

$verdict = $true

# ---- B) 扩容钮可点（点出确认框）且不关窗；C) 点 No 复原 ----
Rpc 'dialog' @{ kind = 'inventory'; action = 'open' } | Out-Null
if (-not (WaitInv $true)) { Write-Host '[B] FAIL：背包没打开'; exit 10 }
Start-Sleep -Milliseconds 500
$boxBefore = ConfirmBoxNodes
Write-Host ("[B] 点扩容前：确认框节点数 = {0}（期望 0）" -f $boxBefore)
Rpc 'click' @{ x = $AddX; y = $AddY } | Out-Null
Start-Sleep -Milliseconds 900
$boxAfter = ConfirmBoxNodes
$stillOpen = InvOpen
Write-Host ("[B] 点扩容钮 ({0},{1}) 后：确认框节点数 = {2}，背包仍开 = {3}" -f `
    $AddX, $AddY, $boxAfter, $stillOpen)
$okB = ($boxBefore -eq 0) -and ($boxAfter -ge 1) -and $stillOpen
Write-Host ("[B] 扩容钮可点（弹出确认框）且不被上层元素吞/不误关窗 → {0}" -f $(if ($okB) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okB

# C) 点确认框 No → 框消失（夹具不改持久状态，可重复跑）
Rpc 'click' @{ x = $NoX; y = $NoY } | Out-Null
Start-Sleep -Milliseconds 800
$boxGone = ConfirmBoxNodes
$okC = ($boxGone -eq 0)
Write-Host ("[C] 点 No ({0},{1}) 后确认框节点数 = {2} → {3}（本夹具不买扩容、可重复跑）" -f `
    $NoX, $NoY, $boxGone, $(if ($okC) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okC

# ---- A) 点关闭钮中心 → 背包必须关 ----
Rpc 'click' @{ x = $CloseX; y = $CloseY } | Out-Null
Start-Sleep -Milliseconds 900
$closed = -not (InvOpen)
Write-Host ("[A] 点关闭钮 ({0},{1}) 后背包关闭 → {2}" -f $CloseX, $CloseY, $(if ($closed) { 'PASS' } else { 'FAIL（X 被上层元素吞掉）' }))
$verdict = $verdict -and $closed

Write-Host ("VERDICT: {0}" -f $(if ($verdict) { 'PASS（关闭钮在扩容钮之上：X 点得动、扩容点得动）' } else { 'FAIL' }))
if (-not $verdict) { exit 10 }
exit 0

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
