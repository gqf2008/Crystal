# l5p_mount_portrait.ps1 — 坐骑立绘（C# `MountDialog.MountImage`）实机验收
# （owner 队列 `mount-image-preview`）
#
# 原版口径（`Client/MirScenes/Dialogs/MountDialog.cs`）：
#   `MountImage = MirAnimatedControl { AnimationCount=16, AnimationDelay=100, Loop=true,
#   UseOffSet=true }`（`:80-91`），`Index = StartIndex + MountType*20`（`:213`，
#   `MountType` = **装备中坐骑物品的 Shape**，`UserObject.cs:249`）；`StartIndex` 与位置按孔数：
#   4 孔 1170 @(110,250)、5 孔 1330 @(0,70)（`:165-194`）；`UseOffSet` ⇒ 绘制点 = Location + 艺术偏移。
#
# 判据（取状态真值，不解析像素）：
#   A) 开坐骑窗 → 客户端日志出现「坐骑」名（说明装备读取成功），且截图里立绘出现在面板内
#   B) 两次相隔 ~400ms 的截图里立绘区域像素**不同**（16 帧 100ms 循环 ⇒ 一定在动）
#   C) 负对照：把立绘窗口关掉（关对话框）后，同一区域与打开态不同（说明立绘只在该窗里画）
#
# 说明：本夹具用「截图像素差」判定**动画在动**，因为客户端当前没有立绘几何探针；
# 几何对齐（帧号 StartIndex+Shape*20、位置 (0,70)/(110,250)、艺术偏移）由单元门禁
# `mount_portrait_matches_csharp` 钉住。
#
# **前置数据（本机 e2e 库已设好）**：需要**4 孔坐骑**——立绘帧 1170..1185 在本仓
# `Crystal/Data/Prguse.Lib` 里是真图（实测 212x470、offset (-86,-106)），而 5 孔档的
# 1330..1345 是 **w=0 h=0 的空帧**（原版在这份美术下同样画不出立绘）。
# 设法：把 `inventory_equipment` 里 bevychar slot=10 的 `item_json.slots` 截成 4 项
# （本会话 `_mount4slot.py` 就是这么做的），再重启服务端让客户端重新读到装备。
#
# 退出码：0 = 全 PASS；10 = 判据未达成；9 = 服务端/客户端未就绪
param(
    [string]$User = 'test',
    [string]$Pass = '123456',
    [string]$ClientHome = '',
    [switch]$NoRestart
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5p_mount_portrait' -TimeoutSec 1800)) { exit 2 }
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$acc = "$PSScriptRoot"
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 5000; $c.SendTimeout = 5000
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream(); $s.ReadTimeout = 5000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { Write-Host ("  [Rpc $m 失败] " + $_.Exception.Message); return $null }
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
if (-not $NoRestart) {
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" `
        -RedirectStandardOut "$acc\l5p_client.log" -RedirectStandardError "$acc\l5p_client.err.log" | Out-Null
}
$st = $null
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
if ($null -eq $st -or $null -eq $st.tile_x) { Write-Host '客户端未进场'; exit 9 }
Write-Host ("[前置] 进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

Rpc 'dialog' @{ kind = 'mount'; action = 'open' } | Out-Null
Start-Sleep -Milliseconds 1200
# A) 日志里应有坐骑名（`mount_ui_system` 每次开窗首帧打印）
$log = "$acc\l5p_client.err.log"
$mountLines = @(Select-String -Path $log -Pattern '🐴 坐骑' -EA SilentlyContinue)
Write-Host ("[A] 客户端日志『坐骑』行 {0} 条，末条：{1}" -f $mountLines.Count, ($mountLines | Select-Object -Last 1).Line)
$okA = ($mountLines.Count -gt 0)

$shot1 = "$acc\l5p_mount_1.png"
$shot2 = "$acc\l5p_mount_2.png"
Rpc 'screenshot' @{ path = '../tools/acceptance/l5p_mount_1.png' } | Out-Null
Start-Sleep -Milliseconds 450
Rpc 'screenshot' @{ path = '../tools/acceptance/l5p_mount_2.png' } | Out-Null
Start-Sleep -Milliseconds 600

$verdict = $okA
Write-Host ("[A] 坐骑窗已开且读到装备 → {0}" -f $(if ($okA) { 'PASS' } else { 'FAIL' }))
# B) 两帧截图差异（动画在动）——用 PIL 比较，比对的是**坐骑立绘区域**（面板 (10,30) 起 324x377）
$py = @'
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print("size-mismatch")
    sys.exit(0)
# 截图是缩放过的：按宽度比例把游戏坐标 (10,30,324,377) 映射过去
sx = a.size[0] / 1024.0
box = (int(10*sx), int(30*sx), int((10+324)*sx), int((30+377)*sx))
diff = ImageChops.difference(a.crop(box), b.crop(box))
bbox = diff.getbbox()
changed = sum(1 for p in diff.getdata() if p != (0,0,0))
print("bbox=%s changed=%d" % (bbox, changed))
'@
$py | Set-Content -Encoding utf8 "$acc\_l5p_diff.py"
$diffOut = py -3.12 "$acc\_l5p_diff.py" $shot1 $shot2 2>&1 | Select-Object -Last 1
Write-Host ("[B] 立绘区域两帧差异：{0}" -f $diffOut)
$okB = ("$diffOut" -match 'changed=(\d+)' -and [int]$Matches[1] -gt 200)
Write-Host ("[B] 16 帧动画在动（差异像素 > 200）→ {0}" -f $(if ($okB) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okB

# C) 关窗后同区域不再有立绘（与开窗态不同）
Rpc 'dialog' @{ kind = 'mount'; action = 'close' } | Out-Null
Start-Sleep -Milliseconds 800
$shot3 = "$acc\l5p_mount_closed.png"
Rpc 'screenshot' @{ path = '../tools/acceptance/l5p_mount_closed.png' } | Out-Null
Start-Sleep -Milliseconds 400
$diffOut2 = py -3.12 "$acc\_l5p_diff.py" $shot1 $shot3 2>&1 | Select-Object -Last 1
Write-Host ("[C] 开窗态 vs 关窗态 同区域差异：{0}" -f $diffOut2)
$okC = ("$diffOut2" -match 'changed=(\d+)' -and [int]$Matches[1] -gt 200)
Write-Host ("[C] 立绘只画在坐骑窗里 → {0}" -f $(if ($okC) { 'PASS' } else { 'FAIL' }))
$verdict = $verdict -and $okC

Remove-Item -LiteralPath "$acc\_l5p_diff.py" -Force -EA SilentlyContinue
Write-Host '截图：l5p_mount_1.png / l5p_mount_2.png / l5p_mount_closed.png'
Write-Host ("VERDICT: {0}" -f $(if ($verdict) { 'PASS（坐骑立绘已渲染且在动）' } else { 'FAIL' }))
if (-not $verdict) { exit 10 }
exit 0
