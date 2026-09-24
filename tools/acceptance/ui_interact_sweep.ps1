#Requires -Version 5.1
<#
.SYNOPSIS
  Crystal 40 窗交互巡回——**门禁脚本**：结论看退出码，不看打印。

.DESCRIPTION
  覆盖清单来自 tools/acceptance/interact_sweep_manifest.json（单一真源；与
  Client-Bevy/src/control.rs 的 RPC 窗口登记对账，新增窗口漏登记会被 `cargo test --lib` 拦下）。
  逐窗：dialog open → dialog_rect 定位标准关闭钮 → click 点它（真实 picking→Interaction 链路）
  → 断言窗口关闭；设计上无关闭钮的窗改验 RPC open/close 往返。
  另有 inventory 拖动、NPC 会话窗 X、hero_manage X 三段专用检查。

  退出码（门禁语义）：
    0  全过（无 FAIL；SKIP 默认容忍，见 -FailOnSkip）
    1  有用例 FAIL（或 -FailOnSkip 下的 SKIP）
    2  前置不满足：产物缺失/过期、Data 资产缺失、服务端未就绪、未进图、巡回中断

  恢复交互级回归的用法：UI 改动合并前跑一次，退出码非 0 就不合。
  结果 JSON（默认 tools/acceptance/ui_interact_results.json）里 gate.exit_code 与退出码一致。

.PARAMETER RepoRoot
  代码所在检出，默认 = 本脚本所在仓库根（$PSScriptRoot\..\..）。
  产物与 Data 都按它解析——**在 worktree 里跑要传 worktree 路径**：以前这里硬编码主检出
  的绝对路径，在 worktree 里跑会静默测到另一份二进制（这就是它进不了门禁的原因之一）。

.PARAMETER ClientExe
  客户端产物，默认 <RepoRoot>\Client-Bevy\target\debug\client_bevy.exe；
  设了 CARGO_TARGET_DIR 时优先按 <CARGO_TARGET_DIR>\debug\client_bevy.exe 找。

.PARAMETER ClientWorkDir
  客户端工作目录（决定 Data 解析：客户端按 cwd\Data 找精灵）。
  默认取第一个真实含 Data\Items.Lib 的：RepoRoot、RepoRoot\Client-Bevy、
  主检出根（git common dir 的父目录，worktree 场景）、主检出\Client-Bevy。

.PARAMETER ManageServer
  自行起停 mir2_server（7000 已有人监听则直接复用，不杀别人的服务端）。

.PARAMETER AllowStaleBinary
  跳过「产物比源码旧」检查（默认检查并在过期时 exit 2——防止拿旧二进制跑出假绿）。

.PARAMETER FailOnSkip
  严格模式：SKIP 也算失败（发版/收口时建议开）。

.PARAMETER TestUser / TestPass
  自动化登录账号，默认 test/123456（与 scripts/run_real_e2e.ps1 同一测试账号）。

.PARAMETER JsonOut
  结论 JSON 路径，默认 <AccDir>\ui_interact_results.json。

.EXAMPLE
  pwsh tools/acceptance/ui_interact_sweep.ps1 -ManageServer
.EXAMPLE
  # worktree 里构建到别的 target 目录：
  pwsh tools/acceptance/ui_interact_sweep.ps1 -RepoRoot $PWD -ClientExe D:\t\debug\client_bevy.exe -ManageServer
#>
[CmdletBinding()]
param(
    [string]$RepoRoot = '',
    [string]$ClientExe = '',
    [string]$ClientWorkDir = '',
    [string]$ServerExe = '',
    [string]$ServerWorkDir = '',
    [switch]$ManageServer,
    [int]$ControlPort = 9000,
    [string]$AccDir = '',
    [string]$JsonOut = '',
    [string]$TestUser = 'test',
    [string]$TestPass = '123456',
    [switch]$AllowStaleBinary,
    [switch]$FailOnSkip
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'ui_interact_sweep' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Stop'
# PS7.3+：原生命令非零退出码默认会被当成异常（下面要读 git 的退出码）——显式关掉
if (Get-Variable PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}
# 客户端依赖 msys64/ucrt64 与 libpinyin 的 DLL：缺任一目录会以 0xC0000135 静默退出
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'

# ---------------- 结论账本 ----------------
$failures = New-Object System.Collections.Generic.List[string]
$skips = New-Object System.Collections.Generic.List[string]
$results = New-Object System.Collections.Generic.List[object]
$dragRow = $null
$srvProc = $null
$clientProc = $null
$enteredGame = $false
$head = ''

function Write-Results {
    param([int]$Code)
    $pass = ($results | Where-Object { $_.closed -eq 'YES' }).Count
    $gate = [ordered]@{
        exit_code = $Code
        pass      = $pass
        total     = $results.Count
        fail      = $failures.Count
        skip      = $skips.Count
        failures  = $failures.ToArray()
        skips     = $skips.ToArray()
        head      = $head
        client    = $ClientExe
        repo      = $RepoRoot
        time      = (Get-Date).ToString('s')
    }
    $out = [ordered]@{ sweep = $results.ToArray(); drag = $dragRow; gate = $gate }
    try {
        $json = $out | ConvertTo-Json -Depth 6
        $dir = Split-Path -Parent $JsonOut
        if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        Set-Content -Path $JsonOut -Value $json -Encoding utf8
    } catch {
        Write-Warning "结论 JSON 写入失败（不影响退出码）: $_"
    }
}

function Stop-Gate {
    param([string]$Msg, [int]$Code = 2)
    Write-Host "❌ $Msg" -ForegroundColor Red
    $failures.Add("precondition: $Msg")
    Write-Results $Code
    exit $Code
}

function Rpc([string]$method, [hashtable]$params = @{}) {
    $c = New-Object Net.Sockets.TcpClient
    $c.Connect('127.0.0.1', $ControlPort)
    $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$method; params=$params } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s)
    $line = $r.ReadLine(); $c.Close()
    if ($null -eq $line) { throw "control 无响应: $method" }
    ($line | ConvertFrom-Json).result
}
function Pascal([string]$snake) { ($snake -split '_' | ForEach-Object { $_.Substring(0,1).ToUpper() + $_.Substring(1) }) -join '' }
function Add-Fail([string]$name, [string]$why) {
    $failures.Add("${name}: $why")
    Write-Host ("FAIL  {0}: {1}" -f $name, $why) -ForegroundColor Red
}
function Add-Skip([string]$name, [string]$why) {
    $skips.Add("${name}: $why")
    Write-Host ("SKIP  {0}: {1}" -f $name, $why) -ForegroundColor Yellow
}
function Test-Port([int]$port) {
    $c = New-Object Net.Sockets.TcpClient
    try { $c.Connect('127.0.0.1', $port); $c.Close(); return $true } catch { return $false }
}
function Test-DataDir([string]$dir) { return [bool]($dir -and (Test-Path (Join-Path $dir 'Data\Items.Lib'))) }

# ---------------- 前置：检出 / 清单 / 产物 / 资产 ----------------
if (-not $RepoRoot) { $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path }
$RepoRoot = (Resolve-Path $RepoRoot).Path
if (-not (Test-Path (Join-Path $RepoRoot 'Client-Bevy\Cargo.toml'))) {
    Stop-Gate "-RepoRoot 不像 Crystal 检出（缺 Client-Bevy\Cargo.toml）：$RepoRoot"
}
if (-not $AccDir) { $AccDir = $PSScriptRoot }
if (-not $JsonOut) { $JsonOut = Join-Path $AccDir 'ui_interact_results.json' }
$manifestPath = Join-Path $AccDir 'interact_sweep_manifest.json'
if (-not (Test-Path $manifestPath)) { Stop-Gate "缺覆盖清单：$manifestPath" }
$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
$kinds = @($manifest.sweep)
$noCloseByDesign = @($manifest.no_close_by_design)
if ($kinds.Count -eq 0) { Stop-Gate "覆盖清单 sweep 为空：$manifestPath" }

if (-not $ClientExe) {
    $cands = @()
    if ($env:CARGO_TARGET_DIR) { $cands += (Join-Path $env:CARGO_TARGET_DIR 'debug\client_bevy.exe') }
    $cands += (Join-Path $RepoRoot 'Client-Bevy\target\debug\client_bevy.exe')
    $ClientExe = $cands | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $ClientExe) {
        Stop-Gate ("找不到客户端产物，候选：`n    " + ($cands -join "`n    ") + "`n  先构建：cd Client-Bevy; cargo build --bin client_bevy（或用 -ClientExe 指定）")
    }
}
if (-not (Test-Path $ClientExe)) { Stop-Gate "-ClientExe 指向的文件不存在：$ClientExe" }
$ClientExe = (Resolve-Path $ClientExe).Path

if (-not $AllowStaleBinary) {
    $crate = Join-Path $RepoRoot 'Client-Bevy'
    $srcFiles = @(Get-ChildItem (Join-Path $crate 'src') -Recurse -File -Filter *.rs -ErrorAction SilentlyContinue)
    foreach ($extra in @('Cargo.toml', 'build.rs')) {
        $p = Join-Path $crate $extra
        if (Test-Path $p) { $srcFiles += Get-Item $p }
    }
    if ($srcFiles.Count -gt 0) {
        $newestItem = $srcFiles | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        $exeTime = (Get-Item $ClientExe).LastWriteTime
        if ($exeTime -lt $newestItem.LastWriteTime) {
            Stop-Gate ("产物过期：{0}`n  exe  {1:yyyy-MM-dd HH:mm:ss}`n  源码 {2:yyyy-MM-dd HH:mm:ss}（{3}）`n  先 cargo build；确要跑旧产物加 -AllowStaleBinary" -f `
                $ClientExe, $exeTime, $newestItem.LastWriteTime, $newestItem.Name)
        }
    }
}

$explicitWorkDir = $ClientWorkDir
if (-not $explicitWorkDir) {
    $wcands = @($RepoRoot, (Join-Path $RepoRoot 'Client-Bevy'))
    # worktree 场景：Data 不入库、只存在于主检出——用 git common dir 的父目录找回主检出根
    try {
        $common = & git -C $RepoRoot rev-parse --path-format=absolute --git-common-dir 2>$null
        if ($LASTEXITCODE -eq 0 -and $common) {
            $mainRoot = Split-Path -Parent (($common | Select-Object -First 1).Trim())
            $wcands += $mainRoot
            $wcands += (Join-Path $mainRoot 'Client-Bevy')
        }
    } catch {
        Write-Host "（git 不可用，跳过主检出 Data 回退：$_）" -ForegroundColor DarkGray
    }
    $ClientWorkDir = $wcands | Where-Object { Test-DataDir $_ } | Select-Object -First 1
}
if (-not (Test-DataDir $ClientWorkDir)) {
    Stop-Gate "找不到 Data\Items.Lib（缺精灵时每个窗口都会假 FAIL）：ClientWorkDir=$ClientWorkDir，用 -ClientWorkDir 指定含 Data 的目录"
}
try { $head = (& git -C $RepoRoot rev-parse --short HEAD 2>$null | Select-Object -First 1) } catch { $head = '' }
Write-Host ("交互巡回: repo={0} head={1}" -f $RepoRoot, $head)
Write-Host ("          exe={0}" -f $ClientExe)
Write-Host ("          workdir={0} control=127.0.0.1:{1}" -f $ClientWorkDir, $ControlPort)

try {
    # ---------------- 前置：服务端 ----------------
    if (-not (Test-Port 7000)) {
        if (-not $ManageServer) {
            Stop-Gate "服务端未就绪（127.0.0.1:7000 无监听）：先起 mir2_server，或加 -ManageServer 由本脚本代起"
        }
        if (-not $ServerWorkDir) { $ServerWorkDir = Join-Path $RepoRoot 'ServerRust' }
        if (-not $ServerExe) { $ServerExe = Join-Path $RepoRoot 'ServerRust\target\debug\mir2_server.exe' }
        if (-not (Test-Path $ServerExe)) { Stop-Gate "找不到服务端产物：$ServerExe（先 cd ServerRust; cargo build --bin mir2_server）" }
        $srvOut = Join-Path $AccDir 'interact_server.log'
        $srvErr = Join-Path $AccDir 'interact_server.err.log'
        $srvProc = Start-Process -FilePath $ServerExe -WorkingDirectory $ServerWorkDir `
            -RedirectStandardOutput $srvOut -RedirectStandardError $srvErr -PassThru -WindowStyle Hidden
        $up = $false
        foreach ($i in 1..60) { Start-Sleep 1; if (Test-Port 7000) { $up = $true; break } }
        if (-not $up) {
            $alive = [bool](Get-Process -Id $srvProc.Id -EA SilentlyContinue)
            $tail = ''
            if (Test-Path $srvErr) { $tail = (Get-Content $srvErr -Tail 8 -EA SilentlyContinue) -join "`n    " }
            Stop-Gate ("服务端起不来（等 60s 仍无 7000 监听；进程存活={0}）`n    日志 {1} 尾：`n    {2}`n    （worktree 里跑要传 -ServerExe/-ServerWorkDir：数据库与地图在带 data\crystal.db、Daneo1989 的那份检出里）" -f $alive, $srvErr, $tail)
        }
        Write-Host ("服务端已启动 PID={0}" -f $srvProc.Id)
    } else {
        Write-Host '服务端已在监听 7000（复用；本脚本不会杀它）'
    }

    # ---------------- 起客户端 ----------------
    # 只清理**本脚本自己这一路**（同 --control-port + --e2e-user）的残留实例。
    # 早前只按进程名 client_bevy.exe + --e2e-user 过滤，会把**别的 agent 正在跑的实机验收**一起杀掉，
    # 对方随后登录就拿到 result=4 密码错误（服务端实为 Account already online）——那是资源互斥假红，
    # 不是产品缺陷，而且会让对方白查半天。锁已经把「同时只跑一组」管住了，这里只收自己那一路的残骸。
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        Where-Object { $_.CommandLine -match '--e2e-user' -and $_.CommandLine -match "--control-port\s+$ControlPort\b" } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 800
    $clientArgs = @('--real-net', '--auto-enter', '--control-port', "$ControlPort", '--e2e-user', $TestUser, '--e2e-pass', $TestPass)
    $clientProc = Start-Process -FilePath $ClientExe -ArgumentList $clientArgs -WorkingDirectory $ClientWorkDir `
        -RedirectStandardOutput (Join-Path $AccDir 'interact_client.log') `
        -RedirectStandardError (Join-Path $AccDir 'interact_client.err.log') -PassThru
    $ok = $false
    foreach ($i in 1..45) {
        Start-Sleep 1
        try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {}
        if (-not (Get-Process -Id $clientProc.Id -EA SilentlyContinue)) { break }
    }
    if (-not $ok) {
        $tail = ''
        $errLog = Join-Path $AccDir 'interact_client.err.log'
        if (Test-Path $errLog) { $tail = (Get-Content $errLog -Tail 8 -EA SilentlyContinue) -join "`n    " }
        $hint = ''
        if ($tail -match 'result=4|密码错误|already online|已在线') {
            $hint = "`n    提示：日志里出现 result=4 —— 多半是**账号被没走锁的会话占着**（服务端实为 Account already online）。" +
                    "`n    重跑本脚本修不了它：先看 pwsh tools/acceptance/e2e_lock.ps1 里的 Get-E2eLockInfo（谁在持锁），" +
                    "`n    并确认有别的 agent 在直接起客户端/跑没接入锁的脚本。"
        }
        Stop-Gate "未进入游戏（客户端提前退出？PATH/libpinyin 缺 DLL 会 0xC0000135 静默退）$hint`n    日志尾：`n    $tail"
    }
    $enteredGame = $true
    Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
    Start-Sleep 2

    # ---------------- 逐窗：open → 点关闭钮 → 断言关闭 ----------------
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
                $results.Add([pscustomobject]@{ kind=$k; open='NO_BTN_BY_DESIGN'; hit=''; closed=$rpcClosed })
                if ($rpcClosed -ne 'YES') {
                    Add-Fail $k '无钮设计窗的 RPC open/close 往返没关掉（关不掉的窗会遮挡后续窗）'
                } else {
                    Write-Host ("{0,-20} 无关闭钮(设计) RPC 往返 closed=YES" -f $k)
                }
            } else {
                $results.Add([pscustomobject]@{ kind=$k; open='FAIL_NO_BTN'; hit=''; closed='-' })
                Add-Fail $k '找不到标准关闭钮（spawn_close_button 没挂上 CloseButton，或该窗没显示）'
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
        $results.Add([pscustomobject]@{ kind=$k; open='OK'; hit=$hits; closed=$closed })
        if ($closed -eq 'YES') {
            Write-Host ("{0,-20} click@({1},{2}) closed=YES" -f $k, $cx, $cy)
        } else {
            Add-Fail $k ("点关闭钮 ({0},{1}) 后窗口仍在（hits=[{2}]）——钮被遮挡/无响应/缺关闭接线" -f $cx, $cy, $hits)
        }
        # 残留兜底：没关掉就用 RPC 关掉，避免遮挡下一个窗口
        if ($closed -eq 'NO') { Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null; Start-Sleep -Milliseconds 300 }
    }

    # ---------------- 拖动测试：inventory 面板空白区 ----------------
    # 按点避开控件：inventory 页签 x 6..218、扩容钮 x 235..283、关闭钮 x 289+、
    # 网格行 4 止于 y=201、删除钮 (291,212)——按 (rx+40, ry+216) 落在面板左下空白带
    # （按钮场景拖动才起拖：C# MirDialog 空白区 BudDrag）
    Write-Host '--- 拖动测试: inventory ---'
    Rpc 'dialog' @{ kind = 'inventory'; action = 'open' } | Out-Null
    Start-Sleep -Milliseconds 800
    $r0 = Rpc 'dialog_rect' @{ kind = 'inventory' }
    if (-not $r0.ok) {
        # 拖动段跑不了也要继续后面的 NPC/hero 段：门禁要尽量多验，别在第一个断点就收摊
        Add-Fail '(drag)inventory' 'inventory 打不开或无标准关闭钮——拖动段无法进行（该窗在逐窗段已单独记过）'
        $dragRow = @{ from = $r0; to = $null; moved = $false; dx = 60; dy = 40 }
        $results.Add([pscustomobject]@{ kind='(drag)inventory'; open='FAIL_NO_BTN'; hit=''; closed='-' })
    } else {
        $px = [math]::Round($r0.rx + 40, 1)
        $py = [math]::Round($r0.ry + 216, 1)
        $dx = [math]::Round($px + 60, 1)
        $dy = [math]::Round($py + 40, 1)
        Write-Host ("drag 按点: ({0},{1}) -> ({2},{3})" -f $px, $py, $dx, $dy)
        $drag = Rpc 'click' @{ x = $px; y = $py; drag_to = @{ x = $dx; y = $dy } }
        Start-Sleep -Milliseconds 500
        $r1 = Rpc 'dialog_rect' @{ kind = 'inventory' }
        $moved = [bool]($r1.ok -and ([math]::Abs($r1.cx - $r0.cx - 60) -lt 8) -and ([math]::Abs($r1.cy - $r0.cy - 40) -lt 8))
        $dragRow = @{ from = $r0; to = $r1; moved = $moved; dx = 60; dy = 40 }
        $results.Add([pscustomobject]@{ kind='(drag)inventory'; open='OK'; hit=''; closed=$(if ($moved) { 'YES' } else { 'NO' }) })
        if ($moved) {
            Write-Host ("drag 钮心: ({0},{1}) -> ({2},{3}) moved=YES" -f $r0.cx, $r0.cy, $r1.cx, $r1.cy)
        } else {
            Add-Fail '(drag)inventory' ("空白区拖动 60,40 未生效：钮心 ({0},{1}) -> ({2},{3})（期望 +60,+40，容差 8px）" -f $r0.cx, $r0.cy, $r1.cx, $r1.cy)
        }
        Rpc 'dialog' @{ kind = 'inventory'; action = 'close' } | Out-Null
    }

    # ---------------- NPC 窗交互：实机流程开 → 点 X 关（传送/呼叫有时不成功，3 次重试） ----------------
    Write-Host '--- NPC 窗关闭钮 ---'
    $npcRect = $null
    foreach ($tryIdx in 1..3) {
        # 锚点用 MirDB 记录且实测确认的仓库 NPC：D002 @ 174,216（同 l5d_npc_link.ps1）。
        # 旧写法 @move 296 612 + 按名字 'Smith' 匹配：出生点根本没有 Smith 这个 NPC，
        # 三次重试必 SKIP（2026-09-23 实测）。改为跨图传送 + 按 kind='npc' 取最近，
        # 不依赖具体 NPC 名字。
        Rpc 'chat' @{ message = '@mapmove D002 174 217' } | Out-Null
        Start-Sleep 4
        $near = Rpc 'nearby' @{ radius = 2000 }
        $npc = $near.entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
        if (-not $npc) { Write-Host ("npc 尝试 {0}: nearby 无 NPC" -f $tryIdx); continue }
        Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
        Start-Sleep -Milliseconds 1500
        $npcRect = Rpc 'dialog_rect' @{ kind = 'npc' }
        if ($npcRect.ok) { break }
        Write-Host ("npc 尝试 {0}: 窗未开" -f $tryIdx)
    }
    if ($npcRect -and $npcRect.ok) {
        $cx = [math]::Round($npcRect.cx, 1)
        $cy = [math]::Round($npcRect.cy, 1)
        $click = Rpc 'click' @{ x = $cx; y = $cy }
        Start-Sleep -Milliseconds 500
        $vis = (Rpc 'visible').visible
        $npcClosed = if ($vis -notmatch 'Npc') { 'YES' } else { 'NO' }
        $results.Add([pscustomobject]@{ kind='(session)npc'; open='OK'; hit=(($click.hits) -join ' | '); closed=$npcClosed })
        if ($npcClosed -eq 'YES') {
            Write-Host ("npc X 点击 closed=YES hits=[{0}]" -f (($click.hits) -join ' | '))
        } else {
            Add-Fail '(session)npc' ("NPC 会话窗点 X 后仍可见（hits=[{0}]）" -f (($click.hits) -join ' | '))
        }
    } else {
        Add-Skip '(session)npc' '3 次 @mapmove+npc_call 都没开出 NPC 窗（摆位/会话前置不满足）——NPC 关闭路径本轮未验证'
    }

    # ---------------- hero_manage 状态窗：X 钮 ----------------
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
        $results.Add([pscustomobject]@{ kind='(state)hero_manage'; open='OK'; hit=(($click.hits) -join ' | '); closed=$hmClosed })
        if ($hmClosed -eq 'YES') {
            Write-Host ("hero_manage X 点击 closed=YES hits=[{0}]" -f (($click.hits) -join ' | '))
        } else {
            Add-Fail '(state)hero_manage' ("点 X 后 HeroManage 仍可见（hits=[{0}]）" -f (($click.hits) -join ' | '))
        }
    } else {
        Add-Skip '(state)hero_manage' 'RPC 开了但窗不可见（该账号无英雄？）——hero_manage 关闭路径本轮未验证'
    }
} catch {
    Add-Fail 'sweep' "巡回中断: $_"
} finally {
    if ($clientProc -and (Get-Process -Id $clientProc.Id -EA SilentlyContinue)) {
        Stop-Process -Id $clientProc.Id -Force -EA SilentlyContinue
    }
    if ($srvProc -and (Get-Process -Id $srvProc.Id -EA SilentlyContinue)) {
        Stop-Process -Id $srvProc.Id -Force -EA SilentlyContinue
    }
}

# ---------------- 退出码：门禁结论 ----------------
# 未进图 = 前置问题（2），优先于用例结论：那种情况下"没跑到"的窗口一个都没验过
$exitCode = 0
if (-not $enteredGame) { $exitCode = 2 }
elseif ($failures.Count -gt 0) { $exitCode = 1 }
elseif ($FailOnSkip -and $skips.Count -gt 0) { $exitCode = 1 }
Write-Results $exitCode

$passCount = ($results | Where-Object { $_.closed -eq 'YES' }).Count
Write-Host ''
Write-Host ("===== 交互巡回: pass={0} total={1} fail={2} skip={3} exit={4} =====" -f `
    $passCount, $results.Count, $failures.Count, $skips.Count, $exitCode)
if ($failures.Count -gt 0) { $failures | ForEach-Object { Write-Host ("  FAIL  " + $_) -ForegroundColor Red } }
if ($skips.Count -gt 0) { $skips | ForEach-Object { Write-Host ("  SKIP  " + $_) -ForegroundColor Yellow } }
Write-Host ("结论 JSON: {0}" -f $JsonOut)
exit $exitCode

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
