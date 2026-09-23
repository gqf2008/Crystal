# login_latency_probe.ps1 — 写锁下的登录时延夹具（crystal-login-latency）
#
# 判据（缺一不可）：
#   L1 写锁**真注入**（等 db_write_lock.py 输出 LOCK_HELD；拿不到就退出码 3，不产出结论——
#      2026-09-23 踩过「按 job 状态判已注入、其实没拿到锁」的假绿）
#   L2 持锁窗口内 20 次 login-only 的 **p95 < P95ThresholdSec**（默认 1.0s ——
#      这条线的物理含义是"一次写尝试的 busy_timeout 上界"：服务端 busy_timeout=800ms，
#      任何被锁住的写最多占用连接 0.8s，登录读的等待上界随之为 ~0.8s；留 1.0s 余量）
#   L3 对照（无锁）p95 也 < 阈值（排除基线本身退化）
#   L4 持锁窗口 p50 < P50LockedMaxSec（默认 0.3s）：中位数只应被『一次写尝试的等待』轻微影响
#
# 历史：修复前同条件实测 p95 = 5.575s / 5.529s / 5.234s（三版递进定位）：
#   ① 根因1：finish_login 的 save_account 在 AccountActor 里 await，等满 busy_timeout(5s)，
#      单 actor 会把后续登录一起拖住 → 把账号写挪到后台任务（登录回复不再等它）；
#   ② 根因2：登出路径的 set_account_offline 同类 → 同样挪后台；
#   ③ 根因3（最终）：被锁住的写每条占住一条池连接等 5s，登录节奏下耗尽连接池 →
#      登录的**读**取不到连接（list_ms≈5.2s；跨进程纯读 0.001s → 读本身没问题）
#      → 把连接级 busy_timeout 收到 800ms，占用上界随之为 0.8s。
#
# 用法：
#   pwsh tools/ops/login_latency_probe.ps1 -DeployDir <deploy> -ExePath <mir2_server.exe> `
#        -OutFile tools/ops/out/login_latency.json
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [Parameter(Mandatory = $true)][string]$ExePath,
    [int]$Port = 7100,
    [int]$Samples = 20,
    [int]$LockSeconds = 30,
    [double]$P95ThresholdSec = 1.0,
    [double]$P50LockedMaxSec = 0.3,
    [string]$AccountPrefix = 'opsload',
    [string]$Password = '123456',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$db = Join-Path $DeployDir 'data/crystal.db'
$stamp = [DateTime]::Now.ToString('HHmmss')
$outDir = Join-Path $ops 'out'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$log = Join-Path $outDir "login_latency_$stamp.log"
$env:RUST_LOG = 'crystal_server=info,crystal_server::actors::account=debug,crystal_server::gate=debug'

Get-Process mir2_login,mir2_server -EA SilentlyContinue | Where-Object { $_.Path -eq $ExePath } | Stop-Process -Force -EA SilentlyContinue
$srv = Start-Process -FilePath $ExePath -WorkingDirectory $DeployDir `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $outDir "login_latency_$stamp.err.log") -PassThru
$ready = $false
for ($i = 0; $i -lt 60; $i++) { Start-Sleep 1; if ((Get-Content $log -EA SilentlyContinue) -match 'Gate listening') { $ready = $true; break } }
if (-not $ready) { Write-Host 'FAIL: 服务端未就绪'; exit 9 }

function Sample([string]$acct, [string]$tag, [int]$idx) {
    $json = Join-Path $outDir "ll_${tag}_$idx.json"
    & python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --accounts $acct --password $Password `
        --sessions 1 --hold 1 --login-only > $json 2>&1
    try {
        $j = Get-Content $json -Raw | ConvertFrom-Json
        if ($j.summary.failed -ne 0) { return -1 }
        return [double]$j.sessions[0].login_reply_sec
    } catch { return -1 }
}
function Stat($vals) {
    $v = @($vals | Where-Object { $_ -ge 0 } | Sort-Object)
    if ($v.Count -eq 0) { return @{ n = 0; p50 = -1; p95 = -1; max = -1 } }
    $at = { param($q) $v[[Math]::Min($v.Count - 1, [Math]::Floor($q * $v.Count))] }
    return @{ n = $v.Count; p50 = (& $at 0.5); p95 = (& $at 0.95); max = $v[-1] }
}

$ctrl = @(); for ($i = 1; $i -le $Samples; $i++) { $ctrl += (Sample ("$AccountPrefix$i") 'ctrl' $i) }
$ctrlStat = Stat $ctrl

# 注入写锁（真判据：LOCK_HELD）
$lockFile = Join-Path $outDir "ll_lock_$stamp.txt"
Remove-Item $lockFile -EA SilentlyContinue
$lockJob = Start-Job -ScriptBlock { param($ops, $db, $secs, $f) & python (Join-Path $ops 'db_write_lock.py') --db $db --seconds $secs *> $f } -ArgumentList $ops, $db, $LockSeconds, $lockFile
$lockHeld = $false
for ($i = 0; $i -lt 12; $i++) {
    Start-Sleep 1
    $t = Get-Content $lockFile -EA SilentlyContinue
    if ($t -match 'LOCK_HELD') { $lockHeld = $true; break }
    if ($t -match 'LOCK_FAILED') { break }
}
if (-not $lockHeld) {
    Write-Host ("FAIL(L1): 写锁未真正注入（db_write_lock.py 输出：{0}）——不产出结论" -f ((Get-Content $lockFile -EA SilentlyContinue) -join ' '))
    Wait-Job $lockJob -Timeout 5 | Out-Null; Remove-Job $lockJob -Force -EA SilentlyContinue
    Stop-Process -Id $srv.Id -Force -EA SilentlyContinue
    exit 3
}
$locked = @(); for ($i = 1; $i -le $Samples; $i++) { $locked += (Sample ("$AccountPrefix$i") 'lock' $i) }
$lockStat = Stat $locked
Wait-Job $lockJob -Timeout ($LockSeconds + 15) | Out-Null; Remove-Job $lockJob -Force -EA SilentlyContinue

$l2 = ($lockStat.n -ge [Math]::Max(10, $Samples - 2)) -and ($lockStat.p95 -ge 0) -and ($lockStat.p95 -lt $P95ThresholdSec)
$l3 = ($ctrlStat.n -ge [Math]::Max(10, $Samples - 2)) -and ($ctrlStat.p95 -ge 0) -and ($ctrlStat.p95 -lt $P95ThresholdSec)
$l4 = ($lockStat.p50 -ge 0) -and ($lockStat.p50 -lt $P50LockedMaxSec)

$timing = @(Select-String -Path $log -Pattern 'LOGIN_TIMING' -EA SilentlyContinue | ForEach-Object { $_.Line -replace '^.*?(INFO|WARN|DEBUG) ', '$1 ' })
$warnTiming = @($timing | Where-Object { $_ -match '^WARN' })
$report = [ordered]@{
    ok = ($l2 -and $l3 -and $l4)
    threshold_p95_sec = $P95ThresholdSec
    samples_per_stage = $Samples
    lock_seconds = $LockSeconds
    lock_acquired = $lockHeld
    lock_output = ((Get-Content $lockFile -EA SilentlyContinue) -join ' | ')
    control = $ctrlStat
    locked = $lockStat
    L2_locked_p95_ok = $l2
    L3_control_p95_ok = $l3
    L4_locked_p50_ok = $l4
    login_timing_lines = $timing.Count
    login_timing_warn_ge_500ms = $warnTiming.Count
    timing_warn_excerpt = @($warnTiming | Select-Object -First 4)
}
Stop-Process -Id $srv.Id -Force -EA SilentlyContinue
$json = $report | ConvertTo-Json -Depth 6
if ($OutFile) { New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutFile) | Out-Null; $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if ($report.ok) { exit 0 } else { exit 4 }
