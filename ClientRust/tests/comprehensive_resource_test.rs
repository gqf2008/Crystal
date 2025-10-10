/// 全面的资源文件测试
/// 扫描 Data 和 Map 目录下的所有资源文件，验证数据解析的正确性

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::Instant;

use mir2_client::graphics::mlibrary::MLibrary;
use mir2_client::objects::map_code::MapReader;

#[derive(Debug, Clone)]
struct TestStats {
    total_files: usize,
    success_count: usize,
    error_count: usize,
    total_images: usize,
    total_size_bytes: u64,
    errors: Vec<(String, String)>, // (file_path, error_message)
}

impl TestStats {
    fn new() -> Self {
        Self {
            total_files: 0,
            success_count: 0,
            error_count: 0,
            total_images: 0,
            total_size_bytes: 0,
            errors: Vec::new(),
        }
    }

    fn add_success(&mut self, image_count: usize, file_size: u64) {
        self.total_files += 1;
        self.success_count += 1;
        self.total_images += image_count;
        self.total_size_bytes += file_size;
    }

    fn add_error(&mut self, file_path: String, error: String) {
        self.total_files += 1;
        self.error_count += 1;
        self.errors.push((file_path, error));
    }

    fn print_summary(&self, category: &str) {
        println!("\n{}", "=".repeat(80));
        println!("📊 {} - 测试统计", category);
        println!("{}", "=".repeat(80));
        println!("总文件数: {}", self.total_files);
        println!("成功加载: {} ({:.1}%)", 
            self.success_count, 
            if self.total_files > 0 { 
                (self.success_count as f64 / self.total_files as f64) * 100.0 
            } else { 
                0.0 
            }
        );
        println!("加载失败: {} ({:.1}%)", 
            self.error_count,
            if self.total_files > 0 { 
                (self.error_count as f64 / self.total_files as f64) * 100.0 
            } else { 
                0.0 
            }
        );
        
        if self.total_images > 0 {
            println!("总图像数: {}", self.total_images);
        }
        
        println!("总文件大小: {:.2} MB", self.total_size_bytes as f64 / 1024.0 / 1024.0);

        if !self.errors.is_empty() {
            println!("\n❌ 错误列表:");
            for (i, (file, error)) in self.errors.iter().enumerate() {
                println!("  {}. {} - {}", i + 1, file, error);
            }
        }
    }
}

/// 递归扫描并测试所有 .Lib 文件
fn test_library_files_recursive(base_dir: &str) -> TestStats {
    let mut stats = TestStats::new();
    let base_path = Path::new(base_dir);
    
    if !base_path.exists() {
        println!("⚠️  目录不存在: {}", base_dir);
        return stats;
    }

    println!("\n{}", "=".repeat(80));
    println!("🗂️  扫描图库文件 (包括子目录)");
    println!("{}", "=".repeat(80));
    println!("📁 基础目录: {}\n", base_dir);

    // 收集所有子目录的统计
    let mut dir_stats: HashMap<String, (usize, usize)> = HashMap::new(); // (success, total)

    // 递归遍历所有文件
    visit_dirs(base_path, &mut |entry: &PathBuf| {
        if let Some(ext) = entry.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            if ext_str.eq_ignore_ascii_case("lib") {
                let file_path = entry.to_str().unwrap();
                let file_name = entry.file_name().unwrap().to_str().unwrap();
                let relative_dir = entry.parent()
                    .and_then(|p| p.strip_prefix(base_path).ok())
                    .map(|p| p.to_str().unwrap_or(""))
                    .unwrap_or("");
                
                let dir_key = if relative_dir.is_empty() {
                    "Data/".to_string()
                } else {
                    format!("Data/{}/", relative_dir)
                };

                match fs::metadata(entry) {
                    Ok(metadata) => {
                        let file_size = metadata.len();
                        
                        match MLibrary::open(file_path) {
                            Ok(lib) => {
                                let image_count = lib.count();
                                stats.add_success(image_count, file_size);
                                
                                // 更新目录统计
                                let entry = dir_stats.entry(dir_key.clone()).or_insert((0, 0));
                                entry.0 += 1;
                                entry.1 += 1;
                                
                                println!("  ✅ {}{} - {} 张图像 ({:.2} MB)", 
                                    if relative_dir.is_empty() { "" } else { &format!("{}/", relative_dir) },
                                    file_name,
                                    image_count,
                                    file_size as f64 / 1024.0 / 1024.0
                                );
                            }
                            Err(e) => {
                                stats.add_error(file_path.to_string(), e.to_string());
                                
                                // 更新目录统计
                                let entry = dir_stats.entry(dir_key.clone()).or_insert((0, 0));
                                entry.1 += 1;
                                
                                println!("  ❌ {}{} - 错误: {}", 
                                    if relative_dir.is_empty() { "" } else { &format!("{}/", relative_dir) },
                                    file_name,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        stats.add_error(file_path.to_string(), format!("无法读取文件元数据: {}", e));
                    }
                }
            }
        }
    });

    // 打印各子目录统计
    println!("\n{}", "-".repeat(80));
    println!("📂 各子目录统计:");
    let mut dir_vec: Vec<_> = dir_stats.iter().collect();
    dir_vec.sort_by(|a, b| b.1.1.cmp(&a.1.1)); // 按总数排序
    
    for (dir, (success, total)) in dir_vec {
        let percentage = (*success as f64 / *total as f64) * 100.0;
        let status = if *success == *total { "✅" } else { "⚠️ " };
        println!("  {} {} - {}/{} ({:.1}%)", status, dir, success, total, percentage);
    }

    stats
}

/// 测试所有地图文件
fn test_map_files(map_dir: &str) -> TestStats {
    let mut stats = TestStats::new();
    let map_path = Path::new(map_dir);
    
    if !map_path.exists() {
        println!("⚠️  目录不存在: {}", map_dir);
        return stats;
    }

    println!("\n{}", "=".repeat(80));
    println!("🗺️  扫描地图文件");
    println!("{}", "=".repeat(80));
    println!("📁 目录: {}\n", map_dir);

    let entries = match fs::read_dir(map_path) {
        Ok(e) => e,
        Err(e) => {
            println!("❌ 无法读取目录: {}", e);
            return stats;
        }
    };

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                let file_path = path.to_str().unwrap();
                let file_name = path.file_name().unwrap().to_str().unwrap();
                
                match fs::metadata(&path) {
                    Ok(metadata) => {
                        let file_size = metadata.len();
                        
                        match MapReader::new(file_path) {
                            Ok(map) => {
                                stats.add_success(0, file_size);
                                println!("  ✅ {} - {}x{} ({:.2} KB)", 
                                    file_name,
                                    map.width,
                                    map.height,
                                    file_size as f64 / 1024.0
                                );
                            }
                            Err(e) => {
                                stats.add_error(file_path.to_string(), e.to_string());
                                println!("  ❌ {} - 错误: {}", file_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        stats.add_error(file_path.to_string(), format!("无法读取文件元数据: {}", e));
                    }
                }
            }
        }
    }

    stats
}

/// 递归访问目录
fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&PathBuf)) {
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        visit_dirs(&path, cb);
                    } else {
                        cb(&path);
                    }
                }
            }
        }
    }
}

#[test]
fn test_all_resources() {
    let start_time = Instant::now();
    
    println!("\n");
    println!("{}", "█".repeat(80));
    println!("🧪 Crystal - 全面资源文件测试");
    println!("{}", "█".repeat(80));
    
    // 测试图库文件
    let data_dir = "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data";
    let lib_stats = test_library_files_recursive(data_dir);
    lib_stats.print_summary("图库文件 (MLibrary)");
    
    // 测试地图文件
    let map_dir = "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Map";
    let map_stats = test_map_files(map_dir);
    map_stats.print_summary("地图文件 (MapCode)");
    
    // 汇总统计
    println!("\n{}", "=".repeat(80));
    println!("🎯 总体统计");
    println!("{}", "=".repeat(80));
    println!("总文件数: {}", lib_stats.total_files + map_stats.total_files);
    println!("成功加载: {}", lib_stats.success_count + map_stats.success_count);
    println!("加载失败: {}", lib_stats.error_count + map_stats.error_count);
    println!("总图像数: {}", lib_stats.total_images);
    println!("总数据量: {:.2} GB", 
        (lib_stats.total_size_bytes + map_stats.total_size_bytes) as f64 / 1024.0 / 1024.0 / 1024.0
    );
    
    let elapsed = start_time.elapsed();
    println!("\n⏱️  总耗时: {:.2}秒", elapsed.as_secs_f64());
    
    println!("\n{}", "█".repeat(80));
    
    // 断言：所有文件都应该成功加载
    let total_errors = lib_stats.error_count + map_stats.error_count;
    if total_errors > 0 {
        println!("\n❌ 测试失败：{} 个文件加载失败", total_errors);
        panic!("资源文件测试失败：{} 个错误", total_errors);
    } else {
        println!("\n✅ 测试通过：所有资源文件解析成功！");
    }
}

#[test]
fn test_library_files_only() {
    let data_dir = "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Data";
    let stats = test_library_files_recursive(data_dir);
    stats.print_summary("图库文件");
    
    assert_eq!(stats.error_count, 0, "图库文件加载失败数: {}", stats.error_count);
}

#[test]
fn test_map_files_only() {
    let map_dir = "d:/Users/gxh/Documents/GitHub/Crystal/ClientRust/Map";
    let stats = test_map_files(map_dir);
    stats.print_summary("地图文件");
    
    assert_eq!(stats.error_count, 0, "地图文件加载失败数: {}", stats.error_count);
}
