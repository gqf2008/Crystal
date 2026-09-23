# capacity_ramp.ps1 — 容量标定：按会话数阶梯加压，记录"从哪里开始劣化"
#
# 判据（每个阶梯独立重启服务端，避免上一步残留影响曲线）：
#   - 会话成功率、登录 p95
#   - 服务端窗口内 `gate mailbox full`（背压丢包）与 `kicking slow reader`（慢读者被踢）次数
#   - tick 心跳的 interval_ms / lag_pct（INFO 级心跳，见 ServerRust world/tick.rs）
# 输出：JSON 阶梯表 + 一句"拐点"结论（第一个出现丢包/踢线的会话数）。
param(
    [Parameter(Mandatory = $true)][string]$DeployDir,
    [int]$Port = 7100,
    # 注意：`pwsh -File` 传数组参数会被当成单个字符串（实测 -Steps 10,20,30 → "10203050"），
    # 故这里收 CSV 字符串自己切。
    [string]$StepsCsv = '10,20,30,50',
    [string]$AccountPrefix = 'opsload',
    [int]$HoldSec = 20,
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$Steps = @($StepsCsv.Split(',') | ForEach-Object { [int]$_.Trim() } | Where-Object { $_ -gt 0 })
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $DeployDir 'mir2_server.exe'
$log = Join-Path $DeployDir 'ramp.log'
$rows = @()

foreach ($n in $Steps) {
    Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep 3
    Remove-Item $log -ErrorAction SilentlyContinue
    $env:RUST_LOG = 'crystal_server=info'
    Start-Process -FilePath $exe -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $DeployDir 'ramp.err.log') | Out-Null
    $ready = $false
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
    }
    if (-not $ready) { $rows += [pscustomobject]@{ sessions = $n; ready = $false }; continue }
    $rssIdle = [Math]::Round((Get-Process -Name mir2_server | Select-Object -First 1).WorkingSet64 / 1MB, 1)

    $acc = (1..$n | ForEach-Object { "$AccountPrefix$_" }) -join ','
    $json = Join-Path $ops "out/ramp_$n.json"
    # 后台跑压测，主线程在保持期采样 RSS（会话全在线时的真实占用）
    $job = Start-Job -ScriptBlock {
        param($ops, $acc, $n, $Port, $HoldSec, $json)
        & python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --accounts $acc `
            --sessions $n --hold $HoldSec --password 123456 > $json 2>&1
    } -ArgumentList $ops, $acc, $n, $Port, $HoldSec, $json
    Start-Sleep ([Math]::Max(8, $HoldSec / 2))
    $rssLoaded = [Math]::Round((Get-Process -Name mir2_server | Select-Object -First 1).WorkingSet64 / 1MB, 1)
    Wait-Job $job | Out-Null
    Remove-Job $job -Force
    Start-Sleep 2
    $bot = $null
    try { $bot = (Get-Content $json -Raw | ConvertFrom-Json) } catch {}
    $text = Get-Content $log -ErrorAction SilentlyContinue
    $mailbox = @($text | Where-Object { $_ -match 'gate mailbox full' }).Count
    $kicks = @($text | Where-Object { $_ -match 'kicking slow reader' }).Count
    $hb = @($text | Where-Object { $_ -match 'heartbeat: tick=' })
    $lag = @()
    foreach ($h in $hb) { if ($h -match 'lag_pct=(-?[\d.]+)') { $lag += [double]$Matches[1] } }
    $rows += [pscustomobject]@{
        sessions = $n
        ready = $true
        ok = if ($bot) { $bot.summary.ok } else { 0 }
        failed = if ($bot) { $bot.summary.failed } else { $n }
        login_p95_sec = if ($bot) { $bot.summary.login_p95_sec } else { $null }
        rss_idle_mb = $rssIdle
        rss_loaded_mb = $rssLoaded
        rss_per_session_mb = [Math]::Round(($rssLoaded - $rssIdle) / [Math]::Max(1, $n), 2)
        mailbox_full = $mailbox
        slow_reader_kicks = $kicks
        heartbeats = $hb.Count
        max_abs_lag_pct = if ($lag.Count) { ($lag | ForEach-Object { [Math]::Abs($_) } | Measure-Object -Maximum).Maximum } else { $null }
    }
}
Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force

$knee = ($rows | Where-Object { $_.mailbox_full -gt 0 -or $_.slow_reader_kicks -gt 0 -or $_.failed -gt 0 } |
    Sort-Object sessions | Select-Object -First 1)
$report = [ordered]@{
    ok = ($null -eq $knee)
    step_table = $rows
    knee_sessions = if ($knee) { $knee.sessions } else { $null }
    knee_note = if ($knee) { "从 $($knee.sessions) 会话起出现劣化（丢包/踢线/失败）" } else { "本阶梯内未出现劣化" }
    criteria = '成功率 / 登录 p95 / 背压丢包 / 慢读者踢线 / tick 滞后'
}
$jsonOut = $report | ConvertTo-Json -Depth 6
if ($OutFile) { $jsonOut | Set-Content -Encoding utf8 $OutFile }
Write-Host $jsonOut
exit 0
