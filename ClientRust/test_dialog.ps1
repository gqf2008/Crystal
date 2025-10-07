# NewCharacterDialog 快速测试指南
# 使用方法: 直接运行此脚本 `.\test_dialog.ps1`

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "🎮 NewCharacterDialog 测试工具" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 检查是否在正确的目录
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "❌ 错误: 请在 ClientRust 目录下运行此脚本!" -ForegroundColor Red
    Write-Host "正确路径: d:\Users\gxh\Documents\GitHub\Crystal\ClientRust" -ForegroundColor Yellow
    exit 1
}

Write-Host "📦 正在编译游戏 (release模式)..." -ForegroundColor Yellow
Write-Host ""

# 编译游戏
$buildOutput = cargo build --bin mir2_client --release 2>&1
$buildSuccess = $LASTEXITCODE -eq 0

if ($buildSuccess) {
    Write-Host "✅ 编译成功!" -ForegroundColor Green
} else {
    Write-Host "❌ 编译失败!" -ForegroundColor Red
    Write-Host $buildOutput
    exit 1
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "🚀 启动游戏测试" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "测试步骤:" -ForegroundColor Yellow
Write-Host "1. 登录游戏账号" -ForegroundColor White
Write-Host "2. 进入角色选择界面" -ForegroundColor White
Write-Host "3. 按 C键 或点击 '新建角色' 按钮" -ForegroundColor White
Write-Host "4. 观察对话框是否正确显示" -ForegroundColor White
Write-Host ""

Write-Host "功能检查清单:" -ForegroundColor Yellow
Write-Host "  ✓ 对话框背景和标题" -ForegroundColor White
Write-Host "  ✓ 5个职业按钮 (战士/法师/道士/刺客/弓箭手)" -ForegroundColor White
Write-Host "  ✓ 2个性别按钮 (男/女)" -ForegroundColor White
Write-Host "  ✓ 角色预览动画 (16帧循环)" -ForegroundColor White
Write-Host "  ✓ 角色名称输入框 (支持中文)" -ForegroundColor White
Write-Host "  ✓ 光标闪烁效果" -ForegroundColor White
Write-Host "  ✓ 职业描述文本更新" -ForegroundColor White
Write-Host "  ✓ 确认/取消按钮" -ForegroundColor White
Write-Host ""

Write-Host "按任意键启动游戏..." -ForegroundColor Cyan
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

Write-Host ""
Write-Host "🎮 正在启动游戏..." -ForegroundColor Green
Write-Host ""

# 启动游戏
.\target\release\mir2_client.exe

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "游戏已退出" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "如果发现问题,请查看日志或报告给开发团队。" -ForegroundColor Yellow
Write-Host ""
Write-Host "完整功能报告: NEW_CHARACTER_DIALOG_完成报告.md" -ForegroundColor Cyan
Write-Host ""
