#Requires -Version 5.1
<#
.SYNOPSIS
  `#2961` 项①「坐骑遮挡半透明」的**实机**判据：只读探针 `mount_layer_probe`。

.DESCRIPTION
  原版语义（`Client/MirObjects/PlayerObject.cs:5001-5050`）：`Draw()` 里
  `if (Hidden && !DXManager.Blending) DXManager.SetOpacity(0.5F);` —— 该 opacity 覆盖
  `DrawMount()` → `DrawWeapon()` → `DrawBody()` → `DrawHead()` → `DrawWings()` 整段，
  即**被遮挡时坐骑与身体同一个 0.5**；`RidingMount` 时坐骑先画（`DrawMount()` 在 `DrawBody()` 之前）。

  本端实现是"残影副本"（`update_local_ghost`：遮挡时把 `GhostLayer` 翻 Visible、镜像对应
  `SpriteLayer` 的当前帧图、alpha 取 `GHOST_ALPHA`）。判据因此落在两条：
    J1 遮挡态成立：`mount_layer_probe.occluded == true`（**同状态连读两次必须一致**＝仪器自检，
       探针内部按字符串排序保证确定性）；
    J2 半透明一致：**每个可见 ghost 层的 alpha 必须 == `csharp_ghost_alpha`(=0.5)**，
       且**坐骑 ghost 与身体 ghost 同值**（骑乘时：`is_mount=true` 的 ghost 必须存在且 alpha 相同）。

  阳性对照（实做）：把 `GHOST_ALPHA` 写回 1.0 → 单测 `ghost_drive_local_only_including_mount` 立即红
  （`实测 Some(1.0)`）；本夹具在实机上会表现为 J2 失败。

  已知边界（如实记）：mock 进场点（本机实测 `map=n0 tile=(354,352)`）本身就在遮挡态，
  所以 J1/J2 可稳定复验；但 mock 不骑乘，**"坐骑 ghost"这一支目前由单测覆盖**
  （`ghost_tests::ghost_drive_local_only_including_mount` 用真图层断言坐骑 ghost 同 alpha 同图），
  实机骑乘版证据是 `tools/acceptance/UI_VERIFICATION_REPORT.md` §10 的截图
  （`@make LeatherBridle`+`Saddle` → `@ride` → 走过树冠：骑手与坐骑以半透明残影压在树叶之上）。

.EXAMPLE
  pwsh tools/acceptance/l5y_mount_ghost.ps1
#>
param(
    [string]$ClientExe = '',
    [string]$Worktree = '',
    [int]$ControlPort = 9071,
    [int]$TimeoutSec = 60,
    [string]$Tag = 'mountghost'
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $Worktree) { $Worktree = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not $ClientExe) { $ClientExe = Join-Path $Worktree 'Client-Bevy\target\debug\client_bevy.exe' }
$root = 'C:\Users\gxh\AppData\Local\Temp\orig-csharp-ab'
New-Item -ItemType Directory -Force -Path $root | Out-Null
$exe = Join-Path $root "$Tag`_client.exe"
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5y_mount_ghost'
$json = Join-Path $PSScriptRoot 'l5y_mount_ghost_results.json'

# 实机资源串行（与其它实机夹具共用同一把锁）
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5y_mount_ghost' -TimeoutSec 900)) {
    Write-Host 'FAIL(2): 等 e2e 锁超时'
    exit 2
}

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 2000; $c.SendTimeout = 2000
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 2000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $line = $r.ReadLine(); $c.Close()
        if (-not $line) { return $null }
        ($line | ConvertFrom-Json).result
    } catch { return $null }
}

if (-not (Test-Path $ClientExe)) { Write-Host "FAIL(2): 缺少客户端产物 $ClientExe"; Exit-E2eLock; exit 2 }
Get-CimInstance Win32_Process -Filter "Name='$Tag`_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Copy-Item -LiteralPath $ClientExe -Destination $exe -Force
$err = Join-Path $root "$Tag.err"
$proc = Start-Process -FilePath $exe -WorkingDirectory 'E:\Users\gxh\Documents\GitHub\Crystal' -PassThru `
    -ArgumentList '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', '--control-port', "$ControlPort" `
    -RedirectStandardOutput (Join-Path $root "$Tag.log") -RedirectStandardError $err

$fail = @()
try {
    $st = $null
    for ($i = 1; $i -le $TimeoutSec; $i++) {
        Start-Sleep 1
        $st = Rpc 'state'
        if ($st -and $null -ne $st.tile_x) { break }
        if (-not (Get-Process -Id $proc.Id -EA SilentlyContinue)) { break }
    }
    if (-not $st -or $null -eq $st.tile_x) {
        Write-Host ("FAIL(2): 未进场（看 {0}）" -f $err)
        $fail += 'not_entered_game'
    } else {
        Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)
        $p1 = Rpc 'mount_layer_probe'
        $p2 = Rpc 'mount_layer_probe'
        $j1 = ($p1 | ConvertTo-Json -Compress -Depth 6)
        $j2 = ($p2 | ConvertTo-Json -Compress -Depth 6)
        Write-Host ("[J0 仪器自检] 同状态连读两次一致：{0}" -f ($j1 -eq $j2))
        if ($j1 -ne $j2) { $fail += '仪器自检失败：同状态两次读数不一致' }
        Write-Host ("读数：{0}" -f $j1)
        if ($null -eq $p1 -or -not $p1.ok) {
            $fail += 'mount_layer_probe 不可用（未编译进二进制？）'
        } else {
            $expect = [double]$p1.csharp_ghost_alpha
            if (-not $p1.occluded) {
                $fail += 'J1: 本轮进场点不在遮挡态（换一个遮挡位再跑；mock 进场点本机实测是遮挡态）'
            }
            $ghosts = @($p1.layers | Where-Object { $_.is_ghost -and $_.visible })
            if ($ghosts.Count -eq 0) { $fail += 'J2: 遮挡态下没有任何可见 ghost 层' }
            foreach ($g in $ghosts) {
                if ([math]::Abs([double]$g.alpha - $expect) -gt 0.001) {
                    $fail += ("J2: ghost({0}) alpha={1} ≠ 原版 {2}" -f $g.lib, $g.alpha, $expect)
                }
            }
            $mountGhosts = @($ghosts | Where-Object { $_.is_mount })
            if ($mountGhosts.Count -gt 0) {
                foreach ($mg in $mountGhosts) {
                    if ([math]::Abs([double]$mg.alpha - $expect) -gt 0.001) {
                        $fail += ("J2: 坐骑 ghost alpha={0} ≠ 身体同值 {1}" -f $mg.alpha, $expect)
                    }
                }
                Write-Host ("坐骑 ghost 层：{0}" -f (($mountGhosts | ForEach-Object { "$($_.lib) alpha=$($_.alpha)" }) -join '; '))
            } else {
                Write-Host '（本轮未骑乘：坐骑 ghost 支由单测 ghost_drive_local_only_including_mount 覆盖）'
            }
        }
    }
} finally {
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    Exit-E2eLock
}

$result = [ordered]@{
    ok      = ($fail.Count -eq 0)
    tag     = $Tag
    failures = $fail
}
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $json -Encoding UTF8
Write-Host ("结论 JSON: " + $json)
if ($fail.Count -gt 0) {
    Write-Host ('FAIL(1): ' + ($fail -join '; '))
    exit 1
}
Write-Host '=== 全部 PASS ==='
exit 0