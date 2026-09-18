# ui_interact_sweep.ps1 — 全 UI 窗口**交互级**验证：
# 对每个 RPC 窗口：dialog open → dialog_rect 取根矩形 → click 右上关闭钮（真实 picking→Interaction 链路）→ 断言窗口关闭
# 另：inventory 拖动测试（press→move→release 走 ButtonInput 拖动链路，断言矩形位移）
# 前提：mir2_server 在跑；客户端 --real-net --auto-enter。
$ErrorActionPreference = 'Stop'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;' + $env:PATH
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
$wd  = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy'

function Rpc([string]$method, [hashtable]$params = @{}) {
    $c = New-Object Net.Sockets.TcpClient
    $c.Connect('127.0.0.1', 9000)
    $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$method; params=$params } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s)
    $line = $r.ReadLine(); $c.Close()
    if ($null -eq $line) { throw "control 无响应: $method" }
    ($line | ConvertFrom-Json).result
}
function Pascal([string]$snake) { ($snake -split '_' | ForEach-Object { $_.Substring(0,1).ToUpper() + $_.Substring(1) }) -join '' }

# A 类 40 窗口（状态驱动的 npc/trade/npc_goods/roll 不在此列；hero_manage 单独走状态路径）
$kinds = @(
    'inventory','character','quest_log','settings','menu','game_shop','minimap',
    'group','friend','inspect','guild','mail','ranking','mentor','relationship',
    'mount','report','hero_inventory','hero_equipment','creature','item_rental',
    'guild_territory','help','notice','buff','fishing','socket','refine','craft',
    'dura_status','npc_awake','timer','keyboard_layout','big_map','chat_notice',
    'market','storage','item_rental_browse','quest_detail','input_box'
)
# 设计上没有关闭钮的窗口（C# 原版即无 X）：跳过点击测试但验证开/关 RPC 往返
$noCloseByDesign = @('menu','minimap','buff','refine','timer','chat_notice')

Get-Process client_bevy -EA SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\interact_client.log" -RedirectStandardError "$acc\interact_client.err.log" -PassThru
$ok = $false
foreach ($i in 1..45) {
    Start-Sleep 1
    try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {}
}
if (-not $ok) { throw '未进入游戏' }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
Start-Sleep 2

$results = @()
foreach ($k in $kinds) {
    $pascal = Pascal $k
    Rpc 'dialog' @{ kind = $k; action = 'open' } | Out-Null
    Start-Sleep -Milliseconds 800
    $rect = Rpc 'dialog_rect' @{ kind = $k }
    if (-not $rect.ok) {
        if ($noCloseByDesign -contains $k) {
            # 无钮设计窗：仅验证 open/close RPC 往返
            Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null
            Start-Sleep -Milliseconds 300
            $after = ((Rpc 'dialogs').dialogs) -join ','
            $rpcClosed = if ($after -notmatch "\b$pascal\b") { 'YES' } else { 'NO' }
            $results += [pscustomobject]@{ kind=$k; open='NO_BTN_BY_DESIGN'; hit=''; closed=$rpcClosed }
            Write-Host ("{0,-20} 无关闭钮(设计) RPC 往返 closed={1}" -f $k, $rpcClosed)
        } else {
            $results += [pscustomobject]@{ kind=$k; open='FAIL_NO_BTN'; hit=''; closed='-' }
            Write-Host ("{0,-20} FAIL: 找不到标准关闭钮" -f $k)
            # 兜底关窗，避免残留遮挡后续窗口
            Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null
            Start-Sleep -Milliseconds 300
        }
        continue
    }
    # 关闭钮中心（dialog_rect 返回 CloseButton 标记实体的布局后中心，逻辑坐标）
    $cx = [math]::Round($rect.cx, 1)
    $cy = [math]::Round($rect.cy, 1)
    $click = Rpc 'click' @{ x = $cx; y = $cy }
    Start-Sleep -Milliseconds 500
    $after = ((Rpc 'dialogs').dialogs) -join ','
    $closed = if ($after -notmatch "\b$pascal\b") { 'YES' } else { 'NO' }
    $hits = ($click.hits) -join ' | '
    $results += [pscustomobject]@{ kind=$k; open='OK'; hit=$hits; closed=$closed }
    Write-Host ("{0,-20} click@({1},{2}) closed={3} hits=[{4}]" -f $k, $cx, $cy, $closed, $hits)
    # 残留兜底：没关掉就用 RPC 关掉，避免遮挡下一个窗口
    if ($closed -eq 'NO') { Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null; Start-Sleep -Milliseconds 300 }
}

# ---- 拖动测试：inventory 面板空白区拖动 (60,40) ----
# 按点避开控件：inventory 页签 x 6..218、扩容钮 x 235..283、关闭钮 x 289+、
# 网格行 4 止于 y=201、删除钮 (291,212)——按 (rx+40, ry+216) 落在面板左下空白带
# （按钮场景拖动才起拖：C# MirDialog 空白区 BudDrag）
Write-Host '--- 拖动测试: inventory ---'
Rpc 'dialog' @{ kind = 'inventory'; action = 'open' } | Out-Null
Start-Sleep -Milliseconds 800
$r0 = Rpc 'dialog_rect' @{ kind = 'inventory' }
$px = [math]::Round($r0.rx + 40, 1)
$py = [math]::Round($r0.ry + 216, 1)
$dx = [math]::Round($px + 60, 1)
$dy = [math]::Round($py + 40, 1)
Write-Host ("drag 按点: ({0},{1}) -> ({2},{3})" -f $px, $py, $dx, $dy)
$drag = Rpc 'click' @{ x = $px; y = $py; drag_to = @{ x = $dx; y = $dy } }
Start-Sleep -Milliseconds 500
$r1 = Rpc 'dialog_rect' @{ kind = 'inventory' }
$moved = ([math]::Abs($r1.cx - $r0.cx - 60) -lt 8) -and ([math]::Abs($r1.cy - $r0.cy - 40) -lt 8)
Write-Host ("drag 钮心: ({0},{1}) -> ({2},{3}) moved={4}" -f $r0.cx, $r0.cy, $r1.cx, $r1.cy, $moved)
Rpc 'dialog' @{ kind = 'inventory'; action = 'close' } | Out-Null

# ---- NPC 窗交互：实机流程开 → 点 X 关（传送/呼叫有时不成功，3 次重试） ----
Write-Host '--- NPC 窗关闭钮 ---'
$npcClosed = 'SKIP'
foreach ($tryIdx in 1..3) {
    Rpc 'chat' @{ message = '@move 296 612' } | Out-Null
    Start-Sleep 2
    $near = Rpc 'nearby'
    $smith = $near.entities | Where-Object { $_.name -match 'Smith' } | Select-Object -First 1
    if (-not $smith) { Write-Host ("npc 尝试 {0}: nearby 无 Smith" -f $tryIdx); continue }
    Rpc 'npc_call' @{ object_id = $smith.object_id; key = '[@MAIN]' } | Out-Null
    Start-Sleep -Milliseconds 1500
    $npcRect = Rpc 'dialog_rect' @{ kind = 'npc' }
    if ($npcRect.ok) {
        $cx = [math]::Round($npcRect.cx, 1)
        $cy = [math]::Round($npcRect.cy, 1)
        $click = Rpc 'click' @{ x = $cx; y = $cy }
        Start-Sleep -Milliseconds 500
        $vis = (Rpc 'visible').visible
        $npcClosed = if ($vis -notmatch 'Npc') { 'YES' } else { 'NO' }
        Write-Host ("npc X 点击 closed={0} hits=[{1}]" -f $npcClosed, (($click.hits) -join ' | '))
        break
    }
    Write-Host ("npc 尝试 {0}: 窗未开" -f $tryIdx)
}
if ($npcClosed -eq 'SKIP') { Write-Host 'npc 3 次尝试均未开（跳过 X 测试）' }

# ---- hero_manage 状态窗：X 钮 ----
Write-Host '--- hero_manage 关闭钮 ---'
Rpc 'dialog' @{ kind = 'hero_manage'; action = 'open' } | Out-Null
Start-Sleep -Milliseconds 1000
$hmRect = Rpc 'dialog_rect' @{ kind = 'hero_manage' }
if ($hmRect.ok) {
    $cx = [math]::Round($hmRect.cx, 1)
    $cy = [math]::Round($hmRect.cy, 1)
    $click = Rpc 'click' @{ x = $cx; y = $cy }
    Start-Sleep -Milliseconds 500
    $vis = (Rpc 'visible').visible
    $hmClosed = if ($vis -notmatch 'HeroManage') { 'YES' } else { 'NO' }
    Write-Host ("hero_manage X 点击 closed={0} hits=[{1}]" -f $hmClosed, (($click.hits) -join ' | '))
} else {
    Write-Host 'hero_manage 未可见（跳过 X 测试）'
}

@{ sweep = $results; drag = @{ from = $r0; to = $r1; moved = $moved } } | ConvertTo-Json -Depth 5 | Out-File "$acc\ui_interact_results.json" -Encoding utf8
Stop-Process -Id $proc.Id -Force
$pass = ($results | Where-Object { $_.closed -eq 'YES' }).Count
Write-Host ("== 交互巡回完成: {0}/{1} 关闭钮点击通过 ==" -f $pass, $results.Count)
