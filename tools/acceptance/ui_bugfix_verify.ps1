# ui_bugfix_verify.ps1 — issue #2961 批次 6 项缺陷**实机复验**
#   项2 聊天窗不可拖 / 项4 商城布局 / 项5 滚动条渲染 / 项6 拖拽穿透（腰带）
#   （项1 坐骑遮挡需坐骑物品，项3 IME 用 ime-shot.ps1 单独跑）
# 前提：mir2_server 在跑；本脚本自起客户端（--real-net --auto-enter）。
# 产物：results JSON + shots/ 截图 + 控制台逐项 PASS/FAIL。
$ErrorActionPreference = 'Stop'
# 客户端依赖 msys64/ucrt64 与 libpinyin 的 DLL：缺任一目录会以 0xC0000135 静默退出
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
$wd  = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy'
$shotDir = "$acc\shots"
New-Item -ItemType Directory -Force -Path $shotDir | Out-Null

Add-Type -AssemblyName System.Drawing
function RegionHash([string]$png, [int]$x, [int]$y, [int]$w, [int]$h) {
    $bmp = [System.Drawing.Bitmap]::FromFile($png)
    try {
        $crop = New-Object System.Drawing.Bitmap($w, $h)
        $g = [System.Drawing.Graphics]::FromImage($crop)
        $g.DrawImage($bmp, (New-Object System.Drawing.Rectangle(0,0,$w,$h)),
                     (New-Object System.Drawing.Rectangle($x,$y,$w,$h)), [System.Drawing.GraphicsUnit]::Pixel)
        $g.Dispose()
        $ms = New-Object IO.MemoryStream
        $crop.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $crop.Dispose()
        $bytes = $ms.ToArray(); $ms.Dispose()
        $sha = [Security.Cryptography.SHA256]::Create()
        ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join ''
    } finally { $bmp.Dispose() }
}
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
function Tile { $st = Rpc 'state'; return "$($st.tile_x),$($st.tile_y)" }
function Shot([string]$name) { $p = "$shotDir\$name.png"; Rpc 'screenshot' @{ path = $p } | Out-Null; Start-Sleep -Milliseconds 700; return $p }

# 世界输入通路正控制：要排除的是「UI 闸门把世界点击整个吞掉」。
# 判据因此是「点击在**世界层**产生了效果」——移动 / 攻击目标 / 召唤 NPC 三者任一都算：
# 它们都证明点击穿过了 UI 闸门到达世界交互层。**不要求必须是移动**：
# 实测村庄里玩家四周常被怪与 NPC 包围，"点空地就会走"这个前提根本不成立
# （曾因此误判：19 个候选点全落空，日志显示全是 攻击/召唤）。
# 只有"点了什么也没发生"才算可疑，且那也可能是落在不可走的非交互格。
function TryWorldMove {
    $cands = @(
        @{x=512;y=344}, @{x=472;y=344}, @{x=552;y=424}, @{x=512;y=424},
        @{x=512;y=304}, @{x=432;y=344}, @{x=592;y=424}, @{x=512;y=464},
        @{x=452;y=384}, @{x=572;y=384}, @{x=512;y=364},
        @{x=512;y=224}, @{x=512;y=544}, @{x=352;y=384}, @{x=672;y=384}
    )
    $tried = @()
    foreach ($pt in $cands) {
        $before = Tile
        $lenBefore = 0
        try { $lenBefore = (Get-Item -LiteralPath $script:clientLog -EA Stop).Length } catch { $lenBefore = 0 }
        Rpc 'click' @{ x = $pt.x; y = $pt.y } | Out-Null
        Start-Sleep -Milliseconds 1400
        $after = Tile
        $effect = ''
        if ($lenBefore -gt 0) {
            try {
                $fs = [IO.File]::Open($script:clientLog, 'Open', 'Read', 'ReadWrite')
                $fs.Seek($lenBefore, 'Begin') | Out-Null
                $sr = New-Object IO.StreamReader($fs)
                $fresh = $sr.ReadToEnd(); $sr.Close(); $fs.Close()
                if ($before -ne $after) { $effect = '移动' }
                elseif ($fresh -match 'CallNPC') { $effect = '召唤NPC' }
                elseif ($fresh -match '攻击目标') { $effect = '攻击' }
            } catch { }
        }
        if ($effect -eq '召唤NPC') {
            # 召唤会弹窗，清掉再继续（弹着的窗会把后续点击吞在 UI 层，干扰判定）
            try { Rpc 'dialog' @{ kind = 'npc'; action = 'close' } | Out-Null } catch { }
        }
        $tried += "$($pt.x),$($pt.y):$(if ($effect) { $effect } else { '无响应' })"
        if ($effect -ne '') { return @{ ok = $true; log = ($tried -join ' | ') } }
    }
    return @{ ok = $false; log = ($tried -join ' | ') }
}
# 等玩家停稳：连续两次读数相同才算静止。
# 必要性：正控制让玩家走动后**走位可能仍在进行**，紧接着的前后读数会跨到同一次
# 移动上 —— 实测因此把"拖腰带不移动玩家"误判成 FAIL（基线刚移动完就进下一项）。
function WaitSettle {
    $last = ''
    for ($i = 0; $i -lt 12; $i++) {
        $now = Tile
        if ($now -eq $last) { return $now }
        $last = $now
        Start-Sleep -Milliseconds 500
    }
    return $last
}

$script:clientLog = "$acc\bugfix_client.err.log"

$results = @()
function Rec([string]$item, [string]$check, [bool]$pass, [string]$detail) {
    $script:results += [pscustomobject]@{ item=$item; check=$check; pass=$pass; detail=$detail }
    $tag = if ($pass) { 'PASS' } else { 'FAIL' }
    Write-Host ("[{0}] {1} :: {2} — {3}" -f $tag, $item, $check, $detail)
}

# 清理 e2e 实例（只杀 --e2e-user 标记）
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    Where-Object { $_.CommandLine -match '--e2e-user' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' -WorkingDirectory $wd -RedirectStandardOut "$acc\bugfix_client.log" -RedirectStandardError "$acc\bugfix_client.err.log" -PassThru
try {
    $ok = $false
    foreach ($i in 1..45) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {} }
    if (-not $ok) { throw '未进入游戏' }
    Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
    # 进图后地图区块是流式加载的（日志 `chunk 流式加载`）：加载完成前点击不产生移动
    # （实测：刚进图 2s 就点，11 点全落空；等一会儿再点同样点位即通）。
    Start-Sleep -Seconds 6

    # ===== 基线正控制（项 2/6 共用）=====
    # 必须在**任何 UI 交互之前**、玩家刚进图的干净状态跑：中途跑会受客户端
    # 状态影响（实测踩过——第一次成功后玩家站到别处，第二批候选点整轮落空）。
    # 它证明的是「UI 闸门没有把世界点击一刀切拦死」，是两项"未移动"结论的前提。
    $wm = TryWorldMove
    if (-not $wm.ok) {
        # 第二轮：有些点击可能正赶上攻击动作/镜头移动
        Start-Sleep -Seconds 3
        $wm2 = TryWorldMove
        if ($wm2.ok) { $wm = @{ ok = $true; log = ($wm.log + ' || R2 ' + $wm2.log) } }
    }
    Rec '基线' '世界输入正控制-点击到达世界层（移动/攻击/召唤任一）' $wm.ok $wm.log
    # 基线可能触发了走位：必须等停稳，否则后续"tile 不变"类断言会跨到同一次移动上
    WaitSettle | Out-Null

    # ================= 项6：拖拽穿透（腰带） =================
    Write-Host '=== 项6 拖拽穿透（药水腰带） ==='
    $t0 = Tile
    # 腰带默认横向 (230,618) 240x38；抓 (250,630) 拖到 (450,300) → 新位 (430,288)
    $d = Rpc 'click' @{ x = 250; y = 630; drag_to = @{ x = 450; y = 300 } }
    Start-Sleep -Milliseconds 600
    $t1 = Tile
    Rec '项6' '拖腰带不移动玩家' ($t0 -eq $t1) "tile $t0 -> $t1"
    $s1 = Shot 'bug6_belt_dragged'
    # 点击拖后腰带新位置 (550,300)：不得穿透成寻路
    $c1 = Rpc 'click' @{ x = 550; y = 300 }
    Start-Sleep -Milliseconds 900
    $t2 = Tile
    Rec '项6' '点击拖后腰带不穿透' ($t1 -eq $t2) "tile $t1 -> $t2 hits=[$(($c1.hits) -join '|')]"
    # 腰带拖回原位，保持会话干净
    Rpc 'click' @{ x = 550; y = 300; drag_to = @{ x = 250; y = 630 } } | Out-Null
    Start-Sleep -Milliseconds 400

    # ================= 项2：聊天窗不可拖 =================
    Write-Host '=== 项2 聊天窗不可拖 ==='
    # 起点必须在面板**内**：C# ChatDialog = Prguse[2221] 632x68 @(230,671)
    # → UI x 230..862 / y 671..739。曾用 (150,700)（x<230 在面板外，压的是世界）
    # 导致"拖动移动玩家"假 FAIL——那是测点错，不是产品错。
    $DRAG_X = 400; $DRAG_Y = 700
    # 像素锚定取**面板左边框条**（UI x230..236 / y677..712；截图为 UI 逻辑坐标的
    # 1.5 倍）。避开三处逐帧必变的内容：聊天行文本（服务器随时推消息）、
    # 2Hz 闪烁光标、以及**输入行**（点面板会给输入框焦点 → 底色随之变化，
    # 先前拿整块面板做哈希正是栽在这，报了一次假 FAIL）。
    # 锚定区先自检两帧稳定性：不稳就不能拿它断言。
    $baseA = Shot 'bug2_chat_baseA'
    Start-Sleep -Milliseconds 900
    $baseB = Shot 'bug2_chat_baseB'
    $h0 = RegionHash $baseA 345 1015 9 53
    $hb = RegionHash $baseB 345 1015 9 53
    $anchorOk = ($h0 -eq $hb)
    $t4 = Tile
    Rpc 'click' @{ x = $DRAG_X; y = $DRAG_Y; drag_to = @{ x = 700; y = 250 } } | Out-Null
    Start-Sleep -Milliseconds 600
    $t5 = Tile
    $after = Shot 'bug2_chat_after'
    $h1 = RegionHash $after 345 1015 9 53
    Rec '项2' '拖聊天区不移动玩家' ($t4 -eq $t5) "tile $t4 -> $t5（起点 $DRAG_X,$DRAG_Y 在面板内）"
    if ($anchorOk) {
        Rec '项2' '聊天面板未跟随拖动（左边框锚定区原位）' ($h0 -eq $h1) "anchor $($h0.Substring(0,12)) vs $($h1.Substring(0,12))"
    } else {
        Rec '项2' '聊天面板未跟随拖动（左边框锚定区原位）' $false "锚定区两帧基线不稳（$($h0.Substring(0,8)) vs $($hb.Substring(0,8))）——本次无法判定，需重跑"
    }

    # ================= 项4：商城布局（C# 坐标命中） =================
    Write-Host '=== 项4 商城布局 ==='
    Rpc 'dialog' @{ kind = 'game_shop'; action = 'open' } | Out-Null
    Start-Sleep -Milliseconds 900
    $rect = Rpc 'dialog_rect' @{ kind = 'game_shop' }
    if (-not $rect.ok) { Rec '项4' '商城可开' $false 'dialog_rect 无 ok' }
    else {
        Rec '项4' '商城可开' $true "panel @($([math]::Round($rect.rx)),$([math]::Round($rect.ry)))"
        $rx = $rect.rx; $ry = $rect.ry
        # C# GameshopDialog：8 格 (152+i%4*132, 115 或 275) 125x146 → 格心 C# 坐标
        $cellCases = @(
            @{ n='格0'; x=(152+0*132+62); y=(115+73) },
            @{ n='格3'; x=(152+3*132+62); y=(115+73) },
            @{ n='格4'; x=(152+0*132+62); y=(275+73) }
        )
        foreach ($cc in $cellCases) {
            $hits = (Rpc 'click' @{ x = [math]::Round($rx + $cc.x,1); y = [math]::Round($ry + $cc.y,1) }).hits
            $hitShop = ($hits -join ' ') -match 'GameShop'
            Rec '项4' "$($cc.n) C# 坐标命中商城" $hitShop "hits=[$($hits -join '|')]"
            Start-Sleep -Milliseconds 300
        }
        # 分类行 2（C# (15,103+15*2) 一带）
        $hitsCat = (Rpc 'click' @{ x = [math]::Round($rx + 45,1); y = [math]::Round($ry + 140,1) }).hits
        Rec '项4' '分类行命中商城' ((($hitsCat -join ' ') -match 'GameShop')) "hits=[$($hitsCat -join '|')]"
        Shot 'bug4_shop' | Out-Null
    }

    # ================= 项5：滚动条（滚轮真值断言，非"看图"） =================
    Write-Host '=== 项5 滚动条（滚轮真值） ==='
    $shopShot = Shot 'bug5_shop_scrollbar'
    Rec '项5' '商城截图已取（存档用）' $true $shopShot

    # `scroll` 返回全部 UiScrollList 的真值（绝对轨道/列表矩形 + offset/total/visible/
    # step/shown）；`wheel` 在给定点注入一行滚轮。判据是 offset 变化，不靠像素。
    #
    # 目标窗选**邮件**：它的列表默认页就可见且有数据（total>visible 才滚得动）。
    # 反例行会：成员/仓库列表在默认页是 HIDDEN——#2968 的闸门正确地不吃滚轮，
    # 拿它做正控会得到"没滚动"的假 FAIL。
    Rpc 'dialog' @{ kind = 'mail'; action = 'open' } | Out-Null
    Start-Sleep -Milliseconds 1000
    $mr = Rpc 'dialog_rect' @{ kind = 'mail' }
    if (-not $mr.ok) {
        Rec '项5' '邮件窗可开' $false 'dialog_rect 无 ok'
    } else {
        $L = (Rpc 'scroll').lists | Where-Object {
            $_.shown -and $_.total -gt $_.visible -and
            $_.rx -ge $mr.rx -and $_.ry -ge $mr.ry -and
            ($_.rx + $_.rw) -le ($mr.rx + $mr.rw) -and ($_.ry + $_.rh) -le ($mr.ry + $mr.rh)
        } | Select-Object -First 1
        if ($null -eq $L) {
            Rec '项5' '邮件窗内存在可见且可滚的列表' $false '无 shown 且 total>visible 的列表（数据不足，无法判定）'
        } else {
            Rec '项5' '邮件窗内存在可见且可滚的列表' $true "total=$($L.total) visible=$($L.visible) step=$($L.step) rect=($($L.rx),$($L.ry),$($L.rw),$($L.rh))"
            $cx = [math]::Round($L.rx + $L.rw / 2, 1)
            $cy = [math]::Round($L.ry + $L.rh / 2, 1)
            $o0 = $L.offset
            # 负控：在所有列表之外滚 → offset 不得变（证明不是"滚什么都动"）
            Rpc 'wheel' @{ x = 50; y = 760; delta = 3 } | Out-Null
            Start-Sleep -Milliseconds 400
            $oN = ((Rpc 'scroll').lists | Where-Object { $_.entity -eq $L.entity }).offset
            Rec '项5' '负控-列表外滚轮不滚动' ($oN -eq $o0) "offset $o0 -> $oN"
            # 正：列表内按 step 滚一格 → offset 恰好 +step
            $step = [int]$L.step
            Rpc 'wheel' @{ x = $cx; y = $cy; delta = 1 } | Out-Null
            Start-Sleep -Milliseconds 500
            $o1 = ((Rpc 'scroll').lists | Where-Object { $_.entity -eq $L.entity }).offset
            Rec '项5' '列表内滚轮生效（offset +step）' ($o1 -eq ($o0 + $step)) "offset $o0 -> $o1（step=$step，注入点 $cx,$cy）"
            Shot 'bug5_mail_scrolled' | Out-Null
            # 回滚到顶，保持会话干净
            Rpc 'wheel' @{ x = $cx; y = $cy; delta = -99 } | Out-Null
            Start-Sleep -Milliseconds 400
            Rpc 'dialog' @{ kind = 'mail'; action = 'close' } | Out-Null
        }
    }
    Rpc 'cursor' @{} | Out-Null
    Rpc 'dialog' @{ kind = 'game_shop'; action = 'close' } | Out-Null
    Start-Sleep -Milliseconds 400
    foreach ($k in @('quest_log','storage','guild','keyboard_layout')) {
        Rpc 'dialog' @{ kind = $k; action = 'open' } | Out-Null
        Start-Sleep -Milliseconds 800
        $r = Rpc 'dialog_rect' @{ kind = $k }
        if ($r.ok) {
            Shot "bug5_$k" | Out-Null
            Rec '项5' "$k 窗口截图" $true "panel @($([math]::Round($r.rx)),$([math]::Round($r.ry)))"
        } else {
            Rec '项5' "$k 窗口截图" $false 'dialog_rect 无 ok'
        }
        Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null
        Start-Sleep -Milliseconds 300
    }
} finally {
    $results | ConvertTo-Json | Set-Content "$acc\ui_bugfix_verify_results.json" -Encoding UTF8
    $pass = ($results | Where-Object pass).Count
    Write-Host ("==== 汇总 {0}/{1} 通过 ====" -f $pass, $results.Count)
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
}
