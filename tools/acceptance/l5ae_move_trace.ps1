# l5ae_move_trace.ps1 — 一次右键走路 + 客户端/服务端位置对账轨迹（owner 反馈「走一段被拉回原处」取证）
#
# 判据（全取状态读数，不看像素）：
#   A) 一次右键远点后客户端 tile 位移 >= 2 格（能走）；
#   B) 轨迹里客户端 tile 不得**回退**（= 没被拉回；owner 原话就是「跑一段被拉回来」）；
#   C) 停下后 `state.in_sync` 必须收敛为 true（客户端与服务端最终一致，证明 B 不是靠"根本没动"糊过去的）。
#
# 为什么判据不是 `in_sync`：服务端**每成功走一步都回一发** UserLocation，那对客户端是落后一个 RTT 的
# 「回显 ACK」，移动期间客户端本来就该领先 ⇒ `in_sync=false` 是常态，不能当判据（旧版夹具把它当判据，
# 等于要求"客户端跟着落后的回显走"，反过来逼出「被拉回」）。现在 ACK 由 `UserLocation.correction=false`
# 标出、客户端不据此挪人（只有真校正才挪），判据因此落在「不回退 + 最终收敛」上。
# 阳性对照（实做）：把客户端 `apply_user_location` 改回"无条件写 self_position" → B 立刻红。
# 用法：pwsh tools/acceptance/l5ae_move_trace.ps1 -ClientHome <worktree> [-User test]
param(
    [string]$ClientHome = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend',
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$Port = 9102,
    [int]$Samples = 16,
    [int]$SampleMs = 300
)
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5ae_move_trace' -TimeoutSec 1800)) { exit 2 }
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5ae_move_trace'
$exe = Join-Path (Split-Path -Parent $exeSrc) 'l5ae_client.exe'
Get-CimInstance Win32_Process -Filter "Name='l5ae_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 600
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient; $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', $Port); $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass,
    '--control-port',"$Port" -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOutput "$env:TEMP\l5ae.out.log" -RedirectStandardError "$env:TEMP\l5ae.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st.tile_x) { Write-Host 'FAIL(前置): 客户端未进场'; exit 2 }
Write-Host ("[前置] map={0} start=({1},{2}) server=({3},{4}) in_sync={5}" -f `
    $st.map, $st.tile_x, $st.tile_y, $st.server_tile_x, $st.server_tile_y, $st.in_sync)
# 落到**已知开阔**的图上再走：角色可能停在副本里一格被封死的点上（实测 D003(351,233) 四周无可行路径），
# 那种点是「没路可走」而不是「被拉回」，会把判据 A 变成假红。
Rpc 'chat' @{ message = '@mapmove 1 330 48' } | Out-Null
Start-Sleep -Seconds 4
$st = Rpc 'state'
Write-Host ("[前置] mapmove 后 map={0} start=({1},{2}) server=({3},{4}) in_sync={5}" -f `
    $st.map, $st.tile_x, $st.tile_y, $st.server_tile_x, $st.server_tile_y, $st.in_sync)
# `walk_to` 与右键寻路走的是**同一条**本地寻路管线（LocalMove）；被拉回的机制在 `apply_self_position`，
# 与输入方式无关。
Rpc 'walk_to' @{ tx = [int]$st.tile_x + 8; ty = [int]$st.tile_y; run = $true } | Out-Null
$trail = @(); $anyOut = $false
foreach ($i in 1..$Samples) {
    Start-Sleep -Milliseconds $SampleMs
    $s = Rpc 'state'
    if ($null -eq $s) { continue }
    $trail += "($($s.tile_x),$($s.tile_y)|srv=$($s.server_tile_x),$($s.server_tile_y)|sync=$($s.in_sync))"
    if (-not $s.in_sync) { $anyOut = $true }
}
Write-Host ("[轨迹] {0}" -f ($trail -join ' '))
$end = Rpc 'state'
$d = [Math]::Abs([int]$end.tile_x - [int]$st.tile_x) + [Math]::Abs([int]$end.tile_y - [int]$st.tile_y)
$back = $false
$lastD = -1
foreach ($t in $trail) {
    if ($t -match '^\((\d+),(\d+)\|') {
        $dd = [Math]::Abs([int]$Matches[1] - [int]$st.tile_x) + [Math]::Abs([int]$Matches[2] - [int]$st.tile_y)
        if ($dd -lt $lastD) { $back = $true }
        $lastD = $dd
    }
}
$converged = $false
foreach ($i in 1..20) {
    Start-Sleep -Milliseconds 200
    $e = Rpc 'state'
    if ($null -ne $e -and $e.in_sync) { $converged = $true; break }
}
Write-Host ("[判据] 末端位移={0} 格；轨迹出现回退={1}；停止后 in_sync 收敛={2}（采样中曾 false={3}）" -f `
    $d, $back, $converged, $anyOut)
Write-Host ("VERDICT move={0} no_pullback={1} converge={2}" -f `
    $(if ($d -ge 2) { 'PASS' } else { 'FAIL' }), $(if ($back) { 'FAIL' } else { 'PASS' }), $(if ($converged) { 'PASS' } else { 'FAIL' }))
if (-not ($d -ge 2 -and -not $back -and $converged)) { exit 10 }
exit 0
} finally {
    Get-CimInstance Win32_Process -Filter "Name='l5ae_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock
}
