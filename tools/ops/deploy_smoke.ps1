# deploy_smoke.ps1 — 部署冒烟：全新目录从零起服 + 协议级登录冒烟 + 产物哈希
#
# 判据（任一不过即 exit 5）：
#   ① 产物齐全且记录 sha256（服务端 exe、config/server.toml）
#   ② 全新目录里起服成功：日志出现 "Gate listening on <addr>"，且端口在 -ReadyTimeoutSec 内可连
#   ③ 登录冒烟成功（bot.py 走完 Connected→ClientVersion→Login，拿回执）
#   ④ 报告写 JSON
# 部署模型：exe + config + Data/（DB 与 spawn，拷贝）复制到目标目录；
#          Daneo1989（地图数据 ~1GB）用**目录联接**挂载，不拷贝（映射"地图数据只读挂载"）。
param(
    [Parameter(Mandatory = $true)][string]$ReleaseDir,   # 例如 ServerRust/target/release
    [Parameter(Mandatory = $true)][string]$SourceRoot,    # 例如 ServerRust（含 config/ Data/ Daneo1989/）
    [string]$DeployDir = '',
    [int]$Port = 7100,
    [string]$Account = 'test',
    [string]$Password = '123456',
    [int]$ReadyTimeoutSec = 90,
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $DeployDir) { $DeployDir = Join-Path (Join-Path $ops 'out') 'deploy' }
$exeSrc = Join-Path $ReleaseDir 'mir2_server.exe'
if (-not (Test-Path $exeSrc)) { Write-Host "FAIL: 找不到 $exeSrc"; exit 2 }

if (Test-Path $DeployDir) { Remove-Item -Recurse -Force $DeployDir -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path $DeployDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DeployDir 'config') | Out-Null

# ① 产物：exe + config
Copy-Item $exeSrc (Join-Path $DeployDir 'mir2_server.exe') -Force
$cfgSrc = Join-Path $SourceRoot 'config/server.toml'
$cfgDst = Join-Path $DeployDir 'config/server.toml'
Copy-Item $cfgSrc $cfgDst -Force
# 部署即改端口：不和生产/开发实例抢 7000
(Get-Content $cfgDst -Raw) -replace 'listen_addr\s*=\s*"[^"]+"', "listen_addr = `"0.0.0.0:$Port`"" |
    Set-Content -Encoding utf8 $cfgDst

# ② 数据：Data/ 拷贝（DB+spawn），Daneo1989 用联接
Copy-Item (Join-Path $SourceRoot 'Data') $DeployDir -Recurse -Force -ErrorAction SilentlyContinue
$link = Join-Path $DeployDir 'Daneo1989'
if (-not (Test-Path $link)) {
    New-Item -ItemType Junction -Path $link -Target (Resolve-Path (Join-Path $SourceRoot 'Daneo1989')).Path | Out-Null
}

$log = Join-Path $DeployDir 'server.log'
$errLog = Join-Path $DeployDir 'server.err.log'
$t0 = Get-Date
$proc = Start-Process -FilePath (Join-Path $DeployDir 'mir2_server.exe') -WorkingDirectory $DeployDir `
    -RedirectStandardOutput $log -RedirectStandardError $errLog -PassThru

# ③ 等 "Gate listening"（起服就绪判据）
$ready = $false
for ($i = 0; $i -lt $ReadyTimeoutSec; $i++) {
    Start-Sleep 1
    if ((Get-Content $log -ErrorAction SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
}
$readySec = [Math]::Round(((Get-Date) - $t0).TotalSeconds, 1)

# ④ 登录冒烟
$smoke = $null
if ($ready) {
    $out = & python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --login-only `
        --accounts $Account --sessions 1 --password $Password 2>&1 | Select-Object -Last 1
    try { $smoke = $out | ConvertFrom-Json } catch { $smoke = $null }
}
if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
Start-Sleep 2

$hashes = [ordered]@{
    server_exe = (Get-FileHash (Join-Path $DeployDir 'mir2_server.exe') -Algorithm SHA256).Hash
    config     = (Get-FileHash $cfgDst -Algorithm SHA256).Hash
    db         = if (Test-Path (Join-Path $DeployDir 'Data/crystal.db')) { (Get-FileHash (Join-Path $DeployDir 'Data/crystal.db') -Algorithm SHA256).Hash } else { $null }
}
$ok = $ready -and ($smoke -and $smoke.summary.failed -eq 0)
$report = [ordered]@{
    ok = [bool]$ok
    deploy_dir = (Resolve-Path $DeployDir).Path
    listen_port = $Port
    gateway_ready = $ready
    gateway_ready_sec = $readySec
    login_smoke = if ($smoke) { $smoke.summary } else { $null }
    hashes = $hashes
    criteria = '① 产物哈希 ② 全新目录起服+端口就绪 ③ 协议级登录冒烟'
}
$json = $report | ConvertTo-Json -Depth 6
if ($OutFile) { $json | Set-Content -Encoding utf8 $OutFile }
Write-Host $json
if (-not $ok) { exit 5 } else { exit 0 }
