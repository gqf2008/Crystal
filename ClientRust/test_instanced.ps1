# 测试GPU实例化性能
# 自动运行程序60秒并收集输出

Write-Host "Starting Instanced Rendering Test..."
Write-Host "Please click on the window and press 'P' to generate particles"
Write-Host "Program will auto-close after 60 seconds"
Write-Host ""

# 启动程序
$process = Start-Process -FilePath "cargo" -ArgumentList "run" -WorkingDirectory "D:\Users\gxh\Documents\GitHub\Crystal\ClientRust" -PassThru -NoNewWindow

# 等待60秒
Start-Sleep -Seconds 60

# 尝试优雅关闭
if (-not $process.HasExited) {
    Write-Host "`nClosing program..."
    $process.CloseMainWindow() | Out-Null
    Start-Sleep -Seconds 2
    
    # 如果还没退出,强制kill
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
}

Write-Host "Test completed."
