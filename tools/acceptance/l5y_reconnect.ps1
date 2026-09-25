<#
.SYNOPSIS
  服务端重启 / 网络闪断后的**客户端自动重连**夹具（`--reconnect-test` 的场景化 runner）。

.DESCRIPTION
  场景（上线运营真实会发生）：玩家在游戏中 → 服务端重启（回滚/发布/崩溃恢复）或链路闪断 →
  客户端应**自动重连并重新进入游戏**，不需要玩家手点。

  本夹具把这条链路做成可复跑判据：
    ① 起服务端 + 起客户端（`--reconnect-test`），等客户端进图；
    ② **杀掉**服务端（模拟中断），`-OutageSec` 秒后把同一份服务端**起回来**；
    ③ 断言客户端依次出现「检测到断线」与「✅ 自动重连成功并重新进入游戏」。

  判据全部取自客户端日志的 `[RECON]` 行（状态而非猜测）：只看「进图后有没有真的断线、
  有没有真的回到游戏」。不断言具体重连耗时（指数退避：2s→4s→…→30s，与负载/关机速度有关）。

.PARAMETER ServerWorkDir  受测服务端工作目录（含 `Data\crystal.db` 与 `config\server.toml`）。
                          默认依次尝试 `<repo>\ServerRust`、`%TEMP%\e2e_workdir`；都没有就 exit 2。
.PARAMETER ClientHome     客户端构建根（其 `Client-Bevy\target\debug\client_bevy.exe`）；默认本仓库。
.PARAMETER OutageSec      服务端停机秒数（默认 2；要测更长闪断就调大）。

.NOTES
  退出码：0 全过 / 1 有 FAIL / 2 前置失败。
  起客户端前先拿实机锁（见 e2e_lock.ps1），整段 try/finally 释放。
  用法：pwsh tools/acceptance/l5y_reconnect.ps1 -ServerWorkDir %TEMP%\e2e_workdir
#>
param(
    [string]$ServerWorkDir = '',
    [string]$ClientHome = '',
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$OutageSec = 2,
    [int]$EnterTimeoutSec = 90,
    [int]$RecoverTimeoutSec = 180
)

# ---- 实机资源互斥 ----------------------------------------------------------
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5y_reconnect' -TimeoutSec 1800)) { exit 2 }

try {
    $ErrorActionPreference = 'Continue'
    $env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
    $env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
    $repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    if (-not $ClientHome) { $ClientHome = $repo }
    $clientExe = Join-Path $ClientHome 'Client-Bevy\target\debug\client_bevy.exe'
    if (-not (Test-Path -LiteralPath $clientExe)) {
        Write-Host ("前置失败：找不到客户端产物 {0}（先 cargo build --bin client_bevy）" -f $clientExe)
        exit 2
    }

    # 受测服务端工作目录：优先参数，其次仓库 ServerRust，再次 %TEMP%\e2e_workdir（真机 E2E 用的那个库）
    $candidates = @()
    if ($ServerWorkDir) { $candidates += $ServerWorkDir }
    $candidates += (Join-Path $repo 'ServerRust')
    $candidates += (Join-Path $env:TEMP 'e2e_workdir')
    $srvHome = $null
    foreach ($c in $candidates) {
        if ($c -and (Test-Path -LiteralPath (Join-Path $c 'Data\crystal.db'))) { $srvHome = (Resolve-Path $c).Path; break }
    }
    if (-not $srvHome) {
        Write-Host '前置失败：找不到含 Data\crystal.db 的服务端工作目录（用 -ServerWorkDir 指定）'
        exit 2
    }
    $serverExe = Join-Path $repo 'ServerRust\target\debug\mir2_server.exe'
    if (-not (Test-Path -LiteralPath $serverExe)) {
        Write-Host ("前置失败：找不到服务端产物 {0}" -f $serverExe)
        exit 2
    }
    Write-Host ("[环境] 受测服务端工作目录={0}" -f $srvHome)
    Write-Host ("[环境] 客户端产物={0}" -f $clientExe)

    $work = Join-Path $env:TEMP 'l5y_reconnect'
    New-Item -ItemType Directory -Path $work -Force | Out-Null
    $cliLog = Join-Path $work 'client.err.log'
    Remove-Item $cliLog -Force -ErrorAction SilentlyContinue

    function Start-TestServer([string]$tag) {
        Start-Process -FilePath $serverExe -WorkingDirectory $srvHome `
            -RedirectStandardOutput (Join-Path $work "server_$tag.out.log") `
            -RedirectStandardError (Join-Path $work "server_$tag.err.log") -PassThru -WindowStyle Hidden
    }
    function Wait-Line([string]$file, [string]$pattern, [int]$timeout) {
        $deadline = (Get-Date).AddSeconds($timeout)
        while ((Get-Date) -lt $deadline) {
            Start-Sleep 1
            if (Get-Content $file -ErrorAction SilentlyContinue | Select-String -Pattern $pattern) { return $true }
        }
        return $false
    }

    Get-Process -Name mir2_server, client_bevy -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep 2
    $srv1 = Start-TestServer 'boot'
    Write-Host ("[A] 服务端已起 pid={0}" -f $srv1.Id)
    for ($i = 0; $i -lt 40; $i++) { Start-Sleep 1; if (Get-NetTCPConnection -LocalPort 7000 -State Listen -EA SilentlyContinue) { break } }
    if (-not (Get-NetTCPConnection -LocalPort 7000 -State Listen -EA SilentlyContinue)) {
        Write-Host '前置失败：服务端 40s 内未监听 7000'
        exit 2
    }

    $cli = Start-Process -FilePath $clientExe -WorkingDirectory (Join-Path $ClientHome 'Client-Bevy') `
        -ArgumentList @('--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass, '--reconnect-test') `
        -RedirectStandardOutput (Join-Path $work 'client.out.log') -RedirectStandardError $cliLog `
        -PassThru -WindowStyle Hidden
    Write-Host ("[A] 客户端已起 pid={0}" -f $cli.Id)

    $entered = Wait-Line $cliLog '\[RECON\] 已进入游戏' $EnterTimeoutSec
    if (-not $entered) {
        Write-Host ("[A] FAIL：{0}s 内未进入游戏" -f $EnterTimeoutSec)
        Get-Content $cliLog -Tail 5 -ErrorAction SilentlyContinue
        exit 2
    }
    Write-Host '[B] 客户端已进图 → 杀掉服务端（模拟重启/闪断）'
    Get-Process -Name mir2_server -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep $OutageSec
    $srv2 = Start-TestServer 'restart'
    Write-Host ("[B] 服务端已重启 pid={0}（停机 {1}s）" -f $srv2.Id, $OutageSec)

    $recovered = Wait-Line $cliLog '\[RECON\] ✅ 自动重连成功并重新进入游戏' $RecoverTimeoutSec
    Start-Sleep 1
    $recon = @(Get-Content $cliLog -ErrorAction SilentlyContinue | Select-String -Pattern '\[RECON\]|自动重连' |
        ForEach-Object { $_.Line })
    Write-Host '--- RECON 判定行 ---'
    $recon | ForEach-Object { Write-Host ("    " + $_) }

    $sawDisconnect = @($recon | Where-Object { $_ -match '检测到断线' }).Count -gt 0
    $ok = $entered -and $sawDisconnect -and $recovered
    Write-Host ("VERDICT enter_game={0} saw_disconnect={1} auto_reconnect={2}" -f `
            $(if ($entered) { 'PASS' } else { 'FAIL' }),
        $(if ($sawDisconnect) { 'PASS' } else { 'FAIL' }),
        $(if ($recovered) { 'PASS' } else { 'FAIL' }))
    if (-not $ok) { exit 5 }
} finally {
    Get-Process -Name client_bevy, mir2_server -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Exit-E2eLock
}
