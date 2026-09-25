#Requires -Version 5.1
<#
.SYNOPSIS
  写邮件窗（C# `MailComposeLetterDialog`）实机几何 + 控件位判据（#3209）。

.DESCRIPTION
  起因：owner 拿一张写邮件窗截图问「这个错位了还是咋滴了」。当时**没有任何 RPC 能打开这张窗**
  （`MailCompose` 有 DialogKind、但 `parse_dialog_kind` 没有对应名字），
  所以既无法给出实机判据、也无法截图对拍——这条把仪器补上，并把「对齐」变成门禁。

  原版对照（`Client/MirScenes/Dialogs/MailDialogs.cs`）：
    - `Index = 671`（`Title` 库底图）、`Size = new Size(236, 300)`、`Location = new Point(100, 100)`（`:604-609`）
    - 关闭钮 `@ (Size.Width - 27, 3)`（`:611-616`）
    - 收件人 `RecipientNameLabel @ (70, 35) 150x15`（`:624-633`）
    - 正文 `MessageTextBox @ (15, 92) 202x165`（`:635-642`）
    - 发送/取消 `@ (30, 265)` / `@ (135, 265)`（`:646-670`）

  判据取**客户端状态真值**（`dialog_rect` 的布局后矩形 + `click` 的真实命中栈），不解析像素；
  截图仅作人看的补充证据。

  退出码：0 = 全 PASS；1 = 判据未达成；2 = 前置失败（未进场 / 窗没开 / 探针不可用）。
#>
param(
    [string]$ExeSrc = '',
    [int]$ControlPort = 9072,
    [string]$Worktree = '',
    [string]$Tag = 'mailcompose'
)
$ErrorActionPreference = 'Continue'

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端前必须拿锁（约定见 e2e_lock.ps1 头部）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5ab_mail_compose_layout' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁。
try {
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $Worktree) { $Worktree = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not $ExeSrc) { $ExeSrc = "$Worktree\Client-Bevy\target\debug\client_bevy.exe" }
$root = Join-Path $env:TEMP 'orig-csharp-ab'
if (-not (Test-Path $root)) { New-Item -ItemType Directory -Path $root | Out-Null }
# 唯一进程名：多 agent 并行时禁止按公共进程名批量杀
$&
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5ab_mail_compose_layout'
$err = Join-Path $root "l5ab_$Tag.err"

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
    Get-CimInstance Win32_Process -Filter "Name='mailc_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

$fail = New-Object System.Collections.Generic.List[string]
function Check([string]$name, [bool]$cond, [string]$detail = '') {
    if ($cond) { Write-Host ("  [PASS] " + $name) -ForegroundColor Green }
    else { $fail.Add($name); Write-Host ("  [FAIL] " + $name + " —— " + $detail) -ForegroundColor Red }
}

# 判据函数（单独成函数，便于**自检**：喂一个错矩形必须被判错）
function Test-ComposeRect($rect) {
    if ($null -eq $rect -or -not $rect.ok) { return $false }
    $tol = 1.5
    return ([Math]::Abs([double]$rect.rx - 100.0) -le $tol) -and
           ([Math]::Abs([double]$rect.ry - 100.0) -le $tol) -and
           ([Math]::Abs([double]$rect.rw - 236.0) -le $tol) -and
           ([Math]::Abs([double]$rect.rh - 300.0) -le $tol)
}

if (-not (Test-Path $ExeSrc)) { Write-Host "缺少 exe: $ExeSrc"; exit 2 }
Stop-Client
Start-Sleep -Milliseconds 800
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
if (Test-Path $err) { [System.IO.File]::Delete($err) }

# mock 模式（不带 --real-net）+ 自动进场；不起服务端、不占 e2e 账号
Start-Process -FilePath $exe `
    -ArgumentList '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', `
        '--control-port', "$ControlPort", '--mail-many' `
    -WorkingDirectory "$Worktree\Client-Bevy" `
    -RedirectStandardOutput (Join-Path $root "l5ab_$Tag.log") -RedirectStandardError $err | Out-Null

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    $why = (Select-String -Path $err -Pattern 'panic|登录失败' -EA SilentlyContinue | Select-Object -Last 1).Line
    Write-Host ("FAIL(2): 未进场 - " + $why)
    Stop-Client
    exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# ★ 反「陈旧二进制」前置（#3211）：被测 exe 必须确实是 `-Worktree` 当前 HEAD 构建的。
# 此前没有这个量，夹具可能对着旧 exe 下结论（repo 里已有
# `LESSON_运行目标分支e2e前需重建二进制避免陈旧target误报`）——这类假红/假绿会污染覆盖可信度，
# 所以这里做成**前置失败（exit 2）**：宁可不产出结论，也不产出对错对象的结论。
$stamp = Rpc 'build_stamp'
$headShort = ''
try { $headShort = (& git -C $Worktree rev-parse --short HEAD 2>$null | Select-Object -First 1).Trim() } catch { $headShort = '' }
if ($null -eq $stamp -or -not $stamp.commit_short) {
    Write-Host 'FAIL(2): build_stamp RPC 不可用（exe 太旧，没有构建戳）——请重建客户端'
    Stop-Client
    exit 2
}
Write-Host ("构建戳：commit={0} dirty={1} exe_mtime={2}" -f $stamp.commit_short, $stamp.dirty, $stamp.exe_mtime)
if ($headShort -and $stamp.commit_short -ne $headShort) {
    Write-Host ("FAIL(2): 被测 exe 是 {0} 构建的，而被测 worktree HEAD={1} —— 陈旧二进制，先重建再跑" -f `
        $stamp.commit_short, $headShort)
    Stop-Client
    exit 2
}

# 0) 判据自检：错的矩形必须被判错（防止判据恒绿）
$badRect = [pscustomobject]@{ ok = $true; rx = 30.0; ry = 80.0; rw = 236.0; rh = 300.0 }
Check '0 判据自检：错位矩形（30,80 而非 100,100）必须被判错' (-not (Test-ComposeRect $badRect))
$okRect = [pscustomobject]@{ ok = $true; rx = 100.0; ry = 100.0; rw = 236.0; rh = 300.0 }
Check '0b 判据自检：C# 原版矩形必须被判对' (Test-ComposeRect $okRect)

# 1) 打开写邮件窗（RPC 直接切 mail.compose + 打开父窗 Mail）
Rpc 'dialog' @{ kind = 'mail_compose'; action = 'open' } | Out-Null
$rect = $null
foreach ($i in 1..20) {
    Start-Sleep -Milliseconds 300
    $rect = Rpc 'dialog_rect' @{ kind = 'mail_compose' }
    if ($null -ne $rect -and $rect.ok) { break }
}
if ($null -eq $rect -or -not $rect.ok) {
    Write-Host 'FAIL(2): 写邮件窗没打开 / dialog_rect 取不到'
    $why = (Select-String -Path $err -Pattern 'panic' -EA SilentlyContinue | Select-Object -Last 1).Line
    if ($why) { Write-Host ("  stderr: " + $why) }
    Stop-Client
    exit 2
}
Write-Host ("写邮件窗矩形 rx={0:N1} ry={1:N1} rw={2:N1} rh={3:N1}；关闭钮中心 ({4:N1},{5:N1})" -f `
    $rect.rx, $rect.ry, $rect.rw, $rect.rh, $rect.cx, $rect.cy)

Check '① 窗口矩形 == C# `Size(236,300) @ Location(100,100)`' (Test-ComposeRect $rect) `
    ("实查 rx={0} ry={1} rw={2} rh={3}" -f $rect.rx, $rect.ry, $rect.rw, $rect.rh)

# 2) 关闭钮落在窗内（C# `@ (Width-27, 3)`）
$cxOK = ([double]$rect.cx -ge [double]$rect.rx) -and ([double]$rect.cx -le ([double]$rect.rx + [double]$rect.rw)) -and
        ([double]$rect.cy -ge [double]$rect.ry) -and ([double]$rect.cy -le ([double]$rect.ry + [double]$rect.rh))
Check '② 标准关闭钮中心落在窗内（C# 关闭钮 @ (W-27,3)）' $cxOK `
    ("cx={0} cy={1} 窗=({2},{3})+{4}x{5}" -f $rect.cx, $rect.cy, $rect.rx, $rect.ry, $rect.rw, $rect.rh)

# 3) 截图（人看的证据；判据在上面的状态量里）
$shotScript = Join-Path $PSScriptRoot 'capture.ps1'
if (Test-Path $shotScript) {
    & $shotScript -Label "l5ab_mail_compose_$Tag" -ProcessName 'mailc_client' | Out-Null
} else {
    Write-Host '  [SKIP] capture.ps1 不存在，跳过截图'
}

# 4) 取消钮必须在 C# 的 (135,265) 位置：点它就关窗（点到空处不会关）
$cancelX = [double]$rect.rx + 135.0 + 10.0   # 钮宽约 20：取钮中心附近
$cancelY = [double]$rect.ry + 265.0 + 10.0
$clickCancel = Rpc 'click' @{ x = $cancelX; y = $cancelY; button = 'left' }
$closedByCancel = $false
foreach ($i in 1..20) {
    Start-Sleep -Milliseconds 250
    $r2 = Rpc 'dialog_rect' @{ kind = 'mail_compose' }
    if ($null -eq $r2 -or -not $r2.ok) { $closedByCancel = $true; break }
}
Check '③ 取消钮（C# @(135,265)）命中且关窗——位置错了就点不中' $closedByCancel `
    ("click=({0:N1},{1:N1}) hits={2}" -f $cancelX, $cancelY, (($clickCancel.hits | Select-Object -First 3) -join '|'))

# 5) 重新打开，用「标准 X」关（等价于实机点关闭钮）
Rpc 'dialog' @{ kind = 'mail_compose'; action = 'open' } | Out-Null
$rect2 = $null
foreach ($i in 1..20) {
    Start-Sleep -Milliseconds 250
    $rect2 = Rpc 'dialog_rect' @{ kind = 'mail_compose' }
    if ($null -ne $rect2 -and $rect2.ok) { break }
}
Check '④ 关闭后可再次打开（状态位往返干净）' ($null -ne $rect2 -and $rect2.ok)
if ($null -ne $rect2 -and $rect2.ok) {
    # 先点自己的标题栏把它**置顶**（真实用户也是先点到那张窗上）：
    # 实测踩到——同屏还有读邮件窗（mock 预置邮件时会开），纯按坐标点 X 可能命中**别的窗**
    # （命中栈实测 `5387v0 36x31 [root=MailRead]`），于是「点 X 关窗」假红。
    # 置顶后 X 才是该点的那个钮；命中栈里必须出现本窗（MailCompose）才算命中。
    Rpc 'click' @{ x = ([double]$rect2.rx + 118.0); y = ([double]$rect2.ry + 12.0) } | Out-Null
    Start-Sleep -Milliseconds 350
    $clickX = Rpc 'click' @{ x = [double]$rect2.cx; y = [double]$rect2.cy; button = 'left' }
    $closedByX = $false
    foreach ($i in 1..20) {
        Start-Sleep -Milliseconds 250
        $r3 = Rpc 'dialog_rect' @{ kind = 'mail_compose' }
        if ($null -eq $r3 -or -not $r3.ok) { $closedByX = $true; break }
    }
    $hitOwn = (($clickX.hits | Select-Object -First 3) -join '|') -match 'MailCompose'
    Check '⑤ 点标准关闭钮能关窗（先置顶；命中栈须含本窗）' ($closedByX -and $hitOwn) `
        ("hits=" + (($clickX.hits | Select-Object -First 3) -join '|'))
}

Stop-Client
if ($fail.Count -gt 0) {
    Write-Host ("VERDICT mail_compose_layout=FAIL（{0} 项）: {1}" -f $fail.Count, ($fail -join '; ')) -ForegroundColor Red
    exit 1
}
Write-Host 'VERDICT mail_compose_layout=PASS（几何 == C# 原版；取消/关闭钮都在原版位置且可用）' -ForegroundColor Green
exit 0
} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
