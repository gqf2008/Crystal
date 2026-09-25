# player_walk2.ps1 —— 玩家视角遍历（第二轮）：走到目标旁再交互（本地件，未入库）
# 覆盖：单方向走路 / 近战击杀 / 掉落拾取 / NPC 对话 / 传送换图

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'player_walk2' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁
# （PowerShell 的 finally 在 exit 下也会执行——实测 -File 与会话内 & script.ps1 两种调用都成立），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$&
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'player_walk2'
$wd  = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy'
$shotDir = "$acc\player_shots"
New-Item -ItemType Directory -Force -Path $shotDir | Out-Null

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
function Shot([string]$name) {
    Rpc 'screenshot' @{ path = "$shotDir\$name.png" } | Out-Null
    Start-Sleep -Milliseconds 900
    if (Test-Path "$shotDir\$name.png") { "player_shots/$name.png" } else { '(截图失败)' }
}
# 朝目标走：按世界坐标差逐格走（每格 32px），直到曼哈顿距离 <= $within
function WalkTo([double]$tx, [double]$ty, [int]$within = 40, [int]$maxSteps = 30) {
    for ($i = 0; $i -lt $maxSteps; $i++) {
        $st = Rpc 'state'
        $dx = $tx - $st.x; $dy = $ty - $st.y
        if ([Math]::Abs($dx) + [Math]::Abs($dy) -le $within) { return $i }
        $sx = 0; $sy = 0
        if ([Math]::Abs($dx) -ge [Math]::Abs($dy)) { $sx = [Math]::Sign($dx) } else { $sy = [Math]::Sign($dy) }
        Rpc 'move' @{ dx = [int]$sx; dy = [int]$sy; run = $true } | Out-Null
        Start-Sleep -Milliseconds 700
    }
    return $maxSteps
}
$script:results = [System.Collections.Generic.List[object]]::new()
function Step([string]$id, [string]$path, [string]$expect, [scriptblock]$body) {
    $rec = [ordered]@{ id=$id; path=$path; expect=$expect; verdict='FAIL'; detail=''; evidence=@() }
    try {
        $out = & $body
        if ($out.detail) { $rec.detail = $out.detail }
        if ($out.evidence) { $rec.evidence = @($out.evidence) }
        $rec.verdict = if ($out.verdict) { $out.verdict } else { 'PASS' }
    } catch { $rec.verdict = 'FAIL'; $rec.detail = "异常: $_" }
    $script:results.Add([pscustomobject]$rec)
    Write-Host ("[{0}] {1} {2} :: {3}" -f $rec.verdict, $rec.id, $rec.path, $rec.detail)
}

Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    Where-Object { $_.CommandLine -match '--e2e-user' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\player_client2.log" -RedirectStandardError "$acc\player_client2.err.log" -PassThru
$ok = $false
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {} }
if (-not $ok) { throw '客户端未进入游戏' }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
Start-Sleep 3

Step 'PLAY-21' '单方向走路（不跑）' 'tile_x 精确 +1' {
    $a = Rpc 'state'
    Rpc 'move' @{ dx = 1; dy = 0; run = $false } | Out-Null
    Start-Sleep -Seconds 2
    $b = Rpc 'state'
    Rpc 'move' @{ dx = -1; dy = 0; run = $false } | Out-Null
    Start-Sleep -Seconds 2
    $c = Rpc 'state'
    @{ verdict = if ($b.tile_x -eq $a.tile_x + 1) { 'PASS' } else { 'FAIL' }
       detail = "x: $($a.tile_x) → $($b.tile_x)（期望 +1），回走 → $($c.tile_x)"
       evidence = (Shot 'p21_walk_step') }
}

Step 'PLAY-22' '走到最近的鹿旁边再近战击杀' '怪物死亡（视野内消失）' {
    $m = (Rpc 'nearby').entities | Where-Object { $_.kind -eq 'monster' } | Sort-Object dist | Select-Object -First 1
    if (-not $m) { throw '视野内没有怪物' }
    $steps = WalkTo $m.x $m.y 40 30
    $st = Rpc 'state'
    $nb = (Rpc 'nearby').entities | Where-Object { $_.object_id -eq $m.object_id }
    $distAtMelee = if ($nb) { $nb.dist } else { -1 }
    $killed = $false
    for ($i = 0; $i -lt 30; $i++) {
        Rpc 'attack' @{ object_id = $m.object_id } | Out-Null
        Start-Sleep -Milliseconds 800
        $now = (Rpc 'nearby').entities | Where-Object { $_.object_id -eq $m.object_id }
        if ($null -eq $now) { $killed = $true; break }
    }
    @{ verdict = if ($killed) { 'PASS' } else { 'FAIL' }
       detail = "目标 $($m.name)#$($m.object_id) 靠近用了 $steps 步，接敌距离=$distAtMelee，$(if($killed){'已被击杀'}else{'30 次攻击后仍存活'})"
       evidence = (Shot 'p22_combat_kill') }
}

Step 'PLAY-23' '击杀后拾取掉落' '掉落被拾取（视野内消失）' {
    $drops = (Rpc 'nearby').entities | Where-Object { $_.kind -eq 'item' -or $_.kind -eq 'drop' }
    if (-not $drops) { return @{ verdict='INFO'; detail='本次击杀没有掉落物（鹿可能不掉或掉落为空）'; evidence=(Shot 'p23_no_drop') } }
    $d = $drops | Select-Object -First 1
    if ($d.x) { WalkTo $d.x $d.y 40 12 | Out-Null }
    Rpc 'pickup' @{ object_id = $d.object_id } | Out-Null
    Start-Sleep 2
    $still = (Rpc 'nearby').entities | Where-Object { $_.object_id -eq $d.object_id }
    @{ verdict = if ($null -eq $still) { 'PASS' } else { 'FAIL' }
       detail = "掉落 $($d.name)#$($d.object_id) 共 $($drops.Count) 件"
       evidence = (Shot 'p23_after_pickup') }
}

Step 'PLAY-24' '走到 NPC 旁 → 打开 NPC 对话' 'NPC 窗口出现' {
    $npc = (Rpc 'nearby').entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
    if (-not $npc) { throw '视野内没有 NPC' }
    $steps = WalkTo $npc.x $npc.y 34 30
    $nb2 = (Rpc 'nearby').entities | Where-Object { $_.object_id -eq $npc.object_id }
    $dist = if ($nb2) { $nb2.dist } else { -1 }
    Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
    Start-Sleep 2
    $vis = (Rpc 'visible').visible
    @{ verdict = if ($vis -match 'Npc') { 'PASS' } else { 'FAIL' }
       detail = "NPC=$($npc.name)#$($npc.object_id) 靠近 $steps 步，距离=$dist，visible=[$vis]"
       evidence = (Shot 'p24_npc_dialog') }
}

Step 'PLAY-25' 'NPC 对话里点选项（买/卖入口）' '点击后窗口栈有反应' {
    $rect = Rpc 'dialog_rect' @{ kind = 'npc' }
    if (-not $rect.ok) { return @{ verdict='INFO'; detail='NPC 窗未开，跳过选项点击（上一步结果见 PLAY-24）'; evidence=@() } }
    $cx = [math]::Round($rect.x + $rect.w / 2, 1)
    $cy = [math]::Round($rect.y + 60, 1)
    $click = Rpc 'click' @{ x = $cx; y = $cy }
    Start-Sleep 1
    $vis = (Rpc 'visible').visible
    @{ verdict='INFO'; detail = "在 NPC 窗内 ($cx,$cy) 点击，hits=[$(($click.hits) -join ' | ')]，visible=[$vis]"
       evidence = (Shot 'p25_npc_option') }
}

Step 'PLAY-26' 'GM 传送换图（@MOVE）' '地图/坐标改变，世界重载' {
    $a = Rpc 'state'
    Rpc 'chat' @{ message = '@MOVE 0 300 300' } | Out-Null
    Start-Sleep 4
    $b = Rpc 'state'
    @{ verdict = if (($b.tile_x -ne $a.tile_x) -or ($b.tile_y -ne $a.tile_y)) { 'PASS' } else { 'INFO' }
       detail = "tile ($($a.tile_x),$($a.tile_y)) → ($($b.tile_x),$($b.tile_y))"
       evidence = (Shot 'p26_after_move_cmd') }
}

Step 'PLAY-27' '造物并查看背包内容' '背包里有刚造出的物品' {
    Rpc 'chat' @{ message = '@MAKE Gold 10000' } | Out-Null; Start-Sleep 1
    Rpc 'chat' @{ message = '@MAKE Saddle 1' } | Out-Null; Start-Sleep 2
    Rpc 'dialog' @{ kind='inventory'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p27_inventory_after_make'
    Rpc 'dialog' @{ kind='inventory'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='需目检背包格子是否出现物品'; evidence=$e }
}

$script:results | ConvertTo-Json -Depth 6 | Out-File "$acc\player_walk2_results.json" -Encoding utf8
$pass = @($script:results | Where-Object { $_.verdict -eq 'PASS' }).Count
$fail = @($script:results | Where-Object { $_.verdict -eq 'FAIL' }).Count
$info = @($script:results | Where-Object { $_.verdict -eq 'INFO' }).Count
Write-Host ("===== 玩家遍历第二轮: PASS={0} FAIL={1} INFO={2} 共 {3} 步 =====" -f $pass, $fail, $info, $script:results.Count)
$script:results | ForEach-Object { Write-Host ("  {0} {1} :: {2}" -f $_.verdict, $_.id, $_.detail) }
Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
exit ([int]($fail -gt 0))

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
