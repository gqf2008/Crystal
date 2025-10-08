# 检查传奇客户端数据文件

Write-Host "=== Crystal Mir2 Client - 数据文件检查 ===" -ForegroundColor Cyan
Write-Host ""

# 检查当前目录
$currentDir = Get-Location
Write-Host "当前目录: $currentDir" -ForegroundColor Yellow
Write-Host ""

# 1. 检查 Tiles 库
Write-Host "=== 1. Tiles 库检查 ===" -ForegroundColor Cyan

$tilesPaths = @(
    "Data\Tiles",
    "..\Data\Tiles",
    "..\..\Data\Tiles",
    "..\..\..\Build\Client\Data\Tiles"
)

$tilesFound = $false
foreach ($path in $tilesPaths) {
    if (Test-Path $path) {
        $libFiles = Get-ChildItem "$path\*.lib" -ErrorAction SilentlyContinue
        if ($libFiles.Count -gt 0) {
            Write-Host "✓ 找到 Tiles 库: $path" -ForegroundColor Green
            Write-Host "  共 $($libFiles.Count) 个文件:" -ForegroundColor Yellow
            $libFiles | Select-Object -First 5 | ForEach-Object { 
                $size = [math]::Round($_.Length / 1MB, 2)
                Write-Host "    - $($_.Name) ($size MB)" 
            }
            if ($libFiles.Count -gt 5) {
                Write-Host "    ... 还有 $($libFiles.Count - 5) 个文件"
            }
            $tilesFound = $true
            break
        }
    }
}

if (-not $tilesFound) {
    Write-Host "✗ 未找到 Tiles 库文件!" -ForegroundColor Red
    Write-Host "  请从传奇客户端复制 Tiles*.lib 文件" -ForegroundColor Yellow
    Write-Host "  例如: copy `"C:\MirClient\Data\Tiles\*.lib`" Data\Tiles\" -ForegroundColor Gray
}

Write-Host ""

# 2. 检查地图文件
Write-Host "=== 2. 地图文件检查 ===" -ForegroundColor Cyan

$mapPaths = @(
    "Data\Map",
    "..\Data\Map",
    "..\..\Data\Map",
    "..\..\..\Build\Client\Data\Map"
)

$mapFound = $false
foreach ($path in $mapPaths) {
    if (Test-Path $path) {
        $mapFiles = Get-ChildItem "$path\*.map" -ErrorAction SilentlyContinue
        if ($mapFiles.Count -gt 0) {
            Write-Host "✓ 找到地图文件: $path" -ForegroundColor Green
            Write-Host "  共 $($mapFiles.Count) 个地图:" -ForegroundColor Yellow
            $mapFiles | Select-Object -First 10 | ForEach-Object { 
                $size = [math]::Round($_.Length / 1KB, 2)
                Write-Host "    - $($_.Name) ($size KB)" 
            }
            if ($mapFiles.Count -gt 10) {
                Write-Host "    ... 还有 $($mapFiles.Count - 10) 个地图"
            }
            $mapFound = $true
            break
        }
    }
}

if (-not $mapFound) {
    Write-Host "✗ 未找到地图文件!" -ForegroundColor Red
    Write-Host "  请从传奇服务端复制 *.map 文件" -ForegroundColor Yellow
    Write-Host "  例如: copy `"C:\MirServer\Mir200\Envir\Map\*.map`" Data\Map\" -ForegroundColor Gray
}

Write-Host ""

# 3. 检查图像库
Write-Host "=== 3. 其他图像库检查 ===" -ForegroundColor Cyan

$prgusePath = "Data\Prguse.lib"
if (Test-Path $prgusePath) {
    $size = [math]::Round((Get-Item $prgusePath).Length / 1MB, 2)
    Write-Host "✓ 找到 Prguse.lib ($size MB)" -ForegroundColor Green
} else {
    Write-Host "✗ 未找到 Prguse.lib (UI 图像)" -ForegroundColor Yellow
}

$humPath = "Data\Hum.lib"
if (Test-Path $humPath) {
    $size = [math]::Round((Get-Item $humPath).Length / 1MB, 2)
    Write-Host "✓ 找到 Hum.lib ($size MB)" -ForegroundColor Green
} else {
    Write-Host "✗ 未找到 Hum.lib (玩家精灵)" -ForegroundColor Yellow
}

Write-Host ""

# 4. 总结
Write-Host "=== 诊断总结 ===" -ForegroundColor Cyan

if ($tilesFound -and $mapFound) {
    Write-Host "✓ 数据文件完整,地图应该可以正常显示!" -ForegroundColor Green
    Write-Host ""
    Write-Host "如果还是看不到地图,请:" -ForegroundColor Yellow
    Write-Host "  1. 运行客户端: cargo run --bin mir2_client"
    Write-Host "  2. 查看控制台日志,寻找错误信息"
    Write-Host "  3. 确认服务器已发送地图数据 (MapInformation 事件)"
} elseif ($tilesFound) {
    Write-Host "✗ 缺少地图文件,地图无法加载!" -ForegroundColor Red
    Write-Host ""
    Write-Host "解决方案:" -ForegroundColor Yellow
    Write-Host "  mkdir -p Data\Map"
    Write-Host "  copy `"C:\MirServer\Mir200\Envir\Map\*.map`" Data\Map\"
} elseif ($mapFound) {
    Write-Host "✗ 缺少 Tiles 库,瓦片无法显示!" -ForegroundColor Red
    Write-Host ""
    Write-Host "解决方案:" -ForegroundColor Yellow
    Write-Host "  mkdir -p Data\Tiles"
    Write-Host "  copy `"C:\MirClient\Data\Tiles\*.lib`" Data\Tiles\"
} else {
    Write-Host "✗ 缺少所有数据文件!" -ForegroundColor Red
    Write-Host ""
    Write-Host "快速修复 (假设传奇客户端在 C:\MirClient):" -ForegroundColor Yellow
    Write-Host "  mkdir -p Data\Tiles"
    Write-Host "  mkdir -p Data\Map"
    Write-Host "  copy `"C:\MirClient\Data\Tiles\*.lib`" Data\Tiles\"
    Write-Host "  copy `"C:\MirServer\Mir200\Envir\Map\*.map`" Data\Map\"
}

Write-Host ""
Write-Host "=== 检查完成 ===" -ForegroundColor Cyan
