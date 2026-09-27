#Requires -Version 5.1
<#
.SYNOPSIS
  英雄**升级**按经验曲线推进的实机门禁（补 PR #3295 的最后一段：升级路径只有单测证据）。

.DESCRIPTION
  判据链（全部取状态真值）：
    A) 用 `seed_hero_probe.py` 幂等造英雄：`level=L`、`experience=curve[L]-1`（**差 1 点升级**）、
       `max_experience=100`（故意过期）；登录后先断言运行时 `hero_max_exp == curve[L]`（登录重算）；
    B) 召唤英雄（`@SUMMONHERO`）→ 去 Bichon 打一只怪 —— 服务端在玩家 WinExp 末尾按
       `Hero.ReduceExp(...)` 给英雄分发经验（`tick.rs` 的 `st.hero_index > 0` 分支）；
    C) 英雄经验越过阈值 ⇒ 升级 ⇒ `S.HeroLevelChanged`；断言 `hero_probe.hero_level == L+1`
       且 `hero_max_exp == curve[L+1]` —— 证明**升级路径**也按曲线取下一级所需经验
       （而不是旧的 ×1.5 回退或截断值）。

  默认 `L=4`：原版曲线 `Level1..4 = 5/5/5/5`、`Level5 = 600`，跨级后 max_exp 从 5 跳到 600，
  判据有区分度（若升级路径没查曲线，会得到 5×1.5=7 或 100）。

  退出码：0 = PASS；1 = FAIL；2 = 前置失败（未进场/未召唤/打不到怪）；3 = 前置不成立（本机无曲线数据）。
  实机资源（客户端 + e2e 账号）必须走 `e2e_lock.ps1` 串行；收尾会删掉探针英雄并回读确认。
#>
param(
    [string]$ClientHome = '',
    [int]$ControlPort = 9064,
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$Level = 4,
    [string]$MapName = '0',
    [int]$MapX = 287,
    [int]$MapY = 615,
    [string]$ConfigDir = 'E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Daneo1989\Configs',
    [string]$DbPath = 'E:\Users\gxh\Documents\GitHub\Crystal\ServerRust\Data\crystal.db',
    [int]$KillTimeoutSec = 90
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5zg_hero_levelup'
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5zg_client.exe'
$log = "$PSScriptRoot\l5zg_hero_levelup.err.log"

# ---- A0：曲线（独立基准） ----------------------------------------------------
$ini = Join-Path $ConfigDir 'HeroExpList.ini'
if (-not (Test-Path $ini)) { Write-Host ("FAIL(3): 本机没有 {0}" -f $ini); exit 3 }
$curve = @{}
foreach ($line in Get-Content $ini) { if ($line -match '^Level(\d+)=(\d+)') { $curve[[int]$Matches[1]] = [int64]$Matches[2] } }
$wantBefore = $curve[$Level]
$wantAfter = $curve[$Level + 1]
if ($null -eq $wantBefore -or $null -eq $wantAfter) { Write-Host ("FAIL(3): 曲线缺 Level{0}/{1}" -f $Level, ($Level + 1)); exit 3 }
$seedExp = [Math]::Max(0, $wantBefore - 1)
Write-Host ("[A0] 曲线：Level{0}={1}（登录期望）→ 升级后期望 Level{2}={3}；种子 experience={4}" -f $Level, $wantBefore, ($Level + 1), $wantAfter, $seedExp)

# ---- A：造英雄 --------------------------------------------------------------
$seedOut = & py -3.12 "$PSScriptRoot\seed_hero_probe.py" --db $DbPath --character bevychar `
    --level $Level --experience $seedExp --stale-max-exp 100 2>&1
Write-Host ("[A] 造英雄：" + ($seedOut -join ' '))
if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL(2): seed_hero_probe.py 非 0 退出'; exit 2 }

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5zg_hero_levelup' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }
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
    Get-CimInstance Win32_Process -Filter "Name='l5zg_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}
function Cleanup-Hero {
    for ($i = 1; $i -le 3; $i++) {
        Start-Sleep -Milliseconds 1200
        & py -3.12 "$PSScriptRoot\seed_hero_probe.py" --db $DbPath --character bevychar --cleanup | Out-Null
        $idx = & py -3.12 -c "import sqlite3;c=sqlite3.connect(r'$DbPath').cursor();print(c.execute('select hero_index from characters where name=?',('bevychar',)).fetchone()[0])" 2>$null
        if ("$idx".Trim() -eq '0') { break }
    }
}

Get-CimInstance Win32_Process -Filter "Name='l5zg_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null } catch { }
if (Test-Path $log) { [System.IO.File]::Delete($log) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, '--control-port', "$ControlPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput "$PSScriptRoot\l5zg_hero_levelup.out.log" -RedirectStandardError $log | Out-Null

$st = $null
foreach ($i in 1..90) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) { Stop-Client; Exit-E2eLock; Cleanup-Hero; Write-Host 'FAIL(2): 未进场'; exit 2 }
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# ---- B：召唤英雄 + 登录后先验（max_exp 已按曲线重算） ------------------------
Rpc 'chat' @{ message = '@SUMMONHERO' } | Out-Null
$p = $null
foreach ($i in 1..20) { Start-Sleep -Milliseconds 700; $p = Rpc 'hero_probe'; if ($null -ne $p -and [int64]$p.hero_max_exp -gt 0) { break } }
if ($null -eq $p -or [int64]$p.hero_max_exp -le 0) {
    Stop-Client; Exit-E2eLock; Cleanup-Hero
    Write-Host 'FAIL(2): 召唤后拿不到英雄信息（hero_probe 空）'; exit 2
}
Write-Host ("[B] 召唤后：hero_level={0} hero_exp={1} hero_max_exp={2}（登录期望 {3}）" -f $p.hero_level, $p.hero_exp, $p.hero_max_exp, $wantBefore)
if ([int64]$p.hero_max_exp -ne $wantBefore) {
    Stop-Client; Exit-E2eLock; Cleanup-Hero
    Write-Host ("FAIL(1): 登录重算不对：max_exp={0} ≠ 曲线 Level{1}={2}" -f $p.hero_max_exp, $Level, $wantBefore)
    exit 1
}

# ---- C：打怪 → 英雄分经验 → 升级 -------------------------------------------
Rpc 'chat' @{ message = "@mapmove $MapName $MapX $MapY" } | Out-Null
foreach ($i in 1..20) { Start-Sleep 1; $at = Rpc 'state'; if ("$($at.map)" -eq "$MapName") { break } }
Write-Host ("[C] 到图 map={0} tile=({1},{2})" -f $at.map, $at.tile_x, $at.tile_y)

$sw = [Diagnostics.Stopwatch]::StartNew()
$target = $null
$kills = 0
$lastTarget = 0
while ($sw.Elapsed.TotalSeconds -lt $KillTimeoutSec) {
    $p = Rpc 'hero_probe'
    if ($null -ne $p -and [int]$p.hero_level -ge ($Level + 1)) { break }
    $near = Rpc 'nearby' @{ radius = 5000 }
    $m = @($near.entities | Where-Object { $_.kind -eq 'monster' }) | Sort-Object dist | Select-Object -First 1
    if ($null -eq $m) { Start-Sleep -Milliseconds 600; continue }
    if ([int]$m.object_id -ne $lastTarget) { $lastTarget = [int]$m.object_id; Write-Host ("    目标怪 {0} id={1} dist={2}" -f $m.name, $m.object_id, $m.dist) }
    Rpc 'attack' @{ object_id = $m.object_id } | Out-Null
    Start-Sleep -Milliseconds 900
}

$pEnd = Rpc 'hero_probe'
Stop-Client
Exit-E2eLock
Cleanup-Hero
Write-Host ("[C] 打完：hero_level={0} hero_exp={1} hero_max_exp={2}（升级后期望 Level{3}={4}）" -f `
    $pEnd.hero_level, $pEnd.hero_exp, $pEnd.hero_max_exp, ($Level + 1), $wantAfter)
if ([int]$pEnd.hero_level -lt ($Level + 1)) {
    Write-Host ("FAIL(3): {0}s 内英雄没升级（击杀/经验分配未达成，前置不成立）" -f $KillTimeoutSec)
    exit 3
}
if ([int64]$pEnd.hero_max_exp -ne $wantAfter) {
    Write-Host ("FAIL(1): 升级后 max_exp={0} ≠ 曲线 Level{1}={2} —— 升级路径没按曲线取下一级经验" -f $pEnd.hero_max_exp, ($Level + 1), $wantAfter)
    exit 1
}
Write-Host '=== 全部 PASS（登录重算 + 升级按曲线推进）==='
exit 0
