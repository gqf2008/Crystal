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
    #[inline]
    pub fn map_to_world(x: i32, y: i32) -> (f32, f32) {
        let world_x = x as f32 * CELL_WIDTH as f32;
        let world_y = y as f32 * CELL_HEIGHT as f32;
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
pub fn setup_map_renderer(mut commands: Commands) {
    use std::path::PathBuf;
    use super::mlibrary_assets::MLibraryAssets;
    
    // 初始化地图渲染数据
    commands.insert_resource(MapRenderData::empty());
    commands.insert_resource(TileCache::default());
    
    // 初始化 MLibrary 资源 (从 Data 文件夹加载纹理)
    let data_path = PathBuf::from("Data");
    commands.insert_resource(MLibraryAssets::new(data_path));
    info!("✅ MLibraryAssets 已初始化");
    
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
        return;
    };
    
    // 如果没有地图数据,跳过
    if map_data.width == 0 || map_data.height == 0 {
        return;
    }

    // 获取摄像机和窗口信息
    let Ok((camera_transform, game_camera)) = camera_query.single() else {
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

    let (start_x, end_x, start_y, end_y) = game_camera.get_visible_tiles(
        camera_pos,
        screen_size,
        map_data.width,
        map_data.height,
    );

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
    // 只渲染偶数行列
    for y in (start_y..=end_y).step_by(2) {
        for x in (start_x..=end_x).step_by(2) {
            if let Some(cell) = map_data.get_cell(x, y) {
                let index = (cell.back_image & 0x1FFFFFFF) - 1;
                if index < 0 || cell.back_index == -1 {
                    continue;
                }

                // 检查缓存
                let key = TileCache::make_key(TileLayer::Back, x, y);
                if tile_cache.entities.contains_key(&key) {
                    continue; // 已经生成过了
                }

                // 加载纹理
                if let Some(texture_handle) =
                    mlibrary.get_map_texture(cell.back_index, index as usize, images)
                {
                    let (world_x, world_y) = MapRenderData::map_to_world(x, y);

                    // 生成 Sprite 实体
                    let entity = commands
                        .spawn((
                            Sprite {
                                image: texture_handle,
                                ..default()
                            },
                            Transform::from_xyz(world_x, -world_y, 0.0), // Bevy Y轴向上,所以取负
                        ))
                        .insert(TileEntity {
                            map_x: x,
                            map_y: y,
                            layer: TileLayer::Back,
                            is_animated: false,
                        })
                        .id();

                    // 加入缓存
                    tile_cache.entities.insert(key, entity);
                }
            }
        }
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
                if let Some(texture_handle) =
                    mlibrary.get_map_texture(cell.middle_index, index as usize, images)
                {
                    let (world_x, world_y) = MapRenderData::map_to_world(x, y);

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
                                image: texture_handle,
                                ..default()
                            },
                            Transform::from_xyz(world_x, -world_y, 1.0), // Z=1 (高于Back层)
                        ))
                        .insert(TileEntity {
                            map_x: x,
                            map_y: y,
                            layer: TileLayer::Middle,
                            is_animated: has_animation,
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
                if let Some(texture_handle) =
                    mlibrary.get_map_texture(cell.front_index, index as usize, images)
                {
                    let (world_x, world_y_base) = MapRenderData::map_to_world(x, y);
                    
                    // TODO: 根据纹理尺寸计算Y偏移 (需要从 mlibrary 获取尺寸)
                    let world_y = world_y_base;

                    // 如果需要更新,先删除旧实体
                    if should_update {
                        if let Some(old_entity) = tile_cache.entities.remove(&key) {
                            commands.entity(old_entity).despawn();
                        }
                    }

                    // 亮度控制
                    let brightness = if use_blend && !has_animation { 1.5 } else { 1.0 };
                    let color = Color::srgb(brightness, brightness, brightness);

                    // 生成 Sprite 实体
                    let entity = commands
                        .spawn((
                            Sprite {
                                image: texture_handle,
                                color,
                                ..default()
                            },
                            Transform::from_xyz(world_x, -world_y, 2.0), // Z=2 (高于Middle层)
                        ))
                        .insert(TileEntity {
                            map_x: x,
                            map_y: y,
                            layer: TileLayer::Front,
                            is_animated: has_animation || has_door,
                        })
                        .id();

                    // 加入缓存
                    tile_cache.entities.insert(key, entity);
                }
            }
        }
    }
}
