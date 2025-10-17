// 简单测试: 检查MLibrary加载
use std::path::Path;

fn main() {
    println!("=== MLibrary 加载测试 ===");
    
    // 检查Data目录
    if !Path::new("Data").exists() {
        println!("❌ Data 目录不存在!");
        println!("当前目录: {:?}", std::env::current_dir().unwrap());
        return;
    }
    
    println!("✅ Data 目录存在");
    
    // 检查ChrSel.Lib
    if !Path::new("Data/ChrSel.Lib").exists() {
        println!("❌ Data/ChrSel.Lib 文件不存在!");
        return;
    }
    
    println!("✅ Data/ChrSel.Lib 文件存在");
    
    // 尝试加载
    use mir2_client::graphics::libraries;
    
    println!("🔄 尝试加载核心库...");
    match libraries::load_core_libraries() {
        Ok(()) => {
            println!("✅ 核心库加载成功!");
        }
        Err(e) => {
            println!("❌ 核心库加载失败: {}", e);
        }
    }
    
    println!("\n=== 测试完成 ===");
}
