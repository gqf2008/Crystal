use mir2_client::objects::map_code::MapReader;
use std::path::Path;

fn main() {
    println!("=== 瓦片数据验证工具 ===\n");
    
    // 加载地图
    let map_path = Path::new("/Users/gqf/Documents/GitHub/Crystal/ClientRust/Map/n0.map");
    let map_reader = MapReader::new(map_path.to_str().unwrap())
        .expect("Failed to load map");
    
    println!("✅ 地图加载成功: {}x{}\n", map_reader.width, map_reader.height);
    
    // 测试坐标 (68, 0)
    let test_x = 68;
    let test_y = 0;
    
    println!("📍 检查坐标 ({}, {}) 是否被绘制两次", test_x, test_y);
    println!("{}", "=".repeat(80));
    
    if let Some(cell) = map_reader.get_cell(test_x, test_y) {
        println!("\n【原始Cell数据 - 坐标 ({}, {})】", test_x, test_y);
        println!("  back_index: {}", cell.back_index);
        println!("  back_image: {} (0x{:X})", cell.back_image, cell.back_image);
        
        if let Some((file_idx, img_idx)) = cell.back_tile() {
            println!("\n【Back层计算结果】");
            println!("  文件索引: {}", file_idx);
            println!("  图像索引: {}", img_idx);
            println!("  原始back_image: 0x{:X}", cell.back_image);
            
            // 检查是否应该跳过渲染
            println!("\n【渲染检查】");
            println!("  坐标 ({}, {}): x%2={}, y%2={}", test_x, test_y, test_x % 2, test_y % 2);
            
            if test_x % 2 != 0 || test_y % 2 != 0 {
                println!("  ⚠️  这是奇数坐标，如果是大瓦片(96x64)应该被跳过");
            } else {
                println!("  ✅ 这是偶数坐标，会被正常渲染");
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("\n【检查周围2x2区域】");
    println!("如果 (68,0) 是大瓦片的一部分，应该只有 (68,0) 被渲染\n");
    
    for dy in 0..2 {
        for dx in 0..2 {
            let x = test_x + dx;
            let y = test_y + dy;
            
            if let Some(cell) = map_reader.get_cell(x, y) {
                if let Some((file_idx, img_idx)) = cell.back_tile() {
                    let should_skip = (x % 2 != 0 || y % 2 != 0);
                    let status = if should_skip {
                        "⏭️  SKIP (奇数坐标)"
                    } else {
                        "✅ RENDER"
                    };
                    
                    println!("  ({:3},{:3}): file={:3}, image={:5}, back_image=0x{:X} - {}",
                        x, y, file_idx, img_idx, cell.back_image, status);
                }
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("\n【检查周围4x4区域 (66-71, 0-3)】");
    println!("显示哪些坐标会被渲染，哪些会被跳过\n");
    
    for y in 0..4 {
        for x in 66..72 {
            if let Some(cell) = map_reader.get_cell(x, y) {
                if let Some((file_idx, img_idx)) = cell.back_tile() {
                    let is_even = (x % 2 == 0) && (y % 2 == 0);
                    let status = if is_even { "✅ RENDER" } else { "⏭️  SKIP" };
                    
                    println!("  ({:3},{:3}): file={:3}, image={:5}, 0x{:08X} - {}",
                        x, y, file_idx, img_idx, cell.back_image, status);
                }
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("\n【分析可能的重复绘制原因】");
    
    // 检查 (68,0) 和相邻格子的back_image是否相同
    if let Some(cell_68_0) = map_reader.get_cell(68, 0) {
        let back_68_0 = cell_68_0.back_image;
        
        println!("\n坐标 (68,0) 的 back_image = 0x{:X}", back_68_0);
        
        let neighbors = [
            (66, 0), (67, 0), (69, 0), (70, 0),  // 左右邻居
            (68, 1), (66, 1), (67, 1), (69, 1), (70, 1),  // 下方邻居
        ];
        
        for (nx, ny) in neighbors.iter() {
            if let Some(neighbor_cell) = map_reader.get_cell(*nx, *ny) {
                if neighbor_cell.back_image == back_68_0 {
                    println!("  ⚠️  坐标 ({},{}) 的 back_image 也是 0x{:X} (相同!)",
                        nx, ny, neighbor_cell.back_image);
                }
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("\n【渲染逻辑说明】");
    println!("当前实现:");
    println!("1. 遍历所有格子");
    println!("2. 对于每个格子，读取其 back_image");
    println!("3. 如果纹理是 96x64 大瓦片:");
    println!("   - 只在偶数坐标 (x%2==0 && y%2==0) 绘制");
    println!("   - 奇数坐标跳过，避免重复绘制");
    println!("\n如果 (68,0) 被绘制两次，可能原因:");
    println!("1. 周围格子的 back_image 指向同一个大瓦片");
    println!("2. 纹理尺寸判断有问题");
    println!("3. 坐标过滤逻辑有bug");
}

