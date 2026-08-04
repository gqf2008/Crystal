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
    map_tile_anim_system, register_blend_material, spawn_anim_tile, spawn_blend_tile,
    MapAnimClock, MapBlendMaterial, TileAnimKind, TileImageCache,
};
use crate::resources::libraries::Libraries;
use crate::resources::map_reader::{resolve_map_path, CellInfo, MapReader};
use crate::resources::mlibrary::ImageInfo;

// #72 拆分：chunks.rs（Front 块生成/流式）、chunks_build.rs（合成/setup）、camera.rs（相机）
mod camera;
mod chunks;
mod chunks_build;

pub use chunks_build::{build_chunk_rgba, make_image};
use camera::{camera_control, camera_follow_system, map_layer_toggle_system, spawn_camera};
use chunks::{chunk_stream_system, spawn_front_chunk};
use chunks_build::setup_world;

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
        Self { back: true, middle: true, front: true, anim: true }
    }
}
#[derive(Component)]
pub struct MapFloorMark(pub Layer);

/// 地图灯光（C# DrawLights Map Lights：cell.Light 1..9，白色径向渐变，ADD 混合）
#[derive(Component)]
pub struct MapLight;

/// C# DXManager.Lights[i] = LightSizes[i+1]（径向渐变光斑尺寸，索引 0..9）
pub const LIGHT_SIZES: [(f32, f32); 10] = [
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
            let vals = [1.0f32, 210.0 / 255.0, 160.0 / 255.0, 70.0 / 255.0, 40.0 / 255.0, 0.0];
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
