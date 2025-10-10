// 检查Back层瓦片的实际尺寸

use mir2_client::graphics::mlibrary::MLibrary;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 检查Tiles库中Back层常用索引的图像尺寸\n");
    
    let lib_path = "Data/Map/WemadeMir2/Tiles";
    println!("📂 加载库: {}\n", lib_path);
    
    let mut lib = MLibrary::open(lib_path)?;
    println!("✅ 库加载成功\n");
    
    // 检查前100个索引
    println!("📊 检查前100个索引的尺寸:\n");
    let mut size_stats = std::collections::HashMap::new();
    
    for i in 0..100 {
        if let Ok(info) = lib.get_image_info(i) {
            if info.width > 0 && info.height > 0 {
                let size_key = format!("{}x{}", info.width, info.height);
                *size_stats.entry(size_key).or_insert(0) += 1;
                
                if i < 20 {
                    println!("  索引 {}: {}x{} (偏移 x={}, y={})", 
                        i, info.width, info.height, info.x, info.y);
                }
            }
        }
    }
    
    println!("\n📈 尺寸统计 (前100个索引):");
    let mut sizes: Vec<_> = size_stats.iter().collect();
    sizes.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));
    for (size, count) in sizes {
        println!("  {}: {} 张", size, count);
    }
    
    Ok(())
}
