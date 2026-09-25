# fresh_mirdb_deploy.ps1 — 核验「全新部署第一步」：从原版 C# `Server.MirDB` 导入 → 起服 → 新玩家全路径冒烟
#
# 为什么单列一个夹具：`docs/DELIVERY.md` §3 把「`migrate_mirdb` 导入 MirDB」写成新部署的第一步，
# 但此前**没有任何测试或已记录的执行证据**。2026-09-25 实跑发现它对**当前发布的数据**
# （`Server.MirDB` version 112）直接失败：物品段读到第 607/1628 个就 EOF、后面各段全空
# （工具只实现了 v≤84 的老布局）。修完后本夹具把这条路径固定成一条命令可复跑。
#
# 判据（任一不过即非零退出）：
#   J1 迁移进程 exit 0，且日志里每段「N 条 → migrated N 条」零失败
#   J2 生成库的关键定义表行数 ≥ 阈值（items ≥ 1000、monsters ≥ 300、npcs ≥ 200、maps ≥ 300）
#   J3 起服就绪（日志出现 `Gate listening`）+ 首启导入落地（drops / NPC 脚本 / goods / recipes 各一行 Imported）
#   J4 新玩家全路径冒烟：`bot.py --self-provision` 建号 → 登录 → 建角 → 进图（`ok=1` 且收到世界帧）
#   J5 冒烟后库里真的有 1 个账号 + 1 个角色（证明不是"看起来成功"）
#
# 用法：
#   pwsh tools/ops/fresh_mirdb_deploy.ps1 -MirDb <Server.MirDB> -ServerDir ServerRust/target/release `
#        -DataRoot E:\path\to\ServerRust -WorkDir $env:TEMP\fresh_mirdb -Port 7500
param(
    [Parameter(Mandatory = $true)][string]$MirDb,
    [Parameter(Mandatory = $true)][string]$ServerDir,
    [Parameter(Mandatory = $true)][string]$DataRoot,     # 含 Daneo1989/ 与 config/server.toml 的目录
    [string]$WorkDir = '',
    [int]$Port = 7500,
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $WorkDir) { $WorkDir = Join-Path $ops ('out/fresh_mirdb_' + (Get-Date -Format 'HHmmss')) }
New-Item -ItemType Directory -Force -Path $WorkDir, (Join-Path $WorkDir 'config'), (Join-Path $WorkDir 'Data') | Out-Null

$migrate = Join-Path $ServerDir 'migrate_mirdb.exe'
$server = Join-Path $ServerDir 'mir2_server.exe'
foreach ($p in @($migrate, $server, $MirDb, (Join-Path $DataRoot 'config/server.toml'))) {
    if (-not (Test-Path $p)) { Write-Host "FATAL: 缺 $p（先 cargo build --release）"; exit 2 }
}
$db = Join-Path $WorkDir 'Data/crystal.db'
if (Test-Path $db) { Remove-Item $db -Force }

$report = [ordered]@{ mir_db = $MirDb; work_dir = $WorkDir; port = $Port; steps = [ordered]@{} }

# ---------------- J1：迁移 ----------------
$migLog = Join-Path $WorkDir 'migrate.log'
$env:RUST_LOG = 'info'
$mp = Start-Process -FilePath $migrate -ArgumentList @($MirDb, $db) `
    -RedirectStandardOutput $migLog -RedirectStandardError (Join-Path $WorkDir 'migrate.err.log') `
    -PassThru -WindowStyle Hidden -Wait
$dbSize = if (Test-Path $db) { (Get-Item $db).Length } else { 0 }
$migErr = @(Select-String -Path $migLog -Pattern 'parse error|failed:' -EA SilentlyContinue)
$report.steps.migrate = [ordered]@{
    exit = $mp.ExitCode; db_bytes = $dbSize
    parse_errors = $migErr.Count
    lines = @(Select-String -Path $migLog -Pattern 'migrated:' -EA SilentlyContinue | ForEach-Object { $_.Line -replace '^.*migrate_mirdb: ', '' })
}
if ($mp.ExitCode -ne 0 -or $migErr.Count -gt 0) {
    Write-Host "FAIL J1: 迁移未干净退出（exit=$($mp.ExitCode), parse_errors=$($migErr.Count)）"
    $report | ConvertTo-Json -Depth 8; exit 3
}

# ---------------- J2：定义表行数 ----------------
$counts = (& python (Join-Path $ops 'mirdb_counts.py') $db | Out-String).Trim() | ConvertFrom-Json
$report.steps.counts = $counts
$need = @{ item_infos = 1000; monster_infos = 300; npc_infos = 200; map_infos = 300 }
foreach ($k in $need.Keys) {
    if ([int]$counts.$k -lt $need[$k]) { Write-Host "FAIL J2: $k=$($counts.$k) < $($need[$k])"; $report | ConvertTo-Json -Depth 8; exit 4 }
}

# ---------------- J3：起服 + 首启导入 ----------------
Copy-Item $server (Join-Path $WorkDir 'mir2_server.exe') -Force
$cfg = Get-Content (Join-Path $DataRoot 'config/server.toml') -Raw
$cfg = $cfg -replace 'listen_addr\s*=\s*"[^"]+"', "listen_addr = `"0.0.0.0:$Port`""
$cfg = $cfg -replace 'map_data_dir\s*=\s*"[^"]+"', "map_data_dir = `"$((Join-Path $DataRoot 'Daneo1989') -replace '\\','/')`""
Set-Content -Path (Join-Path $WorkDir 'config/server.toml') -Value $cfg -NoNewline -Encoding utf8

$srvLog = Join-Path $WorkDir 'server.log'
$proc = Start-Process -FilePath (Join-Path $WorkDir 'mir2_server.exe') -WorkingDirectory $WorkDir `
    -RedirectStandardOutput $srvLog -RedirectStandardError (Join-Path $WorkDir 'server.err.log') -PassThru -WindowStyle Hidden
$ready = $false
for ($i = 0; $i -lt 90; $i++) {
    Start-Sleep 1
    if ((Get-Content $srvLog -EA SilentlyContinue) -match 'Gate listening') { $ready = $true; break }
}
Start-Sleep 3   # 首启导入在这之后落库
$imports = @(Select-String -Path $srvLog -Pattern 'Imported .* (drop|NPC script|NPC goods|recipes)' -EA SilentlyContinue | ForEach-Object { $_.Line -replace '^.*crystal_server::db: ', '' })
$errors = @(Select-String -Path $srvLog -Pattern 'ERROR' -EA SilentlyContinue)
$report.steps.boot = [ordered]@{ ready = $ready; imports = $imports; error_lines = $errors.Count }
if (-not $ready -or $imports.Count -lt 4) {
    Write-Host "FAIL J3: ready=$ready imports=$($imports.Count)（期望 4 条：drops/NPC脚本/goods/recipes）"
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
    $report | ConvertTo-Json -Depth 8; exit 5
}

# ---------------- J4：新玩家全路径冒烟 ----------------
$acc = 'fresh' + (Get-Date -Format 'HHmmss')
$botOut = (& python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --sessions 1 `
        --self-provision --account-prefix $acc --password 123456 --hold 6 | Out-String)
$bot = $null
try { $bot = ($botOut.Trim() | ConvertFrom-Json) } catch {}
$s0 = if ($bot) { $bot.sessions[0] } else { $null }
$smokeOk = ($null -ne $s0) -and ([int]$s0.frames -gt 0) -and ([int]$s0.new_account_result -eq 8)
$report.steps.smoke = [ordered]@{
    account = $acc; frames = if ($s0) { $s0.frames } else { -1 }
    new_account_result = if ($s0) { $s0.new_account_result } else { -1 }
    new_character_frame = if ($s0) { $s0.new_character_frame } else { -1 }
    new_character_name_len = if ($s0) { $s0.new_character_name_len } else { -1 }
    ok = $smokeOk
}

# ---------------- J5：库里真有账号+角色 ----------------
$rows = (& python (Join-Path $ops 'mirdb_counts.py') $db --runtime | Out-String).Trim() | ConvertFrom-Json
$report.steps.runtime_rows = $rows
$rowsOk = ([int]$rows.accounts -ge 1) -and ([int]$rows.characters -ge 1)

if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
if (-not ($smokeOk -and $rowsOk)) {
    Write-Host "FAIL J4/J5: smokeOk=$smokeOk rowsOk=$rowsOk"
    $report | ConvertTo-Json -Depth 8; exit 6
}

$report.ok = $true
$json = $report | ConvertTo-Json -Depth 8
if ($OutFile) { New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutFile) | Out-Null; Set-Content -Path $OutFile -Value $json -Encoding utf8 }
Write-Host $json
Write-Host "PASS fresh_mirdb_deploy：J1–J5 全过"
