// 测试纹理加载 - 检查前几个瓦片的数据
use mir2_client::graphics::mlibrary::MLibrary;

fn main() -> std::io::Result<()> {
    println!("=== 测试纹理加载 ===\n");
    
    // 加载 Tiles 库
    let mut lib = MLibrary::open("Data/Map/WemadeMir2/Tiles")?;
    println!("✅ Tiles 库加载成功");
    
    // 测试前 5 个图像
    for i in 0..5 {
        println!("\n--- 图像 {} ---", i);
        
        match lib.get_image_info(i) {
            Ok(info) => {
                println!("  尺寸: {}x{}", info.width, info.height);
                println!("  偏移: ({}, {})", info.x, info.y);
                println!("  Shadow: ({}, {}), value={}", info.shadow_x, info.shadow_y, info.shadow);
                println!("  压缩长度: {}", info.length);
                println!("  有Mask: {}", info.has_mask);
                
                // 加载实际数据
                match lib.load_rgba_data(i) {
                    Ok((_, rgba_data)) => {
                        let expected_size = (info.width as usize) * (info.height as usize) * 4;
                        println!("  RGBA数据长度: {} (期望: {})", rgba_data.len(), expected_size);
                        
                        // 检查前几个像素
                        println!("  前5个像素 (RGBA):");
                        for j in 0..5.min(rgba_data.len() / 4) {
                            let idx = j * 4;
                            let r = rgba_data[idx];
                            let g = rgba_data[idx + 1];
                            let b = rgba_data[idx + 2];
                            let a = rgba_data[idx + 3];
                            println!("    像素{}: R={:3} G={:3} B={:3} A={:3}", j, r, g, b, a);
                        }
                        
                        // 统计颜色分布
                        let mut transparent_count = 0;
                        let mut black_count = 0;
                        let mut colored_count = 0;
                        
                        for chunk in rgba_data.chunks_exact(4) {
                            let r = chunk[0];
                            let g = chunk[1];
                            let b = chunk[2];
                            let a = chunk[3];
                            
                            if a == 0 {
                                transparent_count += 1;
                            } else if r == 0 && g == 0 && b == 0 {
                                black_count += 1;
                            } else {
                                colored_count += 1;
                            }
                        }
                        
                        let total = rgba_data.len() / 4;
                        println!("  像素统计:");
                        println!("    透明: {} ({:.1}%)", transparent_count, 100.0 * transparent_count as f32 / total as f32);
                        println!("    黑色: {} ({:.1}%)", black_count, 100.0 * black_count as f32 / total as f32);
                        println!("    彩色: {} ({:.1}%)", colored_count, 100.0 * colored_count as f32 / total as f32);
                    }
                    Err(e) => {
                        println!("  ❌ 加载数据失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ❌ 获取信息失败: {}", e);
            }
        }
    }
    
    Ok(())
}
