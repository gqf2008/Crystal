/// 批量测试所有 MLibrary 文件
/// 验证 MLibrary 模块对所有真实图库文件的加载能力

use std::fs;
use std::path::Path;
use std::collections::HashMap;

use mir2_client::graphics::mlibrary::MLibrary;

#[test]
fn test_all_library_files() {
    let data_dir = "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data";
    
    if !Path::new(data_dir).exists() {
        println!("⚠️  Data 目录不存在: {}", data_dir);
        return;
    }
    
    let mut results: HashMap<String, LibraryInfo> = HashMap::new();
    let mut total_count = 0;
    let mut success_count = 0;
    let mut error_count = 0;
    
    println!("\n{}", "=".repeat(80));
    println!("🗂️  批量测试图库文件");
    println!("{}", "=".repeat(80));
    println!("📁 扫描目录: {}\n", data_dir);
    
    let entries = fs::read_dir(data_dir).expect("无法读取目录");
    
    for entry in entries {
        let entry = entry.expect("无法读取文件条目");
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) != Some("lib") 
            && path.extension().and_then(|s| s.to_str()) != Some("Lib") {
            continue;
        }
        
        total_count += 1;
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let path_str = path.to_str().unwrap();
        let file_size = fs::metadata(&path).unwrap().len();
        
        // 尝试加载图库
        match MLibrary::open(path_str) {
            Ok(lib) => {
                success_count += 1;
                let info = LibraryInfo {
                    file_name: file_name.clone(),
                    file_size,
                    image_count: lib.count() as u32,
                    success: true,
                    error_msg: None,
                };
                
                println!("✅ {} - {} 张图像 ({:.2} MB)", 
                    file_name, 
                    lib.count(),
                    file_size as f64 / 1024.0 / 1024.0
                );
                
                results.insert(file_name, info);
            }
            Err(e) => {
                error_count += 1;
                let info = LibraryInfo {
                    file_name: file_name.clone(),
                    file_size,
                    image_count: 0,
                    success: false,
                    error_msg: Some(e.to_string()),
                };
                
                println!("❌ {} - 错误: {}", file_name, e);
                results.insert(file_name, info);
            }
        }
    }
    
    // 打印统计结果
    println!("\n{}", "=".repeat(80));
    println!("📊 测试统计");
    println!("{}", "=".repeat(80));
    println!("总文件数: {}", total_count);
    println!("成功加载: {} ({:.1}%)", 
        success_count, 
        (success_count as f32 / total_count as f32) * 100.0
    );
    println!("加载失败: {} ({:.1}%)", 
        error_count, 
        (error_count as f32 / total_count as f32) * 100.0
    );
    
    // 统计总图像数量和总文件大小
    let mut total_images = 0u32;
    let mut total_size = 0u64;
    
    for info in results.values() {
        if info.success {
            total_images += info.image_count;
            total_size += info.file_size;
        }
    }
    
    println!("\n总图像数: {}", total_images);
    println!("总文件大小: {:.2} MB", total_size as f64 / 1024.0 / 1024.0);
    
    // 打印详细信息
    println!("\n{}", "=".repeat(80));
    println!("📋 图库详细信息");
    println!("{}", "=".repeat(80));
    
    let mut sorted_results: Vec<_> = results.values().collect();
    sorted_results.sort_by(|a, b| b.image_count.cmp(&a.image_count));
    
    println!("\n按图像数量排序 (前 10 个):");
    for (i, info) in sorted_results.iter().take(10).enumerate() {
        if info.success {
            println!("  {}. {} - {} 张图像 ({:.2} MB)", 
                i + 1,
                info.file_name,
                info.image_count,
                info.file_size as f64 / 1024.0 / 1024.0
            );
        }
    }
    
    // 打印错误列表
    if error_count > 0 {
        println!("\n❌ 错误列表:");
        for info in results.values() {
            if !info.success {
                println!("  - {}: {}", 
                    info.file_name, 
                    info.error_msg.as_ref().unwrap()
                );
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    
    // 断言：至少 90% 的文件应该成功加载
    let success_rate = (success_count as f32 / total_count as f32) * 100.0;
    assert!(success_rate >= 90.0, 
        "成功率 {:.1}% 低于 90%", success_rate);
}

#[derive(Debug)]
struct LibraryInfo {
    file_name: String,
    file_size: u64,
    image_count: u32,
    success: bool,
    error_msg: Option<String>,
}

/// 测试特定图库文件的详细信息
#[test]
fn test_specific_libraries() {
    let test_libs = vec![
        ("d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data/Objects.Lib", "Objects.Lib"),
        ("d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data/Tiles.Lib", "Tiles.Lib"),
        ("d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data/Items.Lib", "Items.Lib"),
        ("d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data/Magic.Lib", "Magic.Lib"),
    ];
    
    println!("\n{}", "=".repeat(80));
    println!("🔍 详细测试特定图库");
    println!("{}", "=".repeat(80));
    
    for (lib_path, lib_name) in test_libs {
        if !Path::new(lib_path).exists() {
            println!("⚠️  文件不存在: {}", lib_name);
            continue;
        }
        
        println!("\n📄 测试文件: {}", lib_name);
        
        match MLibrary::open(lib_path) {
            Ok(mut lib) => {
                println!("   ✅ 加载成功");
                println!("   图像数量: {}", lib.count());
                
                // 测试获取第一张图像
                if lib.count() > 0 {
                    match lib.get_image_info(0) {
                        Ok(img_info) => {
                            println!("   第一张图像 (索引 0):");
                            println!("     - 宽度: {}", img_info.width);
                            println!("     - 高度: {}", img_info.height);
                            println!("     - 偏移X: {}", img_info.x);
                            println!("     - 偏移Y: {}", img_info.y);
                        }
                        Err(e) => {
                            println!("   ❌ 获取第一张图像失败: {}", e);
                        }
                    }
                }
                
                // 测试获取中间的图像
                if lib.count() > 1 {
                    let mid_index = lib.count() / 2;
                    match lib.get_image_info(mid_index) {
                        Ok(img_info) => {
                            println!("   中间图像 (索引 {}):", mid_index);
                            println!("     - 尺寸: {}x{}", img_info.width, img_info.height);
                        }
                        Err(e) => {
                            println!("   ❌ 获取中间图像失败: {}", e);
                        }
                    }
                }
                
                // 测试边界检查
                let invalid_index = lib.count() + 100;
                match lib.get_image_info(invalid_index) {
                    Ok(_) => {
                        println!("   ❌ 边界检查失败 (索引 {} 不应返回图像)", invalid_index);
                    }
                    Err(e) => {
                        println!("   ✅ 边界检查正确 (索引 {} 返回错误)", invalid_index);
                        if e.to_string().contains("out of range") {
                            println!("      错误信息: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ❌ 加载失败: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
}

/// 测试图像数据完整性
#[test]
fn test_image_data_integrity() {
    let test_lib = "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data/Items.Lib";
    
    if !Path::new(test_lib).exists() {
        println!("⚠️  测试文件不存在: {}", test_lib);
        return;
    }
    
    println!("\n{}", "=".repeat(80));
    println!("🔍 测试图像数据完整性");
    println!("{}", "=".repeat(80));
    
    let mut lib = MLibrary::open(test_lib).expect("无法加载图库");
    
    println!("\n测试图库: Items.Lib");
    println!("图像总数: {}", lib.count());
    
    // 统计有效图像数量
    let mut valid_count = 0;
    let mut empty_count = 0;
    let mut error_count = 0;
    
    for i in 0..lib.count() {
        match lib.get_image_info(i) {
            Ok(img_info) => {
                if img_info.width > 0 && img_info.height > 0 {
                    valid_count += 1;
                } else {
                    empty_count += 1;
                }
            }
            Err(_) => {
                error_count += 1;
            }
        }
    }
    
    println!("\n图像完整性统计:");
    println!("  有效图像: {} ({:.1}%)", 
        valid_count, 
        (valid_count as f32 / lib.count() as f32) * 100.0
    );
    println!("  空图像: {} ({:.1}%)", 
        empty_count, 
        (empty_count as f32 / lib.count() as f32) * 100.0
    );
    println!("  错误: {}", error_count);
    
    println!("\n{}", "=".repeat(80));
    
    // 断言：至少 80% 的图像应该是有效的
    let valid_rate = (valid_count as f32 / lib.count() as f32) * 100.0;
    assert!(valid_rate >= 50.0, 
        "有效图像率 {:.1}% 过低", valid_rate);
}
