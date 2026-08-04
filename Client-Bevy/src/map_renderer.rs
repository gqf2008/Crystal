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
            rgba[idx] = v;
            rgba[idx + 1] = v;
            rgba[idx + 2] = v;
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

/// 生成一个 chunk 的 Front 层精灵（静态 + 动画 + blend），#31 流式用。
/// 所有实体打 `FrontChunkKey(cx, cy)` 标记，供 chunk_stream_system 按窗口加载/卸载。
#[allow(clippy::too_many_arguments)]
fn spawn_front_chunk(
    commands: &mut Commands,
    libraries: &mut Libraries,
    assets: &mut Assets<Image>,
    tile_cache: &mut TileImageCache,
    blend_materials: &mut Assets<MapBlendMaterial>,
    blend_quad: &Handle<Mesh>,
    front_images: &mut FrontImageCache,
    map: &MapReader,
    cx: i32,
    cy: i32,
) -> usize {
    let mut count = 0usize;
    let f_start_x = cx * CHUNK_TILES as i32;
    let f_start_y = cy * CHUNK_TILES as i32;
    let f_end_x = (f_start_x + CHUNK_TILES as i32).min(map.width);
    let f_end_y = (f_start_y + CHUNK_TILES as i32).min(map.height);
    for x in f_start_x..f_end_x {
        for y in f_start_y..f_end_y {
            let cell = &map.map_cells[x as usize][y as usize];
            if cell.front_animation_frame > 0 {
                // 动画/灯光混合瓦片：单独生成（blend → ADD 混合材质）
                if let Some((file_index, base_image_index)) = cell.front_tile() {
                    let mut animation = cell.front_animation_frame;
                    let blend = (animation & 0x80) > 0;
                    if blend {
                        animation &= 0x7F;
                    }
                    let tick = cell.front_animation_tick;
                    let base_y_world = (y + 1) as f32 * TILE_HEIGHT as f32;
                    let should_apply_offset = if blend {
                        (100..199).contains(&file_index)
                    } else {
                        file_index == 28
                    };
                    if let Some(info) = libraries.get_map_image(file_index, base_image_index) {
                        let off_x = if should_apply_offset { info.offset_x as f32 } else { 0.0 };
                        let off_y = if should_apply_offset { info.offset_y as f32 } else { 0.0 };
                        let left = x as f32 * TILE_WIDTH as f32 + off_x;
                        let (anchor_y, top_anchored) = if blend {
                            (-(base_y_world - 3.0 * TILE_HEIGHT as f32 + off_y), true)
                        } else {
                            (-(base_y_world + off_y), false)
                        };
                        if blend {
                            if let Some(e) = spawn_blend_tile(
                                commands, libraries, assets, tile_cache,
                                blend_materials, blend_quad.clone(),
                                TileAnimKind::Front, file_index, base_image_index,
                                animation, tick, left, anchor_y, top_anchored,
                                depth_y(base_y_world),
                            ) {
                                commands.entity(e).insert(FrontChunkKey(cx, cy));
                            }
                        } else if let Some(e) = spawn_anim_tile(
                            commands, libraries, assets, tile_cache,
                            TileAnimKind::Front, file_index, base_image_index,
                            animation, tick, false, left, anchor_y, top_anchored,
                            depth_y(base_y_world),
                        ) {
                            commands.entity(e).insert(FrontChunkKey(cx, cy));
                        }
                    }
                }
                count += 1;
                continue;
            }
            let Some((file_index, image_index)) = cell.front_tile() else {
                continue;
            };
            let key = (file_index, image_index);
            let cached = front_images.0.get(&key).cloned();
            let (handle, w, h) = match cached {
                Some(c) => c,
                None => {
                    let Some(info) = libraries.get_map_image(file_index, image_index) else {
                        continue;
                    };
                    if info.width <= 0 || info.height <= 0 {
                        continue;
                    }
                    let Some(rgba) = info.rgba.clone() else {
                        continue;
                    };
                    let (w, h) = (info.width, info.height);
                    let mut img = make_image(rgba, w.max(0) as u32, h.max(0) as u32);
                    img.sampler = ImageSampler::nearest();
                    let handle = assets.add(img);
                    front_images.0.insert(key, (handle.clone(), w, h));
                    (handle, w, h)
                }
            };
            // 基准 Y = 格子底边 (y+1)*32
            let base_y = ((y + 1) * TILE_HEIGHT as i32) as f32;
            let left = (x as f32) * TILE_WIDTH;
            let top = base_y - h as f32;
            let center_x = left + w as f32 / 2.0;
            let (center_y, z) = if (w == TILE_WIDTH as i16 && h == TILE_HEIGHT as i16)
                || (w == TILE_WIDTH as i16 * 2 && h == TILE_HEIGHT as i16 * 2)
            {
                // C# DrawFloor：1x1/2x2 地面贴花左上角对齐，且 z 在地板之上、角色之下
                (-(y as f32 * TILE_HEIGHT + h as f32 / 2.0), 0.15)
            } else {
                // C# DrawObjects：高物件底边对齐，与角色按 Y 交错
                (-(base_y - h as f32 / 2.0), depth_y(base_y))
            };
            commands.spawn((
                Sprite::from_image(handle),
                Transform::from_xyz(center_x, center_y, z),
                Visibility::default(),
                FrontChunkKey(cx, cy),
                FrontTile {
                    base_y,
                    left,
                    top,
                    right: left + w as f32,
                    bottom: base_y,
                },
            ));
            count += 1;
        }
    }
    count
}

fn setup_world(
    mut commands: Commands,
    mut assets: ResMut<Assets<Image>>,
    mut game_data: ResMut<GameData>,
    mut game_libs: ResMut<GameLibraries>,
    mut tile_cache: ResMut<TileImageCache>,
    mut front_images: ResMut<FrontImageCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut blend_materials: ResMut<Assets<MapBlendMaterial>>,
    // 只取地图相机（排除 UI 相机：UiEntity + Camera2d；否则两个相机 single_mut 失败 → 相机停在 (0,0) 显示左上角）
    mut camera: Query<
        &mut Transform,
        (With<Camera2d>, Without<crate::ui::sprite_ui::UiEntity>),
    >,
) {
    // 1. 加载图像库（MapLibs）
    game_libs.0.ensure_initialized();
    let libraries = &mut game_libs.0;
    tracing::info!(
        "📚 库状态: 单体 {} 个, MapLibs {} 个",
        libraries.stats().0,
        libraries.stats().1
    );

    // 2. 加载地图（网络 MapChanged 优先，其次命令行 --map）
    let map_name = game_data
        .desired_map
        .clone()
        .unwrap_or_else(map_arg);
    let map_path = resolve_map_path(&map_name);
    let map = match MapReader::new(&map_path) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("❌ 地图加载失败 {}: {}", map_path, e);
            commands.spawn(Camera2d);
            return;
        }
    };
    tracing::info!(
        "🗺️ 地图 {} 加载成功: {}x{}",
        map_path,
        map.width,
        map.height
    );

    // 灯光混合瓦片共享单位 quad（缩放 = 瓦片尺寸）
    let blend_quad = meshes.add(Rectangle::new(1.0, 1.0));

    // 3. 按块生成纹理（流式：只烘焙相机附近初始窗口，其余由 chunk_stream_system 按需加载）
    let mut spawned = 0usize;
    let chunks_x = div_ceil_i32(map.width, CHUNK_TILES as i32);
    let chunks_y = div_ceil_i32(map.height, CHUNK_TILES as i32);

    let cam_cx = (map.width as f32 * TILE_WIDTH / 2.0 / CHUNK_PIXEL_W as f32) as i32;
    let cam_cy = (map.height as f32 * TILE_HEIGHT / 2.0 / CHUNK_PIXEL_H as f32) as i32;
    let radius = 2i32;
    for layer in [Layer::Back, Layer::Middle] {
        for cy in (cam_cy - radius)..=(cam_cy + radius) {
            for cx in (cam_cx - radius)..=(cam_cx + radius) {
                if cx < 0 || cy < 0 || cx >= chunks_x || cy >= chunks_y {
                    continue;
                }
                if let Some(handle) =
                    build_chunk(libraries, &map, layer, cx, cy, &mut assets)
                {
                    let rect_x = (cx * CHUNK_TILES as i32) as f32 * TILE_WIDTH;
                    let rect_y = (cy * CHUNK_TILES as i32) as f32 * TILE_HEIGHT;
                    let px = rect_x + CHUNK_PIXEL_W as f32 / 2.0;
                    let py = -(rect_y + CHUNK_PIXEL_H as f32 / 2.0);
                    commands.spawn((
                        Sprite::from_image(handle),
                        Transform::from_xyz(px, py, layer.z()),
                        Visibility::default(),
                        MapFloorMark(layer),
                        ChunkKey(cx, cy, layer),
                    ));
                    spawned += 1;
                }
            }
        }
    }
    tracing::info!("🧩 地图块初始窗口生成: {} 个 Sprite", spawned);

    // 3.5 Front 层：按 chunk 窗口流式生成（#31 性能：不再全图 4 万精灵）
    // 逐瓦片精灵，z 按基准 Y（格子底边）与角色交错排序。
    let mut front_spawned = 0usize;
    for cy in (cam_cy - radius)..=(cam_cy + radius) {
        for cx in (cam_cx - radius)..=(cam_cx + radius) {
            if cx < 0 || cy < 0 || cx >= chunks_x || cy >= chunks_y {
                continue;
            }
            front_spawned += spawn_front_chunk(
                &mut commands, libraries, &mut assets, &mut tile_cache,
                &mut blend_materials, &blend_quad, &mut front_images, &map, cx, cy,
            );
        }
    }
    tracing::info!("🌳 Front 瓦片精灵初始窗口生成: {} 个", front_spawned);

    // 3.6 地图灯光（C# DrawLights Map Lights）：cell.Light 1..9 全量生成，
    // 白色径向渐变 + ADD 混合，z=0.9（场景之上、UI 之下，F 键可开关）
    let light_tex = make_light_texture(&mut assets, 128);
    let mut light_spawned = 0usize;
    for y in 0..map.height as usize {
        for x in 0..map.width as usize {
            let cell = &map.map_cells[x][y];
            let l = cell.light;
            if l == 0 || l >= 10 {
                continue;
            }
            let li = ((l as usize % 10) * 3).min(9);
            let (lw, lh) = LIGHT_SIZES[li];
            // C#：若该格有 front 动画，叠加库偏移
            let mut off_x = 0.0f32;
            let mut off_y = 0.0f32;
            if cell.front_animation_frame > 0 {
                if let Some((file_index, image_index)) = cell.front_tile() {
                    if let Some(info) = libraries.get_map_image(file_index, image_index) {
                        off_x = info.offset_x as f32;
                        off_y = info.offset_y as f32;
                    }
                }
            }
            let cell_left = x as f32 * TILE_WIDTH as f32;
            let cell_bottom_world = -((y + 1) as f32 * TILE_HEIGHT as f32);
            // C# p.Offset(-(W/2)-24+10, -(H/2)-16-5)，中心=cell_left+off_x-14+W/2, bottom-11
            let cx = cell_left + off_x - 14.0 + lw / 2.0;
            let cy = cell_bottom_world - 11.0;
            let mat = blend_materials.add(crate::map_tile_anim::MapBlendMaterial {
                color: bevy::prelude::LinearRgba::WHITE,
                texture: light_tex.clone(),
            });
            commands.spawn((
                MapLight,
                bevy::prelude::Mesh2d(blend_quad.clone()),
                bevy::prelude::MeshMaterial2d(mat),
                Transform::from_xyz(cx, cy, 0.9)
                    .with_scale(Vec3::new(lw, lh, 1.0)),
                Visibility::default(),
            ));
            light_spawned += 1;
        }
    }
    tracing::info!("💡 地图灯光生成完成: {} 个", light_spawned);

    // 3.7 C# DrawObjects：非 1x1/2x2 的静态 Middle（大树/建筑等）底边对齐单独画
    let mut obj_spawned = 0usize;
    for y in 0..map.height as usize {
        for x in 0..map.width as usize {
            let cell = &map.map_cells[x][y];
            if let Some((file_index, image_index)) = cell.middle_tile() {
                if let Some(info) = libraries.get_map_image(file_index, image_index) {
                    let (w, h) = (info.width.max(0) as u32, info.height.max(0) as u32);
                    if w > 0
                        && h > 0
                        && !((w == TILE_WIDTH as u32 && h == TILE_HEIGHT as u32)
                            || (w == TILE_WIDTH as u32 * 2 && h == TILE_HEIGHT as u32 * 2))
                    {
                        let left = x as f32 * TILE_WIDTH as f32;
                        let bottom = -((y + 1) as f32 * TILE_HEIGHT as f32);
                        let center_x = left + w as f32 / 2.0;
                        let center_y = bottom + h as f32 / 2.0;
                        if let Some(rgba) = info.rgba.clone() {
                            let mut img = make_image(rgba, w, h);
                            img.sampler = ImageSampler::nearest();
                            let handle = assets.add(img);
                            commands.spawn((
                                Sprite::from_image(handle),
                                Transform::from_xyz(
                                    center_x,
                                    center_y,
                                    depth_y((y + 1) as f32 * TILE_HEIGHT as f32),
                                ),
                                Visibility::default(),
                            ));
                            obj_spawned += 1;
                        }
                    }
                }
            }
        }
    }
    tracing::info!("🏛️ 对象层 Middle 大图生成完成: {} 个", obj_spawned);

    // 4. 相机定位（优先玩家出生点，否则地图中心）
    let center_x = map.width as f32 * TILE_WIDTH / 2.0;
    let center_y = -(map.height as f32 * TILE_HEIGHT / 2.0);
    // 相机固定放地图中心（用户要求：中心才能看到建筑；玩家在中心附近）
    let (cam_x, cam_y) = (center_x, center_y);

    if let Ok(mut cam_tf) = camera.single_mut() {
        cam_tf.translation = Vec3::new(cam_x, cam_y, 10.0);
        tracing::info!("[DIAG] 相机定位: ({:.0},{:.0})", cam_x, cam_y);
    } else {
        tracing::warn!("[DIAG] 相机定位失败！Camera2d 数量={}", camera.iter().count());
    }

    // 构建可行走网格（M8 寻路）
    let mut walkable = Vec::with_capacity(map.width as usize);
    for x in 0..map.width {
        let mut col = Vec::with_capacity(map.height as usize);
        for y in 0..map.height {
            col.push(map.map_cells[x as usize][y as usize].is_walkable());
        }
        walkable.push(col);
    }

pub struct GameData {
    pub map: Option<LoadedMap>,
    /// 地图解析器（供 chunk 流式按需加载）
    pub map_reader: Option<std::sync::Arc<MapReader>>,
    /// 网络 MapChanged 指定的地图名（优先于命令行 --map）
    pub desired_map: Option<String>,
    /// 玩家出生位置（瓦片坐标 + 朝向），来自 MapChanged
    pub player_spawn: Option<(f32, f32, u8)>,
}
    game_data.map = Some(LoadedMap {
        name: map_name.clone(),
        width: map.width,
        height: map.height,
        walkable,
    });
    game_data.map_reader = Some(std::sync::Arc::new(map));

}

/// Startup：创建唯一的 2D 相机（登录界面需要相机渲染 egui；进入游戏后重定位）
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// 把指定块的三层之一合成一张 RGBA 画布。块内无任何瓦片时返回 None。
///
/// 供 Bevy 渲染与离屏诊断（examples）共用，确保验证路径与渲染路径一致。
pub fn build_chunk_rgba(
    libraries: &mut Libraries,
    map: &MapReader,
    layer: Layer,
    cx: i32,
    cy: i32,
) -> Option<Vec<u8>> {
    let mut canvas = vec![0u8; (CHUNK_PIXEL_W * CHUNK_PIXEL_H * 4) as usize];
    let mut any_drawn = false;

    let start_x = cx * CHUNK_TILES as i32;
    let start_y = cy * CHUNK_TILES as i32;
    let end_x = (start_x + CHUNK_TILES as i32).min(map.width);
    let end_y = (start_y + CHUNK_TILES as i32).min(map.height);

    // 关键：瓦片/物件的图像尺寸可能超出单格，跨块边界会被 canvas 裁剪，
    // 造成块边界出现 32px 透明缝隙（macroquad 是视图空间整幅绘制，无此问题）。
    // 这里按层多迭代边界外若干行/列，由 blit 自行裁剪，保证跨界瓦片完整衔接。
    // - Back(96x64)：底部/右侧伸出 1 格，顶部/左侧由相邻块补齐
    // - Middle：双向各留 1 格（覆盖稍高的中景瓦片）
    // - Front：高物件从格子底部向上延伸，向上留 16 行、向右留 8 列
    let (x_lo, x_hi, y_lo, y_hi) = match layer {
        Layer::Back => ((start_x - 1).max(0), (end_x + 1).min(map.width), start_y, (end_y + 1).min(map.height)),
        Layer::Middle => ((start_x - 1).max(0), (end_x + 1).min(map.width), (start_y - 1).max(0), (end_y + 1).min(map.height)),
        Layer::Front => ((start_x - 1).max(0), (end_x + 8).min(map.width), (start_y - 1).max(0), (end_y + 16).min(map.height)),
    };

    for x in x_lo..x_hi {
        for y in y_lo..y_hi {
            // Back 层是 2x2 格子共享的（macroquad render_back_layer 只遍历偶数坐标）。
            // 奇数格可能存着与偶数格不一致的图，画出来会造成与参考实现不同的叠放。
            if layer == Layer::Back && (x % 2 != 0 || y % 2 != 0) {
                continue;
            }
            let cell = &map.map_cells[x as usize][y as usize];
            let Some((file_index, image_index)) = layer.tile(cell) else {
                continue;
            };
            let Some(info) = libraries.get_map_image(file_index, image_index) else {
                continue;
            };
            // C# DrawFloor：地板层 Middle 只画 1x1/2x2，其余走对象层
            if layer == Layer::Middle {
                let (w, h) = (info.width, info.height);
                if !((w == TILE_WIDTH as i16 && h == TILE_HEIGHT as i16)
                    || (w == TILE_WIDTH as i16 * 2 && h == TILE_HEIGHT as i16 * 2))
                {
                    continue;
                }
            }
            let Some(rgba) = info.rgba.as_ref() else {
                continue;
            };
            // C# DrawFloor：地板层左上角对齐格子左上角
            let dx = (x - start_x) * TILE_WIDTH as i32;
            let dy = (y - start_y) * TILE_HEIGHT as i32;
            if blit(&mut canvas, dx, dy, &info, rgba) {
                any_drawn = true;
            }
        }
    }

    if !any_drawn {
        return None;
    }

    Some(canvas)
}

/// 把指定块的三层之一合成一张纹理。块内无任何瓦片时返回 None。
fn build_chunk(
    libraries: &mut Libraries,
    map: &MapReader,
    layer: Layer,
    cx: i32,
    cy: i32,
    assets: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let canvas = build_chunk_rgba(libraries, map, layer, cx, cy)?;
    let mut image = make_image(canvas, CHUNK_PIXEL_W, CHUNK_PIXEL_H);
    // 地图瓦片用最近邻过滤，避免缩放时发虚（与 macroquad MapLibs 的 Nearest 一致）
    image.sampler = ImageSampler::nearest();
    Some(assets.add(image))
}

/// 把图像 RGBA 拷贝到画布，返回是否有像素被写入
fn blit(
    canvas: &mut [u8],
    dx: i32,
    dy: i32,
    img: &ImageInfo,
    rgba: &[u8],
) -> bool {
    let w = img.width as i32;
    let h = img.height as i32;
    if w <= 0 || h <= 0 {
        return false;
    }
    let mut drawn = false;
    for yy in 0..h {
        let sy = dy + yy;
        if sy < 0 || sy >= CHUNK_PIXEL_H as i32 {
            continue;
        }
        for xx in 0..w {
            let sx = dx + xx;
            if sx < 0 || sx >= CHUNK_PIXEL_W as i32 {
                continue;
            }
            let src = ((yy * w + xx) * 4) as usize;
            if rgba[src + 3] == 0 {
                continue;
            }
            let dst = ((sy * CHUNK_PIXEL_W as i32 + sx) * 4) as usize;
            canvas[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
            drawn = true;
        }
    }
    drawn
}

/// 用原始 RGBA 数据构造 Bevy Image 资产
pub(crate) fn make_image(rgba: Vec<u8>, width: u32, height: u32) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// 相机控制：WASD/方向键平移，+/- 缩放
fn camera_control(
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };
    let dt = time.delta_secs();

    let mut pan = Vec3::ZERO;
    let speed = 480.0 * dt;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        pan.x -= speed;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        pan.x += speed;
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        pan.y += speed;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        pan.y -= speed;
    }
    transform.translation += pan;

    // 缩放：1.0 = 1 世界单位 ≈ 1 像素
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        ortho.scale = (ortho.scale / 1.02).max(0.02);
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        ortho.scale = (ortho.scale * 1.02).min(4.0);
    }
}


/// 图层调试热键：1=Back 2=Middle 3=Front静态 F=动画/混合
fn map_layer_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<MapLayerShow>,
    mut floors: Query<(&MapFloorMark, &mut Visibility), (Without<FrontTile>,)>,
    mut fronts: Query<&mut Visibility, (With<FrontTile>, Without<MapFloorMark>)>,
    mut anims: Query<&mut Visibility, (With<crate::map_tile_anim::MapTileAnim>, Without<MapFloorMark>, Without<FrontTile>)>,
    mut lights: Query<&mut Visibility, (With<MapLight>, Without<MapFloorMark>, Without<FrontTile>, Without<crate::map_tile_anim::MapTileAnim>)>,
) {
    if keys.just_pressed(KeyCode::Digit1) { show.back = !show.back; tracing::info!("[LAYER] Back {}", if show.back {"ON"} else {"OFF"}); }
    if keys.just_pressed(KeyCode::Digit2) { show.middle = !show.middle; tracing::info!("[LAYER] Middle {}", if show.middle {"ON"} else {"OFF"}); }
    if keys.just_pressed(KeyCode::Digit3) { show.front = !show.front; tracing::info!("[LAYER] Front {}", if show.front {"ON"} else {"OFF"}); }
    if keys.just_pressed(KeyCode::KeyF) { show.anim = !show.anim; tracing::info!("[LAYER] Anim {}", if show.anim {"ON"} else {"OFF"}); }
    for (mark, mut vis) in floors.iter_mut() {
        let on = match mark.0 { Layer::Back => show.back, Layer::Middle => show.middle, Layer::Front => show.front };
        *vis = if on { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in fronts.iter_mut() {
        *vis = if show.front { Visibility::Visible } else { Visibility::Hidden };
    }
    // F 键同时控制动画/混合瓦片与地图灯光（原版 F 键开关动画/灯光）
    for mut vis in anims.iter_mut() {
        *vis = if show.anim { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in lights.iter_mut() {
        *vis = if show.anim { Visibility::Visible } else { Visibility::Hidden };
    }
}


/// 相机跟随（参考 macroquad CameraFollowSystem）：远距直跳 + lerp 平滑
fn camera_follow_system(
    mut camera: Query<
        &mut Transform,
        (
            With<Camera2d>,
            Without<crate::ui::sprite_ui::UiEntity>,
            Without<crate::actor::LocalPlayer>,
        ),
    >,
    players: Query<
        &Transform,
        (
            With<crate::actor::LocalPlayer>,
            With<crate::actor::NetObjectId>,
            Without<Camera2d>,
        ),
    >,
) {
    let Ok(mut cam) = camera.single_mut() else { return };
    let Ok(player) = players.single() else { return };
    // C# 风格：相机精确跟随玩家（玩家恒定在屏幕中心），
    // 消除 lerp 滞后造成的画面轻微抖动/拖影
    let p = player.translation;
    let c = cam.translation;
    let far = (p.x - c.x).abs() > 1024.0 * 6.0 || (p.y - c.y).abs() > 768.0 * 6.0;
    if far || (p.x - c.x).abs() > 0.01 || (p.y - c.y).abs() > 0.01 {
        cam.translation.x = p.x;
        cam.translation.y = p.y;
    }
}


/// chunk 流式系统：相机移动到新 chunk 时，加载 3x3 窗口内的地板块，卸载窗口外的。
/// 全图烘焙 = 数 GB 内存；流式 = 常驻 ~几十 MB。
fn chunk_stream_system(
    mut commands: Commands,
    mut stream: ResMut<ChunkStream>,
    game_data: Res<GameData>,
    mut game_libs: ResMut<GameLibraries>,
    mut assets: ResMut<Assets<Image>>,
    mut tile_cache: ResMut<TileImageCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut blend_materials: ResMut<Assets<MapBlendMaterial>>,
    mut front_images: ResMut<FrontImageCache>,
    camera: Query<
        &Transform,
        (
            With<Camera2d>,
            Without<crate::ui::sprite_ui::UiEntity>,
            Without<crate::actor::LocalPlayer>,
        ),
    >,
    chunks: Query<(Entity, &ChunkKey)>,
    front_chunks: Query<(Entity, &FrontChunkKey)>,
) {
    let Some(map_reader) = game_data.map_reader.clone() else { return };
    let Ok(cam) = camera.single() else { return };
    let cam_cx = (cam.translation.x / CHUNK_PIXEL_W as f32) as i32;
    let cam_cy = ((-cam.translation.y) / CHUNK_PIXEL_H as f32) as i32;
    if stream.last_cam_chunk == Some((cam_cx, cam_cy)) {
        return;
    }
    stream.last_cam_chunk = Some((cam_cx, cam_cy));
    let chunks_x = div_ceil_i32(map_reader.width, CHUNK_TILES as i32);
    let chunks_y = div_ceil_i32(map_reader.height, CHUNK_TILES as i32);
    let radius = 2i32;

    let mut wanted = std::collections::HashSet::new();
    for layer in [Layer::Back, Layer::Middle] {
        for cy in (cam_cy - radius)..=(cam_cy + radius) {
            for cx in (cam_cx - radius)..=(cam_cx + radius) {
                if cx >= 0 && cy >= 0 && cx < chunks_x && cy < chunks_y {
                    wanted.insert((cx, cy, layer));
                }
            }
        }
    }
    let existing: std::collections::HashSet<_> =
        chunks.iter().map(|(_, k)| (k.0, k.1, k.2)).collect();

    // 卸载窗口外
    for (e, k) in chunks.iter() {
        if !wanted.contains(&(k.0, k.1, k.2)) {
            commands.entity(e).despawn();
        }
    }
    // 加载窗口内缺失
    let mut added = 0usize;
    for (cx, cy, layer) in &wanted {
        if existing.contains(&(*cx, *cy, *layer)) {
            continue;
        }
        if let Some(handle) = build_chunk(
            &mut game_libs.0,
            &map_reader,
            *layer,
            *cx,
            *cy,
            &mut assets,
        ) {
            let rect_x = (*cx * CHUNK_TILES as i32) as f32 * TILE_WIDTH;
            let rect_y = (*cy * CHUNK_TILES as i32) as f32 * TILE_HEIGHT;
            let px = rect_x + CHUNK_PIXEL_W as f32 / 2.0;
            let py = -(rect_y + CHUNK_PIXEL_H as f32 / 2.0);
            commands.spawn((
                Sprite::from_image(handle),
                Transform::from_xyz(px, py, layer.z()),
                Visibility::default(),
                MapFloorMark(*layer),
                ChunkKey(*cx, *cy, *layer),
            ));
            added += 1;
        }
    }
    if added > 0 {
        tracing::info!("🧩 chunk 流式加载 {} 个", added);
    }

    // Front 层流式：同窗口生成/卸载精灵（#31 性能，避免全图 4 万实体常驻）
    let blend_quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mut wanted_front = std::collections::HashSet::new();
    for cy in (cam_cy - radius)..=(cam_cy + radius) {
        for cx in (cam_cx - radius)..=(cam_cx + radius) {
            if cx >= 0 && cy >= 0 && cx < chunks_x && cy < chunks_y {
                wanted_front.insert((cx, cy));
            }
        }
    }
    let existing_front: std::collections::HashSet<_> =
        front_chunks.iter().map(|(_, k)| (k.0, k.1)).collect();
    for (e, k) in front_chunks.iter() {
        if !wanted_front.contains(&(k.0, k.1)) {
            commands.entity(e).despawn();
        }
    }
    let mut front_added = 0usize;
    for (cx, cy) in &wanted_front {
        if existing_front.contains(&(*cx, *cy)) {
            continue;
        }
        front_added += spawn_front_chunk(
            &mut commands,
            &mut game_libs.0,
            &mut assets,
            &mut tile_cache,
            &mut blend_materials,
            &blend_quad,
            &mut front_images,
            &map_reader,
            *cx,
            *cy,
        );
    }
    if front_added > 0 {
        tracing::info!("🌳 front 流式加载 {} 个", front_added);
    }
}
