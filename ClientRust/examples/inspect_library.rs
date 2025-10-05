use mir2_client::graphics::mlibrary::MLibrary;
use std::io;

fn main() -> io::Result<()> {
    println!("=== Weather.lib 资源分析工具 ===\n");
    
    // 加载库文件
    let lib_path = "Data/Weather.lib";
    println!("📂 加载文件: {}", lib_path);
    
    let mut library = MLibrary::open(lib_path)?;
    
    println!("✅ 加载成功!\n");
    
    // 获取图片数量
    let image_count = library.count();
    println!("📊 图片总数: {} 张\n", image_count);
    
    // 分析图片尺寸分布
    println!("📐 图片尺寸分析:");
    println!("{:<6} | {:<8} | {:<8} | {:<10} | 偏移", "索引", "宽度", "高度", "数据大小");
    println!("{:-<60}", "");
    
    let mut total_size = 0u64;
    let mut min_width = u16::MAX;
    let mut max_width = 0u16;
    let mut min_height = u16::MAX;
    let mut max_height = 0u16;
    
    // 显示前 20 张图片的详细信息
    let display_count = if image_count > 20 { 20 } else { image_count };
    for i in 0..display_count {
        if let Ok(info) = library.get_image_info(i) {
            let width = info.width;
            let height = info.height;
            let size = info.length;
            let offset_x = info.x;
            let offset_y = info.y;
            
            println!("{:<6} | {:<8} | {:<8} | {:<10} | ({}, {})", 
                i, width, height, size, offset_x, offset_y);
            
            total_size += size as u64;
            min_width = min_width.min(width as u16);
            max_width = max_width.max(width as u16);
            min_height = min_height.min(height as u16);
            max_height = max_height.max(height as u16);
        }
    }
    
    if image_count > 20 {
        println!("... (显示前 20 张,共 {} 张)", image_count);
        
        // 计算所有图片的总大小
        for i in 0..image_count {
            if let Ok(info) = library.get_image_info(i) {
                total_size += info.length as u64;
            }
        }
    }
    
    println!("\n📈 统计信息:");
    println!("  • 图片数量: {} 张", image_count);
    println!("  • 宽度范围: {} - {} 像素", min_width, max_width);
    println!("  • 高度范围: {} - {} 像素", min_height, max_height);
    println!("  • 总数据量: {:.2} MB", total_size as f64 / 1024.0 / 1024.0);
    
    // 尺寸分类统计
    println!("\n📊 尺寸分布:");
    let mut size_distribution: std::collections::HashMap<(i16, i16), usize> = std::collections::HashMap::new();
    
    for i in 0..image_count {
        if let Ok(info) = library.get_image_info(i) {
            let size_key = (info.width, info.height);
            *size_distribution.entry(size_key).or_insert(0) += 1;
        }
    }
    
    let mut sizes: Vec<_> = size_distribution.iter().collect();
    sizes.sort_by(|a, b| b.1.cmp(a.1)); // 按数量降序排序
    
    println!("{:<15} | {:<10} | {:<8}", "尺寸 (宽×高)", "数量", "百分比");
    println!("{:-<45}", "");
    
    for ((width, height), count) in sizes.iter().take(10) {
        let percentage = (**count as f64 / image_count as f64) * 100.0;
        println!("{:<15} | {:<10} | {:.1}%", 
            format!("{}×{}", width, height), count, percentage);
    }
    
    if sizes.len() > 10 {
        println!("... (还有 {} 种其他尺寸)", sizes.len() - 10);
    }
    
    println!("\n✅ 分析完成!");
    
    Ok(())
}
