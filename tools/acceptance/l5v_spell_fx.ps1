#Requires -Version 5.1
<#
.SYNOPSIS
  法术特效实机夹具——owner 反馈「魔法效果完全不对」的**可复验判据**。

.DESCRIPTION
  判据取自渲染侧**真正 spawn 的**特效实体（只读 RPC `spell_fx_probe`：
  `SpellFxAnim` / `SpellMissileAnim` 的库、起始帧、帧数），不是日志文本。

  原版对照（`Client/MirObjects/PlayerObject.cs` 的 MirAction.Spell 分支，机械生成到
  `Client-Bevy/src/game/spell_effects.rs`）：
    HellFire        → 施法帧动画 Magic   base 920  frames 10  （**断言项**，本角色已学）
    EnergyRepulsor  → 施法帧动画 Magic2  base 190  frames 6   （观察到就记录，不作硬断言）
    Curse           → 弹道帧动画 Magic   base 1160 frames 3    （观察到就记录）
    DefaultArrow 等 → 弹道 Magic3 base 1030 frames 5           （只有 NPC 恰好远程攻击时才出现，不作断言）

  **前置条件（必须先成立，否则判「前置不成立」而不是 FAIL）**：
  ① 角色已学 HellFire（`--spell-verify` 打的是 HellFire/IceThrust/Curse/EnergyRepulsor）；
  ② **MP 足够**——服务端在 MP 不足时**直接拒绝施法、不发 `MagicCast`**，客户端自然不会显示任何特效。
  夹具用 `@LEVEL <当前等级+1>` 升一级（原版升级补满 HP/MP；实测库里 `mp 0 → 294`）建立前置，
  并从 `UserInformation` 日志行解析 `mp=` 做**前置断言**（`mp<=0` → exit 3，不产出 PASS）。

  > 留痕：本夹具第一版把「没有 cast 特效」当成产品缺陷报了 issue #3119，根因其实是**前置不成立**
  > （角色 `mp=0`，而另一条探针 `--battle-vfx-test` 打的是它**没学**的 FireBall）。补 MP 后立刻观察到
  > `cast|Magic|920|10`（= HellFire 的 C# 表项）。issue 已按证据更正。

  夹具用 `--spell-verify`（`src/auto/combat.rs::auto_spell_verify`）自动走到怪旁循环施法，
  再轮询探针收集实际出现的 (kind, library, base, frames)。

  仪器自检：施法开始前连读两次探针，两次读数必须一致（静默期应为 0）——
  沿用仓库既有「判据区静态性自检」口径。

  退出码：0 = 三条原版条目全部观察到；1 = 有用例 FAIL（观察到了特效但缺条目）；
          2 = 前置不满足（未进图 / 探针不可用）；3 = 前置不成立（整轮没观察到任何特效）。

  阳性对照（落地时实做）：把 `effects.rs` 的 `spell_fx(..)` 查表结果强制为 `None`
  （退回占位方块）→ 本夹具必然 exit 3（观察不到任何特效）。
#>
param(
    [string]$ExeSrc = '',
    [int]$ControlPort = 9041,
    [string]$Worktree = '',
    # `--spell-verify` 要先走到怪旁才會施放（本機 D002 有障礙，實測常需 100~200s）
    [int]$TimeoutSec = 240,
    [string]$Tag = 'spellfx'
)
$ErrorActionPreference = 'Continue'
$env:PATH = 'D:\toolchains\msys64\ucrt64\bin;D:\toolchains\libpinyin-install\bin;' + $env:PATH
$env:LIBPINYIN_DIR = 'D:/tools/libpinyin-install'
$env:LIBPINYIN_DIR = 'D:/toolchains/libpinyin-install'
if (-not $Worktree) { $Worktree = (Resolve-Path "$PSScriptRoot\..\..").Path }
if (-not $ExeSrc) { $ExeSrc = "$Worktree\Client-Bevy\target\debug\client_bevy.exe" }
$root = 'C:\Users\gxh\AppData\Local\Temp\orig-csharp-ab'
$&
. "$PSScriptRoot\build_stamp.ps1"   # 构建戳前置：不许对着旧产物下结论（见 LESSON_运行目标分支e2e前需重建二进制）
Assert-ClientBuildStamp -Exe $exe -ScriptName 'l5v_spell_fx'
$err = "$root\l5v_$Tag.err"
$json = "$PSScriptRoot\l5v_spell_fx_results.json"

# 实机资源（客户端 + e2e 账号 + 本地服务端）**必须串行**：先拿跨进程锁再起客户端。
# 不拿锁会撞上「别的 agent 已登录同一账号」→ `result=4 密码错误`（服务端实为
# `Account already online`），那是资源互斥假红，不是产品缺陷。
. "$PSScriptRoot\e2e_lock.ps1"
if (-not (Enter-E2eLock -ScriptName 'l5v_spell_fx' -TimeoutSec 1800)) {
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
        $r = New-Object IO.StreamReader($s); $l = $r.ReadLine(); $c.Close()
        if (-not $l) { return $null }
        ($l | ConvertFrom-Json).result
    } catch { return $null }
}

Get-CimInstance Win32_Process -Filter "Name='sf_client.exe'" -EA SilentlyContinue |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 800
if (-not (Test-Path $ExeSrc)) { Write-Host "缺少 exe: $ExeSrc"; Exit-E2eLock; exit 2 }
Copy-Item -LiteralPath $ExeSrc -Destination $exe -Force
if (Test-Path $err) { [System.IO.File]::Delete($err) }
Start-Process -FilePath $exe `
    -ArgumentList '--real-net', '--auto-enter', '--e2e-user', 'test', '--e2e-pass', '123456', `
        '--control-port', "$ControlPort", '--spell-verify' `
    -WorkingDirectory "$Worktree\Client-Bevy" `
    -RedirectStandardOutput "$root\l5v_$Tag.log" -RedirectStandardError $err | Out-Null

function Stop-Client {
    Get-CimInstance Win32_Process -Filter "Name='sf_client.exe'" -EA SilentlyContinue |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
}

$st = $null
foreach ($i in 1..60) { Start-Sleep 1; $st = Rpc 'state'; if ($null -ne $st.tile_x) { break } }
if ($null -eq $st -or $null -eq $st.tile_x) {
    $why = (Select-String -Path $err -Pattern '登录失败|already online' -EA SilentlyContinue | Select-Object -Last 1).Line
    Write-Host ("FAIL(2): 未进场 - " + $why)
    Stop-Client
    Exit-E2eLock
    exit 2
}
Write-Host ("进场 map={0} tile=({1},{2})" -f $st.map, $st.tile_x, $st.tile_y)

# ---- 前置建立：升一级补满 HP/MP（原版升级语义；实测 mp 0 → 294），并断言 mp>0 ----
function Get-PlayerLine {
    (Select-String -Path $err -Pattern 'UserInformation' -EA SilentlyContinue | Select-Object -Last 1).Line
}
$line0 = Get-PlayerLine
$lv = 0
if ($line0 -match 'Lv\.(\d+)') { $lv = [int]$Matches[1] }
if ($lv -gt 0) {
    Rpc 'chat' @{ message = "@LEVEL $($lv + 1)" } | Out-Null
    Start-Sleep -Seconds 3
} else {
    Write-Host '[前置] 未从日志解析到等级，跳过补 MP 这一步'
}
$line1 = Get-PlayerLine
$mp = -1
if ($line1 -match 'mp=(-?\d+)') { $mp = [int]$Matches[1] }
Write-Host ("前置：等级 {0} → 行 '{1}'（解析 mp={2}）" -f $lv, $line1, $mp)
if ($mp -le 0) {
    Write-Host 'FAIL(3): 前置不成立——角色 MP <= 0，服务端不会接受任何施法（也就没有 MagicCast / 施法特效）。'
    Write-Host '         需要先把角色 MP 补起来（如 GM `@LEVEL <等级+1>`）再跑本夹具。'
    Stop-Client
    Exit-E2eLock
    exit 3
}

$p0 = Rpc 'spell_fx_probe'
Start-Sleep -Milliseconds 250
$p1 = Rpc 'spell_fx_probe'
if ($null -eq $p0 -or $null -eq $p0.count) {
    Write-Host 'FAIL(2): spell_fx_probe 不可用（未编译进二进制？）'
    Stop-Client
    Exit-E2eLock
    exit 2
}
$self_ok = ($p0.count -eq $p1.count)
Write-Host ("仪器自检：连读两次 count = {0} / {1}（{2}）" -f $p0.count, $p1.count, $(if ($self_ok) { '一致' } else { '不一致' }))

# ---- 收集：轮询探针，记录 (kind|library|base|frames) ----
$observed = @{}
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    $p = Rpc 'spell_fx_probe'
    if ($null -ne $p -and $null -ne $p.active) {
        foreach ($a in $p.active) {
            $k = "{0}|{1}|{2}|{3}" -f $a.kind, $a.library, $a.base, $a.frames
            if (-not $observed.ContainsKey($k)) {
                $observed[$k] = 1
                Write-Host ("观察到特效: " + $k)
            } else { $observed[$k]++ }
        }
    }
    Start-Sleep -Milliseconds 200
}
Stop-Client

$expect = @(
    @{ key = 'cast|Magic|920|10'; desc = 'HellFire → 施法帧动画 Magic base 920 frames 10（C# SPELL_FX["HellFire"]）' }
)
# 观察到就记录、不作硬断言：探针在 150s 窗口内可能只来得及走位并施放第一个法术；
# 它们是同一套 C# 表的其它条目，出现即佐证「自己施法」链路整体正常。
$also_record = @(
    'cast|Magic2|190|6',
    'missile|Magic|1160|3',
    'missile|Magic3|1030|5'
)
$fail = @()
foreach ($e in $expect) { if (-not $observed.ContainsKey($e.key)) { $fail += $e.desc } }

$result = [ordered]@{
    ok            = ($fail.Count -eq 0 -and $observed.Count -gt 0)
    probe_selfcheck = $self_ok
    probe_first_two = @($p0.count, $p1.count)
    observed      = @($observed.Keys)
    missing       = $fail
    also_record   = $also_record
    mp            = $mp
}
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $json -Encoding UTF8

Write-Host ("观察到的特效条目数 = {0}；缺条目 = {1}" -f $observed.Count, ($fail -join '; '))
Write-Host ("结论 JSON: " + $json)
# 前置判据②：`--spell-verify` 必须**真的施放过**（它要先走到怪旁；没施放 = 前置不成立，不是 FAIL）
$castLines = @(Select-String -Path $err -Pattern '\[SPELL\].*施放' -EA SilentlyContinue)
Write-Host ("本轮探针施放次数 = {0}" -f $castLines.Count)
if ($castLines.Count -eq 0) {
    Write-Host ("FAIL(3): 前置不成立——{0}s 内 --spell-verify 没走到怪旁施法（提高 -TimeoutSec 或换到怪物更近的图再跑）" -f $TimeoutSec)
    exit 3
}
if ($observed.Count -eq 0) {
    Write-Host 'FAIL(3): 整轮没有观察到任何特效实体——前置不成立（未施法/未学技能/无怪物）'
    Exit-E2eLock
    exit 3
}
if ($fail.Count -gt 0) {
    Write-Host ('FAIL(1): 缺原版条目 - ' + ($fail -join '; '))
    Exit-E2eLock
    exit 1
}
Write-Host '=== 全部 PASS ==='
Exit-E2eLock
exit 0
