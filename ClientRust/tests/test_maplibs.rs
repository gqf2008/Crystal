// 测试 MapLibs 初始化
// 运行: cargo test --package client-rust --lib graphics::libraries::tests::test_maplibs_init -- --nocapture

#[cfg(test)]
mod tests {
    use crate::graphics::libraries::{Libraries, LibraryArray};
    
    #[test]
    fn test_maplibs_array_init() {
        let mut libs = Libraries::new();
        
        // 初始化数组
        libs.init_array(LibraryArray::MapLibs, 400);
        
        // 验证数组大小
        assert_eq!(libs.get_array_size(LibraryArray::MapLibs), 400);
        
        // 初始时没有库加载
        assert_eq!(libs.get_array_loaded_count(LibraryArray::MapLibs), 0);
        
        println!("✓ MapLibs 数组初始化成功");
    }
    
    #[test]
    fn test_load_to_array() {
        let mut libs = Libraries::new();
        libs.set_data_path("../Data");
        libs.init_array(LibraryArray::MapLibs, 400);
        
        // 尝试加载 Tiles (可能不存在,不应该panic)
        let result = libs.load_to_array(
            LibraryArray::MapLibs, 
            0, 
            "../Data/Map/WemadeMir2/Tiles"
        );
        
        // 即使文件不存在也应该返回 Ok (内部只是记录警告)
        assert!(result.is_ok());
        
        println!("✓ load_to_array 测试通过");
    }
    
    #[test]
    fn test_get_from_array() {
        let mut libs = Libraries::new();
        libs.init_array(LibraryArray::MapLibs, 400);
        
        // 索引0应该返回 None (未加载)
        assert!(libs.get_from_array(LibraryArray::MapLibs, 0).is_none());
        
        // 超出范围应该返回 None
        assert!(libs.get_from_array(LibraryArray::MapLibs, 500).is_none());
        
        println!("✓ get_from_array 测试通过");
    }
    
    #[test]
    #[ignore] // 需要真实的 Data 目录,手动测试时取消 ignore
    fn test_init_map_libraries() {
        let mut libs = Libraries::new();
        libs.set_data_path("../Data");
        
        // 初始化所有 MapLibs
        let result = libs.init_map_libraries();
        assert!(result.is_ok());
        
        let loaded = libs.get_array_loaded_count(LibraryArray::MapLibs);
        println!("✓ MapLibs 初始化完成: {}/400 个库已加载", loaded);
        
        // 至少应该加载一些库
        assert!(loaded > 0, "应该至少加载一些地图库");
    }
}
