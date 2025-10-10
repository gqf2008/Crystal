// 详细检查解压后的原始数据
use mir2_client::graphics::mlibrary::MLibrary;
use std::io::Write;

fn main() -> std::io::Result<()> {
    println!("=== 检查原始解压数据 ===\n");
    
    let mut lib = MLibrary::open("Data/Map/WemadeMir2/Tiles")?;
    
    // 测试图像 0
    let index = 0;
    println!("图像 {}", index);
    
    let info = lib.get_image_info(index)?;
    println!("尺寸: {}x{}", info.width, info.height);
    
    // 加载原始数据（未转换）
    let (_, raw_data) = lib.load_image_data(index)?;
    
    println!("原始数据长度: {}", raw_data.len());
    println!("前32字节 (8个像素) 原始数据:");
    
    for i in 0..8 {
        let idx = i * 4;
        if idx + 3 < raw_data.len() {
            let b0 = raw_data[idx];
            let b1 = raw_data[idx + 1];
            let b2 = raw_data[idx + 2];
            let b3 = raw_data[idx + 3];
            println!("  像素{}: {:3} {:3} {:3} {:3}  (0x{:02X} 0x{:02X} 0x{:02X} 0x{:02X})",
                i, b0, b1, b2, b3, b0, b1, b2, b3);
        }
    }
    
    // 加载转换后的 RGBA 数据
    let (_, rgba_data) = lib.load_rgba_data(index)?;
    
    println!("\n转换后 RGBA 数据:");
    println!("前32字节 (8个像素):");
    
    for i in 0..8 {
        let idx = i * 4;
        if idx + 3 < rgba_data.len() {
            let r = rgba_data[idx];
            let g = rgba_data[idx + 1];
            let b = rgba_data[idx + 2];
            let a = rgba_data[idx + 3];
            println!("  像素{}: R={:3} G={:3} B={:3} A={:3}  (0x{:02X} 0x{:02X} 0x{:02X} 0x{:02X})",
                i, r, g, b, a, r, g, b, a);
        }
    }
    
    // 导出一小块到文件，用于手动检查
    if info.width >= 16 && info.height >= 16 {
        println!("\n导出16x16小块到 test_tile.raw (RGBA)");
        let mut file = std::fs::File::create("test_tile.raw")?;
        
        for y in 0..16 {
            for x in 0..16 {
                let idx = ((y * info.width as usize + x) * 4) as usize;
                if idx + 3 < rgba_data.len() {
                    file.write_all(&rgba_data[idx..idx+4])?;
                }
            }
        }
        println!("✅ 已导出 (可用 GIMP 打开: Image -> Mode -> RGB, 16x16)");
    }
    
    Ok(())
}
