# 构建戳前置检查（判定仪器，`tools/acceptance/*` 白名单 gitignore ⇒ 需 git add -f）
#
# 为什么有这个文件：owner 反馈与夹具假红/假绿里，「跑的不是当前提交构建的二进制」是反复出现的一类
# ——写邮件窗「错位」、底部对话框滚动/对齐、地图灯光、魔法特效四条反馈实测**全部**是旧构建；
# 夹具侧同型坑见 `LESSON_运行目标分支e2e前需重建二进制避免陈旧target误报`。
# 所以把「这份 exe 出自哪个提交」做成**启动之前**就能验的前置：
#   客户端 `build.rs` 把 `CRYSTAL_BUILD_STAMP_V1 commit=<40hex> short=<hex> dirty=<0|1>`
#   作为**连续 ASCII 字符串**固化进 exe（见 `Client-Bevy/src/control.rs::BUILD_STAMP_RECORD`），
#   这里直接扫字节 —— 不启动进程、不占 e2e 锁、不需要 control 端口。
#
# 用法（夹具里紧跟 `$exe = "$ClientHome\Client-Bevy\target\debug\client_bevy.exe"` 之后）：
#   . "$PSScriptRoot\build_stamp.ps1"
#   Assert-ClientBuildStamp -Exe $exe -Worktree $ClientHome -ScriptName '<夹具名>'
#
# 语义：commit 不一致 ⇒ **前置失败 exit 2**（宁可不产出结论，也不产出对错对象的结论）；
#       exe 里没有戳（旧构建）⇒ 同样 exit 2 并提示重建；dirty=1 只告警不拦
#       （开发机上"改了未提交再构建"是常见且合法的，但它意味着产物不完全等于 HEAD 树）。

function Get-ClientBuildStamp {
    <# 只读扫描 exe，返回 @{ commit; short; dirty } 或 $null（没有戳/读不到） #>
    param([Parameter(Mandatory = $true)][string]$Exe)
    if (-not (Test-Path -LiteralPath $Exe)) { return $null }
    # **分块扫**（实测踩到）：debug 版客户端 exe 有数百 MB，一次性 ReadAllBytes + GetString
    # 会直接 "Insufficient memory to continue the execution of the program."（本机实测）。
    # 所以按 4MB 块流式扫，块间保留一小段尾巴防止记录跨块被切断。
    $needle = 'CRYSTAL_BUILD_STAMP_V1 commit='
    $enc = [Text.Encoding]::GetEncoding(28591)   # Latin1：单字节一一对应，任意字节都不会解码失败
    $found = $null
    $fs = $null
    try {
        $fs = [IO.File]::OpenRead($Exe)
        $buf = New-Object byte[] (4MB)
        $tail = ''
        while (($n = $fs.Read($buf, 0, $buf.Length)) -gt 0) {
            $s = $tail + $enc.GetString($buf, 0, $n)
            $i = $s.IndexOf($needle, [StringComparison]::Ordinal)
            if ($i -ge 0) {
                $found = $s.Substring($i, [Math]::Min(160, $s.Length - $i))
                break
            }
            $keep = [Math]::Min($s.Length, $needle.Length + 64)
            $tail = $s.Substring($s.Length - $keep)
        }
    } catch {
        return $null
    } finally {
        if ($fs) { $fs.Dispose() }
    }
    if (-not $found) { return $null }
    $m = [regex]::Match($found, 'CRYSTAL_BUILD_STAMP_V1 commit=([0-9a-f]{7,40}) short=([0-9a-f]{7,40}) dirty=([01])')
    if (-not $m.Success) { return $null }
    return [pscustomobject]@{
        commit = $m.Groups[1].Value
        short  = $m.Groups[2].Value
        dirty  = $m.Groups[3].Value
    }
}

function Assert-ClientBuildStamp {
    <#
      .SYNOPSIS
        断言被测 exe 出自 `-Worktree` 当前 HEAD；不成立即 exit 2（前置失败）。
      .PARAMETER Exe
        被测客户端 exe（通常是 `$ClientHome\Client-Bevy\target\debug\client_bevy.exe`）。
      .PARAMETER Worktree
        该 exe 的构建根（= 夹具的 `$ClientHome`）。**不是**当前脚本所在 worktree。
      .PARAMETER ScriptName
        夹具名，仅用于输出。
      .PARAMETER AllowDirty
        显式允许 dirty 产物（默认也只是告警，不拦）。
    #>
    param(
        [Parameter(Mandatory = $true)][string]$Exe,
        [Parameter(Mandatory = $true)][string]$Worktree,
        [string]$ScriptName = '(unknown)',
        [switch]$AllowDirty
    )
    $stamp = Get-ClientBuildStamp -Exe $Exe
    if ($null -eq $stamp) {
        Write-Host ("FAIL(2)[{0}]: 被测 exe 里没有构建戳 —— 多半是**旧产物**，请重建客户端：{1}" -f $ScriptName, $Exe) -ForegroundColor Red
        Write-Host ("          （重建：cd {0}\Client-Bevy; cargo build --bin client_bevy）" -f $Worktree) -ForegroundColor Red
        exit 2
    }
    $head = ''
    try {
        $head = (& git -C $Worktree rev-parse HEAD 2>$null | Select-Object -First 1)
        if ($head) { $head = $head.Trim() }
    } catch { $head = '' }
    if (-not $head) {
        Write-Host ("  [WARN][{0}] 取不到 {1} 的 HEAD（不是 git 仓库？）——跳过构建戳比对（stamp={2}）" -f `
            $ScriptName, $Worktree, $stamp.short) -ForegroundColor Yellow
        return
    }
    if ($stamp.commit -ne $head -and -not $head.StartsWith($stamp.commit)) {
        Write-Host ("FAIL(2)[{0}]: 被测 exe 出自 {1}，而构建根 HEAD={2} —— **陈旧二进制**，先重建再跑" -f `
            $ScriptName, $stamp.short, $head.Substring(0, [Math]::Min(9, $head.Length))) -ForegroundColor Red
        Write-Host ("          （重建：cd {0}\Client-Bevy; cargo build --bin client_bevy）" -f $Worktree) -ForegroundColor Red
        exit 2
    }
    $dirtyTag = if ($stamp.dirty -eq '1') { 'dirty=1（构建时工作区有未提交改动：产物≠HEAD 树，仅告警）' } else { 'dirty=0' }
    if ($stamp.dirty -eq '1' -and -not $AllowDirty) {
        Write-Host ("  [WARN][{0}] 构建戳 {1} {2}" -f $ScriptName, $stamp.short, $dirtyTag) -ForegroundColor Yellow
    } else {
        Write-Host ("  [OK][{0}] 构建戳与构建根 HEAD 一致：{1} {2}" -f $ScriptName, $stamp.short, $dirtyTag) -ForegroundColor DarkGray
    }
}
