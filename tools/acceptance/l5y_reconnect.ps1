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
    # 唯一进程名（同批次 #3181 的 20 个夹具）：只启动/清理自己这份改名的客户端副本（硬链接，不占额外磁盘）。
$clientSrc = $clientExe
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置（在改名/复制之前验源产物）
Assert-ClientBuildStamp -Exe $clientSrc -ScriptName 'l5y_reconnect'
    $clientExe = Join-Path (Split-Path -Parent $clientExe) 'l5y_client.exe'

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

    # 清场只清**自己的**：本夹具的 exe 路径（服务端）+ 自己那份唯一命名的客户端副本。
    # 绝不按公共名清场——同机可能有别人的服务端（7000 常驻开发服）与别的 agent 的客户端。
    Get-CimInstance Win32_Process -Filter "Name='mir2_server.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $serverExe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Get-CimInstance Win32_Process -Filter "Name='l5y_client.exe'" -ErrorAction SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Start-Sleep 2
    # 7000 被**别人的**实例占着就明确前置失败（以前是靠"按名杀全场"顺手清掉，那会误杀共享开发服）
    $occupier = Get-NetTCPConnection -LocalPort 7000 -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($occupier) {
        $occExe = (Get-CimInstance Win32_Process -Filter ("ProcessId=" + $occupier.OwningProcess) -ErrorAction SilentlyContinue).ExecutablePath
        if ($occExe -and $occExe -ne $serverExe) {
            Write-Host ("前置失败：7000 已被别的实例占用（pid={0} exe={1}）——本夹具要在 7000 上起自己的服务端；" -f $occupier.OwningProcess, $occExe)
            Write-Host '          请先停掉它（例如共享开发服），跑完再按原样重启；本夹具不会替你杀别的进程。'
            exit 2
        }
    }
    try { New-Item -ItemType HardLink -Path $clientExe -Target $clientSrc -Force -ErrorAction Stop | Out-Null }
    catch { Copy-Item -LiteralPath $clientSrc -Destination $clientExe -Force }
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
    # 故障注入：杀掉**本次自己起的**那个服务端实例（按 PID），不按公共名清场
    if ($srv1 -and -not $srv1.HasExited) { Stop-Process -Id $srv1.Id -Force -ErrorAction SilentlyContinue }
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
    # 收尾：只清自己的（唯一命名客户端 + 本次自己的服务端 PID；再按自己的 exe 路径兜底扫一遍）
    foreach ($p in @($srv1, $srv2)) {
        if ($p -and -not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    }
    Get-CimInstance Win32_Process -Filter "Name='l5y_client.exe'" -ErrorAction SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Get-CimInstance Win32_Process -Filter "Name='mir2_server.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.ExecutablePath -eq $serverExe } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Exit-E2eLock
}
