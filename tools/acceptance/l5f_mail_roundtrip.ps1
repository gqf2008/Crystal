# l5f_mail_roundtrip.ps1 — ⑤ 邮件仓库闭环（邮件半段）：A 发 → B 取包裹 → B 收取附件
#
# 三段式（不是两段）——原版 C# 的两条规则决定了它必须这样走：
#   1) **不能给自己发**（服务端 mail.rs 直接拒绝并回「不能给自己发送邮件」）→ 必须 A→B 两个角色。
#   2) `MailBox.collect_attachment` 要求 `mail.collected == true`，而 `collected` 的语义是
#      **"已从邮局取回"**（C# `CollectParcelKey`，`NPCScript.cs:1081-1093`）——玩家得先去邮局
#      NPC 点 `<Collect/@CollectParcel>` 把包裹取回，之后点「收取」才会把金币/附件入包。
#      直接 collect 会得到 `ParcelCollected: result=-1`（本条就是这么踩出来的）。
#
# 收件人为什么用 bevychar（test 账号）：邮局 NPC 只在 map 6(PastBichon)@94,157 与 map 5 上，
# 而能 `@mapmove` 过去的只有 admin 账号；`test` 是 admin（accounts.admin_account=1）。
# 发件人用 bevy2char（账号 bevy2，非 admin）——发信不需要传送权限。
#
# 判据（全部取状态）：
#   A) 发送：发件人客户端 gold 减少发送额；DB mail 表出现新行（character_name=收件人、gold=发送额、collected=0）
#   B) 取包裹：邮局 NPC 点 <Collect/@CollectParcel> 后 DB 该行 collected 0→1
#   C) 到达+读取：收件人 mail_probe 能看到该封；mail_read 后 detail.subject/body 与发出的一致
#   D) 收取：mail_collect 后收件人 bag_probe.gold 恰好增加发送额
param(
    [string]$SenderUser = 'bevy2',
    [string]$SenderPass = '123456',
    [string]$SenderChar = 'bevy2char',
    [string]$ReceiverUser = 'test',
    [string]$ReceiverPass = '123456',
    [string]$ReceiverChar = 'bevychar',
    [int]$Gold = 123,
    [string]$PostMap = '6',
    [int]$PostX = 94,
    [int]$PostY = 157,
    # 客户端构建根（其 Client-Bevy\target\debug\client_bevy.exe）。
    # 2026-09-24 补：本夹具原先**硬编码 wt-p3**，与其它 l5* 夹具不一致——
    # 换 worktree 时不传参就会拿旧客户端跑，出假红/假绿（同批 l5a 就因此假红过一次）。
    [string]$ClientHome = ''
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$wt = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-p3'
if (-not $ClientHome) { $ClientHome = $wt }
$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
$wd = "$ClientHome\Client-Bevy"

function Rpc([string]$m, [hashtable]$q = @{}) {
    $c = New-Object Net.Sockets.TcpClient; $c.Connect('127.0.0.1', 9000); $s = $c.GetStream()
    $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc='2.0'; id=1; method=$m; params=$q } | ConvertTo-Json -Compress) + "`n")
    $s.Write($b, 0, $b.Length); $s.Flush()
    $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close(); ($l | ConvertFrom-Json).result
}
function Db([string]$sql) { (& python "$wt\tools\acceptance\dbq.py" $sql) -join "`n" }
function Start-Client([string]$user, [string]$pass, [string]$tag) {
    Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 900
    Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$user,'--e2e-pass',$pass `
        -WorkingDirectory $wd `
        -RedirectStandardOut "$acc\l5f_${tag}_client.log" -RedirectStandardError "$acc\l5f_${tag}_client.err.log" | Out-Null
    foreach ($i in 1..60) { Start-Sleep 1; try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { return $st } } catch {} }
    throw "客户端 $tag 未进图"
}

if (-not (Get-Process -Name mir2_server -EA SilentlyContinue)) { Write-Host '服务端未运行'; exit 9 }
$stamp = [int][double]::Parse((Get-Date -UFormat %s))
$subject = "e2e-mail-$stamp"

# ---------- 第一段：发件人登录并发信 ----------
$stA = Start-Client $SenderUser $SenderPass 'send'
$goldA0 = (Rpc 'bag_probe').gold
Write-Host ("[A] {0}({1}) 进图 tile=({2},{3}) gold={4}" -f $SenderChar, $SenderUser, $stA.tile_x, $stA.tile_y, $goldA0)
$r = Rpc 'mail_send' @{ to = $ReceiverChar; message = "$subject`n邮件闭环 e2e 正文"; gold = $Gold }
Write-Host ("[A] mail_send -> {0}" -f ($r | ConvertTo-Json -Compress))
Start-Sleep -Seconds 2
$goldA1 = (Rpc 'bag_probe').gold
$row = Db "select mail_id, character_name, sender_name, subject, gold, collected from mail where subject='$subject'"
Write-Host ("[A] DB: {0}" -f $row)
$mailId = 0
if ($row -match '^\((\d+),') { $mailId = [int]$Matches[1] }
$sent = ($row -match ", $Gold, 0\)") -and (($goldA0 - $goldA1) -eq $Gold) -and ($mailId -gt 0)
Write-Host ("[A] 判据: DB 行(含金额/未取回)={0} 发件人金币 {1}->{2} mail_id={3}" -f ($row -match ", $Gold, 0\)"), $goldA0, $goldA1, $mailId)

# ---------- 第二段：收件人到邮局把包裹取回 ----------
$stB = Start-Client $ReceiverUser $ReceiverPass 'recv'
$goldB0 = (Rpc 'bag_probe').gold
Write-Host ("[B] {0}({1}) 进图 tile=({2},{3}) gold={4}" -f $ReceiverChar, $ReceiverUser, $stB.tile_x, $stB.tile_y, $goldB0)
# 取包裹前记一次「已从邮局取回」标志（客户端探针看到的是服务端权威状态；
# **不要用 DB 判据**：邮件在 PlayerActor 内存邮箱里，DB 是延迟落盘的——实测取回后
# DB 仍写 collected=0，直到角色登出/自动保存才刷新，拿它当判据会得到假 FAIL。）
$entry0 = (Rpc 'mail_probe').mails | Where-Object { $_.subject -eq $subject } | Select-Object -First 1
$collectedBefore = if ($entry0) { [bool]$entry0.collected } else { $null }
Write-Host ("[B] 取包裹前 collected={0}" -f $collectedBefore)
Rpc 'chat' @{ message = "@mapmove $PostMap $PostX $PostY" } | Out-Null
Start-Sleep 4
$stB2 = Rpc 'state'
Write-Host ("[B] @mapmove {0} {1} {2} -> tile=({3},{4})" -f $PostMap, $PostX, $PostY, $stB2.tile_x, $stB2.tile_y)
$npc = (Rpc 'nearby' @{ radius = 3000 }).entities | Where-Object { $_.kind -eq 'npc' } | Sort-Object dist | Select-Object -First 1
if (-not $npc) { Write-Host '[B] FAIL: 邮局旁没有 NPC'; exit 1 }
Write-Host ("[B] 邮局 NPC={0} id={1} dist={2}" -f $npc.name, $npc.object_id, $npc.dist)
Rpc 'npc_call' @{ object_id = $npc.object_id; key = '[@MAIN]' } | Out-Null
Start-Sleep 2
$rows = Rpc 'npc_rows'
$collect = $rows.links | Where-Object { $_.key -eq '[@CollectParcel]' } | Select-Object -First 1
if (-not $collect) {
    Write-Host ('[B] FAIL: 该 NPC 菜单里没有 [@CollectParcel]；links=' + (($rows.links | ForEach-Object { $_.key }) -join ','))
    exit 2
}
Rpc 'click' @{ x = $collect.cx; y = $collect.cy } | Out-Null
Start-Sleep -Seconds 2
$entry1 = (Rpc 'mail_probe').mails | Where-Object { $_.subject -eq $subject } | Select-Object -First 1
$collectedAfter = if ($entry1) { [bool]$entry1.collected } else { $null }
$row2 = Db "select mail_id, collected, gold from mail where mail_id=$mailId"
Write-Host ("[B] 取包裹后 collected={0}（判据）；DB={1}（延迟落盘，仅参考）" -f $collectedAfter, $row2)
$released = (($collectedBefore -eq $false) -and ($collectedAfter -eq $true))

# ---------- 第三段：读取并收取附件 ----------
$probe = Rpc 'mail_probe'
$entry = $probe.mails | Where-Object { $_.subject -eq $subject } | Select-Object -First 1
if (-not $entry) { Write-Host ('[C] FAIL: 收件箱没有 subject=' + $subject); exit 3 }
Write-Host ("[C] 命中 id={0} sender={1} gold={2} collected={3}" -f $entry.mail_id, $entry.sender, $entry.gold, $entry.collected)
Rpc 'mail_read' @{ mail_id = $entry.mail_id } | Out-Null
Start-Sleep -Milliseconds 1200
$d = (Rpc 'mail_probe').detail
$bodyOk = ($d.mail_id -eq $entry.mail_id) -and ($d.body -like "*$subject*")
Write-Host ("[C] detail id={0} subject={1} gold={2} bodyOk={3}" -f $d.mail_id, $d.subject, $d.gold, $bodyOk)
Rpc 'mail_collect' @{ mail_id = $entry.mail_id } | Out-Null
Start-Sleep -Seconds 2
$goldB1 = (Rpc 'bag_probe').gold
$delta = [int]$goldB1 - [int]$goldB0
$row3 = Db "select mail_id, collected, gold from mail where mail_id=$mailId"
Write-Host ("[D] collect 后 收件人 gold {0}->{1} delta={2}(期望 {3})；DB: {4}" -f $goldB0, $goldB1, $delta, $Gold, $row3)

$okA = [bool]$sent
$okB = [bool]$released
$okC = [bool]$bodyOk
$okD = ($delta -eq $Gold)
Write-Host ("VERDICT send={0} parcel_release={1} read={2} collect_gold_delta={3}" -f `
    $(if ($okA) { 'PASS' } else { 'FAIL' }), $(if ($okB) { 'PASS' } else { 'FAIL' }), `
    $(if ($okC) { 'PASS' } else { 'FAIL' }), $(if ($okD) { 'PASS' } else { 'FAIL' }))
if (-not ($okA -and $okB -and $okC -and $okD)) { exit 5 }
