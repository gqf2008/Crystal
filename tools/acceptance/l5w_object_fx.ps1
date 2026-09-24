#Requires -Version 5.1
<#
.SYNOPSIS
  对象特效（`S.ObjectEffect`）实机夹具——护盾/治疗/传送/冰柱/天罚…此前一律画成**纯色方块**。

.DESCRIPTION
  判据取自渲染侧**真正 spawn 的**实体（只读 RPC `spell_fx_probe` 的 `object:<SpellEffect>`
  行：库、起始帧、帧数），不是日志文本。

  原版对照（`Client/MirScenes/GameScene.cs` 的 `ObjectEffect` switch，机械生成到
  `Client-Bevy/src/game/spell_effects.rs` 的 `OBJECT_FX`）：
    MagicShieldUp → 帧动画 Magic base 3890 frames 3，且**循环**（原版 `Repeat = true`，
                    直到收到 MagicShieldDown 才清）← 断言项（**扁平库**取图路径）
    Stunned       → 帧动画 Monsters[StoningStatue] base 632 frames 10，
                    `Repeat = p.Time > 0`（time=6000 → 6 秒窗口）← 断言项（**怪物数组库**
                    取图路径 `ui_array_image`；这条路径此前从未被实机覆盖）
    Critical      → C# 里是**被注释掉的 case**（表里空切片 = 明确不画）← 本夹具的负控

  为什么用 mock：`--battle-vfx-test`（`src/auto/combat.rs`）会自动走到怪旁施法，mock 侧
  回发 `ObjectEffect(Critical)`（负控）+ `ObjectEffect(MagicShieldUp)`（断言项），
  整条链路（真实 codec 编解码 → `handle_social.rs` → `PendingEffect` → 渲染）都是客户端
  自己的代码，只有包源是 mock。选**循环**的护盾光环而不是一次性特效，是为了能跨轮询稳定
  观察到（一次性的 0.8s 窗口与 200ms 轮询之间容易擦肩）。

  退出码：0 = 断言项观察到且负控未出现；1 = FAIL（超时未见 / 负控出现）；
          2 = 前置失败（未进场 / 探针不可用）；3 = 整轮没有任何对象特效条目（前置不成立）。

  阳性对照（落地时实做）：把 `effects.rs` 的 ObjectEffect 分支改回 `PendingEffect::Burst`
  → 本夹具必然 exit 3（探针只看到方块，没有任何 `object:` 行）。
#>
param(
    [string]$ExeSrc = '',
    [int]$ControlPort = 9051,
    [string]$Worktree = '',
    [int]$TimeoutSec = 90,
    [string]$Tag = 'objfx'
)

# ---- 实机资源互斥 ----------------------------------------------------------
# 起客户端 / 登录 e2e 账号前必须先拿锁：客户端 + e2e 账号是「一次只能一组」的资源。
# 并行时后来者登录会拿到 `result=4 密码错误`（服务端实为 Account already online）——
# 那是资源互斥假红，不是产品缺陷，靠 for 循环反复重跑撞「干净窗口」修不了它。
# 拿不到锁就在这里排队；超时未拿到 → 退出码 2（前置失败）。约定见 e2e_lock.ps1 头部。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5w_object_fx' -TimeoutSec 1800)) { exit 2 }

# 整段包 try/finally：任何 exit/return/异常路径都会释放锁（PowerShell 的 finally 在 exit 下也会执行），
# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。
try {
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $Worktree) { $Worktree = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not $ExeSrc) { $ExeSrc = "$Worktree\Client-Bevy\target\debug\client_bevy.exe" }
$root = 'C:\Users\gxh\AppData\Local\Temp\orig-csharp-ab'
if (-not (Test-Path $root)) { New-Item -ItemType Directory -Path $root | Out-Null }
# 唯一进程名：多 agent 并行时禁止按名字批量杀公共名（见 LESSON_多agent并行时按进程名清进程...）
$exe = "$root\ofx_client.exe"
$err = "$root\l5w_$Tag.err"
$json = "$PSScriptRoot\l5w_object_fx_results.json"

function Rpc([string]$m, [hashtable]$q = @{}) {
    try {
        $c = New-Object Net.Sockets.TcpClient
        $c.ReceiveTimeout = 2000; $c.SendTimeout = 2000
        $c.Connect('127.0.0.1', $ControlPort)
        $s = $c.GetStream(); $s.ReadTimeout = 2000
        $b = [Text.Encoding]::UTF8.GetBytes((@{ jsonrpc = '2.0'; id = 1; method = $m; params = $q } | ConvertTo-Json -Compress -Depth 5) + "`n")
        $s.Write($b, 0, $b.Length); $s.Flush()
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}

function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='ofx_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

Stop-Client
Start-Sleep -Milliseconds 800
if (-not (Test-Path $ExeSrc)) { Write-Host "缺少 exe: $ExeSrc"; exit 2 }
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
if (Test-Path $err) { [System.IO.File]::Delete($err) }
# mock 模式（默认，不带 --real-net）+ 自动登录进场 + 自动战斗表现测试
# （mock 也走客户端同一套 Login→StartGame：不进图就没有本地玩家实体，
#   对象特效会因「对象不存在」被丢弃，夹具会假红）
Start-Process -FilePath $exe `
    -ArgumentList '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', `
        '--control-port', "$ControlPort", '--battle-vfx-test' `
    -WorkingDirectory "$Worktree\Client-Bevy" `
    -RedirectStandardOutput "$root\l5w_$Tag.log" -RedirectStandardError $err | Out-Null

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    $why = (Select-String -Path $err -Pattern 'panic|登录失败' -EA SilentlyContinue | Select-Object -Last 1).Line
    Write-Host ("FAIL(2): 未进场 - " + $why)
    Stop-Client
    exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

$probe0 = Rpc 'spell_fx_probe'
if ($null -eq $probe0 -or $null -eq $probe0.count) {
    Write-Host 'FAIL(2): spell_fx_probe 不可用（未编译进二进制？）'
    Stop-Client
    exit 2
}

$expects = @(
    'object:MagicShieldUp|Flat(Magic)|3890|3',
    'object:Stunned|Monster(StoningStatue)|632|10'
)
$forbid = 'object:Critical'
$observed = @{}
$expect_polls = @{}
$first_seen_s = @{}
$violations = @()
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    $p = Rpc 'spell_fx_probe'
    if ($null -ne $p -and $null -ne $p.active) {
        $keys = @()
        foreach ($a in $p.active) {
            $k = "{0}|{1}|{2}|{3}" -f $a.kind, $a.library, $a.base, $a.frames
            $keys += $k
            if (-not $observed.ContainsKey($k)) {
                $observed[$k] = 1
                Write-Host ("观察到特效: " + $k)
            } else { $observed[$k]++ }
        }
        foreach ($ex in $expects) {
            if ($keys -contains $ex) {
                if (-not $first_seen_s.ContainsKey($ex)) { $first_seen_s[$ex] = $sw.Elapsed.TotalSeconds }
                if (-not $expect_polls.ContainsKey($ex)) { $expect_polls[$ex] = 0 }
                $expect_polls[$ex]++
            }
        }
        foreach ($k in $keys) {
            if ($k -like "$forbid*") {
                $violations += $k
                Write-Host ("负控命中（不该出现）: " + $k)
            }
        }
        # 两条断言项都已观察到、且各自跨过 ≥2.0s（证明循环没消失）→ 提前收工
        $all_ok = $true
        foreach ($ex in $expects) {
            if (-not $expect_polls.ContainsKey($ex) -or $expect_polls[$ex] -lt 2 -or
                ($sw.Elapsed.TotalSeconds - $first_seen_s[$ex]) -lt 2.0) { $all_ok = $false }
        }
        if ($all_ok) { break }
    }
    Start-Sleep -Milliseconds 200
}
Stop-Client

$object_rows = @($observed.Keys | Where-Object { $_ -like 'object:*' })
$fail = @()
$spans = [ordered]@{}
foreach ($ex in $expects) {
    if (-not $observed.ContainsKey($ex)) { $fail += "缺断言项 $ex"; continue }
    $n = $expect_polls[$ex]
    $span = [math]::Round($sw.Elapsed.TotalSeconds - $first_seen_s[$ex], 2)
    $spans[$ex] = @{ polls = $n; span_s = $span }
    if ($n -lt 2 -or $span -lt 2.0) { $fail += "断言项只观察到一次（循环语义未验证）: $ex" }
}
if ($violations.Count -gt 0) { $fail += ("负控命中: " + ($violations -join ',')) }

$result = [ordered]@{
    ok              = ($fail.Count -eq 0)
    expects         = $expects
    expect_stats    = $spans
    forbidden_seen  = $violations
    object_rows     = $object_rows
    observed        = @($observed.Keys)
    missing         = $fail
}
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $json -Encoding UTF8

Write-Host ("对象特效条目 = {0}" -f $object_rows.Count)
foreach ($ex in $expects) {
    $s = $spans[$ex]
    if ($null -ne $s) { Write-Host ("  断言项 {0}：命中 {1} 次 / 跨 {2}s" -f $ex, $s.polls, $s.span_s) }
}
Write-Host ("结论 JSON: " + $json)
if ($object_rows.Count -eq 0) {
    Write-Host 'FAIL(3): 整轮没有任何对象特效实体——前置不成立（未施法/未进图/接线断了）'
    exit 3
}
if ($fail.Count -gt 0) {
    Write-Host ('FAIL(1): ' + ($fail -join '; '))
    exit 1
}
Write-Host '=== 全部 PASS ==='
exit 0

} finally {
    Exit-E2eLock   # 幂等：没持锁时直接返回
}
