// tests/test_map_rendering.rs
//
// 测试地图瓦片渲染功能

use mir2_client::graphics::{
    initialize_all_libraries,
    get_map_library,
};

#[test]
fn test_map_library_loading() {
        // 初始化库
        let result = initialize_all_libraries("./Data");
        assert!(result.is_ok(), "库初始化失败");
        
        println!("\n=== 测试地图库加载 ===\n");
        
        // 测试 WemadeMir2 地图库
        println!("📦 测试 WemadeMir2 地图库:");
        for i in 0..10 {
            if let Some(map_lib) = get_map_library(i) {
                let mut lib = map_lib.lock().unwrap();
                println!("  ✓ MapLibs[{}]: {} 张图像", i, lib.count());
                
                // 测试加载第一张图像
                if lib.count() > 0 {
                    match lib.load_rgba_data(0) {
                        Ok((info, rgba_data)) => {
                            println!("    - 第0张图像: {}x{}, {} 字节", 
                                info.width, info.height, rgba_data.len());
                        }
                        Err(e) => {
                            println!("    ✗ 加载失败: {}", e);
                        }
                    }
                }
            } else {
                println!("  ✗ MapLibs[{}]: 未加载", i);
            }
        }
        
        // 测试 ShandaMir2 地图库
        println!("\n📦 测试 ShandaMir2 地图库:");
        for i in 100..110 {
            if let Some(map_lib) = get_map_library(i) {
                let lib = map_lib.lock().unwrap();
                println!("  ✓ MapLibs[{}]: {} 张图像", i, lib.count());
            } else {
                println!("  ✗ MapLibs[{}]: 未加载", i);
            }
        }
        
        // 测试 WemadeMir3 地图库
        println!("\n📦 测试 WemadeMir3 地图库:");
        for i in 200..210 {
            if let Some(map_lib) = get_map_library(i) {
                let lib = map_lib.lock().unwrap();
                println!("  ✓ MapLibs[{}]: {} 张图像", i, lib.count());
            } else {
                println!("  ✗ MapLibs[{}]: 未加载", i);
            }
        }
    }
    
    #[test]
    fn test_tile_image_loading() {
        // 初始化库
        let _ = initialize_all_libraries("./Data");
        
        println!("\n=== 测试瓦片图像加载 ===\n");
        
        // 测试加载 Tiles (MapLibs[0])
        if let Some(tiles_lib) = get_map_library(0) {
            let mut lib = tiles_lib.lock().unwrap();
            println!("📊 Tiles 库统计:");
            println!("  - 总图像数: {}", lib.count());
            
            // 测试加载前10张图像
            println!("\n🖼️  测试加载前10张图像:");
            for i in 0..10.min(lib.count()) {
                match lib.load_rgba_data(i) {
                    Ok((info, rgba_data)) => {
                        println!("  ✓ [{}] 尺寸: {}x{}, 数据: {} 字节, 偏移: ({}, {})",
                            i, info.width, info.height, rgba_data.len(),
                            info.x, info.y
                        );
                        
                        // 验证数据
                        let expected_size = (info.width * info.height * 4) as usize;
                        assert_eq!(rgba_data.len(), expected_size,
                            "图像 {} 数据大小不匹配", i);
                    }
                    Err(e) => {
                        println!("  ✗ [{}] 加载失败: {}", i, e);
                    }
                }
            }
        } else {
            println!("❌ Tiles 库未加载");
        }
        
        // 测试加载 SmTiles (MapLibs[1])
        if let Some(smtiles_lib) = get_map_library(1) {
            let mut lib = smtiles_lib.lock().unwrap();
            println!("\n📊 SmTiles 库统计:");
            println!("  - 总图像数: {}", lib.count());
            
            // 测试加载前5张图像
            println!("\n🖼️  测试加载前5张图像:");
            for i in 0..5.min(lib.count()) {
                match lib.load_rgba_data(i) {
                    Ok((info, _)) => {
                        println!("  ✓ [{}] 尺寸: {}x{}, 偏移: ({}, {})",
                            i, info.width, info.height, info.x, info.y
                        );
                    }
                    Err(e) => {
                        println!("  ✗ [{}] 加载失败: {}", i, e);
                    }
                }
            }
        }
    }
    
    #[test]
    fn test_map_objects_loading() {
        let _ = initialize_all_libraries("./Data");
        
        println!("\n=== 测试地图对象库加载 ===\n");
        
        // 测试 Objects (MapLibs[2])
        if let Some(objects_lib) = get_map_library(2) {
            let mut lib = objects_lib.lock().unwrap();
            println!("📊 Objects 库统计:");
            println!("  - 总图像数: {}", lib.count());
            
            // 测试加载一些随机图像
            let test_indices = [0, 100, 500, 1000, 5000];
            println!("\n🖼️  测试加载图像:");
            for &i in &test_indices {
                if i >= lib.count() {
                    continue;
                }
                
                match lib.load_rgba_data(i) {
                    Ok((info, rgba_data)) => {
                        println!("  ✓ [{}] 尺寸: {}x{}, 数据: {} 字节",
                            i, info.width, info.height, rgba_data.len()
                        );
                        
                        // 检查是否有透明像素
                        let has_transparency = rgba_data.chunks(4)
                            .any(|pixel| pixel[3] == 0);
                        println!("      透明度: {}", 
                            if has_transparency { "有" } else { "无" }
                        );
                    }
                    Err(e) => {
                        println!("  ✗ [{}] 加载失败: {}", i, e);
                    }
                }
            }
        }
    }
