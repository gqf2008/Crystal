# npc_sweep.ps1 — 原版 C# 侧「定位 NPC → 判页型 → 点首条链接」的可复跑工具（2026-09-28，README §3.2z）
#
# 为什么需要它：§3.2q–§3.2x 这几轮里，「点开一只 NPC」一直是散在会话里的临时配方
# （格点扫描 + 肉眼判页型 + 手算链接坐标），每轮重写一遍，而且**判页型靠肉眼反复被骗**
# （§3.2x 就被底部 QUEST 按钮的黄字骗过一次）。这里把它固化成一条命令：
#   ① 固定起点（可选 `-SetPos`：停 Client/Server → `dbtool setpos` → 重启 + 键盘登录）
#   ② 小格点网点击 + 每点截一帧（`Shot-Cs`）
#   ③ 逐帧数值化判据（`npc_page_probe.py`）：窗美术 vs `Prguse[995]` ⇒ 开没开窗；黄字带 ⇒ 页型
#   ④ 命中商人页时，点**最上面那条**链接（黄字带的带心）→ 再截一帧 ⇒ 商品窗（`Prguse[1000]`）
#
# 用法（前置：沙箱副本已建好，见 README §1）：
#   pwsh -NoProfile -File .\npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox -SetPos -PosMap 1 -PosX 296 -PosY 615
#   # 已在游戏里、只想扫一轮：
#   pwsh -NoProfile -File .\npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox
#   # 定点复戳（每行一个 "x,y"）：
#   pwsh -NoProfile -File .\npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox -Points 800,360,780,340
#   # 按 DB 的 NPC 表点名（推荐：不用盲扫，落点由世界格算出来）：
#   pwsh -NoProfile -File .\npc_sweep.ps1 -SandboxRoot $env:TEMP\golden_sandbox `
#       -NpcsJson $env:TEMP\golden_sandbox\npcs_1.json -PlayerX 296 -PlayerY 615 -MaxDist 20
#
# 产物：`<沙箱>\shots\orig_sweep_<x>_<y>.png`、`<沙箱>\shots\orig_shop_<x>_<y>.png`、
#       汇总 JSON `<OutDir>\sweep.json`（含每点页型/链接矩形/商品窗读数）。

param(
    [string]$SandboxRoot = $env:CRYSTAL_CSHARP_SANDBOX,
    [int[]]$GridX = @(200, 300, 400, 500, 600, 700, 800),
    [int[]]$GridY = @(160, 260, 360, 460, 560),
    [string[]]$Points = @(),
    [string]$NpcsJson = '',
    [int]$PlayerX = 0,
    [int]$PlayerY = 0,
    [int]$MaxDist = 24,
    [string]$Only = '',
    [string]$OutDir = '',
    [switch]$SetPos,
    [switch]$RestartPerPoint,
    # 锁屏守卫（本脚本每次跑都会**真的去点原版窗口**，锁屏下点击到不了 winit ⇒ 只会产出
    # "NPC窗=关" 的假红，白烧一轮）。默认：检测到锁屏就 exit 2，不跑。
    # 已知需要锁屏下跑的场景（例如只想验 setpos/判据本身）用 -AllowLocked 显式放行。
    [switch]$AllowLocked,
    [string]$Account = '333',
    [string]$Password = 'abbtest123',
    [int]$PosMap = 1,
    [int]$PosX = 296,
    [int]$PosY = 615,
    [switch]$NoClickLink,
    # HUD 遮挡：按 DB 算出来的落点可能正落在小地图面板上。原版小地图 `MiniMapDialog`
    # （`MainDialogs.cs:1780`）`Location = (ScreenWidth-126, 0)` = **(898,0)**、面板 126 宽，
    # 里面的 `BigMapButton`（`:1827-1838`，面板内 `(25,131)`）**点一下就开大地图**——
    # 实测点 Drapery 的格心 (936,144) 开出来的是大地图，不是 NPC 窗。默认把这块跳掉，
    # 需要它就用 `-HideMinimap`（沙箱 KeyBinds `Minimap=V`）先把小地图收起来。
    [string[]]$Avoid = @('898,0,126,150'),
    [switch]$HideMinimap,
    [int]$MinimapVk = 0x56,
    [string]$CSharpLocale = '',
    [double]$HitThreshold = 0.10,
    [int]$SettleMs = 350
)

if (-not $SandboxRoot) { throw 'pass -SandboxRoot <dir> (or set CRYSTAL_CSHARP_SANDBOX)' }
$script:SW = $SandboxRoot
if (-not $OutDir) { $OutDir = Join-Path $script:SW 'shots\sweep' }
if (-not (Test-Path -LiteralPath $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }

. "$PSScriptRoot\csharp_client_driver.ps1" -SandboxRoot $script:SW

# ---------------------------------------------------------------- 锁屏守卫
# 判据同 README §3.2ag：锁屏时前台窗口是 `Windows.UI.Core.CoreWindow`（Windows 默认锁屏界面）。
# 这道守卫是 2026-09-29 复测时补的：同一台机 02:03 / 02:06 / 02:19 连续三次都是锁屏，
# 而本脚本在锁屏下**不会报错**——它照样点击、照样截图、照样给出 `NPC窗=关`，
# 看起来像"原版点不动这只 NPC"，与 §3.2l 记载的那次假红同型。
function Test-CsDesktopLocked {
    try {
        if (-not ('CsLockProbe' -as [type])) {
            Add-Type -Namespace CsLockProbe -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern IntPtr OpenInputDesktop(int flags, bool inherit, int access);
[DllImport("user32.dll")] public static extern bool CloseDesktop(IntPtr h);
'@ | Out-Null
        }
        $fg = [CsLockProbe.U]::GetForegroundWindow()
        $sb = New-Object System.Text.StringBuilder 256
        [CsLockProbe.U]::GetClassNameW($fg, $sb, 256) | Out-Null
        $cls = $sb.ToString()
        # 第二重证据：打不开输入桌面同样是锁屏（阈值与 GetForegroundWindow 互为兜底）
        $d = [CsLockProbe.U]::OpenInputDesktop(0, $false, 0x0100)
        if ($d -ne [IntPtr]::Zero) { [CsLockProbe.U]::CloseDesktop($d) | Out-Null }
        return @{ locked = ($cls -eq 'Windows.UI.Core.CoreWindow'); cls = $cls }
    } catch {
        # 探测本身失败时**不拦**（宁可跑，也不要因为探测器的兼容问题把门禁卡死），但要说出来
        Write-Host ("[lock-guard] 锁屏探测失败（{0}）→ 不拦，继续" -f $_.Exception.Message)
        return @{ locked = $false; cls = '<probe-failed>' }
    }
}

if (-not $AllowLocked) {
    $lk = Test-CsDesktopLocked
    if ($lk.locked) {
        Write-Host ""
        Write-Host "FAIL(2)[npc_sweep]: 工作站**锁屏**中（前台窗口 class='Windows.UI.Core.CoreWindow'）。"
        Write-Host "  锁屏下 SetCursorPos/Click 到不了 winit，本脚本只会产出 'NPC窗=关' 的**假红**——"
        Write-Host "  按 owner 的边界：不硬跑，等解锁。确认要跑就加 -AllowLocked（并自行承担假红风险）。"
        exit 2
    }
    Write-Host ("[lock-guard] 桌面可交互（前台 class='{0}'）" -f $lk.cls)
}

# ------------------------------------------------ ③' 落点预算（**先算，再碰客户端**）
# 配置类错误必须在"起服务端 / 重启客户端 / 键盘登录"（每个点 ~1 分钟）**之前**拦下来。
$pts = @()
if ($NpcsJson) {
    # 从 `dbtool npcs <map>` 的 NPC 表算落点 —— 比盲扫格点确定得多。
    # C# 的点击命中是**格**判定：`MouseCell = MouseLocation/CellSize - OffSet + User.Movement`
    # （`GameScene.cs:10323`，OffSetX=1024/2/48=10、OffSetY=768/2/32-1=11），
    # 反过来，NPC 那一格的屏幕盒子是 `[(nx-ux+10)*48, (ny-uy+11)*32)` 起、48x32 大；取盒心点。
    # 注意这**不是**精灵的绘制原点（绘制是 `(dx+10)*48-10 / (dy+12)*32`），差着一个精灵锚点。
    if (-not (Test-Path -LiteralPath $NpcsJson)) { throw "NpcsJson 不存在：$NpcsJson（先跑 dbtool npcs <map>）" }
    $npcs = Get-Content -LiteralPath $NpcsJson -Raw | ConvertFrom-Json
    $cand = @()
    foreach ($n in $npcs) {
        if ($Only -and $n.fileName -notmatch $Only) { continue }
        $sx = ($n.x - $PlayerX + 10) * 48 + 24
        $sy = ($n.y - $PlayerY + 11) * 32 + 16
        $dist = [Math]::Abs($n.x - $PlayerX) + [Math]::Abs($n.y - $PlayerY)
        if ($dist -gt $MaxDist) { continue }
        if ($sx -lt 8 -or $sx -gt 1015 -or $sy -lt 8 -or $sy -gt 759) { continue }
        $cand += [pscustomobject]@{ x = $sx; y = $sy; dist = $dist; file = $n.fileName; name = $n.name }
    }
    # 起点**不能与 NPC 同格**：2026-09-29 实测 `-PosX 301 -PosY 257`（= 仓库 NPC 自己那格）时
    # 点出来的是**角色自己**，页型恒 `closed`，看图才明白（同格时角色精灵压在 NPC 上）。
    # 这类"配置错"与"原版点不动"在读数上完全一样，所以直接拦下来，别让它变成一条假结论。
    $same = @($cand | Where-Object { $_.dist -eq 0 })
    if ($same.Count -gt 0) {
        Write-Host ""
        Write-Host ("FAIL(2)[npc_sweep]: 起点与目标 NPC 同格（{0}）——点出来的是角色自己，页型会恒为 closed。" -f (($same | ForEach-Object { $_.file }) -join ','))
        Write-Host "  改法：把 -PosX/-PosY（以及 -PlayerX/-PlayerY）挪到 NPC **旁边**（距离 1~2 格），再跑。"
        exit 2
    }
    if ($cand.Count -eq 0) {
        Write-Host ""
        Write-Host ("FAIL(2)[npc_sweep]: 按 -NpcsJson/-Only/-PlayerX,-Y/-MaxDist 一个落点都没算出来（-Only='{0}' -MaxDist={1}）。" -f $Only, $MaxDist)
        Write-Host "  提示：`-Only` 匹配的是 NPC 的脚本相对路径（如 'BichonWall/Warehouse1'），不是名字；"
        Write-Host "        且可见范围只有 ±10 格（x）/±11 格（y），-PlayerX/-PlayerY 要放在目标附近。"
        exit 2
    }
    foreach ($c in ($cand | Sort-Object dist)) {
        Write-Host ("DB NPC {0} [{1}] 距离={2} → 屏幕格心 ({3},{4})" -f $c.file, $c.name, $c.dist, $c.x, $c.y)
        $pts += , @($c.x, $c.y)
    }
    Write-Host ("按 NPC 表生成 {0} 个落点" -f $pts.Count)
}

$prguse = Join-Path $script:SW 'Client\Data\Prguse.Lib'
$probe = Join-Path $PSScriptRoot 'npc_page_probe.py'

# 沙箱客户端的**语言**：C# 读的是 `Client\Localization\<Language>.json`（`Settings.cs:325`，
# `Language` 来自 `Mir2Config.ini` 的 `[Game] Language`，默认 `English`）——**不是** `Language.ini`。
# 这条以前踩过：改了 `Client\Language.ini` 的 `PriceGold` 一点效果没有，商品窗照样 `Price: 50 gold`
# （沙箱里连 `Localization\` 目录都没有，`LoadClientLanguage` 抛异常被吞掉、回落到英文内置表）。
function Set-IniValue {
    param([string]$Path, [string]$Section, [string]$Key, [string]$Value)
    $lines = [IO.File]::ReadAllLines($Path)
    $out = New-Object System.Collections.Generic.List[string]
    $inSection = $false; $done = $false
    $keyRe = '^\s*' + [regex]::Escape($Key) + '\s*='
    foreach ($ln in $lines) {
        $t = $ln.Trim()
        if ($t -match '^\[(.+)\]\s*$') {
            if ($inSection -and -not $done) { $out.Add("$Key=$Value"); $done = $true }
            $inSection = ($Matches[1].Trim() -eq $Section)
            $out.Add($ln); continue
        }
        if ($inSection -and $t -match $keyRe) { $out.Add("$Key=$Value"); $done = $true; continue }
        $out.Add($ln)
    }
    if (-not $done) { $out.Add("[$Section]"); $out.Add("$Key=$Value") }
    [IO.File]::WriteAllLines($Path, $out)
}

if ($CSharpLocale) {
    $locDir = Join-Path $script:SW 'Client\Localization'
    if (-not (Test-Path -LiteralPath $locDir)) { New-Item -ItemType Directory -Force -Path $locDir | Out-Null }
    $src = Join-Path $PSScriptRoot ("..\..\..\Client\Localization\$CSharpLocale.json")
    $dst = Join-Path $locDir "$CSharpLocale.json"
    if (-not (Test-Path -LiteralPath $src)) { throw "本地化源文件不存在：$src" }
    Copy-Item -LiteralPath $src -Destination $dst -Force
    Set-IniValue -Path (Join-Path $script:SW 'Client\Mir2Config.ini') -Section 'Game' -Key 'Language' -Value $CSharpLocale
    Write-Host "沙箱客户端语言 → $CSharpLocale（$dst + Mir2Config [Game] Language）——需要重启客户端才生效"
    if (-not $SetPos) { Write-Warning '没带 -SetPos：当前这个客户端进程仍是旧语言，要自己重启客户端再跑。' }
}

function Invoke-Probe {
    param([string[]]$Shots, [string]$JsonPath)
    $argv = @($probe, '--prguse', $prguse, '--threshold', $HitThreshold, '--json', $JsonPath)
    foreach ($s in $Shots) { $argv += @('--shot', $s) }
    $text = & py -3.12 @argv 2>&1
    Write-Host ($text -join "`n")
    if (Test-Path -LiteralPath $JsonPath) {
        return (Get-Content -LiteralPath $JsonPath -Raw | ConvertFrom-Json)
    }
    return $null
}

# ---------- ① 固定起点（可选） ----------
function Start-SandboxAt {
    # 停掉**本沙盒自己那份** Client/Server → `dbtool setpos` → 起 Server → 键盘登录。
    # 这是「同一个起点」的唯一可靠来源：格点扫描本身会把角色带走，而服务端被 Stop-Process 杀掉时
    # 不会回写位置，所以每一步都能回到同一格。
    param([int]$Map, [int]$X, [int]$Y)
    $csExe = Join-Path $script:SW 'Client\Client.exe'
    $svExe = Join-Path $script:SW 'Server\Server.exe'
    # 只清**本沙盒自己那份**进程（按 exe 路径过滤）——原版 Client/Server 是本机共享资源
    # （owner/别的 agent 可能正在跑），按进程名清场会把别人的会话带走。
    Get-CimInstance Win32_Process -Filter "Name='Client.exe' or Name='Server.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $csExe -or $_.ExecutablePath -eq $svExe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Start-Sleep -Seconds 3
    $dbtool = Join-Path $PSScriptRoot 'dbtool\bin\Release\net8.0-windows\dbtool.exe'
    if (-not (Test-Path -LiteralPath $dbtool)) { throw "dbtool 未构建：$dbtool（先 dotnet build -c Release）" }
    & $dbtool (Join-Path $script:SW 'Server') setpos $Account $Map $X $Y
    Start-Process (Join-Path $script:SW 'Server\Server.exe') -WorkingDirectory (Join-Path $script:SW 'Server') -WindowStyle Hidden
    Start-Sleep -Seconds 8
    & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'csharp_kbd_login.ps1') `
        -SandboxRoot $script:SW -Account $Account -Password $Password
}

if ($SetPos) { Start-SandboxAt -Map $PosMap -X $PosX -Y $PosY }

Init-CsClient
if ($HideMinimap) { Msg-Key $MinimapVk; Start-Sleep -Milliseconds 500; Write-Host '已收起小地图（V）——被 HUD 压住的落点这下能点到' }

$avoidRects = @()
foreach ($a in $Avoid) {
    $p = $a -split ','; if ($p.Count -eq 4) { $avoidRects += , @([int]$p[0], [int]$p[1], [int]$p[2], [int]$p[3]) }
}
function Test-Avoided([int]$px, [int]$py) {
    foreach ($r in $avoidRects) {
        if ($px -ge $r[0] -and $px -lt ($r[0] + $r[2]) -and $py -ge $r[1] -and $py -lt ($r[1] + $r[3])) { return $r }
    }
    return $null
}

# ---------- ② 归零 + 基线 ----------
function Send-Escape { [void][CsUi]::SendMessage($global:csHwnd, 0x0100, [IntPtr]0x1B, [IntPtr]::Zero); Start-Sleep -Milliseconds 200
    [void][CsUi]::SendMessage($global:csHwnd, 0x0102, [IntPtr]0x1B, [IntPtr]::Zero); Start-Sleep -Milliseconds 200
    [void][CsUi]::SendMessage($global:csHwnd, 0x0101, [IntPtr]0x1B, [IntPtr]0xC0000001); Start-Sleep -Milliseconds 300 }
Send-Escape; Send-Escape

# ---------- ③ 扫描 ----------
# `-NpcsJson` 的落点已经在 ③'（碰客户端**之前**）算好并校验过；这里只补另两种来源。
if (-not $NpcsJson) {
    if ($Points.Count -gt 0) {
        foreach ($p in $Points) { $xy = $p -split ','; $pts += , @([int]$xy[0], [int]$xy[1]) }
    } else {
        foreach ($y in $GridY) { foreach ($x in $GridX) { $pts += , @($x, $y) } }
    }
}

# ---------- ④ 逐点：点击 → 判页型 →（商人页）当场点首条链接 → 商品窗 ----------
# **必须逐点判**，不能等扫完再一起判：点击会把角色带走，等扫完时窗里已经不是那一页了
# （第一版这么写，商人页的 `View` 就没点到，反而把最后一只 Assistant 的页当成商品窗）。
$probeRows = @()
$shopProbe = @()
$shopRows = @()
foreach ($pt in $pts) {
    $x = $pt[0]; $y = $pt[1]
    if ($RestartPerPoint) {
        # 每个落点都从同一格重来：点一次 NPC 会让角色朝它挪一两格，后面按固定起点算的落点就错位
        # （实测第一击必中、后续全空）。代价是每个点 ~1 分钟（重启 + 键盘登录）。
        Start-SandboxAt -Map $PosMap -X $PosX -Y $PosY
        Init-CsClient
        if ($HideMinimap) { Msg-Key $MinimapVk; Start-Sleep -Milliseconds 500 }
    }
    $hit = Test-Avoided -px $x -py $y
    if ($hit) {
        if ($HideMinimap) { } else {
            Write-Host ("跳过 ({0},{1})：落在 HUD 矩形 {2} 上（小地图面板）——要它请加 -HideMinimap" -f $x, $y, ($hit -join ','))
            continue
        }
    }
    # 先归零：不 Escape 的话，上一击开出的窗还在，这一击就打到窗上了（截图里看到的是上一扇窗）
    Send-Escape; Send-Escape
    Move-Image $x $y
    Msg-Click $x $y 120
    Move-Image 512 760          # 把光标移开，免得下一帧的悬停高亮被当成窗内容
    Start-Sleep -Milliseconds $SettleMs
    $label = "sweep_${x}_${y}"
    $r = Shot-Cs $label
    $png = Join-Path $script:SW "shots\orig_$label.png"
    if ($r -like 'FAIL*') { Write-Warning "${label}: $r"; continue }
    Write-Host ("点击 ({0},{1}) → {2}" -f $x, $y, (Split-Path -Leaf $png))

    $one = Join-Path $OutDir "probe_${x}_${y}.json"
    $pr = Invoke-Probe -Shots @($png) -JsonPath $one
    if (-not $pr) { continue }
    $probeRows += $pr
    Write-Host ("    页型={0} 不符率={1} 链接={2}" -f $pr.page, $pr.npc_ratio, @($pr.links).Count)

    if ($NoClickLink -or $pr.page -ne 'merchant' -or -not $pr.links -or @($pr.links).Count -eq 0) { continue }
    $link = ($pr.links | Sort-Object { $_.y0 })[0]     # 最上面那条 = 商人页的 `View Store`
    $lx = [int](($link.x0 + $link.x1) / 2); $ly = [int](($link.y0 + $link.y1) / 2)
    Write-Host ("    商人页 → 点首条链接 ({0},{1})（矩形 x{2}..{3} y{4}..{5}）" -f `
            $lx, $ly, $link.x0, $link.x1, $link.y0, $link.y1)
    Move-Image $lx $ly
    Msg-Click $lx $ly 150
    Start-Sleep -Milliseconds 800
    $slabel = "shop_${x}_${y}"
    $r2 = Shot-Cs $slabel
    if ($r2 -like 'FAIL*') { Write-Warning "${slabel}: $r2"; continue }
    $spng = Join-Path $script:SW "shots\orig_$slabel.png"
    $shopRows += $spng
    $sone = Join-Path $OutDir "probe_shop_${x}_${y}.json"
    $sp = Invoke-Probe -Shots @($spng) -JsonPath $sone
    if ($sp) {
        $shopProbe += $sp
        Write-Host ("    商品窗：不符率={0} 开={1} 页型={2}" -f $sp.goods_ratio, $sp.goods_open, $sp.page)
    }
}
if ($probeRows.Count -eq 0) { throw '一帧都没判出来（客户端没在跑？）' }

[IO.File]::WriteAllText((Join-Path $OutDir 'sweep.json'), (ConvertTo-Json -InputObject $probeRows -Depth 8))
[IO.File]::WriteAllText((Join-Path $OutDir 'sweep_shop.json'), (ConvertTo-Json -InputObject $shopProbe -Depth 8))
$shopJson = Join-Path $OutDir 'sweep_shop.json'

# ---------- ⑤ 汇总 ----------
$summary = [pscustomobject]@{
    sandbox = $script:SW
    setpos  = if ($SetPos) { "$PosMap $PosX $PosY" } else { $null }
    points  = $pts.Count
    swept   = $probeRows
    shops   = $shopProbe
}
$summaryPath = Join-Path $OutDir 'sweep_summary.json'
[IO.File]::WriteAllText($summaryPath, ($summary | ConvertTo-Json -Depth 8))

Write-Host ''
Write-Host '=== 扫描汇总（页型 × 命中点） ==='
if ($probeRows) {
    $probeRows |
        Group-Object page |
        Sort-Object Name |
        ForEach-Object {
            $where = ($_.Group | ForEach-Object {
                    $mm = [regex]::Match([IO.Path]::GetFileNameWithoutExtension($_.shot), '^orig_sweep_(\d+)_(\d+)$')
                    if ($mm.Success) { "($($mm.Groups[1].Value),$($mm.Groups[2].Value))" }
                }) -join ' '
            Write-Host ("  {0,-16} {1,3} 点  {2}" -f $_.Name, $_.Count, $where)
        }
}
if ($shopProbe) {
    Write-Host '=== 商品窗 ==='
    foreach ($s in $shopProbe) {
        Write-Host ("  {0}  商品窗不符率={1} 开={2}" -f (Split-Path -Leaf $s.shot), $s.goods_ratio, $s.goods_open)
    }
}
Write-Host ("汇总：{0}" -f $summaryPath)
