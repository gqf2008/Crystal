// Map Renderer - 地图渲染器 (Bevy版本)
//
// 功能说明:
// - 渲染地图的3个图层 (Back, Middle, Front)
// - 支持动画瓦片和门系统
// - 视锥剔除优化
//
// 架构设计:
// - MapRenderData: 资源,存储地图数据 (复用 objects::CellInfo)
// - TileEntity: 组件标记,用于管理瓦片实体
// - render_map_system: 系统,生成和更新瓦片
//
// 复用策略:
// - 完全复用 objects::CellInfo (地图数据结构)
// - 完全复用 objects::MapReader (地图加载)
// - 完全复用 mlibrary_assets (纹理加载)
// - 保留渲染逻辑 (3层、可见区域裁剪、动画)
// - 适配到 Bevy ECS (Sprite + Transform)

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashMap;

use super::mlibrary_assets::MLibraryAssets;
use super::camera::GameCamera;
use crate::objects::{CellInfo};

/// 地图常量
const CELL_WIDTH: i32 = 48;
const CELL_HEIGHT: i32 = 32;

/// 地图渲染数据资源
///
/// 注意: 这是为渲染优化的地图数据结构,与 components::MapRenderData 不同
/// - components::MapRenderData: 游戏逻辑使用 (地图对象、环境属性等)
/// - MapRenderData: 渲染系统使用 (CellInfo、动画、门等)
#[derive(Resource)]
pub struct MapRenderData {
    /// 地图格子数据 (复用 CellInfo)
    pub cells: Vec<Vec<CellInfo>>,
    /// 地图宽度
    pub width: i32,
    /// 地图高度
    pub height: i32,
    /// 门列表
    pub doors: Vec<DoorInfo>,
    /// 动画计数器
    pub animation_count: i32,
    /// 显示控制
    pub show_layer_back: bool,
    pub show_layer_middle: bool,
    pub show_layer_front: bool,
    pub show_animations: bool,
    pub show_grid: bool,
    pub show_borders: bool,
    pub show_obstacles: bool,
}

impl Default for MapRenderData {
    fn default() -> Self {
        Self::empty()
    }
}

impl MapRenderData {
    /// 创建空地图
    pub fn empty() -> Self {
        Self {
            cells: Vec::new(),
            width: 0,
            height: 0,
            doors: Vec::new(),
            animation_count: 0,
            show_layer_back: true,
            show_layer_middle: true,
            show_layer_front: true,
            show_animations: true,
            show_grid: false,
            show_borders: false,
            show_obstacles: false,
        }
    }
    
    /// 从 CellInfo 数组加载地图
    pub fn from_cells(cells: Vec<Vec<CellInfo>>) -> Self {
        let height = cells.len() as i32;
        let width = if height > 0 { cells[0].len() as i32 } else { 0 };

        // 提取门信息
        let mut doors = Vec::new();
        let mut door_id = 1u8;
        for y in 0..height {
            for x in 0..width {
                if let Some(cell) = cells.get(y as usize).and_then(|row| row.get(x as usize)) {
                    if cell.door_offset > 0 {
                        doors.push(DoorInfo {
                            id: door_id,
                            x,
                            y,
                            offset: cell.door_offset,
                            is_open: false,
                            frame: 0,
                        });
                        door_id += 1;
                    }
                }
            }
        }

        Self {
            cells,
            width,
            height,
            doors,
            animation_count: 0,
            show_layer_back: true,
            show_layer_middle: true,
            show_layer_front: true,
            show_animations: true,
            show_grid: false,
            show_borders: false,
            show_obstacles: false,
        }
    }

    /// 获取指定位置的格子
    #[inline]
    pub fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        self.cells
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
    }

    /// 获取门的动画帧
    pub fn get_door_frame(&self, door_index: u8) -> i32 {
        self.doors
            .iter()
            .find(|d| d.id == door_index)
            .map(|d| d.frame as i32)
            .unwrap_or(0)
    }

    /// 地图坐标 → 世界坐标
    /// 
    /// 注意: 返回Bevy世界坐标 (Y轴向上为正)
    /// - X轴: 向右为正 (与GGEZ相同)
    /// - Y轴: 向上为正 (与GGEZ相反，需要翻转)
    #[inline]
    pub fn map_to_world(x: i32, y: i32) -> (f32, f32) {
        let world_x = x as f32 * CELL_WIDTH as f32;
        let world_y = -(y as f32 * CELL_HEIGHT as f32); // 翻转Y轴: 地图Y向下，世界Y向上
        (world_x, world_y)
    }
}

/// 门信息
#[derive(Clone, Debug)]
pub struct DoorInfo {
    pub id: u8,
    pub x: i32,
    pub y: i32,
    pub offset: u8,
    pub is_open: bool,
    pub frame: u8,
}

/// 瓦片实体标记组件
#[derive(Component)]
pub struct TileEntity {
    /// 地图格子坐标
    pub map_x: i32,
    pub map_y: i32,
    /// 图层类型
    pub layer: TileLayer,
    /// 是否是动画瓦片
    pub is_animated: bool,
    /// 纹理偏移量 (从MLibrary的ImageInfo中获取)
    pub offset_x: i16,
    pub offset_y: i16,
}

/// 图层类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileLayer {
    Back,
    Middle,
    Front,
}

/// 瓦片渲染缓存 (用于跟踪已生成的瓦片实体)
#[derive(Resource, Default)]
pub struct TileCache {
    /// 已渲染的瓦片 (key: "layer_x_y")
    pub entities: HashMap<String, Entity>,
}

impl TileCache {
    pub fn make_key(layer: TileLayer, x: i32, y: i32) -> String {
        format!("{:?}_{}_{}", layer, x, y)
    }
}

/// 初始化地图渲染器
pub fn setup_map_renderer(
    mut commands: Commands,
    mut tile_cache: Option<ResMut<TileCache>>,
    tile_query: Query<Entity, With<TileEntity>>,
) {
    use std::path::PathBuf;
    use super::mlibrary_assets::MLibraryAssets;
    
    // 如果TileCache已存在，清空所有旧的瓦片实体
    if let Some(mut cache) = tile_cache {
        info!("🧹 清理旧的瓦片实体 ({} 个)", tile_query.iter().count());
        for entity in tile_query.iter() {
            commands.entity(entity).despawn();
        }
        cache.entities.clear();
    }
    
    // 初始化地图渲染数据
    commands.insert_resource(MapRenderData::empty());
    commands.insert_resource(TileCache::default());
    
    // 初始化 MLibrary 资源 (从 Data 文件夹加载纹理)
    let data_path = PathBuf::from("Data");
    let mut mlibrary_assets = MLibraryAssets::new(data_path);
    
    // 预加载所有库 (MapLibs[0-399] 和游戏内容库)
    if let Err(e) = mlibrary_assets.preload_all_libraries() {
        error!("❌ MLibrary 库加载失败: {}", e);
        return;
    }
    
    commands.insert_resource(mlibrary_assets);
    info!("✅ MLibraryAssets 已初始化并预加载");
    
    info!("✅ MapRenderer 初始化完成");
}

/// 更新动画计数器
pub fn update_animation_system(mut map_data: ResMut<MapRenderData>) {
    map_data.animation_count = (map_data.animation_count + 1) % 1000000;
}

/// 渲染地图系统 (主入口)
///
/// 策略:
/// 1. 计算可见区域 (基于摄像机)
/// 2. 为可见格子生成 Sprite 实体
/// 3. 使用 TileCache 避免重复生成
/// 4. 按需更新动画瓦片
pub fn render_map_system(
    mut commands: Commands,
    mlibrary: Option<ResMut<MLibraryAssets>>,
    map_data: Res<MapRenderData>,
    mut tile_cache: ResMut<TileCache>,
    mut images: ResMut<Assets<Image>>,
    camera_query: Query<(&Transform, &GameCamera), With<Camera2d>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    tile_query: Query<(Entity, &TileEntity)>,
) {
    // 如果 MLibraryAssets 未初始化,跳过
    let Some(mut mlibrary) = mlibrary else {
        warn!("⚠️ MLibraryAssets 不存在，跳过渲染");
        return;
    };
    
    // 如果没有地图数据,跳过
    if map_data.width == 0 || map_data.height == 0 {
        warn!("⚠️ 地图数据为空 ({}x{})", map_data.width, map_data.height);
        return;
    }
    
    // 第一次渲染时清空缓存并记录日志
    if tile_cache.entities.is_empty() {
        info!("🎨 render_map_system 首次渲染 (地图: {}x{})", map_data.width, map_data.height);
    } else if tile_cache.entities.len() > 0 {
        // 如果缓存中有数据但没有实际创建过Sprite,清空缓存
        static mut CACHE_CLEARED: bool = false;
        unsafe {
            if !CACHE_CLEARED {
                info!("🧹 清空瓦片缓存 ({} 个条目)", tile_cache.entities.len());
                tile_cache.entities.clear();
                CACHE_CLEARED = true;
            }
        }
    }

    // 获取摄像机和窗口信息
    let Ok((camera_transform, game_camera)) = camera_query.single() else {
        warn!("⚠️ 找不到游戏摄像机");
        return;
    };
    let Ok(window) = window_query.single() else {
        return;
    };

    // 计算可见区域 (基于摄像机)
    let camera_pos = Vec2::new(
        camera_transform.translation.x,
        -camera_transform.translation.y, // Bevy Y轴向上,转回世界坐标
    );
    let screen_size = Vec2::new(window.width(), window.height());
    
    // 第一次渲染时输出摄像机信息
    if tile_cache.entities.is_empty() {
        info!("📷 摄像机位置: ({:.1}, {:.1}), 屏幕大小: ({:.0}, {:.0})", 
            camera_pos.x, camera_pos.y, screen_size.x, screen_size.y);
    }

    let (start_x, end_x, start_y, end_y) = game_camera.get_visible_tiles(
        camera_pos,
        screen_size,
        map_data.width,
        map_data.height,
    );
    
    // 第一次渲染时输出可见区域
    if tile_cache.entities.is_empty() {
        info!("🔲 可见区域: X({} ~ {}), Y({} ~ {})", start_x, end_x, start_y, end_y);
    }

    // TODO: 清理不在可见区域的瓦片 (实现更智能的剔除)

    // 渲染3个图层
    if map_data.show_layer_back {
        render_back_layer(
            &mut commands,
            &mut mlibrary,
            &map_data,
            &mut tile_cache,
            &mut images,
            start_x,
            end_x,
            start_y,
            end_y,
        );
    }

    if map_data.show_layer_middle {
        render_middle_layer(
            &mut commands,
            &mut mlibrary,
            &map_data,
            &mut tile_cache,
            &mut images,
            start_x,
            end_x,
            start_y,
            end_y,
        );
    }

    if map_data.show_layer_front {
        render_front_layer(
            &mut commands,
            &mut mlibrary,
            &map_data,
            &mut tile_cache,
            &mut images,
            start_x,
            end_x,
            start_y,
            end_y,
        );
    }
}

/// 渲染 Back 层 (大地砖 - 仅静态)
///
/// 逻辑:
/// - 只渲染偶数行列
/// - 无动画
/// - 尺寸: 48x32 或 96x64
fn render_back_layer(
    commands: &mut Commands,
    mlibrary: &mut MLibraryAssets,
    map_data: &MapRenderData,
    tile_cache: &mut TileCache,
    images: &mut Assets<Image>,
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
) {
    let mut created_count = 0;
    let mut skipped_cache = 0;
    let mut failed_texture = 0;
    
    // 🎯 Back层只渲染偶数坐标 (地砖大小96x64，占2x2格子)
    let adjusted_start_x = if start_x % 2 == 0 { start_x } else { start_x + 1 };
    let adjusted_start_y = if start_y % 2 == 0 { start_y } else { start_y + 1 };
    
    for y in (adjusted_start_y..=end_y).step_by(2) {
        for x in (adjusted_start_x..=end_x).step_by(2) {
            if let Some(cell) = map_data.get_cell(x, y) {
                let index = (cell.back_image & 0x1FFFFFFF) - 1;
                if index < 0 || cell.back_index == -1 {
                    continue;
                }

                // 检查缓存
                let key = TileCache::make_key(TileLayer::Back, x, y);
                if tile_cache.entities.contains_key(&key) {
                    skipped_cache += 1;
                    continue; // 已经生成过了
                }

                // 加载纹理
                if let Some(texture_data) =
                    mlibrary.get_map_texture(cell.back_index, index as usize, images)
                {
                    created_count += 1;
                    
                    // 🎯 纹理对齐到地图网格（不使用offset）
                    // Back层地砖尺寸: 96x64，占用2x2个格子（48x32每格）
                    // 纹理底部对齐到格子底部
                    let (grid_x, grid_y) = MapRenderData::map_to_world(x, y);
                    let texture_height = texture_data.height as f32;
                    
                    // Bevy Sprite原点在中心，所以Y坐标 = 格子底部 + 纹理高度/2
                    let sprite_y = grid_y + texture_height / 2.0;

                    // 生成 Sprite 实体 - 使用 Sprite::from_image (Bevy 0.17推荐方式)
                    let entity = commands
                        .spawn((
                            Sprite::from_image(texture_data.handle.clone()),
                            Transform::from_xyz(grid_x, sprite_y, 0.0),
                        ))
                        .insert(TileEntity {
                            map_x: x,
                            map_y: y,
                            layer: TileLayer::Back,
                            is_animated: false,
                            offset_x: 0,
                            offset_y: 0,
                        })
                        .id();
                    
                    // 第一个创建的瓦片输出详细信息
                    if created_count == 1 {
                        info!("🎯 首个Back瓦片: 网格({}, {}) → 世界坐标({:.1}, {:.1}), 纹理{}x{}, Sprite中心Y={:.1}, Z=0.0", 
                            x, y, grid_x, grid_y, texture_data.width, texture_height, sprite_y);
                    }

                    // 加入缓存
                    tile_cache.entities.insert(key, entity);
                } else {
                    failed_texture += 1;
                }
            }
        }
    }
    
    if created_count > 0 || failed_texture > 0 {
        info!("📦 Back层渲染: 创建={}, 跳过缓存={}, 纹理失败={}", created_count, skipped_cache, failed_texture);
    }
}

/// 渲染 Middle 层 (中间层 - 静态 + 动画)
///
/// 逻辑:
/// - 渲染所有格子
/// - 支持动画瓦片
/// - 尺寸过滤: 48x32 或 96x64
fn render_middle_layer(
    commands: &mut Commands,
    mlibrary: &mut MLibraryAssets,
    map_data: &MapRenderData,
    tile_cache: &mut TileCache,
    images: &mut Assets<Image>,
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
) {
    for y in start_y..=end_y {
        for x in start_x..=end_x {
            if let Some(cell) = map_data.get_cell(x, y) {
                let mut index = cell.middle_image - 1;
                if index < 0 || cell.middle_index == -1 {
                    continue;
                }

                let animation = cell.middle_animation_frame;
                let has_animation = animation > 0 && animation < 255;

                // 检查缓存 (静态瓦片才缓存,动画瓦片每次更新)
                let key = TileCache::make_key(TileLayer::Middle, x, y);
                if !has_animation && tile_cache.entities.contains_key(&key) {
                    continue;
                }

                // 计算动画帧 (如果有动画)
                if has_animation && map_data.show_animations {
                    let animation_tick = cell.middle_animation_tick;
                    let total_frames = animation as i32 + (animation as i32 * animation_tick as i32);
                    let frame_offset =
                        (map_data.animation_count % total_frames) / (1 + animation_tick as i32);
                    index += frame_offset;
                }

                // 加载纹理
                if let Some(texture_data) =
                    mlibrary.get_map_texture(cell.middle_index, index as usize, images)
                {
                    // 🎯 纹理对齐到地图网格（不使用offset）
                    let (grid_x, grid_y) = MapRenderData::map_to_world(x, y);
                    let texture_height = texture_data.height as f32;
                    let sprite_y = grid_y + texture_height / 2.0;

                    // 如果是动画,先删除旧实体
                    if has_animation {
                        if let Some(old_entity) = tile_cache.entities.remove(&key) {
                            commands.entity(old_entity).despawn();
                        }
                    }

                    // 生成 Sprite 实体
                    let entity = commands
                        .spawn((
                            Sprite {
                                image: texture_data.handle,
                                ..default()
                            },
                            Transform::from_xyz(grid_x, sprite_y, 1.0), // Z=1
                            GlobalTransform::default(),
                            Visibility::default(),
                            InheritedVisibility::default(),
                            ViewVisibility::default(),
                        ))
                        .insert(TileEntity {
                            map_x: x,
                            map_y: y,
                            layer: TileLayer::Middle,
                            is_animated: has_animation,
                            offset_x: 0,
                            offset_y: 0,
                        })
                        .id();

                    // 加入缓存
                    tile_cache.entities.insert(key, entity);
                }
            }
        }
    }
}

/// 渲染 Front 层 (前景层 - 静态 + 动画 + 门)
///
/// 逻辑:
/// - 渲染所有格子
/// - 支持动画瓦片和门
/// - 自动计算Y偏移 (大型物体)
/// - 支持混合模式和亮度控制
fn render_front_layer(
    commands: &mut Commands,
    mlibrary: &mut MLibraryAssets,
    map_data: &MapRenderData,
    tile_cache: &mut TileCache,
    images: &mut Assets<Image>,
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
) {
    for y in start_y..=end_y {
        for x in start_x..=end_x {
            if let Some(cell) = map_data.get_cell(x, y) {
                let mut index = (cell.front_image & 0x7FFF) - 1;
                if index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                    continue;
                }

                let mut animation = cell.front_animation_frame;
                let use_blend = (animation & 0x80) != 0;
                animation &= 0x7F;

                let has_animation = animation > 0;
                let has_door = cell.door_index > 0;

                // 检查缓存 (静态瓦片才缓存)
                let key = TileCache::make_key(TileLayer::Front, x, y);
                let should_update = has_animation || has_door;
                if !should_update && tile_cache.entities.contains_key(&key) {
                    continue;
                }

                // 计算动画帧
                if has_animation && map_data.show_animations {
                    let animation_tick = cell.front_animation_tick;
                    let total_frames = animation as i32 + (animation as i32 * animation_tick as i32);
                    let frame_offset =
                        (map_data.animation_count % total_frames) / (1 + animation_tick as i32);
                    index += frame_offset;
                }

                // 计算门动画
                if has_door && map_data.show_animations {
                    let door_frame = map_data.get_door_frame(cell.door_index);
                    if door_frame > 0 {
                        index += (door_frame + 1) * cell.door_offset as i32;
                    }
                }

                // 加载纹理
                if let Some(texture_data) =
                    mlibrary.get_map_texture(cell.front_index, index as usize, images)
                {
                    // 🎯 纹理对齐到地图网格（不使用offset）
                    let (grid_x, grid_y) = MapRenderData::map_to_world(x, y);
                    let texture_height = texture_data.height as f32;
                    let sprite_y = grid_y + texture_height / 2.0;

                    // 如果需要更新,先删除旧实体
                    if should_update {
                        if let Some(old_entity) = tile_cache.entities.remove(&key) {
                            commands.entity(old_entity).despawn();
                        }
                    }

                    // 🌟 Front层亮度控制（模拟ADD混合效果）
                    let brightness = if use_blend && !has_animation { 1.8 } else { 1.0 };
                    let color = Color::srgb(brightness, brightness, brightness);

                    // 生成 Sprite 实体
                    let entity = commands
                        .spawn((
                            Sprite {
                                image: texture_data.handle,
                                color,
                                ..default()
                            },
                            Transform::from_xyz(grid_x, sprite_y, 2.0), // Z=2
                            GlobalTransform::default(),
                            Visibility::default(),
                            InheritedVisibility::default(),
                            ViewVisibility::default(),
                        ))
                        .insert(TileEntity {
                            map_x: x,
                            map_y: y,
                            layer: TileLayer::Front,
                            is_animated: has_animation || has_door,
                            offset_x: 0,
                            offset_y: 0,
                        })
                        .id();

                    // 加入缓存
                    tile_cache.entities.insert(key, entity);
                }
            }
        }
    }
}
