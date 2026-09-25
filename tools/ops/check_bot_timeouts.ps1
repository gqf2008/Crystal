# check_bot_timeouts.ps1 —— 静态门禁：ops 演练里**不许**再出现"同步无超时的 bot 调用"
#
# 背景（2026-09-25 实测）：`& python bot.py … | Select-Object -Last 1` 这种同步写法没有超时，
# 只要被测服务端没绑到 -Port（部署配置端口不一致）就会一直等 ⇒ 整个演练**静默挂死**
# （storage_degrade_drill 实测挂了 6 分钟以上、无任何输出）。统一改走 `_run_bot.ps1` 的
# `Invoke-BotJson`（有界等待 + 超时即杀 + 结果可判定）。
#
# 本门禁做法：扫 tools/ops/*.ps1，凡出现 `& python … bot.py`（同步直调）就报出来。
# 历史上遗留的脚本暂时列在 -Allowlist 里（**那就是待迁移清单**）；新增脚本再这么写就会红。
#
# 用法：pwsh tools/ops/check_bot_timeouts.ps1            # 退出码 0（无新增）/ 1（有新增未迁移）/ 2 前置失败
#       pwsh tools/ops/check_bot_timeouts.ps1 -Strict    # 连 allowlist 里的一起报红（迁移完成后用）
param(
    [string]$OpsDir = '',
    [switch]$Strict,
    # 待迁移清单（2026-09-25 审计结果）：这些脚本仍在同步直调 bot.py，逐个迁移到 _run_bot.ps1
    [string[]]$Allowlist = @(
        'fault_injection.ps1',
        'leak_plateau.ps1',
        'fresh_mirdb_deploy.ps1',
        'migration_drill.ps1',
        'capacity_ramp.ps1',
        'memory_cycle.ps1',
        'memory_ramp.ps1',
        'login_latency_probe.ps1'
    ),
    [string[]]$Scan = @()
)
$ErrorActionPreference = 'Continue'
if (-not $OpsDir) { $OpsDir = $PSScriptRoot }
if (-not (Test-Path -LiteralPath $OpsDir)) { Write-Host "前置失败：ops 目录不存在 $OpsDir"; exit 2 }

if ($Scan.Count -eq 0) {
    $Scan = @(Get-ChildItem -LiteralPath $OpsDir -Filter *.ps1 -File |
        Where-Object { $_.Name -ne (Split-Path -Leaf $PSCommandPath) -and $_.Name -ne '_run_bot.ps1' } |
        ForEach-Object { $_.FullName })
}

$offenders = @()
foreach ($f in $Scan) {
    $text = Get-Content -LiteralPath $f -Raw -ErrorAction SilentlyContinue
    if ($null -eq $text) { continue }
    # 同步直调：`& python … bot.py`（helper 里的调用是 Start-Process，不会命中）
    if ($text -match '&\s+python[^\r\n]*bot\.py') {
        $name = Split-Path -Leaf $f
        $offenders += [pscustomobject]@{
            file      = $name
            allowlist = ($Allowlist -contains $name)
        }
    }
}

$new = @($offenders | Where-Object { -not $_.allowlist })
$known = @($offenders | Where-Object { $_.allowlist })
Write-Host ("扫描 {0} 个 ops 脚本：同步直调 bot.py 的 {1} 个（其中待迁移 allowlist {2} 个、新增 {3} 个）" -f `
        $Scan.Count, $offenders.Count, $known.Count, $new.Count)
foreach ($o in $known) { Write-Host ("  [待迁移] {0}" -f $o.file) -ForegroundColor DarkYellow }
foreach ($o in $new) { Write-Host ("  [违规]   {0} —— 改用 _run_bot.ps1 的 Invoke-BotJson（带 -TimeoutSec）" -f $o.file) -ForegroundColor Red }

if ($new.Count -gt 0) { exit 1 }
if ($Strict -and $known.Count -gt 0) {
    Write-Host '  -Strict：待迁移清单也必须清空（迁移完成后去掉 allowlist 里的条目）' -ForegroundColor Red
    exit 1
}
Write-Host '结果：无新增的无超时 bot 调用 ✅'
exit 0
