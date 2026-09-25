#Requires -Version 5.1
<#
.SYNOPSIS
  `#2961` 项③「中文输入法候选词乱码」的**实机**判据：只读探针 `ime_probe`。

.DESCRIPTION
  判据（取自 #2961 项③ 任务书）：**候选条 / 聊天输入框 / 聊天行**三处动态中文文本，
  其 `TextFont` 字体句柄必须等于**共享 CJK 主字体**（`shared_cjk_font`）的同一个句柄
  —— EQUAL，不是"非空"。理由：主字体是 Arial（C# `Settings.FontName`），Arial 无 CJK 字形，
  动态改写文本在重排版时会退化成 .notdef 豆腐（实机表现为候选词/聊天乱码）。

  另加**仪器自检**：同状态连读两次读数必须一致（探针输出按实体种类固定取一个，字段固定）。

  阳性对照（实做记录在 PR 里）：把候选条字体改成 `UiFont`（Arial）→ 本夹具必须红
  （`candidate_font != cjk_font`）。

.EXAMPLE
  pwsh tools/acceptance/l5z_ime_font.ps1
#>
param(
    [string]$ClientExe = '',
    [string]$Worktree = '',
    [int]$ControlPort = 9072,
    [int]$TimeoutSec = 60,
    [string]$Tag = 'imefont'
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
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5z_ime_font'
$json = Join-Path $PSScriptRoot 'l5z_ime_font_results.json'

. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5z_ime_font' -TimeoutSec 900)) {
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
        # 候选条实体在字体就绪后才 spawn；给它几秒（首帧可能还没建）
        $p1 = $null
        for ($i = 1; $i -le 20; $i++) {
            Start-Sleep 1
            $p1 = Rpc 'ime_probe'
            if ($p1 -and $p1.candidate_font) { break }
        }
        $p2 = Rpc 'ime_probe'
        $s1 = ($p1 | ConvertTo-Json -Compress -Depth 6)
        $s2 = ($p2 | ConvertTo-Json -Compress -Depth 6)
        Write-Host ("[J0 仪器自检] 同状态连读两次一致：{0}" -f ($s1 -eq $s2))
        if ($s1 -ne $s2) { $fail += '仪器自检失败：同状态两次读数不一致' }
        Write-Host ("读数：{0}" -f $s1)
        if ($null -eq $p1 -or -not $p1.ok) {
            $fail += 'ime_probe 不可用（未编译进二进制？）'
        } else {
            if (-not $p1.cjk_font) { $fail += 'J1: 共享 CJK 主字体句柄为空（字体未加载？）' }
            foreach ($pair in @(
                    @{ k = 'candidate_font'; n = '候选条' },
                    @{ k = 'input_box_font'; n = '聊天输入框' },
                    @{ k = 'chat_font'; n = '聊天行' })) {
                $v = $p1.($pair.k)
                if (-not $v) { $fail += ("J1: {0} 字体句柄为空（实体未 spawn？）" -f $pair.n); continue }
                if ($v -ne $p1.cjk_font) {
                    $fail += ("J1: {0} 字体 != 共享 CJK 主字体（{1} vs {2}）——Arial 无 CJK 字形会豆腐" -f $pair.n, $v, $p1.cjk_font)
                } else {
                    Write-Host ("   {0} 字体 == CJK 主字体 ✓" -f $pair.n)
                }
            }
            if (-not $p1.all_cjk) { $fail += 'J1: all_cjk=false（三处未全部同源到 CJK 主字体）' }
        }
    }
} finally {
    Stop-Process -Id $proc.Id -Force -EA SilentlyContinue
    Exit-E2eLock
}

$result = [ordered]@{
    ok       = ($fail.Count -eq 0)
    tag      = $Tag
    probe    = $p1
    failures = $fail
}
$result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $json -Encoding UTF8
Write-Host ("结论 JSON: " + $json)
if ($fail.Count -gt 0) {
    Write-Host ('FAIL(1): ' + ($fail -join '; '))
    exit 1
}
Write-Host '=== 全部 PASS ==='
exit 0