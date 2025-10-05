// examples/create_test_library.rs
// 
// 创建一个简单的测试库文件用于演示

use std::fs::File;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    println!("创建测试 Weather.lib 文件...");
    
    // 创建 Data 目录
    std::fs::create_dir_all("Data")?;
    
    // 创建一个最小的 .lib 文件
    // MLibrary 格式: 
    // Header (36 bytes): Count (4), Reserved (32)
    // IndexList: ImageInfo * Count
    // ImageList: 图像数据
    
    let mut file = File::create("Data/Weather.lib")?;
    
    // Header
    let count: i32 = 3;  // 3 张测试图像
    file.write_all(&count.to_le_bytes())?;
    file.write_all(&[0u8; 32])?;  // Reserved
    
    println!("✓ Header 写入完成 (36 bytes)");
    
    // ImageInfo 结构 (每个 20 bytes):
    // offset: i32, size: i32, width: i16, height: i16, 
    // offset_x: i16, offset_y: i16, shadow_x: i8, shadow_y: i8, shadow: u8, blend: u8
    
    let mut current_offset = 36 + (count * 20) as i32;  // Header + IndexList
    
    for i in 0..count {
        let width = 64i16;
        let height = 64i16;
        let data_size = (width as i32) * (height as i32) * 2;  // 假设 16-bit 像素
        
        // ImageInfo
        file.write_all(&current_offset.to_le_bytes())?;      // offset
        file.write_all(&data_size.to_le_bytes())?;           // size
        file.write_all(&width.to_le_bytes())?;               // width
        file.write_all(&height.to_le_bytes())?;              // height
        file.write_all(&0i16.to_le_bytes())?;                // offset_x
        file.write_all(&0i16.to_le_bytes())?;                // offset_y
        file.write_all(&[0u8])?;                             // shadow_x
        file.write_all(&[0u8])?;                             // shadow_y
        file.write_all(&[0u8])?;                             // shadow
        file.write_all(&[0u8])?;                             // blend
        
        println!("✓ ImageInfo {} 写入完成: {}x{}, offset={}, size={}", 
                 i, width, height, current_offset, data_size);
        
        current_offset += data_size;
    }
    
    println!("✓ IndexList 写入完成 (60 bytes)");
    
    // 写入虚拟图像数据（全白色半透明像素）
    for i in 0..count {
        let width = 64;
        let height = 64;
        let pixel_count = width * height;
        
        // 16-bit ARGB1555 格式: 1 bit alpha + 5 bits R + 5 bits G + 5 bits B
        // 半透明白色: alpha=1, R=31, G=31, B=31
        let white_pixel: u16 = 0b1_11111_11111_11111;
        
        for _ in 0..pixel_count {
            file.write_all(&white_pixel.to_le_bytes())?;
        }
        
        println!("✓ 图像数据 {} 写入完成 ({} pixels)", i, pixel_count);
    }
    
    println!("\n✅ Weather.lib 创建成功!");
    println!("文件大小: {} bytes", current_offset);
    println!("包含 {} 张 64x64 的测试图像", count);
    
    Ok(())
}
