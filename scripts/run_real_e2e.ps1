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

# 用例成功判定标记（A=发起方/单客户端日志，B=接受方日志；全部命中才 PASS，防止登录 ✅ 误判）
$CaseRequired = @{
    "fishing-test"  = @{ A = @('\[FISHTEST\] ✅ 收获消息'); B = @() }
    "mount-test"    = @{ A = @('\[MOUNT\] ✅ 下马成功'); B = @() }
    "gameshop-test" = @{ A = @('\[SHOPTEST\] ✅ 完成（购买 #'); B = @() }
    "ranking-test"  = @{ A = @('\[RANKTEST\] ✅ 排行榜'); B = @() }
    "refine-test"   = @{ A = @('\[REFINETEST\] ✅ 精炼已开始'); B = @() }
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

function Run-Client {
    param([string]$Name, [string[]]$ClientArgs, [int]$Timeout)
    $err = Join-Path $tmp "$Name.err.log"
    $out = Join-Path $tmp "$Name.out.log"
    $p = Start-Process -FilePath $ClientExe -ArgumentList $ClientArgs -RedirectStandardError $err -RedirectStandardOutput $out -PassThru -WindowStyle Hidden
    $done = $p.WaitForExit($Timeout * 1000)
    if (-not $done) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    Start-Sleep -Milliseconds 400
    return ,(Get-Marks $err)
}

function Invoke-Case {
    param([string]$Name, [string[]]$Flags)
    $argsAll = @("--real-net","--auto-enter") + $Flags + @("--e2e-user",$TestUser,"--e2e-pass",$TestPass)
    $err = Join-Path $tmp "$Name.err.log"
    $marks = Run-Client $Name $argsAll $TimeoutSec
    $req = $CaseRequired[$Name]
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
    $done = $a.WaitForExit($TimeoutSec * 1000)
    if (-not $done) { Stop-Process -Id $a.Id -Force -ErrorAction SilentlyContinue }
    Stop-Process -Id $b.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
    $mA = Get-Marks $aErr 3
    $mB = Get-Marks $bErr 3
    $req = $CaseRequired[$Name]
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

# 1) 启动服务端
Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force
$srvErr = Join-Path $tmp "server.err.log"; $srvOut = Join-Path $tmp "server.log"
$srv = Start-Process -FilePath $ServerExe -WorkingDirectory $ServerWorkDir -RedirectStandardError $srvErr -RedirectStandardOutput $srvOut -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 15
if (-not (Get-Process -Id $srv.Id -ErrorAction SilentlyContinue)) {
    Write-Error "服务端启动失败：$(Get-Content $srvErr -Tail 5 -ErrorAction SilentlyContinue)"
}
Write-Output "服务端已启动 PID=$($srv.Id)"

# 2) 单客户端用例
foreach ($f in $SingleFlags) {
    $name = $f.TrimStart('-')
    Invoke-Case $name @($f)
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
