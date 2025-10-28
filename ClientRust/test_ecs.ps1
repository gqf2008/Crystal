# ECS 功能测试脚本
# 用于测试重构后的ECS系统各项功能

Write-Host "🎮 ===== ECS 系统测试工具 =====" -ForegroundColor Cyan
Write-Host ""

$clientPath = "D:\Users\gxh\Documents\GitHub\Crystal\ClientRust"
$exePath = "$clientPath\target\release\mir2x.exe"

# 检查可执行文件
if (-not (Test-Path $exePath)) {
    Write-Host "❌ 找不到可执行文件: $exePath" -ForegroundColor Red
    Write-Host "正在编译..." -ForegroundColor Yellow
    Set-Location $clientPath
    cargo build --release
    if (-not $?) {
        Write-Host "❌ 编译失败！" -ForegroundColor Red
        exit 1
    }
}

Write-Host "✅ 找到可执行文件" -ForegroundColor Green
Write-Host ""

# 测试菜单
Write-Host "请选择测试项目:" -ForegroundColor Yellow
Write-Host "1. 运行完整游戏 (测试所有功能)"
Write-Host "2. 运行地图查看器 (仅测试渲染)"
Write-Host "3. 查看测试计划文档"
Write-Host "4. 检查崩溃日志"
Write-Host "5. 退出"
Write-Host ""

$choice = Read-Host "请输入选项 (1-5)"

switch ($choice) {
    "1" {
        Write-Host ""
        Write-Host "🚀 启动完整游戏..." -ForegroundColor Cyan
        Write-Host "📋 测试要点:" -ForegroundColor Yellow
        Write-Host "  - ✅ 程序是否正常启动"
        Write-Host "  - ✅ 地图是否正确加载"
        Write-Host "  - ✅ 角色是否显示"
        Write-Host "  - ✅ UI是否响应"
        Write-Host "  - 🔧 点击地面，角色是否移动"
        Write-Host "  - 🔧 打开/关闭各个对话框"
        Write-Host "  - 🔧 接近怪物，怪物是否追击"
        Write-Host ""
        Write-Host "按任意键继续..." -ForegroundColor Gray
        $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
        
        Set-Location $clientPath
        cargo run --bin mir2x --release
    }
    "2" {
        Write-Host ""
        Write-Host "🗺️ 启动地图查看器..." -ForegroundColor Cyan
        Set-Location $clientPath
        cargo run --bin map_viewer_ecs --release
    }
    "3" {
        Write-Host ""
        Write-Host "📖 打开测试计划文档..." -ForegroundColor Cyan
        $testPlanPath = "$clientPath\ECS_TEST_PLAN.md"
        if (Test-Path $testPlanPath) {
            code $testPlanPath
            Write-Host "✅ 已在VS Code中打开" -ForegroundColor Green
        } else {
            Write-Host "❌ 找不到文档: $testPlanPath" -ForegroundColor Red
        }
    }
    "4" {
        Write-Host ""
        Write-Host "🔍 检查崩溃日志..." -ForegroundColor Cyan
        Write-Host "TODO: 实现日志分析" -ForegroundColor Yellow
    }
    "5" {
        Write-Host ""
        Write-Host "👋 再见！" -ForegroundColor Cyan
        exit 0
    }
    default {
        Write-Host ""
        Write-Host "❌ 无效选项！" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "按任意键退出..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
