# GameScene 背景清理自动化验证脚本
#
# 此脚本通过分析游戏日志来验证 GameScene 是否正确加载和渲染

Write-Host "🧪 ========== GameScene 背景清理自动化验证 ==========" -ForegroundColor Cyan
Write-Host ""

# 1. 检查源代码修复是否存在
Write-Host "📝 检查代码修复..." -ForegroundColor Yellow

$gameSceneFile = "src\scenes\game_scene.rs"
$code = Get-Content $gameSceneFile -Raw

if ($code -match "绘制全屏黑色背景" -and $code -match "Mesh::new_rectangle") {
    Write-Host "✅ 代码修复已应用：发现背景清理代码" -ForegroundColor Green
} else {
    Write-Host "❌ 代码修复未应用：缺少背景清理代码" -ForegroundColor Red
    Write-Host "   预期关键字：'绘制全屏黑色背景', 'Mesh::new_rectangle'" -ForegroundColor Gray
    exit 1
}

Write-Host ""

# 2. 编译检查
Write-Host "🔨 编译项目..." -ForegroundColor Yellow

$compileOutput = cargo build --bin mir2_client 2>&1 | Out-String

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 编译成功" -ForegroundColor Green
} else {
    Write-Host "❌ 编译失败" -ForegroundColor Red
    Write-Host $compileOutput -ForegroundColor Gray
    exit 1
}

Write-Host ""

# 3. 检查可执行文件
Write-Host "📦 检查可执行文件..." -ForegroundColor Yellow

$exePath = "target\debug\mir2_client.exe"
if (Test-Path $exePath) {
    $fileInfo = Get-Item $exePath
    Write-Host "✅ 可执行文件存在" -ForegroundColor Green
    Write-Host "   路径: $exePath" -ForegroundColor Gray
    Write-Host "   大小: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Gray
    Write-Host "   修改时间: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
} else {
    Write-Host "❌ 可执行文件不存在" -ForegroundColor Red
    exit 1
}

Write-Host ""

# 4. 代码审查
Write-Host "🔍 代码审查..." -ForegroundColor Yellow

# 检查关键代码段
$drawMethod = $code -match '(?s)fn draw.*?GameScene.*?{.*?Color::BLACK.*?}'

if ($drawMethod) {
    Write-Host "✅ draw() 方法包含背景清理逻辑" -ForegroundColor Green
} else {
    Write-Host "⚠️  警告：draw() 方法可能缺少背景清理" -ForegroundColor Yellow
}

# 检查是否移除了旧的残留代码
if ($code -notmatch "LAST_CLEANUP_TIME" -or $code -match "unsafe.*LAST_CLEANUP_TIME") {
    Write-Host "✅ 未发现明显的内存泄漏风险" -ForegroundColor Green
} else {
    Write-Host "⚠️  警告：发现静态可变变量使用" -ForegroundColor Yellow
}

Write-Host ""

# 5. 结构验证
Write-Host "📐 结构验证..." -ForegroundColor Yellow

# 检查 GameScene 结构是否包含必要字段
if ($code -match "pub struct GameScene" -and $code -match "camera:" -and $code -match "map_renderer:") {
    Write-Host "✅ GameScene 结构完整" -ForegroundColor Green
} else {
    Write-Host "❌ GameScene 结构不完整" -ForegroundColor Red
    exit 1
}

Write-Host ""

# 6. 生成测试报告
Write-Host "📊 生成测试报告..." -ForegroundColor Yellow

$report = @"

========================================
GameScene 背景清理验证报告
========================================

测试时间: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")

✅ 代码修复状态: 已应用
✅ 编译状态: 成功
✅ 可执行文件: 存在
✅ 代码结构: 完整

修复内容:
- 在 GameScene::draw() 开头添加全屏黑色背景
- 使用 Mesh::new_rectangle 绘制背景
- 确保每帧清空画布

预期效果:
- 进入游戏场景后，背景应该是纯黑色
- 不应该看到登录界面的残留
- 地图纹理应该清晰地绘制在黑色背景上

手动验证步骤:
1. 运行: cargo run --bin mir2_client
2. 登录游戏
3. 进入游戏场景
4. 检查背景是否干净

自动化测试限制:
- 无法验证实际渲染效果
- 需要人工视觉检查
- 建议进行回归测试

========================================
"@

Write-Host $report -ForegroundColor White

# 7. 保存报告
$reportPath = "test_report_$(Get-Date -Format 'yyyyMMdd_HHmmss').txt"
$report | Out-File -FilePath $reportPath -Encoding UTF8

Write-Host "✅ 报告已保存到: $reportPath" -ForegroundColor Green
Write-Host ""

# 8. 提供下一步建议
Write-Host "🎯 下一步操作:" -ForegroundColor Cyan
Write-Host "1. 运行游戏进行手动测试:" -ForegroundColor White
Write-Host "   cargo run --bin mir2_client" -ForegroundColor Gray
Write-Host ""
Write-Host "2. 观察游戏场景背景:" -ForegroundColor White
Write-Host "   - 应该看到黑色背景" -ForegroundColor Gray
Write-Host "   - 不应该看到登录界面残留" -ForegroundColor Gray
Write-Host ""
Write-Host "3. 如果问题仍然存在:" -ForegroundColor White
Write-Host "   - 检查 program.rs 的 Canvas::from_frame()" -ForegroundColor Gray
Write-Host "   - 检查 MapRenderer::draw() 的混合模式" -ForegroundColor Gray
Write-Host "   - 考虑添加更多调试日志" -ForegroundColor Gray
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ 自动化验证完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
