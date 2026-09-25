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
# 扫描面与"杀法"都不止 PowerShell（2026-09-25 补，与实机锁覆盖面那轮的扩展名盲区同型）：
#   · 扫描面 = 整仓的 `*.ps1` / `*.bat` / `*.cmd`（写死 `.ps1` 时，一条
#     `taskkill /F /IM mir2_server.exe` 的批处理可以整体绕开这道门禁）；
#   · 批处理里的"按名杀"也纳入判据：`taskkill /IM <共享名>.exe`、`wmic process where name='<共享名>.exe' … delete`；
#   · 唯一命名照样放过（`taskkill /IM client_bevy_l5t.exe` 不违规），只探测不杀的 `tasklist | findstr …`
#     也不违规；批处理的注释（`REM` / `::` / `@REM`）与 PowerShell 的 `#` 一样先剔掉，避免门禁自己变噪音。
#   · **注释剔除要连 `<# … #>` 块注释一起剔**（2026-09-25 补）：只剥 `#` 行注释时，
#     「在 comment-based help 里解释"原先按名清场长什么样"」的脚本会被误报成违规
#     （实测：新增的 tools/ops/restart_e2e_server.ps1 的帮助块里写了那句反面教材，立刻见红）。
#     注释不是代码，判据只该看代码行。
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
        ① 该行有 Stop-Process，且引用了**共享名** mir2_server / client_bevy / Client（原版 C# 客户端，
           本机多 agent 会用同一份原版做 A/B 对照，属共享资源）/ mir2_login（\b 边界，故
           `Stop-Process -Name mir2_server_ci_unique` 这种唯一命名不会被误判）；
        ② 该行同时有 Get-Process 与 Stop-Process**且**引用了共享名或没写 `-Name`
           （`Get-Process | Stop-Process` 等于清全场）。
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
    # 注释先剔掉：PowerShell 的 `#` 行注释与 `<# … #>` 块注释、批处理的 `REM`/`@REM`/`::`。
    # 注释里常写"原先按进程名杀…"这种反面教材，算进门禁自己就成噪音；判据只该看**代码行**。
    # 只探测不杀的写法（tasklist/findstr、Get-Process 存在性判断）不受影响：下面的判据都要求
    # 真的出现"杀"的动作。
    $code = @()
    $inBlock = $false
    foreach ($raw in (Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue)) {
        $t = $raw.Trim()
        if ($inBlock) {
            $end = $t.IndexOf('#>')
            if ($end -lt 0) { continue }
            $inBlock = $false
            $t = $t.Substring($end + 2).Trim()
        }
        if ($t.StartsWith('<#')) {
            $end = $t.IndexOf('#>')
            if ($end -lt 2) { $inBlock = $true; continue }
            $t = $t.Substring($end + 2).Trim()
        }
        if (-not $t) { continue }
        if ($t.StartsWith('#') -or $t -match '^(?i)(@?REM\b|::)') { continue }
        $code += $t
    }
    for ($i = 0; $i -lt $code.Count; $i++) {
        $ln = $code[$i]
        # 共享资源名：服务端 / Bevy 客户端 / **原版 C# 客户端 `Client.exe`**（本机多 agent 会用同一份原版
        # 做逐窗 A/B 对照）/ 登录探针用的 mir2_login。唯一命名（如 `mir2_server_ci_unique`）因 \b 不匹配。
        $shared = '\b(mir2_server|client_bevy|Client|mir2_login)\b'
        # 批处理/命令行里的"按名杀"：taskkill /IM xxx.exe、wmic … where name='xxx.exe' … delete。
        # 与 PowerShell 那两支并列，共享名/唯一命名的判定口径完全一致（同一份 $shared 正则）。
        if ($ln -match '(?i)\btaskkill\b' -and $ln -match '(?i)/IM\b' -and $ln -match $shared) {
            $hits += $ln.Trim()
            continue
        }
        if ($ln -match '(?i)\bwmic\b' -and $ln -match '(?i)\bdelete\b' -and
            $ln -match "(?i)name\s*=\s*['`"]?[^'`"]*$shared") {
            $hits += $ln.Trim()
            continue
        }
        if ($ln -match 'Stop-Process') {
            # ① 同行按共享名杀；② 同行 Get-Process（无 -Name 的全场清 / 或按共享名）再杀
            $isSharedByName = $ln -match $shared
            $isGetProcessKill = ($ln -match 'Get-Process') -and (($ln -match $shared) -or ($ln -notmatch 'Get-Process\s+-Name'))
            if ($isSharedByName -or $isGetProcessKill) {
                $hits += $ln.Trim()
            }
            continue
        }
        $byName = ($ln -match "Get-Process\s+-Name\s+.*$shared") -or
        ($ln -match 'Get-CimInstance' -and $ln -match "Name\s*=\s*['\`"](mir2_server|client_bevy|Client|mir2_login)\b")
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

function Get-ScannedScripts {
    <#
      扫描面 = 给定目录下所有 `*.ps1` / `*.bat` / `*.cmd`（排除 `.git`/`target`/`node_modules` 与本门禁自己）。
      抽成函数是为了让自检能对"**扫描面是否真的包含 `.bat`/`.cmd`**"做阳性对照——写死 `*.ps1` 时自检即红。
    #>
    param([Parameter(Mandatory)][string[]]$Dirs, [string]$SelfLeaf = '')
    $exts = @('.ps1', '.bat', '.cmd')
    $skipDirs = '\\(\.git|target|node_modules)\\'
    $out = @()
    foreach ($d in $Dirs) {
        $out += @(Get-ChildItem -LiteralPath $d -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object { $exts -contains $_.Extension.ToLowerInvariant() -and $_.FullName -notmatch $skipDirs } |
            Where-Object { (-not $SelfLeaf) -or ($_.Name -ne $SelfLeaf) } |
            ForEach-Object { $_.FullName })
    }
    $out
}

$userScanDirs = $ScanDir.Count   # 0 = 用默认扫描面（此时必须覆盖整仓，见下面的盲区回归锁）
if ($ScanDir.Count -eq 0) {
    # 2026-09-25 晚：扫描面从「三个目录」改成**整仓**（排除 .git/target/node_modules）——
    # 与实机锁覆盖面门禁（Get-E2eClientScripts）同口径。理由同那份的经验：
    # **写死目录清单本身就是缺口的第一候选**（新目录里的实机/清理脚本不会被看到）。
    # 实测：MapEditor\rust-map-editor\build.ps1 就在旧清单之外（现在被扫进来了）。
    $root = (Resolve-Path "$PSScriptRoot\..\..").Path
    $ScanDir = @($root)
}
$selfLeaf = Split-Path -Leaf $PSCommandPath
foreach ($d in $ScanDir) { if (-not (Test-Path -LiteralPath $d)) { Fail "扫描目录不存在：$d" } }
$files = @(Get-ScannedScripts -Dirs $ScanDir -SelfLeaf $selfLeaf)
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
        # 正对照 4/5（2026-09-25 补的**扩展名**盲区）：批处理里的按名杀。
        # 实测过：只扫 `*.ps1` 的旧实现看不见这两个文件 ⇒ `taskkill /F /IM mir2_server.exe` 能整体绕开门禁。
        [System.IO.File]::WriteAllText((Join-Path $sb 'bad_taskkill.bat'),
            "@echo off`r`ntaskkill /F /IM mir2_server.exe`r`n", $enc)
        [System.IO.File]::WriteAllText((Join-Path $sb 'bad_taskkill.cmd'),
            "@echo off`r`ntaskkill /IM client_bevy.exe /F`r`n", $enc)
        # 负对照 6：批处理按**自己的唯一命名**杀——允许（与 PowerShell 侧同口径）
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_taskkill_unique.bat'),
            "@echo off`r`ntaskkill /F /IM client_bevy_l5t.exe`r`n", $enc)
        # 负对照 7：批处理只**探测**不杀（tasklist/findstr）——不许误判
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_batch_probe.bat'),
            "@echo off`r`ntasklist /FI `"IMAGENAME eq mir2_server.exe`" | findstr /I mir2_server.exe >nul`r`n", $enc)
        # 负对照 8：批处理注释里提到"原先按名杀 mir2_server"——注释剔除后不许误判
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_batch_comment.bat'),
            "@echo off`r`nREM 原先 taskkill /F /IM mir2_server.exe 会杀别人的开发服，现改成按自己 PID`r`n" +
            ":: taskkill /IM client_bevy.exe /F`r`n", $enc)
        # 负对照 9（2026-09-25 补的注释盲区）：**PowerShell 块注释**里写反面教材——不许误判。
        # 实测：只剥 `#` 行注释时，新脚本 comment-based help 里那句"原先 Get-CimInstance … | Stop-Process"
        # 会被判成违规（门禁对着**文档**开火）。注释不是代码。
        [System.IO.File]::WriteAllText((Join-Path $sb 'good_block_comment.ps1'),
            "<#`n  反面教材（**不要照抄**）：Get-CimInstance Win32_Process -Filter `"Name='mir2_server.exe'`" |`n" +
            "      ForEach-Object { Stop-Process -Id `$_.ProcessId -Force }`n#>`nWrite-Host 'ok'`n", $enc)
        # 正对照 6：块注释结束后**同一行**还有真代码时，那行代码仍要判（别把整行都当注释吃掉）
        [System.IO.File]::WriteAllText((Join-Path $sb 'bad_block_tail.ps1'),
            "<# 说明 #> Stop-Process -Name mir2_server -Force -EA SilentlyContinue`n", $enc)
        $bad1 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_pipeline.ps1'))
        $bad2 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_stopbyname.ps1'))
        $bad3 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_crossline.ps1'))
        $bad4 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_taskkill.bat'))
        $bad5 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_taskkill.cmd'))
        $ok1 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_pid.ps1'))
        $ok2 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_unique_name.ps1'))
        $ok3 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_probe.ps1'))
        $ok4 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_scoped.ps1'))
        $ok5 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_path_scoped.ps1'))
        $ok6 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_taskkill_unique.bat'))
        $ok7 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_batch_probe.bat'))
        $ok8 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_batch_comment.bat'))
        $ok9 = @(Find-ProcessNameKill -Path (Join-Path $sb 'good_block_comment.ps1'))
        $bad6 = @(Find-ProcessNameKill -Path (Join-Path $sb 'bad_block_tail.ps1'))
        # 扫描面自证：`.bat`/`.cmd` 必须在扫描面里（写死 `*.ps1` 时这里就是 0，自检即红）
        $sbScanned = @(Get-ScannedScripts -Dirs @($sb))
        $sbBatch = @($sbScanned | Where-Object { $_ -match '\.(bat|cmd)$' })
        $problems = @()
        if ($bad1.Count -eq 0) { $problems += '正对照1（Get-Process 管道杀 mir2_server）没被抓到 —— 判据空了' }
        if ($bad2.Count -eq 0) { $problems += '正对照2（Stop-Process -Name client_bevy）没被抓到 —— 判据空了' }
        if ($bad3.Count -eq 0) { $problems += '正对照3（跨行管道：Get-CimInstance 按名查 → Stop-Process -Id）没被抓到 —— 判据空了' }
        if ($bad4.Count -eq 0) { $problems += '正对照4（.bat：taskkill /F /IM mir2_server.exe）没被抓到 —— 扩展名盲区回来了' }
        if ($bad5.Count -eq 0) { $problems += '正对照5（.cmd：taskkill /IM client_bevy.exe /F）没被抓到 —— 扩展名盲区回来了' }
        if ($bad6.Count -eq 0) { $problems += '正对照6（块注释结束后同一行的真代码 Stop-Process -Name mir2_server）没被抓到 —— 剥注释剥过头了' }
        if ($sbBatch.Count -lt 5) { $problems += ("扫描面没覆盖 .bat/.cmd（只认出 {0} 个）—— 是不是又写死 *.ps1 了？" -f $sbBatch.Count) }
        if ($ok1.Count -ne 0) { $problems += '负对照1（按自己 PID 杀）被误判为违规' }
        if ($ok2.Count -ne 0) { $problems += '负对照2（按自己唯一命名杀）被误判为违规' }
        if ($ok3.Count -ne 0) { $problems += '负对照3（存在性探测，后面接 exit 9）被误判为违规' }
        if ($ok4.Count -ne 0) { $problems += '负对照4（带 CommandLine 唯一判别的清残留）被误判为违规' }
        if ($ok5.Count -ne 0) { $problems += '负对照5（按自己 exe 路径过滤的清场）被误判为违规' }
        if ($ok6.Count -ne 0) { $problems += '负对照6（.bat 按自己的唯一命名杀）被误判为违规' }
        if ($ok7.Count -ne 0) { $problems += '负对照7（.bat 只探测不杀：tasklist/findstr）被误判为违规' }
        if ($ok8.Count -ne 0) { $problems += '负对照8（.bat 注释里提到按名杀）被误判为违规' }
        if ($ok9.Count -ne 0) { $problems += '负对照9（PowerShell 块注释里写反面教材）被误判为违规' }
        if ($problems.Count -gt 0) {
            foreach ($p in $problems) { Write-Host ("  [自检红] " + $p) -ForegroundColor Red }
            Fail ("本门禁自身判据不可信（沙箱 $sb）")
        }
        Write-Host ('自检：沙箱正对照 6/6 乱杀被抓（含跨行管道、.bat/.cmd 的 taskkill、块注释后的真代码）、负对照 9/9 合规写法未被误判；' +
                    '扫描面含 .bat/.cmd（认出 {0} 个）✅' -f $sbBatch.Count)
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
# 盲区回归锁：默认扫描面必须是**整仓**，不能退回「写死的那三个目录」。
# 探针＝`MapEditor\rust-map-editor\build.ps1`（真实存在于旧清单之外的目录）。
$probe = @($files | Where-Object { $_ -match '\\MapEditor\\' -and $_.EndsWith('build.ps1') })
if ($userScanDirs -eq 0 -and $probe.Count -eq 0) {
    Write-Host '  [失效锁] 默认扫描面没覆盖到 MapEditor\ 下的脚本 —— 是不是又退回写死目录清单了？' -ForegroundColor Red
    exit 2
}
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
