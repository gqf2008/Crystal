// 统计 .Lib 文件中的图像信息
// 用于检查库文件中有多少有效图像和空图像

use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct LibraryHeader {
    version: i32,
    count: i32,
    frame_seek: i32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct ImageIndex {
    offset: i32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct ImageMetadata {
    width: i16,
    height: i16,
    offset_x: i16,
    offset_y: i16,
    shadow_x: u8,
    shadow_y: u8,
    shadow: u8,
    data_length: i32,
}

fn read_library_header<R: Read>(reader: &mut R) -> std::io::Result<LibraryHeader> {
    let mut buffer = [0u8; 12];
    reader.read_exact(&mut buffer)?;
    
    Ok(LibraryHeader {
        version: i32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]),
        count: i32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]),
        frame_seek: i32::from_le_bytes([buffer[8], buffer[9], buffer[10], buffer[11]]),
    })
}

fn read_image_index<R: Read>(reader: &mut R) -> std::io::Result<ImageIndex> {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer)?;
    
    Ok(ImageIndex {
        offset: i32::from_le_bytes(buffer),
    })
}

fn read_image_metadata<R: Read>(reader: &mut R) -> std::io::Result<ImageMetadata> {
    let mut buffer = [0u8; 17];
    reader.read_exact(&mut buffer)?;
    
    Ok(ImageMetadata {
        width: i16::from_le_bytes([buffer[0], buffer[1]]),
        height: i16::from_le_bytes([buffer[2], buffer[3]]),
        offset_x: i16::from_le_bytes([buffer[4], buffer[5]]),
        offset_y: i16::from_le_bytes([buffer[6], buffer[7]]),
        shadow_x: buffer[8],
        shadow_y: buffer[9],
        shadow: buffer[10],
        data_length: i32::from_le_bytes([buffer[11], buffer[12], buffer[13], buffer[14]]),
    })
}

fn main() -> std::io::Result<()> {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();
    
    let lib_path = if args.len() > 1 {
        &args[1]
    } else {
        "Data/Map/ShandaMir2/Tiles5.Lib"
    };
    
    println!("📚 分析库文件: {}", lib_path);
    println!("{}", "─".repeat(80));
    
    if !Path::new(lib_path).exists() {
        eprintln!("❌ 文件不存在: {}", lib_path);
        std::process::exit(1);
    }
    
    // 打开文件
    let file = File::open(lib_path)?;
    let mut reader = BufReader::new(file);
    
    // 读取文件头
    let header = read_library_header(&mut reader)?;
    let count = header.count;
    let version = header.version;
    println!("📊 库信息:");
    println!("   版本: {}", version);
    println!("   总图像数: {}", count);
    println!();
    
    // 读取所有索引
    let mut indices = Vec::with_capacity(count as usize);
    for _ in 0..count {
        indices.push(read_image_index(&mut reader)?);
    }
    
    // 统计数据
    let mut valid_count = 0;
    let mut zero_dimension_count = 0;
    let mut zero_data_length_count = 0;
    let mut invalid_offset_count = 0;
    let mut zero_images = Vec::new();
    
    println!("🔍 扫描所有图像...");
    
    for (index, img_index) in indices.iter().enumerate() {
        let offset = img_index.offset;
        if offset == 0 {
            invalid_offset_count += 1;
            continue;
        }
        
        // 定位到图像元数据
        match reader.seek(SeekFrom::Start(offset as u64)) {
            Ok(_) => {},
            Err(_) => {
                invalid_offset_count += 1;
                continue;
            }
        }
        
        // 读取元数据
        let metadata = match read_image_metadata(&mut reader) {
            Ok(m) => m,
            Err(_) => {
                invalid_offset_count += 1;
                continue;
            }
        };
        
        // 检查图像是否有效
        let width = metadata.width;
        let height = metadata.height;
        let data_length = metadata.data_length;
        
        if width == 0 || height == 0 {
            zero_dimension_count += 1;
            zero_images.push((index, metadata));
        } else if data_length == 0 {
            zero_data_length_count += 1;
            zero_images.push((index, metadata));
        } else {
            valid_count += 1;
        }
    }
    
    println!();
    println!("{}", "─".repeat(80));
    println!("📈 统计结果:");
    
    println!("   ✅ 有效图像:          {:>6} ({:>6.2}%)", 
        valid_count, 
        valid_count as f64 / count as f64 * 100.0
    );
    println!("   ❌ 零尺寸图像:        {:>6} ({:>6.2}%)", 
        zero_dimension_count, 
        zero_dimension_count as f64 / count as f64 * 100.0
    );
    println!("   ⚠️  零数据长度图像:   {:>6} ({:>6.2}%)", 
        zero_data_length_count, 
        zero_data_length_count as f64 / count as f64 * 100.0
    );
    println!("   🚫 无效偏移量:        {:>6} ({:>6.2}%)", 
        invalid_offset_count, 
        invalid_offset_count as f64 / count as f64 * 100.0
    );
    println!();
    println!("   总计:                {:>6}", count);
    println!("{}", "─".repeat(80));
    
    // 显示一些零尺寸图像的详细信息
    if !zero_images.is_empty() {
        println!();
        println!("🔎 零尺寸/零数据图像详情 (前20个):");
        println!();
        println!("   {:>6}  {:>6}  {:>6}  {:>10}", "索引", "宽度", "高度", "数据长度");
        println!("   {:-<6}  {:-<6}  {:-<6}  {:-<10}", "", "", "", "");
        
        for (index, metadata) in zero_images.iter().take(20) {
            let width = metadata.width;
            let height = metadata.height;
            let data_length = metadata.data_length;
            println!("   {:>6}  {:>6}  {:>6}  {:>10}", 
                index, 
                width, 
                height, 
                data_length
            );
        }
        
        if zero_images.len() > 20 {
            println!("   ... 还有 {} 个", zero_images.len() - 20);
        }
    }
    
    // 生成所有零图像的索引列表
    if !zero_images.is_empty() {
        println!();
        println!("📋 所有零尺寸/零数据图像索引:");
        print!("   ");
        for (i, (index, _)) in zero_images.iter().enumerate() {
            if i > 0 && i % 10 == 0 {
                println!();
                print!("   ");
            }
            print!("{:>6}, ", index);
        }
        println!();
    }
    
    println!();
    println!("✨ 分析完成!");
    
    Ok(())
}
