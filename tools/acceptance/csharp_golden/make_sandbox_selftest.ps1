# make_sandbox_selftest.ps1 — 自证 `make_sandbox.ps1` 的「**按段**改写端口 + 回读校验」真的成立
#
# 为什么需要它：这个工具的由来就是一次真实事故——2026-09-26 用"全文替换 `Port=`"改端口没生效，
# 原版服务端仍绑 7000，和共享的 Rust 开发服**同时监听 7000**（Windows 允许，客户端可能连错）。
# 事后改成「`[Network]` 段作用域正则 + 回读校验 + 不生效即 exit 3」，但那套逻辑本身**没有自证**：
# 只要有人把段作用域退回全文替换，工具照样"看起来成功"。
#
# 本自证在一个**假沙箱**里跑真工具（不动真实沙箱/真实原版目录）：
#   ① 造 Src/Dst 两个假目录：Src 里放假的 Server.exe/Client.exe + 两个配置文件（含**诱饵**：
#      `[Logs]` 段里也放一个 `Port=7000`，用来区分"按段"与"全文"）；Dst 里先放好假 exe
#      （让工具跳过 robocopy，只走配置改写那一半）+ 待改写的配置 + junction 目标目录。
#   ② 跑 `make_sandbox.ps1 -Src <假> -Dst <假> -Port 7200`，断言：
#      `[Network] Port=7200`（两个文件都改到）、`[Launcher] Enabled=False`、**诱饵段仍是 7000**。
#   ③ 负对照：把同一份输入喂给"全文替换"的旧写法，断言诱饵段**会**被改掉
#      —— 这一步证明第 ② 步里的诱饵不是摆设，真出现回归会被抓住。
#
# 用法：pwsh tools/acceptance/csharp_golden/make_sandbox_selftest.ps1
param(
    [int]$Port = 7200
)
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'make_sandbox.ps1'
if (-not (Test-Path -LiteralPath $script)) { Write-Host "FAIL(2): 缺 $script"; exit 2 }

function Get-IniSectionValue {
    param([string]$Text, [string]$Section, [string]$Key)
    $s = [regex]::Escape($Section); $k = [regex]::Escape($Key)
    $m = [regex]::Match($Text, "(?s)\[$s\][^\[]*?$k\s*=\s*([^\r\n]*)")
    if ($m.Success) { return $m.Groups[1].Value.Trim() }
    return $null
}

$setupIni = @"
[Network]
IPAddress=127.0.0.1
Port=7000
TimeOut=10000
MaxUser=50

[Logs]
LogPath=.\Logs
Port=7000

[Game]
ServerName=Crystal
"@
$mir2Ini = @"
[Network]
IPAddress=127.0.0.1
Port=7000

[Launcher]
Enabled=True

[Logs]
Port=7000
"@

function New-FakeSandbox {
    param([string]$Root)
    foreach ($d in @('Server\Configs', 'Server\Maps', 'Client\Data', 'Client\Map', 'Client\Sound', 'Client\DirectX', 'Client\runtimes')) {
        New-Item -ItemType Directory -Force -Path (Join-Path $Root $d) | Out-Null
    }
    [IO.File]::WriteAllText((Join-Path $Root 'Server\Server.exe'), '')
    [IO.File]::WriteAllText((Join-Path $Root 'Client\Client.exe'), '')
    [IO.File]::WriteAllText((Join-Path $Root 'Server\Configs\Setup.ini'), $setupIni)
    [IO.File]::WriteAllText((Join-Path $Root 'Client\Mir2Config.ini'), $mir2Ini)
}

$ok = $true
$tmp = Join-Path $env:TEMP ("crystal_sandbox_selftest_" + [guid]::NewGuid().ToString('N').Substring(0, 8))
try {
    $src = Join-Path $tmp 'src'; $dst = Join-Path $tmp 'dst'
    New-FakeSandbox -Root $src
    New-FakeSandbox -Root $dst   # Dst 已有假 exe ⇒ 工具跳过 robocopy，只走配置改写

    & pwsh -NoProfile -File $script -Src $src -Dst $dst -Port $Port | Out-Null
    $rc = $LASTEXITCODE
    Write-Host ("① 跑 make_sandbox.ps1 -Port {0} → exit={1}（期望 0）" -f $Port, $rc)
    if ($rc -ne 0) { $ok = $false }

    $setup = [IO.File]::ReadAllText((Join-Path $dst 'Server\Configs\Setup.ini'))
    $mir2 = [IO.File]::ReadAllText((Join-Path $dst 'Client\Mir2Config.ini'))
    $a = Get-IniSectionValue -Text $setup -Section 'Network' -Key 'Port'
    $b = Get-IniSectionValue -Text $mir2 -Section 'Network' -Key 'Port'
    $launcher = Get-IniSectionValue -Text $mir2 -Section 'Launcher' -Key 'Enabled'
    $decoySetup = Get-IniSectionValue -Text $setup -Section 'Logs' -Key 'Port'
    $decoyMir2 = Get-IniSectionValue -Text $mir2 -Section 'Logs' -Key 'Port'
    Write-Host ("② Setup[Network].Port={0} Mir2[Network].Port={1} Mir2[Launcher].Enabled={2}（期望 {3}/{3}/False）" -f $a, $b, $launcher, $Port)
    Write-Host ("   诱饵段 [Logs].Port：Setup={0} Mir2={1}（期望仍是 7000 —— 证明是**按段**改的）" -f $decoySetup, $decoyMir2)
    if ("$a" -ne "$Port" -or "$b" -ne "$Port") { $ok = $false }
    if ("$launcher".ToLower() -ne 'false') { $ok = $false }
    if ("$decoySetup" -ne '7000' -or "$decoyMir2" -ne '7000') { $ok = $false }

    # ③ 负对照：旧写法（全文替换）会把诱饵段一起改掉 ⇒ 本自证的诱饵确实有区分力
    $naive = $setup -replace 'Port=7000', "Port=$Port"
    $decoyNaive = Get-IniSectionValue -Text $naive -Section 'Logs' -Key 'Port'
    Write-Host ("③ 负对照：全文替换下诱饵段 [Logs].Port={0}（期望 {1} —— 会被误改，说明诱饵有区分力）" -f $decoyNaive, $Port)
    if ("$decoyNaive" -ne "$Port") { $ok = $false }
} finally {
    if (Test-Path -LiteralPath $tmp) { [System.IO.Directory]::Delete($tmp, $true) }
}
Write-Host ("VERDICT=" + $(if ($ok) { 'PASS' } else { 'FAIL' }))
exit $(if ($ok) { 0 } else { 1 })
