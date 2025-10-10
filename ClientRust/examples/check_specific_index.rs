use mir2_client::graphics::mlibrary::MLibrary;

fn main() {
    println!("=== 检查特定图像索引 ===\n");
    
    let lib_path = "Data/Map/WemadeMir2/Tiles";
    
    match MLibrary::open(lib_path) {
        Ok(mut lib) => {
            let count = lib.count();
            println!("📚 Tiles库总图像数: {}\n", count);
            
            // 检查索引 69
            let test_indices = vec![68, 69, 70, 120, 121, 122, 123];
            
            for idx in test_indices {
                print!("索引 {}: ", idx);
                
                if idx >= count {
                    println!("❌ 超出范围 (max: {})", count - 1);
                    continue;
                }
                
                match lib.get_image_info(idx) {
                    Ok(info) => {
                        println!("✅ width={}, height={}, x={}, y={}, length={}", 
                            info.width, info.height, info.x, info.y, info.length);
                    }
                    Err(e) => {
                        println!("❌ 读取失败: {:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("❌ 无法加载Tiles库: {:?}", e);
        }
    }
}
