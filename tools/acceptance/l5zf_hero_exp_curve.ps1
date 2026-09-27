#Requires -Version 5.1
<#
.SYNOPSIS
  英雄经验曲线实机门禁：英雄的**运行时** `max_exp` 必须等于配置曲线 `Level<英雄等级>`（且未被 u32 截断）。

.DESCRIPTION
  背景（2026-09-27）：英雄经验曲线修过两个静默缺陷 ——
  ① `load_hero_exp_list` 返回 `Vec<u32>`，原版 500 条里 **412 条 > u32::MAX**
     （`Level100=5_400_000_000`）被 `as u32` 截断；② 登录时直接用库里持久化的 `max_experience`，
     没有按曲线重算（C# `Hero.RefreshMaxExperience`）。
  两者都只在"英雄真的出场"时才可见，而本机 `heroes` 表此前 0 行 ⇒ 缺端到端证据。

  本夹具补齐这条链：
    A) 用 `seed_hero_probe.py` 幂等造一个英雄：`level=<-Level>`、`max_experience=100`（**故意过期**），
       并把 `characters.hero_index` 指到它（`@SUMMONHERO` 的前置）；
    B) 真客户端登录 → `@SUMMONHERO` 召唤英雄 → `S.HeroInformation` 下发给客户端；
    C) 读只读探针 `hero_probe` 的 `hero_max_exp`，断言它等于配置里 `Level<英雄等级>` 的值
       （同时也就证明了 >u32::MAX 的值没被截断）。

  判据取**客户端状态真值**（与 HUD 同源的 `HeroState.hero_max_exp`），不是日志。
  退出码：0 = PASS；1 = FAIL（含"仍是过期的 100"或"被截断"）；2 = 前置失败（未进场/探针不可用/召唤失败）；
           3 = 前置不成立（本机没有 HeroExpList.ini 数据文件，属正常）。
#>
param(
    [string]$ClientHome = '',
    [int]$ControlPort = 9063,
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$Level = 100,
    [string]$ConfigDir = 'E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Daneo1989\Configs',
    [string]$DbPath = 'E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Data\crystal.db',
    [int]$TimeoutSec = 60
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5zf_hero_exp_curve'
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5zf_client.exe'
$log = "$PSScriptRoot\l5zf_hero_exp_curve.err.log"

# ---- A0：配置曲线（独立基准） ------------------------------------------------
$ini = Join-Path $ConfigDir 'HeroExpList.ini'
if (-not (Test-Path $ini)) { Write-Host ("FAIL(3): 本机没有 {0}（数据目录 gitignore，属正常）" -f $ini); exit 3 }
$curve = @{}
foreach ($line in Get-Content $ini) {
    if ($line -match '^Level(\d+)=(\d+)') { $curve[[int]$Matches[1]] = [int64]$Matches[2] }
}
$want = $curve[$Level]
if ($null -eq $want) { Write-Host ("FAIL(3): 配置曲线没有 Level{0}" -f $Level); exit 3 }
Write-Host ("[A0] 曲线：{0} 条；Level{1}={2}（>u32::MAX={3}）" -f $curve.Count, $Level, $want, ($want -gt 4294967295))

# ---- A：幂等造英雄 ----------------------------------------------------------
$seedOut = & py -3.12 "$PSScriptRoot\seed_hero_probe.py" --db $DbPath --character bevychar --level $Level 2>&1
Write-Host ("[A] 造英雄：" + ($seedOut -join ' '))
if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL(2): seed_hero_probe.py 非 0 退出'; exit 2 }

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5zf_hero_exp_curve' -TimeoutSec 1800)) {
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
    Get-CimInstance Win32_Process -Filter "Name='l5zf_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

Get-CimInstance Win32_Process -Filter "Name='l5zf_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null } catch { }
if (Test-Path $log) { [System.IO.File]::Delete($log) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, `
        '--control-port', "$ControlPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput "$PSScriptRoot\l5zf_hero_exp_curve.out.log" -RedirectStandardError $log | Out-Null

$st = $null
foreach ($i in 1..90) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    Write-Host 'FAIL(2): 未进场（登录失败/超时）'
    Stop-Client; Exit-E2eLock; exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# ---- B：召唤英雄 -----------------------------------------------------------
Rpc 'chat' @{ message = '@SUMMONHERO' } | Out-Null
$probe = $null
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    Start-Sleep -Milliseconds 700
    $p = Rpc 'hero_probe'
    if ($null -ne $p -and $null -ne $p.hero_max_exp -and [int64]$p.hero_max_exp -gt 0) { $probe = $p; break }
}
if ($null -eq $probe) {
    Stop-Client; Exit-E2eLock
    & py -3.12 "$PSScriptRoot\seed_hero_probe.py" --db $DbPath --character bevychar --cleanup | Out-Null
    Write-Host ("FAIL(3): {0}s 内没拿到英雄信息（英雄未召唤？@SUMMONHERO 前置 hero_index!=0？）" -f $TimeoutSec)
    exit 3
}
Stop-Client
Exit-E2eLock
# 收尾：删掉探针英雄并把 hero_index 归 0 —— 出战的英雄会参与战斗，留着会污染其它实机夹具。
# 注意竞态：客户端退出时服务端会把**内存里的** hero_index 存档回库（实测把刚清成 0 的值写回 1），
# 所以先等断线存档落盘，再清理并回读确认，必要时重试。
for ($i = 1; $i -le 3; $i++) {
    Start-Sleep -Milliseconds 1200
    & py -3.12 "$PSScriptRoot\seed_hero_probe.py" --db $DbPath --character bevychar --cleanup | Out-Null
    $idx = & py -3.12 -c "import sqlite3;c=sqlite3.connect(r'$DbPath').cursor();print(c.execute('select hero_index from characters where name=?',('bevychar',)).fetchone()[0])" 2>$null
    if ("$idx".Trim() -eq '0') { break }
}

$got = [int64]$probe.hero_max_exp
Write-Host ("[B] hero_probe: object_id={0} hero_exp={1} hero_max_exp={2}" -f $probe.object_id, $probe.hero_exp, $got)
Write-Host ("[C] 期望 = 曲线 Level{0} = {1}" -f $Level, $want)
if ($got -eq 100 -and $want -ne 100) {
    Write-Host 'FAIL(1): 运行时仍是「过期的 100」——服务端登录没有按曲线重算英雄 max_experience'
    exit 1
}
if ($got -ne $want) {
    Write-Host ("FAIL(1): 运行时 {0} ≠ 曲线值 {1}（截断或口径不一致）" -f $got, $want)
    exit 1
}
if ($want -gt 4294967295 -and $got -le 4294967295) {
    Write-Host 'FAIL(1): >u32::MAX 的曲线值被截断了'
    exit 1
}
Write-Host '=== 全部 PASS（登录按曲线重算；值未被 u32 截断）==='
exit 0
