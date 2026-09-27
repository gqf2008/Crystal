# l5ae_move_trace.ps1 — 一次右键走路 + 客户端/服务端位置对账轨迹（owner 反馈「走一段被拉回原处」取证）
#
# 判据（全取状态读数，不看像素）：
#   A) 一次右键远点后客户端 tile 位移 >= 2 格（能走）；
#   B) 采样期间 `state.in_sync` 不得出现 false（客户端预测领先服务端 >2 格）；
#   C) 轨迹里客户端 tile 不得**回退**到更早的采样点（= 被拉回）。
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
Rpc 'click' @{ x = [int]$st.tile_x + 150; y = [int]$st.tile_y; button = 'right' } | Out-Null
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
Write-Host ("[判据] 末端位移={0} 格；采样中 in_sync 曾 false={1}；轨迹出现回退={2}" -f $d, $anyOut, $back)
Write-Host ("VERDICT move={0} sync={1} no_pullback={2}" -f `
    $(if ($d -ge 2) { 'PASS' } else { 'FAIL' }), $(if ($anyOut) { 'FAIL' } else { 'PASS' }), $(if ($back) { 'FAIL' } else { 'PASS' }))
if (-not ($d -ge 2 -and -not $anyOut -and -not $back)) { exit 10 }
exit 0
} finally {
    Get-CimInstance Win32_Process -Filter "Name='l5ae_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock
}
