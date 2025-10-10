/// 批量测试所有地图文件的格式检测
/// 这个测试会扫描 Map 和 Data 目录中的所有 .map 文件
/// 并尝试加载它们，记录每个文件使用的格式类型

use std::fs;
use std::path::Path;
use std::collections::HashMap;

// 引入 MapReader
use mir2_client::objects::map_code::MapReader;

#[test]
fn test_all_map_files() {
    let map_dirs = vec![
        "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Map",
        "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data",
    ];
    
    let mut results: HashMap<String, Vec<String>> = HashMap::new();
    results.insert("Type 0".to_string(), Vec::new());
    results.insert("Type 1".to_string(), Vec::new());
    results.insert("Type 2".to_string(), Vec::new());
    results.insert("Type 3".to_string(), Vec::new());
    results.insert("Type 4".to_string(), Vec::new());
    results.insert("Type 5".to_string(), Vec::new());
    results.insert("Type 6".to_string(), Vec::new());
    results.insert("Type 7".to_string(), Vec::new());
    results.insert("Type 100".to_string(), Vec::new());
    results.insert("Error".to_string(), Vec::new());
    
    let mut total_count = 0;
    let mut success_count = 0;
    let mut error_count = 0;
    
    for dir in map_dirs {
        if !Path::new(dir).exists() {
            println!("⚠️  目录不存在: {}", dir);
            continue;
        }
        
        println!("\n🗂️  扫描目录: {}", dir);
        
        let entries = fs::read_dir(dir).expect("无法读取目录");
        
        for entry in entries {
            let entry = entry.expect("无法读取文件条目");
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) != Some("map") {
                continue;
            }
            
            total_count += 1;
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let path_str = path.to_str().unwrap();
            
            // 尝试加载地图
            match MapReader::new(path_str) {
                Ok(reader) => {
                    success_count += 1;
                    let format_type = detect_format_type(path_str);
                    println!("✅ {} - {} ({}x{})", 
                        file_name, format_type, reader.width, reader.height);
                    
                    if let Some(vec) = results.get_mut(&format_type) {
                        vec.push(file_name.to_string());
                    }
                }
                Err(e) => {
                    error_count += 1;
                    println!("❌ {} - 错误: {}", file_name, e);
                    results.get_mut("Error").unwrap().push(format!("{}: {}", file_name, e));
                }
            }
        }
    }
    
    // 打印统计结果
    println!("\n{}", "=".repeat(80));
    println!("📊 测试统计");
    println!("{}", "=".repeat(80));
    println!("总文件数: {}", total_count);
    println!("成功加载: {} ({:.1}%)", success_count, (success_count as f32 / total_count as f32) * 100.0);
    println!("加载失败: {} ({:.1}%)", error_count, (error_count as f32 / total_count as f32) * 100.0);
    
    println!("\n{}", "=".repeat(80));
    println!("📋 格式分布");
    println!("{}", "=".repeat(80));
    
    for i in 0..=7 {
        let key = format!("Type {}", i);
        if let Some(files) = results.get(&key) {
            if !files.is_empty() {
                println!("{}: {} 个文件", key, files.len());
                // 打印前3个示例
                for (idx, file) in files.iter().take(3).enumerate() {
                    println!("  {}. {}", idx + 1, file);
                }
                if files.len() > 3 {
                    println!("  ... 还有 {} 个文件", files.len() - 3);
                }
            }
        }
    }
    
    if let Some(files) = results.get("Type 100") {
        if !files.is_empty() {
            println!("Type 100: {} 个文件", files.len());
            for file in files.iter().take(3) {
                println!("  - {}", file);
            }
        }
    }
    
    if let Some(errors) = results.get("Error") {
        if !errors.is_empty() {
            println!("\n❌ 错误列表 ({} 个):", errors.len());
            for error in errors.iter().take(10) {
                println!("  - {}", error);
            }
            if errors.len() > 10 {
                println!("  ... 还有 {} 个错误", errors.len() - 10);
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    
    // 断言：至少 90% 的文件应该成功加载
    let success_rate = (success_count as f32 / total_count as f32) * 100.0;
    assert!(success_rate >= 90.0, 
        "成功率 {:.1}% 低于 90%，可能存在未实现的格式", success_rate);
}

/// 通过文件头检测地图格式
fn detect_format_type(path: &str) -> String {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return "Unknown".to_string(),
    };
    
    if bytes.len() < 20 {
        return "Invalid".to_string();
    }
    
    // C# 自定义格式
    if bytes[2] == 0x43 && bytes[3] == 0x23 {
        return "Type 100".to_string();
    }
    
    // Wemade Mir3
    if bytes[0] == 0 {
        return "Type 5".to_string();
    }
    
    // Shanda Mir3
    if bytes[0] == 0x0F && bytes[5] == 0x53 && bytes[14] == 0x33 {
        return "Type 6".to_string();
    }
    
    // Wemade AntiHack
    if bytes[0] == 0x15 && bytes[4] == 0x32 && bytes[6] == 0x41 && bytes[19] == 0x31 {
        return "Type 4".to_string();
    }
    
    // Map 2010 Ver 1.0
    if bytes[0] == 0x10 && bytes[2] == 0x61 && bytes[7] == 0x31 && bytes[14] == 0x31 {
        return "Type 1".to_string();
    }
    
    // Shanda 2012 或旧格式
    if (bytes[4] == 0x0F || bytes[4] == 0x03) && bytes[18] == 0x0D && bytes[19] == 0x0A {
        let w = bytes[0] as usize + ((bytes[1] as usize) << 8);
        let h = bytes[2] as usize + ((bytes[3] as usize) << 8);
        
        if bytes.len() > (52 + (w * h * 14)) {
            return "Type 3".to_string();
        } else {
            return "Type 2".to_string();
        }
    }
    
    // 3/4 Heroes
    if bytes[0] == 0x0D && bytes[1] == 0x4C && bytes[7] == 0x20 && bytes[11] == 0x6D {
        return "Type 7".to_string();
    }
    
    // 默认格式
    "Type 0".to_string()
}

/// 测试特定地图文件的详细信息
#[test]
fn test_specific_maps() {
    let test_maps = vec![
        "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Map/0.map",
        "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Map/1.map",
        "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Map/2.map",
    ];
    
    println!("\n{}", "=".repeat(80));
    println!("🔍 详细测试特定地图");
    println!("{}", "=".repeat(80));
    
    for map_path in test_maps {
        if !Path::new(map_path).exists() {
            println!("⚠️  文件不存在: {}", map_path);
            continue;
        }
        
        let file_name = Path::new(map_path).file_name().unwrap().to_str().unwrap();
        let format_type = detect_format_type(map_path);
        
        println!("\n📄 测试文件: {}", file_name);
        println!("   格式类型: {}", format_type);
        
        match MapReader::new(map_path) {
            Ok(reader) => {
                println!("   ✅ 加载成功");
                println!("   地图尺寸: {}x{}", reader.width, reader.height);
                
                // 检查第一个格子
                if let Some(cell) = reader.get_cell(0, 0) {
                    println!("   第一格 (0,0):");
                    println!("     - BackImage: {}", cell.back_image);
                    println!("     - MiddleImage: {}", cell.middle_image);
                    println!("     - FrontImage: {}", cell.front_image);
                    println!("     - Light: {}", cell.light);
                }
                
                // 检查中心格子
                let center_x = reader.width / 2;
                let center_y = reader.height / 2;
                if let Some(cell) = reader.get_cell(center_x, center_y) {
                    println!("   中心格 ({},{}):", center_x, center_y);
                    println!("     - BackImage: {}", cell.back_image);
                    println!("     - MiddleImage: {}", cell.middle_image);
                    println!("     - FrontImage: {}", cell.front_image);
                }
            }
            Err(e) => {
                println!("   ❌ 加载失败: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
}
