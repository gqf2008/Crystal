# npc_text_verify.ps1 — PR #2952 实机复验：NPC 对话文字必须渲染（不再全黑）
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

Get-Process client_bevy -EA SilentlyContinue | Stop-Process -Force
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
