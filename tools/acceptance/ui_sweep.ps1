# ui_sweep.ps1 — 全 UI 交互窗口巡回验证：open → dialogs RPC 断言 → 截图 → close → 断言消失
# 前提：mir2_server 在跑；客户端以 --real-net --auto-enter 启动并进图。
param([string]$User = 'test', [string]$Pass = '123456')
$ErrorActionPreference = 'Stop'
# 客户端依赖 msys64/ucrt64 与 libpinyin 的 DLL：缺任一目录会以 0xC0000135 静默退出
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
$acc = 'E:\Users\gxh\Documents\GitHub\Crystal\tools\acceptance'
$shots = Join-Path $acc 'shots'
$exe = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy\target\debug\client_bevy.exe'
$wd  = 'E:\Users\gxh\Documents\GitHub\Crystal\Client-Bevy'

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
function Shot([string]$label) {
    $p = (Join-Path $shots "$label.png").Replace('\', '/')
    Rpc 'screenshot' @{ path = $p } | Out-Null
    Start-Sleep -Milliseconds 700
}
function OpenDialogs() { (Rpc 'dialogs').dialogs }

# DialogKind 全量（has_rpc_mapping=true 的 45 个，按枚举序）
$kinds = @(
    'inventory','character','quest_log','settings','menu','game_shop','minimap',
    'npc','group','friend','trade','inspect','npc_goods','guild','mail','ranking',
    'mentor','relationship','mount','report','hero_inventory','hero_equipment',
    'creature','item_rental','guild_territory','help','notice','buff','fishing',
    'socket','refine','craft','dura_status','roll','npc_awake','timer',
    'keyboard_layout','big_map','chat_notice','market','storage',
    'item_rental_browse','hero_manage','quest_detail','input_box'
)

# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---
# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误
# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；
# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'ui_sweep' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁
# （PowerShell 的 finally 在 exit 下也会执行——实测 -File 与会话内 & script.ps1 两种调用都成立），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {

# 只清**自己这份构建**的残留（按 exe 路径过滤）：绝不按公共名 `client_bevy` 清场——
# 那会连带杀掉别的 agent 的验收/人工 GUI 会话（BATCH #3181；`check_process_scope` 门禁会红）。
Get-CimInstance Win32_Process -Filter "Name='client_bevy.exe'" -EA SilentlyContinue |
    Where-Object { $_.ExecutablePath -eq $exe } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
$proc = Start-Process -FilePath $exe -ArgumentList '--real-net','--auto-enter','--e2e-user',$User,'--e2e-pass',$Pass `
    -WorkingDirectory $wd -RedirectStandardOut "$acc\sweep_client.log" -RedirectStandardError "$acc\sweep_client.err.log" -PassThru
$ok = $false
foreach ($i in 1..45) {
    Start-Sleep 1
    try { $st = Rpc 'state'; if ($null -ne $st.tile_x) { $ok = $true; break } } catch {}
}
if (-not $ok) { throw '未进入游戏' }
Write-Host ("进图 tile=({0},{1})" -f $st.tile_x, $st.tile_y)
Start-Sleep 2

$results = @()
$baseline = OpenDialogs
Write-Host "基线已开窗口: $($baseline -join ',')"

foreach ($k in $kinds) {
    $before = OpenDialogs
    Rpc 'dialog' @{ kind = $k; action = 'open' } | Out-Null
    Start-Sleep -Milliseconds 900
    $afterOpen = OpenDialogs
    Shot ("ui_{0}" -f $k)
    Rpc 'dialog' @{ kind = $k; action = 'close' } | Out-Null
    Start-Sleep -Milliseconds 500
    $afterClose = OpenDialogs
    $results += [pscustomobject]@{
        kind = $k
        before = ($before -join ',')
        afterOpen = ($afterOpen -join ',')
        afterClose = ($afterClose -join ',')
    }
    Write-Host ("{0,-20} open:[{1}] close:[{2}]" -f $k, ($afterOpen -join ','), ($afterClose -join ','))
}

# 别名抽查：hero_skill→hero_equipment, npc_drop→npc, trust_merchant→market
foreach ($a in @('hero_skill','npc_drop','trust_merchant')) {
    Rpc 'dialog' @{ kind = $a; action = 'open' } | Out-Null
    Start-Sleep -Milliseconds 700
    $d = OpenDialogs
    Shot ("ui_alias_{0}" -f $a)
    Write-Host ("alias {0,-16} -> open:[{1}]" -f $a, ($d -join ','))
    # 关别名目标本体
    $target = @{ hero_skill='hero_equipment'; npc_drop='npc'; trust_merchant='market' }[$a]
    Rpc 'dialog' @{ kind = $target; action = 'close' } | Out-Null
    Start-Sleep -Milliseconds 400
}

$results | ConvertTo-Json | Out-File "$acc\ui_sweep_results.json" -Encoding utf8
Stop-Process -Id $proc.Id -Force
Write-Host '== 巡回完成 =='

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
