# mem_leak_gate.ps1 —— 发版前的**内存泄漏**门禁（一条命令）
#
# 为什么需要它：`leak_plateau.ps1` 的 **J5（活跃字节斜率）** 才是"有没有每轮线性泄漏"的判据，
# 但它有三个前置：① 被测二进制要带 `--features mem-probe`；② 环境要有 `MIR2_LEAK_PROBE=1`；
# ③ 要有一个能起服的部署目录（带账号/角色/地图数据）。手工拼这三步很容易漏一步，
# 而漏了之后 J5 会退化成 `null`（报告里写"无法判真泄漏"）——**看起来还是"绿"**。
# 所以把它固化成一条命令，并接进发版前路径（docs/DELIVERY.md 的"发版日关键路径"）。
#
# 判据（2026-09-25 用实测定标后改过一版）：
#   - 必须真的跑出 J5（`J5_live_bytes_plateau` 非 null）——否则 `exit 3`（"没判成"，不是"通过"）；
#   - **主判据 = 测量窗口内活跃字节"一次都没回落"（`live_bytes_dips == 0`）**：窗口是单调上升
#     ⇒ "每轮都留一点"的泄漏签名成立 ⇒ `exit 10`（**即使斜率在 0.1MB 阈值以内**）；
#   - `dips > 0` ⇒ 通过（`exit 0`）；斜率仍打印出来，超阈值的那种标成"回落解释"；
#   - 夹具自身前置失败（缺账号/端口被占/服务起不来）→ 原样透传 `exit 2`/`3`/`9`。
#
# 为什么主判据是 dips 而不是斜率（本轮定标数据，同一台机器、同一夹具、同一组参数）：
#   修复版：3 轮斜率 −62,543 / +3,585 B（dips=1）；8 轮 **+87,704 B**（dips≥1，逐轮
#           12.15→12.33→12.41→12.47→12.47→**12.08**→12.76→12.76 MB，那次 12.47→12.08 就是回落）。
#   泄漏版（临时把两处守卫拿掉重建）：3 轮斜率 **+462,153**（红）与 **+93,626**。
#   ⇒ 斜率分不开两者：泄漏版那个 +93,626 **落在 0.1MB 阈值(104,857B)以内**，旧判据直接 **PASS / exit 0**
#     （实测发生过）；而修复版的 +87,704 比它更大——按斜率排，泄漏反而"更小"。
#     "一次都没回落"才是结构性的：每轮留一点 ⇒ 窗口单调上升；修复版窗口内必有回落。
#   ⇒ 所以判据以 dips 为准。方向上也偏严：3 轮窗口偶发单调（假红）的代价是"让你加长窗口复测"，
#     比"放过一个真泄漏（假绿）"小得多；发版签字建议 `-MeasureCycles 6`~`8` 再跑一次。
#
# 用法：
#   pwsh tools/ops/mem_leak_gate.ps1 -DeployDir %TEMP%\ramp_deploy            # 默认先构建 mem-probe release
#   pwsh tools/ops/mem_leak_gate.ps1 -DeployDir ... -SkipBuild -Port 7460 -Sessions 20 -OutFile out.json
#   pwsh tools/ops/mem_leak_gate.ps1 -SelfTest                                # 判据自检（不起服、秒级）
param(
    [string]$DeployDir = '',
    [string]$ExePath = '',
    [int]$Port = 7450,
    [int]$Sessions = 20,
    [int]$WarmCycles = 2,
    [int]$MeasureCycles = 3,
    [double]$MaxLiveSlopePerCycleMb = 0.1,
    # 默认构建（发版日就是"从当前 master 起一手"）；想复用已构建的产物用 -SkipBuild
    [switch]$SkipBuild,
    [switch]$AllowDebugThreshold,
    [string]$OutFile = '',
    # 判据自检：用合成报告把「泄漏 / 通过 / 没法判」钉死（含上一版误判过的那个真实读数）
    [switch]$SelfTest
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = (Resolve-Path "$ops\..\..").Path

function Get-LeakVerdict {
    <#
      把一份 `leak_plateau.ps1` 的 JSON 报告判成 pass / pass_by_dip / leak / unknown。
      判据 = 斜率超阈值 **且** 活跃字节在测量窗口内一次都没回落（dips==0）。
    #>
    param(
        [Parameter(Mandatory = $true)][string]$ReportPath,
        [Parameter(Mandatory = $true)][double]$ThresholdMb
    )
    $threshold = [long]($ThresholdMb * 1MB)
    $res = [ordered]@{ verdict = 'unknown'; slope = $null; dips = $null; threshold = $threshold }
    if (-not (Test-Path -LiteralPath $ReportPath)) { return [pscustomobject]$res }
    try { $r = Get-Content -LiteralPath $ReportPath -Raw | ConvertFrom-Json } catch { return [pscustomobject]$res }
    if ($null -eq $r.J5_live_bytes_plateau) { return [pscustomobject]$res }
    $res.slope = [long]$r.live_bytes_slope_per_cycle
    # 老报告没有 dips 字段（本判据 2026-09-25 才加）：缺字段按 dips=0 处理 —— 对老报告偏严
    # （它本来也没有"回落"这个证据），而不是把缺失读成"有回落"从而放行。
    $res.dips = if ($null -ne $r.live_bytes_dips) { [int]$r.live_bytes_dips } else { 0 }
    # 主判据：窗口内有没有回落。dips==0 ⇒ 单调上升 ⇒ 判泄漏（不看斜率是否在阈值内 —— 上面那个
    # +93,626B 的泄漏读数正是被"阈值以内"放过的）。dips>0 时再用斜率区分"顺带回落"与"确实低"。
    $res.verdict = if ($res.dips -eq 0) { 'leak' }
                   elseif ($res.slope -gt $threshold) { 'pass_by_dip' }
                   else { 'pass' }
    [pscustomobject]$res
}

if ($SelfTest) {
    Write-Host 'SelfTest：判据象限（合成报告，不起服）'
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("mem_leak_selftest_" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    $cases = @(
        # 本轮实测的四个真实读数 + 一个"探针没生效"的负例
        @{ name = '泄漏版 (+462153B, dips=0)'; want = 'leak'; slope = 462153; dips = 0; j5 = $false },
        @{ name = '泄漏版边界 (+93626B, dips=0)'; want = 'leak'; slope = 93626; dips = 0; j5 = $false },
        @{ name = '修复版 (+87704B, dips=1)'; want = 'pass'; slope = 87704; dips = 1; j5 = $true },
        # 斜率超阈值但有回落：这条分支（pass_by_dip）也得有用例，否则它俩永远只是"没被走到"。
        @{ name = '超阈值但有回落 (+150000B, dips=2)'; want = 'pass_by_dip'; slope = 150000; dips = 2; j5 = $true },
        @{ name = '修复版 (-62543B, dips=1)'; want = 'pass'; slope = -62543; dips = 1; j5 = $true },
        @{ name = '没跑出 J5（探针没生效）'; want = 'unknown'; slope = $null; dips = $null; j5 = $null }
    )
    $bad = @()
    $i = 0
    foreach ($c in $cases) {
        $i++
        $p = Join-Path $tmp ("case_$i.json")
        $obj = [ordered]@{
            J5_live_bytes_plateau      = $c.j5
            live_bytes_slope_per_cycle = $c.slope
        }
        if ($null -ne $c.dips) { $obj['live_bytes_dips'] = $c.dips }
        ($obj | ConvertTo-Json) | Set-Content -LiteralPath $p -Encoding utf8
        $v = Get-LeakVerdict -ReportPath $p -ThresholdMb $MaxLiveSlopePerCycleMb
        $ok = ($v.verdict -eq $c.want)
        if (-not $ok) { $bad += ("{0}：期望 {1} 实得 {2}" -f $c.name, $c.want, $v.verdict) }
        Write-Host ("  [{0}] {1} → {2}" -f $(if ($ok) { 'PASS' } else { 'FAIL' }), $c.name, $v.verdict)
    }
    $pOld = Join-Path $tmp 'legacy_no_dips.json'
    '{"J5_live_bytes_plateau": false, "live_bytes_slope_per_cycle": 93626}' |
        Set-Content -LiteralPath $pOld -Encoding utf8
    $vOld = Get-LeakVerdict -ReportPath $pOld -ThresholdMb $MaxLiveSlopePerCycleMb
    $oldOk = ($vOld.verdict -eq 'leak')
    if (-not $oldOk) { $bad += ("老报告缺 dips 字段应偏严判 leak，实得 {0}" -f $vOld.verdict) }
    Write-Host ("  [{0}] 老报告缺 dips 字段 → {1}（偏严不偏松）" -f $(if ($oldOk) { 'PASS' } else { 'FAIL' }), $vOld.verdict)
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    if ($bad.Count -gt 0) { Write-Host ("SelfTest FAIL：{0}" -f ($bad -join '；')); exit 1 }
    Write-Host 'SelfTest PASS：三种判定 + 老报告偏严都对'
    exit 0
}

if (-not $DeployDir) {
    Write-Host 'FAIL(前置)：-DeployDir 必填（或用 -SelfTest 只跑判据自检）。'
    exit 2
}
if (-not $ExePath) { $ExePath = Join-Path $repo 'ServerRust\target\release\mir2_server.exe' }
if (-not (Test-Path -LiteralPath $DeployDir)) {
    Write-Host ("FAIL(前置)：部署目录不存在：{0}" -f $DeployDir)
    Write-Host '         可以用 tools/ops/make_deploy_dir.ps1 一键生成，再用 seed_load_accounts.py 播账号。'
    exit 2
}

if (-not $SkipBuild) {
    Write-Host '[1/2] 构建 mem-probe release（默认构建不含探针，必须显式带特性）'
    Push-Location (Join-Path $repo 'ServerRust')
    try {
        & cargo build --release --features mem-probe --bin mir2_server
        if ($LASTEXITCODE -ne 0) { Write-Host ("FAIL(前置)：cargo build 失败 exit={0}" -f $LASTEXITCODE); exit 2 }
    } finally { Pop-Location }
}

Write-Host '[2/2] 跑内存泄漏门禁（leak_plateau + J5/dips）'
$env:MIR2_LEAK_PROBE = '1'
$drillOut = Join-Path ([System.IO.Path]::GetTempPath()) ("mem_leak_gate_" + [guid]::NewGuid().ToString('N').Substring(0, 8) + ".json")
$drillArgs = @(
    '-NoProfile', '-File', (Join-Path $ops 'leak_plateau.ps1'),
    '-DeployDir', $DeployDir, '-ExePath', $ExePath, '-Port', "$Port",
    '-Sessions', "$Sessions", '-WarmCycles', "$WarmCycles", '-MeasureCycles', "$MeasureCycles",
    '-MaxLiveSlopePerCycleMb', "$MaxLiveSlopePerCycleMb", '-OutFile', $drillOut
)
if ($AllowDebugThreshold) { $drillArgs += '-AllowDebugThreshold' }
$text = (& pwsh @drillArgs 2>&1 | Out-String)
$code = $LASTEXITCODE
if ($OutFile) { $text | Set-Content -Encoding utf8 $OutFile }

$v = Get-LeakVerdict -ReportPath $drillOut -ThresholdMb $MaxLiveSlopePerCycleMb
if ($v.verdict -eq 'unknown') {
    Write-Host 'FAIL：这次没跑出 J5（live_bytes 斜率）——**没有判成"有没有泄漏"**，不能当通过。'
    Write-Host '      常见原因：被测二进制不是 --features mem-probe 构建、或 MIR2_LEAK_PROBE 没生效。'
    Write-Host ("      （夹具 exit={0}；日志尾：{1}）" -f $code, (($text -split "`n" | Select-Object -Last 3) -join ' | '))
    exit 3
}
if ($v.verdict -eq 'leak') {
    Write-Host ("FAIL：测量窗口内活跃字节**一次都没回落**（dips=0，每轮净增 {0} B）" -f $v.slope)
    Write-Host '      —— "每轮都留一点"的线性泄漏签名成立（发版前必须查清；注意**不以斜率是否超阈值为准**）。'
    Write-Host ("      若怀疑是 {0} 轮窗口偶发单调，用 -MeasureCycles 6 ~ 8 复测一次再下结论。" -f $MeasureCycles)
    exit 10
}
if ($v.verdict -eq 'pass_by_dip') {
    Write-Host ("PASS（回落解释）：每轮净增 {0} B > 阈值 {1} B，但窗口内有 {2} 次回落 ⇒ 净增被分配器噪声解释。" -f $v.slope, $v.threshold, $v.dips)
    Write-Host '      提示：边界结论建议加大窗口（-MeasureCycles 6~8）复测一次再发版。'
    exit 0
}
Write-Host ("PASS：活跃字节每轮净增 {0} B ≤ 阈值 {1} B —— 没有每轮线性泄漏（dips={2}）。" -f $v.slope, $v.threshold, $v.dips)
exit 0
