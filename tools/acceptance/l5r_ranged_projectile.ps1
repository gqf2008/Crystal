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
$ErrorActionPreference = 'Continue'
# 客户端链了 libpinyin DLL：PATH 不带这两个目录时进程会**静默退出**（本会话踩过）
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
$log = "$PSScriptRoot\l5r_client.err.log"

Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
if (Test-Path $log) { Remove-Item $log -Force }
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
