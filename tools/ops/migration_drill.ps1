# migration_drill.ps1 — 本机版「数据迁移演练」（对应外部项：脱敏生产数据迁移演练）
#
# 判据对齐外部项：① 备份（含哈希）② 迁移（定义库 + 账号库）③ 起服 + 抽样比对 ④ 真登录冒烟 ⑤ 回滚。
# 与外部项的唯一差别是**数据规模**：这里用原版演示数据（Server.MirDB + Server.MirADB + Daneo1989），
# 生产规模与耗时仍需外部快照，本脚本不据此下容量结论。
#
# 用法：
#   pwsh tools/ops/migration_drill.ps1 -MirDb <Server.MirDB> -MirAdb <Server.MirADB> `
#        -ServerDir ServerRust/target/release -DataRoot <含 Daneo1989 与 config/ 的目录> -Port 7600
param(
    [Parameter(Mandatory = $true)][string]$MirDb,
    [Parameter(Mandatory = $true)][string]$MirAdb,
    [Parameter(Mandatory = $true)][string]$ServerDir,
    [Parameter(Mandatory = $true)][string]$DataRoot,
    [string]$WorkDir = '',
    [int]$Port = 7600,
    [string]$ResetAccount = '333',        # 用哪个迁移账号做真登录（其角色所在图必须存在）
    [string]$ResetPassword = 'drill123456',
    [string]$OutFile = ''
)
$ErrorActionPreference = 'Continue'
$ops = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $WorkDir) { $WorkDir = Join-Path $ops ('out/migration_drill_' + (Get-Date -Format 'HHmmss')) }
New-Item -ItemType Directory -Force -Path $WorkDir, (Join-Path $WorkDir 'config'), (Join-Path $WorkDir 'Data') | Out-Null

$migrateDb = Join-Path $ServerDir 'migrate_mirdb.exe'
$migrateAccount = Join-Path $ServerDir 'migrate.exe'
$server = Join-Path $ServerDir 'mir2_server.exe'
foreach ($p in @($migrateDb, $migrateAccount, $server, $MirDb, $MirAdb, (Join-Path $DataRoot 'config/server.toml'))) {
    if (-not (Test-Path $p)) { Write-Host "FATAL: 缺 $p（先 cargo build --release）"; exit 2 }
}
$db = Join-Path $WorkDir 'Data/crystal.db'
$bak = "$db.bak"
$env:RUST_LOG = 'info'
$report = [ordered]@{ mir_db = $MirDb; mir_adb = $MirAdb; work_dir = $WorkDir; port = $Port; steps = [ordered]@{} }
function Fail([string]$msg) { Write-Host "FAIL $msg"; $report | ConvertTo-Json -Depth 8; exit 3 }

# 产物新鲜度（踩过：拿旧 release 跑演练 ⇒ 服务端没有本批密码兼容修复，登录被误判为失败）：
# exe 必须比 ServerRust/src 下最新源码还新，否则直接拒绝出结论。
$serverRoot = Split-Path -Parent $DataRoot
$newestSrc = Get-ChildItem (Join-Path $serverRoot 'src') -Recurse -File -Include *.rs |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($newestSrc) {
    foreach ($exe in @($server, $migrateDb, $migrateAccount)) {
        if ((Get-Item $exe).LastWriteTime -lt $newestSrc.LastWriteTime) {
            Fail "产物陈旧：$exe 早于源码 $($newestSrc.Name)（先 cargo build --release --bins）"
        }
    }
}

# ---------------- J2 迁移（定义库 + 账号库 → 同一个 DB） ----------------
$p = Start-Process -FilePath $migrateDb -ArgumentList @($MirDb, $db) -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $WorkDir 'migrate_db.log') -RedirectStandardError (Join-Path $WorkDir 'migrate_db.err.log') -Wait
$dbLog = Get-Content (Join-Path $WorkDir 'migrate_db.log') -EA SilentlyContinue
$parseErrors = @($dbLog | Select-String 'parse error').Count
if ($p.ExitCode -ne 0 -or $parseErrors -gt 0) { Fail "J2a 定义库迁移不干净（exit=$($p.ExitCode) parse_errors=$parseErrors）" }

$p = Start-Process -FilePath $migrateAccount -ArgumentList @($MirAdb, $db) -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $WorkDir 'migrate_acct.log') -RedirectStandardError (Join-Path $WorkDir 'migrate_acct.err.log') -Wait
$acctLog = Get-Content (Join-Path $WorkDir 'migrate_acct.log') -EA SilentlyContinue
$acctLine = ($acctLog | Select-String 'Accounts migrated:' | Select-Object -First 1).Line
$charLine = ($acctLog | Select-String 'Characters migrated:' | Select-Object -First 1).Line
$consumed = ($acctLog | Select-String 'File consumed:' | Select-Object -First 1).Line
$accounts = [int](($acctLine -replace '^.*migrated: ', '') -as [int])
$characters = [int](($charLine -replace '^.*migrated: ', '') -as [int])
$report.steps.migrate = [ordered]@{
    accounts = $accounts; characters = $characters
    consumed = ($consumed -replace '^.*migrate: ', '')
    parse_errors = $parseErrors
}
# 判据：**读满整个文件**（结构自证）+ 至少迁出 1 个账号
if ($consumed -notmatch '（完全对齐）' -or $accounts -lt 1) { Fail "J2b 账号库迁移未对齐（$consumed）" }

# ---------------- J1 备份（含哈希） ----------------
Copy-Item $db $bak -Force
$report.steps.backup = [ordered]@{
    db_sha256 = (Get-FileHash $db -Algorithm SHA256).Hash
    mir_db_sha256 = (Get-FileHash $MirDb -Algorithm SHA256).Hash
    mir_adb_sha256 = (Get-FileHash $MirAdb -Algorithm SHA256).Hash
    backup_file = $bak
}

# ---------------- J4a 抽样比对 + 密码重置（必须在起服**之前**：AccountActor 启动时把账号读进内存，
  # 起服后再改库不会被登录路径看到——实测踩过，表现为 login ok=false / frames=0） ----------------
$sample = (& python (Join-Path $ops 'mirdb_counts.py') $db --accounts $ResetAccount | Out-String).Trim() | ConvertFrom-Json
$report.steps.sample = $sample
if ($sample.accounts -lt 1 -or $sample.characters -lt 1 -or -not $sample.reset_salt) {
    Fail "J4a 抽样不达标（accounts=$($sample.accounts) characters=$($sample.characters) salt=$($sample.reset_salt)）"
}
$hashGen = & python (Join-Path $ops 'csharp_pbkdf2_hash.py') $sample.reset_salt $ResetPassword
& python (Join-Path $ops 'db_exec.py') $db "update accounts set password_hash='$hashGen' where username='$ResetAccount'" | Out-Null

# ---------------- J3 起服（含首启导入） ----------------
Copy-Item $server (Join-Path $WorkDir 'mir2_server.exe') -Force
$cfg = Get-Content (Join-Path $DataRoot 'config/server.toml') -Raw
$cfg = $cfg -replace 'listen_addr\s*=\s*"[^"]+"', "listen_addr = `"0.0.0.0:$Port`""
$cfg = $cfg -replace 'map_data_dir\s*=\s*"[^"]+"', "map_data_dir = `"$((Join-Path $DataRoot 'Daneo1989') -replace '\\','/')`""
Set-Content -Path (Join-Path $WorkDir 'config/server.toml') -Value $cfg -NoNewline -Encoding utf8

function StartDrillServer([string]$logName) {
    $log = Join-Path $WorkDir $logName
    $proc = Start-Process -FilePath (Join-Path $WorkDir 'mir2_server.exe') -WorkingDirectory $WorkDir `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err" -PassThru -WindowStyle Hidden
    for ($i = 0; $i -lt 90; $i++) {
        Start-Sleep 1
        if ((Get-Content $log -EA SilentlyContinue) -match 'Gate listening') { return @($proc, $log, $true) }
    }
    return @($proc, $log, $false)
}

$r = StartDrillServer 'run1.log'; $proc = $r[0]; $log1 = $r[1]; $ready = $r[2]
Start-Sleep 3
$imports = @(Get-Content $log1 -EA SilentlyContinue | Select-String 'Imported .* (drop|NPC script|NPC goods|recipes)').Count
$acctLoaded = (Get-Content $log1 -EA SilentlyContinue | Select-String 'Loaded (\d+) accounts' | Select-Object -First 1).Line
$defLoaded = @(Get-Content $log1 -EA SilentlyContinue | Select-String 'Loaded (\d+) (map|item) configs').Count
$report.steps.boot = [ordered]@{
    ready = $ready; imports = $imports
    accounts_line = $acctLoaded; definitions_loaded_lines = $defLoaded
}
if (-not $ready -or $imports -lt 4 -or $defLoaded -lt 2) { if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }; Fail "J3 起服/首启导入不达标" }

# ---------------- J4 真登录（用 C# 口径重置后的密码）+ 进图 ----------------
$botOut = (& python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --accounts $ResetAccount `
        --sessions 1 --password $ResetPassword --hold 5 | Out-String).Trim()
$bot = $null; try { $bot = $botOut | ConvertFrom-Json } catch {}
$loginOk = ($null -ne $bot) -and ($bot.summary.ok -eq 1) -and ($bot.sessions[0].frames -gt 0)
Start-Sleep 2
$after = (& python (Join-Path $ops 'db_exec.py') $db "select substr(password_hash,1,10) from accounts where username='$ResetAccount'" --read) -join ''
$report.steps.login = [ordered]@{
    account = $ResetAccount; ok = $loginOk
    frames = if ($bot) { $bot.sessions[0].frames } else { -1 }
    hash_after_login = $after.Trim()
}
if (-not $loginOk) { if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }; Fail "J4 迁移账号登录/进图失败" }

# ---------------- J5 回滚（恢复备份 + 哈希一致 + 再起服冒烟） ----------------
if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
Start-Sleep 2
Copy-Item $bak $db -Force
$restoredHash = (Get-FileHash $db -Algorithm SHA256).Hash
$r = StartDrillServer 'run2.log'; $proc2 = $r[0]; $log2 = $r[1]; $ready2 = $r[2]
$smoke2 = $null
if ($ready2) {
    $out2 = (& python (Join-Path $ops 'bot.py') --host 127.0.0.1 --port $Port --login-only `
            --accounts $ResetAccount --sessions 1 --password $ResetPassword 2>&1 | Select-Object -Last 1)
    try { $smoke2 = $out2 | ConvertFrom-Json } catch {}
}
if (-not $proc2.HasExited) { Stop-Process -Id $proc2.Id -Force }
$report.steps.rollback = [ordered]@{
    restored_hash_matches = ($restoredHash -eq $report.steps.backup.db_sha256)
    ready = $ready2
    login_after_rollback = if ($smoke2) { $smoke2.summary.ok } else { -1 }
}
if (-not $report.steps.rollback.restored_hash_matches -or -not $ready2) { Fail "J5 回滚校验失败" }

$report.ok = $true
$json = $report | ConvertTo-Json -Depth 8
if ($OutFile) { New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutFile) | Out-Null; Set-Content -Path $OutFile -Value $json -Encoding utf8 }
Write-Host $json
Write-Host 'PASS migration_drill：J1–J5 全过（备份/迁移/起服/抽样+真登录/回滚）'
