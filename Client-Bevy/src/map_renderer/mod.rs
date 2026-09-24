// ============================================================================
// map_renderer 模块拆分（#72）
// ============================================================================

// ============================================================================
// MapRenderPlugin - Bevy 地图渲染（里程碑 1）
// ============================================================================
//
// 把 Client-Macroquad 的 MeshMapRenderer 移植为 Bevy 渲染：
// - 每 32x32 格合成一张块纹理（1536x1024），按 Back/Middle/Front 三层分层
// - 每个块生成一个 Sprite，Bevy 自动做视锥剔除
// - 坐标约定与 macroquad 一致：世界 x 向右、y 向下（屏幕空间），
//   sprite 位置做 y 取反以适配 Bevy 的 y 向上坐标系

use bevy::asset::RenderAssetUsages;
use bevy::camera::{OrthographicProjection, Projection};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::map_tile_anim::{
    map_tile_anim_system, register_blend_material, spawn_anim_tile, spawn_blend_tile, MapAnimClock,
    MapBlendMaterial, TileAnimKind, TileImageCache,
};
use crate::resources::libraries::Libraries;
use crate::resources::map_reader::{resolve_map_path, CellInfo, MapReader};
use crate::resources::mlibrary::ImageInfo;

// #72 拆分：chunks.rs（Front 块生成/流式）、chunks_build.rs（合成/setup）、camera.rs（相机）
mod camera;
mod chunks;
mod chunks_build;

use camera::{camera_control, camera_follow_system, map_layer_toggle_system, spawn_camera};
use chunks::{chunk_stream_system, spawn_front_chunk};
pub(crate) use chunks_build::map_rebuild_system;
pub use chunks_build::{build_chunk_rgba, make_image};
use chunks_build::{cleanup_map_world, setup_world};

/// 瓦片尺寸（与 macroquad 版一致）
pub const TILE_WIDTH: f32 = 48.0;
pub const TILE_HEIGHT: f32 = 32.0;
/// 每个块包含的瓦片数
pub const CHUNK_TILES: u32 = 32;
/// 块纹理尺寸
pub const CHUNK_PIXEL_W: u32 = CHUNK_TILES * TILE_WIDTH as u32; // 1536
pub const CHUNK_PIXEL_H: u32 = CHUNK_TILES * TILE_HEIGHT as u32; // 1024

/// Y 深度函数：所有角色与 front 瓦片共用，按世界 Y（屏幕向下）交错排序。
/// front 瓦片基准 = 格子底边 (y+1)*32，角色基准 = 脚底位置。
/// 基准越大（越靠下）z 越大（越靠前），实现经典传奇遮挡。
pub fn depth_y(world_y_screen_down: f32) -> f32 {
    0.2 + world_y_screen_down * 0.00001
}

/// Front 瓦片标记：记录世界矩形（屏幕向下坐标）与基准 Y，
/// 用于深度排序与本地玩家遮挡检测
#[derive(Component)]
pub struct FrontTile {
    pub base_y: f32,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// 图层显隐调试（热键 1=Back 2=Middle 3=Front静态 F=动画/混合）
#[derive(Resource)]
pub struct MapLayerShow {
    pub back: bool,
    pub middle: bool,
    pub front: bool,
    pub anim: bool,
}
impl Default for MapLayerShow {
    fn default() -> Self {
        Self {
            back: true,
            middle: true,
            front: true,
            anim: true,
        }
    }
}
#[derive(Component)]
pub struct MapFloorMark(pub Layer);

/// 地图灯光（C# DrawLights Map Lights：cell.Light 1..9，白色径向渐变，ADD 混合）
#[derive(Component)]
pub struct MapLight;

/// 灯光所属 chunk（#88：灯光随相机流式加载/卸载，与 front 瓦片一致）
#[derive(Component, Clone, Copy)]
pub struct LightChunkKey(pub i32, pub i32);

/// 灯光径向渐变纹理（setup_world 创建一次，流式生成复用）
#[derive(Resource)]
pub struct MapLightTexture(pub Handle<Image>);

/// C# DrawLights 的 p.X 比 DrawObjects 的 drawX 多 OffSetX（GameScene.cs:10398），
/// OffSetX = ScreenWidth / 2 / CellWidth = 1024 / 2 / 48 = 10。
/// 光斑 x 必须补上 OffSetX 才能与路灯（blend 2723..=2732）精确对齐（#88）。
pub const LIGHT_SCREEN_OFFSET_X: f32 = 10.0;

/// 原版 `DXManager.LightSizes`（`Client/MirGraphics/DXManager.cs:43-56`）**逐项照抄**：
/// **11** 项、index 0 = (125,95)。
///
/// **警告：它不是「绘制尺寸表」。** 原版 `DXManager.CreateLights()` 的循环是
/// `for (int i = 1; i < LightSizes.Length; i++) Lights.Add(...)`（`DXManager.cs:160-215`）
/// ⇒ `Lights[j]` 的**纹理尺寸 = LightSizes[j + 1]**（`Lights.Count == 10`）；
/// 而 Map Lights 的**偏移**用的是 `LightSizes[li]`（`GameScene.cs:11257`）。
/// 两者差一格 ⇒ 光斑中心比「按同一张表抵消」多出 `(S[li+1] − S[li]) / 2 = (40, 30.5)`；
/// 同时绘制尺寸必须取 `S[li+1]`，否则每档小一格（li=9：845×642 vs 原版 925×703）。
/// 取绘制尺寸一律走 [`light_tex_size`]，**不要**直接索引本表。
pub const LIGHT_SIZES: [(f32, f32); 11] = [
    (125.0, 95.0),
    (205.0, 156.0),
    (285.0, 217.0),
    (365.0, 277.0),
    (445.0, 338.0),
    (525.0, 399.0),
    (605.0, 460.0),
    (685.0, 521.0),
    (765.0, 581.0),
    (845.0, 642.0),
    (925.0, 703.0),
];

/// C# `DXManager.Lights[li]` 的**纹理尺寸** = `LightSizes[li + 1]`
/// （`CreateLights` 的循环起点 `i = 1`，`Client/MirGraphics/DXManager.cs:160-215`）。
#[must_use]
pub fn light_tex_size(li: usize) -> (f32, f32) {
    LIGHT_SIZES[li.saturating_add(1).min(LIGHT_SIZES.len() - 1)]
}

/// 原版排版下「纹理半宽 − 偏移里的半宽」= `(S[li+1] − S[li]) / 2`（**逐档不同**：
/// x 侧恒 40，y 侧在 30.0/30.5 之间交替——`LightSizes` 的 y 步进是 61/60 交替）。
#[must_use]
pub fn light_center_radius_diff(li: usize) -> (f32, f32) {
    let i = li.min(LIGHT_SIZES.len() - 2);
    (
        (LIGHT_SIZES[i + 1].0 - LIGHT_SIZES[i].0) / 2.0,
        (LIGHT_SIZES[i + 1].1 - LIGHT_SIZES[i].1) / 2.0,
    )
}

/// 地图灯光的中心（世界坐标，Bevy 约定；原版 `GameScene.cs:11245-11257` 的 Map Lights 段）：
///
/// ```text
/// p = (x * CellWidth, (y + 1) * CellHeight)          // 格左缘 / 格底缘（+32）
/// if (FrontAnimationFrame > 0) p += (off_x, off_y)   // front 动画格叠加库偏移
/// p.Offset(-(LightSizes[li].W / 2) - (CellWidth / 2) + 10,
///          -(LightSizes[li].H / 2) - (CellHeight / 2) - 5)
/// Draw(DXManager.Lights[li], ..., p)   // 纹理左上角落在 p ⇒ 中心 = p + (纹理尺寸 / 2)
/// ```
///
/// **偏移的表与绘制的纹理不是同一格**：偏移用 `LightSizes[li]`，纹理用 `Lights[li] = LightSizes[li+1]`
/// （见 [`light_tex_size`]）⇒ 中心 = `p + (−S[li]/2 − 格/2 + (10, −5)) + S[li+1]/2`，
/// 即比「按同一张表抵消」多出 [`light_center_radius_diff`]（x 恒 +40；y 在 +30.0/+30.5 之间交替——表的 y 步进是 61/60 交替）：
///
/// ```text
/// 中心（屏幕，y 向下） = (x*CellWidth + off_x − S[li].W/2 − 24 + 10 + S[li+1].W/2,
///                       (y+1)*CellHeight + off_y − S[li].H/2 − 16 − 5 + S[li+1].H/2)
/// ```
///
/// 历史坑（两次都出在同一处）：① 早期把 `-CellWidth/2` 写成 `-14` 又额外加 +10（多算 10px）；
/// ② #3053 把表补成原版 11 项却**只改了表**，消费端仍写 `LIGHT_SIZES[li]`
/// ⇒ 绘制尺寸比原版小一格、中心少 (40, 30/30.5)。两处都由本函数与 [`light_tex_size`] 收口。
pub fn light_center(cell_x: usize, cell_y: usize, off_x: f32, off_y: f32, li: usize) -> (f32, f32) {
    let (shift_x, shift_y) = light_center_radius_diff(li);
    let x = cell_x as f32 * TILE_WIDTH as f32 + off_x - TILE_WIDTH as f32 / 2.0
        + LIGHT_SCREEN_OFFSET_X
        + shift_x;
    let y = -((cell_y + 1) as f32 * TILE_HEIGHT as f32 + off_y - TILE_HEIGHT as f32 / 2.0 - 5.0
        + shift_y);
    (x, y)
}

/// C# `GameScene.DrawLights` → `#region Map Lights` 的**前置条件**（与 MapControl/MapCode 同源）：
/// 只有**该格有 Front 图**时才画地图灯光——
/// ```text
/// int imageIndex = (M2CellInfo[x, y].FrontImage & 0x7FFF) - 1;
/// if (imageIndex == -1) continue;      // 没有前景图 → 不画
/// int fileIndex = M2CellInfo[x, y].FrontIndex;
/// if (fileIndex == -1) continue;       // 前景库缺失 → 不画
/// ```
/// **两个灯光生成路径（首帧构建 `chunks_build` 与 chunk 流式 `chunks`）都必须过这一关**：
/// 漏掉它会在没有前景的格子上多画一圈光斑 —— owner 2026-09-24 反馈的「部分地图灯光错位」
/// 就是这个（实测 `0.map` 99 个灯格里有 46 个无 Front 图、`2.map` 9 个里有 8 个）。
pub fn map_light_on_cell(cell: &crate::resources::map_reader::CellInfo) -> bool {
    (cell.front_image & 0x7FFF) != 0 && cell.front_index != -1
}

/// 生成 C# DXManager.CreateLights 同款径向渐变纹理（白心 → 边缘透明）
pub fn make_light_texture(assets: &mut Assets<Image>, size: u32) -> Handle<Image> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let r = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - r;
            let dy = y as f32 + 0.5 - r;
            let d = (dx * dx + dy * dy).sqrt() / r;
            let t = d.clamp(0.0, 1.0);
            // C# ColorBlend: 1.0, 210/255, 160/255, 70/255, 40/255, 0 at 0,.2,.4,.6,.8,1.0
            let stops = [0.0f32, 0.2, 0.4, 0.6, 0.8, 1.0];
            let vals = [
                1.0f32,
                210.0 / 255.0,
                160.0 / 255.0,
                70.0 / 255.0,
                40.0 / 255.0,
                0.0,
            ];
            let mut a = 0.0f32;
            for i in 0..5 {
                if t >= stops[i] && t <= stops[i + 1] {
                    let k = (t - stops[i]) / (stops[i + 1] - stops[i]);
                    a = vals[i] + (vals[i + 1] - vals[i]) * k;
                    break;
                }
            }
            let idx = ((y * size + x) * 4) as usize;
            let v = (a * 255.0).round() as u8;
            // RGB 恒定白 + alpha 渐变：Bevy 灯光走标准 alpha 混合（Material2d
            // specialize 自定义 blend 不生效），白心亮、边缘透明，等效 C# ADD 且无暗圈
            rgba[idx] = 255;
            rgba[idx + 1] = 255;
            rgba[idx + 2] = 255;
            rgba[idx + 3] = v;
        }
    }
    let mut img = make_image(rgba, size, size);
    img.sampler = bevy::image::ImageSampler::linear();
    assets.add(img)
}

/// 已生成的地板块 key（流式加载/卸载用）
#[derive(Component)]
pub struct ChunkKey(pub i32, pub i32, pub Layer);

/// Middle 大图对象（大树/建筑，setup_world 对象层）标记：
/// 对象无 chunk 归属（跨块底边对齐单独画），靠本标记供 OnExit/换图统一清理（S2/B1）
#[derive(Component)]
pub struct MapMiddleObject;

/// Front 层精灵所属 chunk（流式加载/卸载用，#31 性能）
#[derive(Component)]
pub struct FrontChunkKey(pub i32, pub i32);

/// Front 贴图去重缓存（跨 chunk 共享 Image 资产，避免重复创建）
#[derive(Resource, Default)]
pub struct FrontImageCache(pub std::collections::HashMap<(i16, i32), (Handle<Image>, i16, i16)>);

/// chunk 流式状态
#[derive(Resource, Default)]
pub struct ChunkStream {
    pub last_cam_chunk: Option<(i32, i32)>,
}

/// 游戏数据资源：当前地图信息
#[derive(Resource, Default)]
pub struct GameData {
    pub map: Option<LoadedMap>,
    /// 地图解析器（供 chunk 流式按需加载）
    pub map_reader: Option<std::sync::Arc<MapReader>>,
    /// 网络 MapChanged 指定的地图名（优先于命令行 --map）
    pub desired_map: Option<String>,
    /// 玩家出生位置（瓦片坐标 + 朝向），来自 MapChanged
    pub player_spawn: Option<(f32, f32, u8)>,
}

/// 图像库资源（地图库 + 数组库，供渲染系统使用；懒初始化）
#[derive(Resource)]
pub struct GameLibraries(pub Libraries);

impl Default for GameLibraries {
    fn default() -> Self {
        // 路径由 ensure_initialized 在首次使用时修正
        Self(Libraries::new("Data"))
    }
}

/// 已加载地图
pub struct LoadedMap {
    pub name: String,
    pub width: i32,
    pub height: i32,
    /// 可行走网格（M8 寻路用；back_image 障碍标志位）
    /// #1550：门索引网格（C# M2CellInfo.DoorIndex；0=无门）
    pub doors: Vec<Vec<u8>>,
    pub walkable: Vec<Vec<bool>>,
}

impl LoadedMap {
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if !self.in_bounds(x, y) {
            return false;
        }
        self.walkable[x as usize][y as usize]
    }
}

/// 图层
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    Back,
    Middle,
    Front,
}

impl Layer {
    fn z(self) -> f32 {
        match self {
            Layer::Back => 0.0,
            Layer::Middle => 0.1,
            Layer::Front => 0.2,
        }
    }

    fn tile(self, cell: &CellInfo) -> Option<(i16, i32)> {
        match self {
            Layer::Back => cell.back_tile(),
            Layer::Middle => cell.middle_tile(),
            Layer::Front => cell.front_tile(),
        }
    }
}

pub struct MapRenderPlugin;

impl Plugin for MapRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameData>();
        app.init_resource::<GameLibraries>();
        app.add_systems(Startup, spawn_camera);
        app.init_resource::<MapLayerShow>();
        app.init_resource::<ChunkStream>();
        app.init_resource::<FrontImageCache>();
        app.init_resource::<MapAnimClock>();
        app.init_resource::<TileImageCache>();
        register_blend_material(app);
        app.add_systems(OnEnter(crate::scenes::AppState::Game), setup_world);
        // S2：离开 Game（登出/断线回登录）统一清掉地图实体并重置流式游标——
        // 此前只有流式卸载，OnExit 无清理，重进游戏时旧块/灯光/大图全部残留叠加
        app.add_systems(OnExit(crate::scenes::AppState::Game), cleanup_map_world);
        // B1：游戏内收到 MapChanged 时 desired_map 变更 → 清旧世界并原地重建
        // （游戏内 MapChanged 走 next.set_if_neq(Game)，同态不写 Pending，
        // OnEnter 不会重跑，必须靠本系统消费 desired_map）
        // 排序锁（复核严重项）：actor/mod.rs 的网络对象生成链以 .after(本系统) 显式排序——
        // 同帧换图 + 新图 NetObject 到达时先清旧图、后建新图，防止新图 NPC 被幽灵清理误删
        app.add_systems(
            Update,
            map_rebuild_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(
            Update,
            map_layer_toggle_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(
            Update,
            map_tile_anim_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(
            Update,
            camera_follow_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(
            Update,
            camera_control.run_if(in_state(crate::scenes::AppState::Game)),
        );
        // 关键：chunk 流式（之前定义了但漏注册 → 走出初始窗口后地图空白/黑色）
        app.add_systems(
            Update,
            chunk_stream_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
    }
}

/// 命令行参数：--map <name>，默认 n0（新手村，macroquad map_viewer 同款地图）
fn map_arg() -> String {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--map")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "n0".to_string())
}

/// i32 向上取整除法（避免依赖不稳定的 int_roundings）
fn div_ceil_i32(a: i32, b: i32) -> i32 {
    if a % b == 0 {
        a / b
    } else {
        a / b + 1
    }
}

#[cfg(test)]
mod light_alignment_tests {
    use super::*;

    /// 门禁（owner 反馈「部分地图灯光错位」）：灯光中心必须与原版 C#「Map Lights」段同源。
    ///
    /// 原版 `Client/MirScenes/GameScene.cs:11245-11258`：
    ///   p = (x*CellWidth, (y+1)*CellHeight)（+front 动画偏移）
    ///   p.Offset(-LightSizes[li].W/2 - CellWidth/2 + 10, -LightSizes[li].H/2 - CellHeight/2 - 5)
    ///   Draw(DXManager.Lights[li], ..., p)   // 左上角落 p ⇒ 中心 = p + 纹理尺寸/2，
    ///                                        // 而 Lights[li] = LightSizes[li+1]（差一格）
    /// ⇒ 中心 = (x*48 + off_x - 24 + 10 + 40, -((y+1)*32 + off_y - 16 - 5 + 30.5))
    ///
    /// 阳性对照（三条，落地时实做，见测试体末尾注释）。**期望值在这里是手写常量**，
    /// 不复用被测函数里的表达式，避免「用被测常量断言被测常量」的自证门禁。
    #[test]
    fn light_center_matches_csharp_map_lights() {
        // li=0：半径差 (40, 30.5)
        //   格 (10,20) 无偏移 → x = 480 − 24 + 10 + 40 = 506；y = −(21*32 − 16 − 5 + 30.5) = −681.5
        assert_eq!(light_center(10, 20, 0.0, 0.0, 0), (506.0, -681.5));
        assert_eq!(light_center(0, 0, 10.0, -6.0, 0), (36.0, -35.5));
        assert_eq!(light_center(288, 616, -5.0, 3.0, 0), (13845.0, -19756.5));
        // li=2：x 步进仍 80（+40），y 步进是 60（217→277）⇒ 半径差 (40, 30.0)
        assert_eq!(light_center(10, 20, 0.0, 0.0, 2), (506.0, -681.0));
        // 中心相对格锚点（原版口径，li=0）：x = −24 + 10 + 40 = +26；y(屏幕向下) = −16 − 5 + 30.5 = +9.5
        let (ax, ay) = light_center(0, 0, 0.0, 0.0, 0);
        assert_eq!(ax, 26.0, "光斑中心须比格左缘右移 26px（= −24 + 10 + 40）");
        // 原版口径（屏幕 y 向下）：中心 = 格底缘 + 9.5 = 32 + 9.5 = 41.5；Bevy 里 y 取负。
        assert_eq!(
            -ay,
            TILE_HEIGHT as f32 + 9.5,
            "光斑中心须比格底缘下移 9.5px（= −16 − 5 + 30.5）"
        );
        // 阳性对照①（实做）：把 light_center 里的 `shift_x/shift_y` 去掉（= 回到 #3053 那版「w/h 抵消」）
        //   → 上面两条锚点断言立即红（x 少 40、y 少 30/30.5）。
        // 阳性对照②（实做）：把 `− TILE_WIDTH / 2.0` 改回旧的 `−14.0` → x 锚点变 486 而红（多算 10px 的老坑）。
    }

    /// 门禁：**有 light 但无 Front 图的格子不画灯**（C# Map Lights 的
    /// `imageIndex == -1 → continue` / `fileIndex == -1 → continue`）。
    ///
    /// 阳性对照（落地时实做）：把 `map_light_on_cell` 的前置条件删掉（恒 `true`）→ 本测试立即红。
    #[test]
    fn map_light_requires_front_image() {
        use crate::resources::map_reader::CellInfo;
        let mut cell = CellInfo::new();
        cell.light = 1;

        // ① 无 Front 图（`FrontImage & 0x7FFF == 0`）→ 不画（实机 census：0.map 46/99、2.map 8/9、
        //    D002.map 11/19 的灯格属于这一类；修复前它们都会多画一圈光斑）
        cell.front_image = 0;
        cell.front_index = 3;
        assert!(!map_light_on_cell(&cell), "无 Front 图的灯格不得画灯");

        // ② 前景库缺失（`FrontIndex == -1`）→ 不画
        cell.front_image = 5;
        cell.front_index = -1;
        assert!(!map_light_on_cell(&cell), "FrontIndex == -1 不得画灯");

        // ③ 正常前景格 → 画
        cell.front_image = 5;
        cell.front_index = 3;
        assert!(map_light_on_cell(&cell), "有 Front 图必须画灯");
    }

    /// 门禁：光斑尺寸表必须与原版 `DXManager.LightSizes` 逐项一致（**11** 项、index 0 = 125×95）。
    ///
    /// 阳性对照（实做）：把第 0 项删掉（恢复旧 10 项表）→ 本测试立即红。
    #[test]
    fn light_sizes_match_dxmanager_table() {
        let csharp = [
            (125.0, 95.0),
            (205.0, 156.0),
            (285.0, 217.0),
            (365.0, 277.0),
            (445.0, 338.0),
            (525.0, 399.0),
            (605.0, 460.0),
            (685.0, 521.0),
            (765.0, 581.0),
            (845.0, 642.0),
            (925.0, 703.0),
        ];
        assert_eq!(
            LIGHT_SIZES.len(),
            csharp.len(),
            "原版是 11 项（漏 index 0 会整体错位一格）"
        );
        for (i, (w, h)) in csharp.iter().enumerate() {
            assert_eq!(
                LIGHT_SIZES[i],
                (*w, *h),
                "第 {i} 项与原版 DXManager.LightSizes 不一致"
            );
        }
        // `li = (cell.Light % 10) * 3` 最大到 9，必须能索引到（原版 11 项保证）
        assert!(LIGHT_SIZES.len() > 9, "li 最大 9 必须可索引");
    }

    /// 门禁（「光斑小一号」的根因）：**绘制尺寸**必须取 `DXManager.Lights[li] = LightSizes[li + 1]`，
    /// 而不是 `LightSizes[li]`（#3053 只改了表、没改消费端，导致每档光斑小一格）。
    ///
    /// 期望值手写自原版 `CreateLights` 的循环语义（`i` 从 1 起 ⇒ `Lights[j] = LightSizes[j+1]`）。
    /// 阳性对照（实做）：把 `light_tex_size` 改回 `LIGHT_SIZES[li]` → 本测试立即红（每项小一格）。
    #[test]
    fn light_tex_size_matches_csharp_lights_table() {
        let csharp_lights = [
            (205.0, 156.0),
            (285.0, 217.0),
            (365.0, 277.0),
            (445.0, 338.0),
            (525.0, 399.0),
            (605.0, 460.0),
            (685.0, 521.0),
            (765.0, 581.0),
            (845.0, 642.0),
            (925.0, 703.0),
        ];
        assert_eq!(
            csharp_lights.len(),
            10,
            "原版 Lights.Count = LightSizes.Length - 1 = 10"
        );
        for (li, (w, h)) in csharp_lights.iter().enumerate() {
            assert_eq!(
                light_tex_size(li),
                (*w, *h),
                "li={li} 的绘制尺寸必须是原版 Lights[li] = LightSizes[li+1]"
            );
            assert_ne!(
                light_tex_size(li),
                LIGHT_SIZES[li],
                "li={li}：画到 LightSizes[li] 就是「小一格」——这正是 #3053 遗留的缺陷"
            );
        }
        // li 上限 9（`(Light % 10) * 3` 再 min(9)）→ 必须安全取到 10（原版 11 项表的最后一项）
        assert_eq!(light_tex_size(9), (925.0, 703.0));
        assert_eq!(
            light_tex_size(usize::MAX),
            (925.0, 703.0),
            "越界必须钳到最后一项"
        );
    }

    /// 门禁：`light_center_radius_diff(li)` 必须逐档 = 原版 `(S[li+1] − S[li]) / 2`
    /// （x 恒 40；y 在 30.0/30.5 交替 —— `LightSizes` 的 y 步进是 61/60 交替，
    /// 所以**中心补偿不是常数**，写死 30.5 会在 li=2/7 上偏 0.5px）。
    ///
    /// 阳性对照（实做）：把该函数改成返回常量 (40, 30.5) → li=2 的断言立即红。
    #[test]
    fn light_center_radius_diff_follows_light_sizes_table() {
        let csharp = [
            (125.0f32, 95.0f32),
            (205.0, 156.0),
            (285.0, 217.0),
            (365.0, 277.0),
            (445.0, 338.0),
            (525.0, 399.0),
            (605.0, 460.0),
            (685.0, 521.0),
            (765.0, 581.0),
            (845.0, 642.0),
            (925.0, 703.0),
        ];
        let mut y_variants = std::collections::BTreeSet::new();
        for li in 0..LIGHT_SIZES.len() - 1 {
            let expect = (
                (csharp[li + 1].0 - csharp[li].0) / 2.0,
                (csharp[li + 1].1 - csharp[li].1) / 2.0,
            );
            assert_eq!(
                light_center_radius_diff(li),
                expect,
                "li={li} 的半径差必须 = (LightSizes[li+1] − LightSizes[li]) / 2"
            );
            assert_eq!(expect.0, 40.0, "x 半径差恒 40");
            y_variants.insert((expect.1 * 2.0) as i32);
        }
        assert_eq!(
            y_variants,
            [60, 61].into_iter().collect(),
            "y 半径差必须出现 30.0 与 30.5 两种（表步进 61/60 交替）——写死常数会偏 0.5px"
        );
        // 越界钳位：li 上限 9，且 `li+1` 不能越出 11 项表
        assert_eq!(
            light_center_radius_diff(9),
            ((925.0 - 845.0) / 2.0, (703.0 - 642.0) / 2.0)
        );
        assert_eq!(
            light_center_radius_diff(usize::MAX),
            light_center_radius_diff(9)
        );
    }
}
