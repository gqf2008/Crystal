// tests/test_game_libraries.rs
//
// 测试游戏内容数组库的初始化和加载

#[cfg(test)]
mod tests {
    use mir2_client::graphics::{
        initialize_all_libraries, 
        LibraryArray, 
        get_library_from_array,
        LIBRARIES
    };
    
    #[test]
    fn test_initialize_all_libraries() {
        // 初始化所有库
        let result = initialize_all_libraries("./Data");
        
        // 应该成功（即使文件不存在也不应该失败）
        assert!(result.is_ok(), "初始化失败: {:?}", result.err());
    }
    
    #[test]
    fn test_array_library_stats() {
        // 初始化
        let _ = initialize_all_libraries("./Data");
        
        let libs = LIBRARIES.lock().unwrap();
        
        // 检查 MapLibs
        let maplibs_size = libs.get_array_size(LibraryArray::MapLibs);
        let maplibs_loaded = libs.get_array_loaded_count(LibraryArray::MapLibs);
        
        println!("MapLibs: {}/{} 已加载", maplibs_loaded, maplibs_size);
        assert_eq!(maplibs_size, 400, "MapLibs 应该初始化为 400 个槽位");
        
        // 检查 Monsters
        let monsters_size = libs.get_array_size(LibraryArray::Monsters);
        let monsters_loaded = libs.get_array_loaded_count(LibraryArray::Monsters);
        
        println!("Monsters: {}/{} 已加载", monsters_loaded, monsters_size);
        
        // 检查 NPCs
        let npcs_size = libs.get_array_size(LibraryArray::NPCs);
        let npcs_loaded = libs.get_array_loaded_count(LibraryArray::NPCs);
        
        println!("NPCs: {}/{} 已加载", npcs_loaded, npcs_size);
        
        // 检查 CArmours
        let carmours_size = libs.get_array_size(LibraryArray::CArmours);
        let carmours_loaded = libs.get_array_loaded_count(LibraryArray::CArmours);
        
        println!("CArmours: {}/{} 已加载", carmours_loaded, carmours_size);
    }
    
    #[test]
    fn test_get_library_from_array() {
        let _ = initialize_all_libraries("./Data");
        
        // 尝试获取 MapLibs[0]
        match get_library_from_array(LibraryArray::MapLibs, 0) {
            Some(lib) => {
                println!("✓ MapLibs[0] 存在");
                let library = lib.lock().unwrap();
                println!("  图像数量: {}", library.count());
            }
            None => {
                println!("✗ MapLibs[0] 不存在（正常 - 文件可能不存在）");
            }
        }
        
        // 尝试获取 Monsters[0]
        match get_library_from_array(LibraryArray::Monsters, 0) {
            Some(lib) => {
                println!("✓ Monsters[0] 存在");
                let library = lib.lock().unwrap();
                println!("  图像数量: {}", library.count());
            }
            None => {
                println!("✗ Monsters[0] 不存在（正常 - 文件可能不存在）");
            }
        }
    }
    
    #[test]
    fn test_all_array_types() {
        let _ = initialize_all_libraries("./Data");
        
        let libs = LIBRARIES.lock().unwrap();
        
        let array_types = [
            LibraryArray::MapLibs,
            LibraryArray::Monsters,
            LibraryArray::Gates,
            LibraryArray::NPCs,
            LibraryArray::CArmours,
            LibraryArray::CWeapons,
            LibraryArray::CHair,
            LibraryArray::AArmours,
            LibraryArray::AWeaponsL,
            LibraryArray::AWeaponsR,
            LibraryArray::AHair,
            LibraryArray::ARArmours,
            LibraryArray::ARWeapons,
            LibraryArray::ARHair,
            LibraryArray::Mounts,
            LibraryArray::Fishing,
            LibraryArray::Pets,
            LibraryArray::Transform,
            LibraryArray::TransformMounts,
            LibraryArray::TransformEffect,
            LibraryArray::TransformWeaponEffect,
            LibraryArray::MArmours,
            LibraryArray::MWeapons,
            LibraryArray::Title,
            LibraryArray::Deco,
            LibraryArray::Wings,
        ];
        
        println!("\n=== 数组库统计 ===");
        for array_type in &array_types {
            let size = libs.get_array_size(*array_type);
            let loaded = libs.get_array_loaded_count(*array_type);
            
            if size > 0 {
                println!("{:20} : {:4}/{:4} 已加载 ({:.1}%)", 
                    array_type.name(), 
                    loaded, 
                    size,
                    if size > 0 { (loaded as f32 / size as f32 * 100.0) } else { 0.0 }
                );
            }
        }
    }
}
