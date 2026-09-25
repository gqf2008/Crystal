#Requires -Version 5.1
# check_doc_tool_refs.ps1 —— 静态门禁：**文档里引用的 tools/** 脚本必须存在、且必须在 git 里**
#
# 背景（2026-09-26 实测两次同类事故）：
#   本仓 `.gitignore` 对 `tools/acceptance/`、`tools/ops/` 是**白名单式**管理（先 `/*` 全禁、
#   再逐条 `!` 放行）⇒ **新脚本默认被忽略且不报错**。后果是"作者以为提交了"的脚本
#   只存在于那台机器上：
#     ① `walgit_entry.py`（文档/经验都在引它，仓库里查无此文件）；
#     ② 逐窗截图矩阵那套 6 个文件（`ui_shot_matrix.py`/`contact_sheet.py`/`crop_kind.py`/
#        `font_audit.py`/`rpc.py`/`capture.ps1`）被 `UI_VERIFICATION_REPORT.md` §11.5 当方法写进文档；
#     ③ 本门禁上线时又抓到 8 个（`player_walk*.ps1`/`ui_sweep.ps1`/`ui_bugfix_verify.ps1`/
#        `npc_text_verify.ps1`/`ime_rpc_verify.ps1`/`seed_db.py`/`smoke_r5.ps1`），
#        都被 `PLAYER_TEST_REPORT.md` / `UI_VERIFICATION_REPORT.md` / `SMOKE_R5_REPORT.md` 引用。
#   ⇒ 判据：**文档引用的可执行脚本**必须 (a) 文件存在、(b) `git ls-files` 认得。
#      （只查"可执行脚本"扩展名：`tools/ops/out/*.json` 这类**产物路径**是演练输出，不入此判据。）
#
# 用法：
#   pwsh tools/ops/check_doc_tool_refs.ps1              # 0 全过 / 1 有缺失或未跟踪 / 2 前置失败或自检失败
#   pwsh tools/ops/check_doc_tool_refs.ps1 -Strict      # 连 allowlist 里的一起报红
#   pwsh tools/ops/check_doc_tool_refs.ps1 -SkipSelfTest
param(
    [string]$RepoRoot = '',
    # 待迁移清单：**当前为空**（本轮把 8 个引用但未跟踪的脚本一并入库了）。
    # 保留参数是为了将来确实需要临时豁免时有地方记名字（`-Strict` 会连它们一起报红）。
    [hashtable]$Allowlist = @{},
    [switch]$Strict,
    [switch]$SkipSelfTest
)
$ErrorActionPreference = 'Continue'

function Get-ReferencedToolScripts {
    <#
      扫描 root 下所有 *.md，抽出形如 `tools/acceptance/xxx.ps1` 的引用。
      只认**可执行脚本**扩展名（.ps1/.py/.sh/.bat/.cmd）——`tools/ops/out/*.json` 是演练产物路径，
      文档里出现它们是正常的（"报告写到那里"），不该要求入库。
      返回对象数组：Path（仓库相对、正斜杠）/ Doc（引用它的 md）/ Full。
    #>
    param([Parameter(Mandatory)][string]$Root)
    $skipDirs = '\\(\.git|target|node_modules)\\'
    $extPat = 'tools[/\\][A-Za-z0-9_./\\-]+\.(ps1|py|sh|bat|cmd)'
    $out = @()
    $docs = @(Get-ChildItem -LiteralPath $Root -Recurse -File -Filter *.md -EA SilentlyContinue |
        Where-Object { $_.FullName -notmatch $skipDirs })
    foreach ($d in $docs) {
        $text = Get-Content -LiteralPath $d.FullName -Raw -EA SilentlyContinue
        if ($null -eq $text) { continue }
        foreach ($m in [regex]::Matches($text, $extPat)) {
            $rel = ($m.Value -replace '\\', '/').TrimEnd('.', ',', ')', '`', '"')
            $out += [pscustomobject]@{
                Path = $rel
                Full = (Join-Path $Root ($rel -replace '/', '\'))
                Doc  = $d.Name
            }
        }
    }
    $out
}

function Test-Tracked {
    param([Parameter(Mandatory)][string]$Root, [Parameter(Mandatory)][string]$Rel)
    $prev = Get-Location
    try {
        Push-Location $Root
        $r = & git ls-files --error-unmatch -- $Rel 2>$null
        return ($LASTEXITCODE -eq 0 -and $r)
    } finally {
        Pop-Location
    }
}

function Get-DocToolRefOffenders {
    param([Parameter(Mandatory)][string]$Root)
    $off = @()
    foreach ($ref in @(Get-ReferencedToolScripts -Root $Root)) {
        if (-not (Test-Path -LiteralPath $ref.Full)) {
            $off += [pscustomobject]@{ Path = $ref.Path; Doc = $ref.Doc; Kind = 'missing' }
            continue
        }
        if (-not (Test-Tracked -Root $Root -Rel $ref.Path)) {
            $off += [pscustomobject]@{ Path = $ref.Path; Doc = $ref.Doc; Kind = 'untracked' }
        }
    }
    # 同一路径被多份文档引用时只报一次
    $off | Sort-Object Path -Unique
}

if (-not $RepoRoot) { $RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not (Test-Path -LiteralPath $RepoRoot)) { Write-Host "FAIL(前置)：仓库根不存在：$RepoRoot"; exit 2 }

# ---------------- 沙箱自检：判据不许空转 ----------------
if (-not $SkipSelfTest) {
    $sb = Join-Path ([System.IO.Path]::GetTempPath()) ("crystal_doctool_" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Path (Join-Path $sb 'tools\acceptance') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $sb 'docs') -Force | Out-Null
    $enc = New-Object System.Text.UTF8Encoding($false)
    try {
        & git -C $sb init -q 2>&1 | Out-Null
        # ① 负对照：被引用的脚本**已入库** → 不许报
        [IO.File]::WriteAllText((Join-Path $sb 'tools\acceptance\tracked_ok.ps1'), "Write-Host 'ok'`n", $enc)
        [IO.File]::WriteAllText((Join-Path $sb 'docs\a.md'), "跑 pwsh tools/acceptance/tracked_ok.ps1`n", $enc)
        & git -C $sb add tools/acceptance/tracked_ok.ps1 docs/a.md 2>&1 | Out-Null
        # ② 正对照：被引用但**未入库**（真实事故形态）→ 必须报 untracked
        [IO.File]::WriteAllText((Join-Path $sb 'tools\acceptance\untracked_bad.ps1'), "Write-Host 'x'`n", $enc)
        [IO.File]::WriteAllText((Join-Path $sb 'docs\b.md'), "跑 pwsh tools/acceptance/untracked_bad.ps1`n", $enc)
        # ③ 正对照：被引用但**文件不存在** → 必须报 missing
        [IO.File]::WriteAllText((Join-Path $sb 'docs\c.md'), "跑 pwsh tools/acceptance/ghost.ps1`n", $enc)
        # ④ 负对照：产物路径（tools/ops/out/*.json）不该被当成脚本判据
        [IO.File]::WriteAllText((Join-Path $sb 'docs\d.md'), "报告写到 tools/ops/out/report.json`n", $enc)

        $off = @(Get-DocToolRefOffenders -Root $sb)
        $problems = @()
        if (@($off | Where-Object { $_.Path -eq 'tools/acceptance/tracked_ok.ps1' }).Count -ne 0) {
            $problems += '负对照1（已入库脚本）被误报'
        }
        if (@($off | Where-Object { $_.Path -eq 'tools/acceptance/untracked_bad.ps1' -and $_.Kind -eq 'untracked' }).Count -eq 0) {
            $problems += '正对照2（引用未入库脚本）没被抓到 —— 判据空了'
        }
        if (@($off | Where-Object { $_.Path -eq 'tools/acceptance/ghost.ps1' -and $_.Kind -eq 'missing' }).Count -eq 0) {
            $problems += '正对照3（引用不存在的脚本）没被抓到 —— 判据空了'
        }
        if (@($off | Where-Object { $_.Path -like '*out/report.json' }).Count -ne 0) {
            $problems += '负对照4（产物 json 路径）被误判成脚本引用'
        }
        if ($problems.Count -gt 0) {
            foreach ($p in $problems) { Write-Host ("  [自检红] " + $p) -ForegroundColor Red }
            Write-Host "FAIL(自检)：本门禁判据不可信（沙箱 $sb）"
            exit 2
        }
        Write-Host '自检：1 负对照（已入库不报）+ 2 正对照（未入库/不存在必须报）+ 1 负对照（产物 json 不报）✅'
    } finally {
        Remove-Item -LiteralPath $sb -Recurse -Force -EA SilentlyContinue
    }
} else {
    Write-Host '（-SkipSelfTest：本次没跑沙箱对照）' -ForegroundColor DarkYellow
}

# ---------------- 主扫描 ----------------
$refs = @(Get-ReferencedToolScripts -Root $RepoRoot)
if ($refs.Count -eq 0) {
    Write-Host 'FAIL(前置)：一份 md 里都没扫到 tools/** 脚本引用 —— 判据没跑起来，不许当绿'
    exit 2
}
$offenders = @(Get-DocToolRefOffenders -Root $RepoRoot)
$new = @($offenders | Where-Object { -not $Allowlist.ContainsKey($_.Path) })
$known = @($offenders | Where-Object { $Allowlist.ContainsKey($_.Path) })

Write-Host ("扫描 {0} 条脚本引用（来自 {1} 份 md）：缺失/未入库 {2} 条（待迁移 allowlist {3} 条、新增 {4} 条）" -f `
    $refs.Count, (@($refs | Select-Object -ExpandProperty Doc -Unique).Count), $offenders.Count, $known.Count, $new.Count)
foreach ($o in $known) { Write-Host ("  [待迁移] {0} —— 由 {1} 引用：{2}" -f $o.Path, $o.Doc, $Allowlist[$o.Path]) -ForegroundColor DarkYellow }
foreach ($o in $new) {
    $why = if ($o.Kind -eq 'missing') { '文档引用了**不存在**的文件' } else { '文件存在但**没进 git**（白名单式 .gitignore 静默吞掉？）' }
    Write-Host ("  [违规]   {0}（{1}）—— 由 {2} 引用" -f $o.Path, $o.Kind, $o.Doc) -ForegroundColor Red
    Write-Host ("            {0}；修法：补 `!{1}` 白名单 + `git add -f {1}`，或删掉文档里的引用" -f $why, $o.Path) -ForegroundColor DarkGray
}
if ($new.Count -gt 0) { exit 1 }
if ($Strict -and $known.Count -gt 0) {
    Write-Host '  -Strict：待迁移清单也必须清空' -ForegroundColor Red
    exit 1
}
Write-Host '结果：文档引用的 tools 脚本全部存在且已入库 ✅'
exit 0
