// 检查库文件的图像数量

use mir2_client::graphics::mlibrary::MLibrary;

fn main() {
    println!("\n=== 检查地图库图像数量 ===\n");
    
    let libs = [
        ("Tiles", "Data/Map/WemadeMir2/Tiles"),
        ("Smtiles", "Data/Map/WemadeMir2/Smtiles"),
        ("Objects", "Data/Map/WemadeMir2/Objects"),
    ];
    
    for (name, path) in &libs {
        match MLibrary::open(path) {
            Ok(mut lib) => {
                let count = lib.count();
                println!("📚 {}: {} 个图像", name, count);
                
                // 测试一些索引
                let test_indices = [0, 1, 10, 100, 500, 1000, 1450, 1500, 1850, 1854];
                let mut valid_count = 0;
                for &idx in &test_indices {
                    if let Ok(_) = lib.get_image_info(idx) {
                        valid_count += 1;
                    }
                }
                println!("   测试索引可访问: {}/{}", valid_count, test_indices.len());
            }
            Err(e) => {
                println!("❌ {}: 加载失败 - {}", name, e);
            }
        }
    }
}
