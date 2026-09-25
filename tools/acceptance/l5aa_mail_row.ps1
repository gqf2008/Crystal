#Requires -Version 5.1
<#
.SYNOPSIS
  `#3120` 项① 邮件列表行的**渲染 vs 模型**对账夹具（C# `MailItemRow`）。

.DESCRIPTION
  判据：`mail_probe` 同时给出
    - 每封邮件的**模型 + C# 派生渲染意图**（`icon{lib,index}` / `unread_x` / `info_text` / 旗标）；与
    - 每行 7 个视觉层实体（`MailRowSlot`）的**渲染真值**（可见性 / left,top / 图标实际应用的库帧 / 文本）。
  夹具逐行逐层比对两者必须一致（等价于"渲染按 C# 规则画了"）。

  另有**仪器自检**：同状态连读两次读数一致（探针按 (row, role) 排序保证确定性）。

  数据用 mock 的 `--mail-many` 预置 21 封（每封带 1 个附件）——不起服务端、不占 e2e 账号。

.EXAMPLE
  pwsh tools/acceptance/l5aa_mail_row.ps1
#>
param(
    [string]$ClientExe = '',
    [string]$Worktree = '',
    [int]$ControlPort = 9076,
    [int]$TimeoutSec = 60,
    [string]$Tag = 'mailrow'
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
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5aa_mail_row'
$json = Join-Path $PSScriptRoot 'l5aa_mail_row_results.json'

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5aa_mail_row' -TimeoutSec 900)) {
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
    -ArgumentList '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', '--control-port', "$ControlPort", '--mail-many' `
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
    if (-not $st -or $null -eq $st.tile_x) { Write-Host 'FAIL(2): 未进场'; $fail += 'not_entered_game' }
    else {
        Rpc 'dialog' @{ kind = 'mail'; action = 'open' } | Out-Null
        $p = $null
        for ($i = 1; $i -le 20; $i++) {
            Start-Sleep 1
            $p = Rpc 'mail_probe'
            if ($p -and $p.count -ge 21) { break }
        }
        $p2 = Rpc 'mail_probe'
        $s1 = ($p | ConvertTo-Json -Compress -Depth 8)
        $s2 = ($p2 | ConvertTo-Json -Compress -Depth 8)
        Write-Host ("[J0 仪器自检] 同状态连读两次一致：{0}" -f ($s1 -eq $s2))
        if ($s1 -ne $s2) { $fail += '仪器自检失败：同状态两次读数不一致' }
        if (-not $p -or $p.count -lt 21) { $fail += ("邮件没到齐（count={0}）" -f $(if ($p) { $p.count } else { 'null' })) }
        else {
            $start = [int]$p.page_start
            $rows = @{}
            foreach ($r in $p.row_render) { $rows["$($r.row)|$($r.role)"] = $r }
            $checked = 0
            for ($i = 0; $i -lt 10; $i++) {
                $idx = $start + $i
                $m = if ($idx -lt $p.mails.Count) { $p.mails[$idx] } else { $null }
                $expect = @{
                    Selected = ($null -ne $m -and $p.selected -eq $idx)
                    Icon     = ($null -ne $m)
                    Unread   = ($null -ne $m -and [bool]$m.unread)
                    Locked   = ($null -ne $m -and [bool]$m.locked)
                    Parcel   = ($null -ne $m -and -not [bool]$m.collected)
                }
                foreach ($role in 'Selected', 'Icon', 'Unread', 'Locked', 'Parcel') {
                    $k = "$i|$role"
                    if (-not $rows.ContainsKey($k)) { $fail += "行 $i 缺 $role 实体"; continue }
                    $got = [bool]$rows[$k].visible
                    if ($got -ne [bool]$expect[$role]) {
                        $fail += ("行 {0} {1} 可见性: 渲染={2} 期望={3}" -f $i, $role, $got, $expect[$role])
                    }
                    $checked++
                }
                if ($null -ne $m) {
                    $ri = $rows["$i|Icon"]
                    if ($ri.icon_lib -ne $m.icon.lib -or [int]$ri.icon_index -ne [int]$m.icon.index) {
                        $fail += ("行 {0} 图标: 渲染={1}[{2}] 期望={3}[{4}]" -f $i, $ri.icon_lib, $ri.icon_index, $m.icon.lib, $m.icon.index)
                    }
                    $ru = $rows["$i|Unread"]
                    if ([math]::Abs([double]$ru.x - (10.0 + [double]$m.unread_x)) -gt 0.5) {
                        $fail += ("行 {0} 未读角标 x: 渲染={1} 期望={2}" -f $i, $ru.x, (10.0 + [double]$m.unread_x))
                    }
                    $rs = $rows["$i|Sender"]
                    if ($rs.text -ne $m.sender) { $fail += ("行 {0} 发件人文本: 渲染='{1}' 期望='{2}'" -f $i, $rs.text, $m.sender) }
                    $rf = $rows["$i|Info"]
                    if ($rf.text -ne $m.info_text) { $fail += ("行 {0} 信息文本: 渲染='{1}' 期望='{2}'" -f $i, $rf.text, $m.info_text) }
                    $checked += 3
                }
            }
            Write-Host ("对账完成：断言 {0} 项" -f $checked)
            Write-Host ("首行样本：icon={0}[{1}] unread_x={2} info='{3}'" -f $p.mails[$start].icon.lib, $p.mails[$start].icon.index, $p.mails[$start].unread_x, $p.mails[$start].info_text)
        }
    }
} finally {
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    Exit-E2eLock
}

$result = [ordered]@{ ok = ($fail.Count -eq 0); tag = $Tag; failures = $fail }
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $json -Encoding UTF8
Write-Host ("结论 JSON: " + $json)
if ($fail.Count -gt 0) { Write-Host ('FAIL(1): ' + (($fail | Select-Object -First 8) -join '; ')); exit 1 }
Write-Host '=== 全部 PASS ==='
exit 0