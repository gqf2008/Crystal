# make_sandbox.ps1 — 原版 C#「金标准」沙箱：**按段**改写端口 + 回读校验（不生效就拒绝继续）
#
# 为什么单独做成工具：2026-09-26 实机复跑时，我用"全文正则替换 Port="改端口没生效（打到别的键/段上），
# 原版服务端仍绑 7000，**与共享 Rust 开发服同时监听 7000**（Windows 允许这种绑定，客户端可能连错）。
# 教训：改配置必须①**按段定位**（[Network] 段内）②**回读校验**③不生效就 fail-fast，别让下游带着错端口跑。
#
# 用法：pwsh make_sandbox.ps1 [-Port 7100] [-Dst <沙箱>] [-Force]
param(
    [string]$Src = 'E:\Users\gxh\Desktop\游戏相关\Crystal',
    [string]$Dst = "$env:TEMP\golden_sandbox",
    [int]$Port = 7100,
    [switch]$Force
)
$ErrorActionPreference = 'Stop'

# 用**段作用域正则**改写：[Network] 到下一个 '[' 之间的那个 Key。为什么不用逐行循环：上一版逐行写法
# 在真实文件上没生效（回读仍是 7000），而同样的段作用域正则**干跑验证过**能改对 —— 既然验证过的是它，
# 就用它，并把"回读校验"留在调用处（改不对就 fail-fast，别让下游带着错端口跑）。
function Set-IniSectionValue {
    param([string]$Text, [string]$Section, [string]$Key, [string]$Value)
    $s = [regex]::Escape($Section); $k = [regex]::Escape($Key)
    $pat = "(?s)(\[$s\][^\[]*?$k\s*=\s*)[^\r\n]*"
    if ([regex]::IsMatch($Text, $pat)) { return [regex]::Replace($Text, $pat, "`${1}$Value", 1) }
    return ($Text.TrimEnd() + "`r`n[$Section]`r`n$Key=$Value`r`n")
}

function Get-IniSectionValue {
    param([string]$Text, [string]$Section, [string]$Key)
    $s = [regex]::Escape($Section); $k = [regex]::Escape($Key)
    $m = [regex]::Match($Text, "(?s)\[$s\][^\[]*?$k\s*=\s*([^\r\n]*)")
    if ($m.Success) { return $m.Groups[1].Value.Trim() }
    return $null
}

# ---- 0) 前置 ----
foreach ($need in @('Server\Server.exe', 'Client\Client.exe')) {
    if (-not (Test-Path -LiteralPath (Join-Path $Src $need))) { Write-Host "FAIL(2): 原版目录缺 $need：$Src"; exit 2 }
}
if (Get-NetTCPConnection -LocalPort $Port -State Listen -EA SilentlyContinue) {
    Write-Host "FAIL(2): 端口 $Port 已被占用——换 -Port（本机 7000 是共享 Rust 开发服）"; exit 2
}

# ---- 1) 沙箱（幂等；-Force 时重建小文件副本，junction 不动） ----
$exists = Test-Path -LiteralPath (Join-Path $Dst 'Server\Server.exe')
if (-not $exists -or $Force) {
    Write-Host "建/更新沙箱：$Dst（小文件复制，大目录 junction 回指原版；原版目录只读）"
    New-Item -ItemType Directory -Force "$Dst\Server", "$Dst\Client" | Out-Null
    robocopy "$Src\Server" "$Dst\Server" /E /XD Maps /NFL /NDL /NJH /NJS /NP | Out-Null
    robocopy "$Src\Client" "$Dst\Client" /E /XD Data Map Sound DirectX runtimes /NFL /NDL /NJH /NJS /NP | Out-Null
} else { Write-Host "复用已有沙箱：$Dst" }
if (-not (Test-Path "$Dst\Server\Maps")) { New-Item -ItemType Junction -Path "$Dst\Server\Maps" -Target "$Src\Server\Maps" | Out-Null }
foreach ($d in @('Data', 'Map', 'Sound', 'DirectX', 'runtimes')) {
    $t = Join-Path $Src "Client\$d"
    if ((Test-Path -LiteralPath $t) -and -not (Test-Path "$Dst\Client\$d")) { New-Item -ItemType Junction -Path "$Dst\Client\$d" -Target $t | Out-Null }
}

# ---- 2) 端口：按 [Network] 段改写 + **回读校验** ----
$setup = "$Dst\Server\Configs\Setup.ini"
$t = [IO.File]::ReadAllText($setup)
[IO.File]::WriteAllText($setup, (Set-IniSectionValue -Text $t -Section 'Network' -Key 'Port' -Value "$Port"))
$got = Get-IniSectionValue -Text ([IO.File]::ReadAllText($setup)) -Section 'Network' -Key 'Port'
Write-Host ("服务端 [Network] Port 回读 = {0}" -f $got)
if ($got -ne "$Port") { Write-Host "FAIL(3): 服务端端口改写未生效（期望 $Port，回读 '$got'）——拒绝继续"; exit 3 }

$mir2 = "$Dst\Client\Mir2Config.ini"
$t = [IO.File]::ReadAllText($mir2)
$t = Set-IniSectionValue -Text $t -Section 'Network' -Key 'Port' -Value "$Port"
$t = Set-IniSectionValue -Text $t -Section 'Launcher' -Key 'Enabled' -Value 'False'
[IO.File]::WriteAllText($mir2, $t)
$gotPort = Get-IniSectionValue -Text ([IO.File]::ReadAllText($mir2)) -Section 'Network' -Key 'Port'
$gotLau = Get-IniSectionValue -Text ([IO.File]::ReadAllText($mir2)) -Section 'Launcher' -Key 'Enabled'
Write-Host ("客户端 [Network] Port 回读 = {0}；[Launcher] Enabled 回读 = {1}" -f $gotPort, $gotLau)
if ($gotPort -ne "$Port" -or "$gotLau".ToLower() -ne 'false') { Write-Host "FAIL(3): 客户端配置改写未生效——拒绝继续"; exit 3 }

Write-Host ("OK：沙箱就绪（端口 {0}），可用 Start-Process `"{1}\Server\Server.exe`" -WorkingDirectory `"{1}\Server`"" -f $Port, $Dst)
