// 测试 macroquad LibraryManager
// 验证库加载和纹理创建功能

use macroquad::prelude::*;
use std::path::PathBuf;
use mir2_client::backends::macroquad::LibraryManager;

#[macroquad::main("LibraryManager Test")]
async fn main() {
    // 1. 创建 LibraryManager
    let data_path = PathBuf::from("/Users/gqf/Documents/GitHub/Crystal/ClientRust/Data");
    let lib_manager = LibraryManager::new(data_path);
    
    println!("📦 测试 LibraryManager (macroquad 版本)");
    println!("{}", "=".repeat(60));
    
    // 2. 测试加载多个库
    let libraries_to_test = vec![
        ("MapLib_0", "Map/WemadeMir2/Tiles.Lib"),
        ("MapLib_1", "Map/WemadeMir2/SmTiles.Lib"),
        ("MapLib_104", "Map/ShandaMir2/Tiles4.Lib"),
    ];
    
    for (lib_name, lib_path) in &libraries_to_test {
        match lib_manager.load_library(lib_name, lib_path) {
            Ok(_) => {
                let count = lib_manager.get_library_count(lib_name).unwrap_or(0);
                println!("✅ {} 加载成功 ({} 张图像)", lib_name, count);
            }
            Err(e) => {
                println!("❌ {} 加载失败: {}", lib_name, e);
            }
        }
    }
    
    println!();
    println!("🎨 测试纹理创建");
    println!("{}", "=".repeat(60));
    
    // 3. 测试获取纹理
    let test_images = vec![
        ("MapLib_0", 0),
        ("MapLib_0", 100),
        ("MapLib_104", 6000),
        ("MapLib_104", 6004),
    ];
    
    let mut textures_loaded = Vec::new();
    
    for (lib_name, image_index) in &test_images {
        match lib_manager.get_or_create_texture(lib_name, *image_index) {
            Some(texture) => {
                println!("✅ {} [{}]: {}x{} 像素", 
                    lib_name, image_index, texture.width(), texture.height());
                textures_loaded.push((lib_name, image_index, texture));
            }
            None => {
                println!("❌ {} [{}]: 获取失败", lib_name, image_index);
            }
        }
    }
    
    println!();
    println!("🖼️  渲染测试");
    println!("{}", "=".repeat(60));
    
    // 4. 渲染循环 - 显示加载的纹理
    let mut frame_count = 0;
    loop {
        clear_background(BLACK);
        
        // 显示标题
        draw_text("LibraryManager Test - Macroquad Backend", 10.0, 30.0, 24.0, WHITE);
        draw_text(&format!("Frame: {}", frame_count), 10.0, 60.0, 20.0, GRAY);
        
        // 显示加载的纹理（横向排列）
        let mut x = 10.0;
        let y = 100.0;
        
        for (lib_name, image_index, texture) in &textures_loaded {
            // 绘制纹理
            draw_texture(texture, x, y, WHITE);
            
            // 绘制标签
            let label = format!("{} [{}]", lib_name, image_index);
            draw_text(&label, x, y + texture.height() + 20.0, 16.0, YELLOW);
            
            // 绘制尺寸信息
            let size_info = format!("{}x{}", texture.width(), texture.height());
            draw_text(&size_info, x, y + texture.height() + 40.0, 14.0, GREEN);
            
            // 下一个位置
            x += texture.width() + 20.0;
        }
        
        // 显示说明
        draw_text("Press ESC to exit", 10.0, screen_height() - 20.0, 18.0, LIGHTGRAY);
        
        // 退出检查
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        frame_count += 1;
        next_frame().await;
    }
    
    println!();
    println!("✅ 测试完成");
}
