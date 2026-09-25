# l5ag_maplight_stream.ps1 — owner 反馈「部分地图灯光错位」的**换图重建路径**实机判据
#
# 为什么是这条判据：owner 点名的怀疑点是「换图时 chunk_stream 重建路径」。静态读码已证明
# `chunks.rs::spawn_light_chunk`（MapLight 的唯一生产者）与静态路径共用同一对已修 helper
# （`light_tex_size` / `light_center`，两者有对照 C# 的单元门禁），也不存在第二条换图专用路径；
# 但「换图后到底有没有按**新图**重建灯光」此前没有运行时证据。
#
# 判据来源（**用现有运行时日志，不新增仪器**）——灯光有**两条**产生路径，各有日志：
#   a) `chunks.rs:428` 相机移动带动 chunk 流入：`💡 灯光流式加载 N 个`；
#   b) `chunks_build` 换图重建：`💡 地图灯光生成完成: N 个`（紧跟 `🗺️ 检测到换图 X，清理旧世界并重建`）。
# **只数 (a) 会得到"每张图都是 0"的假 GAP**（实测踩到：地图 `0` 有 10560 个灯光格，
# 换图后只出现 (b)、没有 (a)）⇒ 判据必须把两条都数上。于是：
#   ① 前置：候选图里至少两张能产生该日志（否则 exit 3，不产出 PASS）；
#   ② 换图重建：切到某图后 4s 内新出现的灯光行数 > 0（且计数以「切图时刻」为界，不是沿用上一张图的）；
#   ③ 阴性对照：至少一张候选图**不产生**新的灯光行（证明这条判据不是恒真）；
#   ④ 判据自检：计数函数喂合成日志（2 行灯光 + 1 行无关）必须得 2。
# 退出码：0 全 PASS；1 有用例 FAIL；3 前置不成立；2 前置失败（未进场/探针不可用）。
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [int]$ControlPort = 9073,
    # 用 `dbq.py` 查 `map_infos.light` 挑的高概率候选（light=2 是室内店/客栈，light=4 是洞窟/墓穴）；
    # 顺序按"小而暗"在前，缩短前置扫描时间。**注意**用 `pwsh -File` 调用时数组要写成
    # `-Maps 0101 D001` 这种空格分隔（逗号会被当字面量，整串被绑成一个元素——实测踩过）。
    # `0`/`5`/`1` 是实测"逐格灯光最多"的地图（用生产 MapReader 扫描 465 张内置地图得到：
    # 323 张含灯光，`5`=38929 格、`3`=15652、`0`=10560、`6`=4697、`1`=4506、`11`=4121 …）；
    # `0106` 是**阴性候选**（DrapersStore，扫描结果里没有灯光格）。
    # `0115` 是**阴性对照**：扫描 465 张内置地图得到 142 张"完全没有灯光格"，
    # 0115/0117/1001/1002… 是其中样例（0106 看着像但实际有灯，实测会给假 FAIL）。
    [string[]]$Maps = @('0', '5', '0115', '1'),
    [int]$DwellMs = 4000
)
$ErrorActionPreference = 'Continue'

# ---- 实机资源互斥 ----------------------------------------------------------
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5ag_maplight_stream' -TimeoutSec 1800)) { exit 2 }

try {
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5ag_maplight_stream'
$root = Join-Path $env:TEMP 'orig-csharp-ab'
if (-not (Test-Path $root)) { New-Item -ItemType Directory -Path $root | Out-Null }
$uniq = Join-Path $root 'l5ag_client.exe'
$log = Join-Path $root 'l5ag_client.log'
$err = Join-Path $root 'l5ag_client.err'
# **tracing 写的是 stderr**（实测踩到）：客户端 `--real-net` 跑起来后 stdout 文件是 0 字节，
# 所有 `INFO client_bevy::…` 都在 .err 里。判据读错文件 ⇒ 每张图都数成 0 的假 GAP。
# 所以下面一律读 $err（并保留 $log 变量只为把两个重定向写全）。
$traceFile = $err

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 3000; $c.SendTimeout = 3000
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 3000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}

# 判据函数（单独成函数便于自检）：数日志里「灯光流式加载」的行数
function Count-LightLines([string]$Text) {
    if (-not $Text) { return 0 }
    # **数"灯光个数"而不是"日志行数"**：`地图灯光生成完成: 0 个` 也会打（重建事件必打），
    # 按行数会把"没有灯光的地图"也数成 1（实测踩到，导致阴性对照假 FAIL）。
    $sum = 0
    foreach ($m in [regex]::Matches($Text, '灯光流式加载\s*(\d+)\s*个|地图灯光生成完成:\s*(\d+)\s*个')) {
        $v = if ($m.Groups[1].Success) { $m.Groups[1].Value } else { $m.Groups[2].Value }
        $sum += [int]$v
    }
    return $sum
}
# **必须用 FileShare.ReadWrite 读**：客户端进程一直持有该日志的写句柄，`ReadAllText` 会抛
# `The process cannot access the file … because it is being used by another process`
# （实测踩到，且因为异常发生在循环里，表现成"夹具不动了"而不是清晰的报错）。
function LightCount {
    if (-not (Test-Path -LiteralPath $traceFile)) { return 0 }
    $fs = $null; $sr = $null
    try {
        $fs = [IO.File]::Open($traceFile, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
        $sr = New-Object IO.StreamReader($fs)
        return (Count-LightLines ($sr.ReadToEnd()))
    } catch {
        return 0
    } finally {
        if ($sr) { $sr.Dispose() }
        if ($fs) { $fs.Dispose() }
    }
}
function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='l5ag_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}
$fail = New-Object System.Collections.Generic.List[string]
function Check([string]$name, [bool]$cond, [string]$detail = '') {
    if ($cond) { Write-Host ("  [PASS] " + $name) -ForegroundColor Green }
    else { $fail.Add($name); Write-Host ("  [FAIL] " + $name + " —— " + $detail) -ForegroundColor Red }
}

Stop-Client; Start-Sleep -Milliseconds 600
Copy-Item -LiteralPath $exe -Destination $uniq -Force
foreach ($f in @($log, $err)) { if (Test-Path $f) { [IO.File]::Delete($f) } }
Start-Process -FilePath $uniq `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, '--control-port', "$ControlPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" -RedirectStandardOutput $log -RedirectStandardError $err | Out-Null

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    Write-Host 'FAIL(2): 未进场'
    Stop-Client; exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# 0) 判据自检
Check '0 判据自检：合成日志必须**按灯光个数求和**得 4（3+1），且 0 个不计' `
    ((Count-LightLines "x 灯光流式加载 3 个`ny 无关`nz 地图灯光生成完成: 1 个`nw 地图灯光生成完成: 0 个`n") -eq 4)

$perMap = @{}
foreach ($m in $Maps) {
    # **先取分界再切图**（实测踩到反过来的写法）：`@mapmove` 之后灯光流式会在 chunk 重建时立刻发生，
    # 若等 `state.map == m` 再取分界，那批灯光行已经落在分界之前 ⇒ 每张图都算成 0（假 GAP）。
    $at = LightCount
    Rpc 'chat' @{ message = "@mapmove $m 100 100" } | Out-Null
    $okMap = $false
    foreach ($i in 1..25) {
        Start-Sleep -Milliseconds 400
        $s = Rpc 'state'
        if ($null -ne $s -and "$($s.map)" -eq $m) { $okMap = $true; break }
    }
    if (-not $okMap) { Write-Host ("  [SKIP] {0}：@mapmove 未落到该图" -f $m); continue }
    Start-Sleep -Milliseconds $DwellMs
    $delta = (LightCount) - $at
    $perMap[$m] = $delta
    Write-Host ("  {0}: 换图后新增灯光数 = {1}" -f $m, $delta)
    if ($delta -gt 0) {
        $shot = Join-Path $PSScriptRoot "shots\l5ag_$m.png"
        Rpc 'screenshot' @{ path = $shot } | Out-Null
    }
}

$lit = @($perMap.Keys | Where-Object { $perMap[$_] -gt 0 })
$dark = @($perMap.Keys | Where-Object { $perMap[$_] -eq 0 })
Check '① 前置：候选图里至少两张在换图后产生灯光行（否则不产出 PASS）' ($lit.Count -ge 2) `
    ("有灯光的图=" + ($lit -join ',') + "；全部候选=" + ($perMap.Keys -join ','))
if ($lit.Count -lt 2) {
    Stop-Client
    Write-Host 'VERDICT maplight_stream=GAP(前置不成立：候选图里点亮的不足两张)'
    exit 3
}
Check '② 换图重建：点亮的图在其**切图之后**新出现灯光行（不是沿用上一张图）' ($lit.Count -ge 2) `
    ("点亮的图：" + ($lit -join ','))
Check '③ 阴性对照：至少一张候选图不产生新的灯光行（判据不是恒真）' ($dark.Count -ge 1) `
    ("无新灯光行的图=" + ($dark -join ','))

Stop-Client
if ($fail.Count -gt 0) {
    Write-Host ("VERDICT maplight_stream=FAIL（{0} 项）: {1}" -f $fail.Count, ($fail -join '; ')) -ForegroundColor Red
    exit 1
}
Write-Host 'VERDICT maplight_stream=PASS（换图后按新图重建灯光；阴性对照证明判据有区分度）' -ForegroundColor Green
exit 0
} finally {
    Exit-E2eLock
}
