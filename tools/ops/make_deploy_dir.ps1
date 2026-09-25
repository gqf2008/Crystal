#Requires -Version 5.1
<#
.SYNOPSIS
  造一个**能起服的最小部署目录**（回滚演练 / 存储降级演练 / 容量标定等夹具的前置）。

.DESCRIPTION
  内容 = `mir2_server.exe`（拷贝）+ `config/`（拷贝）+ `Data/`（拷贝）+ `Daneo1989`（**目录联接**，不拷 1GB）。
  可选 `-Port` 顺带改写副本配置里的 `listen_addr`（演练里"服务端监听口必须与 -Port 一致"这条契约）。

  为什么要有它：2026-09-25 我手工造夹具时漏了 `config/`，服务端于是退化成
  `Config not found → 默认配置（listen 7000 + 内存库）`，演练报告里只体现成 `ready=false`
  ——读起来像"服务端起不来"，实际是夹具残缺。手造夹具容易缺件，这一条命令把它固化下来。

.PARAMETER SourceRoot  含 `config/` 与 `Data/` 的目录（通常是 ServerRust）。
.PARAMETER ExePath     要放进部署目录的服务端二进制。
.PARAMETER OutDir      目标目录；已存在时必须配 `-Force` 才会清空重建。
.PARAMETER MapDataDir  地图数据目录；默认 `<SourceRoot>/Daneo1989`，用目录联接挂进去。
.PARAMETER Port        >0 时把副本配置的 `[network].listen_addr` 改成 `0.0.0.0:<Port>`。
.PARAMETER Force       已存在时清空重建。**拒绝**用于盘根或只有一级的目录（防误删）。

.NOTES
  退出码：0 成功 / 2 前置失败（缺件、目标已存在且未加 -Force、目标路径过高危）。
  用法：pwsh tools/ops/make_deploy_dir.ps1 -SourceRoot ServerRust -ExePath ServerRust/target/debug/mir2_server.exe `
            -OutDir $env:TEMP\deploy_run -Port 7200 -Force
#>
param(
    [Parameter(Mandatory = $true)][string]$SourceRoot,
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][string]$OutDir,
    [string]$MapDataDir = '',
    [int]$Port = 0,
    [switch]$Force
)
$ErrorActionPreference = 'Continue'
function Fail([string]$msg) { Write-Host ("FAIL(前置)：{0}" -f $msg); exit 2 }

$src = (Resolve-Path -LiteralPath $SourceRoot -EA SilentlyContinue).Path
if (-not $src) { Fail "SourceRoot 不存在：$SourceRoot" }
if (-not (Test-Path -LiteralPath $ExePath)) { Fail "ExePath 不存在：$ExePath" }
if (-not $MapDataDir) { $MapDataDir = Join-Path $src 'Daneo1989' }
if (-not (Test-Path -LiteralPath $MapDataDir)) { Fail "地图数据目录不存在：$MapDataDir" }
foreach ($need in @('config/server.toml', 'Data')) {
    if (-not (Test-Path -LiteralPath (Join-Path $src $need))) { Fail "SourceRoot 缺 $need（部署目录必须是能起服的最小集）" }
}
if (-not (Test-Path -LiteralPath (Join-Path $src 'Data/crystal.db'))) {
    Fail "SourceRoot 里没有 Data/crystal.db —— 部署夹具需要一个可用的库副本"
}

# ---- 高危路径守卫：绝不对盘根 / 顶层目录做递归删除 ----
$full = [System.IO.Path]::GetFullPath($OutDir)
$vol = [System.IO.Path]::GetPathRoot($full)
if ($full -eq $vol) { Fail "拒绝：OutDir 是盘根（$full）" }
$segments = @($full.Substring($vol.Length).Trim('\', '/') -split '[\\/]' | Where-Object { $_ })
if ($Force -and $segments.Count -lt 2) { Fail "拒绝：-Force 只允许用于至少两级的目录（$full）" }

if (Test-Path -LiteralPath $full) {
    if (-not $Force) { Fail "OutDir 已存在：$full（要清空重建请加 -Force）" }
    Write-Host ("[1/4] 清空已存在的目标目录：{0}" -f $full)
    Remove-Item -LiteralPath $full -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $full) { Fail "清空失败（可能被占用）：$full" }
}
New-Item -ItemType Directory -Path $full, (Join-Path $full 'config'), (Join-Path $full 'Data') -Force | Out-Null

Write-Host '[2/4] 拷贝二进制 / config / Data'
Copy-Item -LiteralPath $ExePath -Destination (Join-Path $full 'mir2_server.exe') -Force
Copy-Item -LiteralPath (Join-Path $src 'config/server.toml') -Destination (Join-Path $full 'config/server.toml') -Force
Copy-Item -LiteralPath (Join-Path $src 'Data') -Destination $full -Recurse -Force -ErrorAction SilentlyContinue

Write-Host '[3/4] 挂载地图数据（目录联接，不拷贝）'
$link = Join-Path $full 'Daneo1989'
if (-not (Test-Path -LiteralPath $link)) {
    New-Item -ItemType Junction -Path $link -Target (Resolve-Path -LiteralPath $MapDataDir).Path | Out-Null
}

if ($Port -gt 0) {
    Write-Host ("[4/4] 改写副本配置监听口 → 0.0.0.0:{0}" -f $Port)
    $cfg = Join-Path $full 'config/server.toml'
    $text = Get-Content -LiteralPath $cfg -Raw
    if ($text -match 'listen_addr\s*=\s*"[^"]+"') {
        $text = [regex]::Replace($text, 'listen_addr\s*=\s*"[^"]+"', "listen_addr = `"0.0.0.0:$Port`"", 1)
    } else {
        $text = "[network]`nlisten_addr = `"0.0.0.0:$Port`"`n" + $text
    }
    Set-Content -LiteralPath $cfg -Value $text -Encoding utf8
} else {
    Write-Host '[4/4] 未指定 -Port：保留源配置的监听口'
}

foreach ($need in @('mir2_server.exe', 'config/server.toml', 'Data/crystal.db')) {
    if (-not (Test-Path -LiteralPath (Join-Path $full $need))) { Fail "建好的部署目录仍缺 $need（$full）" }
}
Write-Host '--- 部署目录就绪 ---'
Write-Host ("  OutDir      = {0}" -f $full)
Write-Host ("  exe         = {0}" -f (Join-Path $full 'mir2_server.exe'))
Write-Host ("  config      = {0}" -f (Join-Path $full 'config/server.toml'))
Write-Host ("  db          = {0}" -f (Join-Path $full 'Data/crystal.db'))
Write-Host ("  Daneo1989   = {0}（目录联接 → {1}）" -f $link, (Resolve-Path -LiteralPath $MapDataDir).Path)
exit 0
