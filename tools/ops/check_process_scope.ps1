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
    # 待迁移清单：**已清空**（2026-09-25）——原先记名的三条（fault_injection / l5y_reconnect /
    # run_real_e2e）都已改成"只清自己的"：故障注入按自己的 **PID** 杀；l5y 按自己的 PID + 自己的 exe 路径；
    # run_real_e2e 按**自己那份构建的 exe 路径**过滤。加上批次 #3181 的 20 个夹具迁移到唯一命名副本后，
    # 全仓扫描下来已经**没有任何"按进程名杀共享资源"**了。
    # 保留这个参数是为了将来真需要临时豁免时有地方记名（`-Strict` 会连它们一起报红）。
    [hashtable]$Allowlist = @{},
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
        ③ **跨行管道**（2026-09-25 补的盲区）：按名字查到进程对象、再由**同一条管道**下游的
           Stop-Process 杀掉——典型形态是 `Get-CimInstance Win32_Process -Filter "Name='mir2_server.exe'" |`
           换行后 `ForEach-Object { Stop-Process -Id $_.ProcessId }`。这里 Stop-Process 用的是 `-Id`，
           乍看像"按 PID"，但那个 Id 是**按名字查来的**，所以照样是清共享资源。
           判据按**管道**走，不按"行窗口"走：查询行必须以 `|` 结尾，然后沿管道往下找 Stop-Process，
           遇到不以 `|` 结尾的行就说明管道结束。这样 `if (-not (Get-Process -Name mir2_server …)) { … exit 9 }`
           这种只做**存在性探测**的写法不会被误判（第一版按 4 行窗口找，实测把 14 个夹具的存在性探测
           全误报成违规——门禁自己成了噪音）。
           共享名一律带 `\b`（`client_bevy_l5t.exe` 这类**唯一命名**是 LESSON 推荐的做法，不算违规）。
           带每次运行唯一判别（CommandLine 上的 `--control-port <本脚本端口>`）的清残留也放过。
    #>
    param([Parameter(Mandatory)][string]$Path)
    $hits = @()
    $code = @(Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue |
        Where-Object { $_.Trim() -and -not $_.Trim().StartsWith('#') })
    for ($i = 0; $i -lt $code.Count; $i++) {
        $ln = $code[$i]
        if ($ln -match 'Stop-Process') {
            if (($ln -match '\b(mir2_server|client_bevy)\b') -or ($ln -match 'Get-Process')) {
                $hits += $ln.Trim()
            }
            continue
        }
        $byName = ($ln -match 'Get-Process\s+-Name\s+.*\b(mir2_server|client_bevy)\b') -or
        ($ln -match 'Get-CimInstance' -and $ln -match "Name\s*=\s*['\`"](mir2_server|client_bevy)\b")
        if (-not $byName) { continue }
        if ($ln.TrimEnd() -notmatch '\|$') { continue }   # 不是管道 → 只是查询/探测
        $j = $i + 1
        $violation = $false
        while ($j -lt $code.Count) {
            if ($code[$j] -match 'Stop-Process') {
                # 管道里带"只认自己那份"的判别就不算违规：
                #   · CommandLine —— 按本次运行唯一的端口/唯一名过滤；
                #   · ExecutablePath / $_ .Path —— 按**自己那份构建的 exe 路径**过滤
                #     （2026-09-25 收口 fault_injection / l5y_reconnect / run_real_e2e 时用的形态：
                #      它们要清的是"自己 deploy 目录/自己 build 出来的实例"，不是同机所有同名进程）。
                $violation = @($code[$i..$j] | Where-Object { $_ -match 'CommandLine|ExecutablePath|\.Path\s' }).Count -eq 0
                break
            }
            if ($code[$j].TrimEnd() -notmatch '\|$') { break }
            $j++
        }
        if ($violation) { $hits += $ln.Trim() }
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
        # 正对照 3（2026-09-25 补的盲区）：跨行管道——按名字查、换行后 Stop-Process -Id $_.ProcessId。
        # 这是 leak_plateau.ps1 里真实存在的形态，第一版判据（只认 Get-Process / 同行 -Name）漏掉了它。
        [System.IO.File]::WriteAllText((Join-Path $sb 'bad_crossline.ps1'),
            "Get-CimInstance Win32_Process -Filter `"Name='mir2_server.exe'`" -EA SilentlyContinue |`n" +
            "    ForEach-Object { Stop-Process -Id `$_.ProcessId -Force -EA SilentlyContinue }`n", $enc)
        # 负对照 3：只做**存在性探测**（后面接 exit 9）——不是清场，不许误判
        # （第一版按"4 行窗口"找 Stop-Process，把 14 个夹具的存在性探测全误报成违规。）
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_probe.ps1'),
            "if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }`n", $enc)
        # 负对照 4：带"每次运行唯一"判别的清残留（CommandLine 上的端口）——允许
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_scoped.ps1'),
            "Get-CimInstance Win32_Process -Filter `"Name='client_bevy.exe'`" |`n" +
            "    Where-Object { `$_.CommandLine -match `"--control-port\s+`$Port\b`" } |`n" +
            "    ForEach-Object { Stop-Process -Id `$_.ProcessId -Force }`n", $enc)
        # 负对照 5：按**自己那份 exe 路径**过滤的清场——允许（只清自己的构建，不碰同机同名进程）
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_path_scoped.ps1'),
            "Get-CimInstance Win32_Process -Filter `"Name='mir2_server.exe'`" |`n" +
            "    Where-Object { `$_.ExecutablePath -eq `$ServerExe } |`n" +
            "    ForEach-Object { Stop-Process -Id `$_.ProcessId -Force }`n", $enc)
        $bad1 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_pipeline.ps1'))
        $bad2 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_stopbyname.ps1'))
        $bad3 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_crossline.ps1'))
        $ok1 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_pid.ps1'))
        $ok2 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_unique_name.ps1'))
        $ok3 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_probe.ps1'))
        $ok4 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_scoped.ps1'))
        $ok5 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_path_scoped.ps1'))
        $problems = @()
        if ($bad1.Count -eq 0) { $problems += '正对照1（Get-Process 管道杀 mir2_server）没被抓到 —— 判据空了' }
        if ($bad2.Count -eq 0) { $problems += '正对照2（Stop-Process -Name client_bevy）没被抓到 —— 判据空了' }
        if ($bad3.Count -eq 0) { $problems += '正对照3（跨行管道：Get-CimInstance 按名查 → Stop-Process -Id）没被抓到 —— 判据空了' }
        if ($ok1.Count -ne 0) { $problems += '负对照1（按自己 PID 杀）被误判为违规' }
        if ($ok2.Count -ne 0) { $problems += '负对照2（按自己唯一命名杀）被误判为违规' }
        if ($ok3.Count -ne 0) { $problems += '负对照3（存在性探测，后面接 exit 9）被误判为违规' }
        if ($ok4.Count -ne 0) { $problems += '负对照4（带 CommandLine 唯一判别的清残留）被误判为违规' }
        if ($ok5.Count -ne 0) { $problems += '负对照5（按自己 exe 路径过滤的清场）被误判为违规' }
        if ($problems.Count -gt 0) {
            foreach ($p in $problems) { Write-Host ("  [自检红] " + $p) -ForegroundColor Red }
            Fail ("本门禁自身判据不可信（沙箱 $sb）")
        }
        Write-Host '自检：沙箱正对照 3/3 乱杀被抓（含跨行管道）、负对照 5/5 合规写法未被误判 ✅'
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
