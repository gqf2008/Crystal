# ab_aligned.ps1 — 「金标准」A/B 对拍：把本站 Bevy 客户端摆到与原版帧**同一状态**再截图比对
#
# 为什么必须"同状态"：金标准帧（原版 C# 客户端）是 BichonProvince @ (277,609)、背包窗 + 角色窗
# 都打开、并沿用该机的 Mir2Config.ini 设置。本站客户端若停在别的地图/别的窗口组合，逐像素差异
# 里绝大部分是"状态不同"，判不出 UI 差异（2026-09-26 第一轮就是这个坑：全帧 81% 差异里几乎
# 没有一条是可归因的 UI 缺陷）。
#
# 两条硬纪律（都来自实测踩坑，见下）：
#   1. **起客户端前断言控制端口空闲**：9090 曾被 FlClashCore.exe 占着，客户端起得来但控制通道
#      静默缺失 → `state` 永远读不到、对齐/开窗全部落空，却还能拍出一张"看起来跑过了"的图。
#      本脚本默认**自动挑一个空闲端口**（9095 起扫），显式指定时被占用即 exit 2。
#   2. **镜像金标准的设置**：原版那台机的 `Mir2Config.ini` 里 `[Game] SkillBar=False`，
#      而两端默认值都是 true。设置不同 ⇒ 顶端技能栏一边有一边没有，差异里混进非缺陷项。
#      本脚本把金标准的 `Mir2Config.ini` 原样复制到本站客户端的运行目录（[Network] 段本站
#      不读，服务端地址仍走 `config.ini`），使"同状态"包含"同设置"。
#
# 用法：
#   pwsh tools/acceptance/csharp_golden/ab_aligned.ps1 -Golden "$env:TEMP\golden_sandbox\Client\Screenshots\Image 9.png"
param(
    [string]$Repo = (Resolve-Path "$PSScriptRoot\..\..\..").Path,
    [string]$ClientHome = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend',
    [string]$Golden = "$env:TEMP\golden_sandbox\Client\Screenshots\Image 9.png",
    [string]$GoldenConfig = "$env:TEMP\golden_sandbox\Client\Mir2Config.ini",
    [int]$ControlPort = 0,
    [int]$Map = 0,
    [int]$TileX = 277,
    [int]$TileY = 609,
    [string]$Label = 'ab_bevy_aligned',
    [switch]$MirrorSettings
)
$ErrorActionPreference = 'Continue'

# ---- 0) 前置：金标准帧/配置必须在 ----
if (-not (Test-Path -LiteralPath $Golden)) { Write-Host "FAIL(2): 金标准帧不存在：$Golden"; exit 2 }
if (-not (Test-Path -LiteralPath $GoldenConfig)) { Write-Host "FAIL(2): 金标准配置不存在：$GoldenConfig"; exit 2 }

# ---- 1) 控制端口：默认自动挑空闲；显式指定则被占用即失败（绝不允许静默复用） ----
function Test-PortFree([int]$p) {
    -not (Get-NetTCPConnection -LocalPort $p -State Listen -EA SilentlyContinue)
}
if ($ControlPort -ne 0) {
    if (-not (Test-PortFree $ControlPort)) {
        Write-Host ("FAIL(2): 控制端口 {0} 已被占用（本机 9090 就被 FlClashCore.exe 占着）——换端口或先释放" -f $ControlPort)
        exit 2
    }
} else {
    $ControlPort = 0
    foreach ($p in 9095..9199) { if (Test-PortFree $p) { $ControlPort = $p; break } }
    if ($ControlPort -eq 0) { Write-Host 'FAIL(2): 9095..9199 无空闲控制端口'; exit 2 }
    Write-Host ("自动挑空闲控制端口：{0}" -f $ControlPort)
}

. "$PSScriptRoot\..\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'csharp_golden_ab_aligned' -TimeoutSec 1800)) { exit 2 }
try {
    $env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
    $env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
    $exe = Join-Path $ClientHome 'Client-Bevy\target\debug\client_bevy.exe'
    if (-not (Test-Path -LiteralPath $exe)) { Write-Host "FAIL(2): 客户端不存在：$exe"; exit 2 }
    $work = Join-Path $env:TEMP 'ab_bevy_work'
    # 构建戳前置（T11.1）：对着上个分支的旧 exe 拍出来的 A/B 结论没有意义
    . "$PSScriptRoot\..\build_stamp.ps1"
    Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName 'csharp_golden_ab_aligned'

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

    if (-not (Test-Path $work)) { New-Item -ItemType Directory -Force -Path $work | Out-Null }
    [IO.File]::WriteAllText((Join-Path $work 'config.ini'),
        "[Network]`nServerAddr=127.0.0.1:7000`nUseMock=false`n", (New-Object System.Text.UTF8Encoding($false)))
    # 设置镜像（默认开）：把金标准那台机的 Mir2Config.ini 复制过来，消除"设置不同"造成的伪差异
    if ($MirrorSettings) {
        Copy-Item -LiteralPath $GoldenConfig -Destination (Join-Path $work 'Mir2Config.ini') -Force
        Write-Host '已镜像金标准 Mir2Config.ini（含 [Game] SkillBar 等，避免伪差异）'
    }

    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $exe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 800
    Start-Process -FilePath $exe `
        -ArgumentList '--real-net', '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', '--control-port', "$ControlPort" `
        -WorkingDirectory $work -RedirectStandardOutput (Join-Path $work 'a.log') -RedirectStandardError (Join-Path $work 'a.err') | Out-Null

    # 控制通道必须**真的应答**（端口空闲的前置只保证"没被别人占"，不保证客户端起得来）
    $st = $null
    foreach ($i in 1..90) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
    if ($null -eq $st -or $null -eq $st.tile_x) {
        Write-Host ("FAIL(2): 进场失败或控制通道不可用（端口 {0}；见 {1}\a.err）" -f $ControlPort, $work)
        exit 2
    }
    Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

    if ("$($st.map)" -ne "$Map" -or $st.tile_x -ne $TileX -or $st.tile_y -ne $TileY) {
        $null = Rpc 'chat' @{ message = "@mapmove $Map $TileX $TileY" }
        foreach ($i in 1..40) { Start-Sleep -Milliseconds 500; $s2 = Rpc 'state'; if ("$($s2.map)" -eq "$Map") { break } }
    }
    $s3 = Rpc 'state'
    Write-Host ("对齐后 map={0} tile=({1},{2})" -f $s3.map, $s3.tile_x, $s3.tile_y)
    if ("$($s3.map)" -ne "$Map") { Write-Host "FAIL(2): @mapmove 未生效（仍在 map=$($s3.map)）"; exit 2 }

    # 与金标准帧相同的窗口组合：左 ITEMS I（背包）+ 右 CHAR（角色）
    $null = Rpc 'dialog' @{ kind = 'inventory'; action = 'open' }
    $null = Rpc 'dialog' @{ kind = 'character'; action = 'open' }
    Start-Sleep -Seconds 3

    # 取帧优先走**客户端自己的** `screenshot` RPC（Bevy `Screenshot::primary_window` → 直接读渲染目标）：
    # 窗口被遮挡、工作站锁屏、150% DPI 都不影响，比宿主侧 `PrintWindow` 稳（`capture.ps1` 需要能取到
    # 主窗口句柄，锁屏/无前台时会取不到窗口而拍不出图）。RPC 没落盘时再退回 `capture.ps1`。
    $shotsDir = Join-Path $Repo 'tools\acceptance\shots'
    if (-not (Test-Path -LiteralPath $shotsDir)) { New-Item -ItemType Directory -Force -Path $shotsDir | Out-Null }
    $raw = Join-Path $shotsDir "$Label.png"
    $rpcShot = Join-Path $work "$Label-rpc.png"
    [IO.File]::Delete($raw)
    [IO.File]::Delete($rpcShot)
    $null = Rpc 'screenshot' @{ path = $rpcShot }
    foreach ($i in 1..30) { Start-Sleep -Milliseconds 400; if (Test-Path -LiteralPath $rpcShot) { break } }
    if (Test-Path -LiteralPath $rpcShot) {
        Copy-Item -LiteralPath $rpcShot -Destination $raw -Force
        Write-Host ('取帧：客户端 screenshot RPC → {0}' -f $raw)
    } else {
        Write-Host '取帧：screenshot RPC 未落盘 → 回退 capture.ps1（PrintWindow）'
        $null = pwsh -NoProfile -File "$PSScriptRoot\..\capture.ps1" -Label $Label -ProcessName client_bevy
        Start-Sleep 1
    }
    if (-not (Test-Path -LiteralPath $raw)) { Write-Host "FAIL(2): 截图未生成 $raw"; exit 2 }
    $norm = Join-Path $env:TEMP "$Label`_1024.png"
    py -3.12 -c "from PIL import Image; Image.open(r'$raw').convert('RGB').resize((1024,768), Image.LANCZOS).save(r'$norm'); print('归一化 1024x768 ok')"

    $diff = Join-Path $PSScriptRoot 'shot_diff.py'
    $regions = [ordered]@{
        '全帧'            = $null
        '左上 ITEMS 区'   = '8,8,300,235'
        '右上 CHAR 区'    = '762,8,1020,372'
        '底部热键/聊天'   = '232,612,808,742'
    }
    foreach ($k in $regions.Keys) {
        $box = $regions[$k]
        Write-Host "=== $k ==="
        if ($box) { py -3.12 $diff $Golden $norm $box 2>&1 | Select-Object -Last 2 }
        else { py -3.12 $diff $Golden $norm 2>&1 | Select-Object -Last 2 }
    }
    Write-Host ("原始帧 {0}；归一化帧 {1}；金标准 {2}" -f $raw, $norm, $Golden)

    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $exe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
} finally { Exit-E2eLock }
