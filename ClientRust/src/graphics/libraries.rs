// src/graphics/libraries.rs
//
// 全局库管理器
// 对应 C# 的 Libraries static class
//
// C# 原版:
// ```csharp
// public static class Libraries {
//     public static readonly MLibrary Prguse = new MLibrary("Data/Prguse");
//     public static readonly MLibrary Magic = new MLibrary("Data/Magic");
//     // ... 等等
// }
// ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::path::Path;
use super::mlibrary::MLibrary;

/// 库名称枚举
/// 
/// 对应 C# Libraries 类中的所有静态字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryName {
    // UI 相关
    ChrSel,      // 角色选择界面
    Prguse,      // 主要 UI 资源
    Prguse2,     // 次要 UI 资源
    Prguse3,     // 额外 UI 资源
    BuffIcon,    // Buff 图标
    Help,        // 帮助界面
    MiniMap,     // 小地图
    MapLinkIcon, // 地图链接图标
    Title,       // 标题
    MagIcon,     // 魔法图标
    MagIcon2,    // 魔法图标2
    Background,  // 背景
    Dragon,      // 龙
    
    // 魔法效果
    Magic,       // 主要魔法效果
    Magic2,      // 魔法效果2
    Magic3,      // 魔法效果3
    Effect,      // 特效
    MagicC,      // 魔法C
    GuildSkill,  // 公会技能
    
    // 天气效果
    Weather,     // 天气/粒子效果
    
    // 物品
    Items,       // 物品
    StateItems,  // 状态物品
    FloorItems,  // 地面物品
    
    // 装饰
    Deco,        // 装饰物
    
    // 其他（可扩展）
    Custom(u32), // 自定义库（用于动态加载）
}

impl LibraryName {
    /// 获取库的默认路径（相对于 Data 目录）
    pub fn default_path(&self) -> String {
        match self {
            LibraryName::ChrSel => "ChrSel".to_string(),
            LibraryName::Prguse => "Prguse".to_string(),
            LibraryName::Prguse2 => "Prguse2".to_string(),
            LibraryName::Prguse3 => "Prguse3".to_string(),
            LibraryName::BuffIcon => "BuffIcon".to_string(),
            LibraryName::Help => "Help".to_string(),
            LibraryName::MiniMap => "MMap".to_string(),
            LibraryName::MapLinkIcon => "MapLinkIcon".to_string(),
            LibraryName::Title => "Title".to_string(),
            LibraryName::MagIcon => "MagIcon".to_string(),
            LibraryName::MagIcon2 => "MagIcon2".to_string(),
            LibraryName::Magic => "Magic".to_string(),
            LibraryName::Magic2 => "Magic2".to_string(),
            LibraryName::Magic3 => "Magic3".to_string(),
            LibraryName::Effect => "Effect".to_string(),
            LibraryName::MagicC => "MagicC".to_string(),
            LibraryName::GuildSkill => "GuildSkill".to_string(),
            LibraryName::Weather => "Weather".to_string(),
            LibraryName::Background => "Background".to_string(),
            LibraryName::Dragon => "Dragon".to_string(),
            LibraryName::Items => "Items".to_string(),
            LibraryName::StateItems => "StateItem".to_string(),
            LibraryName::FloorItems => "DNItems".to_string(),
            LibraryName::Deco => "Deco".to_string(),
            LibraryName::Custom(id) => format!("Custom{}", id),
        }
    }
    
    /// 从字符串解析库名称
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ChrSel" => Some(LibraryName::ChrSel),
            "Prguse" => Some(LibraryName::Prguse),
            "Prguse2" => Some(LibraryName::Prguse2),
            "Prguse3" => Some(LibraryName::Prguse3),
            "BuffIcon" => Some(LibraryName::BuffIcon),
            "Help" => Some(LibraryName::Help),
            "MMap" | "MiniMap" => Some(LibraryName::MiniMap),
            "MapLinkIcon" => Some(LibraryName::MapLinkIcon),
            "Title" => Some(LibraryName::Title),
            "MagIcon" => Some(LibraryName::MagIcon),
            "MagIcon2" => Some(LibraryName::MagIcon2),
            "Magic" => Some(LibraryName::Magic),
            "Magic2" => Some(LibraryName::Magic2),
            "Magic3" => Some(LibraryName::Magic3),
            "Effect" => Some(LibraryName::Effect),
            "MagicC" => Some(LibraryName::MagicC),
            "GuildSkill" => Some(LibraryName::GuildSkill),
            "Weather" => Some(LibraryName::Weather),
            "Background" => Some(LibraryName::Background),
            "Dragon" => Some(LibraryName::Dragon),
            "Items" => Some(LibraryName::Items),
            "StateItem" | "StateItems" => Some(LibraryName::StateItems),
            "DNItems" | "FloorItems" => Some(LibraryName::FloorItems),
            "Deco" => Some(LibraryName::Deco),
            _ => None,
        }
    }
}

impl std::fmt::Display for LibraryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.default_path())
    }
}

/// 全局库管理器
/// 
/// C# equivalent: Libraries static class
/// 
/// C# 使用静态类 + 静态字段，Rust 使用 Lazy 单例
pub struct Libraries {
    /// 已加载的库
    libraries: HashMap<LibraryName, Arc<Mutex<MLibrary>>>,
    
    /// 数据根目录
    data_path: String,
    
    /// 加载统计
    pub loaded: bool,
    pub count: usize,
    pub progress: usize,
}

impl Libraries {
    /// 创建新的库管理器
    fn new() -> Self {
        Self {
            libraries: HashMap::new(),
            data_path: "Data".to_string(),
            loaded: false,
            count: 0,
            progress: 0,
        }
    }
    
    /// 设置数据根目录
    pub fn set_data_path(&mut self, path: impl Into<String>) {
        self.data_path = path.into();
    }
    
    /// 加载单个库
    /// 
    /// C# equivalent: 在静态构造函数中直接 new MLibrary()
    pub fn load(&mut self, name: LibraryName) -> std::io::Result<()> {
        let path = format!("{}/{}", self.data_path, name.default_path());
        
        tracing::info!("加载库: {} ({})", name, path);
        
        match MLibrary::open(&path) {
            Ok(lib) => {
                tracing::info!("✓ 成功加载 {} ({} 张图像)", name, lib.count());
                self.libraries.insert(name, Arc::new(Mutex::new(lib)));
                self.progress += 1;
                Ok(())
            }
            Err(e) => {
                tracing::warn!("✗ 加载失败 {}: {}", name, e);
                Err(e)
            }
        }
    }
    
    /// 加载库（自定义路径）
    pub fn load_custom(&mut self, name: LibraryName, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path_str = path.as_ref().display().to_string();
        
        tracing::info!("加载库: {} (自定义路径: {})", name, path_str);
        
        match MLibrary::open(path) {
            Ok(lib) => {
                tracing::info!("✓ 成功加载 {} ({} 张图像)", name, lib.count());
                self.libraries.insert(name, Arc::new(Mutex::new(lib)));
                self.progress += 1;
                Ok(())
            }
            Err(e) => {
                tracing::warn!("✗ 加载失败 {}: {}", name, e);
                Err(e)
            }
        }
    }
    
    /// 获取库引用
    /// 
    /// C# equivalent: 直接访问 Libraries.Weather
    pub fn get(&self, name: LibraryName) -> Option<Arc<Mutex<MLibrary>>> {
        self.libraries.get(&name).cloned()
    }
    
    /// 检查库是否已加载
    pub fn is_loaded(&self, name: LibraryName) -> bool {
        self.libraries.contains_key(&name)
    }
    
    /// 卸载单个库
    pub fn unload(&mut self, name: LibraryName) {
        if self.libraries.remove(&name).is_some() {
            tracing::info!("卸载库: {}", name);
        }
    }
    
    /// 卸载所有库
    pub fn unload_all(&mut self) {
        tracing::info!("卸载所有库 ({} 个)", self.libraries.len());
        self.libraries.clear();
        self.loaded = false;
        self.progress = 0;
    }
    
    /// 获取已加载库的数量
    pub fn loaded_count(&self) -> usize {
        self.libraries.len()
    }
}

/// 全局库管理器单例
/// 
/// C# equivalent: Libraries static class
pub static LIBRARIES: Lazy<Mutex<Libraries>> = Lazy::new(|| {
    Mutex::new(Libraries::new())
});

/// 便捷函数: 初始化数据路径
pub fn set_data_path(path: impl Into<String>) {
    LIBRARIES.lock().unwrap().set_data_path(path);
}

/// 便捷函数: 加载库
pub fn load_library(name: LibraryName) -> std::io::Result<()> {
    LIBRARIES.lock().unwrap().load(name)
}

/// 便捷函数: 加载库（自定义路径）
pub fn load_library_custom(name: LibraryName, path: impl AsRef<Path>) -> std::io::Result<()> {
    LIBRARIES.lock().unwrap().load_custom(name, path)
}

/// 便捷函数: 获取库
pub fn get_library(name: LibraryName) -> Option<Arc<Mutex<MLibrary>>> {
    LIBRARIES.lock().unwrap().get(name)
}

/// 便捷函数: 检查库是否已加载
pub fn is_library_loaded(name: LibraryName) -> bool {
    LIBRARIES.lock().unwrap().is_loaded(name)
}

/// 便捷函数: 卸载库
pub fn unload_library(name: LibraryName) {
    LIBRARIES.lock().unwrap().unload(name);
}

/// 便捷函数: 卸载所有库
pub fn unload_all_libraries() {
    LIBRARIES.lock().unwrap().unload_all();
}

/// 批量加载核心游戏库
/// 
/// C# equivalent: Libraries 静态构造函数中的初始化逻辑
pub fn load_core_libraries() -> std::io::Result<()> {
    let mut libs = LIBRARIES.lock().unwrap();
    
    // 计算需要加载的库数量
    let core_libs = vec![
        LibraryName::Prguse,
        LibraryName::Prguse2,
        LibraryName::Magic,
        LibraryName::Magic2,
        LibraryName::Weather,
        LibraryName::Effect,
        LibraryName::Items,
        LibraryName::MagIcon,
        LibraryName::BuffIcon,
    ];
    
    libs.count = core_libs.len();
    libs.progress = 0;
    
    tracing::info!("开始加载核心库 ({} 个)...", libs.count);
    
    let mut errors = Vec::new();
    
    for lib_name in core_libs {
        if let Err(e) = libs.load(lib_name) {
            errors.push((lib_name, e));
        }
    }
    
    libs.loaded = errors.is_empty();
    
    if libs.loaded {
        tracing::info!("✓ 所有核心库加载完成 ({}/{})", libs.progress, libs.count);
        Ok(())
    } else {
        tracing::error!("✗ 部分库加载失败:");
        for (name, err) in &errors {
            tracing::error!("  - {}: {}", name, err);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} 个库加载失败", errors.len())
        ))
    }
}

/// 批量加载所有游戏库（可选）
/// 
/// 包括 UI、魔法、物品、装备等所有库
pub fn load_all_libraries() -> std::io::Result<()> {
    let mut libs = LIBRARIES.lock().unwrap();
    
    let all_libs = vec![
        // UI
        LibraryName::ChrSel,
        LibraryName::Prguse,
        LibraryName::Prguse2,
        LibraryName::Prguse3,
        LibraryName::BuffIcon,
        LibraryName::Help,
        LibraryName::MiniMap,
        LibraryName::MapLinkIcon,
        LibraryName::Title,
        LibraryName::Background,
        LibraryName::Dragon,
        
        // 魔法
        LibraryName::MagIcon,
        LibraryName::MagIcon2,
        LibraryName::Magic,
        LibraryName::Magic2,
        LibraryName::Magic3,
        LibraryName::Effect,
        LibraryName::MagicC,
        LibraryName::GuildSkill,
        
        // 天气/粒子
        LibraryName::Weather,
        
        // 物品
        LibraryName::Items,
        LibraryName::StateItems,
        LibraryName::FloorItems,
        
        // 装饰
        LibraryName::Deco,
    ];
    
    libs.count = all_libs.len();
    libs.progress = 0;
    
    tracing::info!("开始加载所有库 ({} 个)...", libs.count);
    
    let mut errors = Vec::new();
    
    for lib_name in all_libs {
        if let Err(e) = libs.load(lib_name) {
            errors.push((lib_name, e));
            // 继续加载其他库，不中断
        }
    }
    
    libs.loaded = !libs.libraries.is_empty();
    
    tracing::info!(
        "库加载完成: 成功 {}/{}, 失败 {}",
        libs.progress - errors.len(),
        libs.count,
        errors.len()
    );
    
    if !errors.is_empty() {
        tracing::warn!("以下库加载失败（可能不影响功能）:");
        for (name, err) in &errors {
            tracing::warn!("  - {}: {}", name, err);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_library_name_parsing() {
        assert_eq!(LibraryName::from_str("Weather"), Some(LibraryName::Weather));
        assert_eq!(LibraryName::from_str("Prguse"), Some(LibraryName::Prguse));
        assert_eq!(LibraryName::from_str("Invalid"), None);
    }
    
    #[test]
    fn test_library_default_path() {
        assert_eq!(LibraryName::Weather.default_path(), "Weather");
        assert_eq!(LibraryName::Prguse.default_path(), "Prguse");
    }
}
