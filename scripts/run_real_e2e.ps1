#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Crystal 真实服 E2E 回归脚本（Real-server end-to-end regression）。

.DESCRIPTION
    固化已验证的核心流程为可重复回归：
      - 单客户端：登录/进图 + 钓鱼/坐骑/商城/排行榜/精炼/举报/升级特效
      - 双客户端配对：组队/交易/私聊/邮件/好友
      - 服务端存活检查
    每个用例按客户端日志中的 ✅/❌ 标记判定；结束后打印汇总并停止服务端。

.PARAMETER ServerExe      mir2_server.exe 路径（默认 仓库/ServerRust/target/debug/mir2_server.exe）。
.PARAMETER ClientExe      client_bevy.exe 路径（默认 仓库/Client-Bevy/target/debug/client_bevy.exe）。
.PARAMETER ServerWorkDir  服务端工作目录（含 config/server.toml、data/crystal.db、Daneo1989；默认 仓库/ServerRust）。
.PARAMETER TestUser/TestPass      主测试账号（默认 test/123456，角色 bevychar）。
.PARAMETER SecondUser/SecondPass  配对测试账号（默认 bevy2/123456，角色 bevy2char）。
.PARAMETER TimeoutSec     单个用例超时秒数（默认 75）。
.PARAMETER SingleFlags    单客户端用例 flag 列表。
.PARAMETER KeepServer     结束时保留服务端运行（默认停止）。

.EXAMPLE
    ./scripts/run_real_e2e.ps1
#>
param(
    [string]$ServerExe = "",
    [string]$ClientExe = "",
    [string]$ServerWorkDir = "",
    [string]$TestUser = "test",
    [string]$TestPass = "123456",
    [string]$SecondUser = "bevy2",
    [string]$SecondPass = "123456",
    [int]$TimeoutSec = 75,
    [string[]]$SingleFlags = @("--fishing-test","--mount-test","--gameshop-test","--ranking-test","--refine-test","--report-test","--level-fx-test"),
    [switch]$KeepServer
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
if (-not $ServerExe) { $ServerExe = Join-Path $repo "ServerRust\target\debug\mir2_server.exe" }
if (-not $ClientExe) { $ClientExe = Join-Path $repo "Client-Bevy\target\debug\client_bevy.exe" }
if (-not $ServerWorkDir) { $ServerWorkDir = Join-Path $repo "ServerRust" }
$tmp = Join-Path $env:TEMP "crystal_e2e"
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
$results = [System.Collections.Generic.List[object]]::new()

# 用例超时（秒）：default 见 $TimeoutSec；精炼要等结算（客户端 65s 等待 + 结算 + 取回）
$CaseTimeout = @{ "refine-test" = 180 }

# 用例成功判定标记（A=发起方/单客户端日志，B=接受方日志；全部命中才 PASS，防止登录 ✅ 误判）
$CaseRequired = @{
    "fishing-test"  = @{ A = @('\[FISHTEST\] ✅ 收获消息'); B = @() }
    "mount-test"    = @{ A = @('\[MOUNT\] ✅ 下马成功'); B = @() }
    "gameshop-test" = @{ A = @('\[SHOPTEST\] ✅ 完成（购买 #'); B = @() }
    "ranking-test"  = @{ A = @('\[RANKTEST\] ✅ 排行榜'); B = @() }
    # #2887：精炼判「全流程」（存入 → 开始 → 结果 → 取回），只到「已开始」不算过
    "refine-test"   = @{ A = @('\[REFINETEST\] ✅ 精炼已开始','\[REFINETEST\] ✅ 取回成功，精炼全流程完成'); B = @() }
    "report-test"   = @{ A = @('\[REPORTTEST\] ✅ 举报已提交确认'); B = @() }
    "level-fx-test" = @{ A = @('\[LEVELFX\] ✅ PASS 升级生效'); B = @() }
    "group"         = @{ A = @('\[GROUPTEST\] ✅ 组队成功'); B = @('\[GROUPACCEPT\] ✅ 接受邀请') }
    "whisper"       = @{ A = @(); B = @('\[WHCHECK\] ✅ 收到私聊') }
    "mail"          = @{ A = @(); B = @('\[MAILREAD\] ✅ 已读取邮件') }
    "trade"         = @{ A = @('\[TRADETEST\] ✅ 交易窗口已打开'); B = @('\[TRADEACCEPT\] ✅ 接受邀请') }
    "friend"        = @{ A = @('\[FRIENDTEST\] ✅ 好友列表包含'); B = @() }
    "marriage"      = @{ A = @('\[MARRY\] ✅ 离婚成功'); B = @('\[MARRYACC\] ✅ 离婚完成') }
}

function Test-Marks {
    param([string]$LogPath, [string[]]$Patterns)
    if ($Patterns.Count -eq 0) { return $true }
    foreach ($pat in $Patterns) {
        $hit = Select-String -Path $LogPath -Pattern $pat -ErrorAction SilentlyContinue
        if (-not $hit) { return $false }
    }
    return $true
}

function Get-Marks {
    param([string]$LogPath, [int]$Last = 4)
    Select-String -Path $LogPath -Pattern "✅|❌" -ErrorAction SilentlyContinue |
        Select-Object -Last $Last | ForEach-Object { ($_.Line -replace '^.*? (INFO|WARN|ERROR) ', '') -replace "\x1b\[[0-9;]*m", '' }
}

function Get-LogoutCount {
    param([string]$User)
    @(Select-String -Path $srvOut -Pattern "Account logged out: $User" -ErrorAction SilentlyContinue).Count
}

# 停客户端前记基线，停完等它登出落盘（#2890）
# —— 实测客户端被杀后服务端立刻记 logout，这步只是兜底：
#    否则提前停用例会让下一个用例抢在「登出」前登录，被服务端按「账号已在线」拒绝（历史偶发 FAIL）
function Snapshot-Logout {
    param([string[]]$Users)
    $h = @{}
    foreach ($u in $Users) { if ($u) { $h[$u] = Get-LogoutCount $u } }
    return $h
}

function Wait-Logout {
    param([hashtable]$Before, [int]$TimeoutSec = 15)
    if ($Before.Count -eq 0) { return $true }
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $ok = $true
        foreach ($u in $Before.Keys) { if ((Get-LogoutCount $u) -le $Before[$u]) { $ok = $false } }
        if ($ok) { return $true }
        Start-Sleep -Milliseconds 400
    }
    Write-Warning "等待账号登出超时（$($Before.Keys -join ',')）——下一个用例可能被「账号已在线」拒绝"
    return $false
}

function Run-Client {
    param(
        [string]$Name,
        [string[]]$ClientArgs,
        [int]$Timeout,
        [string[]]$RequiredA = @(),
        [string]$User = ""
    )
    $err = Join-Path $tmp "$Name.err.log"
    $out = Join-Path $tmp "$Name.out.log"
    $p = Start-Process -FilePath $ClientExe -ArgumentList $ClientArgs -RedirectStandardError $err -RedirectStandardOutput $out -PassThru -WindowStyle Hidden
    # #2890：判定标记一出现就停（用例本身 14~30s 就出标记，之前每个用例都白等满超时）
    $deadline = (Get-Date).AddSeconds($Timeout)
    while ((Get-Date) -lt $deadline) {
        if ($RequiredA.Count -gt 0 -and (Test-Marks $err $RequiredA)) { break }
        if (-not (Get-Process -Id $p.Id -ErrorAction SilentlyContinue)) { break }
        Start-Sleep -Milliseconds 1000
    }
    $before = Snapshot-Logout @($User)
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Wait-Logout -Before $before | Out-Null
    Start-Sleep -Milliseconds 300
    return ,(Get-Marks $err)
}

function Invoke-Case {
    param([string]$Name, [string[]]$Flags)
    $argsAll = @("--real-net","--auto-enter") + $Flags + @("--e2e-user",$TestUser,"--e2e-pass",$TestPass)
    $err = Join-Path $tmp "$Name.err.log"
    $caseTimeout = if ($CaseTimeout.ContainsKey($Name)) { $CaseTimeout[$Name] } else { $TimeoutSec }
    $req = $CaseRequired[$Name]
    $requiredA = if ($null -ne $req) { @($req.A) } else { @() }
    $marks = Run-Client $Name $argsAll $caseTimeout $requiredA $TestUser
    if ($null -eq $req) {
        # 自定义用例：退回任意 ✅ 判定
        $pass = ($marks | Where-Object { $_ -match "✅" }).Count -gt 0
    } else {
        $pass = Test-Marks $err $req.A
    }
    $results.Add([pscustomobject]@{ Case=$Name; Pass=$pass; Marks=($marks -join " | ") })
    Write-Output ("[{0}] {1}" -f $(if($pass){"PASS"}else{"FAIL"}), $Name)
    if ($marks) { $marks | ForEach-Object { Write-Output ("    " + $_) } }
}

function Invoke-PairCase {
    param([string]$Name, [string]$FlagA, [string]$FlagB)
    $aErr = Join-Path $tmp "${Name}_A.err.log"; $aOut = Join-Path $tmp "${Name}_A.out.log"
    $bErr = Join-Path $tmp "${Name}_B.err.log"; $bOut = Join-Path $tmp "${Name}_B.out.log"
    $aArgs = @("--real-net","--auto-enter",$FlagA,"--e2e-user",$TestUser,"--e2e-pass",$TestPass)
    $bArgs = @("--real-net","--auto-enter","--e2e-user",$SecondUser,"--e2e-pass",$SecondPass)
    if ($FlagB) { $bArgs = @("--real-net","--auto-enter",$FlagB) + $bArgs }
    # 配对用例前置重置（#1230）：前置用例会改变角色朝向/位置并被自动保存，
    # 交易要求目标在正前方一格且面对面，必须每次配对前恢复摆位（否则 trade 偶发失败）。
    $pyCmd = Get-Command python -ErrorAction SilentlyContinue
    if ($pyCmd) {
        & python (Join-Path $PSScriptRoot "e2e_setup_db.py") (Join-Path $ServerWorkDir "data\crystal.db") | Out-Null
    }
    $a = Start-Process -FilePath $ClientExe -ArgumentList $aArgs -RedirectStandardError $aErr -RedirectStandardOutput $aOut -PassThru -WindowStyle Hidden
    $b = Start-Process -FilePath $ClientExe -ArgumentList $bArgs -RedirectStandardError $bErr -RedirectStandardOutput $bOut -PassThru -WindowStyle Hidden
    $req = $CaseRequired[$Name]
    # #2890：两侧判定标记都出现就停（不必等满超时）
    #        某侧没有判定标记（如 friend 只看 A、whisper 只看 B）→ 该侧视为已满足
    $aPat = if ($null -ne $req) { @($req.A) } else { @() }
    $bPat = if ($null -ne $req) { @($req.B) } else { @() }
    $hasAnyPattern = ($aPat.Count + $bPat.Count) -gt 0
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $aOk = if ($aPat.Count -gt 0) { Test-Marks $aErr $aPat } else { $true }
        $bOk = if ($bPat.Count -gt 0) { Test-Marks $bErr $bPat } else { $true }
        if ($hasAnyPattern -and $aOk -and $bOk) { break }
        if (-not (Get-Process -Id $a.Id -ErrorAction SilentlyContinue) -and -not (Get-Process -Id $b.Id -ErrorAction SilentlyContinue)) { break }
        Start-Sleep -Milliseconds 1000
    }
    $before = Snapshot-Logout @($TestUser, $SecondUser)
    Stop-Process -Id $a.Id -Force -ErrorAction SilentlyContinue
    Stop-Process -Id $b.Id -Force -ErrorAction SilentlyContinue
    Wait-Logout -Before $before | Out-Null
    Start-Sleep -Milliseconds 300
    $mA = Get-Marks $aErr 3
    $mB = Get-Marks $bErr 3
    if ($null -eq $req) {
        $pass = ($mA -match "✅").Count -gt 0 -and ($mB -match "✅").Count -gt 0
    } else {
        $pass = (Test-Marks $aErr $req.A) -and (Test-Marks $bErr $req.B)
    }
    $results.Add([pscustomobject]@{ Case=$Name; Pass=$pass; Marks=("A: " + ($mA -join " | ") + "  B: " + ($mB -join " | ")) })
    Write-Output ("[{0}] {1}" -f $(if($pass){"PASS"}else{"FAIL"}), $Name)
    $mA | ForEach-Object { Write-Output ("    A " + $_) }
    $mB | ForEach-Object { Write-Output ("    B " + $_) }
}

# 0) 测试库准备：安全点 + 背包物品（#990 怪物 AI 后城镇出生点会被围杀）
$pyCmd = Get-Command python -ErrorAction SilentlyContinue
if ($pyCmd) {
    & python (Join-Path $PSScriptRoot "e2e_setup_db.py") (Join-Path $ServerWorkDir "data\crystal.db")
} else {
    Write-Warning "python 不可用，跳过 E2E 测试库准备（角色可能被怪物围杀）"
}

# 0b) 精炼 e2e 专用配置（#2887）：服务端支持 `mir2_server <config>` 启动参数
#     （ServerRust/src/main.rs:55-57）。用临时 config 而不是改仓库里的 config/server.toml，
#     跑崩了也不会留下改过的配置。
#     注意：这里**只生成配置**；改库（角色挪到铁匠旁）必须等精炼用例前再做，
#     否则会把前面的钓鱼/坐骑/商城用例的角色位置一起带偏。
$wantRefine = $SingleFlags -contains "--refine-test"
$refinePrep = Join-Path $PSScriptRoot "e2e_refine_prep.py"
$refineCfg = Join-Path $tmp "server.refine-e2e.toml"
$srvArgs = @()
if ($wantRefine -and $pyCmd) {
    & python $refinePrep config (Join-Path $ServerWorkDir "config\server.toml") $refineCfg
    if ($LASTEXITCODE -eq 0) { $srvArgs = @($refineCfg) } else { Write-Warning "精炼前置失败，refine-test 可能不通过" }
} elseif ($wantRefine) {
    Write-Warning "python 不可用，跳过精炼前置（refine-test 大概率失败）"
}

# 1) 启动服务端
Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force
$srvErr = Join-Path $tmp "server.err.log"; $srvOut = Join-Path $tmp "server.log"
$srv = Start-Process -FilePath $ServerExe -ArgumentList $srvArgs -WorkingDirectory $ServerWorkDir -RedirectStandardError $srvErr -RedirectStandardOutput $srvOut -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 15
if (-not (Get-Process -Id $srv.Id -ErrorAction SilentlyContinue)) {
    Write-Error "服务端启动失败：$(Get-Content $srvErr -Tail 5 -ErrorAction SilentlyContinue)"
}
Write-Output "服务端已启动 PID=$($srv.Id)"

# 2) 单客户端用例
foreach ($f in $SingleFlags) {
    $name = $f.TrimStart('-')
    # 2a) 精炼用例前：摆位 + 材料（#2887）；跑完立刻还原，避免影响后面的配对用例同图摆位
    if ($name -eq "refine-test" -and $pyCmd) {
        & python $refinePrep prepare-db (Join-Path $ServerWorkDir "data\crystal.db")
    }
    Invoke-Case $name @($f)
    if ($name -eq "refine-test" -and $pyCmd) {
        & python $refinePrep restore-db (Join-Path $ServerWorkDir "data\crystal.db")
    }
}

# 3) 双客户端配对用例（组队/私聊/邮件/交易/好友）
Invoke-PairCase "group"   "--group-test"   "--group-accept"
Invoke-PairCase "whisper" "--whisper-send" "--whisper-check"
Invoke-PairCase "mail"    "--mail-test"    "--mail-read"
Invoke-PairCase "trade"   "--trade-test"   "--trade-accept"
Invoke-PairCase "friend"  "--friend-test"  ""
Invoke-PairCase "marriage" "--marriage-test" "--marriage-accept"

# 4) 服务端存活检查
$alive = Get-Process -Id $srv.Id -ErrorAction SilentlyContinue
Write-Output ("服务端存活: " + [bool]$alive)

# 5) 汇总
Write-Output "===== 汇总 ====="
$results | Format-Table Case, Pass, Marks -AutoSize | Out-String | Write-Output
$passCount = ($results | Where-Object Pass).Count
Write-Output ("通过 {0}/{1}" -f $passCount, $results.Count)

if (-not $KeepServer) { Stop-Process -Id $srv.Id -Force -ErrorAction SilentlyContinue }
