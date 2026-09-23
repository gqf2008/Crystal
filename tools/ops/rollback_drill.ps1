# rollback_drill.ps1 — 回滚演练：新二进制 → 起服+冒烟+数据快照 → 换回旧二进制 → 再验一次
#
# 判据（任一不过即 exit 5）：
#   ① 新版本起服 + 登录冒烟通过
#   ② 换回上一版二进制后**仍**起服 + 登录冒烟通过（回滚可用）
#   ③ 角色数据快照逐字段不变（gold / level / map_index / x / y）——回滚不能把玩家档弄丢或弄坏
#   ④ 报告写 JSON
# 说明：快照取自部署目录里的 Data/crystal.db（**副本**，不动开发库），只读打开。
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [string]$PrevBinary = '',
    [string[]]$Characters = @('bevychar', 'bevy2char'),
    [int]$Port = 7100,
    [string]$Account = 'test',
    [string]$Password = '123456',
    [int]$ReadyTimeoutSec = 90,
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$dbPath = Join-Path $DeployDir 'Data/crystal.db'
$cur = Join-Path $DeployDir 'mir2_server.exe'
if (-not (Test-Path $cur)) { Write-Host "FAIL: 部署目录没有二进制：$cur"; exit 2 }
if (-not $PrevBinary -or -not (Test-Path $PrevBinary)) {
    Write-Host "FAIL: 需要 -PrevBinary 指向上一个版本二进制（回滚对象）"; exit 2
}
$prevCopy = Join-Path $DeployDir 'mir2_server.prev.exe'
Copy-Item $PrevBinary $prevCopy -Force

function Snapshot {
    if (-not (Test-Path $dbPath)) { return $null }
    # 用 ops 自带的只读快照脚本（不依赖 tools/acceptance；那份在旧检出里可能不存在，
    # 会让快照读成空串、判据退化成"空 == 空"的假 PASS——本轮实测踩过）
    $json = & python (Join-Path $ops 'db_snapshot.py') $dbPath ($Characters -join ',') 2>&1 | Select-Object -Last 1
    try { return ($json | ConvertFrom-Json) } catch { return $null }
}
function Start-And-Smoke([string]$exe, [string]$tag) {
    $log = Join-Path $DeployDir "server.$tag.log"
    $proc = Start-Process -FilePath $exe -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $DeployDir "server.$tag.err.log") -PassThru
    $ready = $false
    for ($i = 0; $i -lt $ReadyTimeoutSec; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
    }
    $smoke = $null
    if ($ready) {
        $out = & python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --login-only `
            --accounts $Account --sessions 1 --password $Password 2>&1 | Select-Object -Last 1
        try { $smoke = $out | ConvertFrom-Json } catch { $smoke = $null }
    }
    if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
    Start-Sleep 3
    return [pscustomobject]@{ ready = $ready; smoke = $smoke; log = $log }
}

# ① 新版
$new = Start-And-Smoke $cur 'new'
$snapNew = Snapshot
# ② 回滚
Copy-Item $prevCopy $cur -Force
$old = Start-And-Smoke $cur 'prev'
$snapOld = Snapshot

$newOk = $new.ready -and ($new.smoke -and $new.smoke.summary.failed -eq 0)
$oldOk = $old.ready -and ($old.smoke -and $old.smoke.summary.failed -eq 0)
$snapPresent = $snapNew -and $snapOld -and ($Characters | ForEach-Object { $null -ne $snapNew.$_ }) -notcontains $false
$dataOk = $true
$diff = @()
foreach ($c in $Characters) {
    $a = $snapNew.$c | ConvertTo-Json -Compress
    $b = $snapOld.$c | ConvertTo-Json -Compress
    if ($null -eq $snapNew.$c) { $dataOk = $false; $diff += "$c : 新版本快照为空（读库失败或角色不存在）" ; continue }
    if ($a -ne $b) { $dataOk = $false; $diff += "$c : $a → $b" }
}
$ok = $newOk -and $oldOk -and $dataOk -and $snapPresent
$report = [ordered]@{
    ok = [bool]$ok
    deploy_dir = (Resolve-Path $DeployDir).Path
    new = [ordered]@{ ready = $new.ready; login_ok = ($new.smoke.summary.failed -eq 0); exe = (Get-FileHash $prevCopy -Algorithm SHA256).Hash }
    prev = [ordered]@{ ready = $old.ready; login_ok = ($old.smoke.summary.failed -eq 0) }
    data_snapshot = [ordered]@{
        characters = $Characters; present = [bool]$snapPresent; unchanged = [bool]$dataOk
        diff = $diff; before = $snapNew; after = $snapOld
    }
    criteria = '① 新版起服+登录 ② 回滚后起服+登录 ③ 角色数据逐字段不变'
}
$json = $report | ConvertTo-Json -Depth 6
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if (-not $ok) { exit 5 } else { exit 0 }
