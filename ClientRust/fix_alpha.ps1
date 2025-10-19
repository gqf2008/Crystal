# 修复 alpha 预处理代码
$file = "src\bin\map_viewer_bevy.rs"
$content = Get-Content $file -Raw

# 查找并替换整个 alpha 预处理块
$pattern = '(?s)(\s+// 转换BGRA.*?)((\s+})\s+if is_debug)'
$replacement = @'
        // 🔧 转换BGRA → RGBA（直接转换，不预处理alpha）
        // C#原版: Format.A8R8G8B8 + SpriteFlags.AlphaBlend (标准alpha混合)
        // Bevy: Rgba8UnormSrgb (默认标准alpha混合，无需预处理)
        let mut rgba_data = Vec::with_capacity(image_data.len());
        for chunk in image_data.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let alpha = chunk[3];
            
            // 直接BGRA→RGBA转换，保留原始数据
            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(alpha);
        }

        if is_debug
'@

if ($content -match $pattern) {
    $content = $content -replace $pattern, $replacement
    Set-Content $file $content -NoNewline
    Write-Host "✅ Alpha preprocessing code replaced successfully"
} else {
    Write-Host "❌ Pattern not found, trying manual approach..."
    
    # 尝试按行读取和替换
    $lines = Get-Content $file
    $output = New-Object System.Collections.ArrayList
    $skip = $false
    $found_start = $false
    
    for ($i = 0; $i < $lines.Count; $i++) {
        $line = $lines[$i]
        
        if ($line -match '\s+// 转换BGRA') {
            $found_start = $true
            $skip = $true
            # 添加新代码
            [void]$output.Add('        // 🔧 转换BGRA → RGBA（直接转换，不预处理alpha）')
            [void]$output.Add('        // C#原版: Format.A8R8G8B8 + SpriteFlags.AlphaBlend (标准alpha混合)')
            [void]$output.Add('        // Bevy: Rgba8UnormSrgb (默认标准alpha混合，无需预处理)')
            [void]$output.Add('        let mut rgba_data = Vec::with_capacity(image_data.len());')
            [void]$output.Add('        for chunk in image_data.chunks_exact(4) {')
            [void]$output.Add('            let b = chunk[0];')
            [void]$output.Add('            let g = chunk[1];')
            [void]$output.Add('            let r = chunk[2];')
            [void]$output.Add('            let alpha = chunk[3];')
            [void]$output.Add('            ')
            [void]$output.Add('            // 直接BGRA→RGBA转换，保留原始数据')
            [void]$output.Add('            rgba_data.push(r);')
            [void]$output.Add('            rgba_data.push(g);')
            [void]$output.Add('            rgba_data.push(b);')
            [void]$output.Add('            rgba_data.push(alpha);')
            [void]$output.Add('        }')
            continue
        }
        
        if ($skip -and $line -match '^\s+if is_debug') {
            $skip = $false
            [void]$output.Add('')
            [void]$output.Add($line)
            continue
        }
        
        if (-not $skip) {
            [void]$output.Add($line)
        }
    }
    
    if ($found_start) {
        $output | Set-Content $file
        Write-Host "✅ Manual replacement successful"
    } else {
        Write-Host "❌ Could not find the code to replace"
    }
}
