# l5e_storage_roundtrip.ps1 — ⑤ 邮件仓库闭环（仓库半段）：造物 → 存 → 取，用格数增减判成交
#
# 判据（缺一不可，全部取状态而非像素）：
#   A0) 密码闸门（#3260）：`RequireStoragePassword && !HasStoragePassword` 时 C# `StorageDialog.Show()`
#       走 `ForceStoragePasswordSetup` —— **先设密码、窗不显示**；本夹具用 `MirInputBox` 真实路径输入
#       （点输入区拿焦点 → `type_text` → Enter）走完「新密码 → 确认」，然后断言客户端自己把窗打开
#       （`_pendingOpenAfterPasswordSet`）。已设过密码的库走 `PromptStorageUnlock`（输一次密码解锁）。
#       判据：闸门期间 `storage_probe.visible=false`（**不是** total：total 是格数容量，
#       服务端在无密码时就已经下发过 UserStorage）；过闸后 `visible=true`。输入法是中文时先切英文
#       （`ime_probe.enabled` → 按一次 Shift），否则字母进拼音组合、数字选候选。
#   A) 开仓库前置达成：storage_probe.visible=true **且** total != 0（窗口真开、内容真到手）
#   B) 存入：bag.used 减 1 且 storage.used 加 1，且物品出现在仓库的 occupied 里
#   C) 取回：storage.used 减 1 且 bag.used 加 1
# 判据仪器：bag_probe / storage_probe 的 occupied（格号→名称），动作侧 storage_store/
# storage_take 发的是与点击路径同一个包（C.StoreItem=15 / C.TakeBackItem=16）。
#
# 客户端构建根可用 `-ClientHome` 指定（与 l5g/l5i/l5j 同款）。**必须**能指向含修复的构建：
# 本夹具此前把 `$exe` 硬编码成主工作区的构建，实机跑出来的是**旧客户端**（主工作区落后
# master 数十个提交，缺 #3058 的 npc_object_id 边沿清零修复），于是「第一次 npc_call 后
# npc_object_id 读回 0、第二次才正常」被当成本端竞态追了一轮——真因是夹具指向了旧构建。
# 夹具自身的构建来源必须显式、可覆盖，否则测的根本不是当前代码。
param(
    [string]$ClientHome = ''
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5e_storage_roundtrip' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName 'l5e_storage_roundtrip'
# 唯一进程名（见 LESSON_多agent并行时按进程名清进程会污染他人GUI实验）：只用自己改名的副本，
# 清场也只清这个唯一名——公共名 client_bevy.exe 可能是别的 agent 的验收或人工 GUI 会话。
$exeSrc = $exe
$exe = Join-Path (Split-Path -Parent $exe) 'l5e_client.exe'
function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Shot([string]$n) { Rpc 'screenshot' @{ path = "$acc\player_shots\l5e_$n.png" } | Out-Null; Start-Sleep -Milliseconds 700 }

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
Get-CimInstance Win32_Process -Filter "Name='l5e_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 900
# 硬链接起唯一命名副本：不占额外磁盘（同一个文件、多一个目录项），
# 且源文件正被别的进程执行时也能建链（Copy-Item 会因文件占用失败）。失败则退回拷贝。
try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user','test','--e2e-pass','123456' `
    -WorkingDirectory "$ClientHome\Client-Bevy" `
    -RedirectStandardOut "$acc\l5e_client.log" -RedirectStandardError "$acc\l5e_client.err.log" -PassThru
foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } } catch {} }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)

# 造物（GM 通道）：一件可堆叠的普通消耗品，避免装备栏/重量干扰
Rpc 'chat' @{ message = '@MAKE Saddle 1' } | Out-Null
Start-Sleep -Seconds 2
$bag0 = Rpc 'bag_probe'
Write-Host ("bag before: used={0}/{1} occupied={2}" -f $bag0.used, $bag0.total, (($bag0.occupied | ForEach-Object { "$($_.cell):$($_.name)" }) -join ','))
$item = $bag0.occupied | Where-Object { $_.name -eq 'Saddle' } | Select-Object -First 1
if (-not $item) { Write-Host 'FAIL: @MAKE 后背包里没有 Saddle'; exit 1 }
$srcCell = [int]$item.cell

# 物品**件数**（occupied 里 count 之和）——存取判据用件数，不用"占用格数"：
# 可堆叠物品（Saddle 6 个占一格）存 1 个进仓库时源格**仍然占用**，`used` 不会减 1。
# 2026-09-26 实测踩到：`storage+1=True bag-1=False`（那一次源格是 1 个、这次是 6 个里的 1 个）。
function ItemCount($probe) {
    $n = 0
    foreach ($o in $probe.occupied) { $n += [int]$o.count }
    return $n
}

# 开仓库：走到仓库 NPC（Storage_Jake，map 40 / 文件名 D002 @ 174,216）
#   → npc_call [@MAIN] → 点 <Access/@Storage> 链接
#
# 夹具自身硬化（2026-09-24，实机踩到假 FAIL）：
#   ① **按名字选 NPC**（Storage_* / Warehouse_*），不按「最近」——同图可能有别的 NPC
#      （D002 上还有 StrangeMan），而且换图后 `nearby` 头几秒可能仍是上一张图的实体列表，
#      「取最近的」会把上一图的 NPC（如传送 NPC）当成仓库 NPC 去 npc_call，服务端找不到该
#      object_id 就静默丢弃 → 探针读回 npc_object_id=0，被记成「仓库窗没开」的假 FAIL。
#   ② 轮询到「NPC 出现在本图」与「[@MAIN] 真的开窗（npc_object_id 非 0）」两个条件，
#      不用固定 sleep（冷图客户端首次生成对象要数秒）。
# 开仓库是**重试到判据成立**的步骤，不是「call 一次点一下」（2026-09-24 实机踩到）：
#   换图后地图重建期间发出的 CallNPC，客户端可能已经渲染出行（npc_rows.links 有 [@Storage]）
#   但 `npc_object_id` 仍为 0——此时点行发出的 CallNPC{object_id:0} 会被服务端静默丢弃，
#   表现成「点了没反应」。判据必须是 storage_probe.total != 0（窗真开），而不是中间态字段；
#   未成立就重新按名字定位 NPC 并重发 npc_call（object_id 可能随地图重建变化）。
# 注意只认**地图名**：服务端 `MAPMOVE` 按 `map_infos.file_name` 查图（`session.rs:6598-6620`），
# 传索引会查不到（实测 `@mapmove 40 …` 无反应/无换图，`@mapmove D002 …` → map=D002 tile=(174,217)）。
Rpc 'chat' @{ message = '@mapmove D002 174 217' } | Out-Null
# 硬前置：**等换图真的完成**（`state.map` 变成 D002）再扫 nearby。
# 2026-09-26 实测踩到过假 FAIL：`@mapmove` 发出后立刻轮询 `nearby`，拿到的是**上一张图**的
# NPC 列表（20s 都没变），于是报"本图没有仓库 NPC"——其实人已经在 D002 上（同轮手动
# `@mapmove D002 174 217` 单发验证：`map=D002 tile=(174,217)`）。
# 判据取 `state.map`（不是 nearby 的名字表），超时才失败，并把服务端系统消息尾打出来
# （便于区分"没权限/地图不存在"这类真因）。
$switched = $false
foreach ($i in 1..20) {
    Start-Sleep 1
    $sm = Rpc 'state'
    if ($sm.map -eq 'D002') { $switched = $true; Write-Host ("[切换] D002 @ ({0},{1})，用时 {2}s" -f $sm.tile_x, $sm.tile_y, $i); break }
}
if (-not $switched) {
    $cp = Rpc 'chat_probe'
    $sys = @($cp.lines | Where-Object { $_.channel -eq 'System' } | Select-Object -Last 3 | ForEach-Object { $_.text })
    Write-Host ("FAIL: 20s 内没换到 D002（state.map={0}）；系统消息尾：{1}" -f (Rpc 'state').map, ($sys -join ' / '))
    exit 3
}
$st0 = $null
$npc = $null
$opened = $false
foreach ($attempt in 1..3) {
    # ① 按名字定位仓库 NPC（Storage_* / Warehouse_*），轮询到它出现在本图
    $npc = $null
    foreach ($i in 1..20) {
        Start-Sleep 1
        $near = Rpc 'nearby' @{ radius = 2000 }
        $npc = $near.entities |
            Where-Object { $_.kind -eq 'npc' -and ($_.name -like 'Storage_*' -or $_.name -like 'Warehouse_*') } |
            Sort-Object dist | Select-Object -First 1
        if ($npc) { break }
    }
    if (-not $npc) {
        Write-Host ('FAIL: 20s 内本图没有仓库 NPC（Storage_*/Warehouse_*）；nearby npcs=' +
            (($near.entities | Where-Object { $_.kind -eq 'npc' } | ForEach-Object { $_.name }) -join ','))
        exit 1
    }
    Write-Host ("[attempt {0}] 仓库 NPC={1} id={2} dist={3}" -f $attempt, $npc.name, $npc.object_id, [int]$npc.dist)

    # ② 开菜单：npc_call [@MAIN] → 轮询到 npc_object_id 非 0（未开窗就重试）
    Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
    $rows = $null
    $rowOpen = $false
    foreach ($i in 1..10) {
        Start-Sleep 1
        $rows = Rpc 'npc_rows'
        if ($rows.npc_object_id -ne 0) { $rowOpen = $true; break }
    }
    if (-not $rowOpen) {
        Write-Host ("[attempt {0}] npc_object_id 仍为 0（links={1}）——重试" -f $attempt,
            (($rows.links | ForEach-Object { $_.key }) -join ','))
        continue
    }
    $link = $rows.links | Where-Object { $_.key -eq '[@Storage]' } | Select-Object -First 1
    if (-not $link) {
        # 2026-09-24 实测：换图/重建后 object_id 过期时，`npc_object_id` 可能读回非 0 而 `links` 为空——
        # 这与「窗没开」同属**可重试**状态，不能一读就判死（首跑就在 attempt 1 上因此假红，单独重跑即绿；
        # 独立探针证明产品侧 [@Storage]/[@exit] 两条链接与 visible=true 都正常）。
        Write-Host ("[attempt {0}] npc_rows 有对象但 links 为空（links={1}）——重试" -f $attempt,
            (($rows.links | ForEach-Object { $_.key }) -join ','))
        continue
    }

    # ③ 点 <Access/@Storage> → 轮询到仓库窗真开（total 非 0 才算达成前置）
    Rpc 'click' @{ x = $link.cx; y = $link.cy } | Out-Null
    # #3260 密码闸门：`RequireStoragePassword && !HasStoragePassword` 时，C# `StorageDialog.Show()`
    # 走 `ForceStoragePasswordSetup` —— **先设密码、不显示仓库**（`NPCDialogs.cs:2974-2980`）。
    # 所以判据分两段：① 先抓到「闸门状态 + total=0」（闸门确实挡住了窗）；
    # ② 用**真实 UI 路径**过闸（`MirInputBox` 里输入新密码 → 回车 → 再输确认 → 回车），
    #    过闸后客户端应自己把窗打开（`_pendingOpenAfterPasswordSet`，`:3067-3072`）。
    $gateSeen = $false
    $unlockSeen = $false
    foreach ($i in 1..14) {
        Start-Sleep 1
        $st0 = Rpc 'storage_probe'
        if ($null -eq $st0) { continue }
        if ($st0.require_password -and -not $st0.has_password -and -not $gateSeen) {
            # 判据是 `visible`（= C# `StorageDialog.Visible`），不是 `total`：`total` 是**格数容量**，
            # 服务端在「无密码」时就已经下发过 UserStorage（C# `SendStorage()` 同款），
            # 所以闸门期间 total=80 但窗**没显示**（实测就是这样，一开始把 total 当「窗开」判错了）。
            if ($st0.visible) {
                Write-Host ("FAIL(A1): 闸门期间仓库窗不该显示（visible={0} total={1}）" -f $st0.visible, $st0.total)
                exit 4
            }
            Write-Host ("闸门出现：require={0} has={1} pending={2} unlocked={3} step={4} total={5}（仓库未开，符合 C# Show()）" -f `
                    $st0.require_password, $st0.has_password, $st0.pending_open_after_set, $st0.unlocked, $st0.pwd_step, $st0.total)
            Shot '0a_password_gate'
            # 先点一下 `MirInputBox` 的输入区拿到焦点：`click` 是**分帧注入**（phase3 才会真正派发），
            # 点 NPC 行那次 click 的 phase3 会落在闸门开框**之后**，把输入焦点清掉 ⇒ 直接 `type_text`
            # 打进去的字会没人接（实测 body_len=0，状态机停在 SetNew）。玩家也是先点框再打字。
            # 输入区绝对坐标 = 面板(368,306) + InputTextBox(23,86) 240x19 的中心。
            # 鼠标点击这条路要**核验焦点**再打字：自动化里 `click` 是分帧注入，
            # 后续相位/其它控件可能把焦点又清掉（实测：点完 400ms 后 active=None）。
            # 判据直接读 `storage_probe.text_input_active`（= `TextInputState.active`），
            # 不对就重点，拿到焦点才继续。
            $focused = $false
            foreach ($tryFocus in 1..6) {
                Rpc 'click' @{ x = 511; y = 401 } | Out-Null
                Start-Sleep -Milliseconds 300
                $stFocus = Rpc 'storage_probe'
                if ($stFocus.text_input_active -eq 40) { $focused = $true; break }
            }
            Write-Host ("聚焦：input_box_open={0} active={1} text_len={2}（试 {3} 次，focused={4}）" -f `
                    $stFocus.input_box_open, $stFocus.text_input_active, $stFocus.input_box_text_len, $tryFocus, $focused)
            if (-not $focused) {
                Write-Host 'FAIL(A2a): 点输入区拿不到焦点（TextInputState.active != 40）'
                exit 4
            }
            # 打 ASCII 密码前必须确认输入法是**英文**模式：中文模式下字母进拼音组合、
            # 数字选候选（实测把「阿保存」打进密码框）。`ime_probe.enabled=true` 时按一次
            # Shift（单按切换中/英，C# 客户端同款）再继续。
            $ime = Rpc 'ime_probe'
            if ($ime.enabled) {
                Rpc 'key' @{ key = 'shift' } | Out-Null
                Start-Sleep -Milliseconds 300
                $ime2 = Rpc 'ime_probe'
                Write-Host ("输入法：中文 → 切英文（enabled={0}）" -f $ime2.enabled)
            }
            Rpc 'type_text' @{ text = 'abc123' } | Out-Null
            Rpc 'key' @{ key = 'enter' } | Out-Null
            Start-Sleep -Milliseconds 900
            $stMid = Rpc 'storage_probe'
            Write-Host ("第一遍输入后 step={0}（期望 SetConfirm{{ new: ... }}）" -f $stMid.pwd_step)
            if ("$($stMid.pwd_step)" -notlike '*SetConfirm*') {
                Write-Host ('FAIL(A2): 输完新密码后状态机没走到「等确认」（step=' + $stMid.pwd_step + '）')
                exit 4
            }
            Rpc 'type_text' @{ text = 'abc123' } | Out-Null
            Rpc 'key' @{ key = 'enter' } | Out-Null
            $gateSeen = $true
            continue
        }
        # 已设过密码（第二次起跑）→ C# `Show()` 走 `PromptStorageUnlock`：同样先不开窗，
        # 输入密码过闸后才由服务端 `UserStorage` 开窗。输入的密码就是本夹具第一次设的那个。
        if ($st0.require_password -and $st0.has_password -and -not $st0.unlocked -and $st0.unlock_prompt_open -and -not $unlockSeen) {
            Write-Host ("解锁提示出现：step={0} unlocked={1} total={2}（未输入前不开窗）" -f $st0.pwd_step, $st0.unlocked, $st0.total)
            if ($st0.visible) {
                Write-Host ("FAIL(A3): 未解锁时仓库窗不该显示（visible={0}）" -f $st0.visible)
                exit 4
            }
            Shot '0c_unlock_prompt'
            Rpc 'click' @{ x = 511; y = 401 } | Out-Null   # 同上：先点输入区拿焦点
            Start-Sleep -Milliseconds 400
            $imeU = Rpc 'ime_probe'
            if ($imeU.enabled) { Rpc 'key' @{ key = 'shift' } | Out-Null; Start-Sleep -Milliseconds 300 }
            Rpc 'type_text' @{ text = 'abc123' } | Out-Null
            Rpc 'key' @{ key = 'enter' } | Out-Null
            $unlockSeen = $true
            continue
        }
        # 「窗真开」的判据：`visible`（Show() 的结果）+ 内容已在（total != 0）
        if ($st0.visible -and $st0.total -ne 0) { $opened = $true; break }
    }
    if ($opened) { break }
    Write-Host ("[attempt {0}] 点了 [@Storage] 但 storage_probe.total 仍为 0——重试" -f $attempt)
}
    if (-not $opened) {
        Write-Host ('FAIL(A): 3 次尝试后仓库窗仍未开（storage_probe.visible=false）；npc=' + $npc.name)
        exit 4
    }
    if ($gateSeen) { Write-Host '闸门：已用 MirInputBox 走完「新密码 → 确认」，客户端自动开窗 ✅' }
    if ($unlockSeen) { Write-Host '闸门：已用 MirInputBox 输入密码解锁，客户端开窗 ✅' }
Write-Host ("storage open: total={0} used={1} visible={2}" -f $st0.total, $st0.used, $st0.visible)
Shot '1_storage_open'

# 存入：背包格 srcCell → 仓库空格（第一个 None）。仓库格号从 occupied 反推空格。
$dstCell = 0
while (($st0.occupied | Where-Object { $_.cell -eq $dstCell })) { $dstCell++ }
Write-Host ("store: bag[{0}] -> storage[{1}]" -f $srcCell, $dstCell)
Rpc 'storage_store' @{ from = $srcCell; to = $dstCell } | Out-Null
# 轮询到判据成立（服务端 StoreItem 生效 + 两个探针都真的有回包）——单次「sleep 2 后读一次」
# 会踩两类假 FAIL：① 包晚到；② 探针偶发空回包（debug 客户端帧重时实测出现过）。
$bag1 = $null; $st1 = $null
foreach ($i in 1..10) {
    Start-Sleep 1
    $b = Rpc 'bag_probe'; $s = Rpc 'storage_probe'
    if ($null -ne $b -and $null -ne $s) {
        $bag1 = $b; $st1 = $s
        if ((ItemCount $s) -eq ((ItemCount $st0) + 1) -and (ItemCount $b) -eq ((ItemCount $bag0) - 1)) { break }
    }
}
$bagItems0 = ItemCount $bag0; $stItems0 = ItemCount $st0
Write-Host ("after store: bag.used={0} storage.used={1}（件数 {2}→{3} / {4}→{5}）storage.occupied={6}" -f `
        $bag1.used, $st1.used, $bagItems0, (ItemCount $bag1), $stItems0, (ItemCount $st1),
        (($st1.occupied | ForEach-Object { "$($_.cell):$($_.name)x$($_.count)" }) -join ','))
$c1 = ((ItemCount $st1) -eq ($stItems0 + 1))
$c2 = ((ItemCount $bag1) -eq ($bagItems0 - 1))
$c3 = (@($st1.occupied | Where-Object { $_.cell -eq $dstCell }).Count -eq 1)
Write-Host ("  store sub-checks（按件数）: storage+1={0} bag-1={1} item-at-{2}={3}" -f $c1, $c2, $dstCell, $c3)
$stored = $c1 -and $c2 -and $c3
Shot '2_stored'

# 取回：仓库格 dstCell → 原背包格
Rpc 'storage_take' @{ from = $dstCell; to = $srcCell } | Out-Null
$bag2 = $null; $st2 = $null
foreach ($i in 1..10) {
    Start-Sleep 1
    $b = Rpc 'bag_probe'; $s = Rpc 'storage_probe'
    if ($null -ne $b -and $null -ne $s) {
        $bag2 = $b; $st2 = $s
        if ((ItemCount $s) -eq $stItems0 -and (ItemCount $b) -eq $bagItems0) { break }
    }
}
Write-Host ("after take: bag.used={0} storage.used={1}（件数 {2}/{3}）" -f $bag2.used, $st2.used, (ItemCount $bag2), (ItemCount $st2))
$t1 = ((ItemCount $st2) -eq $stItems0)
$t2 = ((ItemCount $bag2) -eq $bagItems0)
Write-Host ("  take sub-checks: storage回0={0} bag回满={1}" -f $t1, $t2)
$taken = $t1 -and $t2
Shot '3_taken'

Write-Host ("VERDICT store={0} take={1}" -f $(if ($stored) { 'PASS' } else { 'FAIL' }), $(if ($taken) { 'PASS' } else { 'FAIL' }))
if (-not ($stored -and $taken)) { exit 5 }

} finally {
    # 收尾：只清自己那份唯一命名的客户端（不再依赖"下一次运行按公共名清场"——那会误杀别人）。
    Get-CimInstance Win32_Process -Filter "Name='l5e_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
