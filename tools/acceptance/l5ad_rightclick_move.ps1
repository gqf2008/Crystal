# l5ad_rightclick_move.ps1 —— 真机「鼠标控制人物移动」端到端（owner 反馈③）
#
# 原版语义：**右键点空地 = 寻路移动**；左键 = 交互（点空地不动）。
# 两个判据：
#   ① 正路：右键点空地 → 人物**真的移动**（tile 变化 ≥2 格）且**到位后不被拉回**（3s 内停在终点附近）；
#   ② 负对照：在**同一个点**用左键点 → 不得移动（证明"能走"来自右键分流，而不是任何点击都会走）。
# 依赖：客户端 control RPC 的 `click` 支持 `button`（master f02f9086 起）；服务端在 7000。
param(
    [string]$User = 'bevychar',
    [string]$Pass = '123456',
    [string]$ClientHome = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend',
    [int]$MaxTries = 6
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = $PSScriptRoot
. "$acc\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5ad_rightclick_move' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }
$clientProc = $null
try {
    if (-not (Get-NetTCPConnection -LocalPort 7000 -State Listen -EA SilentlyContinue)) { Write-Host 'FAIL(9): 7000 上没有服务端'; exit 9 }
    function Rpc([string]$m, [hashtable]$q = @{}) {
        $c = New-Object Net.Sockets.TcpClient
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream()
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        ($l | ConvertFrom-Json).result
    }
    $exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
    if (-not (Test-Path $exeSrc)) { Write-Host "FAIL(9): 找不到客户端 $exeSrc"; exit 9 }
    $exe = Join-Path (Split-Path -Parent $exeSrc) 'l5ad_client.exe'
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5ad_rightclick_move'
    Get-CimInstance Win32_Process -Filter "Name='l5ad_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
    catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
    $out = "$env:TEMP\l5ad.out.log"; $err = "$env:TEMP\l5ad.err.log"
    Remove-Item $out, $err -Force -EA SilentlyContinue
    $clientProc = Start-Process -FilePath $exe -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass `
        -WorkingDirectory "$ClientHome\Client-Bevy" -PassThru -RedirectStandardOutput $out -RedirectStandardError $err

    # 等进图（state 有 tile_x）
    $st = $null
    foreach ($i in 1..90) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
    if ($null -eq $st -or $null -eq $st.tile_x) {
        Write-Host ("FAIL(9): 90s 未进图（日志尾：" + ((Get-Content $err -Tail 3 -EA SilentlyContinue) -join ' | ') + "）")
        exit 9
    }
    Write-Host ("[前置] map={0} start=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)
    # 屏幕坐标：客户端把玩家画在画面中心附近；48px/格 ⇒ 150px ≈ 3 格。
    $centerX = 512; $centerY = 384
    $offsets = @(@(150, 0), @(-150, 0), @(0, 150), @(0, -150), @(150, 150), @(-150, -150))

    $fail = @()
    $movedBy = $null
    $usedOffset = $null
    for ($k = 0; $k -lt [Math]::Min($MaxTries, $offsets.Count); $k++) {
        $o = $offsets[$k]
        $sx = [int]$st.tile_x; $sy = [int]$st.tile_y
        $pt = @{ x = $centerX + $o[0]; y = $centerY + $o[1]; button = 'right' }
        $reply = Rpc 'click' $pt
        Start-Sleep -Milliseconds 1500
        $after = Rpc 'state'
        if ($null -eq $after.tile_x) { continue }
        $d = [Math]::Abs([int]$after.tile_x - $sx) + [Math]::Abs([int]$after.tile_y - $sy)
        Write-Host ("[right] offset=({0},{1}) click={2} → tile=({3},{4}) 位移={5}" -f $o[0], $o[1], ($reply | ConvertTo-Json -Compress), $after.tile_x, $after.tile_y, $d)
        if ($d -ge 2) { $movedBy = $d; $usedOffset = $o; $startX = $sx; $startY = $sy; break }
    }
    if (-not $usedOffset) { $fail += '正路：六个方向的右键点空地都没让人物移动' }

    if ($usedOffset) {
        # 到位后 3s 采样：不该回到起点（橡皮筋）
        $samples = @()
        foreach ($i in 1..8) { Start-Sleep -Milliseconds 400; $s = Rpc 'state'; $samples += "($($s.tile_x),$($s.tile_y))" }
        $last = Rpc 'state'
        $endD = [Math]::Abs([int]$last.tile_x - $startX) + [Math]::Abs([int]$last.tile_y - $startY)
        Write-Host ("[轨迹] " + ($samples -join ' '))
        Write-Host ("[判据] 右键盘位移={0} 格、末点距起点={1} 格" -f $movedBy, $endD)
        if ($endD -lt 2) { $fail += '正路：走起来又被拉回起点（橡皮筋）' }

        # 负对照：同一点改左键 —— 原版左键点空地不移动
        $before = Rpc 'state'
        $ptL = @{ x = $centerX + $usedOffset[0]; y = $centerY + $usedOffset[1]; button = 'left' }
        Rpc 'click' $ptL | Out-Null
        Start-Sleep -Milliseconds 1500
        $afterL = Rpc 'state'
        $dL = [Math]::Abs([int]$afterL.tile_x - [int]$before.tile_x) + [Math]::Abs([int]$afterL.tile_y - [int]$before.tile_y)
        Write-Host ("[left] 同一点左键 → tile=({0},{1}) 位移={2}" -f $afterL.tile_x, $afterL.tile_y, $dL)
        if ($dL -ge 2) { $fail += '负对照：左键点空地把人物也带走了（左右键没分流）' }
    }

    if ($fail.Count -gt 0) { foreach ($f in $fail) { Write-Host ("FAIL: " + $f) }; Write-Host 'VERDICT rightclick=FAIL leftcontrol=FAIL'; exit 1 }
    Write-Host 'VERDICT rightclick=PASS leftcontrol=PASS'
    exit 0
} finally {
    if ($clientProc -and -not $clientProc.HasExited) { Stop-Process -Id $clientProc.Id -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 500
    Get-CimInstance Win32_Process -Filter "Name='l5ad_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock
}