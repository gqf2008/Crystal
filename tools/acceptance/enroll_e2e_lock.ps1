#Requires -Version 5.1
<#
.SYNOPSIS
  批量把「实机入口」（会起客户端并登录 e2e 账号的脚本）接入跨进程互斥锁 e2e_lock.ps1。幂等。

.DESCRIPTION
  为什么需要它：客户端 + e2e 账号 + 本地服务端是**一次只能跑一组**的资源。没接入锁的脚本与
  接入锁的脚本并行跑，后者登录会拿到 `result=4 密码错误`（服务端日志实为 `Account already online`）
  ——那是资源互斥假红，不是产品缺陷；靠 `for(i=1..8){ 跑夹具; sleep 60 }` 撞"干净窗口"
  只会把交付时间耗在等待上。

  本脚本对每个尚未接入的实机入口做两处**机械插入**（只插不删，重复跑不会重复插）：
    ① 在「参数块之后 / 第一句可执行语句之前」插入 dot-source + `Enter-E2eLock` + `try {`；
    ② 在文件末尾补 `} finally { Exit-E2eLock }`。

  为什么用 try/finally 包整段而不是在每个 exit 前插一行：夹具的退出路径不止一条，
  既有文件末尾的 `exit 0`，也有中段的 `if (...) { exit 5 }`、以及 `throw`——
  逐个插会漏，而**漏掉的锁要等 StaleSec=1800s 才回收**（更糟的是在长驻 shell 里
  `& 夹具.ps1` 调用的场景：持有者 PID 就是 shell 自己，进程不死就一直占着）。
  PowerShell 的 `finally` 在 `exit` / `return` / 异常下都会执行（实测 `-File` 与会话内
  `& script.ps1` 两种模式都跑到了），所以整段包一层是覆盖所有路径的唯一简单做法。

  覆盖判据与门禁在 `e2e_lock_selftest.ps1`：它扫描 `Get-E2eClientScripts` 认出来的每个脚本，
  缺锁即红——所以新增夹具忘了接入会被门禁抓住，不靠人记。

.PARAMETER RepoRoot
  检出根，默认 = 本脚本所在仓库根。

.PARAMETER Apply
  真的写文件。默认只打印将要做的改动（dry-run）。

.EXAMPLE
  pwsh tools/acceptance/enroll_e2e_lock.ps1
  pwsh tools/acceptance/enroll_e2e_lock.ps1 -Apply
#>
[CmdletBinding()]
param(
    [string]$RepoRoot = '',
    [switch]$Apply
)
$ErrorActionPreference = 'Stop'
if (-not $RepoRoot) { $RepoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path }
. "$PSScriptRoot\e2e_lock.ps1"

function Get-InsertAnchor {
    <# 插入点：param 块收尾之后；没有 param 块就是第一句可执行语句之前。#>
    param([string[]]$Lines)
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        if ($Lines[$i] -match '^\s*param\s*\(') {
            for ($j = $i; $j -lt $Lines.Count; $j++) {
                if ($Lines[$j] -match '^\s*\)\s*$') { return ($j + 1) }
            }
            break
        }
    }
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        if ($Lines[$i] -match '^\s*[^#\s]') { return $i }
    }
    return 0
}

function Get-LockRelPath {
    param([string]$ScriptDir, [string]$RepoRoot)
    $accDir = (Join-Path $RepoRoot 'tools\acceptance')
    if ((Resolve-Path -LiteralPath $ScriptDir).Path -eq (Resolve-Path -LiteralPath $accDir).Path) {
        return '$PSScriptRoot\e2e_lock.ps1'
    }
    return '$PSScriptRoot\..\tools\acceptance\e2e_lock.ps1'
}

function Get-LockBlock {
    param([string]$ScriptName, [string]$LockRel)
    @(
        '',
        '# --- 实机资源串行：客户端 + e2e 账号 + 本地服务端一次只能跑一组（跨进程锁）---',
        '# 不拿锁就会撞上「别的 agent 已登录同一账号」→ 日志里的 result=4 密码错误',
        '# （服务端实为 Account already online），那是资源互斥假红、不是产品缺陷，重试再多也修不了它；',
        '# 详见 tools\acceptance\e2e_lock.ps1 与 e2e_lock_selftest.ps1（门禁会查漏接入）。',
        ". `"$LockRel`"",
        "if (-not (Enter-E2eLock -ScriptName '$ScriptName' -TimeoutSec 1800)) { Write-Host 'FAIL(2): 等 e2e 锁超时'; exit 2 }",
        '',
        '# 整段包 try/finally：任何 exit/return/异常路径都会释放锁',
        '# （PowerShell 的 finally 在 exit 下也会执行——实测 -File 与会话内 & script.ps1 两种调用都成立），',
        '# 所以早退分支（例如中段的 if (...) { exit 5 }）不会把锁漏给别人：漏了要等 StaleSec=1800s 才回收。',
        'try {'
    )
}

$targets = @(Get-E2eClientScripts -RepoRoot $RepoRoot)
Write-Host ("实机入口扫描：{0} 个脚本会起客户端（判据：--e2e-user / client_bevy.exe / --real-net / --auto-enter，扫描面＝整仓 *.ps1/.bat/.cmd）" -f $targets.Count)

$changed = 0
$skipped = 0
$manual = 0
foreach ($t in $targets) {
    # 非 PowerShell 启动器（.bat/.cmd）：本接入器只会往 .ps1 里插 dot-source + try/finally，
    # 对文本启动器插进去就是把文件改坏（它没有那种写法），所以只如实报给人工：
    # 要么让它点名调用一个已接入的 .ps1 夹具、要么让它在正文里点名 e2e_lock。
    if ($t.Kind -ne 'ps1') {
        Write-Host ("  MANUAL {0,-32} 非 PowerShell 启动器（{1}）：需人工接入（点名已接入的 .ps1 或 e2e_lock）" -f $t.Name, $t.Kind)
        $manual++
        continue
    }
    $check = Test-E2eLockEnrollment -Path $t.Path
    if ($check.ok) {
        Write-Host ("  SKIP   {0,-32} 已接入（Enter×{1} Exit×{2}）" -f $t.Name, $t.EnterCount, $t.ExitCount)
        $skipped++
        continue
    }

    $raw = [System.IO.File]::ReadAllText($t.Path)
    $eol = if ($raw -match "`r`n") { "`r`n" } else { "`n" }
    $hasBom = $false
    $bytes = [System.IO.File]::ReadAllBytes($t.Path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) { $hasBom = $true }
    $endsWithNewline = $raw.EndsWith("`n")
    $lines = @($raw -split "`r`n|`n")
    if ($endsWithNewline -and $lines.Count -gt 0 -and $lines[-1] -eq '') { $lines = $lines[0..($lines.Count - 2)] }

    # ① 顶端插入 dot-source + Enter-E2eLock + try {
    $anchor = Get-InsertAnchor -Lines $lines
    $block = Get-LockBlock -ScriptName ([System.IO.Path]::GetFileNameWithoutExtension($t.Name)) `
        -LockRel (Get-LockRelPath -ScriptDir $t.Dir -RepoRoot $RepoRoot)
    $final = @()
    if ($anchor -gt 0) { $final += $lines[0..($anchor - 1)] }
    $final += $block
    if ($anchor -lt $lines.Count) { $final += $lines[$anchor..($lines.Count - 1)] }
    # ② 末尾补 } finally { Exit-E2eLock }
    $final += @('', '} finally {', "    Exit-E2eLock   # 幂等：没持锁时直接返回", '}')

    Write-Host ("  {0} {1,-32} +Enter@L{2}  +try/finally 包整段" -f $(if ($Apply) { 'APPLY' } else { 'DRY  ' }), $t.Name, ($anchor + 1))
    if ($Apply) {
        $text = ($final -join $eol) + $(if ($endsWithNewline) { $eol } else { '' })
        [System.IO.File]::WriteAllText($t.Path, $text, (New-Object System.Text.UTF8Encoding($hasBom)))
    }
    $changed++
}

Write-Host ''
Write-Host ("合计：需改 {0} 个、需人工接入 {1} 个、已接入 {2} 个{3}" -f $changed, $manual, $skipped,
    $(if ($Apply) { '（已写入）' } else { '（dry-run：加 -Apply 才会写）' }))
if ($changed -gt 0 -and $Apply) {
    Write-Host '改完请跑门禁：pwsh tools/acceptance/e2e_lock_selftest.ps1'
}
exit 0
