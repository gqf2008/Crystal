#Requires -Version 5.1
<#
.SYNOPSIS
  客户端 auto 开关（`--xxx-test`）覆盖清单的对账门禁 —— 防「开关加了/删了，覆盖清单不知道」。

.DESCRIPTION
  `tools/acceptance/CLIENT_AUTO_FLAGS.md` 是「哪些开关进发版门禁、哪些只是历史探针」的单一真源。
  这份清单最容易的失效方式是**静默漂移**：有人加了新开关（或删了旧开关），清单没跟。
  本门禁把三件事钉住：
    ① 清单表格里的开关集合 == `Client-Bevy/src/auto/mod.rs` 里声明的开关集合（多/少都红）；
    ② 桶标记为 `gate` 的开关，必须确实出现在 `scripts/run_real_e2e.ps1`（发版门禁）里；
    ③ 桶标记为 `fixture` 的开关，其「谁在跑」列出的 runner 文件必须存在且真的引用了它。
  纯文本分析，不起客户端、不连服务端、秒级。

.PARAMETER ClientSource  客户端开关声明文件；默认 <RepoRoot>\Client-Bevy\src\auto\mod.rs
.PARAMETER DocPath       覆盖清单；默认 <本脚本同目录>\CLIENT_AUTO_FLAGS.md
.PARAMETER GateScript    发版门禁脚本；默认 <RepoRoot>\scripts\run_real_e2e.ps1

.NOTES
  退出码：0 全过 / 1 有 FAIL / 2 前置失败（文件缺失）。
  阳性对照（落地时实做）：① 往源码副本里插一个假开关 → ①红；② 把文档里某个 probe 桶改成 gate → ②红。
  用法：pwsh tools/acceptance/flag_coverage_check.ps1
        pwsh tools/acceptance/flag_coverage_check.ps1 -ClientSource <源码副本>   # A/B 对照
#>
param(
    [string]$RepoRoot = '',
    [string]$ClientSource = '',
    [string]$DocPath = '',
    [string]$GateScript = ''
)
$ErrorActionPreference = 'Continue'
if (-not $RepoRoot) { $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path }
if (-not $ClientSource) { $ClientSource = Join-Path $RepoRoot 'Client-Bevy\src\auto\mod.rs' }
if (-not $DocPath) { $DocPath = Join-Path $PSScriptRoot 'CLIENT_AUTO_FLAGS.md' }
if (-not $GateScript) { $GateScript = Join-Path $RepoRoot 'scripts\run_real_e2e.ps1' }

foreach ($p in @($ClientSource, $DocPath, $GateScript)) {
    if (-not (Test-Path -LiteralPath $p)) {
        Write-Host ("前置失败：找不到 {0}" -f $p) -ForegroundColor Red
        exit 2
    }
}

$pass = 0
$fail = 0
function Check([string]$name, [bool]$cond, [string]$detail = '') {
    if ($cond) {
        Write-Host ("  [PASS] {0}" -f $name) -ForegroundColor Green
        $script:pass++
    } else {
        $msg = "  [FAIL] $name"
        if ($detail) { $msg += " —— $detail" }
        Write-Host $msg -ForegroundColor Red
        $script:fail++
    }
}

# ---- ① 源码声明的开关集合 ----
$src = Get-Content -LiteralPath $ClientSource -Raw
$srcFlags = @([regex]::Matches($src, '"(--[a-z0-9-]+-test)"') | ForEach-Object { $_.Groups[1].Value } |
    Sort-Object -Unique)

# ---- ② 清单表格里的开关 + 桶 + 谁在跑 ----
$docLines = Get-Content -LiteralPath $DocPath
$rows = @()
foreach ($line in $docLines) {
    if ($line -match '^\|\s*`(--[a-z0-9-]+-test)`\s*\|\s*(gate|fixture|probe)\s*\|\s*(.*?)\s*\|\s*(.*?)\s*\|\s*$') {
        $rows += [pscustomobject]@{
            Flag   = $Matches[1]
            Bucket = $Matches[2]
            Who    = $Matches[3]
            System = $Matches[4]
        }
    }
}
$docFlags = @($rows | ForEach-Object { $_.Flag } | Sort-Object -Unique)

Check 'C1 清单表格解析出开关（没解析到多半是表格格式被改坏）' ($docFlags.Count -ge 50) ("doc_flags=" + $docFlags.Count)
Check 'C2 源码解析出开关（判据非空）' ($srcFlags.Count -ge 50) ("src_flags=" + $srcFlags.Count)

$missing = @($srcFlags | Where-Object { $docFlags -notcontains $_ })
$extra = @($docFlags | Where-Object { $srcFlags -notcontains $_ })
Check 'C3 清单覆盖源码全部开关（源码加了新开关必须同步清单）' `
    ($missing.Count -eq 0) ("清单缺：" + ($missing -join ', '))
Check 'C4 清单没有多余开关（源码删了开关必须同步清单）' `
    ($extra.Count -eq 0) ("清单多出：" + ($extra -join ', '))

# ---- ③ gate 桶必须真的在发版门禁里跑 ----
$gateText = Get-Content -LiteralPath $GateScript -Raw
$gateRows = @($rows | Where-Object { $_.Bucket -eq 'gate' })
$notInGate = @($gateRows | Where-Object { $gateText -notmatch [regex]::Escape($_.Flag) } | ForEach-Object { $_.Flag })
Check 'C5 标记 gate 的开关确实出现在发版门禁脚本里' `
    ($notInGate.Count -eq 0) ("gate 桶但门禁脚本没有：" + ($notInGate -join ', '))
Check 'C6 gate 桶非空（否则等于把门禁覆盖写没了）' ($gateRows.Count -ge 1) ("gate=" + $gateRows.Count)

# ---- ④ fixture 桶的 runner 必须存在且真的引用它 ----
$badFixture = @()
foreach ($r in @($rows | Where-Object { $_.Bucket -eq 'fixture' })) {
    $refs = @([regex]::Matches($r.Who, '`([^`]+)`') | ForEach-Object { $_.Groups[1].Value } |
        Where-Object { $_ -match '\.(ps1|py)$' })
    if ($refs.Count -eq 0) { $badFixture += "$($r.Flag)（未列出 runner 文件）"; continue }
    foreach ($rel in $refs) {
        $full = Join-Path $RepoRoot ($rel -replace '/', '\')
        if (-not (Test-Path -LiteralPath $full)) { $badFixture += "$($r.Flag)（$rel 不存在）"; continue }
        if ((Get-Content -LiteralPath $full -Raw) -notmatch [regex]::Escape($r.Flag)) {
            $badFixture += "$($r.Flag)（$rel 未引用它）"
        }
    }
}
Check 'C7 fixture 桶列出的 runner 存在且确实引用该开关' `
    ($badFixture.Count -eq 0) ($badFixture -join '；')

Write-Host ("结果：{0} passed / {1} failed（开关 {2} 个：gate {3} / fixture {4} / probe {5}）" -f `
        $pass, $fail, $docFlags.Count, $gateRows.Count,
        @($rows | Where-Object { $_.Bucket -eq 'fixture' }).Count,
        @($rows | Where-Object { $_.Bucket -eq 'probe' }).Count)
if ($fail -gt 0) { exit 1 }
exit 0
