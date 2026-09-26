# probe_ui_nodes.ps1 — 「这一块像素是谁画的？」只读探针（金标准 A/B 归因用）
#
# 背景：2026-09-26 对齐 A/B 时，本站客户端背包标题栏右多出一排东西，肉眼看着像
# `BUY / M.gg / CHG / BRG / CRT` 五个控件——纯读源码（含 C# 源码）grep 这些字样**全无命中**，
# 说明要么是美术自带、要么根本不是同一扇窗画的。这条探针不看代码猜，直接问运行中的客户端：
#   - `ui_nodes_at {x,y}`（逻辑坐标）→ 命中该点的**所有**节点（含祖先链、显隐、z），
#     据此认出"那是 `SkillBarRoot` 的子节点"（=技能栏，被设置开关控制，不是背包多画的控件）；
#   - `dialog_rect {kind}` → 窗口根面板的真实屏幕矩形（与 C# 的 `(0,0) 316x236` 对表）；
#   - `bag_probe` → 背包格数（判"ITEMS II 用 738 还是灰掉的 169"这类**随状态变化**的美术规则）。
# 结论落 `%TEMP%\golden_ab_probe.json`，供人复核。
#
# 用法：pwsh tools/acceptance/csharp_golden/probe_ui_nodes.ps1 -ClientHome <带 client_bevy.exe 的 worktree>
param(
    [string]$Repo = 'E:\Users\gxh\Documents\GitHub\Crystal',
    [string]$ClientHome = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend',
    [int]$ControlPort = 9095,
    [string]$Out = "$env:TEMP\golden_ab_probe.json"
)
$ErrorActionPreference = 'Continue'
. "$PSScriptRoot\..\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName '_probe_golden_ab' -TimeoutSec 1800)) { exit 2 }
try {
    $env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
    $env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
    $exe = Join-Path $ClientHome 'Client-Bevy\target\debug\client_bevy.exe'
    $work = Join-Path $env:TEMP 'ab_bevy_work'
    # 构建戳前置（T11.1）：探针读的是运行中的客户端，旧产物只能给出旧结论
    . "$PSScriptRoot\..\build_stamp.ps1"
    Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName 'csharp_golden_probe_ui_nodes'
    if (-not (Test-Path $work)) { New-Item -ItemType Directory -Force -Path $work | Out-Null }
    [IO.File]::WriteAllText((Join-Path $work 'config.ini'), "[Network]`nServerAddr=127.0.0.1:7000`nUseMock=false`n", (New-Object System.Text.UTF8Encoding($false)))
    function Rpc([string]$m, [hashtable]$q = @{}) {
        try {
            $c = New-Object Net.Sockets.TcpClient; $c.ReceiveTimeout = 3000; $c.SendTimeout = 3000
            $c.Connect('127.0.0.1', $ControlPort); $s = $c.GetStream(); $s.ReadTimeout = 3000
            $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress) + "`n")
            $s.Write($b, 0, $b.Length); $s.Flush()
            $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
            if (-not $l) { return $null }; ($l | ConvertFrom-Json).result
        } catch { return $null }
    }
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue | Where-Object { $_.ExecutablePath -eq $exe } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 800
    Start-Process -FilePath $exe -ArgumentList '--real-net', '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', '--control-port', "$ControlPort" `
        -WorkingDirectory $work -RedirectStandardOutput (Join-Path $work 'p.log') -RedirectStandardError (Join-Path $work 'p.err') | Out-Null
    $st = $null
    foreach ($i in 1..90) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
    if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host 'FAIL: 未进场'; exit 2 }
    Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)
    $null = Rpc 'chat' @{ message = '@mapmove 0 277 609' }
    foreach ($i in 1..40) { Start-Sleep -Milliseconds 500; $s2 = Rpc 'state'; if ("$($s2.map)" -eq '0') { break } }
    $s3 = Rpc 'state'; Write-Host ("对齐后 map={0} tile=({1},{2})" -f $s3.map, $s3.tile_x, $s3.tile_y)
    $null = Rpc 'dialog' @{ kind = 'inventory'; action = 'open' }
    $null = Rpc 'dialog' @{ kind = 'character'; action = 'open' }
    Start-Sleep -Seconds 3
    $res = [ordered]@{}
    $res['state'] = $s3
    $res['bag_probe'] = Rpc 'bag_probe'
    foreach ($k in @('inventory', 'character')) {
        $res["rect_$k"] = Rpc 'dialog_rect' @{ kind = $k }
    }
    $pts = @(265, 45), (270, 15), (330, 12), (400, 12), (480, 12), (560, 12), (310, 25), (310, 100), (150, 10)
    $nodes = [ordered]@{}
    foreach ($p in $pts) {
        $r = Rpc 'ui_nodes_at' @{ x = $p[0]; y = $p[1] }
        $nodes["$($p[0]),$($p[1])"] = $r
    }
    $res['nodes'] = $nodes
    $json = $res | ConvertTo-Json -Depth 8
    [IO.File]::WriteAllText($Out, $json, (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "已写 $Out"
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue | Where-Object { $_.ExecutablePath -eq $exe } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
} finally { Exit-E2eLock }
