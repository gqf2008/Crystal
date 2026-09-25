# smoke_r5.ps1 — 上线前真机冒烟：登录→进图→移动→攻击→过门/传送→断线→重登（顶号路径）
# 前提：mir2_server 已在 7000 监听；火绒已加信任/暂停主动防御。
param([string]$User = 'test', [string]$Pass = '123456')
$ErrorActionPreference = 'Stop'
# 运行时需要 msys64 ucrt64 DLL（libdb/libglib/libstdc++），否则 0xC0000142 静默退出
# 客户端依赖 msys64/ucrt64 与 libpinyin 的 DLL：缺任一目录会以 0xC0000135 静默退出
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$shots = Join-Path $acc 'shots'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
$wd  = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy'
if (-not (Test-Path $shots)) { New-Item -ItemType Directory $shots | Out-Null }

function Rpc([string]$method, [hashtable]$params = @{}) {
    $c = New-Object Net.Sockets.TcpClient
    $c.Connect('127.0.0.1', 9000)
    $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$method; params=$params } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s)
    $line = $r.ReadLine(); $c.Close()
    if ($null -eq $line) { throw "control 无响应: $method" }
    ($line | ConvertFrom-Json).result
}
function Shot([string]$label) {
    $p = (Join-Path $shots "$label.png").Replace('\', '/')
    Rpc 'screenshot' @{ path = $p } | Out-Null
    Start-Sleep -Milliseconds 900
    "  📸 $label"
}
function Wait-Game([int]$sec = 40) {
    foreach ($i in 1..$sec) {
        Start-Sleep 1
        try { $s = Rpc 'state'; if ($null -ne $s.tile_x) { return $s } } catch {}
    }
    throw '未进入游戏（state 无玩家坐标）'
}

Get-Process client_bevy -EA SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800

# ── 阶段 1：登录 + 进图 ─────────────────────────────
Write-Host '[1] 启动客户端（--real-net --auto-enter）'
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\smoke_client.log" -RedirectStandardError "$acc\smoke_client.err.log" -PassThru
Start-Sleep 6
if ((Get-Process -Id $proc.Id -EA SilentlyContinue) -eq $null) { throw '客户端启动后即死（火绒？）' }
Shot 'r5_01_login'
$st = Wait-Game
Write-Host ("  ✅ 进图 tile=({0},{1}) dir={2}" -f $st.tile_x, $st.tile_y, $st.direction)
Shot 'r5_02_game'

# ── 阶段 2：移动 ────────────────────────────────────
Write-Host '[2] 移动（向右跑 6 步）'
$x0 = $st.tile_x
foreach ($i in 1..6) { Rpc 'move' @{ dx=1; dy=0; run=$true } | Out-Null; Start-Sleep -Milliseconds 350 }
Start-Sleep 2
$st2 = Rpc 'state'
Write-Host ("  tile: ({0},{1}) -> ({2},{3})" -f $st.tile_x, $st.tile_y, $st2.tile_x, $st2.tile_y)
if ($st2.tile_x -eq $st.tile_x -and $st2.tile_y -eq $st.tile_y) { Write-Host '  ⚠️ 坐标未变（可能被挡）' } else { Write-Host '  ✅ 移动生效' }
Shot 'r5_03_moved'

# ── 阶段 3：攻击最近怪物 ────────────────────────────
Write-Host '[3] 攻击（nearby 找怪）'
$nb = Rpc 'nearby'
$mob = $nb.entities | Where-Object { $_.creature -eq 'monster' } | Select-Object -First 1
if ($mob) {
    Write-Host ("  目标: {0} id={1} dist={2}" -f $mob.name, $mob.id, $mob.dist)
    Rpc 'attack' @{ object_id = [uint64]$mob.id } | Out-Null
    Start-Sleep 6
    Shot 'r5_04_attack'
    Write-Host '  ✅ 攻击指令已发（战斗 6s，截图）'
} else { Write-Host '  ⚠️ 附近无怪，跳过攻击' }

# ── 阶段 4：同图传送（GM @move <x> <y>，走 teleport_core）────
# 前置：accounts.admin_account=1（GM）；无权限时服务端回"你没有权限使用此命令"
Write-Host '[4] 同图传送（聊天 @move 300 300 → 回原位）'
$stA = Rpc 'state'
Rpc 'chat' @{ message = "@move 300 300" } | Out-Null
Start-Sleep 3
$st4 = Rpc 'state'
if ($st4.tile_x -eq 300 -and $st4.tile_y -eq 300) { Write-Host '  ✅ 传送生效（客户端已同步）' } else { Write-Host "  ⚠️ 传送未生效 tile=($($st4.tile_x),$($st4.tile_y))（GM 权限？）" }
Shot 'r5_05_teleport'
Rpc 'chat' @{ message = "@move $($stA.tile_x) $($stA.tile_y)" } | Out-Null
Start-Sleep 2

# ── 阶段 5：断线（杀进程）→ 服务端应干净清理 ────────
Write-Host '[5] 杀客户端进程（测断线清理）'
Stop-Process -Id $proc.Id -Force
Start-Sleep 4

# ── 阶段 6：重登同账号（测顶号/自动注册 is_online 路径）──────────
Write-Host '[6] 重登同账号（dup-kick/unbind 实机路径）'
$proc2 = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\smoke_client2.log" -RedirectStandardError "$acc\smoke_client2.err.log" -PassThru
try {
    $st6 = Wait-Game 45
    Write-Host ("  ✅ 重登进图 tile=({0},{1})" -f $st6.tile_x, $st6.tile_y)
    Shot 'r5_06_relogin'
} finally {
    Stop-Process -Id $proc2.Id -Force -EA SilentlyContinue
}
Write-Host '== 冒烟完成 =='
