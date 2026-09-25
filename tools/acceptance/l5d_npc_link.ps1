# l5d_npc_link.ps1 — ⑤ 共同前置：NPC 窗链接点击 A/B
#
# 判据链（缺一不可）：
#   A) npc_call [@MAIN] 后截图里能看到完整正文 + 可点行（前置：服务端 NPC 脚本段解析已修）
#   B) cursor 注入到链接行 → 该行颜色应变（悬停高亮）——证明注入光标能到达 NPC 行命中逻辑
#   C) click 同点 → 客户端应发出 CallNPC[@Storage]，storage_probe.total 非 0
#
# A 过关但 B/C 不过 = 注入光标没进 window.cursor_position()（夹具/探针缺口，须修）

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5d_npc_link' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5d_client.exe'
function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Shot([string]$n) { Rpc 'screenshot' @{ path = "$acc\player_shots\l5d_$n.png" } | Out-Null; Start-Sleep -Milliseconds 700 }

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='l5d_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
# 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy' `
    -RedirectStandardOut "$acc\l5d_client.log" -RedirectStandardError "$acc\l5d_client.err.log" -PassThru
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)

# 走到仓库 NPC 旁（D002 @ 174,216 已由 MirDB 记录 + 上轮 nearby 实测确认）
Rpc 'chat' @{ message = '@mapmove D002 174 217' } | Out-Null
Start-Sleep 4
$near = Rpc 'nearby' @{ radius = 2000 }
$npc = $near.entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
if (-not $npc) { Write-Host 'FAIL: nearby 里没有 NPC'; exit 1 }
Write-Host ("NPC={0} id={1} dist={2}" -f $npc.name, $npc.object_id, $npc.dist)
Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
Start-Sleep 2
Shot '1_npc'

# A) NPC 窗文本状态 + 行内链接的**精确命中矩形**（与客户端点击分发同源几何）
$rows = Rpc 'npc_rows'
Write-Host ("npc_rows: visible={0} lines={1} npc_object_id={2}" -f $rows.visible, $rows.lines.Count, $rows.npc_object_id)
if ($rows.npc_object_id -eq 0) {
    # 客户端没记住当前 NPC → 选项点击会发 CallNPC{object_id:0}，服务端 `NPC call for unknown object_id 0` 丢弃
    Write-Host 'FAIL: npc_object_id=0（开窗时没记住当前 NPC）'
    exit 3
}
foreach ($l in $rows.links) {
    Write-Host ("  link {0} row={1} rect=({2},{3})-({4},{5}) center=({6},{7})" -f `
        $l.key, $l.row, $l.x0, $l.y0, $l.x1, $l.y1, $l.cx, $l.cy)
}
$link = $rows.links | Where-Object { $_.key -eq '[@Storage]' } | Select-Object -First 1
if (-not $link) { Write-Host 'FAIL: 对话框里没有 [@Storage] 链接'; exit 2 }

# B) 悬停：注入光标到该链接中心 → 该行应变色（探针确实进了 NPC 行命中逻辑）
Rpc 'cursor' @{ x = $link.cx; y = $link.cy } | Out-Null
Start-Sleep -Milliseconds 900
Shot '2_hover'

# C) 点击链接中心
$hit = Rpc 'click' @{ x = $link.cx; y = $link.cy }
Start-Sleep -Milliseconds 1800
Shot '3_click'
$sp = Rpc 'storage_probe'
Write-Host ("click hits = {0}" -f ($hit.hits -join ' | '))
Write-Host ("storage_probe = {0}" -f ($sp | ConvertTo-Json -Compress))

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5d_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
