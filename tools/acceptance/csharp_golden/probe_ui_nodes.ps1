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
    [string]$Out = "$env:TEMP\golden_ab_probe.json",
    # 要问的点（逻辑坐标），**单串 `x,y;x,y;…`**。为什么不用 [string[]]：`pwsh -File` 传数组参数时
    # PowerShell 会把后续 token 当**位置参数**绑定，实测把 `-Points 891,97 909,113` 里的 "909,113" 塞给了
    # 别的参数（客户端收到 `--control-port "909113"` → 回退 9000 → 探针永远读不到 9095 ⇒ 白等整轮）。
    # 默认那组是背包标题栏归因时用的；做**几何对表**时按目标传，例：角色窗装备格表
    # C# `CharacterDialog.cs:229-340`（页内偏移 + CharacterPage(8,90) + 对话框(760,0)）：
    #   -Points '891,97;909,113;971,97;...'  （Weapon 左上/中心、Helmet 左上…）
    [string]$Points = '265,45;270,15;330,12;400,12;480,12;560,12;310,25;310,100;150,10',
    # 只开背包窗（角色窗会盖住 x>=760 的点）；要在角色窗上取几何时把它关掉，别让 z 更高的窗抢先命中
    [switch]$InventoryOnly,
    # 要取 `dialog_rect` 的窗口种类，逗号分隔；`all`/`sweep` = 按**单一真源**
    # `tools/acceptance/interact_sweep_manifest.json` 的 `sweep` 名单逐窗「开→取矩形→关」
    # （逐窗几何对表用，见 window_rect_table.py --compare）。默认只取金标准帧里那两个窗。
    #
    # 为什么必须逐窗开一次：`dialog_rect` 是从**关闭钮**反推窗口矩形的
    # （`rx = cx - w/2`、`ry = cy - h/2`），窗口没开时关闭钮不存在 ⇒ 返回
    # `{ok:false, error:"close button not found"}`（实测：45 个 kind 一口气问只回 3 个 ok）。
    [string]$RectKinds = 'inventory,character'
    ,
    # 角色窗页（0=装备 1=状态 2=State 3=技能）；>=0 时用 `char_page` RPC 开窗并切页
    # （页签只能点、无热键，C# 亦然；见 control.rs 的 char_page）。做窗内控件对表时用它。
    [int]$CharPage = -1
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
    if ($CharPage -ge 0) {
        Write-Host ("角色窗切页 char_page={0}（0=装备 1=状态 2=State 3=技能）" -f $CharPage)
        $null = Rpc 'char_page' @{ page = $CharPage }
    } elseif (-not $InventoryOnly) {
        $null = Rpc 'dialog' @{ kind = 'character'; action = 'open' }
    }
    Start-Sleep -Seconds 3
    $res = [ordered]@{}
    $res['state'] = $s3
    $res['bag_probe'] = Rpc 'bag_probe'
    if ($RectKinds -eq 'all' -or $RectKinds -eq 'sweep') {
        $manifest = Join-Path $Repo 'tools\acceptance\interact_sweep_manifest.json'
        if (-not (Test-Path -LiteralPath $manifest)) { Write-Host "FAIL(2): 缺 $manifest"; exit 2 }
        $mf = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
        $noClose = @($mf.no_close_by_design)
        $kinds = @($mf.sweep)
        Write-Host ("逐窗几何：按 manifest 的 {0} 个 kind 逐个「开→取矩形→关」" -f $kinds.Count)
        foreach ($k in $kinds) {
            $openOk = (Rpc 'dialog' @{ kind = $k; action = 'open' })
            Start-Sleep -Milliseconds 350
            $res["rect_$k"] = Rpc 'dialog_rect' @{ kind = $k }
            $res["open_$k"] = $openOk
            if ($noClose -notcontains $k) {
                $null = Rpc 'dialog' @{ kind = $k; action = 'close' }
                Start-Sleep -Milliseconds 120
            }
        }
        # 关掉无关闭钮的那几个（RPC close 仍可用）
        foreach ($k in ($kinds | Where-Object { $noClose -contains $_ })) { $null = Rpc 'dialog' @{ kind = $k; action = 'close' } }
    } else {
        foreach ($k in ($RectKinds -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })) {
            $res["rect_$k"] = Rpc 'dialog_rect' @{ kind = $k }
        }
    }
    $nodes = [ordered]@{}
    foreach ($p in ($Points -split ';' | Where-Object { $_.Trim() })) {
        $xy = $p.Trim().Split(',')
        if ($xy.Count -ne 2) { Write-Host "跳过非法点 '$p'（要 x,y）"; continue }
        $r = Rpc 'ui_nodes_at' @{ x = [double]$xy[0]; y = [double]$xy[1] }
        $nodes["$([double]$xy[0]),$([double]$xy[1])"] = $r
    }
    $res['nodes'] = $nodes
    $json = $res | ConvertTo-Json -Depth 8
    [IO.File]::WriteAllText($Out, $json, (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "已写 $Out"
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue | Where-Object { $_.ExecutablePath -eq $exe } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
} finally { Exit-E2eLock }
