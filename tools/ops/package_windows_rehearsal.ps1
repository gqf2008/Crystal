# package_windows_rehearsal.ps1 — 本机预演 CI 的 Windows 打包配方（发版日关键路径）
#
# 为什么单列：`.github/workflows/build.yml` 的 Windows 打包段是**发版日**才跑的关键路径，
# 而 `docs/DELIVERY.md` 对用户有一句强承诺——「Windows 客户端 zip 内已自动带上 MSYS2 UCRT64
# 运行库 DLL（libpinyin/glib/libdb 等），**解压即跑**」。这条承诺在本机从没被验证：
# 配方里的依赖 DLL 递归解析 / assets 与 libpinyin 数据 staging / 结构断言，任何一处漂移
# 都要等打 tag 发版才暴露。本脚本把同一配方在本机跑一遍，再解压到**干净目录**真启动一次。
#
# 判据：
#   J1 GNU release 产物存在
#   J2 staging 齐备（exe + 依赖 DLL + assets + libpinyin/{data,conf}）
#   J2b **依赖闭包**：stage 内所有 PE 的导入 DLL 要么已在 stage，要么是 Windows 系统 DLL
#       （这一条才是「解压即跑」的真正判据——CI 的 objdump 循环就是为它服务的）
#   J3 zip 生成且结构断言通过
#   J4 解压到干净目录后能启动（给了 -SmokeServer 则进一步验到「进图」）
param(
    [string]$RepoRoot = '',
    [string]$WorkDir = '',
    [int]$ControlPort = 9300,
    [string]$SmokeServer = '',
    [string]$E2eUser = 'test',
    [string]$E2ePass = '123456',
    [string]$DataDir = '',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $RepoRoot) { $RepoRoot = Split-Path -Parent (Split-Path -Parent $ops) }
if (-not $WorkDir) { $WorkDir = Join-Path $ops ('out/package_rehearsal_' + (Get-Date -Format 'HHmmss')) }
New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
$stage = Join-Path $WorkDir 'stage'
$unzip = Join-Path $WorkDir 'unzipped'
$ucrtBin = 'D:\toolchains\msys64\ucrt64\bin'
$objdump = Join-Path $ucrtBin 'objdump.exe'
$report = [ordered]@{ repo = $RepoRoot; work_dir = $WorkDir; steps = [ordered]@{} }
function Fail([string]$m) { Write-Host "FAIL $m"; $report | ConvertTo-Json -Depth 8; exit 3 }

function Get-ImportedDlls([string]$file) {
    if (-not (Test-Path $file)) { return @() }
    $out = & $objdump -p $file 2>$null
    @($out | Select-String -Pattern 'DLL Name:\s*(\S+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })
}
$sysDlls = @('kernel32.dll','user32.dll','gdi32.dll','advapi32.dll','shell32.dll','ole32.dll','oleaut32.dll',
    'ws2_32.dll','crypt32.dll','bcrypt.dll','ntdll.dll','msvcrt.dll','comdlg32.dll','winmm.dll','iphlpapi.dll',
    'dwmapi.dll','imm32.dll','secur32.dll','shlwapi.dll','wintrust.dll','d3d11.dll','dxgi.dll','opengl32.dll',
    'setupapi.dll','version.dll','uxtheme.dll','winspool.drv','comctl32.dll','dnsapi.dll','mswsock.dll')
# `api-ms-win-*` / `ext-ms-*` 是 Windows 自带的 **UCRT api-set 转发 DLL**（Win10+ 系统内即有；
# 旧系统需装 UCRT 更新）。CI 的配方只从 `/ucrt64/bin` 解析依赖，本就不把它们打进包——
# 它们属于"目标机应当提供"的系统依赖，不是打包漏项（本脚本把它们单独归类并记录）。
$apiSetPattern = '^(api-ms-win-|ext-ms-)'

# ---------------- J1 产物 ----------------
$exe = Join-Path $RepoRoot 'Client-Bevy\target\x86_64-pc-windows-gnu\release\client_bevy.exe'
if (-not (Test-Path $exe)) {
    Fail "J1 缺 GNU release 产物：$exe（先 CARGO_BUILD_TARGET=x86_64-pc-windows-gnu cargo build --release --bin client_bevy）"
}
$report.steps.artifact = [ordered]@{ exe = $exe; bytes = (Get-Item $exe).Length; built = (Get-Item $exe).LastWriteTime.ToString('s') }

# ---------------- J2 staging（语义照抄 CI：objdump 递归解析 /ucrt64/bin 依赖） ----------------
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item $exe $stage -Force
$pending = New-Object System.Collections.Generic.Queue[string]
$pending.Enqueue((Join-Path $stage 'client_bevy.exe'))
$seen = @{}
for ($i = 0; $i -lt 25 -and $pending.Count -gt 0; $i++) {
    $f = $pending.Dequeue()
    foreach ($dll in (Get-ImportedDlls $f)) {
        $key = $dll.ToLowerInvariant()
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $cand = Join-Path $ucrtBin $dll
        if (Test-Path $cand) {
            Copy-Item $cand $stage -Force
            $pending.Enqueue((Join-Path $stage $dll))
        }
    }
}
Copy-Item (Join-Path $RepoRoot 'Client-Bevy\assets') (Join-Path $stage 'assets') -Recurse -Force
$lpData = 'D:\toolchains\libpinyin-install\lib\libpinyin'
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'libpinyin\conf') | Out-Null
if (Test-Path (Join-Path $lpData 'conf')) { Copy-Item (Join-Path $lpData 'conf\*') (Join-Path $stage 'libpinyin\conf') -Force }
Copy-Item (Join-Path $lpData 'data') (Join-Path $stage 'libpinyin\data') -Recurse -Force
$dlls = @(Get-ChildItem $stage -Filter *.dll)
$report.steps.staging = [ordered]@{
    dll_count = $dlls.Count
    dlls = @($dlls | ForEach-Object { $_.Name })
    has_assets = Test-Path (Join-Path $stage 'assets')
    has_lp_data = Test-Path (Join-Path $stage 'libpinyin\data')
}
if ($dlls.Count -lt 3 -or -not $report.steps.staging.has_assets -or -not $report.steps.staging.has_lp_data) { Fail 'J2 staging 不完整' }

# ---------------- J2b 依赖闭包（「解压即跑」的真正判据） ----------------
$missing = @()
$osProvided = @()
$sys32 = Join-Path $env:WINDIR 'System32'
foreach ($pe in (Get-ChildItem $stage -Include *.exe, *.dll -Recurse)) {
    foreach ($dll in (Get-ImportedDlls $pe.FullName)) {
        $local = Test-Path (Join-Path $stage $dll)
        if ($local) { continue }
        # 经验判据（比硬编码名单可靠）：MSYS2 那批运行库**不在** System32，
        # 而 Windows 自带 DLL（combase/pdh/powrprof/bcryptprimitives/uiautomationcore/api-ms-win-*…）
        # 都在 System32 ⇒ 「System32 里能解析到」= 目标机应当提供，不算打包漏项。
        # 两类"目标机提供"：① System32 里真有这个文件（combase/pdh/powrprof/…）；
        # ② `api-ms-win-*` / `ext-ms-*` 这类**虚拟 api-set**（由 apisetschema 解析，磁盘上没有同名文件）。
        if ((Test-Path (Join-Path $sys32 $dll)) -or ($dll -match $apiSetPattern)) {
            if ($osProvided -notcontains $dll) { $osProvided += $dll }
            continue
        }
        $missing += ('{0} -> {1}' -f $pe.Name, $dll)
    }
}
$apiSets = @()
foreach ($pe in (Get-ChildItem $stage -Include *.exe, *.dll -Recurse)) {
    foreach ($dll in (Get-ImportedDlls $pe.FullName)) {
        if ($dll -match $apiSetPattern -and $apiSets -notcontains $dll) { $apiSets += $dll }
    }
}
$report.steps.closure = [ordered]@{
    missing = $missing; ok = ($missing.Count -eq 0)
    os_provided = $osProvided
    os_provided_count = $osProvided.Count
    windows_api_sets_required = $apiSets
    note = 'System32 可解析的依赖（含 api-ms-win-*/combase/pdh/powrprof/uiautomationcore 等）由 Windows 提供，CI 配方本就不随包分发；其余一律要求已打进 stage'
}
if ($missing.Count -gt 0) { Fail ("J2b 依赖闭包不成立（解压后必然启动失败）：`n  " + ($missing -join "`n  ")) }

# ---------------- J3 打包 + 结构断言 ----------------
$zip = Join-Path $WorkDir 'client_bevy-windows-x86_64.zip'
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -Force
Add-Type -AssemblyName System.IO.Compression.FileSystem
$entries = [System.IO.Compression.ZipFile]::OpenRead($zip).Entries | ForEach-Object { $_.FullName }
$need = @('client_bevy.exe')
$missEntry = @($need | Where-Object { $entries -notcontains $_ })
$hasAssets = [bool]($entries | Where-Object { $_ -like 'assets/*' })
$hasLp = [bool]($entries | Where-Object { $_ -like 'libpinyin/data/*' })
$hasPinyinDll = [bool]($entries | Where-Object { $_ -like '*libpinyin*.dll' })
$report.steps.package = [ordered]@{
    zip = $zip; zip_bytes = (Get-Item $zip).Length; entry_count = $entries.Count
    missing_required = $missEntry; has_assets = $hasAssets; has_libpinyin_data = $hasLp; has_libpinyin_dll = $hasPinyinDll
}
# 注意：**不要求包里有 libpinyin DLL**——本端把 libpinyin 以 `libpinyin.a` **静态链接**进 exe
# （objdump 的导入表里没有 pinyin*，只有 glib/libdb/libiconv… 那批），随包带的是它的 data/conf。
# （DELIVERY.md 原文把 libpinyin 与 glib/libdb 并列写成"DLL"，属措辞不准，已在文档里更正。）
if ($missEntry.Count -gt 0 -or -not $hasAssets -or -not $hasLp) { Fail 'J3 结构断言不通过' }

# ---------------- J4 解压到干净目录后真启动 ----------------
if (Test-Path $unzip) { Remove-Item $unzip -Recurse -Force }
New-Item -ItemType Directory -Force -Path $unzip | Out-Null
Expand-Archive -Path $zip -DestinationPath $unzip -Force
if (-not $DataDir) { $DataDir = Join-Path $RepoRoot 'Data' }
if (Test-Path $DataDir) {
    # 「解压即跑」：资源目录按文档放在 exe 同目录（这里用 junction 指向仓库 Data，避免再拷 7GB）
    New-Item -ItemType Junction -Path (Join-Path $unzip 'Data') -Target $DataDir | Out-Null
}
$log = Join-Path $WorkDir 'client.out.log'
$err = Join-Path $WorkDir 'client.err.log'
$args = @('--control-port', "$ControlPort")
if ($SmokeServer) {
    $args += @('--real-net', '--auto-enter', '--e2e-user', $E2eUser, '--e2e-pass', $E2ePass)
    # 客户端没有"服务器地址"命令行/环境变量：地址来自 Mir2Config.ini 的 server_addr，
    # 缺省即 127.0.0.1:7000（`network/mod.rs:55`）。所以冒烟只支持默认地址；
    # 要用别的地址请在解压目录放一份 Mir2Config.ini。
    if ($SmokeServer -ne '127.0.0.1:7000') {
        Write-Warning "SmokeServer=$SmokeServer 不是客户端默认地址（127.0.0.1:7000）；需在解压目录提供 Mir2Config.ini 才会生效"
    }
}
$env:PATH = "$ucrtBin;D:\toolchains\libpinyin-install\bin;$env:PATH"
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
# 起客户端并登录 e2e 账号 → 必须先拿实机锁（见 tools/acceptance/e2e_lock.ps1 的约定）
$locked = $false
if ($SmokeServer) {
    . (Join-Path $RepoRoot 'tools\acceptance\e2e_lock.ps1')
    if (-not (Enter-E2eLock -ScriptName 'package_windows_rehearsal' -TimeoutSec 600)) { Fail 'J4 拿不到实机锁（别的实机验收在跑）' }
    $locked = $true
}
try {
$proc = Start-Process -FilePath (Join-Path $unzip 'client_bevy.exe') -ArgumentList $args `
    -WorkingDirectory $unzip -RedirectStandardOutput $log -RedirectStandardError $err -PassThru -WindowStyle Hidden
Start-Sleep 12
$proc.Refresh()
$alive = -not $proc.HasExited
$rpc = $null
if ($alive) {
    try {
        $tcp = New-Object System.Net.Sockets.TcpClient('127.0.0.1', $ControlPort)
        $tcp.Close(); $rpc = $true
    } catch { $rpc = $false }
}
$entered = $false
if ($alive -and $rpc -and $SmokeServer) {
    # control 口是 **JSON-RPC over TCP（逐行）**，不是 HTTP（见 tools/acceptance/rpc.ps1）——
    # 用 HTTP 探测会永远拿不到 state（本轮踩过：客户端其实已进图、world 包都收到了，
    # 却因为探测协议错而记 entered_game=false）。
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep 1
        try {
            $c = New-Object Net.Sockets.TcpClient
            $c.Connect('127.0.0.1', $ControlPort)
            $s = $c.GetStream()
            $req = "{`"jsonrpc`":`"2.0`",`"id`":1,`"method`":`"state`",`"params`":{}}`n"
            $b = [Text.Encoding]::UTF8.GetBytes($req)
            $s.Write($b, 0, $b.Length); $s.Flush()
            $r = New-Object IO.StreamReader($s)
            $line = $r.ReadLine()
            $c.Close()
            if ($line) {
                $obj = $line | ConvertFrom-Json
                if ($null -ne $obj.result.tile_x) { $entered = $true; break }
            }
        } catch {}
    }
}
if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
} finally {
    if ($locked) { Exit-E2eLock }
}
$errTail = @(Get-Content $err -EA SilentlyContinue | Select-Object -Last 6)
$report.steps.extract_run = [ordered]@{
    exe = (Join-Path $unzip 'client_bevy.exe'); alive = $alive; control_rpc = $rpc
    entered_game = $entered; smoke_server = $SmokeServer; err_tail = $errTail
}
if (-not $alive -or -not $rpc) { Fail "J4 解压后未能启动（alive=$alive rpc=$rpc）；日志尾部见 err_tail" }

$report.ok = $true
$json = $report | ConvertTo-Json -Depth 8
if ($OutFile) { New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutFile) | Out-Null; Set-Content -Path $OutFile -Value $json -Encoding utf8 }
Write-Host $json
Write-Host 'PASS package_windows_rehearsal：J1–J4 全过（产物/staging/依赖闭包/zip 结构/解压即跑）'
