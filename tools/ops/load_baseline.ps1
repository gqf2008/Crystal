# load_baseline.ps1 — 压测基线：并发登录（gate 路径）+ 世界会话（进图并保持）
# 判据见 tools/ops/README.md §3；阈值可用参数覆盖。输出 JSON。
param(
    [Parameter(Mandatory = $true)][string]$ServerDir,
    [string]$Host_ = '127.0.0.1',
    [int]$Port = 7000,
    [int]$GateSessions = 30,
    [int]$WorldSessions = 4,
    [int]$HoldSec = 20,
    [string]$GateAccounts = 'test,bevy2',
    [string]$LogFile = '',
    [string]$OutFile = '',
    [double]$MaxLoginP95Sec = 1.0,
    # 日志窗口回看行数：DEBUG 级别下服务端 ~1k 行/秒（怪物 AI 逐只打点），
    # 默认 40k 行只覆盖几秒，会漏掉 10s 一次的 "World tick" 行 → tick 指标恒"不可用"。
    [int]$WindowTailLines = 200000
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = Split-Path -Parent (Split-Path -Parent $ops)
$bot = Join-Path $ops 'bot.py'

function Run-Bot([string[]]$extra) {
    $out = & python $bot --host $Host_ --port $Port @extra 2>&1 | Select-Object -Last 1
    try { return ($out | ConvertFrom-Json) } catch { return $null }
}

# tick 与 RSS 的窗口前后对比
$procBefore = Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Select-Object -First 1
$rssBefore = if ($procBefore) { [Math]::Round($procBefore.WorkingSet64 / 1MB, 1) } else { $null }
$runStart = (Get-Date).ToUniversalTime()

Write-Host "== 1) gate 路径：$GateSessions 并发登录（复用账号轮转）=="
$acct = ($GateAccounts -split ',' | Where-Object { $_ })
$loginAccounts = @()
for ($i = 0; $i -lt $GateSessions; $i++) { $loginAccounts += $acct[$i % $acct.Count] }
$gate = Run-Bot @('--login-only', '--accounts', ($loginAccounts -join ','), '--sessions', "$GateSessions", '--password', '123456')

Write-Host "== 2) 世界路径：$WorldSessions 会话进图并保持 $HoldSec s =="
$world = Run-Bot @('--accounts', $GateAccounts, '--sessions', "$WorldSessions", '--hold', "$HoldSec", '--password', '123456')

Start-Sleep 3
$procAfter = Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Select-Object -First 1
$rssAfter = if ($procAfter) { [Math]::Round($procAfter.WorkingSet64 / 1MB, 1) } else { $null }

$newLog = ''
if ($LogFile -and (Test-Path $LogFile)) {
    $newLog = Join-Path ([IO.Path]::GetTempPath()) ("ops_window_{0}.log" -f (Get-Date -Format 'HHmmss'))
    # 服务端正在写这个文件（持有句柄、单文件可到上百 MB）：`Get-Content -Raw` 在这种情形会返回垃圾
    # （实测 91MB 文件读出 3 个字符），`[IO.File]::Open` 独占读也不稳。用 `-Tail` 读快照 +
    # **按时间戳**过滤本次运行窗口——比字节偏移语义更准。
    $tail = Get-Content -Tail $WindowTailLines -Path $LogFile -ErrorAction SilentlyContinue
    $windowed = @()
    foreach ($l in $tail) {
        if ($l -match '^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?)Z') {
            # 日志时间戳是 UTC（尾部 Z 被我截掉了）：必须显式按 UTC 解释，
            # 否则 Parse 会当本地时间（+8），与 UTC 的 runStart 比较时整窗被过滤空。
            $ts = [datetime]::SpecifyKind(
                [datetime]::Parse($Matches[1], $null, [System.Globalization.DateTimeStyles]::RoundtripKind),
                [System.DateTimeKind]::Utc)
            if ($ts -ge $runStart) { $windowed += $l }
        } else { $windowed += $l }   # 无时间戳的续行也保留
    }
    if ($windowed.Count -gt 0) {
        $windowed | Set-Content -Encoding utf8 $newLog
    } else {
        $newLog = ''
    }
}
$health = $null
if ($newLog) {
    # 用文件交接而不是抓 stdout：health_report 的 JSON 是多行的，
    # `| Select-Object -Last 1` 只会拿到最后一个 `}`，ConvertFrom-Json 必失败（本轮实测）。
    $healthFile = Join-Path ([IO.Path]::GetTempPath()) 'ops_health_window.json'
    & pwsh (Join-Path $ops 'health_report.ps1') -LogFile $newLog -OutFile $healthFile | Out-Null
    if (Test-Path $healthFile) {
        try { $health = (Get-Content -Raw $healthFile | ConvertFrom-Json) } catch { $health = $null }
    }
}

$gateOk = ($gate -and $gate.summary.failed -eq 0)
$p95Ok = ($gate -and $gate.summary.login_p95_sec -ne $null -and $gate.summary.login_p95_sec -le $MaxLoginP95Sec)
$worldOk = ($world -and $world.summary.failed -eq 0)
$rssGrowth = if ($rssBefore -ne $null -and $rssAfter -ne $null) { [Math]::Round($rssAfter - $rssBefore, 1) } else { $null }
$rssOk = ($rssGrowth -eq $null) -or ($rssGrowth -lt 200)
$verdict = if ($gateOk -and $p95Ok -and $worldOk -and $rssOk) { 'PASS' } else { 'FAIL' }

$report = [ordered]@{
    ok = ($verdict -eq 'PASS')
    verdict = $verdict
    criteria = [ordered]@{
        gate_sessions = $GateSessions; gate_all_ok = $gateOk
        login_p95_sec = if ($gate) { $gate.summary.login_p95_sec } else { $null }; max_login_p95_sec = $MaxLoginP95Sec; login_p95_ok = $p95Ok
        world_sessions = $WorldSessions; world_hold_sec = $HoldSec; world_all_ok = $worldOk
        rss_before_mb = $rssBefore; rss_after_mb = $rssAfter; rss_growth_mb = $rssGrowth; rss_growth_ok = $rssOk
    }
    gate = $gate
    world = $world
    health_window = $health
    note = '单机基线，不是生产容量结论（见 tools/ops/README.md §3）'
}
$json = $report | ConvertTo-Json -Depth 8
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if ($verdict -ne 'PASS') { exit 5 } else { exit 0 }
