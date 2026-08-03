// Libraries - MIR2 图像库管理器 (Bevy 版本)
// 对应: Client/MirGraphics/Libraries.cs + Client-Macroquad/src/resources/libraries.rs
//
// 与 macroquad 版保持相同的文件/索引映射，但使用本 crate 的
// `MLibrary`（原始 RGBA）而非带 macroquad 纹理的版本。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::resources::mlibrary::{ImageInfo, MLibrary};

/// 库名称枚举（核心子集，后续按需扩展）
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LibraryName {
    ChrSel,
    Prguse,
    Prguse2,
    Prguse3,
    BuffIcon,
    Help,
    MiniMap,
    MapLinkIcon,
    Title,
    MagIcon,
    MagIcon2,
    Magic,
    Magic2,
    Magic3,
    Effect,
    MagicC,
    GuildSkill,
    Weather,
    Background,
    Dragon,
    Items,
    StateItems,
    FloorItems,
    Deco,
}

impl LibraryName {
    /// 库相对 Data 目录的路径
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
        }
    }
}

/// 数组库类型（角色/怪物/NPC 装备库，按需懒加载）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayLibType {
    /// 怪物库 Monster/{:03}.Lib
    Monsters,
    /// NPC 库 NPC/{:02}.Lib
    Npcs,
    /// 战士/法师/道士护甲 CArmour/{:02}.Lib
    CArmours,
    /// 发型 CHair/{:02}.Lib
    CHair,
    /// 武器 CWeapon/{:02}.Lib
    CWeapons,
    /// 人物特效（翅膀等）CHumEffect/{:02}.Lib
    CHumEffect,
    /// 坐骑 Mount/{:02}.Lib（M60；帧布局：站立 0/行走 32/奔跑 96/受击 144/攻击 168）
    Mounts,
    /// 武器特效 CWeaponEffect/{:02}.Lib（M62；DrawBlend 0.4 透明度）
    CWeaponEffect,
}

impl ArrayLibType {
    pub fn name(&self) -> &'static str {
        match self {
            ArrayLibType::Monsters => "Monsters",
            ArrayLibType::Npcs => "Npcs",
            ArrayLibType::CArmours => "CArmours",
            ArrayLibType::CHair => "CHair",
            ArrayLibType::CWeapons => "CWeapons",
            ArrayLibType::CHumEffect => "CHumEffect",
            ArrayLibType::Mounts => "Mounts",
            ArrayLibType::CWeaponEffect => "CWeaponEffect",
        }
    }

    /// 相对 Data 目录的路径（不含扩展名）
    pub fn default_path(&self, index: usize) -> String {
        match self {
            ArrayLibType::Monsters => format!("Monster/{:03}", index),
            ArrayLibType::Npcs => format!("NPC/{:02}", index),
            ArrayLibType::CArmours => format!("CArmour/{:02}", index),
            ArrayLibType::CHair => format!("CHair/{:02}", index),
            ArrayLibType::CWeapons => format!("CWeapon/{:02}", index),
            ArrayLibType::CHumEffect => format!("CHumEffect/{:02}", index),
            ArrayLibType::Mounts => format!("Mount/{:02}", index),
            ArrayLibType::CWeaponEffect => format!("CWeaponEffect/{:02}", index),
        }
    }
}

impl std::fmt::Display for ArrayLibType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 解析 Data 根目录。
///
/// 优先使用本 crate 的 Data/，其次共享其他客户端的数据目录
/// （避免复制数 GB 资源）。
pub fn resolve_data_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{}/Data", manifest_dir),
        format!("{}/../Client-Macroquad/Data", manifest_dir),
        format!("{}/../../Crystal/Client-Macroquad/Data", manifest_dir),
        format!("{}/../ClientRust/Data", manifest_dir),
        format!("{}/../../Crystal/ClientRust/Data", manifest_dir),
    ];
    // 要求目录内确实存在 .Lib 数据（Items.Lib 是核心库）。
    // 独立 worktree 里 Client-Macroquad/Data 被 gitignore 不存在，
    // 会正确回落到主仓库的数据目录。
    for c in &candidates {
        let p = Path::new(c);
        if p.join("Items.Lib").exists() {
            return p.to_path_buf();
        }
    }
    // 兜底：第一个存在的目录（用于报错提示）
    for c in &candidates {
        let p = Path::new(c);
        if p.exists() {
            return p.to_path_buf();
        }
    }
    PathBuf::from(format!("{}/Data", manifest_dir))
}

/// 全局库管理器
pub struct Libraries {
    /// 单体库
    libraries: HashMap<LibraryName, MLibrary>,
    /// 地图库 (C#: MapLibs[400])
    map_libs: Vec<Option<MLibrary>>,
    /// 数组库（角色/怪物/NPC，懒加载）
    array_libs: HashMap<ArrayLibType, Vec<Option<MLibrary>>>,
    /// 数据根目录
    data_path: PathBuf,
    /// 已加载数量
    pub loaded: usize,
    /// 是否已初始化（单例+地图库）
    pub initialized: bool,
}

impl Libraries {
    pub fn new(data_path: impl Into<PathBuf>) -> Self {
        Self {
            libraries: HashMap::new(),
            map_libs: (0..400).map(|_| None).collect(),
            array_libs: HashMap::new(),
            data_path: data_path.into(),
            loaded: 0,
            initialized: false,
        }
    }

    /// 一次性初始化：解析数据目录 + 加载单体库 + 地图库
    pub fn ensure_initialized(&mut self) {
        if self.initialized {
            return;
        }
        self.data_path = resolve_data_path();
        self.init_single_libraries();
        self.init_map_libraries();
        self.initialized = true;
    }

    /// 加载所有单体库（UI/物品/特效等）。缺失的库跳过并记警告。
    pub fn init_single_libraries(&mut self) {
        let names = [
            LibraryName::ChrSel,
            LibraryName::Prguse,
            LibraryName::Prguse2,
            LibraryName::Prguse3,
            LibraryName::BuffIcon,
            LibraryName::Help,
            LibraryName::MiniMap,
            LibraryName::MapLinkIcon,
            LibraryName::Title,
            LibraryName::MagIcon,
            LibraryName::MagIcon2,
            LibraryName::Magic,
            LibraryName::Magic2,
            LibraryName::Magic3,
            LibraryName::Effect,
            LibraryName::MagicC,
            LibraryName::GuildSkill,
            LibraryName::Weather,
            LibraryName::Background,
            LibraryName::Dragon,
            LibraryName::Items,
            LibraryName::StateItems,
            LibraryName::FloorItems,
            LibraryName::Deco,
        ];
        for name in names {
            let path = self.data_path.join(name.default_path());
            match MLibrary::open(&path) {
                Ok(lib) => {
                    tracing::info!("✓ {} ({} 张图像)", path.display(), lib.count());
                    self.libraries.insert(name, lib);
                    self.loaded += 1;
                }
                Err(e) => {
                    tracing::warn!("✗ {} 加载失败: {}", path.display(), e);
                }
            }
        }
        tracing::info!("✓ 单体库加载完成: {}/{}", self.libraries.len(), names.len());
    }

    /// 初始化 MapLibs[0-399]
    ///
    /// MapLibs 索引分配:
    /// - 0-99: WeMade Mir2
    /// - 100-199: Shanda Mir2
    /// - 200-299: WeMade Mir3
    /// - 300-399: Shanda Mir3
    pub fn init_map_libraries(&mut self) {
        tracing::info!("初始化 MapLibs[0-399]...");
        self.init_wemade_mir2_maps();
        self.init_shanda_mir2_maps();
        self.init_wemade_mir3_maps();
        self.init_shanda_mir3_maps();

        let loaded = self.map_libs.iter().filter(|l| l.is_some()).count();
        tracing::info!("✓ MapLibs 初始化完成: {}/400 个库已加载", loaded);
    }

    fn load_to_map_slot(&mut self, index: usize, path: impl AsRef<Path>) {
        if index >= self.map_libs.len() {
            return;
        }
        let path_ref = path.as_ref();
        match MLibrary::open(path_ref) {
            Ok(lib) => {
                tracing::debug!("✓ MapLibs[{}] = {} ({} 张图像)", index, path_ref.display(), lib.count());
                self.map_libs[index] = Some(lib);
                self.loaded += 1;
            }
            Err(e) => {
                tracing::warn!("✗ MapLibs[{}] = {} 失败: {}", index, path_ref.display(), e);
                self.map_libs[index] = None;
            }
        }
    }

    fn init_wemade_mir2_maps(&mut self) {
        let base = self.data_path.join("Map/WemadeMir2");
        self.load_to_map_slot(0, base.join("Tiles"));
        self.load_to_map_slot(1, base.join("Smtiles"));
        self.load_to_map_slot(2, base.join("Objects"));
        for i in 2..28 {
            self.load_to_map_slot(i + 1, base.join(format!("Objects{}", i)));
        }
        self.load_to_map_slot(90, base.join("Objects_32bit"));
    }

    fn init_shanda_mir2_maps(&mut self) {
        let base = self.data_path.join("Map/ShandaMir2");
        self.load_to_map_slot(100, base.join("Tiles"));
        for i in 1..10 {
            self.load_to_map_slot(100 + i, base.join(format!("Tiles{}", i + 1)));
        }
        self.load_to_map_slot(110, base.join("SmTiles"));
        for i in 1..10 {
            self.load_to_map_slot(110 + i, base.join(format!("SmTiles{}", i + 1)));
        }
        self.load_to_map_slot(120, base.join("Objects"));
        for i in 1..31 {
            self.load_to_map_slot(120 + i, base.join(format!("Objects{}", i + 1)));
        }
        self.load_to_map_slot(190, base.join("AniTiles1"));
    }

    fn init_wemade_mir3_maps(&mut self) {
        let base = self.data_path.join("Map/WemadeMir3");
        let map_states = ["", "wood/", "sand/", "snow/", "forest/"];
        for (state_idx, state) in map_states.iter().enumerate() {
            let state_base = base.join(state);
            let offset = 200 + (state_idx * 15);
            let tiles = [
                "Tilesc",
                "Tiles30c",
                "Tiles5c",
                "Smtilesc",
                "Housesc",
                "Objects0c",
                "Objects1c",
                "Objects2c",
                "Objects3c",
                "Objects4c",
                "Objects5c",
                "Objects6c",
                "Objects7c",
                "Objects8c",
                "Objects9c",
            ];
            for (i, t) in tiles.iter().enumerate() {
                self.load_to_map_slot(offset + i, state_base.join(t));
            }
        }
    }

    fn init_shanda_mir3_maps(&mut self) {
        let base = self.data_path.join("Map/ShandaMir3");
        // 与 C# 一致: 300 = Tiles, 301-309 = Tiles2-10, 310 = SmTiles,
        // 311-319 = SmTiles2-10, 320 = Objects, 321-350 = Objects2-31
        self.load_to_map_slot(300, base.join("Tiles"));
        for i in 1..10 {
            self.load_to_map_slot(300 + i, base.join(format!("Tiles{}", i + 1)));
        }
        self.load_to_map_slot(310, base.join("SmTiles"));
        for i in 1..10 {
            self.load_to_map_slot(310 + i, base.join(format!("SmTiles{}", i + 1)));
        }
        self.load_to_map_slot(320, base.join("Objects"));
        for i in 1..31 {
            self.load_to_map_slot(320 + i, base.join(format!("Objects{}", i + 1)));
        }
    }

    // ===== 数组库（角色/怪物/NPC，懒加载） =====

    /// 获取数组库（不存在/未加载返回 None）
    pub fn get_array_lib(&self, ty: ArrayLibType, index: usize) -> Option<&MLibrary> {
        self.array_libs.get(&ty)?.get(index)?.as_ref()
    }

    /// 加载数组库到指定槽位（懒加载：只有实际用到才打开文件）
    fn ensure_array_lib(&mut self, ty: ArrayLibType, index: usize) -> Option<()> {
        let slot = self
            .array_libs
            .entry(ty)
            .or_default();
        if index >= slot.len() {
            slot.resize_with(index + 1, || None);
        }
        if slot[index].is_some() {
            return Some(());
        }
        let path = self.data_path.join(ty.default_path(index));
        match MLibrary::open(&path) {
            Ok(lib) => {
                tracing::debug!("✓ {}[{}] = {} ({} 张图像)", ty, index, path.display(), lib.count());
                slot[index] = Some(lib);
                self.loaded += 1;
                Some(())
            }
            Err(e) => {
                tracing::warn!("✗ {}[{}] = {} 失败: {}", ty, index, path.display(), e);
                None
            }
        }
    }

    /// 获取数组库图像（懒加载 + 解压 RGBA）
    pub fn get_array_image(
        &mut self,
        ty: ArrayLibType,
        index: usize,
        image_index: usize,
    ) -> Option<ImageInfo> {
        self.try_get_array_image(ty, index, image_index).ok()
    }

    /// 获取数组库图像（返回具体错误，用于诊断）
    pub fn get_array_image_debug(
        &mut self,
        ty: ArrayLibType,
        index: usize,
        image_index: usize,
    ) -> Result<ImageInfo, String> {
        self.try_get_array_image(ty, index, image_index)
    }

    fn try_get_array_image(
        &mut self,
        ty: ArrayLibType,
        index: usize,
        image_index: usize,
    ) -> Result<ImageInfo, String> {
        self.ensure_array_lib(ty, index)
            .ok_or_else(|| format!("{}[{}] 加载失败", ty, index))?;
        let slot = self.array_libs.get_mut(&ty).unwrap();
        let lib = slot[index].as_mut().unwrap();
        let count = lib.count();
        lib.get_or_load_image(image_index)
            .cloned()
            .map_err(|e| format!("{}[{}] idx {} (count {}): {}", ty, index, image_index, count, e))
    }

    /// 获取单体库
    pub fn get_library(&self, name: LibraryName) -> Option<&MLibrary> {
        self.libraries.get(&name)
    }

    /// 获取地图库
    pub fn get_map_library(&self, index: i16) -> Option<&MLibrary> {
        if !(0..400).contains(&index) {
            return None;
        }
        self.map_libs[index as usize].as_ref()
    }

    /// 获取地图图像（加载并解压 RGBA 数据）
    ///
    /// 注意：image_index 是库内图像索引（可能远大于 400），
    /// 边界由 MLibrary::get_or_load_image 校验，不要用 map_libs.len() 限制。
    pub fn get_map_image(&mut self, file_index: i16, image_index: i32) -> Option<ImageInfo> {
        self.try_get_map_image(file_index, image_index).ok()
    }

    /// 获取地图图像（返回具体错误，用于诊断）
    pub fn get_map_image_debug(
        &mut self,
        file_index: i16,
        image_index: i32,
    ) -> Result<ImageInfo, String> {
        self.try_get_map_image(file_index, image_index)
    }

    fn try_get_map_image(
        &mut self,
        file_index: i16,
        image_index: i32,
    ) -> Result<ImageInfo, String> {
        if !(0..400).contains(&file_index) {
            return Err(format!("file_index {} out of range", file_index));
        }
        if image_index < 0 {
            return Err(format!("image_index {} < 0", image_index));
        }
        let idx = image_index as usize;
        let slot = self
            .map_libs
            .get_mut(file_index as usize)
            .ok_or_else(|| format!("MapLibs[{}] not allocated", file_index))?
            .as_mut()
            .ok_or_else(|| format!("MapLibs[{}] not loaded", file_index))?;
        let count = slot.count();
        slot.get_or_load_image(idx)
            .cloned()
            .map_err(|e| format!("MapLibs[{}] idx {} (count {}): {}", file_index, idx, count, e))
    }

    /// 获取单体库图像
    pub fn get_image(&mut self, name: LibraryName, index: usize) -> Option<ImageInfo> {
        let lib = self.libraries.get_mut(&name)?;
        lib.get_or_load_image(index).ok().cloned()
    }

    /// 加载统计
    pub fn stats(&self) -> (usize, usize) {
        let map = self.map_libs.iter().filter(|l| l.is_some()).count();
        (self.libraries.len(), map)
    }
}
