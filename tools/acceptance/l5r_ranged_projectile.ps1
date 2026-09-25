# l5r_ranged_projectile.ps1 — 远程攻击弹道用**原版帧表**的实机验收
# （C# `Client/MirObjects/PlayerObject.cs` MirAction.AttackRange1/2/3 的 CreateProjectile）
#
# 判据（取客户端日志真值，不解析像素）：
#   A) 客户端进入 Game 态并发出 C.RangeAttack（`🏹 [RANGETEST] 发送 RangeAttack`）
#   B) mock 回 `S.ObjectRangeAttack` / `S.RangeAttack` 后，本端**用原版帧表**生成弹道：
#      日志出现 `🏹 远程攻击弹道=原版 Magic3[1030] ×5帧`
#   C) 不再出现「表未覆盖，退回占位弹道」的降级日志（spell=0 必须命中 DefaultArrow）
#
# 为什么跑 mock：`--auto-ranged-attack` 固定打目标 101（真服没有该目标就不回包），
# mock 会按同一套包结构回 ObjectRangeAttack + RangeAttack，用来验**客户端侧**接线。
#
# 退出码：0 = 全 PASS；10 = 判据未达成
param(
    [string]$ClientHome = '',
    [int]$WaitSeconds = 45
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5r_ranged_projectile' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
# 客户端链了 libpinyin DLL：PATH 不带这两个目录时进程会**静默退出**（本会话踩过）
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName 'l5r_ranged_projectile'
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5r_client.exe'
$log = "$PSScriptRoot\l5r_client.err.log"

Get-CimInstance Win32_Process -Filter "Name='l5r_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
if (Test-Path $log) { Remove-Item $log -Force }
# 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
Start-Process -FilePath $exe -ArgumentList '--auto-ranged-attack','--skip-login' `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$PSScriptRoot\l5r_client.log" -RedirectStandardError $log | Out-Null

$sent = $false; $original = $false; $fallback = $false
for ($i = 1; $i -le $WaitSeconds; $i++) {
    Start-Sleep -Seconds 1
    if (-not (Test-Path $log)) { continue }
    $t = Get-Content $log -Raw
    if ($t -match '\[RANGETEST\] 发送 RangeAttack') { $sent = $true }
    if ($t -match '远程攻击弹道=原版 Magic3\[1030\] ×5帧') { $original = $true }
    if ($t -match '表未覆盖，退回占位弹道') { $fallback = $true }
    if ($original) { break }
}

Write-Host ("[A] 发出 C.RangeAttack → {0}" -f $(if ($sent) { 'PASS' } else { 'FAIL' }))
Write-Host ("[B] 弹道用原版帧表 Magic3[1030]×5 → {0}" -f $(if ($original) { 'PASS' } else { 'FAIL' }))
Write-Host ("[C] 未退回占位弹道 → {0}" -f $(if (-not $fallback) { 'PASS' } else { 'FAIL' }))
if ($original -and -not $fallback) {
    Select-String -Path $log -Pattern 'RANGETEST|远程攻击弹道' | Select-Object -Last 4 | ForEach-Object { Write-Host ('    ' + $_.Line) }
    Write-Host '=== 全部 PASS ==='
    exit 0
}
Write-Host '=== 有 FAIL ==='
exit 10

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5r_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
