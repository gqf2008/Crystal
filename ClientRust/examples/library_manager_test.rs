// examples/library_manager_test.rs
// 
// 全局库管理器功能测试
// 
// 测试内容:
// - 加载多个库文件
// - 从全局管理器获取库引用
// - 读取图片信息
// - 验证线程安全性

use mir2_client::graphics::{
    LibraryName,
    load_library,
    get_library,
    set_data_path,
};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 全局库管理器功能测试 ===\n");
    
    // 1. 设置数据路径
    println!("1️⃣  设置数据路径");
    set_data_path("Data");
    println!("   ✓ 数据路径设置为: Data/\n");
    
    // 2. 测试单个库加载
    println!("2️⃣  测试单个库加载 (Weather)");
    match load_library(LibraryName::Weather) {
        Ok(_) => println!("   ✓ Weather.lib 加载成功"),
        Err(e) => {
            eprintln!("   ✗ 加载失败: {}", e);
            return Err(e.into());
        }
    }
    
    // 3. 测试从全局管理器获取库
    println!("\n3️⃣  从全局管理器获取库引用");
    if let Some(lib_arc) = get_library(LibraryName::Weather) {
        println!("   ✓ 成功获取 Weather 库的 Arc 引用");
        
        // 4. 读取库信息
        println!("\n4️⃣  读取库信息");
        let mut library = lib_arc.lock().unwrap();
        let image_count = library.count();
        println!("   ✓ 图片总数: {} 张", image_count);
        
        // 5. 读取前几张图片的信息
        println!("\n5️⃣  读取前 10 张图片信息:");
        println!("   {:<6} | {:<8} | {:<8} | {:<10}", "索引", "宽度", "高度", "数据大小");
        println!("   {:-<40}", "");
        
        for i in 0..10.min(image_count) {
            if let Ok(info) = library.get_image_info(i) {
                println!("   {:<6} | {:<8} | {:<8} | {:<10}", 
                    i, info.width, info.height, info.length);
            }
        }
    } else {
        eprintln!("   ✗ 无法获取 Weather 库引用");
        return Err("Failed to get library".into());
    }
    
    // 6. 测试多次获取相同库(应该返回同一个实例)
    println!("\n6️⃣  测试多次获取相同库");
    let lib1 = get_library(LibraryName::Weather);
    let lib2 = get_library(LibraryName::Weather);
    
    if let (Some(l1), Some(l2)) = (lib1, lib2) {
        let same_instance = Arc::ptr_eq(&l1, &l2);
        if same_instance {
            println!("   ✓ 两次获取返回同一个实例 (正确的单例行为)");
        } else {
            println!("   ✗ 两次获取返回不同实例 (错误!)");
        }
    }
    
    // 7. 测试加载多个库
    println!("\n7️⃣  测试加载多个核心库");
    let libraries_to_load = vec![
        LibraryName::Prguse,
        LibraryName::Prguse2,
        LibraryName::Prguse3,
    ];
    
    for lib_name in &libraries_to_load {
        print!("   加载 {:?}... ", lib_name);
        match load_library(*lib_name) {
            Ok(_) => println!("✓"),
            Err(e) => println!("✗ ({})", e),
        }
    }
    
    // 8. 验证所有已加载的库
    println!("\n8️⃣  验证所有已加载的库");
    let mut loaded_count = 0;
    let all_libraries = [
        LibraryName::Weather,
        LibraryName::Prguse,
        LibraryName::Prguse2,
        LibraryName::Prguse3,
    ];
    
    for lib_name in &all_libraries {
        if get_library(*lib_name).is_some() {
            println!("   ✓ {:?} - 已加载", lib_name);
            loaded_count += 1;
        } else {
            println!("   ✗ {:?} - 未加载", lib_name);
        }
    }
    
    println!("\n   已加载库数量: {}/{}", loaded_count, all_libraries.len());
    
    // 9. 测试获取图片数据
    println!("\n9️⃣  测试加载图片数据 (Weather 索引 1-5)");
    if let Some(lib_arc) = get_library(LibraryName::Weather) {
        let mut library = lib_arc.lock().unwrap();
        
        for i in 1..=5 {
            match library.load_image_data(i) {
                Ok((info, data)) => {
                    println!("   ✓ 图片 {}: {}×{}, 数据大小: {} bytes", 
                        i, info.width, info.height, data.len());
                }
                Err(e) => {
                    println!("   ✗ 图片 {} 加载失败: {}", i, e);
                }
            }
        }
    }
    
    // 10. 线程安全测试
    println!("\n🔟 线程安全测试");
    use std::thread;
    
    let handles: Vec<_> = (0..5).map(|thread_id| {
        thread::spawn(move || {
            if let Some(lib_arc) = get_library(LibraryName::Weather) {
                let library = lib_arc.lock().unwrap();
                let count = library.count();
                println!("   线程 {}: Weather 库有 {} 张图片", thread_id, count);
                true
            } else {
                println!("   线程 {}: 无法获取库", thread_id);
                false
            }
        })
    }).collect();
    
    let mut success_count = 0;
    for handle in handles {
        if handle.join().unwrap() {
            success_count += 1;
        }
    }
    
    println!("   ✓ {}/5 个线程成功访问全局库", success_count);
    
    // 总结
    println!("\n{}", "=".repeat(50));
    println!("✅ 全局库管理器测试完成!");
    println!("{}", "=".repeat(50));
    
    Ok(())
}
