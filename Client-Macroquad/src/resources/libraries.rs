//
// MIR2 图像库管理器
// 对应: Client/MirGraphics/Libraries.cs
//
// 提供所有游戏图像库的集中管理，类似 C# 原版的静态 Libraries 类
//
// # 使用示例
//
// ## 便捷访问单个图像
// ```rust
// use crate::resources::libraries::LibraryName;
//
// // 方式1: 使用 LibraryName 的便捷方法（推荐）
// if let Some(image_info) = LibraryName::Prguse.get_image(360) {
//     println!("图像: {}x{}, 偏移: ({}, {})", 
//         image_info.width, image_info.height,
//         image_info.x, image_info.y);
// }
//
// // 方式2: 只获取尺寸
// if let Some((w, h)) = LibraryName::Title.get_size(200) {
//     println!("OK按钮尺寸: {}x{}", w, h);
// }
// ```
//
// ## 全局库管理
// ```rust
// use crate::resources::libraries::{initialize_all_libraries, get_library};
//
// // 初始化所有库
// initialize_all_libraries("Data")?;
//
// // 访问单体库
// if let Some(lib_rc) = get_library(LibraryName::Magic) {
//     let mut lib = lib_rc.borrow_mut();
//     // 使用库...
// }
// ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::resources::mlibrary::ImageInfo;
use egui_macroquad::egui;

use super::mlibrary::MLibrary;

// 字符串填充辅助 trait
trait StringPadding {
    fn pad_to_width_with_char(&self, width: usize, ch: char) -> String;
}

impl StringPadding for String {
    fn pad_to_width_with_char(&self, width: usize, ch: char) -> String {
        if self.len() >= width {
            self.clone()
        } else {
            format!("{}{}", ch.to_string().repeat(width - self.len()), self)
        }
    }
}

/// 库名称枚举
///
/// 对应 C# Libraries 类中的所有静态字段
/// 
/// # 新特性：便捷访问方法
/// 
/// 每个库名称现在都支持直接访问图像：
/// ```ignore
/// // 旧方式（复杂）
/// let lib = get_library(LibraryName::Prguse)?;
/// let mut lib_borrow = lib.borrow_mut();
/// let info = lib_borrow.get_or_create_texture(360)?;
/// 
/// // 新方式（简洁） ✨
/// let info = LibraryName::Prguse.get_image(360)?;
/// 
/// // 只需要尺寸？更简单
/// let (w, h) = LibraryName::Title.get_size(200)?;
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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
    Magic,      // 主要魔法效果
    Magic2,     // 魔法效果2
    Magic3,     // 魔法效果3
    Effect,     // 特效
    MagicC,     // 魔法C
    GuildSkill, // 公会技能

    // 天气效果
    Weather, // 天气/粒子效果

    // 物品
    Items,      // 物品
    StateItems, // 状态物品
    FloorItems, // 地面物品

    // 装饰
    Deco, // 装饰物

    // ==================== 角色装备库 (NEW) ====================
    /// 通用装备库 (Warrior/Wizard/Taoist)
    /// CArmour/0000.Lib - CArmour/0999.Lib
    CArmours(usize),

    /// 刺客装备库 (Assassin)
    /// AArmour/0000.Lib - AArmour/0999.Lib
    AArmours(usize),

    /// 弓箭手装备库 (Archer alternative animation)
    /// ARArmour/0000.Lib - ARArmour/0999.Lib
    ARArmours(usize),

    /// 通用发型库 (Warrior/Wizard/Taoist)
    /// CHair/0000.Lib - CHair/0999.Lib
    CHair(usize),

    /// 刺客发型库
    /// AHair/0000.Lib - AHair/0999.Lib
    AHair(usize),

    /// 弓箭手发型库
    /// ARHair/0000.Lib - ARHair/0999.Lib
    ARHair(usize),

    /// 通用武器库
    /// CWeapon/0000.Lib - CWeapon/0999.Lib
    CWeapons(usize),

    /// 弓箭手武器库
    /// ARWeapon/0000.Lib - ARWeapon/0999.Lib
    ARWeapons(usize),

    /// 人物特效库 (翅膀等)
    /// CHumEffect/0000.Lib - CHumEffect/0999.Lib
    CHumEffect(usize),

    // 其他（可扩展）
    Custom(u32), // 自定义库（用于动态加载）
}

impl LibraryName {
    /// 便捷访问：从库中获取指定索引的图像信息
    /// 
    /// 如果库未加载会自动加载，如果索引不存在返回 None
    /// 
    /// # 参数
    /// - `index`: 图像索引
    /// 
    /// # 返回
    /// - `Some(ImageInfo)`: 图像信息的克隆
    /// - `None`: 库加载失败或索引不存在
    /// 
    /// # 示例
    /// ```ignore
    /// // 获取 Prguse 库的第 360 张图像
    /// if let Some(info) = LibraryName::Prguse.get_image(360) {
    ///     println!("图像尺寸: {}x{}", info.width, info.height);
    /// }
    /// ```
    pub fn get_image(&self, index: usize) -> Option<ImageInfo> {
        LIBRARIES.with(|libs| {
            let mut libs = libs.borrow_mut();
            let lib_rc = libs.get_or_load(*self)?;
            let mut lib = lib_rc.borrow_mut();
            lib.get_or_create_texture(index).ok().cloned()
        })
    }


    /// 便捷访问：获取指定索引图像的尺寸（宽度和高度）
    /// 
    /// 比 `get_image()` 更轻量，只返回尺寸信息
    /// 
    /// # 参数
    /// - `index`: 图像索引
    /// 
    /// # 返回
    /// - `Some((width, height))`: 图像的宽度和高度
    /// - `None`: 库加载失败或索引不存在
    /// 
    /// # 示例
    /// ```ignore
    /// if let Some((w, h)) = LibraryName::Title.get_size(200) {
    ///     println!("按钮尺寸: {}x{}", w, h);
    /// }
    /// ```
    pub fn get_size(&self, index: usize) -> Option<(i16, i16)> {
        LIBRARIES.with(|libs| {
            let libs = libs.borrow();
            let lib_rc = libs.get(*self)?;
            let mut lib = lib_rc.borrow_mut();
            lib.get_size(index).ok()
        })
    }

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

            // 角色装备库 (使用 {:02} 格式,对应 C# ToString("00"))
            // C# 代码: library[i] = new MLibrary(path + i.ToString("00") + suffix);
            // 生成文件名: 00.Lib, 01.Lib, 02.Lib, ... 99.Lib
            LibraryName::CArmours(idx) => format!("CArmour/{:02}", idx),
            LibraryName::AArmours(idx) => format!("AArmour/{:02}", idx),
            LibraryName::ARArmours(idx) => format!("ARArmour/{:02}", idx),
            LibraryName::CHair(idx) => format!("CHair/{:02}", idx),
            LibraryName::AHair(idx) => format!("AHair/{:02}", idx),
            LibraryName::ARHair(idx) => format!("ARHair/{:02}", idx),
            LibraryName::CWeapons(idx) => format!("CWeapon/{:02}", idx),
            LibraryName::ARWeapons(idx) => format!("ARWeapon/{:02}", idx),
            LibraryName::CHumEffect(idx) => format!("CHumEffect/{:02}", idx),

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

/// 数组库类型枚举
///
/// 对应 C# Libraries 类中的所有数组字段
/// C# Reference: Client/MirGraphics/MLibrary.cs lines 44-70
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryArray {
    // 地图瓦片 (C#: MapLibs[400])
    MapLibs,

    // 战士/法师/道士 (C# CArmours, CWeapons, etc.)
    CArmours,      // 护甲
    CWeapons,      // 武器
    CWeaponEffect, // 武器特效
    CHair,         // 发型
    CHumEffect,    // 人物特效

    // 刺客 (C# AArmours, AWeaponsL, etc.)
    AArmours,   // 刺客护甲
    AWeaponsL,  // 刺客左手武器
    AWeaponsR,  // 刺客右手武器
    AHair,      // 刺客发型
    AHumEffect, // 刺客特效

    // 弓箭手 (C# ARArmours, ARWeapons, etc.)
    ARArmours,   // 弓箭手护甲
    ARWeapons,   // 弓箭手武器
    ARWeaponsS,  // 弓箭手特殊武器
    ARHair,      // 弓箭手发型
    ARHumEffect, // 弓箭手特效

    // 生物和对象 (C# Monsters, Gates, etc.)
    Monsters, // 怪物 (1000+)
    Gates,    // 门
    Flags,    // 旗帜
    Siege,    // 攻城器械
    Mounts,   // 坐骑
    NPCs,     // NPC
    Fishing,  // 钓鱼
    Pets,     // 宠物

    // 变身系统 (C# Transform, TransformMounts, etc.)
    Transform,             // 变身
    TransformMounts,       // 变身坐骑
    TransformEffect,       // 变身特效
    TransformWeaponEffect, // 变身武器特效

    // 怪物装备 (C# MArmours, MWeapons, etc.)
    MArmours,      // 怪物护甲
    MWeapons,      // 怪物武器
    MWeaponEffect, // 怪物武器特效

    // 其他 (C# Title, Deco, Wings)
    Title, // 称号
    Deco,  // 装饰
    Wings, // 翅膀
}

impl LibraryArray {
    /// 获取数组库的名称（用于日志）
    pub fn name(&self) -> &'static str {
        match self {
            LibraryArray::MapLibs => "MapLibs",
            LibraryArray::CArmours => "CArmours",
            LibraryArray::CWeapons => "CWeapons",
            LibraryArray::CWeaponEffect => "CWeaponEffect",
            LibraryArray::CHair => "CHair",
            LibraryArray::CHumEffect => "CHumEffect",
            LibraryArray::AArmours => "AArmours",
            LibraryArray::AWeaponsL => "AWeaponsL",
            LibraryArray::AWeaponsR => "AWeaponsR",
            LibraryArray::AHair => "AHair",
            LibraryArray::AHumEffect => "AHumEffect",
            LibraryArray::ARArmours => "ARArmours",
            LibraryArray::ARWeapons => "ARWeapons",
            LibraryArray::ARWeaponsS => "ARWeaponsS",
            LibraryArray::ARHair => "ARHair",
            LibraryArray::ARHumEffect => "ARHumEffect",
            LibraryArray::Monsters => "Monsters",
            LibraryArray::Gates => "Gates",
            LibraryArray::Flags => "Flags",
            LibraryArray::Siege => "Siege",
            LibraryArray::Mounts => "Mounts",
            LibraryArray::NPCs => "NPCs",
            LibraryArray::Fishing => "Fishing",
            LibraryArray::Pets => "Pets",
            LibraryArray::Transform => "Transform",
            LibraryArray::TransformMounts => "TransformMounts",
            LibraryArray::TransformEffect => "TransformEffect",
            LibraryArray::TransformWeaponEffect => "TransformWeaponEffect",
            LibraryArray::MArmours => "MArmours",
            LibraryArray::MWeapons => "MWeapons",
            LibraryArray::MWeaponEffect => "MWeaponEffect",
            LibraryArray::Title => "Title",
            LibraryArray::Deco => "Deco",
            LibraryArray::Wings => "Wings",
        }
    }
}

impl std::fmt::Display for LibraryArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 全局库管理器
///
/// C# equivalent: Libraries static class
///
/// C# 使用静态类 + 静态字段，Rust 使用 Lazy 单例
pub struct Libraries {
    /// 单体库 (C# 的静态字段)
    libraries: HashMap<LibraryName, Rc<RefCell<MLibrary>>>,

    /// 数组库 (C# 的静态数组字段)
    /// 每个数组元素可能为 None (文件不存在)
    array_libraries: HashMap<LibraryArray, Vec<Option<Rc<RefCell<MLibrary>>>>>,

    /// egui 纹理缓存（全局）
    texture_cache: HashMap<String, egui::TextureHandle>,

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
            array_libraries: HashMap::new(),
            texture_cache: HashMap::new(),
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

    // ===== 数组库管理方法 =====

    /// 初始化数组库（分配空间）
    ///
    /// C# equivalent: `public static readonly MLibrary[] MapLibs = new MLibrary[400];`
    ///
    /// # 参数
    /// - `array_type`: 数组库类型
    /// - `size`: 数组大小
    pub fn init_array(&mut self, array_type: LibraryArray, size: usize) {
        tracing::debug!("初始化数组库 {} [0..{}]", array_type, size);
        self.array_libraries.insert(array_type, vec![None; size]);
    }

    /// 加载库到数组指定位置
    ///
    /// C# equivalent: `MapLibs[index] = new MLibrary(path);`
    ///
    /// # 参数
    /// - `array_type`: 数组库类型
    /// - `index`: 数组索引
    /// - `path`: 库文件路径
    pub fn load_to_array(
        &mut self,
        array_type: LibraryArray,
        index: usize,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let array = self.array_libraries.get_mut(&array_type).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("数组库 {} 未初始化", array_type),
            )
        })?;

        if index >= array.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("索引 {} 超出范围 [0..{})", index, array.len()),
            ));
        }

        let path_ref = path.as_ref();
        match MLibrary::open(path_ref) {
            Ok(lib) => {
                let count = lib.count();
                tracing::debug!(
                    "✓ {}[{}] = {} ({} 张图像)",
                    array_type,
                    index,
                    path_ref.display(),
                    count
                );
                array[index] = Some(Rc::new(RefCell::new(lib)));
                self.progress += 1;
                Ok(())
            }
            Err(e) => {
                // 文件不存在不是错误，只记录警告
                tracing::warn!(
                    "✗ {}[{}] = {} 失败: {}",
                    array_type,
                    index,
                    path_ref.display(),
                    e
                );
                array[index] = None;
                Ok(()) // 返回 Ok，允许继续加载其他库
            }
        }
    }

    /// 从数组库获取指定索引的库
    ///
    /// C# equivalent: `Libraries.MapLibs[index]`
    ///
    /// # 参数
    /// - `array_type`: 数组库类型
    /// - `index`: 数组索引
    ///
    /// # 返回
    /// - `Some(Rc<RefCell<MLibrary>>)`: 库引用
    /// - `None`: 索引无效或库未加载
    pub fn get_from_array(
        &self,
        array_type: LibraryArray,
        index: usize,
    ) -> Option<Rc<RefCell<MLibrary>>> {
        self.array_libraries.get(&array_type)?.get(index)?.clone()
    }

    /// 获取数组库的大小
    pub fn get_array_size(&self, array_type: LibraryArray) -> usize {
        self.array_libraries
            .get(&array_type)
            .map(|arr| arr.len())
            .unwrap_or(0)
    }

    /// 获取数组库中已加载库的数量
    pub fn get_array_loaded_count(&self, array_type: LibraryArray) -> usize {
        self.array_libraries
            .get(&array_type)
            .map(|arr| arr.iter().filter(|lib| lib.is_some()).count())
            .unwrap_or(0)
    }

    /// 获取数组库中所有已加载的库 (用于纹理缓存清理)
    ///
    /// # 参数
    /// - `array_type`: 数组库类型
    ///
    /// # 返回
    /// - Vec<Rc<RefCell<MLibrary>>>: 所有已加载的库引用
    pub fn get_all_from_array(&self, array_type: LibraryArray) -> Vec<Rc<RefCell<MLibrary>>> {
        self.array_libraries
            .get(&array_type)
            .map(|arr| arr.iter().filter_map(|lib| lib.clone()).collect())
            .unwrap_or_default()
    }

    // // ===== 全局 egui 纹理缓存管理 =====

    // /// 获取或创建 egui 纹理
    // pub fn get_or_create_egui_texture(
    //     &mut self,
    //     ctx: &egui::Context,
    //     lib: &mut MLibrary,
    //     lib_name: &str,
    //     index: usize,
    // ) -> Option<egui::TextureHandle> {
    //     let key = format!("{}_{}", lib_name, index);

    //     // 检查缓存
    //     if let Some(handle) = self.texture_cache.get(&key) {
    //         return Some(handle.clone());
    //     }

    //     // 从库中加载纹理
    //     if let Ok(info) = lib.get_or_create_texture(index) {
    //         if let Some(ref texture) = info.image {
    //             // 直接从 macroquad 纹理创建 egui 纹理
    //             let image_data = texture.get_texture_data();
    //             let width = texture.width() as usize;
    //             let height = texture.height() as usize;

    //             let mut pixels = Vec::with_capacity(width * height);
    //             for y in 0..height {
    //                 for x in 0..width {
    //                     let idx = (y * width + x) * 4;
    //                     let r = image_data.bytes[idx];
    //                     let g = image_data.bytes[idx + 1];
    //                     let b = image_data.bytes[idx + 2];
    //                     let a = image_data.bytes[idx + 3];
    //                     pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
    //                 }
    //             }

    //             let color_image = egui::ColorImage {
    //                 size: [width, height],
    //                 pixels,
    //             };

    //             let handle = ctx.load_texture(&key, color_image, Default::default());
    //             self.texture_cache.insert(key, handle.clone());
    //             return Some(handle);
    //         }
    //     }

    //     None
    // }

    /// 清理 egui 纹理缓存
    pub fn clear_texture_cache(&mut self) {
        self.texture_cache.clear();
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
                self.libraries.insert(name, Rc::new(RefCell::new(lib)));
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
    pub fn load_custom(
        &mut self,
        name: LibraryName,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let path_str = path.as_ref().display().to_string();

        tracing::info!("加载库: {} (自定义路径: {})", name, path_str);

        match MLibrary::open(path) {
            Ok(lib) => {
                tracing::info!("✓ 成功加载 {} ({} 张图像)", name, lib.count());
                self.libraries.insert(name, Rc::new(RefCell::new(lib)));
                self.progress += 1;
                Ok(())
            }
            Err(e) => {
                tracing::warn!("✗ 加载失败 {}: {}", name, e);
                Err(e)
            }
        }
    }

    /// 获取库引用 (如果未加载则自动加载)
    ///
    /// C# equivalent: 直接访问 Libraries.Weather
    pub fn get_or_load(&mut self, name: LibraryName) -> Option<Rc<RefCell<MLibrary>>> {
        // 如果已加载，直接返回
        if let Some(lib) = self.libraries.get(&name) {
            return Some(lib.clone());
        }

        // 否则尝试加载
        tracing::info!("🔄 懒加载库: {:?}", name);
        if self.load(name.clone()).is_ok() {
            self.libraries.get(&name).cloned()
        } else {
            None
        }
    }

    /// 获取库引用 (不自动加载)
    ///
    /// C# equivalent: 直接访问 Libraries.Weather
    pub fn get(&self, name: LibraryName) -> Option<Rc<RefCell<MLibrary>>> {
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

    // ===== MapLibs 专用初始化方法 =====

    /// 初始化所有 MapLibs[0-399]
    ///
    /// C# Reference: Client/MirGraphics/MLibrary.cs lines 122-201
    ///
    /// MapLibs 索引分配:
    /// - 0-99: WeMade Mir2
    /// - 100-199: Shanda Mir2
    /// - 200-299: WeMade Mir3
    /// - 300-399: Shanda Mir3
    pub fn init_map_libraries(&mut self) -> std::io::Result<()> {
        tracing::info!("初始化 MapLibs[0-399]...");

        // 初始化数组
        self.init_array(LibraryArray::MapLibs, 400);

        // WeMade Mir2 (0-99)
        self.init_wemade_mir2_maps()?;

        // Shanda Mir2 (100-199)
        self.init_shanda_mir2_maps()?;

        // WeMade Mir3 (200-299)
        self.init_wemade_mir3_maps()?;

        // Shanda Mir3 (300-399)
        self.init_shanda_mir3_maps()?;

        let loaded = self.get_array_loaded_count(LibraryArray::MapLibs);
        tracing::info!("✓ MapLibs 初始化完成: {}/400 个库已加载", loaded);

        Ok(())
    }

    /// 初始化 WeMade Mir2 地图库 (0-99)
    /// C# Reference: lines 122-131
    fn init_wemade_mir2_maps(&mut self) -> std::io::Result<()> {
        let base = format!("{}/Map/WemadeMir2", self.data_path);

        // MapLibs[0] = Tiles
        self.load_to_array(LibraryArray::MapLibs, 0, format!("{}/Tiles", base))?;

        // MapLibs[1] = Smtiles
        self.load_to_array(LibraryArray::MapLibs, 1, format!("{}/Smtiles", base))?;

        // MapLibs[2] = Objects
        self.load_to_array(LibraryArray::MapLibs, 2, format!("{}/Objects", base))?;

        // MapLibs[3-29] = Objects2-Objects28
        for i in 2..28 {
            self.load_to_array(
                LibraryArray::MapLibs,
                i + 1,
                format!("{}/Objects{}", base, i),
            )?;
        }

        // MapLibs[90] = Objects_32bit
        self.load_to_array(LibraryArray::MapLibs, 90, format!("{}/Objects_32bit", base))?;

        Ok(())
    }

    /// 初始化 Shanda Mir2 地图库 (100-199)
    /// C# Reference: lines 133-151
    fn init_shanda_mir2_maps(&mut self) -> std::io::Result<()> {
        let base = format!("{}/Map/ShandaMir2", self.data_path);

        // MapLibs[100] = Tiles
        self.load_to_array(LibraryArray::MapLibs, 100, format!("{}/Tiles", base))?;

        // MapLibs[101-109] = Tiles2-Tiles10
        for i in 1..10 {
            self.load_to_array(
                LibraryArray::MapLibs,
                100 + i,
                format!("{}/Tiles{}", base, i + 1),
            )?;
        }

        // MapLibs[110] = SmTiles
        self.load_to_array(LibraryArray::MapLibs, 110, format!("{}/SmTiles", base))?;

        // MapLibs[111-119] = SmTiles2-SmTiles10
        for i in 1..10 {
            self.load_to_array(
                LibraryArray::MapLibs,
                110 + i,
                format!("{}/SmTiles{}", base, i + 1),
            )?;
        }

        // MapLibs[120] = Objects
        self.load_to_array(LibraryArray::MapLibs, 120, format!("{}/Objects", base))?;

        // MapLibs[121-150] = Objects2-Objects31
        for i in 1..31 {
            self.load_to_array(
                LibraryArray::MapLibs,
                120 + i,
                format!("{}/Objects{}", base, i + 1),
            )?;
        }

        // MapLibs[190] = AniTiles1
        self.load_to_array(LibraryArray::MapLibs, 190, format!("{}/AniTiles1", base))?;

        Ok(())
    }

    /// 初始化 WeMade Mir3 地图库 (200-299)
    /// C# Reference: lines 152-168
    fn init_wemade_mir3_maps(&mut self) -> std::io::Result<()> {
        let base = format!("{}/Map/WemadeMir3", self.data_path);
        let map_states = ["", "wood/", "sand/", "snow/", "forest/"];

        for (state_idx, state) in map_states.iter().enumerate() {
            let state_base = format!("{}/{}", base, state);
            let offset = 200 + (state_idx * 15);

            // 每个状态15个库
            let tiles = [
                "Tilesc",
                "Tiles30c",
                "Tiles5c",
                "Smtilesc",
                "Housesc",
                "Cliffsc",
                "Dungeonsc",
                "Innersc",
                "Furnituresc",
                "Wallsc",
                "smObjectsc",
                "Animationsc",
                "Object1c",
                "Object2c",
            ];

            for (i, tile_name) in tiles.iter().enumerate() {
                self.load_to_array(
                    LibraryArray::MapLibs,
                    offset + i,
                    format!("{}{}", state_base, tile_name),
                )?;
            }
        }

        Ok(())
    }

    /// 初始化 Shanda Mir3 地图库 (300-399)
    /// C# Reference: lines 169-184
    fn init_shanda_mir3_maps(&mut self) -> std::io::Result<()> {
        let base = format!("{}/Map/ShandaMir3", self.data_path);
        let map_states = ["", "wood", "sand", "snow", "forest"];

        for (state_idx, state) in map_states.iter().enumerate() {
            let offset = 300 + (state_idx * 15);

            // 每个状态15个库（注意文件名格式与 WeMade 不同）
            let tiles = [
                "Tilesc",
                "Tiles30c",
                "Tiles5c",
                "Smtilesc",
                "Housesc",
                "Cliffsc",
                "Dungeonsc",
                "Innersc",
                "Furnituresc",
                "Wallsc",
                "smObjectsc",
                "Animationsc",
                "Object1c",
                "Object2c",
            ];

            for (i, tile_name) in tiles.iter().enumerate() {
                let full_name = if state.is_empty() {
                    tile_name.to_string()
                } else {
                    format!("{}{}", tile_name, state)
                };

                self.load_to_array(
                    LibraryArray::MapLibs,
                    offset + i,
                    format!("{}/{}", base, full_name),
                )?;
            }
        }

        Ok(())
    }

    // ==================== 其他数组库初始化 ====================

    /// 通用方法: 从目录自动扫描并初始化数组库
    ///
    /// C# Reference: InitLibrary() (line 197-212)
    ///
    /// # Arguments
    /// * `array_type` - 数组库类型
    /// * `dir_path` - 目录路径 (相对于 data_path)
    /// * `padding` - 文件名数字填充 (如 "000" 表示3位填充)
    /// * `suffix` - 文件名后缀 (可选)
    pub fn init_library_from_directory(
        &mut self,
        array_type: LibraryArray,
        dir_path: impl AsRef<Path>,
        padding: &str,
        suffix: &str,
    ) -> std::io::Result<()> {
        let full_path = PathBuf::from(&self.data_path).join(dir_path.as_ref());

        // 如果目录不存在,创建空数组
        if !full_path.exists() {
            tracing::warn!(
                "✗ 目录不存在,创建空数组: {:?} - {:?}",
                array_type,
                full_path
            );
            self.init_array(array_type, 0);
            return Ok(());
        }

        // 扫描目录中的所有 .lib 文件 (不区分大小写)
        let _pattern = format!("*{}.lib", suffix);
        let mut lib_files: Vec<_> = std::fs::read_dir(&full_path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let path = entry.path();
                if !path.is_file() {
                    return false;
                }
                // Windows不区分大小写,检查扩展名时也不区分
                match path.extension().and_then(|s| s.to_str()) {
                    Some(ext) => ext.eq_ignore_ascii_case("lib"),
                    None => false,
                }
            })
            .collect();

        if lib_files.is_empty() {
            tracing::warn!("✗ 目录中无 .lib 文件: {:?}", full_path);
            self.init_array(array_type, 0);
            return Ok(());
        }

        // 提取文件名中的数字并排序
        lib_files.sort_by_key(|entry| {
            let filename = entry.file_name();
            let name_str = filename.to_string_lossy();

            // 提取数字部分
            name_str
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<usize>()
                .unwrap_or(0)
        });

        // 找到最大的索引号
        let last_file = lib_files.last().unwrap();
        let last_filename = last_file.file_name();
        let last_name = last_filename.to_string_lossy();
        let max_index = last_name
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(0);

        let array_size = max_index + 1;

        tracing::info!(
            "📚 初始化 {:?}: 扫描到 {} 个文件, 数组大小 {}",
            array_type,
            lib_files.len(),
            array_size
        );

        // 初始化数组
        self.init_array(array_type, array_size);

        // 加载所有文件
        let mut loaded_count = 0;
        for i in 0..array_size {
            // Windows不区分大小写,尝试.lib和.Lib两种扩展名
            let filename_lower = format!(
                "{}{}.lib",
                i.to_string().pad_to_width_with_char(padding.len(), '0'),
                suffix
            );
            let filename_upper = format!(
                "{}{}.Lib",
                i.to_string().pad_to_width_with_char(padding.len(), '0'),
                suffix
            );

            let file_path_lower = full_path.join(&filename_lower);
            let file_path_upper = full_path.join(&filename_upper);

            let file_path = if file_path_lower.exists() {
                file_path_lower
            } else if file_path_upper.exists() {
                file_path_upper
            } else {
                continue; // 文件不存在,跳过
            };

            match self.load_to_array(array_type, i, &file_path) {
                Ok(_) => {
                    loaded_count += 1;
                }
                Err(e) => {
                    tracing::debug!("  ✗ 加载失败 [{}]: {} - {}", i, file_path.display(), e);
                }
            }
        }

        tracing::info!(
            "✓ {:?} 初始化完成: {}/{} 个库已加载",
            array_type,
            loaded_count,
            array_size
        );

        Ok(())
    }

    /// 初始化所有游戏内容数组库
    ///
    /// C# Reference: LoadGameLibraries() (line 241-289)
    pub fn init_game_libraries(&mut self) -> std::io::Result<()> {
        println!("🎮🎮🎮 [LIBRARIES] 开始初始化游戏内容库...");
        tracing::info!("🎮 开始初始化游戏内容库...");

        // Monsters (怪物)
        println!("👹 初始化 Monsters 库...");
        self.init_library_from_directory(LibraryArray::Monsters, "Monster", "000", "")?;

        // Gates (传送门)
        self.init_library_from_directory(LibraryArray::Gates, "Gate", "00", "")?;

        // NPCs
        self.init_library_from_directory(LibraryArray::NPCs, "NPC", "00", "")?;

        // Mounts (坐骑)
        self.init_library_from_directory(LibraryArray::Mounts, "Mount", "00", "")?;

        // Fishing (钓鱼)
        self.init_library_from_directory(LibraryArray::Fishing, "Fishing", "00", "")?;

        // Pets (宠物)
        self.init_library_from_directory(LibraryArray::Pets, "Pets", "00", "")?;

        // Transform (变身)
        self.init_library_from_directory(LibraryArray::Transform, "Transform", "00", "")?;

        // Transform Mounts (坐骑变身)
        self.init_library_from_directory(
            LibraryArray::TransformMounts,
            "TransformMount",
            "00",
            "",
        )?;

        // Transform Effect (变身特效)
        self.init_library_from_directory(
            LibraryArray::TransformEffect,
            "TransformEffect",
            "00",
            "",
        )?;

        // Transform Weapon Effect (武器变身特效)
        self.init_library_from_directory(
            LibraryArray::TransformWeaponEffect,
            "TransformWeaponEffect",
            "00",
            "",
        )?;

        // Character Armours (人物盔甲 - 8方向)
        self.init_library_from_directory(
            LibraryArray::CArmours,
            "CArmour",
            "00", // ✅ 修复：文件名是00.Lib而不是000.lib
            "",
        )?;

        // Character Weapons (人物武器 - 8方向)
        self.init_library_from_directory(
            LibraryArray::CWeapons,
            "CWeapon",
            "00", // ✅ 修复：文件名是00.Lib而不是000.lib
            "",
        )?;

        // Character Hair (人物发型)
        self.init_library_from_directory(
            LibraryArray::CHair,
            "CHair",
            "00", // ✅ 修复：文件名是00.Lib而不是000.lib
            "",
        )?;

        // Character Weapon Effects (人物武器特效)
        self.init_library_from_directory(LibraryArray::CWeaponEffect, "CWeaponEffect", "00", "")?;

        // Assistant Armours (助手/英雄盔甲 - 3方向)
        self.init_library_from_directory(
            LibraryArray::AArmours,
            "AArmour",
            "00", // ✅ 修复：文件名是00.Lib
            "",
        )?;

        // Assistant Weapons Left (助手/英雄左手武器 - 3方向)
        self.init_library_from_directory(LibraryArray::AWeaponsL, "AWeaponL", "000", "")?;

        // Assistant Weapons Right (助手/英雄右手武器 - 3方向)
        self.init_library_from_directory(LibraryArray::AWeaponsR, "AWeaponR", "000", "")?;

        // Assistant Hair (助手/英雄发型)
        self.init_library_from_directory(
            LibraryArray::AHair,
            "AHair",
            "00", // ✅ 修复
            "",
        )?;

        // Assistant Riding Armours (助手/英雄骑乘盔甲)
        self.init_library_from_directory(
            LibraryArray::ARArmours,
            "ARArmour",
            "00", // ✅ 修复
            "",
        )?;

        // Assistant Riding Weapons (助手/英雄骑乘武器)
        self.init_library_from_directory(
            LibraryArray::ARWeapons,
            "ARWeapon",
            "00", // ✅ 修复
            "",
        )?;

        // Assistant Riding Hair (助手/英雄骑乘发型)
        self.init_library_from_directory(
            LibraryArray::ARHair,
            "ARHair",
            "00", // ✅ 修复
            "",
        )?;

        // Title (称号)
        self.init_library_from_directory(LibraryArray::Title, "Title", "000", "")?;

        // Deco (装饰)
        self.init_library_from_directory(LibraryArray::Deco, "Deco", "00", "")?;

        // Monster Armours (怪物盔甲)
        self.init_library_from_directory(LibraryArray::MArmours, "MArmour", "000", "")?;

        // Monster Weapons (怪物武器)
        self.init_library_from_directory(LibraryArray::MWeapons, "MWeapon", "000", "")?;

        // Monster Weapon Effects (怪物武器特效)
        self.init_library_from_directory(LibraryArray::MWeaponEffect, "MWeaponEffect", "00", "")?;

        // Wings (翅膀)
        self.init_library_from_directory(LibraryArray::Wings, "Wing", "00", "")?;

        // 🆕 Character Human Effects (人物特效 - 包括角色身体动画)
        println!("🧍 初始化 CHumEffect 库...");
        self.init_library_from_directory(LibraryArray::CHumEffect, "CHumEffect", "00", "")?;

        tracing::info!("✓ 游戏内容库初始化完成");

        Ok(())
    }

    // get_array_loaded_count 和 get_array_size 已在上面定义 (第368-380行附近)
}

// 全局库管理器单例 (单线程)
//
// C# equivalent: Libraries static class
thread_local! {
    static LIBRARIES: RefCell<Libraries> = RefCell::new(Libraries::new());
}

// ===== 便捷访问函数 =====

/// 便捷函数: 获取单体库 (如果未加载则自动懒加载)
pub fn get_library(name: LibraryName) -> Option<Rc<RefCell<MLibrary>>> {
    LIBRARIES.with(|libs| libs.borrow_mut().get_or_load(name))
}

/// 便捷函数: 获取数组库中的某个元素
pub fn get_library_from_array(
    array_type: LibraryArray,
    index: usize,
) -> Option<Rc<RefCell<MLibrary>>> {
    LIBRARIES.with(|libs| libs.borrow().get_from_array(array_type, index))
}

pub fn get_or_create_texture(lib_name: LibraryName, index: usize) -> Option<ImageInfo> {
    LIBRARIES.with(|libs| {
        let mut libs = libs.borrow_mut();
        // 获取库
        let lib_rc = libs.get_or_load(lib_name.clone())?;
        let mut lib = lib_rc.borrow_mut();
        lib.get_or_create_texture(index).ok().cloned()
    })
}

pub fn get_size(lib_name: LibraryName, index: usize) -> Option<(i16, i16)> {
    LIBRARIES.with(|libs| {
        let libs = libs.borrow();
        // 获取库
        let lib_rc = libs.get(lib_name)?;
        let mut lib = lib_rc.borrow_mut();
        lib.get_size(index).ok()
    })
}

// /// 便捷函数: 获取或创建 egui 纹理（使用全局缓存）
// ///
// /// 这是推荐的方式，避免每个组件都维护自己的纹理缓存
// pub fn get_or_create_egui_texture(
//     ctx: &egui::Context,
//     lib_name: LibraryName,
//     index: usize,
// ) -> Option<egui::TextureHandle> {
//     LIBRARIES.with(|libs| {
//         let mut libs = libs.borrow_mut();

//         // 获取库
//         let lib_rc = libs.get_or_load(lib_name.clone())?;
//         let mut lib = lib_rc.borrow_mut();

//         // 使用库名称作为缓存键
//         libs.get_or_create_egui_texture(ctx, &mut *lib, &lib_name.to_string().to_lowercase(), index)
//     })
// }

/// 便捷函数: 清理全局纹理缓存
pub fn clear_egui_texture_cache() {
    LIBRARIES.with(|libs| libs.borrow_mut().clear_texture_cache())
}

/// 便捷函数: 获取 MapLibs[index]
///
/// 这是最常用的访问方式，专门为地图渲染优化
pub fn get_map_library(index: i16) -> Option<Rc<RefCell<MLibrary>>> {
    if index < 0 || index >= 400 {
        return None;
    }
    get_library_from_array(LibraryArray::MapLibs, index as usize)
}

/// 便捷函数: 初始化所有库（包括 MapLibs）
///
/// C# equivalent: Libraries static constructor
///
/// 这是推荐的初始化方式，一次性完成所有准备工作
pub fn initialize_all_libraries(data_path: &str) -> std::io::Result<()> {
    LIBRARIES.with(|libs| {
        let mut libs = libs.borrow_mut();
        libs.set_data_path(data_path);

        tracing::info!("=== 开始初始化所有库 ===");
        load_core_libraries()?;
        // 1. 初始化 MapLibs[0-399]
        libs.init_map_libraries()?;
        // // 2. 加载核心 UI 库 (同步)
        // let core_libs = [
        //     LibraryName::ChrSel,
        //     LibraryName::Prguse,
        //     LibraryName::Prguse2,
        //     LibraryName::Prguse3,
        //     LibraryName::Title,
        // ];

        // for lib_name in core_libs {
        //     if let Err(e) = libs.load(lib_name.clone()) {
        //         tracing::warn!("核心库 {} 加载失败: {}", lib_name, e);
        //     }
        // }

        // 3. 初始化游戏内容数组库 (异步/延迟加载)
        // 这些库在后台加载，不会阻塞主线程
        if let Err(e) = libs.init_game_libraries() {
            tracing::warn!("游戏内容库初始化部分失败: {}", e);
        }

        tracing::info!("=== 库初始化完成 ===");
        tracing::info!(
            "  - MapLibs: {}/{} 个已加载",
            libs.get_array_loaded_count(LibraryArray::MapLibs),
            libs.get_array_size(LibraryArray::MapLibs)
        );
        tracing::info!(
            "  - Monsters: {}/{} 个已加载",
            libs.get_array_loaded_count(LibraryArray::Monsters),
            libs.get_array_size(LibraryArray::Monsters)
        );
        tracing::info!(
            "  - NPCs: {}/{} 个已加载",
            libs.get_array_loaded_count(LibraryArray::NPCs),
            libs.get_array_size(LibraryArray::NPCs)
        );
        tracing::info!("  - 单体库: {} 个已加载", libs.loaded_count());

        Ok(())
    })
}

/// 便捷函数: 初始化数据路径
pub fn set_data_path(path: impl Into<String>) {
    LIBRARIES.with(|libs| libs.borrow_mut().set_data_path(path));
}

/// 便捷函数: 初始化地图库 (MapLibs[0-399])
pub fn init_map_libraries() -> std::io::Result<()> {
    LIBRARIES.with(|libs| libs.borrow_mut().init_map_libraries())
}

/// 便捷函数: 加载库
pub fn load_library(name: LibraryName) -> std::io::Result<()> {
    LIBRARIES.with(|libs| libs.borrow_mut().load(name))
}

/// 便捷函数: 加载库（自定义路径）
pub fn load_library_custom(name: LibraryName, path: impl AsRef<Path>) -> std::io::Result<()> {
    LIBRARIES.with(|libs| libs.borrow_mut().load_custom(name, path))
}

// get_library 已在上面定义 (第620行附近)

/// 便捷函数: 检查库是否已加载
pub fn is_library_loaded(name: LibraryName) -> bool {
    LIBRARIES.with(|libs| libs.borrow().is_loaded(name))
}

/// 便捷函数: 卸载库
pub fn unload_library(name: LibraryName) {
    LIBRARIES.with(|libs| libs.borrow_mut().unload(name));
}

/// 便捷函数: 卸载所有库
pub fn unload_all_libraries() {
    LIBRARIES.with(|libs| libs.borrow_mut().unload_all());
}

/// 批量加载核心游戏库
///
/// C# equivalent: Libraries 静态构造函数中的初始化逻辑
pub fn load_core_libraries() -> std::io::Result<()> {
    LIBRARIES.with(|libs| {
        let mut libs = libs.borrow_mut();

        // 计算需要加载的库数量
        let core_libs = vec![
            LibraryName::ChrSel, // 角色选择/登录背景
            LibraryName::Title,  // 标题和按钮
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
            if let Err(e) = libs.load(lib_name.clone()) {
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
                format!("{} 个库加载失败", errors.len()),
            ))
        }
    })
}

/// 批量加载所有游戏库（可选）
///
/// 包括 UI、魔法、物品、装备等所有库
pub fn load_all_libraries() -> std::io::Result<()> {
    LIBRARIES.with(|libs| {
        let mut libs = libs.borrow_mut();

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
            if let Err(e) = libs.load(lib_name.clone()) {
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
    })
}

// ==================== MapLibs 地图资源库 ====================
//
// 对应 C# Libraries.MapLibs[400] 数组
// 旧的 MapLibs 结构体已被移除
// 现在使用统一的 Libraries.array_libraries[LibraryArray::MapLibs] 系统

/// 便捷函数: 获取所有 MapLibs (用于纹理缓存清理)
///
/// 对应 C# 中遍历 MapLibs 数组清理纹理的操作
///
/// # 返回
/// - Vec<Rc<RefCell<MLibrary>>>: 所有已加载的 MapLibs
pub fn get_all_map_libraries() -> Vec<Rc<RefCell<MLibrary>>> {
    LIBRARIES.with(|libs| libs.borrow().get_all_from_array(LibraryArray::MapLibs))
}
