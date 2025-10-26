# ============================================================================
# NPC系统自动化测试运行脚本
# ============================================================================
#
# 使用方法:
#   ./run_npc_tests.ps1                    # 运行所有测试
#   ./run_npc_tests.ps1 -Verbose          # 显示详细输出
#   ./run_npc_tests.ps1 -TestName "action" # 只运行包含"action"的测试

param(
    [string]$TestName = "",
    [switch]$Verbose = $false,
    [switch]$NoCaptureOutput = $false
)

$ErrorActionPreference = "Stop"

Write-Host "🧪 NPC系统自动化测试" -ForegroundColor Cyan
Write-Host "=" * 60

# 切换到ClientRust目录
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location (Join-Path $scriptDir "..")

# 构建测试命令
$testCmd = "cargo test --tests"

if ($TestName -ne "") {
    $testCmd += " $TestName"
    Write-Host "🔍 筛选测试: $TestName" -ForegroundColor Yellow
}

if ($NoCaptureOutput) {
    $testCmd += " -- --nocapture"
}

# 运行测试
Write-Host ""
Write-Host "▶️  执行命令: $testCmd" -ForegroundColor Green
Write-Host ""

$startTime = Get-Date

if ($Verbose) {
    # 详细模式 - 显示所有输出
    Invoke-Expression $testCmd
} else {
    # 简洁模式 - 只显示测试结果
    $output = Invoke-Expression "$testCmd 2>&1"
    
    # 解析输出
    $testResults = @()
    
    foreach ($line in $output) {
        if ($line -match "test (.+?) \.\.\. (.+)") {
            $testResults += [PSCustomObject]@{
                Name = $matches[1]
                Status = $matches[2]
            }
        }
    }
    
    # 显示测试结果摘要
    Write-Host "📊 测试结果摘要" -ForegroundColor Cyan
    Write-Host ("-" * 60)
    
    $passed = 0
    $failed = 0
    $ignored = 0
    
    foreach ($result in $testResults) {
        $icon = switch ($result.Status) {
            "ok" { "✅"; $passed++; break }
            "FAILED" { "❌"; $failed++; break }
            "ignored" { "⏭️"; $ignored++; break }
            default { "❓" }
        }
        
        $color = switch ($result.Status) {
            "ok" { "Green" }
            "FAILED" { "Red" }
            "ignored" { "Yellow" }
            default { "Gray" }
        }
        
        Write-Host "$icon $($result.Name)" -ForegroundColor $color
    }
    
    Write-Host ("-" * 60)
    Write-Host "✅ 通过: $passed" -ForegroundColor Green -NoNewline
    Write-Host " | " -NoNewline
    Write-Host "❌ 失败: $failed" -ForegroundColor Red -NoNewline
    Write-Host " | " -NoNewline
    Write-Host "⏭️  跳过: $ignored" -ForegroundColor Yellow
    
    # 如果有失败,显示详细错误
    if ($failed -gt 0) {
        Write-Host ""
        Write-Host "❌ 失败详情:" -ForegroundColor Red
        Write-Host $output | Select-String -Pattern "FAILED" -Context 5, 5
    }
}

$endTime = Get-Date
$duration = $endTime - $startTime

Write-Host ""
Write-Host "⏱️  总耗时: $($duration.TotalSeconds.ToString('F2'))秒" -ForegroundColor Cyan

# 返回退出码
if ($LASTEXITCODE -eq 0) {
    Write-Host "🎉 所有测试通过!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "💥 测试失败!" -ForegroundColor Red
    exit 1
}
