# mem_attribution.ps1 — 「世界路径 ~3.5MB/会话」归属测量（一条命令）
#
# 背景（`tools/ops/CAPACITY.md` §4 / §4.6，master `79bc244f` 的 release 复测）：
#   - 只登录**不进图**：0.13 MB/会话 ⇒ 与 gate 会话本身无关；
#   - 进图后：~3.5 MB/会话留在**世界路径**；
#   - 外推 `RSS ≈ 28MB + 3.5MB × 会话数` ⇒ 200 会话约 0.73GB；
#   - 「登出后不归还」已定性为**一次性高水位**（不是每轮线性泄漏）。
# 本脚本要回答的是：那 3.5MB 到底属于哪一档——① 只登录 ② 进了图但不动 ③ 进图且有广播（走动）。
#
# 为什么每档都**重启服务端**：登出后不归还（高水位）会让下一档从"已涨过的底"开始，档间差就不可信；
# 重启后各档都从 idle 起算，per-session 增量才可比。
#
# 判据（全取**自己那一个 PID** 的 WorkingSet，绝不按进程名取——本机 7000 常驻别人的开发服）：
#   per_session(tier) = (rss_loaded - rss_idle) / sessions
#   Δ(②-①) = 入场成本；Δ(③-②) = 广播/走动成本。
# 退出码：0 = 三档都测到；2 = 前置失败（端口被占/exe 缺失/起服未就绪）；3 = 采样不完整（不产出结论）。
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [int]$Port = 7101,
    [int]$Sessions = 10,
    [int]$HoldSec = 20,
    [string]$AccountPrefix = 'opsload',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $DeployDir 'mir2_server.exe'
$log = Join-Path $DeployDir 'memattr.log'

function Get-OwnRssMb([System.Diagnostics.Process]$P) {
    if (-not $P) { return $null }
    try { $P.Refresh() } catch { return $null }
    if ($P.HasExited) { return $null }
    return [Math]::Round($P.WorkingSet64 / 1MB, 1)
}

# ---- 前置（宁可 exit 2，也不产出假数字）--------------------------------------
if (-not (Test-Path -LiteralPath $exe)) { Write-Host "FAIL(2): 找不到 $exe"; exit 2 }
if (Get-NetTCPConnection -LocalPort $Port -State Listen -EA SilentlyContinue) {
    # 端口被别人占着时，"起服成功"其实连的是别人的服，RSS 就量的不是自己的进程
    Write-Host "FAIL(2): 端口 $Port 已被占用（换个 -Port；本机 7000 是常驻开发服）"; exit 2
}

$acc = (1..$Sessions | ForEach-Object { "$AccountPrefix$_" }) -join ','

function Start-OwnServer {
    if (Test-Path $log) { [IO.File]::Delete($log) }
    $env:RUST_LOG = 'crystal_server=info'
    $p = Start-Process -FilePath $exe -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $DeployDir 'memattr.err.log') -PassThru
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        $fs = $null; $sr = $null
        try {
            if (Test-Path $log) {
                $fs = [IO.File]::Open($log, 'Open', 'Read', [IO.FileShare]::ReadWrite)
                $sr = New-Object IO.StreamReader($fs)
                if ($sr.ReadToEnd() -match 'Gate listening') { return $p }
            }
        } catch { } finally { if ($sr) { $sr.Dispose() }; if ($fs) { $fs.Dispose() } }
    }
    return $null
}
function Stop-OwnServer([System.Diagnostics.Process]$P) {
    if ($P -and -not $P.HasExited) { Stop-Process -Id $P.Id -Force -EA SilentlyContinue }
    Start-Sleep 2
}

$tiers = @(
    [pscustomobject]@{ name = 'login_only'; args = @('--login-only') },
    [pscustomobject]@{ name = 'in_map_idle'; args = @() },
    [pscustomobject]@{ name = 'in_map_walk'; args = @('--activity', 'walk') }
)
$rows = @()
foreach ($t in $tiers) {
    $proc = Start-OwnServer
    if (-not $proc) {
        Write-Host ("FAIL(3): {0} 档起服未就绪（未等到 'Gate listening'）" -f $t.name)
        $rows += [pscustomobject]@{ tier = $t.name; ok = $false; note = 'server not ready' }
        continue
    }
    $rssIdle = Get-OwnRssMb $proc
    $json = Join-Path $ops ("out/memattr_{0}.json" -f $t.name)
    # 进图档的前置：账号必须先**有角色**，否则 bot 的 ok:true 只表示"流程没抛异常"，
    # 实际可能停在登录后保持 —— 那会把"没进图"的档产出 per-session 数字（假绿）。
    # 只登录档不需要角色（`--login-only` 不建角也不进图）。
    if ($t.name -ne 'login_only') {
        # 不引入 SQL 依赖（各处 DB 工具/VenV 不一）：这里只提示前置，**真判据在采样后**看 bot JSON 的进图证据。
        Write-Host ("  [{0}] 进图档前置：请确认账号已有角色（先跑一次 --self-provision 播种）" -f $t.name) -ForegroundColor DarkGray
    }
    $job = Start-Job -ScriptBlock {
        param($ops, $acc, $Port, $HoldSec, $json, $extra)
        . (Join-Path $ops '_run_bot.ps1')
        $botArgs = @('--host', '127.0.0.1', '--port', "$Port", '--accounts', $acc,
            '--hold', "$HoldSec", '--password', '123456') + $extra
        $r = Invoke-BotJson -OpsDir $ops -BotArgs $botArgs -TimeoutSec ($HoldSec + 90) -Tag ('memattr_' + $extra.Count)
        if ($r.json) { $r.json | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $json }
    } -ArgumentList $ops, $acc, $Port, $HoldSec, $json, $t.args
    Start-Sleep ([Math]::Max(8, $HoldSec / 2))
    $rssLoaded = Get-OwnRssMb $proc
    $done = Wait-Job $job -Timeout ($HoldSec + 120)
    $jsonExists = Test-Path $json
    # **进图证据**：进图档要求 bot JSON 里出现角色/进图相关字段（`new_character_result` 非 null，
    # 或 `frames`/`opcodes` 明显多于登录往返），否则记 ok=$false 并注明"没有进图证据"，
    # 避免给"登录后保持"产出 per-session 数字。
    $entryEvidence = $true
    if ($jsonExists -and $t.name -ne 'login_only') {
        try {
            $bj = Get-Content $json -Raw | ConvertFrom-Json
            $s0 = @($bj.sessions)[0]
            $frames = [int]$s0.frames
            $charRes = $s0.new_character_result
            $entryEvidence = ($null -ne $charRes) -or ($frames -ge 8)
            if (-not $entryEvidence) {
                Write-Host ("WARN: {0} 档没有进图证据（char_result={1} frames={2}）——本档读数不可用" -f $t.name, $charRes, $frames) -ForegroundColor Yellow
            }
        } catch { $entryEvidence = $false }
    }
    if (-not $done) { Write-Host ("WARN: {0} 档 bot 超时" -f $t.name) }
    Receive-Job $job -EA SilentlyContinue | Out-Null
    Remove-Job $job -Force -EA SilentlyContinue
    Stop-OwnServer $proc
    $delta = if ($null -ne $rssIdle -and $null -ne $rssLoaded -and $jsonExists) {
        [Math]::Round(($rssLoaded - $rssIdle) / $Sessions, 3)
    } else { $null }
    $rows += [pscustomobject]@{
        tier          = $t.name
        ok            = (($null -ne $delta) -and $entryEvidence)
        entry_evidence = $entryEvidence
        sessions      = $Sessions
        rss_idle_mb   = $rssIdle
        rss_loaded_mb = $rssLoaded
        per_session_mb = $delta
        bot_json      = $(if ($jsonExists) { $json } else { '' })
    }
    Write-Host ("  {0,-12} idle={1} loaded={2} per-session={3} MB" -f $t.name, $rssIdle, $rssLoaded, $delta)
}

$byName = @{}
foreach ($r in $rows) { $byName[$r.tier] = $r }
$report = [pscustomobject]@{
    ts_utc    = (Get-Date).ToUniversalTime().ToString('o')
    port      = $Port
    sessions  = $Sessions
    hold_sec  = $HoldSec
    deploy    = $DeployDir
    tiers     = $rows
    # 差值：入场成本 = ②-①；广播成本 = ③-②（都为 null 时不产出结论）
    delta_enter_map = if ($byName['in_map_idle'].per_session_mb -ne $null -and $byName['login_only'].per_session_mb -ne $null) {
        [Math]::Round($byName['in_map_idle'].per_session_mb - $byName['login_only'].per_session_mb, 3)
    } else { $null }
    delta_broadcast = if ($byName['in_map_walk'].per_session_mb -ne $null -and $byName['in_map_idle'].per_session_mb -ne $null) {
        [Math]::Round($byName['in_map_walk'].per_session_mb - $byName['in_map_idle'].per_session_mb, 3)
    } else { $null }
}
$jsonOut = $report | ConvertTo-Json -Depth 6
if ($OutFile) { [IO.File]::WriteAllText($OutFile, $jsonOut, (New-Object System.Text.UTF8Encoding($false))) }
Write-Host $jsonOut

$bad = @($rows | Where-Object { -not $_.ok })
if ($bad.Count -gt 0) {
    Write-Host ("VERDICT mem_attribution=INCOMPLETE（{0} 档未测到：{1}）" -f $bad.Count, (($bad | ForEach-Object { $_.tier }) -join ',')) -ForegroundColor Yellow
    exit 3
}
Write-Host ("VERDICT mem_attribution=OK（入场成本={0} MB/会话；广播成本={1} MB/会话）" -f `
    $report.delta_enter_map, $report.delta_broadcast) -ForegroundColor Green
exit 0
