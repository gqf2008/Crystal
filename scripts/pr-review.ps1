#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Crystal PR 实时跟踪评审脚本（Real-time PR tracking & review）。

.DESCRIPTION
    轮询 GitHub 仓库的开放 PR；对 head SHA 有更新（尚未评审）的 PR 自动：
      1. 在独立 worktree（主仓库同级，命名 Crystal-prreview-worktrees/pr-<n>）检出 PR head，
         不触碰主工作区与其他 agent 的 worktree。
      2. 按 AGENTS.md 要求执行验证：
         - 涉及 Client-Bevy   ：cargo check + cargo test --lib
                               （用 --lib 跑 47 个单测，避免依赖外部 Data 目录的集成测试 ui_alignment）
         - 涉及 ServerRust    ：cargo test
         - 仅涉及 SharedRust  ：cargo test --lib（服务端依赖它，改动会随 ServerRust 一起验证）
      3. 通过 gh pr review 把结构化评审意见发回 PR：
         - 任一验证失败 / PR 描述不符合 AGENTS.md → request-changes
         - 全部通过 → comment（加 -Approve 则自动 approve，自己的 PR 无法 approve 时会自动降级为 comment）
    已评审的 head SHA 记录在状态文件，避免重复评审；PR 有新提交（SHA 变化）会再次评审。

.PARAMETER Repo
    目标仓库，默认 gqf2008/Crystal。

.PARAMETER Base
    只跟踪合入该分支的 PR，默认 master。

.PARAMETER Watch
    持续轮询模式（每 IntervalSec 秒一次）；缺省只跑一轮。

.PARAMETER IntervalSec
    轮询间隔秒数，默认 60。

.PARAMETER Approve
    验证全部通过时自动 approve（默认只发 comment 评审，等待人工/协作 agent 确认）。

.PARAMETER Draft
    也评审 draft PR（默认跳过）。

.PARAMETER Force
    忽略状态文件，强制重新评审所有开放 PR。

.PARAMETER WorktreesRoot
    评审 worktree 根目录，默认为主仓库同级的 "Crystal-prreview-worktrees"。
    实际 worktree 路径为 <WorktreesRoot>\pr-<n>；建议保持默认，
    Client-Bevy 的 resolve_data_path 需要回退到主仓库的 Client-Macroquad/Data。

.PARAMETER StateFile
    已评审状态 JSON 文件路径，默认 <WorktreesRoot>\state.json。

.PARAMETER TimeoutSec
    单条验证命令超时秒数，默认 1800（30 分钟）。

.PARAMETER CodexThreadId
    发现新 PR / 新提交后要唤醒的 Codex 会话 ID（默认取 $env:CODEX_THREAD_ID，
    即 watcher 所在会话；用 `codex exec resume <ID> -` 把评审任务交给 Codex 处理）。

.PARAMETER NoWake
    关闭"唤醒 Codex"（只回帖 + 记录，不唤醒任何会话）。

.PARAMETER FeishuChatId
    可选：飞书群/会话 chat_id（oc_xxx），配置后新 PR 会通过 lark-cli 发飞书通知。
    需要飞书应用 bot 权限正常（当前环境 app secret 无效，默认关闭）。

.PARAMETER WakeCooldownMin
    同一 PR 两次唤醒的最短间隔分钟数（防刷屏），默认 5。

.EXAMPLE
    # 一轮评审当前所有开放 PR
    pwsh tools/pr-review.ps1 -Repo gqf2008/Crystal

    # 实时持续跟踪（每 60s 轮询）
    pwsh tools/pr-review.ps1 -Repo gqf2008/Crystal -Watch

    # 持续跟踪且验证通过后自动 approve
    pwsh tools/pr-review.ps1 -Repo gqf2008/Crystal -Watch -Approve -IntervalSec 120
#>
[CmdletBinding()]
param(
    [string]$Repo = "gqf2008/Crystal",
    [string]$Base = "master",
    [switch]$Watch,
    [int]$IntervalSec = 60,
    [switch]$Approve,
    [switch]$Draft,
    [switch]$Force,
    [string]$WorktreesRoot = "",
    [string]$StateFile = "",
    [int]$TimeoutSec = 1800,
    [string]$CodexThreadId = "",
    [switch]$NoWake,
    [string]$FeishuChatId = "",
    [int]$WakeCooldownMin = 5
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# ---------- 基础检查 ----------
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "未找到 gh CLI，请先安装并登录：https://cli.github.com/ （gh auth login）"
}

# 主仓库根目录（脚本位于 <repo>/tools/ 下）
$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) { throw "无法定位仓库根目录（$PSScriptRoot 不在 git 仓库内）" }
$RepoRoot = [IO.Path]::GetFullPath($RepoRoot)
$ParentDir = [IO.Path]::GetDirectoryName($RepoRoot)

if (-not $WorktreesRoot) { $WorktreesRoot = Join-Path $ParentDir "Crystal-prreview-worktrees" }
$WorktreesRoot = [IO.Path]::GetFullPath($WorktreesRoot)
if (-not $StateFile) { $StateFile = Join-Path $WorktreesRoot "state.json" }
New-Item -ItemType Directory -Force -Path $WorktreesRoot | Out-Null
if (-not $CodexThreadId) { $CodexThreadId = $env:CODEX_THREAD_ID }

# ---------- 状态文件 ----------
function Read-State {
    if (Test-Path -LiteralPath $StateFile) {
        try { return (Get-Content -Raw -LiteralPath $StateFile | ConvertFrom-Json -AsHashtable) }
        catch { Write-Warning "状态文件解析失败，将重新开始：$StateFile" }
    }
    return @{}
}

function Write-State([hashtable]$State) {
    # 动态序列化所有字段（含 notifiedSha 等扩展字段），避免丢状态
    $obj = [ordered]@{}
    foreach ($k in ($State.Keys | Sort-Object)) {
        $entry = [ordered]@{}
        $rec = $State[$k]
        foreach ($fk in $rec.Keys) {
            if ($null -ne $rec[$fk]) { $entry[$fk] = [string]$rec[$fk] }
        }
        $obj[$k] = $entry
    }
    $tmp = "$StateFile.tmp"
    $obj | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $tmp -Encoding UTF8
    Move-Item -LiteralPath $tmp -Destination $StateFile -Force
}

# ---------- PR 列表 ----------
function Get-OpenPrs {
    $json = gh pr list --repo $Repo --state open --base $Base `
        --json number,title,isDraft,headRefName,headRefOid,updatedAt,url,author,additions,deletions,changedFiles,body 2>$null
    if ($LASTEXITCODE -ne 0) { throw "gh pr list 失败" }
    if (-not $json) { return @() }
    return @($json | ConvertFrom-Json)
}

# ---------- worktree 管理 ----------
function Get-RegisteredWorktreePath([string]$WtPath) {
    $porcelain = git -C $RepoRoot worktree list --porcelain
    foreach ($line in $porcelain) {
        if ($line -like "worktree *") {
            $p = $line.Substring("worktree ".Length)
            if ([IO.Path]::GetFullPath($p) -eq [IO.Path]::GetFullPath($WtPath)) { return $p }
        }
    }
    return $null
}

function Ensure-ReviewWorktree([string]$PrNumber, [string]$Sha) {
    $wtPath = Join-Path $WorktreesRoot "pr-$PrNumber"

    # 安全校验：worktree 路径必须位于 WorktreesRoot 内
    $wtFull = [IO.Path]::GetFullPath($wtPath)
    $rootFull = [IO.Path]::GetFullPath($WorktreesRoot) + [IO.Path]::DirectorySeparatorChar
    if (-not $wtFull.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "worktree 路径越界，拒绝操作：$wtFull"
    }

    # 先确保对象存在：fetch PR head 到本地引用
    git -C $RepoRoot fetch origin "pull/$PrNumber/head:refs/remotes/origin/pr-review/$PrNumber" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "fetch PR #$PrNumber head 失败" }

    $registered = Get-RegisteredWorktreePath $wtPath
    if ($registered) {
        git -C $registered checkout --detach $Sha 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "worktree checkout $Sha 失败：$registered" }
        return $registered
    }

    if (Test-Path -LiteralPath $wtPath) {
        # 目录存在但未登记为 worktree（上次异常残留）——为避免误删，换用带短 SHA 的路径
        $wtPath = Join-Path $WorktreesRoot "pr-$PrNumber-$($Sha.Substring(0, 8))"
        $wtFull = [IO.Path]::GetFullPath($wtPath)
    }

    git -C $RepoRoot worktree add --detach $wtFull "refs/remotes/origin/pr-review/$PrNumber" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "git worktree add 失败：$wtFull" }
    return $wtFull
}

function Remove-ReviewWorktree([string]$WtPath) {
    $wtFull = [IO.Path]::GetFullPath($WtPath)
    $rootFull = [IO.Path]::GetFullPath($WorktreesRoot) + [IO.Path]::DirectorySeparatorChar
    if (-not $wtFull.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        Write-Warning "拒绝删除越界路径：$wtFull"
        return
    }
    if (Get-RegisteredWorktreePath $wtFull) {
        git -C $RepoRoot worktree remove --force $wtFull 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Warning "worktree 清理失败（下次运行会复用/跳过）：$wtFull" }
    }
}

# ---------- 验证执行 ----------
function Invoke-Cmd {
    param(
        [string]$File,
        [string[]]$ArgList,
        [string]$Dir,
        [string]$LogPrefix
    )
    $stdout = "$LogPrefix.out.log"
    $stderr = "$LogPrefix.err.log"
    $p = Start-Process -FilePath $File -ArgumentList $ArgList -WorkingDirectory $Dir `
        -NoNewWindow -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if (-not $p.WaitForExit($TimeoutSec * 1000)) {
        try { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } catch { }
        return @{ Ok = $false; TimedOut = $true; Code = -1; Tail = "超时（>$TimeoutSec 秒）" }
    }
    $code = $p.ExitCode
    $tail = New-Object System.Collections.Generic.List[string]
    if (Test-Path -LiteralPath $stdout) { $tail.AddRange([string[]](Get-Content -LiteralPath $stdout -Tail 30)) }
    if (Test-Path -LiteralPath $stderr) { $tail.AddRange([string[]](Get-Content -LiteralPath $stderr -Tail 30)) }
    $text = ($tail | Select-Object -Last 40) -join [Environment]::NewLine
    if ([string]::IsNullOrWhiteSpace($text)) { $text = "(无输出)" }
    return @{ Ok = ($code -eq 0); TimedOut = $false; Code = $code; Tail = $text }
}

function Get-TouchedCrates([string]$PrNumber) {
    $files = @(git -C $RepoRoot diff --name-only "refs/remotes/origin/$Base...refs/remotes/origin/pr-review/$PrNumber")
    if ($LASTEXITCODE -ne 0) {
        # 三点的 merge-base 可能失败（例如 base 已删），退回两点比较
        $files = @(git -C $RepoRoot diff --name-only "refs/remotes/origin/$Base" "refs/remotes/origin/pr-review/$PrNumber")
    }
    $crates = @{}
    foreach ($f in [array]$files) {
        if ($f -like "Client-Bevy/*")          { $crates["Client-Bevy"] = $true }
        elseif ($f -like "SharedRust/*")       { $crates["SharedRust"] = $true }
        elseif ($f -like "ServerRust/*")       { $crates["ServerRust"] = $true }
        elseif ($f -like "Client-Macroquad/*") { $crates["Client-Macroquad"] = $true }
    }
    return @($crates.Keys)
}

function Invoke-Validation([string]$PrNumber, [string]$WtPath) {
    $results = New-Object System.Collections.Generic.List[object]
    $touched = @(Get-TouchedCrates $PrNumber)
    if ($touched.Count -eq 0) {
        return @{ Results = $results; Touched = $touched; Passed = $true }
    }

    $stamp = Get-Date -Format "yyyyMMddHHmmss"
    $anyFailed = $false

    if ($touched -contains "Client-Bevy") {
        $cb = Join-Path $WtPath "Client-Bevy"
        foreach ($cmd in @(@{ Name = "cargo check"; Args = @("check") }, @{ Name = "cargo test"; Args = @("test", "--lib") })) {
            $r = Invoke-Cmd -File "cargo" -ArgList $cmd.Args -Dir $cb -LogPrefix (Join-Path $WorktreesRoot "cb-$stamp")
            if (-not $r.Ok) { $anyFailed = $true }
            $results.Add([pscustomobject]@{ Crate = "Client-Bevy"; Cmd = $cmd.Name; Ok = $r.Ok; Tail = $r.Tail })
        }
    }
    if ($touched -contains "ServerRust" -or $touched -contains "SharedRust") {
        $sr = Join-Path $WtPath "ServerRust"
        $r = Invoke-Cmd -File "cargo" -ArgList @("test") -Dir $sr -LogPrefix (Join-Path $WorktreesRoot "sr-$stamp")
        if (-not $r.Ok) { $anyFailed = $true }
        $results.Add([pscustomobject]@{ Crate = "ServerRust"; Cmd = "cargo test"; Ok = $r.Ok; Tail = $r.Tail })
    }
    if ($touched -contains "SharedRust" -and $touched -notcontains "ServerRust") {
        # 只改 SharedRust 时额外跑它的 lib 单测
        $sh = Join-Path $WtPath "SharedRust"
        $r = Invoke-Cmd -File "cargo" -ArgList @("test", "--lib") -Dir $sh -LogPrefix (Join-Path $WorktreesRoot "sh-$stamp")
        if (-not $r.Ok) { $anyFailed = $true }
        $results.Add([pscustomobject]@{ Crate = "SharedRust"; Cmd = "cargo test --lib"; Ok = $r.Ok; Tail = $r.Tail })
    }
    if ($touched -contains "Client-Macroquad" -and $touched -notcontains "Client-Bevy") {
        $mc = Join-Path $WtPath "Client-Macroquad"
        $r = Invoke-Cmd -File "cargo" -ArgList @("check") -Dir $mc -LogPrefix (Join-Path $WorktreesRoot "mc-$stamp")
        if (-not $r.Ok) { $anyFailed = $true }
        $results.Add([pscustomobject]@{ Crate = "Client-Macroquad"; Cmd = "cargo check"; Ok = $r.Ok; Tail = $r.Tail })
    }

    return @{ Results = $results; Touched = $touched; Passed = (-not $anyFailed) }
}

# ---------- 评审意见 ----------
function New-ReviewBody {
    param($Pr, $Validation, $CiSummary, $Viewer)
    $sb = New-Object System.Text.StringBuilder
    [void]$sb.AppendLine("## 🤖 自动评审（pr-review.ps1 · $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')）")
    [void]$sb.AppendLine()
    [void]$sb.AppendLine("**PR**: [$($Pr.number) $($Pr.title)]($($Pr.url)) · 分支 ``$($Pr.headRefName)`` → ``$Base``")
    [void]$sb.AppendLine("**Head**: ``$($Pr.headRefOid.Substring(0,8))`` · +$($Pr.additions) −$($Pr.deletions) · $($Pr.changedFiles) 个文件")
    [void]$sb.AppendLine()

    # 验证矩阵
    [void]$sb.AppendLine("### 验证结果（AGENTS.md 要求）")
    [void]$sb.AppendLine()
    [void]$sb.AppendLine("| Crate | 命令 | 结果 |")
    [void]$sb.AppendLine("|---|---|---|")
    if ($Validation.Touched.Count -eq 0) {
        [void]$sb.AppendLine("| （未涉及 Rust crate） | — | ⏭️ 无需本地验证 |")
    } else {
        foreach ($r in $Validation.Results) {
            $icon = if ($r.Ok) { "✅ 通过" } else { "❌ 失败" }
            [void]$sb.AppendLine("| $($r.Crate) | ``$($r.Cmd)`` | $icon |")
        }
    }
    [void]$sb.AppendLine()

    # 失败详情
    $failed = @($Validation.Results | Where-Object { -not $_.Ok })
    if ($failed.Count -gt 0) {
        [void]$sb.AppendLine("### ❌ 失败详情")
        [void]$sb.AppendLine()
        foreach ($f in $failed) {
            [void]$sb.AppendLine("**$($f.Crate) · $($f.Cmd)**（exit code: $($f.Code)）")
            [void]$sb.AppendLine()
            [void]$sb.AppendLine("``````")
            [void]$sb.AppendLine(($f.Tail -split "`n" | ForEach-Object { $_.TrimEnd() } | Select-Object -Last 40) -join "`n")
            [void]$sb.AppendLine("``````")
            [void]$sb.AppendLine()
        }
    }

    # 检查项
    $bodyOk = ($Pr.body -and $Pr.body.Length -gt 80)
    [void]$sb.AppendLine("### 检查项")
    [void]$sb.AppendLine()
    [void]$sb.AppendLine("- $(if ($bodyOk) {'[x]'} else {'[ ]'}) PR 描述包含 改了什么 / 为什么 / 验证了什么（AGENTS.md）")
    [void]$sb.AppendLine("- $(if ($CiSummary) {'[x]'} else {'[ ]'}) CI 状态：$CiSummary")
    [void]$sb.AppendLine()

    # 结论
    $pass = $Validation.Passed -and $bodyOk
    if ($pass) {
        if ($Approve -and $Viewer -ne $Pr.author.login) {
            [void]$sb.AppendLine("### ✅ 结论")
            [void]$sb.AppendLine("本地验证通过，PR 描述完整 → **已自动 approve**。")
        } else {
            [void]$sb.AppendLine("### ✅ 结论")
            [void]$sb.AppendLine("本地验证通过，PR 描述完整。未自动 approve，等待人工/协作 agent 确认后合并。")
            if ($Approve -and $Viewer -eq $Pr.author.login) {
                [void]$sb.AppendLine("> 注：评审账号与 PR 作者相同，GitHub 不允许自 approve，已降级为 comment。")
            }
        }
    } elseif (-not $Validation.Passed) {
        [void]$sb.AppendLine("### ❌ 结论")
        [void]$sb.AppendLine("本地验证未通过，请修复后推送新提交（新 SHA 会自动重新评审）。")
    } else {
        [void]$sb.AppendLine("### ⚠️ 结论")
        [void]$sb.AppendLine("验证通过但 PR 描述不完整，请补充 改了什么 / 为什么 / 验证了什么 后推送。")
    }
    return $sb.ToString()
}

function Post-Review($Pr, $Body, $Passed, $Viewer) {
    $bodyFile = Join-Path $WorktreesRoot "review-$($Pr.number)-$(Get-Date -Format 'yyyyMMddHHmmss').md"
    Set-Content -LiteralPath $bodyFile -Value $Body -Encoding UTF8

    if ($Passed) {
        if ($Approve -and $Viewer -ne $Pr.author.login) { $event = "--approve" } else { $event = "--comment" }
    } else {
        $event = "--request-changes"
    }
    gh pr review $Pr.number --repo $Repo $event --body-file $bodyFile
    if ($LASTEXITCODE -ne 0) { Write-Warning "gh pr review #$($Pr.number) 失败（$event）" }
}

# ---------- 通知：发现新 PR 后唤醒 Codex / 飞书 ----------
function Send-Notify {
    param($Pr, [string]$CiSummary, [string]$Result, [string]$PrNum, [string]$Sha, $State)

    # 同一 head SHA 只唤醒一次（防止 -Force/崩溃重跑导致重复唤醒）
    if ($State[$PrNum] -and $State[$PrNum].notifiedSha -eq $Sha) { return }

    if (-not $NoWake -and $CodexThreadId) {
        $prompt = "【PR 实时跟踪】发现需要处理的 GitHub PR #$($Pr.number)：$($Pr.title)`n" +
            "URL: $($Pr.url)`n" +
            "分支 $($Pr.headRefName) → $Base，+$($Pr.additions) −$($Pr.deletions)，$($Pr.changedFiles) 个文件`n" +
            "CI: $CiSummary`n" +
            "本地验证: $Result（watcher 已按 AGENTS.md 回帖）`n" +
            "请不要调用任何工具，仅回复：已收到 PR #$($Pr.number)，等待主会话处理。"
        $stamp = Get-Date -Format 'yyyyMMddHHmmss'
        $pf = Join-Path $WorktreesRoot "wake-$($Pr.number)-$stamp.txt"
        $wl = Join-Path $WorktreesRoot "wake-$($Pr.number)-$stamp.log"
        try {
            Set-Content -LiteralPath $pf -Value $prompt -Encoding UTF8
            $cmd = "Get-Content -Raw -LiteralPath '$pf' | codex exec resume '$CodexThreadId' -"
            Start-Process -FilePath 'pwsh' -ArgumentList @('-NoProfile', '-Command', $cmd) `
                -WindowStyle Hidden -RedirectStandardOutput $wl -RedirectStandardError "$wl.err" | Out-Null
            Write-Host "[$(Get-Date -Format 'HH:mm:ss')] 已唤醒 Codex 处理 PR #$($Pr.number)"
        }
        catch {
            Write-Warning "唤醒 Codex 失败：$($_.Exception.Message)"
        }
    }

    if ($FeishuChatId) {
        try {
            $md = "**PR #$($Pr.number) 需要评审**：$($Pr.title)`n$($Pr.url)`nCI: $CiSummary"
            lark-cli im +messages-send --as bot --chat-id $FeishuChatId --markdown $md 2>&1 | Out-Null
            Write-Host "[$(Get-Date -Format 'HH:mm:ss')] 已发送飞书通知（PR #$($Pr.number)）"
        }
        catch {
            Write-Warning "飞书通知失败：$($_.Exception.Message)"
        }
    }
}

# ---------- 主流程 ----------
function Invoke-ReviewRound {
    $State = Read-State
    $Prs = @(Get-OpenPrs)
    $Viewer = (gh api user --jq .login 2>$null) ?? ""

    if ($null -eq $Prs -or $Prs.Count -eq 0) {
        Write-Host "[$(Get-Date -Format 'HH:mm:ss')] 没有开放的 PR（base=$Base）"
        return
    }

    foreach ($pr in $Prs) {
        $prNum = [string]$pr.number
        $sha = [string]$pr.headRefOid
        $prev = $State[$prNum]
        if (-not $Force -and $prev -and $prev.sha -eq $sha) {
            Write-Host "[$(Get-Date -Format 'HH:mm:ss')] PR #$prNum 已评审（$($prev.result)），head 未变，跳过"
            continue
        }
        if ($pr.isDraft -and -not $Draft) {
            Write-Host "[$(Get-Date -Format 'HH:mm:ss')] PR #$prNum 是 draft，跳过（-Draft 可评审）"
            continue
        }

        Write-Host "[$(Get-Date -Format 'HH:mm:ss')] 开始评审 PR #$prNum（$($pr.title)） head=$($sha.Substring(0,8))"
        $wt = $null
        try {
            $wt = Ensure-ReviewWorktree $prNum $sha
            $validation = Invoke-Validation $prNum $wt
            $ci = gh pr view $prNum --repo $Repo --json statusCheckRollup --jq '[.statusCheckRollup[]? | .name + ":" + (if (.conclusion != null and .conclusion != "") then .conclusion else .status end)] | join(", ")' 2>$null
            if ([string]::IsNullOrWhiteSpace($ci)) { $ci = "无 CI 检查" }
            $body = New-ReviewBody -Pr $pr -Validation $validation -CiSummary $ci -Viewer $Viewer
            Post-Review -Pr $pr -Body $body -Passed ($validation.Passed -and $pr.body -and $pr.body.Length -gt 80) -Viewer $Viewer
            $result = if ($validation.Passed) { "passed" } else { "failed" }
            Send-Notify -Pr $pr -CiSummary $ci -Result $result -PrNum $prNum -Sha $sha -State $State
            $State[$prNum] = @{ sha = $sha; result = $result; reviewedAt = (Get-Date -Format 'o'); url = $pr.url; notifiedSha = $sha }
            Write-State $State
            Write-Host "[$(Get-Date -Format 'HH:mm:ss')] PR #$prNum 评审完成：$result"
        }
        catch {
            Write-Warning "PR #$prNum 评审失败：$($_.Exception.Message)"
            Write-Warning "位置：$($_.InvocationInfo.PositionMessage)"
            Write-Warning "栈：$($_.ScriptStackTrace)"
        }
        finally {
            if ($wt) { Remove-ReviewWorktree $wt }
        }
    }
}

Invoke-ReviewRound
if ($Watch) {
    while ($true) {
        Start-Sleep -Seconds $IntervalSec
        try { Invoke-ReviewRound } catch { Write-Warning "轮询出错：$($_.Exception.Message)" }
    }
}
