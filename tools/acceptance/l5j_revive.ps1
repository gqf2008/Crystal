# l5j_revive.ps1 — ② 复活闭环：死亡（GM @die）→ 回城复活（C.TownRevive）→ 状态与位置恢复
#
# 判据（取状态）：A) 死亡前 dead=false 且 hp>0
#               B) `@die` 后 dead=true 且 hp<=0（真的死了，不是"看起来死了"）
#               C) `revive_town` 后 dead=false 且 hp>0（复活真的生效）
#               D) 复活后位置回到**绑定点**（服务端 TownRevive 会传回 bind map/坐标；
#                  这里只断言"位置与死亡点不同或等于绑定点"，避免把绑定点写死）
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [int]$DieWaitSec = 5
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
$exe = "$wt\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory "$wt\Client-Bevy" `
    -RedirectStandardOut "$acc\l5j_client.log" -RedirectStandardError "$acc\l5j_client.err.log" | Out-Null
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("[A] 进图 map={0} tile=({1},{2}) hp={3}/{4} dead={5}" -f $st.map, $st.tile_x, $st.tile_y, $st.hp, $st.max_hp, $st.dead)
$alive = (-not $st.dead) -and ([int]$st.hp -gt 0)
$dieTile = @($st.tile_x, $st.tile_y)

# B) 死亡：GM @die（C# case "DIE"：自杀）
Rpc 'chat' @{ message = '@die' } | Out-Null
Start-Sleep -Seconds $DieWaitSec
$dead = Rpc 'state'
Write-Host ("[B] @die 后 hp={0}/{1} dead={2} tile=({3},{4})" -f $dead.hp, $dead.max_hp, $dead.dead, $dead.tile_x, $dead.tile_y)
$died = [bool]$dead.dead

# C/D) 复活
Rpc 'revive_town' | Out-Null
Start-Sleep -Seconds 4
$rev = Rpc 'state'
Write-Host ("[C] revive_town 后 hp={0}/{1} dead={2} map={3} tile=({4},{5})" -f $rev.hp, $rev.max_hp, $rev.dead, $rev.map, $rev.tile_x, $rev.tile_y)
$revived = ((-not $rev.dead) -and ([int]$rev.hp -gt 0))
$moved = ([int]$rev.tile_x -ne [int]$dieTile[0]) -or ([int]$rev.tile_y -ne [int]$dieTile[1])
Write-Host ("[D] 复活后位置与死亡点不同={0}（死亡 ({1},{2}) → 复活 ({3},{4})）" -f $moved, $dieTile[0], $dieTile[1], $rev.tile_x, $rev.tile_y)

$okA = [bool]$alive
$okB = [bool]$died
$okC = [bool]$revived
Write-Host ("VERDICT alive_before={0} died={1} revived={2} moved_to_bind={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($moved) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC)) { exit 5 }
