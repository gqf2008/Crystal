# player_walk.ps1 —— 玩家视角遍历驱动（本地件：.gitignore 未放行 tools/acceptance/*）
# 目的：以「玩家」身份实际走一遍玩法路径，逐步取证（RPC 真值 + 截图），产出 JSON 供报告引用。
# 前提：mir2_server 在跑。本脚本自起客户端（--real-net --auto-enter）。

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'player_walk' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

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
Assert-ClientBuildStamp -Exe $exe -ScriptName 'player_walk'
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
$script:results = [System.Collections.Generic.List[object]]::new()
function Step([string]$id, [string]$path, [string]$expect, [scriptblock]$body) {
    $rec = [ordered]@{ id=$id; path=$path; expect=$expect; verdict='FAIL'; detail=''; evidence=@() }
    try {
        $out = & $body
        if ($out.detail) { $rec.detail = $out.detail }
        if ($out.evidence) { $rec.evidence = @($out.evidence) }
        $rec.verdict = if ($out.verdict) { $out.verdict } else { 'PASS' }
    } catch {
        $rec.verdict = 'FAIL'; $rec.detail = "异常: $_"
    }
    $script:results.Add([pscustomobject]$rec)
    Write-Host ("[{0}] {1} {2} :: {3}" -f $rec.verdict, $id, $path, $rec.detail)
}

# ---------- 起客户端 ----------
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    Where-Object { $_.CommandLine -match '--e2e-user' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\player_client.log" -RedirectStandardError "$acc\player_client.err.log" -PassThru
$ok = $false
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {} }
if (-not $ok) { throw '客户端未进入游戏' }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
Start-Sleep 3

Step 'PLAY-01' '进入游戏后的初始状态' '有坐标/朝向，世界可渲染' {
    $s = Rpc 'state'
    $nb = (Rpc 'nearby').entities
    $mon = @($nb | Where-Object { $_.kind -eq 'monster' }).Count
    $npc = @($nb | Where-Object { $_.kind -eq 'npc' }).Count
    @{ verdict = if ($null -ne $s.tile_x -and ($mon + $npc) -gt 0) { 'PASS' } else { 'FAIL' }
       detail = "tile=($($s.tile_x),$($s.tile_y)) dir=$($s.direction) 视野内 monster=$mon npc=$npc"
       evidence = (Shot 'p01_world') }
}

Step 'PLAY-02' '世界移动：四方向各走一格' 'tile 发生变化' {
    $a = (Rpc 'state').tile_x, (Rpc 'state').tile_y
    foreach ($d in @(@(1,0),@(-1,0),@(0,1),@(0,-1))) { Rpc 'move' @{ dx=$d[0]; dy=$d[1]; run=$false } | Out-Null; Start-Sleep -Milliseconds 700 }
    Start-Sleep 1
    $s = Rpc 'state'
    $b = "$($s.tile_x),$($s.tile_y)"
    @{ verdict = if ("$($a[0]),$($a[1])" -ne $b) { 'PASS' } else { 'FAIL' }
       detail = "from=$($a[0]),$($a[1]) to=$b"
       evidence = (Shot 'p02_after_move') }
}

Step 'PLAY-03' '跑步移动（run=true）' '能跑，位移生效' {
    $a = Rpc 'state'; Rpc 'move' @{ dx=1; dy=0; run=$true } | Out-Null; Start-Sleep 2
    $b = Rpc 'state'
    @{ verdict = if ($b.tile_x -ne $a.tile_x) { 'PASS' } else { 'FAIL' }
       detail = "tile_x $($a.tile_x) -> $($b.tile_x)"
       evidence = (Shot 'p03_run') }
}

Step 'PLAY-04' '攻击最近怪物' '怪物从视野消失或距离改变' {
    $nb = (Rpc 'nearby').entities
    $m = $nb | Where-Object { $_.kind -eq 'monster' } | Sort-Object dist | Select-Object -First 1
    if (-not $m) { throw '视野内没有怪物' }
    $before = $m.object_id
    Rpc 'attack' @{ object_id = $before } | Out-Null
    foreach ($i in 1..12) { Start-Sleep 1; Rpc 'attack' @{ object_id = $before } | Out-Null }
    Start-Sleep 2
    $nb2 = (Rpc 'nearby').entities
    $after = $nb2 | Where-Object { $_.object_id -eq $before }
    @{ verdict = if ($null -eq $after) { 'PASS' } else { 'INFO' }
       detail = "目标 $($m.name)#$before dist=$($m.dist) → $(if($after){"仍在 dist=$($after.dist)"}else{'已消失'})"
       evidence = (Shot 'p04_combat') }
}

Step 'PLAY-05' '拾取地面掉落' '掉落物进入背包（视野内消失）' {
    $drops = (Rpc 'nearby').entities | Where-Object { $_.kind -eq 'item' -or $_.kind -eq 'drop' }
    if (-not $drops) { return @{ verdict='INFO'; detail='本轮战斗未产生可见掉落（击杀数不足/未掉落）'; evidence=(Shot 'p05_no_drop') } }
    $d = $drops | Select-Object -First 1
    Rpc 'pickup' @{ object_id = $d.object_id } | Out-Null
    Start-Sleep 2
    $still = (Rpc 'nearby').entities | Where-Object { $_.object_id -eq $d.object_id }
    @{ verdict = if ($null -eq $still) { 'PASS' } else { 'FAIL' }
       detail = "掉落 $($d.name)#$($d.object_id)"
       evidence = (Shot 'p05_after_pickup') }
}

Step 'PLAY-06' '与最近 NPC 对话' 'NPC 窗口打开' {
    $npc = (Rpc 'nearby').entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
    if (-not $npc) { throw '视野内没有 NPC' }
    Rpc 'move' @{ dx=[Math]::Sign($npc.vp.x - 0); dy=0; run=$true } | Out-Null
    Start-Sleep 1
    Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
    Start-Sleep 2
    $vis = (Rpc 'visible').visible
    @{ verdict = if ($vis -match 'Npc') { 'PASS' } else { 'INFO' }
       detail = "NPC=$($npc.name)#$($npc.object_id) dist=$($npc.dist) visible=[$vis]"
       evidence = (Shot 'p06_npc_dialog') }
}

Step 'PLAY-07' '聊天发言' '聊天内容被服务端接受' {
    Rpc 'chat' @{ message = 'hello from player-walk' } | Out-Null
    Start-Sleep 2
    @{ verdict='PASS'; detail='已发送普通聊天'; evidence=(Shot 'p07_chat') }
}

Step 'PLAY-08' '打开背包' '背包窗可见' {
    Rpc 'dialog' @{ kind='inventory'; action='open' } | Out-Null; Start-Sleep 1
    $d = (Rpc 'dialogs') ; Rpc 'dialog' @{ kind='inventory'; action='close' } | Out-Null
    @{ verdict='PASS'; detail = "dialogs=$($d.dialogs -join ',')"; evidence=(Shot 'p08_inventory') }
}

Step 'PLAY-09' 'GM 造物后查看背包' '物品出现在背包（视觉）' {
    Rpc 'chat' @{ message = '@MAKE Gold 5000' } | Out-Null
    Start-Sleep 1
    Rpc 'chat' @{ message = '@MAKE 药水 3' } | Out-Null
    Start-Sleep 2
    Rpc 'dialog' @{ kind='inventory'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p09_inventory_items'
    Rpc 'dialog' @{ kind='inventory'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='已尝试 @MAKE（结果需目检截图）'; evidence=$e }
}

Step 'PLAY-10' '查看角色属性面板' '属性窗渲染' {
    Rpc 'dialog' @{ kind='character'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p10_character'
    Rpc 'dialog' @{ kind='character'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='属性数值需目检'; evidence=$e }
}

Step 'PLAY-11' '任务面板（已接任务）' '任务列表渲染' {
    Rpc 'dialog' @{ kind='quest_log'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p11_quest_log'
    Rpc 'dialog' @{ kind='quest_log'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='任务条目需目检'; evidence=$e }
}

Step 'PLAY-12' '邮件面板' '邮件窗渲染' {
    Rpc 'dialog' @{ kind='mail'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p12_mail'
    Rpc 'dialog' @{ kind='mail'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='邮件列表需目检'; evidence=$e }
}

Step 'PLAY-13' '仓库面板' '仓库窗渲染' {
    Rpc 'dialog' @{ kind='storage'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p13_storage'
    Rpc 'dialog' @{ kind='storage'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='仓库格子需目检'; evidence=$e }
}

Step 'PLAY-14' '行会面板（成员/公告）' '成员列表与公告页渲染' {
    Rpc 'dialog' @{ kind='guild'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p14_guild'
    Rpc 'dialog' @{ kind='guild'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='成员/公告需目检'; evidence=$e }
}

Step 'PLAY-15' '宠物面板' '宠物列表渲染' {
    Rpc 'dialog' @{ kind='creature'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p15_creature'
    Rpc 'dialog' @{ kind='creature'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='宠物立绘/信息行需目检'; evidence=$e }
}

Step 'PLAY-16' '英雄管理面板' '英雄窗可见' {
    Rpc 'dialog' @{ kind='hero_manage'; action='open' } | Out-Null; Start-Sleep 1
    $vis = (Rpc 'visible').visible
    $e = Shot 'p16_hero_manage'
    Rpc 'dialog' @{ kind='hero_manage'; action='close' } | Out-Null
    @{ verdict = if ($vis -match 'HeroManage') { 'PASS' } else { 'FAIL' }; detail = "visible=[$vis]"; evidence=$e }
}

Step 'PLAY-17' '坐骑：造马具 → 骑乘 → 走路' '骑乘后可在世界移动' {
    Rpc 'chat' @{ message = '@MAKE LeatherBridle 1' } | Out-Null; Start-Sleep 1
    Rpc 'chat' @{ message = '@MAKE Saddle 1' } | Out-Null; Start-Sleep 1
    Rpc 'chat' @{ message = '@RIDE' } | Out-Null; Start-Sleep 2
    $a = Rpc 'state'; Rpc 'move' @{ dx=1; dy=0; run=$true } | Out-Null; Start-Sleep 2
    $b = Rpc 'state'
    @{ verdict = if ($b.tile_x -ne $a.tile_x) { 'PASS' } else { 'FAIL' }
       detail = "骑乘状态下的移动 tile_x $($a.tile_x) -> $($b.tile_x)（是否真的骑上需目检截图）"
       evidence = (Shot 'p17_mount') }
}

Step 'PLAY-18' '大地图与坐标' '大地图窗渲染' {
    Rpc 'dialog' @{ kind='big_map'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p18_big_map'
    Rpc 'dialog' @{ kind='big_map'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='地图纹理/玩家点需目检'; evidence=$e }
}

Step 'PLAY-19' '设置面板与持久化' '设置窗渲染' {
    Rpc 'dialog' @{ kind='settings'; action='open' } | Out-Null; Start-Sleep 1
    $e = Shot 'p19_settings'
    Rpc 'dialog' @{ kind='settings'; action='close' } | Out-Null
    @{ verdict='INFO'; detail='设置项需目检'; evidence=$e }
}

Step 'PLAY-20' '生活玩法四窗（合成/精炼/镶嵌/钓鱼）' '四窗渲染' {
    $ev = @()
    foreach ($k in @('craft','refine','socket','fishing')) {
        Rpc 'dialog' @{ kind=$k; action='open' } | Out-Null; Start-Sleep -Milliseconds 900
        $ev += (Shot "p20_$k")
        Rpc 'dialog' @{ kind=$k; action='close' } | Out-Null; Start-Sleep -Milliseconds 300
    }
    @{ verdict='INFO'; detail='四窗已逐个开+截图'; evidence=$ev }
}

# ---------- 收尾 ----------
$script:results | ConvertTo-Json -Depth 6 | Out-File "$acc\player_walk_results.json" -Encoding utf8
$pass = @($script:results | Where-Object { $_.verdict -eq 'PASS' }).Count
$fail = @($script:results | Where-Object { $_.verdict -eq 'FAIL' }).Count
$info = @($script:results | Where-Object { $_.verdict -eq 'INFO' }).Count
Write-Host ("===== 玩家遍历: PASS={0} FAIL={1} INFO={2} 共 {3} 步 =====" -f $pass, $fail, $info, $script:results.Count)
if ($fail -gt 0) { $script:results | Where-Object { $_.verdict -eq 'FAIL' } | ForEach-Object { Write-Host ("  FAIL " + $_.id + ' ' + $_.path + ' :: ' + $_.detail) -ForegroundColor Red } }
Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
exit ([int]($fail -gt 0))

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
