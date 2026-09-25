# l5ac_newchar_create.ps1 —— 真机建角端到端（owner 反馈"无法创建角色"）
#
# 判据（任一不成立即 FAIL）：
#   ① 负对照：**2 字名字**（客户端规则 3..=15）点确定 → 不得建出角色（既没有 DB 行、也没有"新建角色成功"）
#   ② 正路：3 字中文名 → 点确定（真实点击）→ DB 出现该角色 + 客户端日志 `✅ 新建角色成功`
# 依赖：master 服务端在 7000；客户端 control RPC 支持 `type_text` 与 `click.button`（见 control.rs）。
# 幂等：目标账号已建过同名/任意角色时，② 记 SKIP（不假绿），① 仍照跑。
param(
    [string]$User = 'bevychar',
    [string]$Pass = '123456',
    [string]$Name = '小明明',
    [string]$ClientHome = 'E:\Users\gxh\Documents\GitHub\Crystal-wt-blend',
    # 留空 = 从 `%TEMP%\e2e_run\server_build_record.json` 的 `data_root` 解析（= **正在跑的那个服务端**用的库）。
    # 判据来源必须是受测实例那一份（本仓踩过：判据读主检出的库、受测服务端却跑在别的目录 → 假红）。
    [string]$ServerWorkDir = '',
    # 幂等开关：跑之前把这个测试账号（`$User`）的角色清掉，好让建角正路能反复跑。
    # **默认关**，且只动 `$User` 这一个账号的角色行；生产库别开。
    [switch]$CleanupCreated
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = $PSScriptRoot
. "$acc\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5ac_newchar_create' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }
$clientProc = $null
try {
    if (-not (Get-NetTCPConnection -LocalPort 7000 -State Listen -EA SilentlyContinue)) {
        Write-Host 'FAIL(9): 7000 上没有服务端'; exit 9
    }
    if (-not $ServerWorkDir) {
        $rec = Join-Path $env:TEMP 'e2e_run\server_build_record.json'
        if (-not (Test-Path $rec)) { Write-Host "FAIL(9): 没有 $rec —— 请先 pwsh tools\ops\restart_e2e_server.ps1（或用 -ServerWorkDir 显式指定）"; exit 9 }
        $ServerWorkDir = (Get-Content $rec -Raw | ConvertFrom-Json).data_root
    }
    $db = Join-Path $ServerWorkDir 'Data\crystal.db'
    if (-not (Test-Path $db)) { Write-Host "FAIL(9): 找不到库 $db"; exit 9 }
    Write-Host ("[前置] 受测库 = {0}（服务端部署记录 data_root）" -f $db)
    if ($CleanupCreated) {
        Write-Host ("WARN: -CleanupCreated 会删掉账号 '{0}' 的**全部角色行**（测试账号专用）" -f $User)
        $del = "delete from characters where account_username='$User'"
        py -3.12 -c "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute(sys.argv[2]); c.commit(); print('deleted rows:', c.total_changes)" $db $del
    }
    function DbCount([string]$name) {
        $sql = if ($name) {
            "select count(*) from characters where account_username='$User' and name='$name'"
        } else {
            "select count(*) from characters where account_username='$User'"
        }
        [int](& py -3.12 "$acc\dbq.py" --db $db $sql | Select-Object -First 1)
    }
    function Rpc([string]$m, [hashtable]$q = @{}) {
        $c = New-Object Net.Sockets.TcpClient
        $c.Connect('127.0.0.1', 9000)
        $s = $c.GetStream()
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        ($l | ConvertFrom-Json).result
    }
    # 选角界面「新建角色」按钮 = Title[343] @(296,736) 100x25（select.rs: bottom_xs[1], y=768-32）
    $newCharBtn = @{ x = 346; y = 748 }
    # 建角对话框里名字框 (DLG_X+325, DLG_Y+268, 240x20)；OK 按钮 Title[360] @(DLG_X+160, DLG_Y+425) 60x25
    $nameField = @{ x = 560; y = 432 }
    $okBtn     = @{ x = 408; y = 591 }

    $exeSrc = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"
$&
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5ac_newchar_create'
    if (-not (Test-Path $exeSrc)) { Write-Host "FAIL(9): 找不到客户端 $exeSrc"; exit 9 }

    function Start-Client([string]$logTag) {
        Get-CimInstance Win32_Process -Filter "Name='l5ac_client.exe'" -EA SilentlyContinue |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
        try { New-Item -ItemType HardLink -Path $exe -Target $exeSrc -Force -ErrorAction Stop | Out-Null }
        catch { Copy-Item -LiteralPath $exeSrc -Destination $exe -Force }
        $env:BEVY_OPEN_NEWCHAR = '1'
        $out = "$env:TEMP\l5ac_$logTag.out.log"
        $err = "$env:TEMP\l5ac_$logTag.err.log"
        Remove-Item $out, $err -Force -EA SilentlyContinue
        $p = Start-Process -FilePath $exe -ArgumentList '--real-net', '--auto-enter', '--e2e-user', $User, '--e2e-pass', $Pass `
            -WorkingDirectory "$ClientHome\Client-Bevy" -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
        # 等"登录成功"（角色数由客户端日志给出）
        foreach ($i in 1..90) {
            Start-Sleep 1
            if (Select-String -LiteralPath $err -Pattern '登录成功' -Quiet -EA SilentlyContinue) { break }
        }
        $login = (Select-String -LiteralPath $err -Pattern '登录成功' | Select-Object -Last 1).Line
        if (-not $login) { Write-Host 'FAIL(9): 客户端 90s 未登录成功'; Stop-Process -Id $p.Id -Force -EA SilentlyContinue; exit 9 }
        Write-Host ("[$logTag] " + $login.Trim())
        Start-Sleep -Seconds 2   # 等选角界面 UI 完成 spawn
        return @{ proc = $p; err = $err }
    }

    $fail = @()
    $skipCreate = $false

    # 打开建角对话框：点选角界面的「新建角色」，最多重试 10 次。
    # 为什么要重试：选角 UI 的按钮是登录后才 spawn 的，客户端刚进选角界面时点会打空
    # （实测同一支夹具两次跑，一次成功一次 visible=false —— 是时机问题、不是产品缺陷）。
    # 判据只看 `new_char_probe.visible`，不看"点了几次"。
    function Open-NewCharDialog {
        for ($i = 1; $i -le 10; $i++) {
            Rpc 'click' $newCharBtn | Out-Null
            Start-Sleep -Milliseconds 700
            $p = Rpc 'new_char_probe'
            if ($p.visible) { return $p }
        }
        return $null
    }

    # ---------- ① 负对照：2 字名字不得建出角色 ----------
    $before = DbCount $null
    $s1 = Start-Client 'neg2char'
    $clientProc = $s1.proc
    $negOpen = Open-NewCharDialog
    Write-Host ('[neg] probe(开窗后) → ' + ($negOpen | ConvertTo-Json -Compress))
    if (-not $negOpen) { $fail += '负对照：10 次点「新建角色」都没打开对话框' }
    Write-Host ('[neg] type_text 小明 → ' + (Rpc 'type_text' @{ text = '小明' } | ConvertTo-Json -Compress))
    Write-Host ('[neg] probe(输入后) → ' + (Rpc 'new_char_probe' | ConvertTo-Json -Compress))
    Write-Host ('[neg] click 确定   → ' + (Rpc 'click' $okBtn | ConvertTo-Json -Compress))
    Write-Host ('[neg] probe(点后)   → ' + (Rpc 'new_char_probe' | ConvertTo-Json -Compress))
    Start-Sleep -Seconds 2
    $afterNeg = DbCount $null
    $negLog = (Select-String -LiteralPath $s1.err -Pattern '新建角色成功' -Quiet -EA SilentlyContinue)
    if ($afterNeg -ne $before) { $fail += "负对照：2 字名字竟然建出了角色（$before→$afterNeg）" }
    if ($negLog) { $fail += '负对照：2 字名字竟然收到"新建角色成功"' }
    Stop-Process -Id $s1.proc.Id -Force -EA SilentlyContinue
    $clientProc = $null
    Start-Sleep -Milliseconds 800

    # ---------- ② 正路：3 字中文名 → 点确定 → 建出角色 ----------
    if ((DbCount $null) -gt 0) {
        $skipCreate = $true
        Write-Host 'SKIP(②): 该账号已有角色，跳过建角正路（负对照仍照跑）'
    } else {
        $s2 = Start-Client 'create'
        $clientProc = $s2.proc
        $probeOpen = Open-NewCharDialog
        Write-Host ('[pos] probe(开窗后) → ' + ($probeOpen | ConvertTo-Json -Compress))
        if (-not $probeOpen) { $fail += '正路：10 次点「新建角色」都没打开对话框' }
        if ($probeOpen -and -not $probeOpen.visible) { $fail += '正路：点了「新建角色」但对话框 visible=false' }
        Write-Host ("[pos] type_text {0} → {1}" -f $Name, (Rpc 'type_text' @{ text = $Name } | ConvertTo-Json -Compress))
        $probeTyped = Rpc 'new_char_probe'
        Write-Host ('[pos] probe(输入后) → ' + ($probeTyped | ConvertTo-Json -Compress))
        if ($probeTyped.name -ne $Name) { $fail += ("正路：type_text 后名字框内容 = '{0}'（期望 '{1}'）" -f $probeTyped.name, $Name) }
        Write-Host ('[pos] click 确定 → ' + (Rpc 'click' $okBtn | ConvertTo-Json -Compress))
        Write-Host ('[pos] probe(点后) → ' + (Rpc 'new_char_probe' | ConvertTo-Json -Compress))
        # 等**两件事**同时成立：DB 出现该角色 + 客户端处理了 `S.NewCharacterSuccess`
        # （后者证明客户端真的走完了成功分支；先前的写法在客户端还没打印就把它杀了 → 假 FAIL）
        $created = $false
        for ($i = 1; $i -le 20; $i++) {
            Start-Sleep 1
            if ((DbCount $Name) -ge 1) { $created = $true; break }
        }
        $okLog = $false
        for ($i = 1; $i -le 15; $i++) {
            Start-Sleep 1
            if (Select-String -LiteralPath $s2.err -Pattern '新建角色成功' -Quiet -EA SilentlyContinue) { $okLog = $true; break }
        }
        if (-not $created) { $fail += "正路：点确定后 DB 里没有角色 '$Name'（日志尾：$((Get-Content $s2.err -Tail 3 -EA SilentlyContinue) -join ' | ')）" }
        if (-not $okLog) { $fail += '正路：客户端日志没有"新建角色成功"' }
        Stop-Process -Id $s2.proc.Id -Force -EA SilentlyContinue
        $clientProc = $null
    }

    $negVerdict = if (@($fail | Where-Object { $_ -like '负对照*' }).Count -gt 0) { 'FAIL' } else { 'PASS' }
    $posVerdict = if ($skipCreate) { 'SKIP(已有角色)' }
                  elseif (@($fail | Where-Object { $_ -like '正路*' }).Count -gt 0) { 'FAIL' }
                  else { 'PASS' }
    if ($fail.Count -gt 0) { foreach ($f in $fail) { Write-Host ("FAIL: " + $f) } }
    Write-Host ("VERDICT neg2char={0} create={1}" -f $negVerdict, $posVerdict)
    if ($fail.Count -gt 0) { exit 1 }
    exit 0
} finally {
    if ($clientProc -and -not $clientProc.HasExited) { Stop-Process -Id $clientProc.Id -Force -EA SilentlyContinue }
    Start-Sleep -Milliseconds 500
    Get-CimInstance Win32_Process -Filter "Name='l5ac_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
    Exit-E2eLock
}
