# npc_text_verify.ps1 — PR #2952 实机复验：NPC 对话文字必须渲染（不再全黑）

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'npc_text_verify' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁
# （PowerShell 的 finally 在 exit 下也会执行——实测 -File 与会话内 & script.ps1 两种调用都成立），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Stop'
# 客户端依赖 msys64/ucrt64 与 libpinyin 的 DLL：缺任一目录会以 0xC0000135 静默退出
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
$wd  = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy'

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

# 只清**自己这份构建**的残留（按 exe 路径过滤）：绝不按公共名 `client_bevy` 清场——
# 那会连带杀掉别的 agent 的验收/人工 GUI 会话（BATCH #3181；`check_process_scope` 门禁会红）。
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    Where-Object { $_.ExecutablePath -eq $exe } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\npc_verify_client.log" -RedirectStandardError "$acc\npc_verify_client.err.log" -PassThru
$ok = $false
foreach ($i in 1..45) {
    Start-Sleep 1
    try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {}
}
if (-not $ok) { throw '未进入游戏' }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
Start-Sleep 2

# 走到铁匠旁（Merchant_Smith @ 297,612 map 1）
Rpc 'chat' @{ text = '@move 296 612' } | Out-Null
Start-Sleep 2
$st = Rpc 'state'
Write-Host ("移动后 tile=({0},{1})" -f $st.tile_x, $st.tile_y)

# 找铁匠 object_id 并呼叫 @main
$near = Rpc 'nearby'
 $smith = $near.entities | Where-Object { $_.name -match 'Smith' } | Select-Object -First 1
if (-not $smith) { Write-Host 'nearby:'; $near | ConvertTo-Json -Depth 5; throw '附近没找到铁匠' }
Write-Host ("铁匠 id={0} name={1} @({2},{3})" -f $smith.object_id, $smith.name, $smith.x, $smith.y)
Rpc 'npc_call' @{ object_id = $smith.object_id; key = '[@MAIN]' } | Out-Null
Start-Sleep 2

$d = (Rpc 'dialogs').dialogs
Write-Host "开着的窗口: $($d -join ',')"
Rpc 'screenshot' @{ path = "$acc/shots/npc_text_fixed.png" } | Out-Null
Start-Sleep 1
Stop-Process -Id $proc.Id -Force
Write-Host '== 截图已存 npc_text_fixed.png =='

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
