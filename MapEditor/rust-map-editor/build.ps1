# 编译和测试脚本

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "Crystal Map Editor - Rust Port" -ForegroundColor Cyan
Write-Host "编译和测试" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# 进入项目目录
Set-Location "d:\Users\gxh\Documents\GitHub\Crystal.MapEditor\rust-map-editor"

Write-Host "1. 清理之前的构建..." -ForegroundColor Yellow
cargo clean

Write-Host ""
Write-Host "2. 检查编译错误..." -ForegroundColor Yellow
$checkResult = cargo check 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ 编译检查通过!" -ForegroundColor Green
} else {
    Write-Host "✗ 发现编译错误:" -ForegroundColor Red
    Write-Host $checkResult
    exit 1
}

Write-Host ""
Write-Host "3. 构建 Release 版本..." -ForegroundColor Yellow
$buildResult = cargo build --release 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ 构建成功!" -ForegroundColor Green
} else {
    Write-Host "✗ 构建失败:" -ForegroundColor Red
    Write-Host $buildResult
    exit 1
}

Write-Host ""
Write-Host "4. 运行测试..." -ForegroundColor Yellow
cargo test --release

Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "构建完成!" -ForegroundColor Green
Write-Host "可执行文件位置:" -ForegroundColor Cyan
Write-Host "  target\release\crystal-map-editor.exe" -ForegroundColor White
Write-Host ""
Write-Host "运行程序:" -ForegroundColor Cyan
Write-Host "  cargo run --release" -ForegroundColor White
Write-Host "或者:" -ForegroundColor Cyan
Write-Host "  .\target\release\crystal-map-editor.exe" -ForegroundColor White
Write-Host "======================================" -ForegroundColor Cyan
