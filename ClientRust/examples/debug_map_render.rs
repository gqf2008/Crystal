/// 调试地图渲染 - 检查图像索引是否正确/// 调试地图渲染 - 检查坐标、索引、图像是否正确

use mir2_client::graphics::mlibrary::MLibrary;use mir2_client::graphics::mlibrary::MLibrary;

use std::fs::File;use std::fs::File;

use std::io::{self, Read};use std::io::{self, Read};

use std::path::Path;use std::path::Path;

use std::collections::HashMap;

fn main() -> io::Result<()> {

fn main() -> io::Result<()> {    println!("=== 地图渲染调试工具 ===\n");

    println!("=== 地图渲染调试工具 ===\n");    

        // 1. 加载图像库

    // 1. 加载图像库    let mut tiles_lib = MLibrary::open("Data/WemadeMir2/Tiles.lib")?;

    let mut tiles_lib = MLibrary::open("Data/WemadeMir2/Tiles.lib")?;    println!("✅ 图像库: Tiles.lib, 图像数={}\n", tiles_lib.count());

    println!("✅ 图像库: Tiles.lib, 图像数={}\n", tiles_lib.count());    

        // 2. 读取地图文件的原始数据

    // 2. 读取地图文件    let map_path = Path::new("Map/0.map");

    let map_path = Path::new("Map/0.map");    let mut file = File::open(map_path)?;

    let mut file = File::open(map_path)?;    let mut buffer = Vec::new();

    let mut buffer = Vec::new();    file.read_to_end(&mut buffer)?;

    file.read_to_end(&mut buffer)?;    

    println!("✅ 地图文件: {} bytes\n", buffer.len());    println!("✅ 地图文件: {} bytes\n", buffer.len());

        

    // 3. 解析地图头部    // 3. 解析地图头部 (Type 100 format)

    if buffer.len() < 8 {    if buffer.len() < 8 {

        return Err(io::Error::new(io::ErrorKind::InvalidData, "文件太小"));        return Err(io::Error::new(io::ErrorKind::InvalidData, "文件太小"));

    }    }

        

    let width = u16::from_le_bytes([buffer[0], buffer[1]]) as i32;    let width = u16::from_le_bytes([buffer[0], buffer[1]]) as i32;

    let height = u16::from_le_bytes([buffer[2], buffer[3]]) as i32;    let height = u16::from_le_bytes([buffer[2], buffer[3]]) as i32;

    println!("地图尺寸: {}x{}\n", width, height);    println!("地图尺寸: {}x{}\n", width, height);

        

    let cell_data_start = 8;    // 4. 计算格子数据起始位置

    let bytes_per_cell = 32;    let cell_data_start = 8;  // 跳过头部

        let bytes_per_cell = 32;  // Type 100 格式每格32字节

    // 4. 检查前5x5格子    

    println!("=== 前5x5格子的MiddleImage ===");    // 5. 检查前5x5格子的MiddleImage

    for y in 0..5 {    println!("=== 前5x5格子的MiddleImage ===");

        for x in 0..5 {    for y in 0..5 {

            let cell_index = (y * width + x) as usize;        for x in 0..5 {

            let offset = cell_data_start + cell_index * bytes_per_cell;            let cell_index = (y * width + x) as usize;

                        let offset = cell_data_start + cell_index * bytes_per_cell;

            if offset + bytes_per_cell > buffer.len() {            

                println!("({}, {}): 超出文件范围", x, y);            if offset + bytes_per_cell > buffer.len() {

                continue;                println!("({}, {}): 超出文件范围", x, y);

            }                continue;

                        }

            let raw_mid = u32::from_le_bytes([            

                buffer[offset + 2],            // Type 100 格式:

                buffer[offset + 3],            // offset+0:  BackImage (2 bytes) - 跳过

                buffer[offset + 4],            // offset+2:  MiddleImage (4 bytes)

                buffer[offset + 5],            let raw_mid = u32::from_le_bytes([

            ]);                buffer[offset + 2],

                            buffer[offset + 3],

            let masked_mid = raw_mid & 0x1FFFFFFF;                buffer[offset + 4],

            let index = masked_mid.saturating_sub(1) as usize;                buffer[offset + 5],

                        ]);

            if masked_mid > 0 {            

                match tiles_lib.get_image_info(index) {            let masked_mid = raw_mid & 0x1FFFFFFF;

                    Ok(info) => {            let index = masked_mid.saturating_sub(1) as usize;

                        println!("({}, {}): Raw={} ({:#X}), Masked={}, Index={}, 图像={}x{}, 偏移=({},{})",            

                            x, y, raw_mid, raw_mid, masked_mid, index,            // 检查图像是否存在

                            info.width, info.height, info.x, info.y);            if masked_mid > 0 {

                    }                let raw_mid = cell.middle_image;

                    Err(e) => {                let masked_mid = raw_mid & 0x1FFFFFFF;

                        println!("({}, {}): Raw={} ({:#X}), Masked={}, Index={} ❌ 图像不存在: {}",                let index = masked_mid.saturating_sub(1) as usize;

                            x, y, raw_mid, raw_mid, masked_mid, index, e);                

                    }                // 检查图像是否存在

                }                if masked_mid > 0 {

            } else {                    match tiles_lib.get_image_info(index) {

                println!("({}, {}): 空白", x, y);                        Ok(info) => {

            }                            println!("({}, {}): Raw={} ({:#X}), Masked={}, Index={}, 图像={}x{}, 偏移=({},{})",

        }                                x, y, raw_mid, raw_mid, masked_mid, index,

        println!();                                info.width, info.height, info.offset_x, info.offset_y);

    }                        }

                            Err(e) => {

    // 5. 统计不同的MiddleImage值 (前100x100)                            println!("({}, {}): Raw={} ({:#X}), Masked={}, Index={} ❌ 图像不存在: {}",

    println!("\n=== MiddleImage统计 (前100x100) ===");                                x, y, raw_mid, raw_mid, masked_mid, index, e);

    let mut stats: HashMap<u32, usize> = HashMap::new();                        }

                        }

    for y in 0..100 {                } else {

        for x in 0..100 {                    println!("({}, {}): 空白", x, y);

            let cell_index = (y * width + x) as usize;                }

            let offset = cell_data_start + cell_index * bytes_per_cell;            }

                    }

            if offset + bytes_per_cell > buffer.len() {        println!();

                continue;    }

            }    

                // 4. 检查中心区域 (350, 350)

            let raw_mid = u32::from_le_bytes([    println!("\n=== 地图中心 (350,350) 附近 ===");

                buffer[offset + 2],    for y in 348..353 {

                buffer[offset + 3],        for x in 348..353 {

                buffer[offset + 4],            if let Some(cell) = map.get_cell(x, y) {

                buffer[offset + 5],                let masked_mid = cell.middle_image & 0x1FFFFFFF;

            ]);                let index = masked_mid.saturating_sub(1) as usize;

            let masked = raw_mid & 0x1FFFFFFF;                

            *stats.entry(masked).or_insert(0) += 1;                if masked_mid > 0 {

        }                    match tiles_lib.get_image_info(index) {

    }                        Ok(info) => {

                                println!("({}, {}): Idx={}, 图像={}x{}", x, y, index, info.width, info.height);

    let mut sorted: Vec<_> = stats.iter().collect();                        }

    sorted.sort_by_key(|(k, _)| *k);                        Err(_) => {

                                println!("({}, {}): Idx={} ❌", x, y, index);

    println!("不同的瓦片值 (前20个):");                        }

    for (value, count) in sorted.iter().take(20) {                    }

        if **value > 0 {                }

            let index = value.saturating_sub(1) as usize;            }

            match tiles_lib.get_image_info(index) {        }

                Ok(info) => {    }

                    println!("  值={}, 索引={}, 出现次数={}, 图像={}x{}", value, index, count, info.width, info.height);    

                }    // 5. 统计不同的MiddleImage值

                Err(_) => {    println!("\n=== MiddleImage统计 (前100x100) ===");

                    println!("  值={}, 索引={}, 出现次数={} ❌ 图像不存在", value, index, count);    use std::collections::HashMap;

                }    let mut stats: HashMap<u32, usize> = HashMap::new();

            }    

        }    for y in 0..100 {

    }        for x in 0..100 {

                if let Some(cell) = map.get_cell(x, y) {

    // 6. 测试加载前3个有效图像的实际数据                let masked = cell.middle_image & 0x1FFFFFFF;

    println!("\n=== 测试加载前3个有效图像 ===");                *stats.entry(masked).or_insert(0) += 1;

    let mut tested = 0;            }

    for (value, _) in sorted.iter() {        }

        if **value > 0 && tested < 3 {    }

            let index = value.saturating_sub(1) as usize;    

            match tiles_lib.load_rgba_data(index) {    let mut sorted: Vec<_> = stats.iter().collect();

                Ok((info, data)) => {    sorted.sort_by_key(|(k, _)| *k);

                    println!("✅ 索引{}: 加载成功, {}x{}, {} bytes",     

                        index, info.width, info.height, data.len());    println!("不同的瓦片值 (前20个):");

                    // 采样第一个像素    for (value, count) in sorted.iter().take(20) {

                    if data.len() >= 4 {        if **value > 0 {

                        println!("   第一个像素: R={} G={} B={} A={}",             let index = value.saturating_sub(1) as usize;

                            data[0], data[1], data[2], data[3]);            match tiles_lib.get_image_info(index) {

                    }                Ok(info) => {

                    tested += 1;                    println!("  值={}, 索引={}, 出现次数={}, 图像={}x{}", value, index, count, info.width, info.height);

                }                }

                Err(e) => {                Err(_) => {

                    println!("❌ 索引{}: 加载失败: {}", index, e);                    println!("  值={}, 索引={}, 出现次数={} ❌ 图像不存在", value, index, count);

                }                }

            }            }

        }        }

    }    }

        

    Ok(())    Ok(())

}}

