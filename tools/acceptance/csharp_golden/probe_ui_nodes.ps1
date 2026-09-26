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
#
# 2026-09-26 三条**已验证**的使用要点（都踩过）：
#   1. `-Map/-TileX/-TileY` 的对齐判据是「**位置真的变了**」，不是「map 等于目标」——后者在
#      同图内换坐标时一进来就成立，命令还没生效就被当成已对齐（曾打印"对齐后 tile=(277,609)"，
#      实际位移没发生）。位置没变会打 WARN。
#   2. `nearby` 的 `radius` 是**世界像素不是格**（1 格 = 48px，`control.rs` 的 `Nearby` 直接比
#      `translation` 像素距）⇒ 按"40 格"找要给 ≈1920；传 12/40 恒为 0（曾误判"商人区没有 NPC"）。
#      返回体是 `{count, entities:[{kind,name,object_id,x,y,dist,vp}]}` —— 数组字段是 **entities**、
#      类型字段是 **kind**（写成 `$x.nearby` 时 `@($null)` 恒为 1 个空元素，看着像"只有一个实体、
#      字段全空"）。
#   3. `-NpcCallKey` 打开 NPC 窗**仍受阻**：`npc_call` 只发 `CallNPC{object_id,key}` 给服务端，
#      服务端要求目标在**交互距离内**才响应——实测 `Merchant_Ruben` 在 240px(≈5 格) 外被忽略、
#      窗口不开；`Teleport_Gilbert`（57px）脚本里没有 `[@MAIN]` 也不响应；按 `nearby.vp` 先
#      `click` 一次（3s）也不够（多半要先走到跟前）。⇒ 要拿 NPC 窗的实机证据，下一步得
#      **先 `walk_to` 贴近**再交互/`npc_call`。
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
    [string]$RectKinds = 'inventory,character',
    # 角色窗页（0=装备 1=状态 2=State 3=技能）；>=0 时用 `char_page` RPC 开窗并切页
    # （页签只能点、无热键，C# 亦然；见 control.rs 的 char_page）。做窗内控件对表时用它。
    [int]$CharPage = -1,
    # 对齐目标地图/坐标（默认 = 金标准帧那张：BichonProvince map 0 @ (277,609)）。
    # 开 NPC 窗时按 NPC 所在地传（商人区 ≈ (288,616)）。
    [int]$Map = 0,
    [int]$TileX = 277,
    [int]$TileY = 609,
    # 可选：发 `npc_call <key>` 打开 NPC 窗（NPC 窗是状态驱动窗，只能走这条真实路径）。
    # 形如 '[@MAIN]'；留空则不开。会先从 `nearby` 的 `entities` 里挑最近的 `kind=npc`。
    [string]$NpcCallKey = ''
    ,
    # `npc_call` 前找 NPC 的 `nearby` 半径，单位是**世界像素而不是格**（1 格 = 48px）：
    # `control.rs` 的 `Nearby` 拿 `translation` 的像素距与 radius 直接比。按 40 格找就得给 ≈1920；
    # 传 12/40 恒为 0（2026-09-26 实测踩过：以为"商人区没有 NPC"，其实是半径单位错了）。
    [int]$NearbyRadius = 2000,
    # 挑 NPC 时优先匹配的名字子串（如 'Merchant'）：`[@MAIN]` 不是每个 NPC 脚本都有，
    # 选到传送员（Teleport_Gilbert 之类）会"点了没反应"（实测）。留空 = 取最近的 NPC。
    [string]$NpcNameLike = 'Merchant'
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
    # 对齐：**按 tile/map 是否真的变了**判，不要按「map 等于目标」判——在目标地图内换坐标时
    # 后者一进来就成立，循环立刻 break，`@mapmove` 还没生效就被当成"已对齐"
    # （2026-09-26 实测：`-TileX 288 -TileY 616` 打印"对齐后 tile=(277,609)"，位移压根没发生）。
    # 同时把坐标写进结论，夹具/人一眼能看出到底动没动。
    $before = "$($st.map):$($st.tile_x),$($st.tile_y)"
    $target = "$Map`:$TileX,$TileY"
    $null = Rpc 'chat' @{ message = "@mapmove $Map $TileX $TileY" }
    $s3 = $st
    if ($before -ne $target) {
        $moved = $false
        foreach ($i in 1..60) {
            Start-Sleep -Milliseconds 500
            $s3 = Rpc 'state'
            if ("$($s3.map):$($s3.tile_x),$($s3.tile_y)" -ne $before) { $moved = $true; break }
        }
        if (-not $moved) {
            Write-Host ("WARN: @mapmove {0} {1} {2} 后位置没变（仍 {3}）——目标坐标不可走？还是命令没生效？" -f $Map, $TileX, $TileY, $before)
        }
    }
    Write-Host ("对齐后 map={0} tile=({1},{2})（目标 {3}）" -f $s3.map, $s3.tile_x, $s3.tile_y, $target)
    $null = Rpc 'dialog' @{ kind = 'inventory'; action = 'open' }
    if ($CharPage -ge 0) {
        Write-Host ("角色窗切页 char_page={0}（0=装备 1=状态 2=State 3=技能）" -f $CharPage)
        $null = Rpc 'char_page' @{ page = $CharPage }
    } elseif (-not $InventoryOnly) {
        $null = Rpc 'dialog' @{ kind = 'character'; action = 'open' }
    }
    if ($NpcCallKey) {
        # NPC 窗是**状态驱动**窗（不进 `DialogManager.open`），只能走真实路径：站在 NPC 旁 →
        # `npc_call <object_id> <key>`。`nearby` 的返回是 `{count, entities:[{kind,name,object_id,x,y,dist,…}]}`
        # ——数组字段名是 **entities**、类型字段名是 **kind**（此前写成 `$near.nearby`，`@($null)` 恒为
        # 1 个空元素，看起来像"只有 1 个实体且字段全空"，把归因带偏）。
        $near = Rpc 'nearby' @{ radius = $NearbyRadius }
        $npcs = @()
        foreach ($e in @($near.entities)) {
            if ($null -ne $e -and "$($e.kind)" -eq 'npc') { $npcs += $e }
        }
        if ($npcs.Count -eq 0) {
            Write-Host ("NPC 窗：半径 {1}px 内没找到 kind=npc（返回 count={0}，实体 kind 取值：{1}）" -f `
                $near.count, (($near.entities | ForEach-Object { $_.kind } | Sort-Object -Unique) -join ','))
        } else {
            $pref = @($npcs | Where-Object { $NpcNameLike -and "$($_.name)" -like "*$NpcNameLike*" })
            $npc = if ($pref.Count -gt 0) { $pref | Sort-Object dist | Select-Object -First 1 }
                   else { $npcs | Sort-Object dist | Select-Object -First 1 }
            Write-Host ("NPC 窗：npc_call object_id={0} name={1} dist={2} key={3}" -f `
                $npc.object_id, $npc.name, $npc.dist, $NpcCallKey)
            # **先走过去**再 call：`npc_call` 只发 `CallNPC{object_id,key}` 给服务端，服务端要求
            # **交互距离内**才响应——实测 `Merchant_Ruben` 在 240px(≈5 格) 外时被忽略、窗口不开；
            # 光 `click` 一次（3s，人还在原地）也不够。
            # `walk_to` 接受**世界坐标** `{x,y}`（也接受 `{tx,ty}`，但瓦片→世界要过
            # `movement::tile_to_world`，别自己算），而 `nearby.entities[]` 给的正是世界 `x/y`，
            # 直接喂进去最省事、也不会踩"两套瓦片换算"的坑。
            Write-Host ("NPC 窗：walk_to 世界坐标 ({0},{1})（{2}）" -f $npc.x, $npc.y, $npc.name)
            $null = Rpc 'walk_to' @{ x = [double]$npc.x; y = [double]$npc.y }
            $near2 = $null
            foreach ($i in 1..40) {
                Start-Sleep -Milliseconds 500
                $near2 = Rpc 'nearby' @{ radius = $NearbyRadius }
                $me = @($near2.entities) | Where-Object { $null -ne $_ -and "$($_.object_id)" -eq "$($npc.object_id)" } | Select-Object -First 1
                if ($null -ne $me -and [int]$me.dist -lt 60) { $npc = $me; break }
            }
            Write-Host ("NPC 窗：走近后 dist={0}（目标 <60px）" -f (@($me)[0].dist))
            # 开窗要**轮询 + 重试**：`l5e` 的实测经验是「object_id 会随地图重建变化」，未开窗时
            # 重新按名字定位再发一次；成功信号取 `npc_rows` 有行（NPC 窗是状态驱动窗，不进 dialogs）。
            $opened = $false
            foreach ($try in 1..8) {
                $null = Rpc 'npc_call' @{ object_id = [int]$npc.object_id; key = $NpcCallKey }
                Start-Sleep -Milliseconds 800
                $rows = Rpc 'npc_rows'
                if (@($rows.links).Count -gt 0 -or [int]$rows.count -gt 0) { $opened = $true; break }
                # 没开就重新定位（按名字优先，回退最近）
                $near3 = Rpc 'nearby' @{ radius = $NearbyRadius }
                $cand = @($near3.entities) | Where-Object { $null -ne $_ -and "$($_.kind)" -eq 'npc' }
                $pref3 = @($cand | Where-Object { $NpcNameLike -and "$($_.name)" -like "*$NpcNameLike*" })
                $pick = if ($pref3.Count -gt 0) { $pref3 | Sort-Object dist | Select-Object -First 1 } else { $cand | Sort-Object dist | Select-Object -First 1 }
                if ($null -ne $pick) { $npc = $pick }
            }
            Write-Host ("NPC 窗：开窗={0}（试 {1} 次，name={2} object_id={3}）" -f $opened, $try, $npc.name, $npc.object_id)
        }
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
