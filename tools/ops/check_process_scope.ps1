#Requires -Version 5.1
# check_process_scope.ps1 —— 静态门禁：ops / 验收脚本**不许按进程名杀共享进程**
#
# 背景（本机常态：多 agent 并行 + 7000 端口常驻开发服）：
#   演练/夹具用 `Get-Process -Name mir2_server | Stop-Process -Force` 清场，会连带杀掉
#   **别人正在跑**的服务端与验收，对方拿到的是假红（登录 result=4 / 连不上服务端），
#   重试再多也修不了 —— 与"靠重试撞干净窗口"是同一个坑
#   （见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）。
# 正确口径：只按**自己启动的 PID** 管控 —— `$proc = Start-Process … -PassThru`、
#   `Stop-Process -Id $proc.Id`（参考 tools/ops/memory_ramp.ps1 / memory_cycle.ps1；
#   capacity_ramp.ps1 已按本门禁改齐）。
# 唯一命名不算违规：`Stop-Process -Name <自己的唯一名>` 正是 LESSON 推荐的做法，本门禁只认
#   **共享名**（mir2_server / client_bevy）以及 "Get-Process … | Stop-Process" 这种按名管道。
#   只做存在性探测（`if (-not (Get-Process -Name mir2_server …)) { exit 9 }`）的脚本不受影响。
#
# 用法：pwsh tools/ops/check_process_scope.ps1                 # 0 无新增 / 1 有新增未迁移 / 2 前置失败或门禁自检失败
#       pwsh tools/ops/check_process_scope.ps1 -Strict         # allowlist 里的一起报红（全部迁移完后用）
#       pwsh tools/ops/check_process_scope.ps1 -SkipSelfTest   # 跳过沙箱正/负对照
param(
    [string[]]$ScanDir = @(),
    # 待迁移清单：这几处的"杀"是演练目的本身或仓库级 harness 的清场，本轮只明确记名、不静默放过。
    [hashtable]$Allowlist = @{
        'fault_injection.ps1' = '杀服务端就是本演练的目的（故障注入）；待迁移到按自己 PID'
        'l5y_reconnect.ps1'   = '断线重连需要真杀服务端；待迁移到按自己 PID'
        'run_real_e2e.ps1'    = '仓库级 harness 开跑前清场；待迁移到按自己 PID'
    },
    [switch]$Strict,
    [switch]$SkipSelfTest
)
$ErrorActionPreference = 'Continue'

function Fail([string]$msg) { Write-Host ("FAIL(前置)：{0}" -f $msg); exit 2 }

function Find-ProcessNameKill {
    <#
      单文件判据：只看**代码行**（注释里常写"原先按进程名杀…"这种说明，算进来门禁自己就成噪音）。
      命中条件（二者其一）：
        ① 该行有 Stop-Process，且引用了共享名 mir2_server / client_bevy（\b 边界，故
           `Stop-Process -Name mir2_server_ci_unique` 这种唯一命名不会被误判）；
        ② 该行同时有 Get-Process 与 Stop-Process（`Get-Process -Name x | … | Stop-Process` 管道）。
    #>
    param([Parameter(Mandatory)][string]$Path)
    $hits = @()
    $code = @(Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue |
        Where-Object { $_.Trim() -and -not $_.Trim().StartsWith('#') })
    foreach ($ln in $code) {
        if ($ln -notmatch 'Stop-Process') { continue }
        if (($ln -match '\b(mir2_server|client_bevy)\b') -or ($ln -match 'Get-Process')) {
            $hits += $ln.Trim()
        }
    }
    $hits
}

if ($ScanDir.Count -eq 0) {
    $root = (Resolve-Path "$PSScriptRoot\..\..").Path
    $ScanDir = @((Join-Path $root 'tools/ops'), (Join-Path $root 'tools/acceptance'), (Join-Path $root 'scripts'))
}
$selfLeaf = Split-Path -Leaf $PSCommandPath
$files = @()
foreach ($d in $ScanDir) {
    if (-not (Test-Path -LiteralPath $d)) { Fail "扫描目录不存在：$d" }
    $files += @(Get-ChildItem -LiteralPath $d -Filter *.ps1 -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -ne $selfLeaf } | ForEach-Object { $_.FullName })
}
if ($files.Count -eq 0) { Fail "扫描面为空（$($ScanDir -join ', ')）——判据没跑起来，不许当绿" }

# ---------------- 沙箱正/负对照：判据不许空转 ----------------
if (-not $SkipSelfTest) {
    $sb = Join-Path ([System.IO.Path]::GetTempPath()) ("crystal_procscope_" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Path $sb -Force | Out-Null
    $enc = New-Object System.Text.UTF8Encoding($false)
    try {
        # 正对照：两种按共享名杀法，都必须被判违规
        [System.IO.File]::WriteAllText((Join-Path $sb 'bad_pipeline.ps1'),
            "Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force`n", $enc)
        [System.IO.File]::WriteAllText((Join-Path $sb 'bad_stopbyname.ps1'),
            "Stop-Process -Name client_bevy -Force -ErrorAction SilentlyContinue`n", $enc)
        # 负对照：按自己 PID / 按自己的唯一命名，都不许被判违规
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_pid.ps1'),
            "`$p = Start-Process -FilePath s.exe -PassThru`nif (-not `$p.HasExited) { Stop-Process -Id `$p.Id -Force }`n", $enc)
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_unique_name.ps1'),
            "Stop-Process -Name mir2_server_ci_unique -Force -ErrorAction SilentlyContinue`n", $enc)
        $bad1 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_pipeline.ps1'))
        $bad2 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_stopbyname.ps1'))
        $ok1 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_pid.ps1'))
        $ok2 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_unique_name.ps1'))
        $problems = @()
        if ($bad1.Count -eq 0) { $problems += '正对照1（Get-Process 管道杀 mir2_server）没被抓到 —— 判据空了' }
        if ($bad2.Count -eq 0) { $problems += '正对照2（Stop-Process -Name client_bevy）没被抓到 —— 判据空了' }
        if ($ok1.Count -ne 0) { $problems += '负对照1（按自己 PID 杀）被误判为违规' }
        if ($ok2.Count -ne 0) { $problems += '负对照2（按自己唯一命名杀）被误判为违规' }
        if ($problems.Count -gt 0) {
            foreach ($p in $problems) { Write-Host ("  [自检红] " + $p) -ForegroundColor Red }
            Fail ("本门禁自身判据不可信（沙箱 $sb）")
        }
        Write-Host '自检：沙箱正对照 2/2 乱杀被抓、负对照 2/2 合规写法未被误判 ✅'
    } finally {
        Remove-Item -LiteralPath $sb -Recurse -Force -ErrorAction SilentlyContinue
    }
} else {
    Write-Host '（-SkipSelfTest：本次没跑沙箱正/负对照）' -ForegroundColor DarkYellow
}

# ---------------- 主扫描 ----------------
$offenders = @()
foreach ($f in $files) {
    $hits = @(Find-ProcessNameKill -Path $f)
    if ($hits.Count -eq 0) { continue }
    $name = Split-Path -Leaf $f
    $offenders += [pscustomobject]@{
        file      = $name
        line      = $hits[0]
        count     = $hits.Count
        allowlist = ($Allowlist.ContainsKey($name))
        reason    = if ($Allowlist.ContainsKey($name)) { $Allowlist[$name] } else { '' }
    }
}

$new = @($offenders | Where-Object { -not $_.allowlist })
$known = @($offenders | Where-Object { $_.allowlist })
Write-Host ("扫描 {0} 个脚本（{1}）：按进程名杀共享资源的 {2} 个（待迁移 allowlist {3} 个、新增 {4} 个）" -f `
        $files.Count, ($ScanDir -join ' / '), $offenders.Count, $known.Count, $new.Count)
foreach ($o in $known) { Write-Host ("  [待迁移] {0} —— {1}" -f $o.file, $o.reason) -ForegroundColor DarkYellow }
foreach ($o in $new) {
    Write-Host ("  [违规]   {0} ({1} 处) —— {2}" -f $o.file, $o.count, $o.line) -ForegroundColor Red
    Write-Host ("           改用：`$proc = Start-Process … -PassThru；Stop-Process -Id `$proc.Id（绝不按共享名清场）") -ForegroundColor DarkGray
}

if ($new.Count -gt 0) { exit 1 }
if ($Strict -and $known.Count -gt 0) {
    Write-Host '  -Strict：待迁移清单也必须清空（全部迁移完后去掉 allowlist 里的条目）' -ForegroundColor Red
    exit 1
}
Write-Host '结果：无新增的按进程名杀共享资源 ✅'
exit 0
