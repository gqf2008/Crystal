#Requires -Version 5.1
<#
.SYNOPSIS
  安全地重启本机**共享 e2e 开发服**（默认 7000）：只从"含 origin/master 的检出"取产物，只杀端口持有者。

.DESCRIPTION
  为什么需要它：这台机器上的 e2e 夹具、验收与多个 agent 都连同一个 7000 开发服，而它的重启此前靠
  `%TEMP%\e2e_run\restart_new_server.ps1` 这种**临时脚本**，有两个会造成"证据失真"的坑：

  ① **产物来源会落后 master**：临时脚本从某个 worktree 的 `target\` 拷贝 exe。那个 worktree 是
     detached 的旧提交时，**每重启一次就把已合并的修复整体回滚**——2026-09-25 实际发生过一次
     （11:47 UTC 重启顶掉修复版，20:27 UTC 才恢复）。随后所有连 7000 的实机结论都在测旧代码。
  ② **按进程名清场**：`Get-CimInstance Win32_Process -Filter "Name='mir2_server.exe'" | Stop-Process`
     会连带杀掉**别人**正在跑的演练/验收（对方拿到 `result=4 密码错误` 这类假红），
     与本仓 `tools/ops/check_process_scope.ps1` 要防的正是同一类问题。

  本脚本把这两条都做成硬前置：
  - **来源校验**：先 `git fetch`，再要求源检出 `HEAD` **包含 `origin/master`**（即 `merge-base
    --is-ancestor origin/master HEAD`）；不满足直接 `exit 2` 并打印补救命令。想显式放行落后版本
    必须写 `-AllowBehindMaster`（会大声警告并写进记录）。
  - **接管前校验**：只认**监听该端口的那一个进程**，且要求它的可执行路径与"本脚本将要部署的那份"
    一致；否则 `exit 2` 并把占用者信息打出来（**绝不按名字清场**）。
  - **构建来源留痕**：重启成功后写 `<DeployDir>\server_build_record.json`（源检出 / 源 HEAD /
    origin/master HEAD / profile / exe sha256 / pid / port / 时间），"运行中的服务端是哪一份代码"
    从此有据可查。

.PARAMETER SourceRoot
  取产物的检出（默认 = 本脚本所在检出）。它必须能 `git fetch` 且 `HEAD` 包含 `origin/master`。

.PARAMETER Profile
  `debug`（默认，与既有 e2e 用法一致）或 `release`。

.PARAMETER SkipBuild
  不重新构建、直接用 `target\<Profile>` 里已有的 exe。**注意**：这时"exe 是否真由当前 HEAD 构建"
  无法证明（旧 target 残留是本仓的老坑），脚本会打 WARN。

.PARAMETER SelfTest
  判据自检（不需要起服）：用一个合成 git 仓库验"落后即拒 / 跟上即收"，用一个真监听进程验
  "按端口找持有者 + 预期 exe 不匹配即拒"，并静态自检本脚本自己没有按名清场。

.NOTES
  退出码：0 成功 / 1 自检失败 / 2 前置失败（来源落后、端口被别人的实例占着、exe 缺失）/ 5 起服或校验失败。
#>
param(
    [string]$SourceRoot = '',
    [ValidateSet('debug', 'release')][string]$Profile = 'debug',
    [int]$Port = 7000,
    [string]$DeployDir = '',
    [string]$DataRoot = '',
    [string]$ServerExe = '',
    [switch]$SkipFetch,
    [switch]$SkipBuild,
    [switch]$AllowBehindMaster,
    [switch]$SelfTest,
    [int]$ReadyTimeoutSec = 90
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $SourceRoot) { $SourceRoot = (Resolve-Path "$ops\..\..").Path }
if (-not $DeployDir) { $DeployDir = Join-Path ([System.IO.Path]::GetTempPath()) 'e2e_run' }
if (-not $DataRoot) { $DataRoot = Join-Path $SourceRoot 'ServerRust' }
if (-not $ServerExe) { $ServerExe = Join-Path $DataRoot ("target\{0}\mir2_server.exe" -f $Profile) }
$deployExe = Join-Path $DeployDir 'mir2_server.exe'

function Invoke-GitText {
    param([Parameter(Mandatory)][string]$Dir, [Parameter(Mandatory)][string[]]$Args)
    $out = & git -C $Dir @Args 2>&1
    [pscustomobject]@{ exit = $LASTEXITCODE; text = (@($out) -join "`n").Trim() }
}

function Get-GitHead {
    param([Parameter(Mandatory)][string]$Dir)
    (Invoke-GitText -Dir $Dir -Args @('rev-parse', 'HEAD')).text
}

function Get-OriginMasterHead {
    param([Parameter(Mandatory)][string]$Dir)
    (Invoke-GitText -Dir $Dir -Args @('rev-parse', 'origin/master')).text
}

function Test-ContainsMaster {
    <# 源检出 HEAD 是否**包含** origin/master（= 不落后）。落后 ⇒ 部署它会回滚已合并的修复。#>
    param([Parameter(Mandatory)][string]$Dir)
    (Invoke-GitText -Dir $Dir -Args @('merge-base', '--is-ancestor', 'origin/master', 'HEAD')).exit -eq 0
}

function Get-PortOwner {
    <# 只按**端口**定位持有者——绝不按进程名。#>
    param([Parameter(Mandatory)][int]$Port)
    $conn = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $conn) { return $null }
    $proc = Get-Process -Id $conn.OwningProcess -ErrorAction SilentlyContinue
    if (-not $proc) { return $null }
    [pscustomobject]@{ pid = $proc.Id; exe = $proc.Path; started = $proc.StartTime }
}

function Test-CanTakeOver {
    <#
      能不能接管这个端口：没人监听 ⇒ 可以；有人在听 ⇒ 只有当它的可执行路径**就是**我们要部署的那份
      （即"这是我自己的 e2e 服务端"）才允许停它——否则是别人的实例，必须拒。
    #>
    param(
        [Parameter(Mandatory)][int]$Port,
        [Parameter(Mandatory)][string]$ExpectedExe
    )
    $owner = Get-PortOwner -Port $Port
    if (-not $owner) { return [pscustomobject]@{ ok = $true; owner = $null; reason = '端口空闲' } }
    $same = ($owner.exe -and ((Resolve-Path -LiteralPath $owner.exe -EA SilentlyContinue).Path -eq
                              (Resolve-Path -LiteralPath $ExpectedExe -EA SilentlyContinue).Path))
    if ($same) { return [pscustomobject]@{ ok = $true; owner = $owner; reason = '占用者就是本脚本要部署的那份 e2e 服务端' } }
    [pscustomobject]@{
        ok = $false; owner = $owner
        reason = ("端口 {0} 被**别的**进程占着：pid={1} exe={2}" -f $Port, $owner.pid, $owner.exe)
    }
}

if ($SelfTest) {
    Write-Host 'SelfTest：来源校验 + 接管校验 + 自检"没有按名清场"'
    $bad = @()
    # A) 合成 git 仓库：HEAD 落后 origin/master 必须拒、跟上必须收
    $sb = Join-Path ([System.IO.Path]::GetTempPath()) ("e2e_restart_selftest_" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Path $sb | Out-Null
    try {
        & git -C $sb init -q 2>&1 | Out-Null
        & git -C $sb config user.email 'selftest@local' 2>&1 | Out-Null
        & git -C $sb config user.name 'selftest' 2>&1 | Out-Null
        Set-Content -LiteralPath (Join-Path $sb 'a.txt') -Value 'A' -Encoding utf8
        & git -C $sb add a.txt 2>&1 | Out-Null
        & git -C $sb commit -qm 'A' 2>&1 | Out-Null
        $shaA = (& git -C $sb rev-parse HEAD).Trim()
        Set-Content -LiteralPath (Join-Path $sb 'a.txt') -Value 'B' -Encoding utf8
        & git -C $sb add a.txt 2>&1 | Out-Null
        & git -C $sb commit -qm 'B' 2>&1 | Out-Null
        $shaB = (& git -C $sb rev-parse HEAD).Trim()
        & git -C $sb update-ref refs/remotes/origin/master $shaB 2>&1 | Out-Null
        & git -C $sb checkout -q --detach $shaA 2>&1 | Out-Null
        $staleOk = -not (Test-ContainsMaster -Dir $sb)
        if (-not $staleOk) { $bad += 'A1：HEAD 落后 origin/master 时 Test-ContainsMaster 竟然为真（落后会被放过）' }
        & git -C $sb checkout -q --detach $shaB 2>&1 | Out-Null
        $freshOk = (Test-ContainsMaster -Dir $sb)
        if (-not $freshOk) { $bad += 'A2：HEAD 就是 origin/master 时 Test-ContainsMaster 竟然为假（跟上的会被拒）' }
        Write-Host ("  [{0}] A 来源校验：落后→拒 {1}；跟上→收 {2}（{3} → {4}）" -f `
            $(if ($staleOk -and $freshOk) { 'PASS' } else { 'FAIL' }), $staleOk, $freshOk,
            $shaA.Substring(0, 7), $shaB.Substring(0, 7))
    } finally {
        Remove-Item -LiteralPath $sb -Recurse -Force -ErrorAction SilentlyContinue
    }
    # B) 真监听进程：按端口能定位持有者；预期 exe 不匹配必须拒
    $free = 7100
    while (Get-NetTCPConnection -LocalPort $free -State Listen -ErrorAction SilentlyContinue) { $free++ }
    $listener = Start-Process -FilePath (Get-Process -Id $PID).Path -PassThru -WindowStyle Hidden -ArgumentList @(
        '-NoProfile', '-Command',
        "`$l=[System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback,$free);`$l.Start();Start-Sleep -Seconds 12"
    )
    try {
        $owner = $null
        foreach ($i in 1..20) { Start-Sleep -Milliseconds 500; $owner = Get-PortOwner -Port $free; if ($owner) { break } }
        if (-not $owner) {
            $bad += ("B1：合成监听进程起来后按端口找不到持有者（port={0}）" -f $free)
        } else {
            $refuse = Test-CanTakeOver -Port $free -ExpectedExe (Join-Path $env:TEMP 'not-my-server.exe')
            $accept = Test-CanTakeOver -Port $free -ExpectedExe $owner.exe
            if ($refuse.ok) { $bad += 'B2：占用者不是我们那份时竟然允许接管（会杀别人的进程）' }
            if (-not $accept.ok) { $bad += 'B3：占用者正是我们那份时竟然拒绝接管' }
            Write-Host ("  [{0}] B 接管校验：找到 pid={1}；非本份→拒 {2}；本份→收 {3}" -f `
                $(if ((-not $refuse.ok) -and $accept.ok) { 'PASS' } else { 'FAIL' }), $owner.pid, (-not $refuse.ok), $accept.ok)
        }
    } finally {
        if ($listener -and -not $listener.HasExited) { Stop-Process -Id $listener.Id -Force -ErrorAction SilentlyContinue }
    }
    # C) 静态自检：本脚本自己不许出现"按名清场"的写法。两步防自伤：
    #    ① 先按注释语义剥掉 `#` 行注释与 `<# … #>` 块注释（帮助块里正是那句反面教材）；
    #    ② 判据字面量本身拼串写，避免自检命中自己的判据文本。
    $selfCode = @()
    $inB = $false
    foreach ($raw in (Get-Content -LiteralPath $PSCommandPath -ErrorAction SilentlyContinue)) {
        $t = $raw.Trim()
        if ($inB) {
            $e = $t.IndexOf('#>')
            if ($e -lt 0) { continue }
            $inB = $false
            $t = $t.Substring($e + 2).Trim()
        }
        if ($t.StartsWith('<#')) {
            $e = $t.IndexOf('#>')
            if ($e -lt 2) { $inB = $true; continue }
            $t = $t.Substring($e + 2).Trim()
        }
        if (-not $t) { continue }
        if ($t.StartsWith('#') -or $t -match '^(?i)(@?REM\b|::)') { continue }
        $selfCode += $t
    }
    $selfText = ($selfCode -join "`n")
    $patFilter = '-Fil' + 'ter "Name='
    $patNameKill = 'Stop-Process ' + '-Name'
    $selfBad = @()
    # 连**报错文案**也要拼串：否则自检会把"它自己的提示文本"当成违规（本脚本第一版就是这么自伤的）。
    if ($selfText -match [regex]::Escape($patFilter)) { $selfBad += ('出现了 ' + $patFilter + ' 形态的按名查询') }
    if ($selfText -match [regex]::Escape($patNameKill)) { $selfBad += ('出现了 ' + $patNameKill) }
    if ($selfBad.Count -gt 0) { $bad += ('C1：本脚本自己按名清场：' + ($selfBad -join '；')) }
    Write-Host ("  [{0}] C 静态自检：本脚本没有按名清场写法" -f $(if ($selfBad.Count -eq 0) { 'PASS' } else { 'FAIL' }))
    if ($bad.Count -gt 0) { Write-Host ("SelfTest FAIL：{0}" -f ($bad -join '；')); exit 1 }
    Write-Host 'SelfTest PASS：来源/接管/自检三条都对'
    exit 0
}

# ---------------- 前置：来源必须是"含 origin/master"的检出 ----------------
if (-not (Test-Path -LiteralPath (Join-Path $SourceRoot '.git'))) { Write-Host "FAIL(前置)：$SourceRoot 不是 git 检出"; exit 2 }
if (-not $SkipFetch) {
    $f = Invoke-GitText -Dir $SourceRoot -Args @('fetch', 'origin', '--prune')
    if ($f.exit -ne 0) { Write-Host ("FAIL(前置)：git fetch 失败：{0}" -f $f.text); exit 2 }
}
$head = Get-GitHead -Dir $SourceRoot
$master = Get-OriginMasterHead -Dir $SourceRoot
$contains = Test-ContainsMaster -Dir $SourceRoot
Write-Host ("源检出 {0}`n  HEAD          = {1}`n  origin/master = {2}" -f $SourceRoot, $head, $master)
if (-not $contains) {
    if (-not $AllowBehindMaster) {
        Write-Host 'FAIL(前置)：源检出**落后 origin/master** —— 用它的产物重启会把已合并的修复回滚。' -ForegroundColor Red
        Write-Host '  补救（择一，然后在**含 master 的检出**上重跑本脚本）：'
        Write-Host ('    git -C "{0}" fetch origin --prune; git -C "{0}" checkout --detach origin/master   # 临时/scratch 工作树' -f $SourceRoot)
        Write-Host ('    git -C "{0}" fetch origin --prune; git -C "{0}" merge --ff-only origin/master    # 长期检出（先确认工作区干净）' -f $SourceRoot)
        Write-Host '  确要放行落后版本：加 -AllowBehindMaster（会写进 server_build_record.json）。'
        exit 2
    }
    Write-Host 'WARN：-AllowBehindMaster 已放行**落后 master** 的产物 —— 连这个服务端的实机结论可能不反映 master。' -ForegroundColor DarkYellow
}

# ---------------- 构建 ----------------
if (-not $SkipBuild) {
    Write-Host ("[1/4] cargo build --{0} --bin mir2_server（源：{1}）" -f $Profile, $SourceRoot)
    Push-Location $DataRoot
    try {
        $buildArgs = @('build')
        if ($Profile -eq 'release') { $buildArgs += '--release' }
        $buildArgs += @('--bin', 'mir2_server')
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) { Write-Host ("FAIL(前置)：cargo build exit={0}" -f $LASTEXITCODE); exit 2 }
    } finally { Pop-Location }
} else {
    Write-Host 'WARN：-SkipBuild —— 无法证明 exe 是由当前 HEAD 构建的（旧 target 残留是本仓老坑）。' -ForegroundColor DarkYellow
}
if (-not (Test-Path -LiteralPath $ServerExe)) { Write-Host "FAIL(前置)：找不到产物 $ServerExe"; exit 2 }
$builtHash = (Get-FileHash -LiteralPath $ServerExe -Algorithm SHA256).Hash

# ---------------- 接管：只动端口持有者，且必须是"我们这份" ----------------
New-Item -ItemType Directory -Path $DeployDir -Force | Out-Null
$take = Test-CanTakeOver -Port $Port -ExpectedExe $deployExe
if (-not $take.ok) {
    Write-Host ("FAIL(前置)：{0}" -f $take.reason) -ForegroundColor Red
    Write-Host '  本脚本只接管"端口上那个进程，且它的 exe 就是本 e2e 部署目录里的那份"；'
    Write-Host '  别人的实例（别的 deploy 目录 / 别人的演练）一律不动 —— 按名清场会杀到他们。'
    exit 2
}
if ($take.owner) {
    Write-Host ("[2/4] 停掉端口 {0} 的持有者 pid={1}（就是本部署目录那份）" -f $Port, $take.owner.pid)
    Stop-Process -Id $take.owner.pid -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
} else {
    Write-Host ("[2/4] 端口 {0} 空闲，直接部署" -f $Port)
}

# ---------------- 部署 + 起服 ----------------
Write-Host ("[3/4] 部署 {0} → {1}" -f $ServerExe, $deployExe)
Copy-Item -LiteralPath $ServerExe -Destination $deployExe -Force
$outLog = Join-Path $DeployDir 'server.out.log'
$errLog = Join-Path $DeployDir 'server.err.log'
Start-Process -FilePath $deployExe -WorkingDirectory $DataRoot -WindowStyle Hidden `
    -RedirectStandardOutput $outLog -RedirectStandardError $errLog | Out-Null
$up = $false
$owner = $null
foreach ($i in 1..$ReadyTimeoutSec) {
    Start-Sleep -Seconds 1
    $owner = Get-PortOwner -Port $Port
    if ($owner) { $up = $true; break }
}
if (-not $up) {
    Write-Host ("FAIL：{0}s 内端口 {1} 没有起来（日志尾：{2}）" -f $ReadyTimeoutSec, $Port,
        ((Get-Content -LiteralPath $errLog -Tail 3 -EA SilentlyContinue) -join ' | '))
    exit 5
}
$runningHash = (Get-FileHash -LiteralPath $owner.exe -Algorithm SHA256).Hash
if ($runningHash -ne $builtHash) {
    Write-Host ("FAIL：端口 {0} 起来的进程 exe 与刚部署的产物不一致（{1} vs {2}）" -f $Port, $runningHash, $builtHash)
    exit 5
}

# ---------------- 留痕 ----------------
$record = [ordered]@{
    ts_utc              = (Get-Date).ToUniversalTime().ToString('o')
    port                = $Port
    pid                 = $owner.pid
    source_root         = $SourceRoot
    source_head         = $head
    origin_master_head  = $master
    source_contains_master = $contains
    allowed_behind_master  = [bool]$AllowBehindMaster
    profile             = $Profile
    skipped_build       = [bool]$SkipBuild
    server_exe          = $ServerExe
    deployed_exe        = $deployExe
    exe_sha256          = $builtHash
    data_root           = $DataRoot
    operator            = $env:USERNAME
}
$recordPath = Join-Path $DeployDir 'server_build_record.json'
($record | ConvertTo-Json -Depth 4) | Set-Content -LiteralPath $recordPath -Encoding utf8
Write-Host ("[4/4] PASS：pid={0} 监听 {1}；exe sha256={2}；构建来源 HEAD={3}（含 origin/master={4}）" -f `
    $owner.pid, $Port, $builtHash.Substring(0, 12), $head.Substring(0, 12), $contains)
Write-Host ("  记录：{0}" -f $recordPath)
exit 0
