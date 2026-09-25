# capacity_ramp.ps1 — 容量标定：按会话数阶梯加压，记录"从哪里开始劣化"
#
# 判据（每个阶梯独立重启服务端，避免上一步残留影响曲线）：
#   - 会话成功率、登录 p95
#   - 服务端窗口内 `gate mailbox full`（背压丢包）与 `kicking slow reader`（慢读者被踢）次数
#   - tick 心跳的 interval_ms / lag_pct（INFO 级心跳，见 ServerRust world/tick.rs）
# 输出：JSON 阶梯表 + 一句"拐点"结论（第一个出现丢包/踢线的会话数）。
#
# 多 agent 并行（本机常态；7000 端口上有常驻开发服）：本脚本**只按 PID 管控自己启动的实例**
# ——既不按进程名 `Stop-Process`，也不按进程名取 RSS。判据里杀错/量错进程会让容量曲线变成
# "来源不明"的数字，并且把别的 agent 正在跑的验收打成假红
# （见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）。口径与 memory_ramp.ps1 /
# memory_cycle.ps1 一致（见 tools/ops/README.md「只停自己启动的 PID」那条约定）。
# 静态门禁：tools/ops/check_process_scope.ps1（按进程名杀共享进程即红）。
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
$proc = $null   # 本脚本自己启动的那个服务端实例；停止与取数都只认这个 PID

function Get-OwnRssMb {
    # 只读「自己启动的进程」的 RSS。按进程名 `Get-Process -Name mir2_server | Select -First 1`
    # 会量到别的 agent 的常驻开发服，rss_idle/rss_loaded 就不可信了（本机 7000 常态在线）。
    param([System.Diagnostics.Process]$P)
    if (-not $P) { return $null }
    try { $P.Refresh() } catch { return $null }
    if ($P.HasExited) { return $null }
    return [Math]::Round($P.WorkingSet64 / 1MB, 1)
}

# 端口前置校验：多 agent 并行时若 $Port 已被别人占着，本演练会"起服成功"但连的是别人的服
# （2026-09-25 存储降级演练就因端口不一致静默挂死过）。宁可前置失败（exit 2）也不产出假绿。
if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) {
    Write-Host ("FAIL(前置)：{0} 端口已被占用——本机多 agent 并行，请换 -Port，或先确认那是自己的实例" -f $Port)
    exit 2
}

foreach ($n in $Steps) {
    # 只收自己上一阶梯启动的实例。$proc.HasExited 为真时不 Stop-Process：PID 可能已被系统复用，
    # 按 PID 杀会误杀无关进程（换成新 PID 后那个对象认的就是别人了）。
    if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
    $proc = $null
    Start-Sleep 3
    Remove-Item $log -ErrorAction SilentlyContinue
    $env:RUST_LOG = 'crystal_server=info'
    $proc = Start-Process -FilePath $exe -WorkingDirectory $DeployDir `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $DeployDir 'ramp.err.log') -PassThru
    $ready = $false
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
    }
    if (-not $ready) {
        $exitNote = if ($proc -and $proc.HasExited) { "exit=$($proc.ExitCode)" } else { '' }
        $rows += [pscustomobject]@{
            sessions = $n; ready = $false
            pid = if ($proc) { $proc.Id } else { $null }
            note = ("未等到 'Gate listening' " + $exitNote)
        }
        continue
    }
    $rssIdle = Get-OwnRssMb $proc

    $acc = (1..$n | ForEach-Object { "$AccountPrefix$_" }) -join ','
    $json = Join-Path $ops "out/ramp_$n.json"
    # 后台跑压测，主线程在保持期采样 RSS（会话全在线时的真实占用）
    $job = Start-Job -ScriptBlock {
        param($ops, $acc, $n, $Port, $HoldSec, $json)
        # job 里的 runspace 不继承父作用域的函数 ⇒ 这里自己 dot-source 同一个 helper，
        # 让"有界等待"在 job 内部也成立（父线程的 Wait-Job -Timeout 是第二层）。
        . (Join-Path $ops '_run_bot.ps1')
        $r = Invoke-BotJson -OpsDir $ops -BotArgs @('--host', '127.0.0.1', '--port', "$Port", '--accounts', $acc,
            '--sessions', "$n", '--hold', "$HoldSec", '--password', '123456') `
            -TimeoutSec ($HoldSec + 60) -Tag 'capacity_ramp'
        if ($r.json) { $r.json | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $json }
    } -ArgumentList $ops, $acc, $n, $Port, $HoldSec, $json
    Start-Sleep ([Math]::Max(8, $HoldSec / 2))
    $rssLoaded = Get-OwnRssMb $proc
    # 2026-09-25：Wait-Job 原先**没有超时**——bot 卡住就整轮永久等下去（同类"静默挂死"）。
    # 现在有界等待：超时则终止该轮采样并明确告警（采样数不足后面自会判失败）。
    if (-not (Wait-Job $job -Timeout ($HoldSec + 90))) {
        Write-Host ("WARN: 压测 bot 超过 {0}s 未结束（port={1}）——终止该轮采样" -f ($HoldSec + 90), $Port)
        Stop-Job $job -ErrorAction SilentlyContinue
    }
    Remove-Job $job -Force -ErrorAction SilentlyContinue
    Start-Sleep 2
    $bot = $null
    try { $bot = (Get-Content $json -Raw | ConvertFrom-Json) } catch {}
    $text = Get-Content $log -ErrorAction SilentlyContinue
    $mailbox = @($text | Where-Object { $_ -match 'gate mailbox full' }).Count
    $kicks = @($text | Where-Object { $_ -match 'kicking slow reader' }).Count
    # 背压信号分离（2026-09-23）：`gate mailbox full` 现在是**单目标 SendToClient** 的丢弃；
    # 广播路径挤不动时走世界侧发件箱（deferred），发件箱满才 outbox_full 丢。三者必须分开看，
    # 否则"广播改排队"会被误读成"丢包更多/更少"。
    $deferred = @($text | Where-Object { $_ -match 'broadcast deferred to outbox' }).Count
    $outboxFull = @($text | Where-Object { $_ -match 'broadcast outbox full' }).Count
    $hb = @($text | Where-Object { $_ -match 'heartbeat: tick=' })
    $lag = @()
    foreach ($h in $hb) { if ($h -match 'lag_pct=(-?[\d.]+)') { $lag += [double]$Matches[1] } }
    $rows += [pscustomobject]@{
        sessions = $n
        ready = $true
        # pid 一并入表：容量曲线的每个 RSS 数字都能追到"是哪一次启动的哪个进程"量的
        pid = if ($proc) { $proc.Id } else { $null }
        ok = if ($bot) { $bot.summary.ok } else { 0 }
        failed = if ($bot) { $bot.summary.failed } else { $n }
        login_p95_sec = if ($bot) { $bot.summary.login_p95_sec } else { $null }
        rss_idle_mb = $rssIdle
        rss_loaded_mb = $rssLoaded
        rss_per_session_mb = if ($null -ne $rssIdle -and $null -ne $rssLoaded) {
            [Math]::Round(($rssLoaded - $rssIdle) / [Math]::Max(1, $n), 2)
        } else { $null }
        mailbox_full = $mailbox
        slow_reader_kicks = $kicks
        broadcast_deferred = $deferred
        broadcast_outbox_full = $outboxFull
        heartbeats = $hb.Count
        max_abs_lag_pct = if ($lag.Count) { ($lag | ForEach-Object { [Math]::Abs($_) } | Measure-Object -Maximum).Maximum } else { $null }
    }
}
if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }

$knee = ($rows | Where-Object { $_.mailbox_full -gt 0 -or $_.slow_reader_kicks -gt 0 -or $_.failed -gt 0 } |
    Sort-Object sessions | Select-Object -First 1)
# 诚实标注：tick 心跳约每 30s 一条（CAPACITY.md §5.1），HoldSec 短于一个 tick 周期时本阶梯
# **根本没采到** tick 样本——判据里列了它，不等于拿到了它；不能让"没测到"被读成"没劣化"。
$lagMeasured = (@($rows | Where-Object { $null -ne $_.max_abs_lag_pct }).Count -gt 0)
$report = [ordered]@{
    ok = ($null -eq $knee)
    step_table = $rows
    knee_sessions = if ($knee) { $knee.sessions } else { $null }
    knee_note = if ($knee) { "从 $($knee.sessions) 会话起出现劣化（丢包/踢线/失败）" } else { "本阶梯内未出现劣化" }
    criteria = '成功率 / 登录 p95 / 背压丢包 / 慢读者踢线 / tick 滞后（见 lag_measured）'
    lag_measured = $lagMeasured
    lag_note = if ($lagMeasured) {
        '本阶梯采到了 tick 心跳样本'
    } else {
        "本阶梯未采到 tick 心跳（HoldSec=$HoldSec 短于一个 tick 周期，约 30s）——tick 滞后本轮**未测**；要测请 -HoldSec 60 以上，或用 tick_lag_probe.ps1"
    }
}
$jsonOut = $report | ConvertTo-Json -Depth 6
if ($OutFile) { $jsonOut | Set-Content -Encoding utf8 $OutFile }
Write-Host $jsonOut
exit 0
