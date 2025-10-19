// Bevy Map Viewer - Bevy版地图查看器
//
// 功能:
// - 完整地图渲染 (Back/Middle/Front 三层)
// - 鼠标拖拽移动相机
// - 鼠标滚轮缩放
// - 显示坐标和FPS
// - G键切换网格显示
// - M键选择地图文件
// - 1/2/3键切换图层显示
//
// 运行: cargo run --bin map_viewer_bevy --release

use bevy::prelude::*;
use bevy::window::{WindowResolution, PrimaryWindow};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, DiagnosticsStore};
use bevy::input::mouse::{MouseWheel, MouseScrollUnit, MouseMotion};
use mir2_client::graphics::libraries::{initialize_all_libraries,get_map_library};
use mir2_client::objects::{CellInfo, MapReader};
use rfd::FileDialog;
use std::path::PathBuf;

// ============================================================================
// 常量定义
// ============================================================================

/// 地图格子尺寸
const CELL_WIDTH: i32 = 48;
const CELL_HEIGHT: i32 = 32;

/// 初始缩放
const INITIAL_ZOOM: f32 = 1.0;

/// 缩放范围
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 4.0;

// ============================================================================
// 组件定义
// ============================================================================

/// 相机组件
#[derive(Component)]
struct MapCamera {
    /// 目标位置（世界坐标）
    target: Vec2,
    /// 缩放级别
    zoom: f32,
    /// 是否正在拖拽
    dragging: bool,
    /// 拖拽起始位置（屏幕坐标）
    drag_start: Vec2,
    /// 拖拽起始相机位置（世界坐标）
    drag_start_camera: Vec2,
}

impl Default for MapCamera {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            zoom: INITIAL_ZOOM,
            dragging: false,
            drag_start: Vec2::ZERO,
            drag_start_camera: Vec2::ZERO,
        }
    }
}

/// 地图数据资源
#[derive(Resource)]
struct MapData {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
    animation_count: i32,
    map_name: String,
}

impl MapData {
    fn empty() -> Self {
        Self {
            cells: Vec::new(),
            width: 0,
            height: 0,
            animation_count: 0,
            map_name: String::from("未加载"),
        }
    }

    fn from_reader(reader: MapReader, map_name: String) -> Self {
        Self {
            cells: reader.map_cells,
            width: reader.width,
            height: reader.height,
            animation_count: 0,
            map_name,
        }
    }

    #[inline]
    fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            Some(&self.cells[x as usize][y as usize])
        } else {
            None
        }
    }
}

/// MLibrary纹理数据
#[derive(Clone)]
struct TextureData {
    handle: Handle<Image>,
    width: i16,
    height: i16,
    offset_x: i16,
    offset_y: i16,
}

/// MLibrary资源管理器（简化版）
#[derive(Resource)]
struct MLibraryAssets {
    data_path: PathBuf,
    texture_cache: std::collections::HashMap<String, TextureData>,
}

impl MLibraryAssets {
    fn new(data_path: PathBuf) -> Self {
        Self {
            data_path,
            texture_cache: std::collections::HashMap::new(),
        }
    }

    /// 获取地图纹理
    fn get_map_texture(
        &mut self,
        file_index: i16,
        image_index: i32,
        images: &mut Assets<Image>,
    ) -> Option<TextureData> {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

        if image_index <= 0 {
            return None;
        }

        // 缓存key
        let cache_key = format!("Map_{}_{}", file_index, image_index);
        
        // 检查缓存
        if let Some(data) = self.texture_cache.get(&cache_key) {
            return Some(data.clone());
        }

        // 从MLibrary加载
        let mlibrary = get_map_library(file_index)?;
        let mut lib = mlibrary.lock().unwrap();
        let (image_info, image_data) = lib.get_image_with_data(image_index as usize).ok()?;

        if image_data.is_empty() {
            return None;
        }

        // 转换BGRA → RGBA
        let mut rgba_data = Vec::with_capacity(image_data.len());
        for chunk in image_data.chunks_exact(4) {
            rgba_data.push(chunk[2]); // R
            rgba_data.push(chunk[1]); // G
            rgba_data.push(chunk[0]); // B
            rgba_data.push(chunk[3]); // A
        }

        // 创建Bevy Image
        let image = Image::new(
            Extent3d {
                width: image_info.width as u32,
                height: image_info.height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba_data,
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );

        let handle = images.add(image);
        
        let texture_data = TextureData {
            handle: handle.clone(),
            width: image_info.width,
            height: image_info.height,
            offset_x: image_info.x,
            offset_y: image_info.y,
        };

        // 缓存
        self.texture_cache.insert(cache_key, texture_data.clone());

        Some(texture_data)
    }
}

/// 显示设置
#[derive(Resource)]
struct ViewSettings {
    show_back: bool,
    show_middle: bool,
    show_front: bool,
    show_grid: bool,
    show_info: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: false,
            show_info: true,
        }
    }
}

/// 瓦片实体标记
#[derive(Component)]
struct TileSprite {
    grid_x: i32,
    grid_y: i32,
    layer: TileLayer,
    is_animated: bool,  // 是否是动画瓦片
    animation_index: i16,  // 动画索引
}

/// 图层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TileLayer {
    Back,
    Middle,
    Front,
    Door,  // 新增：门层
}

/// 可见区域缓存（用于检测相机移动）
#[derive(Resource, Default)]
struct VisibleArea {
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
    zoom: f32,
}

/// UI文本标记
#[derive(Component)]
struct InfoText;

/// 网格线标记
#[derive(Component)]
struct GridLines;

// ============================================================================
// 坐标转换函数
// ============================================================================

/// 🗺️ 地图格子坐标 → 世界像素坐标
/// 
/// 关键修正：Bevy Y轴向上，需要翻转Y坐标
#[inline]
fn map_to_world(grid_x: i32, grid_y: i32) -> Vec2 {
    Vec2::new(
        (grid_x * CELL_WIDTH) as f32,
        -((grid_y * CELL_HEIGHT) as f32),  // ⚠️ Y轴翻转
    )
}

/// 🌍 世界像素坐标 → 地图格子坐标
#[inline]
fn world_to_map(world_pos: Vec2) -> (i32, i32) {
    (
        (world_pos.x / CELL_WIDTH as f32).floor() as i32,
        (-world_pos.y / CELL_HEIGHT as f32).floor() as i32,  // ⚠️ Y轴翻转
    )
}

// ============================================================================
// 主函数和系统注册
// ============================================================================

fn main() {
    // 初始化库文件
    println!("📚 正在初始化地图库...");
    initialize_all_libraries("Data").expect("初始化地图库失败");
    println!("✅ 地图库初始化完成");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy地图查看器 v2.1 (性能优化版)".to_string(),
                resolution: WindowResolution::new(1600, 900),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(MapData::empty())
        .insert_resource(ViewSettings::default())
        .insert_resource(VisibleArea::default())
        .add_systems(Startup, setup_system)
        .add_systems(Update, (
            // 输入处理
            keyboard_input_system,
            mouse_input_system,
            camera_zoom_system,
            // 渲染
            update_animation_system,
            render_static_tiles_system,  // 🆕 静态瓦片渲染（仅在相机移动时）
            update_animated_tiles_system,  // 🆕 动画瓦片更新（每帧）
            render_grid_system,
            // UI更新
            update_info_text_system,
        ).chain())
        .run();
}

// ============================================================================
// 初始化系统
// ============================================================================

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut clear_color: ResMut<ClearColor>,
) {
    // 设置浅灰色背景
    clear_color.0 = Color::srgb(0.85, 0.85, 0.85);

    // 创建2D相机
    commands.spawn((
        Camera2d,
        MapCamera::default(),
        Name::new("MapCamera"),
    ));

    // 初始化MLibrary资源
    let mlibrary = MLibraryAssets::new(PathBuf::from("Data"));
    commands.insert_resource(mlibrary);

    // 加载中文字体
    let font_handle = asset_server.load("../resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");

    // 创建UI根节点
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        Name::new("UI Root"),
    )).with_children(|parent| {
        // 信息文本（左上角）
        parent.spawn((
            Text::new("Bevy地图查看器 v2.1\n\n按M键加载地图\n按G键显示/隐藏网格\n按1/2/3键切换图层\n按I键显示/隐藏信息\n鼠标中键拖拽\n滚轮缩放"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(0.1, 0.1, 0.1)),  // 深灰色文字（在浅色背景上更清晰）
            TextFont {
                font: font_handle.clone(),  // 使用中文字体
                font_size: 20.0,
                ..default()
            },
            InfoText,
            Name::new("Info Text"),
        ));
    });

    info!("✅ Bevy地图查看器初始化完成");
    info!("📖 按M键加载地图");
}

// ============================================================================
// 输入处理系统
// ============================================================================

/// 键盘输入处理
fn keyboard_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut map_data: ResMut<MapData>,
    mut view_settings: ResMut<ViewSettings>,
) {
    // M键：打开文件选择对话框
    if keyboard.just_pressed(KeyCode::KeyM) {
        if let Some(path) = FileDialog::new()
            .add_filter("地图文件", &["map"])
            .pick_file()
        {
            info!("📂 选择地图: {:?}", path);
            match MapReader::new(path.to_str().unwrap_or("")) {
                Ok(reader) => {
                    let map_name = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("未知")
                        .to_string();
                    
                    info!("✅ 加载成功: {} ({}x{})", map_name, reader.width, reader.height);
                    *map_data = MapData::from_reader(reader, map_name);
                }
                Err(e) => {
                    error!("❌ 加载失败: {:?}", e);
                }
            }
        }
    }

    // G键：切换网格显示
    if keyboard.just_pressed(KeyCode::KeyG) {
        view_settings.show_grid = !view_settings.show_grid;
        info!("🟢 网格显示: {}", if view_settings.show_grid { "开启" } else { "关闭" });
    }

    // I键：切换信息显示
    if keyboard.just_pressed(KeyCode::KeyI) {
        view_settings.show_info = !view_settings.show_info;
    }

    // 1/2/3键：切换图层显示
    if keyboard.just_pressed(KeyCode::Digit1) {
        view_settings.show_back = !view_settings.show_back;
        info!("🎨 Back层: {}", if view_settings.show_back { "显示" } else { "隐藏" });
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        view_settings.show_middle = !view_settings.show_middle;
        info!("🎨 Middle层: {}", if view_settings.show_middle { "显示" } else { "隐藏" });
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        view_settings.show_front = !view_settings.show_front;
        info!("🎨 Front层: {}", if view_settings.show_front { "显示" } else { "隐藏" });
    }
}

/// 鼠标输入处理（拖拽）
fn mouse_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera_query: Query<&mut MapCamera>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(mut camera) = camera_query.single_mut() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    // 开始拖拽
    if mouse_button.just_pressed(MouseButton::Middle) {
        if let Some(cursor_pos) = window.cursor_position() {
            camera.dragging = true;
            camera.drag_start = cursor_pos;
            camera.drag_start_camera = camera.target;
            info!("🖱️ 开始拖拽");
        }
    }

    // 结束拖拽
    if mouse_button.just_released(MouseButton::Middle) {
        camera.dragging = false;
        info!("🖱️ 结束拖拽");
    }

    // 更新拖拽
    if camera.dragging {
        for event in mouse_motion.read() {
            let delta = event.delta;
            camera.target.x -= delta.x / camera.zoom;
            camera.target.y += delta.y / camera.zoom;  // Y轴翻转
        }
    } else {
        mouse_motion.clear();
    }
}

/// 相机缩放处理
fn camera_zoom_system(
    mut mouse_wheel: EventReader<MouseWheel>,
    mut camera_query: Query<(&mut MapCamera, &mut Transform)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok((mut camera, mut transform)) = camera_query.single_mut() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    for event in mouse_wheel.read() {
        let zoom_delta = match event.unit {
            MouseScrollUnit::Line => event.y * 0.1,
            MouseScrollUnit::Pixel => event.y * 0.001,
        };

        let old_zoom = camera.zoom;
        camera.zoom = (camera.zoom * (1.0 + zoom_delta)).clamp(MIN_ZOOM, MAX_ZOOM);

        // 以鼠标位置为中心缩放
        if let Some(cursor_pos) = window.cursor_position() {
            let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
            let cursor_offset = cursor_pos - screen_center;
            
            let world_offset_before = cursor_offset / old_zoom;
            let world_offset_after = cursor_offset / camera.zoom;
            let offset_change = world_offset_after - world_offset_before;
            
            camera.target -= offset_change;
        }
    }

    // 更新相机Transform
    transform.translation.x = camera.target.x;
    transform.translation.y = camera.target.y;
    transform.scale = Vec3::splat(camera.zoom);
}

// ============================================================================
// 动画更新系统
// ============================================================================

fn update_animation_system(mut map_data: ResMut<MapData>) {
    // 每帧递增，每10帧切换一次动画（约6fps的动画速度）
    map_data.animation_count = (map_data.animation_count + 1) % 1000;
}

// ============================================================================
// 地图渲染系统
// ============================================================================

// ============================================================================
// 🚀 性能优化：静态瓦片渲染系统（仅在相机移动时执行）
// ============================================================================

fn render_static_tiles_system(
    mut commands: Commands,
    map_data: Res<MapData>,
    view_settings: Res<ViewSettings>,
    camera_query: Query<(&Transform, &MapCamera)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut visible_area: ResMut<VisibleArea>,
    static_tile_query: Query<Entity, (With<TileSprite>, Without<AnimatedTile>)>,
    mut mlibrary: ResMut<MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    // 如果地图为空，跳过渲染
    if map_data.width == 0 || map_data.height == 0 {
        return;
    }

    let Ok((_camera_transform, camera)) = camera_query.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    // 计算可见区域（世界坐标）
    let half_width = window.width() / 2.0 / camera.zoom;
    let half_height = window.height() / 2.0 / camera.zoom;

    let left = camera.target.x - half_width;
    let right = camera.target.x + half_width;
    let top = camera.target.y + half_height;
    let bottom = camera.target.y - half_height;

    // 转换为地图格子坐标
    let start_x = ((left / CELL_WIDTH as f32).floor() as i32 - 2).max(0);
    let end_x = ((right / CELL_WIDTH as f32).ceil() as i32 + 2).min(map_data.width - 1);
    let start_y = ((-top / CELL_HEIGHT as f32).floor() as i32 - 2).max(0);
    let end_y = ((-bottom / CELL_HEIGHT as f32).ceil() as i32 + 2).min(map_data.height - 1);

    // 🔍 检测可见区域或缩放是否变化
    let area_changed = visible_area.start_x != start_x
        || visible_area.end_x != end_x
        || visible_area.start_y != start_y
        || visible_area.end_y != end_y
        || (visible_area.zoom - camera.zoom).abs() > 0.001;

    if !area_changed {
        return;  // ⚡ 可见区域未变化，跳过静态瓦片重建
    }

    // 更新可见区域缓存
    visible_area.start_x = start_x;
    visible_area.end_x = end_x;
    visible_area.start_y = start_y;
    visible_area.end_y = end_y;
    visible_area.zoom = camera.zoom;

    // 清除所有静态瓦片（但保留动画瓦片）
    for entity in static_tile_query.iter() {
        commands.entity(entity).despawn();
    }

    // ============ Back层渲染（静态） ============
    if view_settings.show_back {
        let back_start_x = if start_x % 2 == 0 { start_x } else { start_x + 1 };
        let back_start_y = if start_y % 2 == 0 { start_y } else { start_y + 1 };
        
        for y in (back_start_y..=end_y).step_by(2) {
            for x in (back_start_x..=end_x).step_by(2) {
                if let Some(cell) = map_data.get_cell(x, y) {
                    if cell.back_image > 0 {
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.back_index,
                            cell.back_image,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            
                            // Bevy Sprite中心偏移
                            let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let sprite_y = world_pos.y + texture_data.height as f32 / 2.0;

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 0.0),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Back,
                                    is_animated: false,
                                    animation_index: 0,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    // ============ Middle层渲染（仅静态瓦片） ============
    if view_settings.show_middle {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    // Middle静态瓦片
                    if cell.middle_image > 0 {
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.middle_index,
                            cell.middle_image,
                            &mut images,
                        ) {
                            // 过滤非标准尺寸
                            if (texture_data.width == 48 && texture_data.height == 32) ||
                               (texture_data.width == 96 && texture_data.height == 64) {
                                let world_pos = map_to_world(x, y);
                                let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                                let sprite_y = world_pos.y + texture_data.height as f32 / 2.0;

                                commands.spawn((
                                    Sprite {
                                        image: texture_data.handle.clone(),
                                        ..default()
                                    },
                                    Transform::from_xyz(sprite_x, sprite_y, 1.0),
                                    TileSprite {
                                        grid_x: x,
                                        grid_y: y,
                                        layer: TileLayer::Middle,
                                        is_animated: false,
                                        animation_index: 0,
                                    },
                                ));
                            }
                        }
                    }
                    
                    // Middle动画瓦片 → 移到 update_animated_tiles_system
                }
            }
        }
    }

    // ============ Front层渲染（仅静态瓦片和门） ============
    if view_settings.show_front {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    // Front静态瓦片
                    if cell.front_image > 0 {
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.front_index,
                            cell.front_image,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            let mut sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let mut sprite_y = world_pos.y + texture_data.height as f32 / 2.0;

                            // 大型物体Y偏移（树木、建筑等）
                            let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
                                         (texture_data.width != 96 || texture_data.height != 64);
                            if is_large {
                                sprite_y += CELL_HEIGHT as f32 - texture_data.height as f32;
                            }

                            // Blend偏移（光效等特殊效果）
                            let use_blend = (cell.front_image as u32) & 0x8000_0000 != 0;
                            if use_blend {
                                sprite_x -= CELL_WIDTH as f32;
                                sprite_y -= (CELL_HEIGHT * 4) as f32;
                            }

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    color: if use_blend {
                                        Color::srgba(1.0, 1.0, 1.0, 0.8)
                                    } else {
                                        Color::WHITE
                                    },
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 2.0),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Front,
                                    is_animated: false,
                                    animation_index: 0,
                                },
                            ));
                        }
                    }
                    
                    // Front层动画瓦片 → 移到 update_animated_tiles_system

                    // 门渲染（静态）
                    if cell.door_index > 0 {
                        // 门的图像索引基于door_offset
                        // door_offset: 0=关闭, 1-7=打开动画帧
                        let door_image_index = cell.door_index as i32 + cell.door_offset as i32;
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.front_index,
                            door_image_index,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let sprite_y = world_pos.y + texture_data.height as f32 / 2.0 + CELL_HEIGHT as f32 - texture_data.height as f32;

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 2.2),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Door,
                                    is_animated: false,
                                    animation_index: cell.door_index as i16,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// 🚀 性能优化：动画瓦片更新系统（每帧执行，但只更新纹理）
// ============================================================================

/// 动画瓦片标记（区分静态和动画瓦片）
#[derive(Component)]
struct AnimatedTile {
    cell_x: i32,
    cell_y: i32,
    layer: TileLayer,
    base_index: i16,  // 动画起始索引
    frames: u8,       // 总帧数
    offset: i32,      // 偏移量
}

fn update_animated_tiles_system(
    mut commands: Commands,
    map_data: Res<MapData>,
    view_settings: Res<ViewSettings>,
    camera_query: Query<(&Transform, &MapCamera)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    visible_area: Res<VisibleArea>,
    animated_tile_query: Query<Entity, With<AnimatedTile>>,
    mut mlibrary: ResMut<MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    // 如果地图为空或可见区域未初始化，跳过
    if map_data.width == 0 || visible_area.start_x == 0 && visible_area.end_x == 0 {
        return;
    }

    let Ok((_camera_transform, _camera)) = camera_query.single() else {
        return;
    };

    let Ok(_window) = windows.single() else {
        return;
    };

    // 使用缓存的可见区域
    let start_x = visible_area.start_x;
    let end_x = visible_area.end_x;
    let start_y = visible_area.start_y;
    let end_y = visible_area.end_y;

    // 清除所有动画瓦片（每帧重建，但数量少）
    for entity in animated_tile_query.iter() {
        commands.entity(entity).despawn();
    }

    // ============ Middle层动画瓦片 ============
    if view_settings.show_middle {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    if cell.tile_animation_frames > 0 && cell.tile_animation_image > 0 {
                        // 计算当前动画帧
                        let current_frame = (map_data.animation_count / 10) % cell.tile_animation_frames as i32;
                        let image_index = cell.tile_animation_image as i32 + cell.tile_animation_offset as i32 + current_frame;
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.middle_index,
                            image_index,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let sprite_y = world_pos.y + texture_data.height as f32 / 2.0;

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 1.1),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Middle,
                                    is_animated: true,
                                    animation_index: cell.tile_animation_image,
                                },
                                AnimatedTile {
                                    cell_x: x,
                                    cell_y: y,
                                    layer: TileLayer::Middle,
                                    base_index: cell.tile_animation_image,
                                    frames: cell.tile_animation_frames,
                                    offset: cell.tile_animation_offset as i32,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    // ============ Front层动画瓦片 ============
    if view_settings.show_front {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    if cell.front_animation_frame > 0 {
                        // 计算当前动画帧
                        let current_frame = (map_data.animation_count / 10) % cell.front_animation_frame as i32;
                        let image_index = cell.front_image + current_frame;
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.front_index,
                            image_index,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            let mut sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let mut sprite_y = world_pos.y + texture_data.height as f32 / 2.0;

                            // 大型物体Y偏移
                            let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
                                         (texture_data.width != 96 || texture_data.height != 64);
                            if is_large {
                                sprite_y += CELL_HEIGHT as f32 - texture_data.height as f32;
                            }

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 2.1),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Front,
                                    is_animated: true,
                                    animation_index: cell.front_image as i16,
                                },
                                AnimatedTile {
                                    cell_x: x,
                                    cell_y: y,
                                    layer: TileLayer::Front,
                                    base_index: cell.front_image as i16,
                                    frames: cell.front_animation_frame,
                                    offset: 0,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// 网格渲染系统
// ============================================================================

fn render_grid_system(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    view_settings: Res<ViewSettings>,
    camera_query: Query<(&Transform, &MapCamera)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !view_settings.show_grid || map_data.width == 0 {
        return;
    }

    let Ok((_, camera)) = camera_query.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    // 计算可见区域
    let half_width = window.width() / 2.0 / camera.zoom;
    let half_height = window.height() / 2.0 / camera.zoom;

    let left = camera.target.x - half_width - 500.0;
    let right = camera.target.x + half_width + 500.0;
    let top = camera.target.y + half_height + 500.0;
    let bottom = camera.target.y - half_height - 500.0;

    // 网格线范围
    let grid_left = (left / CELL_WIDTH as f32).floor() * CELL_WIDTH as f32;
    let grid_right = (right / CELL_WIDTH as f32).ceil() * CELL_WIDTH as f32;
    let grid_top = (top / CELL_HEIGHT as f32).ceil() * CELL_HEIGHT as f32;
    let grid_bottom = (bottom / CELL_HEIGHT as f32).floor() * CELL_HEIGHT as f32;

    let color = Color::srgba(0.0, 1.0, 0.0, 0.3);

    // 垂直线
    let mut x = grid_left;
    while x <= grid_right {
        gizmos.line(
            Vec3::new(x, grid_bottom, 0.5),
            Vec3::new(x, grid_top, 0.5),
            color,
        );
        x += CELL_WIDTH as f32;
    }

    // 水平线
    let mut y = grid_bottom;
    while y <= grid_top {
        gizmos.line(
            Vec3::new(grid_left, y, 0.5),
            Vec3::new(grid_right, y, 0.5),
            color,
        );
        y += CELL_HEIGHT as f32;
    }
}

// ============================================================================
// UI更新系统
// ============================================================================

fn update_info_text_system(
    map_data: Res<MapData>,
    view_settings: Res<ViewSettings>,
    camera_query: Query<&MapCamera>,
    windows: Query<&Window, With<PrimaryWindow>>,
    diagnostics: Res<DiagnosticsStore>,
    mut text_query: Query<(&mut Text, &mut Visibility), With<InfoText>>,
) {
    let Ok((mut text, mut visibility)) = text_query.single_mut() else {
        return;
    };

    // 根据设置显示/隐藏
    *visibility = if view_settings.show_info {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    if !view_settings.show_info {
        return;
    }

    let Ok(camera) = camera_query.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    // 获取FPS
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);

    // 获取鼠标位置对应的地图格子
    let (grid_x, grid_y) = if let Some(cursor_pos) = window.cursor_position() {
        let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        let cursor_offset = cursor_pos - screen_center;
        let world_pos = camera.target + cursor_offset / camera.zoom;
        world_to_map(world_pos)
    } else {
        (-1, -1)
    };

    // 更新文本
    text.0 = format!(
        "Bevy地图查看器 - {}\n\
        \n\
        地图: {} ({}x{})\n\
        相机: ({:.0}, {:.0}) 缩放: {:.2}x\n\
        鼠标: 格子({}, {})\n\
        FPS: {:.1}\n\
        \n\
        [M] 加载地图\n\
        [G] 网格: {}\n\
        [1] Back层: {}\n\
        [2] Middle层: {}\n\
        [3] Front层: {}\n\
        [I] 隐藏信息\n\
        [鼠标中键] 拖拽\n\
        [滚轮] 缩放",
        env!("CARGO_PKG_VERSION"),
        map_data.map_name,
        map_data.width,
        map_data.height,
        camera.target.x,
        camera.target.y,
        camera.zoom,
        grid_x,
        grid_y,
        fps,
        if view_settings.show_grid { "✓" } else { "✗" },
        if view_settings.show_back { "✓" } else { "✗" },
        if view_settings.show_middle { "✓" } else { "✗" },
        if view_settings.show_front { "✓" } else { "✗" },
    );
}
