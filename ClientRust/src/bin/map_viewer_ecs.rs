// ============================================================================
// Map Viewer ECS - 基于 ECS 架构的地图查看器
// ============================================================================
//
// 功能:
// - 使用 hecs ECS 架构组织代码
// - 完整地图渲染 (Back/Middle/Front 三层)
// - 鼠标拖拽移动相机
// - 鼠标滚轮缩放
// - 显示坐标和FPS
// - M键选择地图文件
// - B/M/F键切换图层显示
// - G键切换网格
// - O键切换障碍物
// - A键切换动画
//
// 运行: cargo run --bin map_viewer_ecs --release

use ggez::winit::event::MouseButton;
use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{
        self, BlendComponent, BlendFactor, BlendMode, BlendOperation, Canvas, Color, DrawParam,
        Text,
    },
    Context, ContextBuilder, GameResult,
};
use hecs::{Entity, World};
use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};
use rfd::FileDialog;
use std::time::Instant;

// ============================================================================
// ECS 组件定义
// ============================================================================

/// 位置组件 - 世界坐标
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

/// 相机组件 - 视口控制
#[derive(Debug, Clone)]
struct Camera {
    zoom: f32,
    screen_width: f32,
    screen_height: f32,
}

/// 拖拽组件 - 鼠标拖拽状态
#[derive(Debug, Clone)]
struct Draggable {
    is_dragging: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    drag_start_pos_x: f32,
    drag_start_pos_y: f32,
}

/// 地图瓦片组件
#[derive(Debug, Clone)]
struct MapTile {
    grid_x: i32,
    grid_y: i32,
    layer: TileLayer,
    library_index: i16,
    image_index: i32,
    use_blend: bool,
    brightness: f32,
    z_order: i32,  // 🎯 Z轴绘制顺序：数值越大越后绘制（在上层）
}

/// 瓦片层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TileLayer {
    Back = 0,
    Middle = 1,
    Front = 2,
}

/// 动画瓦片组件
#[derive(Debug, Clone)]
struct AnimatedTile {
    frame_count: u8,
    frame_interval: u8,
    base_image_index: i32,
}

/// 门组件
#[derive(Debug, Clone)]
struct Door {
    door_index: u8,
    door_offset: i32,
    state: DoorState,
    current_frame: i32,
    last_tick: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DoorState {
    Closed = 0,
    Opening = 1,
    Open = 2,
    Closing = 3,
}

/// 地图数据组件 (单例)
#[derive(Clone)]
struct MapData {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
}

/// 渲染配置组件 (单例)
#[derive(Debug, Clone)]
struct RenderConfig {
    show_back: bool,
    show_middle: bool,
    show_front: bool,
    show_grid: bool,
    show_obstacles: bool,
    show_animations: bool,
    show_borders: bool,
    max_fps: u32,  // 🎯 最大帧率限制
    enable_lod: bool,  // 🎯 启用LOD（缩小时过滤纹理）
}

/// 时间组件 (单例) - 用于动画计数
#[derive(Debug, Clone)]
struct TimeTracker {
    animation_count: i32,
    frame_count: u64,
    fps: f32,
    last_fps_update: Instant,
    last_frame_time: Instant,  // 🎯 用于帧率限制
}

/// 可见区域缓存 (单例) - 用于视口裁剪优化
#[derive(Debug, Clone)]
struct VisibleArea {
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
    front_end_y: i32,  // Front层需要额外扩展
    zoom: f32,
    camera_x: f32,
    camera_y: f32,
    // 🔥 只缓存实体ID，渲染时实时读取MapTile（支持动画更新）
    visible_entities: Vec<hecs::Entity>,
    last_update: Instant,
}

impl Default for VisibleArea {
    fn default() -> Self {
        Self {
            start_x: -999999,
            end_x: -999999,
            start_y: -999999,
            end_y: -999999,
            front_end_y: -999999,
            zoom: -1.0,
            camera_x: -999999.0,
            camera_y: -999999.0,
            visible_entities: Vec::new(),
            last_update: Instant::now(),
        }
    }
}

// ============================================================================
// 常量定义
// ============================================================================

const CELL_WIDTH: i32 = 48;
const CELL_HEIGHT: i32 = 32;

// ============================================================================
// ECS 系统实现
// ============================================================================

/// 相机系统 - 处理相机移动和缩放
struct CameraSystem;

impl CameraSystem {
    /// 屏幕坐标转世界坐标
    fn screen_to_world(pos: &Position, camera: &Camera, screen_x: f32, screen_y: f32) -> (f32, f32) {
        (
            pos.x + (screen_x - camera.screen_width / 2.0) / camera.zoom,
            pos.y + (screen_y - camera.screen_height / 2.0) / camera.zoom,
        )
    }

    /// 世界坐标转屏幕坐标
    fn world_to_screen(pos: &Position, camera: &Camera, world_x: f32, world_y: f32) -> (f32, f32) {
        (
            (world_x - pos.x) * camera.zoom + camera.screen_width / 2.0,
            (world_y - pos.y) * camera.zoom + camera.screen_height / 2.0,
        )
    }

    /// 开始拖拽
    fn start_drag(draggable: &mut Draggable, pos: &Position, mouse_x: f32, mouse_y: f32) {
        draggable.is_dragging = true;
        draggable.drag_start_x = mouse_x;
        draggable.drag_start_y = mouse_y;
        draggable.drag_start_pos_x = pos.x;
        draggable.drag_start_pos_y = pos.y;
    }

    /// 更新拖拽
    fn update_drag(draggable: &Draggable, pos: &mut Position, camera: &Camera, mouse_x: f32, mouse_y: f32) {
        if draggable.is_dragging {
            let dx = mouse_x - draggable.drag_start_x;
            let dy = mouse_y - draggable.drag_start_y;
            pos.x = draggable.drag_start_pos_x - dx / camera.zoom;
            pos.y = draggable.drag_start_pos_y - dy / camera.zoom;
        }
    }

    /// 结束拖拽
    fn end_drag(draggable: &mut Draggable) {
        draggable.is_dragging = false;
    }

    /// 缩放
    fn zoom(pos: &mut Position, camera: &mut Camera, delta: f32, mouse_x: f32, mouse_y: f32) {
        let old_zoom = camera.zoom;
        camera.zoom = (camera.zoom * (1.0 + delta * 0.1)).clamp(0.1, 4.0);
        
        // 以鼠标位置为中心缩放
        let world_x = pos.x + (mouse_x - camera.screen_width / 2.0) / old_zoom;
        let world_y = pos.y + (mouse_y - camera.screen_height / 2.0) / old_zoom;
        
        pos.x = world_x - (mouse_x - camera.screen_width / 2.0) / camera.zoom;
        pos.y = world_y - (mouse_y - camera.screen_height / 2.0) / camera.zoom;
    }
}

/// 动画系统 - 更新动画帧
struct AnimationSystem;

impl AnimationSystem {
    fn update(world: &mut World, animation_count: i32) {
        for (_entity, (tile, anim)) in world.query_mut::<(&mut MapTile, &AnimatedTile)>() {
            let total_frames = anim.frame_count as i32 + (anim.frame_count as i32 * anim.frame_interval as i32);
            let frame_offset = (animation_count % total_frames) / (1 + anim.frame_interval as i32);
            tile.image_index = anim.base_image_index + frame_offset;
        }
    }
}

/// 门系统 - 更新门动画
struct DoorSystem;

impl DoorSystem {
    fn update(world: &mut World) {
        for (_entity, (tile, door)) in world.query_mut::<(&mut MapTile, &mut Door)>() {
            match door.state {
                DoorState::Opening => {
                    // 门正在打开 (0 -> 8)
                    if door.last_tick.elapsed().as_millis() > 100 {
                        door.current_frame += 1;
                        if door.current_frame >= 8 {
                            door.current_frame = 8;
                            door.state = DoorState::Open;
                        }
                        door.last_tick = Instant::now();
                    }
                }
                DoorState::Closing => {
                    // 门正在关闭 (8 -> 0)
                    if door.last_tick.elapsed().as_millis() > 100 {
                        door.current_frame -= 1;
                        if door.current_frame <= 0 {
                            door.current_frame = 0;
                            door.state = DoorState::Closed;
                        }
                        door.last_tick = Instant::now();
                    }
                }
                _ => {}
            }
            
            // 更新瓦片图像索引
            if door.current_frame > 0 {
                tile.image_index += (door.current_frame + 1) * door.door_offset;
            }
        }
    }
}

/// 渲染系统 - 绘制所有可见瓦片
struct RenderSystem;

impl RenderSystem {
    /// 创建 ADD 混合模式 (火焰/特效)
    fn create_blend_mode() -> BlendMode {
        BlendMode {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        }
    }

    /// 绘制所有瓦片 (带视口裁剪优化 + 屏幕剔除)
    fn draw_tiles(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
        visible_area_entity: hecs::Entity,
    ) -> GameResult<()> {
        // 计算可见区域
        let projection_scale = 1.0 / camera.zoom;
        let half_width = camera.screen_width / 2.0 * projection_scale;
        let half_height = camera.screen_height / 2.0 * projection_scale;

        let left = pos.x - half_width;
        let right = pos.x + half_width;
        let top = pos.y - half_height;
        let bottom = pos.y + half_height;

        // 🔧 动态缓冲区：zoom越小(缩小)，buffer越小，减少过度渲染
        // 🚀 极致优化：缩放<0.4x时，buffer=0，完全不渲染屏幕外瓦片
        let base_buffer = if camera.zoom < 0.4 { 0 } else { 1 };
        let buffer = ((base_buffer as f32 * projection_scale).ceil() as i32).max(0).min(5);

        // 转换为地图格子坐标
        let start_x = ((left / CELL_WIDTH as f32).floor() as i32 - buffer).max(0);
        let end_x = (right / CELL_WIDTH as f32).ceil() as i32 + buffer;
        let start_y = ((top / CELL_HEIGHT as f32).floor() as i32 - buffer).max(0);
        let end_y = (bottom / CELL_HEIGHT as f32).ceil() as i32 + buffer;

        // 🎨 Front层特殊处理：向下扩展更多格子（建筑物可能很高）
        // 🏢 重要：Front层必须保持足够扩展，否则屏幕底部建筑物顶部会被裁剪
        let front_extra_cells = ((15.0 * projection_scale).ceil() as i32).max(8).min(30);
        let front_end_y = end_y + front_extra_cells;

        // 🔍 检测可见区域或缩放是否变化
        if let Ok(mut visible_area) = world.get::<&mut VisibleArea>(visible_area_entity) {
            let min_cell_threshold = 2; // 至少移动2个格子才重建
            let x_changed = (visible_area.start_x - start_x).abs() >= min_cell_threshold
                || (visible_area.end_x - end_x).abs() >= min_cell_threshold;
            let y_changed = (visible_area.start_y - start_y).abs() >= min_cell_threshold
                || (visible_area.end_y - end_y).abs() >= min_cell_threshold;
            let front_y_changed =
                (visible_area.front_end_y - front_end_y).abs() >= min_cell_threshold;
            let zoom_changed = (visible_area.zoom - camera.zoom).abs() > 0.05;

            let area_changed = x_changed || y_changed || front_y_changed || zoom_changed;

            // 如果区域变化，重新查询可见瓦片
            if area_changed {
                visible_area.visible_entities.clear();

                // 🔥 收集可见实体（带 z_order 和 Y 坐标用于排序）
                let mut visible_with_sort_key: Vec<(hecs::Entity, i32, i32)> = Vec::new();

                // 🎯 LOD优化：缩小时跳过部分纹理
                let lod_skip = if config.enable_lod && camera.zoom < 0.5 {
                    // 缩放 < 0.5 时，跳过部分 Middle 和 Front 层瓦片（棋盘模式）
                    true
                } else {
                    false
                };

                // 查询所有瓦片并过滤
                for (entity, tile) in world.query::<&MapTile>().iter() {
                    // 🎯 LOD过滤：缩小时跳过部分纹理（棋盘剔除）
                    if lod_skip && tile.layer != TileLayer::Back {
                        // Back 层保留（地面），Middle 和 Front 层棋盘剔除
                        if (tile.grid_x + tile.grid_y) % 2 == 0 {
                            continue;  // 跳过 50% 的 Middle/Front 瓦片
                        }
                    }

                    let in_visible_range = match tile.layer {
                        TileLayer::Front => {
                            tile.grid_x >= start_x
                                && tile.grid_x <= end_x
                                && tile.grid_y >= start_y
                                && tile.grid_y <= front_end_y
                        }
                        _ => {
                            tile.grid_x >= start_x
                                && tile.grid_x <= end_x
                                && tile.grid_y >= start_y
                                && tile.grid_y <= end_y
                        }
                    };

                    if in_visible_range {
                        // 🎯 使用 z_order 作为主排序键，Y坐标作为次排序键
                        visible_with_sort_key.push((entity, tile.z_order, tile.grid_y));
                    }
                }

                // 🎯 按 Z轴排序（z_order 优先，相同则按 Y 坐标）
                visible_with_sort_key.sort_by(|a, b| {
                    match a.1.cmp(&b.1) {
                        std::cmp::Ordering::Equal => a.2.cmp(&b.2),  // z_order 相同则按 Y
                        other => other,  // 否则按 z_order
                    }
                });

                // 只保存实体ID
                visible_area.visible_entities = visible_with_sort_key.into_iter().map(|(e, _, _)| e).collect();

                // 更新缓存
                visible_area.start_x = start_x;
                visible_area.end_x = end_x;
                visible_area.start_y = start_y;
                visible_area.end_y = end_y;
                visible_area.front_end_y = front_end_y;
                visible_area.zoom = camera.zoom;
                visible_area.camera_x = pos.x;
                visible_area.camera_y = pos.y;
                visible_area.last_update = Instant::now();
            }

            // 🎯 绘制缓存的可见瓦片（实时读取MapTile以支持动画）
            // 🚀 批量渲染优化：按混合模式分组，减少状态切换
            let mut normal_tiles = Vec::new();
            let mut blend_tiles = Vec::new();
            
            for &entity in &visible_area.visible_entities {
                if let Ok(tile) = world.get::<&MapTile>(entity) {
                    // 根据配置跳过某些层
                    match tile.layer {
                        TileLayer::Back if !config.show_back => continue,
                        TileLayer::Middle if !config.show_middle => continue,
                        TileLayer::Front if !config.show_front => continue,
                        _ => {}
                    }

                    // 按混合模式分组
                    if tile.use_blend {
                        blend_tiles.push(tile);
                    } else {
                        normal_tiles.push(tile);
                    }
                }
            }

            // 先渲染普通瓦片（一次设置混合模式）
            if !normal_tiles.is_empty() {
                canvas.set_blend_mode(BlendMode::ALPHA);
                for tile in normal_tiles {
                    Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config)?;
                }
            }

            // 再渲染混合瓦片（一次设置混合模式）
            if !blend_tiles.is_empty() {
                canvas.set_blend_mode(Self::create_blend_mode());
                for tile in blend_tiles {
                    Self::draw_tile_fast(ctx, canvas, &tile, pos, camera, config)?;
                }
                canvas.set_blend_mode(BlendMode::ALPHA);
            }
        }

        Ok(())
    }

    /// 绘制单个瓦片（快速版：不切换混合模式）
    fn draw_tile_fast(
        ctx: &mut Context,
        canvas: &mut Canvas,
        tile: &MapTile,
        pos: &Position,
        camera: &Camera,
        config: &RenderConfig,
    ) -> GameResult<()> {
        if let Some(mlib) = get_map_library(tile.library_index) {
            if let Ok(mut mlib) = mlib.lock() {
                // 先获取尺寸
                let (tile_w, tile_h) = mlib
                    .get_size(tile.image_index as usize)
                    .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                
                // 再获取纹理
                match mlib.get_or_create_texture(ctx, tile.image_index as usize) {
                    Ok(info) => {
                        if let Some(ref texture) = info.image {
                        // 计算世界坐标
                        let mut world_x = (tile.grid_x * CELL_WIDTH) as f32;
                        let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

                        // 调整Y坐标 (大型物体需要向上偏移)
                        let mut adjusted_y = if (tile_w as i32 != CELL_WIDTH
                            || tile_h as i32 != CELL_HEIGHT)
                            && (tile_w as i32 != CELL_WIDTH * 2
                                || tile_h as i32 != CELL_HEIGHT * 2)
                        {
                            world_y + CELL_HEIGHT as f32 - tile_h as f32
                        } else {
                            world_y
                        };

                        // 🔥 Front层混合模式偏移（火焰、光效等特效）
                        if tile.use_blend && tile.layer == TileLayer::Front {
                            world_x = world_x - 1.0 * CELL_WIDTH as f32;
                            adjusted_y = adjusted_y - 4.0 * CELL_HEIGHT as f32;
                        }

                        // 世界坐标转屏幕坐标
                        let (screen_x, screen_y) =
                            CameraSystem::world_to_screen(pos, camera, world_x, adjusted_y);

                        // 🚀 屏幕剔除：如果完全在屏幕外，跳过绘制
                        // 🏢 但 Front 层不做剔除，因为高建筑物纹理是长条状的
                        let tile_screen_w = tile_w as f32 * camera.zoom;
                        let tile_screen_h = tile_h as f32 * camera.zoom;
                        
                        // 只对 Back 和 Middle 层做屏幕外剔除
                        if tile.layer != TileLayer::Front {
                            if screen_x + tile_screen_w < 0.0 
                                || screen_x > camera.screen_width
                                || screen_y + tile_screen_h < 0.0
                                || screen_y > camera.screen_height {
                                return Ok(());
                            }
                        }

                        // 绘制瓦片（不切换混合模式，外部已设置）
                        let color = Color::from_rgba(
                            (255.0 * tile.brightness) as u8,
                            (255.0 * tile.brightness) as u8,
                            (255.0 * tile.brightness) as u8,
                            255,
                        );

                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([screen_x, screen_y])
                                .scale([camera.zoom, camera.zoom])
                                .color(color),
                        );

                        // 绘制边框 (调试用)
                        if config.show_borders {
                            let border_color = match tile.layer {
                                TileLayer::Back => Color::from_rgb(255, 0, 0),
                                TileLayer::Middle => Color::from_rgb(0, 255, 0),
                                TileLayer::Front => Color::from_rgb(0, 150, 255),
                            };

                            let border = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::stroke(1.0),
                                graphics::Rect::new(
                                    screen_x,
                                    screen_y,
                                    tile_screen_w,
                                    tile_screen_h,
                                ),
                                border_color,
                            )?;
                            canvas.draw(&border, DrawParam::default());
                        }
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        Ok(())
    }

    /// 绘制网格
    fn draw_grid(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        // 获取地图尺寸
        let (map_width, map_height) = world
            .query::<&MapData>()
            .iter()
            .next()
            .map(|(_, data)| (data.width, data.height))
            .unwrap_or((100, 100));

        let left = pos.x + (0.0 - camera.screen_width / 2.0) / camera.zoom;
        let right = pos.x + (camera.screen_width - camera.screen_width / 2.0) / camera.zoom;
        let top = pos.y + (0.0 - camera.screen_height / 2.0) / camera.zoom;
        let bottom = pos.y + (camera.screen_height - camera.screen_height / 2.0) / camera.zoom;

        let start_x = ((left / CELL_WIDTH as f32).floor() as i32).max(0);
        let end_x = ((right / CELL_WIDTH as f32).ceil() as i32).min(map_width);
        let start_y = ((top / CELL_HEIGHT as f32).floor() as i32).max(0);
        let end_y = ((bottom / CELL_HEIGHT as f32).ceil() as i32).min(map_height);

        let grid_color = Color::from_rgba(0, 255, 0, 120);

        // 垂直线
        for x in start_x..=end_x {
            let world_x = (x * CELL_WIDTH) as f32;
            let (screen_x, _) = CameraSystem::world_to_screen(pos, camera, world_x, 0.0);

            if screen_x >= 0.0 && screen_x <= camera.screen_width {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[screen_x, 0.0], [screen_x, camera.screen_height]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 水平线
        for y in start_y..=end_y {
            let world_y = (y * CELL_HEIGHT) as f32;
            let (_, screen_y) = CameraSystem::world_to_screen(pos, camera, 0.0, world_y);

            if screen_y >= 0.0 && screen_y <= camera.screen_height {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[0.0, screen_y], [camera.screen_width, screen_y]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        Ok(())
    }

    /// 绘制障碍物
    fn draw_obstacles(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        let map_data = world
            .query::<&MapData>()
            .iter()
            .next()
            .map(|(_, data)| data.clone());

        if let Some(map_data) = map_data {
            let left = pos.x + (0.0 - camera.screen_width / 2.0) / camera.zoom;
            let right = pos.x + (camera.screen_width - camera.screen_width / 2.0) / camera.zoom;
            let top = pos.y + (0.0 - camera.screen_height / 2.0) / camera.zoom;
            let bottom = pos.y + (camera.screen_height - camera.screen_height / 2.0) / camera.zoom;

            let start_x = ((left / CELL_WIDTH as f32).floor() as i32).max(0);
            let end_x = ((right / CELL_WIDTH as f32).ceil() as i32).min(map_data.width);
            let start_y = ((top / CELL_HEIGHT as f32).floor() as i32).max(0);
            let end_y = ((bottom / CELL_HEIGHT as f32).ceil() as i32).min(map_data.height);

            let obstacle_color = Color::from_rgba(255, 0, 0, 100);

            for y in start_y..end_y {
                for x in start_x..end_x {
                    if x >= 0 && x < map_data.width && y >= 0 && y < map_data.height {
                        let cell = &map_data.cells[x as usize][y as usize];
                        // 检查是否有阻挡 (简化版 - 有 Front 层或 Middle 高位标记通常表示障碍)
                        let has_obstacle = cell.front_image > 0 || (cell.middle_image & 0x8000) != 0;
                        if has_obstacle {
                            let world_x = (x * CELL_WIDTH) as f32;
                            let world_y = (y * CELL_HEIGHT) as f32;
                            let (screen_x, screen_y) =
                                CameraSystem::world_to_screen(pos, camera, world_x, world_y);

                            let rect = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::fill(),
                                graphics::Rect::new(
                                    screen_x,
                                    screen_y,
                                    CELL_WIDTH as f32 * camera.zoom,
                                    CELL_HEIGHT as f32 * camera.zoom,
                                ),
                                obstacle_color,
                            )?;
                            canvas.draw(&rect, DrawParam::default());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// 地图加载器
// ============================================================================

struct MapLoader;

impl MapLoader {
    /// 从 MapReader 创建所有瓦片实体
    fn load_map(world: &mut World, reader: MapReader) -> GameResult<()> {
        let width = reader.width;
        let height = reader.height;
        let cells = reader.map_cells.clone();

        // 创建地图数据单例
        world.spawn((MapData {
            cells: cells.clone(),
            width,
            height,
        },));

        println!("📦 正在加载地图瓦片到 ECS...");
        let mut tile_count = 0;

        // 遍历所有格子，创建瓦片实体
        for x in 0..width {
            for y in 0..height {
                let cell = &cells[x as usize][y as usize];

                // Back 层 - 只加载偶数行列 (传奇地图特性：Back层使用大瓦片96x64覆盖4个格子)
                if x % 2 == 0 && y % 2 == 0 {
                    Self::load_back_tile(world, cell, x, y, &mut tile_count);
                }

                // Middle 层
                Self::load_middle_tile(world, cell, x, y, &mut tile_count);

                // Front 层
                Self::load_front_tile(world, cell, x, y, &mut tile_count);
            }
        }

        println!("✅ 加载完成: {} 个瓦片实体", tile_count);
        Ok(())
    }

    fn load_back_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
        let index = (cell.back_image & 0x1FFFFFFF) - 1;
        if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
            return;
        }

        // Back层只有静态瓦片，无动画（传奇地图特性）
        let tile = MapTile {
            grid_x: x,
            grid_y: y,
            layer: TileLayer::Back,
            library_index: cell.back_index,
            image_index: index,
            use_blend: false,
            brightness: 1.0,
            z_order: 0,  // 🎯 Back层最底层
        };

        world.spawn((tile,));
        *count += 1;
    }

    fn load_middle_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
        let mut index = (cell.middle_image & 0x7FFF) - 1;
        if index < 0 || cell.middle_index < 0 {
            return;
        }

        let mut animation = cell.middle_animation_frame;
        let use_blend = (animation & 0x0f) > 0;
        animation &= 0x0f;

        if animation > 0 {
            // 动画瓦片
            let tile = MapTile {
                grid_x: x,
                grid_y: y,
                layer: TileLayer::Middle,
                library_index: cell.middle_index,
                image_index: index,
                use_blend: use_blend && (animation == 10 || animation == 8),
                brightness: 1.0,
                z_order: 1000,  // 🎯 Middle层中间层
            };

            let anim = AnimatedTile {
                frame_count: animation,
                frame_interval: cell.middle_animation_tick,
                base_image_index: index,
            };

            world.spawn((tile, anim));
        } else {
            // 静态瓦片
            let tile = MapTile {
                grid_x: x,
                grid_y: y,
                layer: TileLayer::Middle,
                library_index: cell.middle_index,
                image_index: index,
                use_blend: false,
                brightness: 1.0,
                z_order: 1000,  // 🎯 Middle层中间层
            };

            world.spawn((tile,));
        }

        *count += 1;
    }

    fn load_front_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
        let mut index = (cell.front_image & 0x7FFF) - 1;
        if index < 0 || cell.front_index < 0 || cell.front_index == 200 {
            return;
        }

        let mut animation = cell.front_animation_frame;
        let use_blend = (animation & 0x80) != 0;
        animation &= 0x7F;

        let has_animation = animation > 0;
        let has_door = cell.door_index > 0;

        // 创建瓦片
        let mut tile = MapTile {
            grid_x: x,
            grid_y: y,
            layer: TileLayer::Front,
            library_index: cell.front_index,
            image_index: index,
            use_blend,
            brightness: if use_blend && !has_animation { 1.5 } else { 1.0 },
            z_order: 2000,  // 🎯 Front层最上层
        };

        let mut builder = hecs::EntityBuilder::new();
        builder.add(tile);

        // 添加动画组件
        if has_animation {
            let anim = AnimatedTile {
                frame_count: animation,
                frame_interval: cell.front_animation_tick,
                base_image_index: index,
            };
            builder.add(anim);
        }

        // 添加门组件
        if has_door {
            let door = Door {
                door_index: cell.door_index,
                door_offset: cell.door_offset as i32,
                state: DoorState::Closed,
                current_frame: 0,
                last_tick: Instant::now(),
            };
            builder.add(door);
        }

        world.spawn(builder.build());
        *count += 1;
    }
}

// ============================================================================
// 主应用程序
// ============================================================================

struct MapViewerApp {
    world: World,
    camera_entity: Entity,
    time_entity: Entity,
    config_entity: Entity,
    visible_area_entity: Entity,
}

impl MapViewerApp {
    fn new(ctx: &mut Context, map_path: &str) -> GameResult<Self> {
        // 初始化库
        println!("📚 正在初始化地图库...");
        initialize_all_libraries("Data").expect("初始化地图库失败");
        println!("✅ 地图库初始化完成");

        // 加载地图
        println!("🗺️ 正在加载地图: {}", map_path);
    let reader = MapReader::new(map_path)?;
        println!("✅ 地图加载完成: {}x{}", reader.width, reader.height);

        // 创建 ECS 世界
        let mut world = World::new();

        // 加载地图瓦片到 ECS
        MapLoader::load_map(&mut world, reader)?;

        // 创建相机实体
        let screen = ctx.gfx.drawable_size();
        let camera_entity = world.spawn((
            Position {
                x: 2400.0,
                y: 1600.0,
            },
            Camera {
                zoom: 1.0,
                screen_width: screen.0,
                screen_height: screen.1,
            },
            Draggable {
                is_dragging: false,
                drag_start_x: 0.0,
                drag_start_y: 0.0,
                drag_start_pos_x: 0.0,
                drag_start_pos_y: 0.0,
            },
        ));

        // 创建时间跟踪实体
        let time_entity = world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),  // 🎯 帧率限制计时
        },));

        // 创建渲染配置实体
        let config_entity = world.spawn((RenderConfig {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: false,
            show_obstacles: false,
            show_animations: true,
            show_borders: false,
            max_fps: 160,  // 🎯 最高160帧
            enable_lod: true,  // 🎯 启用LOD优化
        },));

        // 创建可见区域缓存实体
        let visible_area_entity = world.spawn((VisibleArea::default(),));

        Ok(Self {
            world,
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
        })
    }

    /// 选择并加载新地图
    fn load_new_map(&mut self, ctx: &mut Context) -> GameResult<()> {
        if let Some(path) = FileDialog::new()
            .add_filter("地图文件", &["map"])
            .set_directory("Map")
            .pick_file()
        {
            println!("🗺️ 正在加载新地图: {:?}", path);

            // 清除旧瓦片
            let tile_entities: Vec<_> = self
                .world
                .query::<&MapTile>()
                .iter()
                .map(|(e, _)| e)
                .collect();

            for entity in tile_entities {
                let _ = self.world.despawn(entity);
            }

            // 加载新地图
            let reader = MapReader::new(path.to_str().unwrap())?;
            println!("✅ 地图加载完成: {}x{}", reader.width, reader.height);

            MapLoader::load_map(&mut self.world, reader)?;

            // 重置相机位置
            if let Ok(mut pos) = self.world.get::<&mut Position>(self.camera_entity) {
                pos.x = 2400.0;
                pos.y = 1600.0;
            }
        }

        Ok(())
    }
}

impl EventHandler for MapViewerApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // 🎯 帧率限制（最高 160 FPS）
        let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap();
        let max_fps = config.max_fps;
        drop(config);  // 释放借用

        if let Ok(mut time) = self.world.get::<&mut TimeTracker>(self.time_entity) {
            // 计算目标帧时间
            let target_frame_time = std::time::Duration::from_secs_f32(1.0 / max_fps as f32);
            let elapsed = time.last_frame_time.elapsed();
            
            // 如果距离上一帧时间太短，提前返回（跳过此帧）
            if elapsed < target_frame_time {
                return Ok(());
            }
            
            // 更新时间跟踪
            time.last_frame_time = Instant::now();
            time.animation_count += 1;
            time.frame_count += 1;

            if time.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                time.fps = time.frame_count as f32 / time.last_fps_update.elapsed().as_secs_f32();
                time.frame_count = 0;
                time.last_fps_update = Instant::now();
            }
        }

        // 获取配置
        let show_animations = self
            .world
            .get::<&RenderConfig>(self.config_entity)
            .map(|c| c.show_animations)
            .unwrap_or(true);

        // 更新动画系统
        if show_animations {
            let animation_count = self
                .world
                .get::<&TimeTracker>(self.time_entity)
                .map(|t| t.animation_count)
                .unwrap_or(0);

            AnimationSystem::update(&mut self.world, animation_count);
            DoorSystem::update(&mut self.world);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);

        // 获取相机组件
        let (pos, camera) = {
            let pos = self.world.get::<&Position>(self.camera_entity).unwrap().clone();
            let camera = self.world.get::<&Camera>(self.camera_entity).unwrap().clone();
            (pos, camera)
        };

        let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap().clone();

        // 渲染瓦片 (带视口裁剪优化)
        RenderSystem::draw_tiles(
            ctx,
            &mut canvas,
            &self.world,
            &pos,
            &camera,
            &config,
            self.visible_area_entity,
        )?;

        // 渲染网格
        if config.show_grid {
            RenderSystem::draw_grid(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 渲染障碍物
        if config.show_obstacles {
            RenderSystem::draw_obstacles(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 绘制 UI 文本
        let time = self.world.get::<&TimeTracker>(self.time_entity).unwrap();
        let ui_text = format!(
            "FPS: {:.1} / {} (最大)  LOD: {}\n\
             位置: ({:.0}, {:.0})  缩放: {:.2}x\n\
             图层: B={} M={} F={}\n\
             [M]选择地图 [G]网格 [O]障碍 [A]动画 [L]LOD\n\
             [+/-]调整最大帧率 [1/2/3]切换图层\n\
             [鼠标拖拽]移动 [滚轮]缩放",
            time.fps,
            config.max_fps,
            if config.enable_lod { "开" } else { "关" },
            pos.x,
            pos.y,
            camera.zoom,
            if config.show_back { "√" } else { "×" },
            if config.show_middle { "√" } else { "×" },
            if config.show_front { "√" } else { "×" },
        );

        let text = Text::new(ui_text);
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([10.0, 10.0])
                .color(Color::from_rgb(255, 255, 0)),
        );

        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult<()> {
        if button == MouseButton::Left {
            let pos = self.world.get::<&Position>(self.camera_entity).unwrap().clone();
            let mut draggable = self.world.get::<&mut Draggable>(self.camera_entity).unwrap();
            CameraSystem::start_drag(&mut draggable, &pos, x, y);
        }
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult<()> {
        if button == MouseButton::Left {
            let mut draggable = self.world.get::<&mut Draggable>(self.camera_entity).unwrap();
            CameraSystem::end_drag(&mut draggable);
        }
        Ok(())
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult<()> {
        let draggable = self.world.get::<&Draggable>(self.camera_entity).unwrap().clone();
        let camera = self.world.get::<&Camera>(self.camera_entity).unwrap().clone();
        let mut pos = self.world.get::<&mut Position>(self.camera_entity).unwrap();
        CameraSystem::update_drag(&draggable, &mut pos, &camera, x, y);
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) -> GameResult<()> {
        // 先获取鼠标位置（不涉及 world 借用）
        let mouse_pos = _ctx.mouse.position();
        
        // 然后一次性获取可变引用并调用 zoom
        let mut pos = self.world.get::<&mut Position>(self.camera_entity).unwrap();
        let mut camera = self.world.get::<&mut Camera>(self.camera_entity).unwrap();
        
        CameraSystem::zoom(&mut pos, &mut camera, y, mouse_pos.x, mouse_pos.y);
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult<()> {
        use ggez::input::keyboard::KeyCode;
        use ggez::winit::keyboard::PhysicalKey;

        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            match keycode {
                KeyCode::KeyM => {
                    self.load_new_map(ctx)?;
                }
                KeyCode::Digit1 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_back = !config.show_back;
                    println!("Back 层 (1): {}", if config.show_back { "显示" } else { "隐藏" });
                }
                KeyCode::Digit2 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_middle = !config.show_middle;
                    println!("Middle 层 (2): {}", if config.show_middle { "显示" } else { "隐藏" });
                }
                KeyCode::Digit3 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_front = !config.show_front;
                    println!("Front 层 (3): {}", if config.show_front { "显示" } else { "隐藏" });
                }
                KeyCode::KeyB => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_borders = !config.show_borders;
                    println!("纹理边框 (B): {}", if config.show_borders { "显示" } else { "隐藏" });
                }
                KeyCode::KeyG => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_grid = !config.show_grid;
                    println!("网格 (G): {}", if config.show_grid { "显示" } else { "隐藏" });
                }
                KeyCode::KeyO => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_obstacles = !config.show_obstacles;
                    println!("障碍物 (O): {}", if config.show_obstacles { "显示" } else { "隐藏" });
                }
                KeyCode::KeyA => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_animations = !config.show_animations;
                    println!("动画 (A): {}", if config.show_animations { "播放" } else { "暂停" });
                }
                KeyCode::KeyL => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.enable_lod = !config.enable_lod;
                    println!("🎯 LOD优化 (L): {}", if config.enable_lod { "启用（缩小时过滤50%瓦片）" } else { "禁用" });
                }
                KeyCode::Equal | KeyCode::NumpadAdd => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.max_fps = (config.max_fps + 10).min(300);
                    println!("🎯 最大FPS (+ 键): {} 帧", config.max_fps);
                }
                KeyCode::Minus | KeyCode::NumpadSubtract => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                    println!("🎯 最大FPS (- 键): {} 帧", config.max_fps);
                }
                KeyCode::Escape => {
                    ctx.request_quit();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> GameResult<()> {
        let mut camera = self.world.get::<&mut Camera>(self.camera_entity).unwrap();
        camera.screen_width = width;
        camera.screen_height = height;
        Ok(())
    }
}

// ============================================================================
// 主函数
// ============================================================================

fn main() -> GameResult {
    // 默认地图路径
    let default_map = "Map/0.map";

    // 创建 GGEZ 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer_ecs", "Crystal Team")
        .window_setup(WindowSetup::default().title("传奇地图查看器 ECS - GGEZ + hecs").vsync(false))
        .window_mode(
            WindowMode::default()
                .dimensions(1280.0, 720.0)
                .resizable(true),
        )
        .build()?;

    // 创建应用
    let app = MapViewerApp::new(&mut ctx, default_map)?;

    println!("\n🎮 ECS 地图查看器已启动!");
    println!("📋 快捷键:");
    println!("  [M] - 选择地图文件");
    println!("  [1/2/3] - 切换 Back/Middle/Front 层");
    println!("  [G] - 切换网格显示");
    println!("  [O] - 切换障碍物显示");
    println!("  [A] - 切换动画播放");
    println!("  [L] - 🎯 切换 LOD 优化（缩小时过滤纹理）");
    println!("  [+/-] - 🎯 调整最大帧率限制");
    println!("  [B] - 切换边框显示 (调试)");
    println!("  [鼠标拖拽] - 移动视角");
    println!("  [鼠标滚轮] - 缩放");
    println!("  [ESC] - 退出");
    println!("\n🚀 性能优化:");
    println!("  • 最大帧率: 160 FPS (可调)");
    println!("  • LOD: 缩放 < 0.5x 时自动过滤 50% Middle/Front 瓦片");
    println!("  • Z轴排序: 灵活控制绘制顺序\n");

    // 运行事件循环
    event::run(ctx, event_loop, app)?;
    Ok(())
}
