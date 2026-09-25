# _run_bot.ps1 —— ops 演练共用的「带超时的子进程 / bot 调用」
#
# 为什么需要它（2026-09-25 实测）：多个演练脚本此前都是 `& python bot.py … | Select-Object -Last 1`
# 这种**同步无超时**写法。一旦被测服务端没绑到 -Port（例如部署配置写 7000、而 -Port 传 7100），
# bot 会一直等下去 ⇒ 整个演练**静默挂死**（实测 storage_degrade_drill 挂了 6 分钟以上、无任何输出，
# 调用方分不清"在跑"还是"卡住"）。本文件把「有界等待 + 超时即杀 + 结果可判定」收敛成一处。
#
# 用法（在脚本里 dot-source）：
#   . (Join-Path $ops '_run_bot.ps1')
#   $r = Invoke-BotJson -BotArgs @('--host','127.0.0.1','--port',"$Port",'--login-only','--accounts',$Account,'--sessions','1','--password',$Password) -TimeoutSec 120 -Tag 'deploy_smoke'
#   if ($r.timedOut) { …按失败处理… } else { $smoke = $r.json }
#
# 自检（阳性对照）：`pwsh tools/ops/_run_bot.ps1 -SelfTest` —— 用一个必然睡眠 30s 的子进程配 2s 超时，
# 必须得到 timedOut=True 且退出码 0（自检通过）；若把超时机制去掉，这条自检会变成"等 30 秒"从而红。
param([switch]$SelfTest)

function Invoke-WithTimeout {
    <#
      起一个子进程并**有界等待**。返回：
        timedOut : 是否超时（超时会先杀掉进程）
        exitCode : 正常结束时的退出码
        outFile / errFile / durationSec
    #>
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [int]$TimeoutSec = 120,
        [string]$Tag = 'proc',
        [string]$OutDir = ''
    )
    # 默认输出目录走 TEMP：**不要**在函数里用 `$MyInvocation.MyCommand.Path` 求脚本目录——
    # 函数内它是空值（那是"命令"的路径，不是脚本的），会让 Start-Process 拿到空重定向路径而**静默没起进程**，
    # 自检于是"假通过"（第一版就踩了这个：进程没起来也算 timedOut=True）。调用方要自定义就传 -OutDir。
    if (-not $OutDir) { $OutDir = Join-Path $env:TEMP 'ops_run_bot' }
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    $stamp = "{0}_{1}" -f (Get-Random), (Get-Date -Format 'HHmmss')
    $outFile = Join-Path $OutDir ("{0}_{1}.out.json" -f $Tag, $stamp)
    $errFile = Join-Path $OutDir ("{0}_{1}.err.txt" -f $Tag, $stamp)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $FilePath -ArgumentList $Arguments `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile -PassThru -WindowStyle Hidden
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while (-not $p.HasExited -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
    $timedOut = -not $p.HasExited
    if ($timedOut) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    $sw.Stop()
    [pscustomobject]@{
        timedOut    = $timedOut
        exitCode    = if ($timedOut) { $null } else { $p.ExitCode }
        outFile     = $outFile
        errFile     = $errFile
        durationSec = [Math]::Round($sw.Elapsed.TotalSeconds, 1)
    }
}

function Invoke-BotJson {
    <#
      调 `bot.py`（参数由调用方给），解析它**最后一行** JSON（bot.py 会先打日志再打结果）。
      超时即返回 ok=$false / timedOut=$true，并把当时的 stdout/stderr 路径带回去（便于人工核对）。
    #>
    param(
        [Parameter(Mandatory)][string]$OpsDir,
        [Parameter(Mandatory)][string[]]$BotArgs,
        [int]$TimeoutSec = 120,
        [string]$Tag = 'bot',
        [string]$OutDir = ''
    )
    $r = Invoke-WithTimeout -FilePath 'python' `
        -Arguments (@((Join-Path $OpsDir 'bot.py')) + @($BotArgs)) `
        -TimeoutSec $TimeoutSec -Tag $Tag -OutDir $OutDir
    $json = $null
    if (-not $r.timedOut -and (Test-Path -LiteralPath $r.outFile)) {
        $last = Get-Content -LiteralPath $r.outFile -ErrorAction SilentlyContinue |
            Where-Object { $_.Trim() } | Select-Object -Last 1
        if ($last) { try { $json = $last | ConvertFrom-Json } catch { $json = $null } }
    }
    [pscustomobject]@{
        ok          = (-not $r.timedOut -and $null -ne $json)
        timedOut    = $r.timedOut
        exitCode    = $r.exitCode
        json        = $json
        outFile     = $r.outFile
        errFile     = $r.errFile
        durationSec = $r.durationSec
    }
}

if ($SelfTest) {
    Write-Host '[自检] 必然睡眠 30s 的子进程 + 2s 超时 → 期望 timedOut=True 且不阻塞'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    # 注意：Start-Process 的 -ArgumentList **不会**替你给含空格的参数加引号（它只是用空格拼起来），
    # 所以 `-c 'import time; time.sleep(30)'` 会被 python 当成 3 个参数而立刻报错退出（自检第一版因此假红）。
    # 这里显式加引号；同理，调用方传含空格的 bot 参数时也要自己加引号。
    $r = Invoke-WithTimeout -FilePath 'python' -Arguments @('-c', '"import time; time.sleep(30)"') `
        -TimeoutSec 2 -Tag 'selftest'
    $sw.Stop()
    $wall = [Math]::Round($sw.Elapsed.TotalSeconds, 1)
    if ($r.timedOut -and $wall -lt 10) {
        Write-Host ("[自检] PASS：timedOut=True，实际用时 {0}s（< 10s 说明真的被超时掐断）" -f $wall)
        exit 0
    }
    Write-Host ("[自检] FAIL：timedOut={0}，用时 {1}s（超时机制没生效？）" -f $r.timedOut, $wall)
    exit 1
}
