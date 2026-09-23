# alert_probe.ps1 — 告警接入最小集：把健康信号折算成"有没有告警"，并可外送
#
# 这层刻意只做三件事（其余交给调度/值班流程）：
#   ① 跑一次 health_report（tick 滞后 / ERROR / 已知坏味道 / 进程指标）；
#   ② 按阈值判 ALERT（不是"看一眼日志"，是可判定的布尔量）；
#   ③ 落 alerts.json + 可选 webhook 外送 + **退出码**（给任务计划程序/CI 用）。
#
# 阈值（可用参数覆盖）：lag_pct > MaxLagPct、真错误数 > MaxErrors、
#                       慢读者被踢、邮箱背压丢包、RSS > MaxRssMb
param(
    [Parameter(Mandatory = $true)][string]$LogFile,
    [string]$ProcessName = 'mir2_server',
    [double]$MaxLagPct = 50.0,
    [int]$MaxErrors = 0,
    [double]$MaxRssMb = 1500.0,
    [string]$Webhook = '',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$tmp = Join-Path ([IO.Path]::GetTempPath()) 'alert_health.json'
& pwsh (Join-Path $ops 'health_report.ps1') -LogFile $LogFile -ProcessName $ProcessName -OutFile $tmp | Out-Null
$health = $null
if (Test-Path $tmp) { try { $health = (Get-Content -Raw $tmp | ConvertFrom-Json) } catch { $health = $null } }

$reasons = @()
if (-not $health) { $reasons += "健康报告不可用（日志读取失败）" }
else {
    if ($health.ticks.available -and ([Math]::Abs([double]$health.ticks.jitter_pct) -gt $MaxLagPct)) {
        $reasons += "tick 抖动 $($health.ticks.jitter_pct)% > $MaxLagPct%"
    }
    if ($health.errors -gt $MaxErrors) { $reasons += "真错误 $($health.errors) 条 > $MaxErrors" }
    if ($health.signals.slow_reader_kicked -gt 0) { $reasons += "慢读者被踢 $($health.signals.slow_reader_kicked) 次" }
    if ($health.signals.gate_mailbox_full -gt 0) { $reasons += "邮箱背压丢包 $($health.signals.gate_mailbox_full) 次" }
    if ($health.process -and ([double]$health.process.rss_mb -gt $MaxRssMb)) {
        $reasons += "RSS $($health.process.rss_mb)MB > ${MaxRssMb}MB"
    }
}
$alert = [ordered]@{
    alert = ($reasons.Count -gt 0)
    reasons = $reasons
    thresholds = [ordered]@{ max_lag_pct = $MaxLagPct; max_errors = $MaxErrors; max_rss_mb = $MaxRssMb }
    health = $health
    ts = (Get-Date).ToUniversalTime().ToString('o')
}
$json = $alert | ConvertTo-Json -Depth 8
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if ($alert.alert -and $Webhook) {
    try {
        Invoke-RestMethod -Method Post -Uri $Webhook -ContentType 'application/json' -Body $json -TimeoutSec 10 | Out-Null
        Write-Host "(已外送 webhook)"
    } catch { Write-Host "(webhook 外送失败：$($_.Exception.Message))" }
}
if ($alert.alert) { exit 3 } else { exit 0 }
