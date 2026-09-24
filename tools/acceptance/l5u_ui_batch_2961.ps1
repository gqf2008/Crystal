#Requires -Version 5.1
<#
.SYNOPSIS
  #2961 六项 UI 缺陷的**可复验**夹具（补交付缺口：报告引用的 ui_bugfix_verify/ime_rpc_verify 从未入库）。

.DESCRIPTION
  只覆盖本轮能拿到**真值判据**的项，其余如实标注为「本轮不可复验」——不把不可复验的项写成通过。

  判据来源：
    项2 聊天窗不可拖：`chat_probe`（面板矩形真值）+ `state`（玩家坐标真值）
    项5 滚动条：直接跑既有 `l5k_ranking_scroll.ps1`（服务端分页 + `scroll` RPC 真值，退出码即结论）
    项6 拖拽穿透：`ui_nodes_at` 证明拖动打到了 HUD 腰带（节点从原位置消失）+ `state` 证明玩家没动
    项1 坐骑遮挡半透明：**本轮不可复验**——需要世界渲染层探针（现有 RPC 只能看 UI 节点），报告里只有截图人工目视
    项3 IME 候选乱码：**本轮不可复验**——control RPC 没有 IME 通道（原 `ime_rpc_verify.ps1` 未入库）
    项4 商城错位：**部分**——`ui_alignment`/`ui_interact_sweep` 已覆盖商城窗；报告用的「4 个 C# 坐标」在库里没有出处

  退出码：0 = 可验项全过；1 = 有 FAIL；2 = 前置不满足（未进图/探针不可用）；3 = 前置不成立
#>
param(
    [string]$Worktree = '',
    [int]$ControlPort = 9061,
    [string]$Tag = 'u2961',
    # 项5（滚动条）判据来自**独立运行**的 `l5k_ranking_scroll.ps1`（它自带客户端与服务端分页断言）。
    # 这里只登记它的退出码——**不做内联调用**：内联时 l5k 启动的客户端会继承本脚本的 stdout 管道，
    # l5k 自己退出后管道仍不关闭，调用方会永久等 EOF（本夹具第一版就这么挂住了）。
    [int]$L5kExitCode = -1
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $Worktree) { $Worktree = (Resolve-Path "$PSScriptRoot\..\..").Path }
$root = 'C:\Users\gxh\AppData\Local\Temp\orig-csharp-ab'
$exeSrc = "$Worktree\Client-Bevy\target\debug\client_bevy.exe"
$exe = "$root\u2961_client.exe"
$err = "$root\l5u_$Tag.err"
$json = "$PSScriptRoot\l5u_ui_batch_2961_results.json"

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 3000; $c.SendTimeout = 3000
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 3000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}
function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='u2961_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}
function Tile($st) { if ($null -eq $st) { $null } else { "$($st.tile_x),$($st.tile_y)" } }
function PanelRect($p) {
    if ($null -eq $p -or $null -eq $p.panel) { $null }
    else { "{0},{1},{2},{3}" -f $p.panel.x, $p.panel.y, $p.panel.w, $p.panel.h }
}

$results = [ordered]@{}

# 实机资源（客户端 + e2e 账号 + 本地服务端）**必须串行**：先拿跨进程锁再起客户端。
# 不拿锁就会撞上「别的 agent 已登录同一账号」——表现为 `result=4 密码错误`（服务端实为
# `Account already online`），那是资源互斥假红，重试撞窗口只是把交付时间耗在等待上。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5u_ui_batch_2961' -TimeoutSec 1800)) {
    Write-Host 'FAIL(2): 等 e2e 锁超时（有其它实机任务长时间占用）'
    exit 2
}
try {

Get-CimInstance Win32_Process -Filter "Name='u2961_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
if (-not (Test-Path $exeSrc)) { Write-Host "缺少 exe: $exeSrc"; exit 2 }
Copy-Item -LiteralPath $exeSrc -Destination $exe -Force
if (Test-Path $err) { [System.IO.File]::Delete($err) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', '--control-port', "$ControlPort" `
    -WorkingDirectory "$Worktree\Client-Bevy" `
    -RedirectStandardOutput "$root\l5u_$Tag.log" -RedirectStandardError $err | Out-Null

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    $why = (Select-String -Path $err -Pattern '登录失败|already online' -EA SilentlyContinue | Select-Object -Last 1).Line
    Write-Host ("FAIL(2): 未进场 - " + $why)
    Stop-Client
    exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)
$fails = @()

# ---------------- 项2：聊天窗不可拖（C# ChatDialog 不设 Movable） ----------------
$cp0 = Rpc 'chat_probe'
if ($null -eq $cp0 -or $null -eq $cp0.panel) {
    $fails += '项2 前置不成立：chat_probe.panel 不可读'
    $results['item2_chat_not_draggable'] = 'SKIP(探针不可用)'
} else {
    $rect0 = PanelRect $cp0
    $tile0 = Tile (Rpc 'state')
    $cx = [double]$cp0.panel.x + [double]$cp0.panel.w / 2.0
    $cy = [double]$cp0.panel.y + [double]$cp0.panel.h / 2.0
    Rpc 'click' @{ x = $cx; y = $cy; drag_to = @{ x = $cx + 80; y = $cy + 50 } } | Out-Null
    Start-Sleep -Milliseconds 700
    $cp1 = Rpc 'chat_probe'
    $rect1 = PanelRect $cp1
    $tile1 = Tile (Rpc 'state')
    $ok2 = ($rect0 -eq $rect1) -and ($tile0 -eq $tile1)
    Write-Host ("项2 聊天窗拖动: 面板 [{0}] → [{1}]；玩家 {2} → {3}  ⇒ {4}" -f $rect0, $rect1, $tile0, $tile1, $(if ($ok2) { 'PASS' } else { 'FAIL' }))
    if (-not $ok2) { $fails += "项2 FAIL：面板或玩家位置被拖动改变（rect $rect0→$rect1, tile $tile0→$tile1）" }
    $results['item2_chat_not_draggable'] = @{ before = $rect0; after = $rect1; tile_before = $tile0; tile_after = $tile1; pass = $ok2 }
}

# ---------------- 项6：拖 HUD 腰带不移动玩家（拖拽穿透） ----------------
# C# `HeroBeltDialog` 位置 (475,618) 100x38（hero_belt.rs 的 BELT_X/BELT_Y/PANEL_SIZE）
$bx = 475.0 + 50.0; $by = 618.0 + 19.0
$hits0 = Rpc 'ui_nodes_at' @{ x = $bx; y = $by }
$tileA = Tile (Rpc 'state')
Rpc 'click' @{ x = $bx; y = $by; drag_to = @{ x = $bx + 80; y = $by + 50 } } | Out-Null
Start-Sleep -Milliseconds 700
$hits1 = Rpc 'ui_nodes_at' @{ x = $bx; y = $by }
$tileB = Tile (Rpc 'state')
$n0 = (@($hits0.nodes) | Measure-Object).Count
$n1 = (@($hits1.nodes) | Measure-Object).Count
# 判定：① 拖前该点有 UI 节点（腰带）；② 拖后该点不再命中同一批节点（腰带被拖走了）；③ 玩家坐标不变
$moved_belt = ($n0 -gt 0) -and (($hits0.nodes | ConvertTo-Json -Compress) -ne ($hits1.nodes | ConvertTo-Json -Compress))
$ok6 = $moved_belt -and ($tileA -eq $tileB)
Write-Host ("项6 腰带拖动: 命中数 {0}→{1}；腰带位移={2}；玩家 {3}→{4} ⇒ {5}" -f $n0, $n1, $moved_belt, $tileA, $tileB, $(if ($ok6) { 'PASS' } else { 'FAIL' }))
if (-not $ok6) { $fails += "项6 FAIL：腰带未响应拖动 或 玩家被拖动带走（tile $tileA→$tileB）" }
$results['item6_drag_no_passthrough'] = @{ hits_before = $n0; hits_after = $n1; belt_moved = $moved_belt; tile_before = $tileA; tile_after = $tileB; pass = $ok6 }

Stop-Client

# ---------------- 项5：滚动条（登记独立运行的 l5k 退出码） ----------------
if ($L5kExitCode -eq 0) {
    Write-Host '项5 滚动条：l5k_ranking_scroll.ps1 独立运行为 0（A–E 全绿）'
    $results['item5_scrollbars'] = @{ l5k_exit = 0; pass = $true;
        how = 'pwsh -File tools/acceptance/l5k_ranking_scroll.ps1 -ClientHome <worktree>（独立运行）' }
} elseif ($L5kExitCode -gt 0) {
    $fails += "项5 FAIL：l5k_ranking_scroll.ps1 独立运行退出码 $L5kExitCode"
    $results['item5_scrollbars'] = @{ l5k_exit = $L5kExitCode; pass = $false }
} else {
    $results['item5_scrollbars'] = 'NOT_RUN: 需先独立运行 l5k_ranking_scroll.ps1，并用 -L5kExitCode 传入其退出码'
}

# ---------------- 不可复验项（如实记录，不写成通过） ----------------
$results['item1_mount_ghost_translucent'] = 'NOT_VERIFIABLE: 需世界渲染层探针（现有 RPC 只能读 UI 节点）；原报告仅截图人工目视'
$results['item3_ime_candidates'] = 'NOT_VERIFIABLE: control RPC 无 IME 通道（原 ime_rpc_verify.ps1 从未入库）'
$results['item4_shop_alignment'] = 'PARTIAL: ui_alignment/ui_interact_sweep 覆盖商城窗；报告引用的 4 个 C# 坐标在库里无出处'

$result = [ordered]@{
    ok      = ($fails.Count -eq 0)
    failures = $fails
    items   = $results
}
$result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $json -Encoding UTF8
Write-Host ("结论 JSON: " + $json)
if ($fails.Count -gt 0) {
    Write-Host ('FAIL(1): ' + ($fails -join '; '))
    exit 1
}
Write-Host '=== 可复验项全部 PASS（项1/项3 标注为不可复验） ==='
exit 0

} finally {
    Exit-E2eLock
}
