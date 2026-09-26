# l5r_rental_name.ps1 — P3-3（#782）**租赁窗半段**：租客侧「对方物品窗」显示的物品名是真名还是 `#id`？
#
# 为什么单开一条：`S.UpdateRentalItem` 是**裸 `write_to`**（不带 ItemInfo，
# `SharedRust/src/packets/server/rental_system.rs:230`），而租客侧对方物品窗的格子**直接**由它填充
# （`Client-Bevy/src/game/dialogs/item_rental.rs:958` 的 `RentalItemUpdate`）。其它 NO_INFO 包都被排除过
# （`GainedItem` 后面必跟完整 UserInformation；`RefreshItem`/`SplitItem` 的载荷被客户端丢弃；
#  `MarketListing`/`GuildStorageList` 客户端自带「线包名 → 本地表」降级），只剩这一处没判定。
#
# 判据（全部取状态读数，不看像素）：
#   R1) 租客侧 `rental_probe.partner_item` 非空 —— 对方物品窗真有物品（前置成立）
#   R2) 它的 `name` **不是** `#id` 占位（`placeholder == false`）—— 玩家所见是真名
#   R3) 物主侧 `rental_probe.deposit_item.name` 同样是真名（对照组）
#
# 驱动：两条真客户端各带自己的自动开关（`--rental-test` 物主 / `--rental-renter` 租客），
# 租客先起（物主 10s 后按**角色名** `bevy2char` 发起请求，目标必须在线）。
# 退出码：0 = R1∧R2∧R3；10 = 判据未达成；2 = 前置不满足（锁/构建戳/进场/会话未建立）
param(
    [string]$ClientHome = '',
    [string]$OwnerUser = 'test',
    [string]$OwnerPass = '123456',
    [string]$RenterUser = 'bevy2',
    [string]$RenterPass = '123456',
    [int]$OwnerPort = 9095,
    [int]$RenterPort = 9096,
    [int]$TimeoutSec = 120
)

# ---- 实机资源互斥 ----------------------------------------------------------
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5r_rental_name' -TimeoutSec 1800)) { exit 2 }

try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $ClientHome) { $ClientHome = (Resolve-Path "$PSScriptRoot\..\..").Path }
$exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"
Assert-ClientBuildStamp -Exe $exeSrc -Worktree $ClientHome -ScriptName 'l5r_rental_name'

# 唯一进程名（LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只清自己这两份改名副本
$ownerExe = Join-Path (Split-Path -Parent $exeSrc) 'l5r_owner.exe'
$renterExe = Join-Path (Split-Path -Parent $exeSrc) 'l5r_renter.exe'
foreach ($p in 'l5r_owner.exe', 'l5r_renter.exe') {
    Get-CimInstance Win32_Process -Filter "Name='$p'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}
Start-Sleep -Milliseconds 700
foreach ($pair in @(@($ownerExe, $exeSrc), @($renterExe, $exeSrc))) {
    try { New-Item -ItemType HardLink -Path $pair[0] -Target $pair[1] -Force -ErrorAction Stop | Out-Null }
    catch { Copy-Item -LiteralPath $pair[1] -Destination $pair[0] -Force }
}

function Rpc([int]$Port, [string]$m, [hashtable]$q = @{}) {
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
$tmp = $env:TEMP
$ownerLog = Join-Path $tmp 'l5r_owner.err.log'
$renterLog = Join-Path $tmp 'l5r_renter.err.log'

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 2 }

# 租客先起（物主 10s 后按角色名发起请求，目标必须已在线）
Start-Process -FilePath $renterExe -ArgumentList '--real-net','--auto-enter','--rental-renter',
    '--e2e-user', $RenterUser, '--e2e-pass', $RenterPass, '--control-port', "$RenterPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" -RedirectStandardOutput (Join-Path $tmp 'l5r_renter.out.log') `
    -RedirectStandardError $renterLog | Out-Null
$rst = $null
foreach ($i in 1..60) { Start-Sleep 1; $rst = Rpc $RenterPort 'state'; if ($null -ne $rst.tile_x) { break } }
if ($null -eq $rst.tile_x) { Write-Host 'FAIL(前置): 租客客户端未进场'; exit 2 }
Write-Host ("[前置] 租客进场 map={0} tile=({1},{2})" -f $rst.map, $rst.tile_x, $rst.tile_y)

Start-Process -FilePath $ownerExe -ArgumentList '--real-net','--auto-enter','--rental-test',
    '--e2e-user', $OwnerUser, '--e2e-pass', $OwnerPass, '--control-port', "$OwnerPort" `
    -WorkingDirectory "$ClientHome\Client-Bevy" -RedirectStandardOutput (Join-Path $tmp 'l5r_owner.out.log') `
    -RedirectStandardError $ownerLog | Out-Null
$ost = $null
foreach ($i in 1..60) { Start-Sleep 1; $ost = Rpc $OwnerPort 'state'; if ($null -ne $ost.tile_x) { break } }
if ($null -eq $ost.tile_x) { Write-Host 'FAIL(前置): 物主客户端未进场'; exit 2 }
Write-Host ("[前置] 物主进场 map={0} tile=({1},{2})" -f $ost.map, $ost.tile_x, $ost.tile_y)

# 等「租客侧对方物品窗真出现物品」（= UpdateRentalItem 已生效）
$rental = $null
foreach ($i in 1..$TimeoutSec) {
    Start-Sleep 1
    $r = Rpc $RenterPort 'rental_probe'
    if ($null -ne $r -and $null -ne $r.partner_item) { $rental = $r; break }
    if ($i % 10 -eq 0) {
        Write-Host ("  [{0}s] 租客侧还没出现对方物品（request_received={1} role={2}）" -f `
                $i, $r.request_received, $r.role)
    }
}
$owner = Rpc $OwnerPort 'rental_probe'
if ($null -eq $rental) {
    Write-Host ('FAIL(R1): {0}s 内租客侧对方物品窗没有物品；物主侧 rental_probe=' -f $TimeoutSec)
    Write-Host (($owner | ConvertTo-Json -Compress -Depth 6))
    Write-Host '--- 租客日志尾 ---'; Get-Content $renterLog -Tail 5 -EA SilentlyContinue | ForEach-Object { Write-Host ('   ' + $_) }
    Write-Host '--- 物主日志尾 ---'; Get-Content $ownerLog -Tail 5 -EA SilentlyContinue | ForEach-Object { Write-Host ('   ' + $_) }
    exit 2
}

$pi = $rental.partner_item
$di = $null; if ($null -ne $owner) { $di = $owner.deposit_item }
Write-Host ("[R1] 租客 role={0} 对方名={1} 对方物品 name='{2}' item_index={3} uid={4} has_item={5} fee={6} period={7}" -f `
        $rental.role, $rental.partner_name, $pi.name, $pi.item_index, $pi.unique_id, $rental.has_item, $rental.partner_fee, $rental.partner_period)
if ($null -ne $di) {
    Write-Host ("[R3] 物主 deposit_item name='{0}' item_index={1} uid={2}（对照组）" -f $di.name, $di.item_index, $di.unique_id)
} else {
    Write-Host '[R3] 物主侧还没读到 deposit_item（不影响 R1/R2 判定）'
}
$r1 = ($null -ne $pi)
$r2 = $r1 -and (-not $pi.placeholder) -and ("$($pi.name)" -notmatch '^#\d+$') -and ("$($pi.name)" -ne '')
$r3 = ($null -eq $di) -or ((-not $di.placeholder) -and ("$($di.name)" -notmatch '^#\d+$'))
Write-Host ("  R1 对方物品窗有物品={0}｜R2 显示名不是内部 ID={1}（'{2}'）｜R3 物主侧对照={3}" -f $r1, $r2, $pi.name, $r3)
Write-Host ("VERDICT rental_name={0}" -f $(if ($r1 -and $r2 -and $r3) { 'PASS' } else { 'FAIL' }))
if (-not ($r1 -and $r2 -and $r3)) { exit 10 }
exit 0

} finally {
    foreach ($p in 'l5r_owner.exe', 'l5r_renter.exe') {
        Get-CimInstance Win32_Process -Filter "Name='$p'" -EA SilentlyContinue |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    }
    Exit-E2eLock
}
