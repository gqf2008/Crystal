#Requires -Version 5.1
<#
.SYNOPSIS
  经验曲线实机门禁：运行时的 `max_exp` 必须等于配置里的原版曲线（且曲线本身不是占位）。

.DESCRIPTION
  背景（2026-09-27 实机定性）：任务交付夹具观察到「一次任务涨几十级」（111→167、167→200），
  根因是本机数据目录 `ServerRust/Daneo1989/Configs/ExpList.ini` 被放成了**占位文件**
  （500 条全部 =100）——升级所需经验恒为 100，按 C# 口径设计的奖励一发放就暴走。
  原版金标准（`E:\...\Crystal\Server\Configs\ExpList.ini`）是
  `Level1=100, Level2=200, Level3=300, Level4=400, Level5=600, … Level500=45400000000`。

  判据（全部取真值，不取日志）：
    A) 配置文件存在且**不是占位曲线**（至少两个不同值）——缺失/占位 ⇒ 前置不成立（exit 3）；
    B) 客户端 `bag_probe` 读到的 `level` / `max_exp` 与配置文件里 `Level<level>` 的值**逐值相等**
       （证明服务端真的把这条曲线应用到了运行时状态，而不是只躺在文件里）。

  退出码：0 = 两条都 PASS；1 = 判据 FAIL（含"配置对、运行时不对"）；2 = 前置失败（未进场/探针不可用）；
           3 = 前置不成立（本机没有该数据文件——该目录被 gitignore，属正常，不当红）。
  实机资源（客户端 + e2e 账号）必须走 `e2e_lock.ps1` 串行。
#>
param(
    [string]$ClientHome = '',
    [int]$ControlPort = 9062,
    [string]$User = 'test',
    [string]$Pass = '123456',
    # 数据目录（gitignore，不在库里）：默认主检出的 ServerRust/Daneo1989/Configs
    [string]$ConfigDir = 'E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Daneo1989\Configs'
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5ze_exp_curve'
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5ze_client.exe'
$log = "$PSScriptRoot\l5ze_exp_curve.err.log"

# ---- 前置 A：配置曲线必须存在且不是占位 ------------------------------------
$ini = Join-Path $ConfigDir 'ExpList.ini'
if (-not (Test-Path $ini)) {
    Write-Host ("FAIL(3): 本机没有 {0}（数据目录 gitignore，属正常）——跳过，不当红" -f $ini)
    exit 3
}
$curve = @{}
foreach ($line in Get-Content $ini) {
    if ($line -match '^Level(\d+)=(\d+)') { $curve[[int]$Matches[1]] = [int64]$Matches[2] }
}
$distinct = ($curve.Values | Select-Object -Unique).Count
if ($curve.Count -lt 2 -or $distinct -lt 2) {
    Write-Host ("FAIL(3): {0} 疑似占位曲线（{1} 条 / {2} 个不同值）——请用原版 C# Server/Configs/ExpList.ini 覆盖" -f $ini, $curve.Count, $distinct)
    exit 3
}
Write-Host ("[A] 曲线：{0} 条 / {1} 个不同值；抽查 Level1={2} Level5={3} Level111={4}" -f `
    $curve.Count, $distinct, $curve[1], $curve[5], $curve[111])

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5ze_exp_curve' -TimeoutSec 1800)) {
    Write-Host 'FAIL(2): 等 e2e 锁超时'
    exit 2
}
function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 2500; $c.SendTimeout = 2500
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 2500
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}
function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='l5ze_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

Get-CimInstance Win32_Process -Filter "Name='l5ze_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null } catch { }
if (Test-Path $log) { [System.IO.File]::Delete($log) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, `
        '--control-port', "$ControlPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput "$PSScriptRoot\l5ze_exp_curve.out.log" -RedirectStandardError $log | Out-Null

$st = $null
foreach ($i in 1..90) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    Write-Host 'FAIL(2): 未进场（登录失败/超时）'
    Stop-Client; Exit-E2eLock; exit 2
}
$bag = $null
foreach ($i in 1..20) { Start-Sleep 1; $bag = Rpc 'bag_probe'; if ($null -ne $bag.level) { break } }
if ($null -eq $bag -or $null -eq $bag.level) {
    Write-Host 'FAIL(2): bag_probe 不可用'
    Stop-Client; Exit-E2eLock; exit 2
}
Stop-Client
Exit-E2eLock

$lvl = [int]$bag.level
$maxExp = [int64]$bag.max_exp
$want = if ($curve.ContainsKey($lvl)) { [int64]$curve[$lvl] } else { $null }
Write-Host ("[B] 运行时：level={0} max_exp={1}；配置 curve[{0}]={2}" -f $lvl, $maxExp, $want)
if ($null -eq $want) {
    Write-Host ("FAIL(3): 配置曲线没有 Level{0} 这一档（曲线只到 {1}）" -f $lvl, ($curve.Keys | Measure-Object -Maximum).Maximum)
    exit 3
}
if ($maxExp -ne $want) {
    Write-Host ("FAIL(1): 运行时 max_exp={0} ≠ 配置 Level{1}={2} —— 服务端没有应用这条曲线" -f $maxExp, $lvl, $want)
    exit 1
}
Write-Host ("=== 全部 PASS（曲线非占位；运行时 max_exp 与配置 Level{0} 一致）===" -f $lvl)
exit 0
