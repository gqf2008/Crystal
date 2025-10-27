# 玩家移动日志分析器
# 自动过滤和总结关键移动事件

param(
    [int]$Seconds = 10  # 运行秒数
)

Write-Host "🎮 启动游戏并监控玩家移动..." -ForegroundColor Cyan
Write-Host "⏱️ 监控时长: $Seconds 秒" -ForegroundColor Yellow
Write-Host ""

# 启动游戏进程
$process = Start-Process -FilePath "cargo" -ArgumentList "run --package mir2_client --bin mir2x" -PassThru -NoNewWindow -RedirectStandardOutput "temp_game_log.txt" -RedirectStandardError "temp_game_error.txt" -WorkingDirectory "D:\Users\gxh\Documents\GitHub\Crystal\ClientRust"

Write-Host "⏳ 等待游戏启动和运行 $Seconds 秒..." -ForegroundColor Green
Start-Sleep -Seconds $Seconds

# 停止游戏
Write-Host "🛑 停止游戏..." -ForegroundColor Red
Stop-Process -Id $process.Id -Force

Start-Sleep -Seconds 2

# 分析日志
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "📊 日志分析报告" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$logContent = Get-Content "temp_game_log.txt" -ErrorAction SilentlyContinue
$errorContent = Get-Content "temp_game_error.txt" -ErrorAction SilentlyContinue

if ($logContent) {
    # 统计帧数
    $frames = ($logContent | Select-String "🎮 \[玩家移动\] 帧开始").Count
    Write-Host "📈 总帧数: $frames" -ForegroundColor Yellow
    
    # 统计移动模式切换
    $enterDirectFollow = ($logContent | Select-String "🎯 \[输入\] 进入DirectFollow模式").Count
    $updateTarget = ($logContent | Select-String "🎯 \[输入\] DirectFollow更新目标").Count
    $speedSwitch = ($logContent | Select-String "🎯 \[输入\] 切换速度").Count
    $mouseRelease = ($logContent | Select-String "🖱️ \[输入\] 松开鼠标").Count
    
    Write-Host ""
    Write-Host "🎮 输入事件统计:" -ForegroundColor Green
    Write-Host "  ├─ 进入DirectFollow: $enterDirectFollow 次"
    Write-Host "  ├─ 更新目标位置: $updateTarget 次"
    Write-Host "  ├─ 切换速度: $speedSwitch 次"
    Write-Host "  └─ 松开鼠标: $mouseRelease 次"
    
    # 统计移动事件
    $directFollowMove = ($logContent | Select-String "🚶 \[DirectFollow\] 开始移动").Count
    $reachTarget = ($logContent | Select-String "✅ \[移动\] DirectFollow到达目标").Count
    $collision = ($logContent | Select-String "⛔ \[碰撞\] 遇到障碍").Count
    $walkable = ($logContent | Select-String "✅ \[碰撞\] 可行走").Count
    
    Write-Host ""
    Write-Host "🚶 移动事件统计:" -ForegroundColor Green
    Write-Host "  ├─ DirectFollow移动: $directFollowMove 次"
    Write-Host "  ├─ 到达目标: $reachTarget 次"
    Write-Host "  ├─ 遇到障碍: $collision 次"
    Write-Host "  └─ 成功移动: $walkable 次"
    
    # 统计网络事件
    $sendWalk = ($logContent | Select-String "🌐 \[网络\] 发送Walk命令").Count
    $sendRun = ($logContent | Select-String "🌐 \[网络\] 发送Run命令").Count
    $sendTurn = ($logContent | Select-String "🌐 \[网络\] 发送Turn命令").Count
    $serverPos = ($logContent | Select-String "🌐 \[网络\] 收到服务器位置").Count
    $skipSync = ($logContent | Select-String "🎮 \[同步\] DirectFollow模式: 忽略服务器位置").Count
    $forceSync = ($logContent | Select-String "⚠️ \[同步\] 位置偏差较大! 强制同步").Count
    
    Write-Host ""
    Write-Host "🌐 网络事件统计:" -ForegroundColor Green
    Write-Host "  ├─ 发送Walk命令: $sendWalk 次"
    Write-Host "  ├─ 发送Run命令: $sendRun 次"
    Write-Host "  ├─ 发送Turn命令: $sendTurn 次"
    Write-Host "  ├─ 收到服务器位置: $serverPos 次"
    Write-Host "  ├─ 跳过同步(DirectFollow): $skipSync 次"
    Write-Host "  └─ 强制同步(偏差>1格): $forceSync 次"
    
    # 提取关键问题
    Write-Host ""
    Write-Host "⚠️ 潜在问题:" -ForegroundColor Red
    
    $hasIssues = $false
    
    if ($forceSync -gt 0) {
        Write-Host "  ❌ 检测到 $forceSync 次位置强制同步 - DirectFollow模式不应该被同步!" -ForegroundColor Red
        $hasIssues = $true
        
        # 显示强制同步的详细信息
        Write-Host ""
        Write-Host "  强制同步详情:" -ForegroundColor Yellow
        $syncLines = $logContent | Select-String "⚠️ \[同步\] 位置偏差较大! 强制同步" -Context 0,2
        foreach ($line in $syncLines | Select-Object -First 3) {
            Write-Host "    $($line.Line)" -ForegroundColor Gray
        }
    }
    
    if ($collision -gt $walkable * 0.2) {
        Write-Host "  ⚠️ 碰撞次数较多 ($collision 次) - 可能路径规划有问题" -ForegroundColor Yellow
        $hasIssues = $true
    }
    
    if ($reachTarget -lt $enterDirectFollow * 0.5) {
        Write-Host "  ⚠️ 到达目标次数 ($reachTarget) < 进入DirectFollow次数 ($enterDirectFollow) - 可能提前停止" -ForegroundColor Yellow
        $hasIssues = $true
    }
    
    if ($mouseRelease -gt 0 -and $reachTarget -eq 0) {
        Write-Host "  ❌ 松开鼠标后没有到达目标 - 可能立即停止了!" -ForegroundColor Red
        $hasIssues = $true
    }
    
    if (-not $hasIssues) {
        Write-Host "  ✅ 未检测到明显问题" -ForegroundColor Green
    }
    
    # 提取最后几次移动的详细信息
    Write-Host ""
    Write-Host "🔍 最后3次DirectFollow移动详情:" -ForegroundColor Cyan
    $moveBlocks = $logContent | Select-String "🚶 \[DirectFollow\] 开始移动" -Context 0,6
    $lastMoves = $moveBlocks | Select-Object -Last 3
    
    foreach ($move in $lastMoves) {
        Write-Host ""
        Write-Host "  ──────────────────────────────────" -ForegroundColor DarkGray
        $move.Context.PostContext | ForEach-Object {
            Write-Host "  $_" -ForegroundColor Gray
        }
    }
    
    # 显示最后的玩家状态
    Write-Host ""
    Write-Host "📍 最后的玩家状态:" -ForegroundColor Cyan
    $lastState = $logContent | Select-String "🎮 \[玩家移动\] 帧开始" -Context 0,5 | Select-Object -Last 1
    if ($lastState) {
        $lastState.Context.PostContext | ForEach-Object {
            Write-Host "  $_" -ForegroundColor Gray
        }
    }
    
} else {
    Write-Host "❌ 未找到日志文件" -ForegroundColor Red
}

# 检查错误日志
if ($errorContent) {
    $errors = $errorContent | Where-Object { $_ -match "error|Error|ERROR|panic" }
    if ($errors) {
        Write-Host ""
        Write-Host "❌ 错误日志 (前5条):" -ForegroundColor Red
        $errors | Select-Object -First 5 | ForEach-Object {
            Write-Host "  $_" -ForegroundColor Red
        }
    }
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "📝 完整日志已保存到:" -ForegroundColor Yellow
Write-Host "  - temp_game_log.txt" -ForegroundColor Gray
Write-Host "  - temp_game_error.txt" -ForegroundColor Gray
Write-Host ""
Write-Host "💡 提示: 使用以下命令查看完整日志" -ForegroundColor Green
Write-Host "  Get-Content temp_game_log.txt | Select-String 'DirectFollow'" -ForegroundColor Gray
Write-Host ""
