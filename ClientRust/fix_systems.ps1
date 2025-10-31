# 批量修复脚本：将 name 和 priority 从 impl System 移到 impl SystemMeta
Get-ChildItem -Path 'src\ecs\systems\update' -Recurse -Filter '*.rs' | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    $modified = $false
    
    # 匹配pattern: impl System for XxxSystem {  fn name...  fn priority...
    if ($content -match '(?s)(impl System for (\w+System) \{)\s*(fn name\(&self\) -> &''static str \{[^\}]+\})\s*(fn priority\(&self\) -> u32 \{[^\}]+\})') {
        $systemName = $Matches[2]
        $implStart = $Matches[1]
        $nameMethod = $Matches[3]
        $priorityMethod = $Matches[4]
        
        # 创建新的 impl SystemMeta
        $newImpl = "impl crate::ecs::systems::SystemMeta for $systemName {`r`n    $nameMethod`r`n    `r`n    $priorityMethod`r`n}`r`n`r`n$implStart"
        
        # 替换内容
        $pattern = [regex]::Escape($implStart) + '\s*' + [regex]::Escape($nameMethod) + '\s*' + [regex]::Escape($priorityMethod)
        $newContent = $content -replace $pattern, $newImpl
        
        if ($newContent -ne $content) {
            Set-Content $_.FullName -Value $newContent -NoNewline
            Write-Host "修复: $($_.Name)"
            $modified = $true
        }
    }
}
