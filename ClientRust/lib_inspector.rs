// .lib 文件查看工具
// 用于查看 MLibrary 文件中的图像信息,帮助找到正确的索引

use std::env;
use std::path::PathBuf;

// 简单的 MLibrary 头部读取
fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("用法: cargo run --bin lib_inspector <库名>");
        println!("例如: cargo run --bin lib_inspector Prguse");
        println!("\n可用的库:");
        println!("  Prguse   - UI 元素、对话框、按钮");
        println!("  Title    - 标题和背景");
        println!("  Background - 背景图像");
        println!("  Items    - 物品图标");
        println!("  Magic    - 魔法特效");
        return;
    }
    
    let lib_name = &args[1];
    let lib_path = PathBuf::from(format!("Data/{}.lib", lib_name));
    
    if !lib_path.exists() {
        println!("❌ 文件不存在: {:?}", lib_path);
        return;
    }
    
    println!("📚 查看库: {}", lib_name);
    println!("📁 路径: {:?}", lib_path);
    println!("═══════════════════════════════════════\n");
    
    // 使用项目中的 MLibrary 代码
    match load_and_inspect(&lib_path) {
        Ok(_) => println!("\n✓ 查看完成"),
        Err(e) => println!("\n❌ 错误: {}", e),
    }
}

fn load_and_inspect(path: &PathBuf) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    
    let mut file = File::open(path)?;
    
    // 读取文件大小
    let file_size = file.metadata()?.len();
    println!("文件大小: {} bytes ({:.2} MB)\n", file_size, file_size as f64 / 1024.0 / 1024.0);
    
    // 读取 MLibrary 头部 (12字节: version(4) + count(4) + frame_seek(4))
    let mut version_buf = [0u8; 4];
    let mut count_buf = [0u8; 4];
    let mut frame_seek_buf = [0u8; 4];
    
    file.read_exact(&mut version_buf)?;
    file.read_exact(&mut count_buf)?;
    file.read_exact(&mut frame_seek_buf)?;
    
    let version = i32::from_le_bytes(version_buf);
    let image_count = i32::from_le_bytes(count_buf) as usize;
    let frame_seek = i32::from_le_bytes(frame_seek_buf);
    
    println!("版本: {}", version);
    println!("Frame Seek: {}", frame_seek);
    
    println!("图像数量: {}\n", image_count);
    
    if image_count == 0 || image_count > 100000 {
        println!("⚠️  图像数量异常,可能不是有效的 .lib 文件");
        return Ok(());
    }
    
    // 读取索引表 (每个索引4字节: offset(4))
    println!("前 20 张图像信息:");
    println!("─────────────────────────────────────────────────────────────");
    println!("索引  | 偏移量      | 宽x高");
    println!("─────────────────────────────────────────────────────────────");
    
    let max_display = image_count.min(20);
    
    for i in 0..max_display {
        // 读取索引信息 (从字节12开始,每个索引4字节)
        file.seek(SeekFrom::Start(12 + (i as u64) * 4))?;
        
        let mut offset_buf = [0u8; 4];
        file.read_exact(&mut offset_buf)?;
        let offset = i32::from_le_bytes(offset_buf);
        
        // 尝试读取图像头部获取宽高
        if offset > 0 && (offset as u64) < file_size {
            file.seek(SeekFrom::Start(offset as u64))?;
            
            let mut width_buf = [0u8; 2];
            let mut height_buf = [0u8; 2];
            
            if file.read_exact(&mut width_buf).is_ok() && file.read_exact(&mut height_buf).is_ok() {
                let width = i16::from_le_bytes(width_buf);
                let height = i16::from_le_bytes(height_buf);
                
                println!("{:4}  | {:10}  | {}x{}", 
                    i, offset, width, height);
            } else {
                println!("{:4}  | {:10}  | (无法读取)", 
                    i, offset);
            }
        } else {
            println!("{:4}  | {:10}  | (空)", 
                i, offset);
        }
    }
    
    if image_count > 20 {
        println!("...");
        println!("(还有 {} 张图像未显示,共 {} 张)", image_count - 20, image_count);
    }
    
    println!("─────────────────────────────────────────────────────────────");
    
    // 寻找大图像 (可能是背景)
    println!("\n🔍 寻找大图像 (可能的背景, 宽度 >= 800):");
    println!("─────────────────────────────────────────────────────────────");
    
    for i in 0..image_count.min(500) {
        file.seek(SeekFrom::Start(12 + (i as u64) * 4))?;
        
        let mut offset_buf = [0u8; 4];
        file.read_exact(&mut offset_buf)?;
        let offset = i32::from_le_bytes(offset_buf);
        
        if offset > 0 && (offset as u64) < file_size {
            file.seek(SeekFrom::Start(offset as u64))?;
            
            let mut width_buf = [0u8; 2];
            let mut height_buf = [0u8; 2];
            
            if file.read_exact(&mut width_buf).is_ok() && file.read_exact(&mut height_buf).is_ok() {
                let width = i16::from_le_bytes(width_buf);
                let height = i16::from_le_bytes(height_buf);
                
                if width >= 800 {
                    println!("  索引 {:4}: {}x{} - 可能是背景或大图", i, width, height);
                } else if width >= 200 && height >= 100 {
                    println!("  索引 {:4}: {}x{} - 可能是对话框", i, width, height);
                }
            }
        }
    }
    
    Ok(())
}
