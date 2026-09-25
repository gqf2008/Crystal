#Requires -Version 5.1
<#
.SYNOPSIS
  邮件列表分页键 / 页号实机夹具（#3120 ③）：按 C# `MailDialogs.cs:99-151/303-306` 验翻页与边界。

.DESCRIPTION
  原版对照（`Client/MirScenes/Dialogs/MailDialogs.cs`）：
    - `PreviousButton Prguse2[240..242] @ (102, H-55)`；点击 `if (CurrentPage <= 1) return;`
      → `SelectedMail = null; CurrentPage--; StartIndex -= 10; UpdateInterface();`（`:109-120`）
    - `PageLabel @ (120, H-55) 67x15`，文本 `"{CurrentPage} / {PageCount}"`（`:122-129`、`:306`）
    - `NextButton Prguse2[243..245] @ (192, H-55)`；`if (CurrentPage >= PageCount) return;`
      → `SelectedMail = null; CurrentPage++; StartIndex += 10;`（`:141-151`）
    - `PageCount = ceil(Mail.Count / 10)`，至少 1（`:303-304`）

  判据取**客户端状态真值**（只读 RPC `mail_probe` 的 `page` / `page_count` / `page_start` /
  `selected`），不解析像素；点击走真实 picking（`click` RPC 注入光标 + 按键），滚轮走 `wheel` RPC。
  数据用 mock 的 `--mail-many` 预置 21 封（3 页）——不起服务端、不写 DB、不占 e2e 账号。

  退出码：0 = 全 PASS；1 = 判据未达成；2 = 前置失败（未进场 / 探针不可用 / 窗口没开）；
          3 = 前置不成立（邮件数不足 21，翻页无从验起）。
#>
param(
    [string]$ExeSrc = '',
    [int]$ControlPort = 9071,
    [string]$Worktree = '',
    [int]$TimeoutSec = 120,
    [string]$Tag = 'mailpager'
)
$ErrorActionPreference = 'Continue'

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁（约定见 e2e_lock.ps1 头部）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5x_mail_pager' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（finally 在 exit 下也会执行）。
try {
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $Worktree) { $Worktree = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not $ExeSrc) { $ExeSrc = "$Worktree\Client-Bevy\target\debug\client_bevy.exe" }
$root = 'C:\Users\gxh\AppData\Local\Temp\orig-csharp-ab'
if (-not (Test-Path $root)) { New-Item -ItemType Directory -Path $root | Out-Null }
# 唯一进程名：多 agent 并行时禁止按公共进程名批量杀（见 LESSON_多agent并行时按进程名清进程...）
$&
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5x_mail_pager'
$err = "$root\l5x_$Tag.err"

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
    Get-CimInstance Win32_Process -Filter "Name='mailp_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}
function Probe { Rpc 'mail_probe' }
function Wait-Page([int]$want, [int]$ms = 4000) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalMilliseconds -lt $ms) {
        $p = Probe
        if ($null -ne $p -and $p.page -eq $want) { return $p }
        Start-Sleep -Milliseconds 150
    }
    Probe
}

$fail = New-Object System.Collections.Generic.List[string]
$results = New-Object System.Collections.Generic.List[object]
function Check([string]$name, [bool]$cond, [string]$detail = '') {
    $results.Add([pscustomobject]@{ name = $name; pass = $cond; detail = $detail })
    if ($cond) { Write-Host ("  [PASS] " + $name) -ForegroundColor Green }
    else {
        $fail.Add($name)
        Write-Host ("  [FAIL] $name —— " + $detail) -ForegroundColor Red
    }
}

if (-not (Test-Path $ExeSrc)) { Write-Host "缺少 exe: $ExeSrc"; exit 2 }
Stop-Client
Start-Sleep -Milliseconds 800
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
if (Test-Path $err) { [System.IO.File]::Delete($err) }

# mock 模式（不带 --real-net）+ 自动登录进场 + 预置 21 封邮件
Start-Process -FilePath $exe `
    -ArgumentList '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', `
        '--control-port', "$ControlPort", '--mail-many' `
    -WorkingDirectory "$Worktree\Client-Bevy" `
    -RedirectStandardOutput "$root\l5x_$Tag.log" -RedirectStandardError $err | Out-Null

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    $why = (Select-String -Path $err -Pattern 'panic|登录失败' -EA SilentlyContinue | Select-Object -Last 1).Line
    Write-Host ("FAIL(2): 未进场 - " + $why)
    Stop-Client
    exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 打开邮件窗（列表窗）
Rpc 'dialog' @{ kind = 'mail'; action = 'open' } | Out-Null
$rect = $null
foreach ($i in 1..20) {
    Start-Sleep -Milliseconds 300
    $rect = Rpc 'dialog_rect' @{ kind = 'mail' }
    if ($null -ne $rect -and $rect.ok) { break }
}
if ($null -eq $rect -or -not $rect.ok) { Write-Host 'FAIL(2): 邮件窗没打开 / dialog_rect 取不到'; Stop-Client; exit 2 }
$rx = [double]$rect.rx; $ry = [double]$rect.ry
Write-Host ("邮件窗矩形 rx={0:N1} ry={1:N1} rw={2:N1} rh={3:N1}" -f $rx, $ry, $rect.rw, $rect.rh)

# 等 21 封邮件到齐（mock 进场 2s 后推）
$p0 = $null
foreach ($i in 1..40) {
    Start-Sleep -Milliseconds 300
    $p0 = Probe
    if ($null -ne $p0 -and $p0.count -ge 21) { break }
}
if ($null -eq $p0 -or $null -eq $p0.count) { Write-Host 'FAIL(2): mail_probe 不可用'; Stop-Client; exit 2 }
if ($p0.count -lt 21) {
    Write-Host ("FAIL(3): 前置不成立——只看到 {0} 封邮件（--mail-many 应推 21 封）" -f $p0.count)
    Stop-Client
    exit 3
}
Write-Host ("邮件数={0}，初始 page={1}/{2} page_start={3} selected={4}" -f `
    $p0.count, $p0.page, $p0.page_count, $p0.page_start, $p0.selected)

# 分子：期望 C# 的 PageCount = ceil(21/10) = 3，初始页 1、页首 0
Check '① 初始页 = 1 且 PageCount = ceil(21/10) = 3' `
    ($p0.page -eq 1 -and $p0.page_count -eq 3) ("page={0} page_count={1}" -f $p0.page, $p0.page_count)
Check '② 初始页首 StartIndex = 0（= (1-1)*10）' ($p0.page_start -eq 0) ("page_start={0}" -f $p0.page_start)

# 选中一行（用于验证「翻页清空选中」= C# SelectedMail = null）
$rowY = $ry + 55.0 + 33.0 * 0.5
Rpc 'click' @{ x = ($rx + 150.0); y = $rowY } | Out-Null
Start-Sleep -Milliseconds 800
$sel = Probe
Check '③ 点第 1 行 → 选中它（真实 picking 命中行）' ($null -ne $sel.selected) ("selected={0}" -f $sel.selected)

# 点「下一页」：按钮矩形 = (192,389)+原生尺寸，点 (192+7,389+7) 必在框内
# 注意 `click` RPC 的参数名是 `{x,y}`（逻辑坐标），不是 `{pos:[x,y]}`——传错会静默不点。
$nextX = $rx + 199.0; $nextY = $ry + 396.0
$prevX = $rx + 109.0; $prevY = $ry + 396.0
Rpc 'click' @{ x = $nextX; y = $nextY } | Out-Null
$p1 = Wait-Page 2
Check '④ 点下一页 → CurrentPage 2（真 picking 命中分页键）' ($p1.page -eq 2) ("page={0}" -f $p1.page)
Check '⑤ 翻页清空选中（C# SelectedMail = null）' ($null -eq $p1.selected) ("selected={0}" -f $p1.selected)
Check '⑥ 第 2 页页首 StartIndex = 10（= (2-1)*10）' ($p1.page_start -eq 10) ("page_start={0}" -f $p1.page_start)

# 再点一次下一页 → 第 3 页（= PageCount = ceil(21/10)）
Rpc 'click' @{ x = $nextX; y = $nextY } | Out-Null
$p2 = Wait-Page 3
Check '⑦ 再点下一页 → CurrentPage 3（= PageCount）' ($p2.page -eq 3) ("page={0}" -f $p2.page)

# 边界：末页再点下一页应原地不动（C# `:143` `if (CurrentPage >= PageCount) return;`）
Rpc 'click' @{ x = $nextX; y = $nextY } | Out-Null
Start-Sleep -Milliseconds 900
$p3 = Probe
Check '⑧ 末页再点下一页 → 仍停在第 3 页（边界不越界）' ($p3.page -eq 3) ("page={0}" -f $p3.page)

# 末页截图（页号应显示 "3 / 3"，另有左右分页键）
$shot = "$PSScriptRoot\player_shots\l5x_mail_pager_page3.png"
New-Item -ItemType Directory -Path (Split-Path $shot) -Force | Out-Null
Rpc 'screenshot' @{ path = $shot } | Out-Null
Start-Sleep -Milliseconds 900
if (Test-Path $shot) { Write-Host ("   截图: {0}（{1} bytes）" -f $shot, (Get-Item $shot).Length) }

# 上一页 ×1 → 第 2 页；再滚轮上滚 → 第 1 页；第 1 页再点上一页 → 原地
Rpc 'click' @{ x = $prevX; y = $prevY } | Out-Null
$p4 = Wait-Page 2
Check '⑨ 点上一页 → 回到第 2 页' ($p4.page -eq 2) ("page={0}" -f $p4.page)
Rpc 'wheel' @{ x = ($rx + 150.0); y = ($ry + 200.0); delta = -1 } | Out-Null
$p5 = Wait-Page 1
Check '⑩ 列表内滚轮上滚一格 → 回到第 1 页（滚轮与分页键同一状态）' ($p5.page -eq 1) ("page={0}" -f $p5.page)
Rpc 'click' @{ x = $prevX; y = $prevY } | Out-Null
Start-Sleep -Milliseconds 900
$p6 = Probe
Check '⑪ 首页再点上一页 → 仍停在第 1 页（边界不越界）' ($p6.page -eq 1) ("page={0}" -f $p6.page)

Stop-Client
$pass = ($results | Where-Object { $_.pass }).Count
Write-Host ''
Write-Host ("===== l5x 邮件分页: pass={0} total={1} fail={2} =====" -f $pass, $results.Count, $fail.Count)
if ($fail.Count -gt 0) { $fail | ForEach-Object { Write-Host ("  FAIL  " + $_) -ForegroundColor Red }; exit 1 }
Write-Host '=== 全部 PASS ==='
exit 0

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
