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
    /// 刺客职业武器右/左：`AWeapon/{:02} R.Lib` / `AWeapon/{:02} L.Lib`
    /// （C# `PlayerObject.cs:625-626`：`Index = Weapon - 100`，WeaponLibrary1=AWeaponsR、WeaponLibrary2=AWeaponsL）
    AWeaponsR,
    AWeaponsL,
    /// 弓箭手职业武器：`ARWeapon/{:02}.Lib`（站立/行走以外动作）与 `ARWeapon/{:02} S.Lib`
    /// （altAnim = 走路/奔跑/AttackRange1-2 时；C# `PlayerObject.cs:531-533`，`Index = Weapon - 200`）
    ARWeapons,
    ARWeaponsS,
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
            ArrayLibType::AWeaponsR => "AWeaponsR",
            ArrayLibType::AWeaponsL => "AWeaponsL",
            ArrayLibType::ARWeapons => "ARWeapons",
            ArrayLibType::ARWeaponsS => "ARWeaponsS",
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
            // 资产实测命名（`Data/AWeapon/00 L.Lib`、`Data/ARWeapon/00 S.Lib`）
            ArrayLibType::AWeaponsR => format!("AWeapon/{:02} R", index),
            ArrayLibType::AWeaponsL => format!("AWeapon/{:02} L", index),
            ArrayLibType::ARWeapons => format!("ARWeapon/{:02}", index),
            ArrayLibType::ARWeaponsS => format!("ARWeapon/{:02} S", index),
        }
    }
}

/// 武器外观层计划（纯函数，门禁可测）：照 C# `PlayerObject.cs:97-108 / 524-535 / 620-627`。
///
/// 规则（`Globals.ClassWeaponCount = 100`，`HasClassWeapon` 还要**职业匹配**）：
/// - `0..99`   → 战士/法师/道士默认武器：`CWeapon/{shape:02}`（C# `CWeapons[Weapon]`）
/// - `100..199`→ 刺客：`AWeapon/{shape-100} R` + `AWeapon/{shape-100} L`（左右两层）
/// - `200..299`→ 弓箭手：`ARWeapon/{shape-200}` 或 altAnim（走/跑/远程攻击）时 `ARWeapon/{shape-200} S`
/// - 职业与区间不匹配时，退回 `CWeapon/{shape:02}`（C# 走 `else` 分支；超范围时该库不存在 → 不画）
///
/// 为什么要有这条门禁（2026-09-24 定性）：本端 `ArrayLibType` 此前**只有 CWeapons**，
/// 而库里 63 个 `shape 100..199` 的物品（HoaSword/MirSword4/… 刺客武器）会被当成
/// `CWeapon/100..152` 去加载 —— 目录里只有 `00..78` ⇒ 贴图直接缺失（owner 报的
/// 「CWeapons[793] 贴图缺失」就是这一类：拿 shape/索引去开不存在的职业武器库）。
pub fn weapon_layer_plan(
    class: mir2_shared::enums::MirClass,
    shape: i16,
    alt_anim: bool,
) -> Vec<(ArrayLibType, u32)> {
    use mir2_shared::enums::MirClass;
    if shape < 0 {
        return Vec::new();
    }
    let shape = shape as u32;
    let class_matches = match shape / 100 {
        0 => matches!(
            class,
            MirClass::Warrior | MirClass::Wizard | MirClass::Taoist
        ),
        1 => matches!(class, MirClass::Assassin),
        2 => matches!(class, MirClass::Archer),
        _ => false,
    };
    if class_matches {
        match shape / 100 {
            1 => {
                let index = shape - 100;
                return vec![
                    (ArrayLibType::AWeaponsR, index),
                    (ArrayLibType::AWeaponsL, index),
                ];
            }
            2 => {
                let index = shape - 200;
                let lib = if alt_anim {
                    ArrayLibType::ARWeaponsS
                } else {
                    ArrayLibType::ARWeapons
                };
                return vec![(lib, index)];
            }
            _ => {}
        }
    }
    vec![(ArrayLibType::CWeapons, shape)]
}

impl std::fmt::Display for ArrayLibType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 解析 Data 根目录。
///
/// 优先使用本 crate 的 Data/，其次仓库根 Data/（游戏数据在仓库根，本地保留不入库）。
pub fn resolve_data_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // #2809：`CRYSTAL_NO_DATA_ASSETS=1` → 返回一个不存在的目录，忠实复现 CI（只 checkout
    // 仓库、无 `Data/`）的无资产环境。若只让 `data_assets_present()` 说谎、这里仍能读到真实
    // 资产，则"跳过判据"无法红检（去掉判据也不会 FAILED）。
    if std::env::var_os("CRYSTAL_NO_DATA_ASSETS").is_some() {
        return PathBuf::from(format!("{}/Data.crystal_no_assets", manifest_dir));
    }
    // 运行时候选（cwd / exe 相对）：worktree 构建的 exe 共享主检出 target 目录时，
    // CARGO_MANIFEST_DIR 是编译期常量、指向 worktree（无 Data 资产，gitignore 不入库），
    // 曾导致地图地面 0 瓦片全黑屏（#2599 排查记录）。运行时路径按启动环境解析，
    // 从主检出 cwd 启动的 worktree exe 也能找到正确 Data。
    let runtime_candidates = || -> Vec<PathBuf> {
        let mut v = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            v.push(cwd.join("Data"));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join("Data"));
            }
        }
        v
    };
    let mut candidates: Vec<String> = runtime_candidates()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    candidates.extend([
        format!("{}/Data", manifest_dir),
        format!("{}/../Data", manifest_dir),
        format!("{}/../../Crystal/Data", manifest_dir),
    ]);
    // 要求目录内确实存在 .Lib 数据（Items.Lib 是核心库）。
    // Data 在仓库根（本地保留，gitignore 不入库），独立 worktree 会回落到主仓库数据目录。
    for c in &candidates {
        let p = Path::new(c);
        if p.join("Items.Lib").exists() {
            return p.to_path_buf();
        }
    }
    PathBuf::from(format!("{}/Data", manifest_dir))
}

/// 游戏资产（`Data/`，核心标志 `Items.Lib`）是否可用。
///
/// CI 只 checkout 仓库：`Data/` 是本地保留、不入库的游戏资源，因此依赖真实 `.Lib` 精灵的
/// 单测应据此**跳过**而不是 FAILED（本函数就是那份判据）。
/// 设 `CRYSTAL_NO_DATA_ASSETS=1` 可强制判定为"无资产"，用于在本机复现 CI 的跳过路径。
pub fn data_assets_present() -> bool {
    if std::env::var_os("CRYSTAL_NO_DATA_ASSETS").is_some() {
        return false;
    }
    resolve_data_path().join("Items.Lib").exists()
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
                tracing::debug!(
                    "✓ MapLibs[{}] = {} ({} 张图像)",
                    index,
                    path_ref.display(),
                    lib.count()
                );
                self.map_libs[index] = Some(lib);
                self.loaded += 1;
            }
            Err(e) => {
                // 缺失的地图库为可选项（真实服务器 400 个地图配置，客户端不加载全部）
                // 降为 debug 避免刷屏；汇总行 "x/400 已加载" 已给出整体情况
                tracing::debug!("✗ MapLibs[{}] = {} 失败: {}", index, path_ref.display(), e);
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
            // 与 C# MLibrary.cs 一致（WemadeMir3 每状态 14 个库）：
            // Tilesc/Tiles30c/Tiles5c/Smtilesc/Housesc/Cliffsc/Dungeonsc/
            // Innersc/Furnituresc/Wallsc/smObjectsc/Animationsc/Object1c/Object2c
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
                "SmObjectsc",
                "Animationsc",
                "Object1c",
                "Object2c",
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
        let slot = self.array_libs.entry(ty).or_default();
        if index >= slot.len() {
            slot.resize_with(index + 1, || None);
        }
        if slot[index].is_some() {
            return Some(());
        }
        let path = self.data_path.join(ty.default_path(index));
        match MLibrary::open(&path) {
            Ok(lib) => {
                tracing::debug!(
                    "✓ {}[{}] = {} ({} 张图像)",
                    ty,
                    index,
                    path.display(),
                    lib.count()
                );
                slot[index] = Some(lib);
                self.loaded += 1;
                Some(())
            }
            Err(e) => {
                // 缺失的资源库降为 debug：真实服务器对象（武器/护甲索引）本地 Data 可能没有，
                // 缺库时该层不渲染即可，避免每帧重复 warn 刷屏
                tracing::debug!("✗ {}[{}] = {} 失败: {}", ty, index, path.display(), e);
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
        lib.get_or_load_image(image_index).cloned().map_err(|e| {
            format!(
                "{}[{}] idx {} (count {}): {}",
                ty, index, image_index, count, e
            )
        })
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
        slot.get_or_load_image(idx).cloned().map_err(|e| {
            format!(
                "MapLibs[{}] idx {} (count {}): {}",
                file_index, idx, count, e
            )
        })
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

#[cfg(test)]
mod weapon_plan_tests {
    use super::*;
    use mir2_shared::enums::MirClass;

    /// 门禁（2026-09-24 定性 owner 报的「CWeapons[793] 贴图缺失」）：**职业武器必须走各自的库**——
    /// 刺客 `shape 100..199` → `AWeapon/{idx} R`+`L`、弓箭手 `shape 200..299` → `ARWeapon/{idx}`（altAnim 用 `S`），
    /// 不能再落到 `CWeapon/{shape}`（目录只有 `00..78`，于是贴图直接缺失）。
    ///
    /// 阳性对照：把 `weapon_layer_plan` 里 `class_matches` 的 1/2 两个分支删掉（退回 CWeapons）→ 本测试立即红。
    #[test]
    fn weapon_layer_plan_matches_csharp_class_weapons() {
        // 默认武器（战/法/道）仍走 CWeapons
        assert_eq!(
            weapon_layer_plan(MirClass::Warrior, 0, false),
            vec![(ArrayLibType::CWeapons, 0)]
        );
        assert_eq!(
            weapon_layer_plan(MirClass::Taoist, 19, false),
            vec![(ArrayLibType::CWeapons, 19)]
        );
        // 刺客 100..199：右/左两层（C# WeaponLibrary1=AWeaponsR、WeaponLibrary2=AWeaponsL）
        assert_eq!(
            weapon_layer_plan(MirClass::Assassin, 100, false),
            vec![(ArrayLibType::AWeaponsR, 0), (ArrayLibType::AWeaponsL, 0)]
        );
        assert_eq!(
            weapon_layer_plan(MirClass::Assassin, 152, false)[0],
            (ArrayLibType::AWeaponsR, 52)
        );
        // 弓箭手 200..299：站立用 ARWeapons，altAnim（走/跑/远程攻击）用 ARWeaponsS
        assert_eq!(
            weapon_layer_plan(MirClass::Archer, 200, false),
            vec![(ArrayLibType::ARWeapons, 0)]
        );
        assert_eq!(
            weapon_layer_plan(MirClass::Archer, 205, true),
            vec![(ArrayLibType::ARWeaponsS, 5)]
        );
        // 职业与区间不匹配 → 退回 CWeapons（C# 的 else 分支）
        assert_eq!(
            weapon_layer_plan(MirClass::Warrior, 100, false),
            vec![(ArrayLibType::CWeapons, 100)]
        );
        // 空/非法
        assert!(weapon_layer_plan(MirClass::Assassin, -1, false).is_empty());
        // 库路径命名照资产实测（Data/AWeapon/00 L.Lib、Data/ARWeapon/00 S.Lib）
        assert_eq!(ArrayLibType::AWeaponsL.default_path(3), "AWeapon/03 L");
        assert_eq!(ArrayLibType::AWeaponsR.default_path(3), "AWeapon/03 R");
        assert_eq!(ArrayLibType::ARWeapons.default_path(1), "ARWeapon/01");
        assert_eq!(ArrayLibType::ARWeaponsS.default_path(1), "ARWeapon/01 S");
    }
}
