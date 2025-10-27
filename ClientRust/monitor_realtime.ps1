# 实时监控玩家移动关键事件
# 过滤掉噪音，只显示重要信息

Write-Host "🎮 启动游戏并实时监控关键事件..." -ForegroundColor Cyan
Write-Host "按 Ctrl+C 停止监控" -ForegroundColor Yellow
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor Cyan

# 运行游戏并过滤关键日志
cargo run --package mir2_client --bin mir2x 2>&1 | ForEach-Object {
    $line = $_.ToString()
    
    # 只显示关键事件
    if ($line -match "🎮 \[玩家移动\] 帧开始") {
        # 每10帧显示一次状态（减少输出）
        if ($script:frameCount % 10 -eq 0) {
            Write-Host "━━━ 第 $($script:frameCount) 帧 ━━━" -ForegroundColor DarkGray
        }
        $script:frameCount++
    }
    elseif ($line -match "🎯 \[输入\]") {
        Write-Host $line -ForegroundColor Yellow
    }
    elseif ($line -match "🖱️ \[输入\]") {
        Write-Host $line -ForegroundColor Cyan
    }
    elseif ($line -match "✅ \[移动\] DirectFollow到达目标") {
        Write-Host $line -ForegroundColor Green
    }
    elseif ($line -match "⛔ \[碰撞\]") {
        Write-Host $line -ForegroundColor Red
    }
    elseif ($line -match "🌐 \[网络\] 发送") {
        Write-Host $line -ForegroundColor Magenta
    }
    elseif ($line -match "🌐 \[网络\] 收到服务器位置") {
        Write-Host $line -ForegroundColor Blue
    }
    elseif ($line -match "⚠️ \[同步\] 位置偏差较大") {
        Write-Host $line -ForegroundColor Red -BackgroundColor Yellow
    }
    elseif ($line -match "🎮 \[同步\] DirectFollow模式: 忽略服务器位置") {
        Write-Host $line -ForegroundColor Green
    }
    elseif ($line -match "error|Error|ERROR|panic") {
        Write-Host "❌ ERROR: $line" -ForegroundColor Red -BackgroundColor White
    }
}

$script:frameCount = 0
