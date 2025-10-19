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

        // 🐛 针对特定索引的调试
        let is_debug = file_index == 0 && (image_index == 0 || image_index == 1);
        
        if is_debug {
            warn!("🔧 get_map_texture 调用: file_index={}, image_index={}", file_index, image_index);
        }

        // 🔧 修复：索引0是有效的，只拒绝负数
        if image_index < 0 {
            if is_debug {
                warn!("  ❌ image_index < 0, 返回None");
            }
            return None;
        }

        // 缓存key
        let cache_key = format!("Map_{}_{}", file_index, image_index);
        
        // 检查缓存
        if let Some(data) = self.texture_cache.get(&cache_key) {
            if is_debug {
                warn!("  ✅ 从缓存获取: {}x{}", data.width, data.height);
            }
            return Some(data.clone());
        }

        if is_debug {
            warn!("  - 缓存未命中，从MLibrary加载...");
        }

        // 从MLibrary加载
        let mlibrary = get_map_library(file_index)?;
        let mut lib = mlibrary.lock().unwrap();
        
        if is_debug {
            warn!("  - MLibrary获取成功，调用get_image_with_data...");
        }
        
        let (image_info, image_data) = lib.get_image_with_data(image_index as usize).ok()?;

        if is_debug {
            warn!("  - 图像数据获取成功:");
            warn!("    - width: {}", image_info.width);
            warn!("    - height: {}", image_info.height);
            warn!("    - offset: ({}, {})", image_info.x, image_info.y);
            warn!("    - data_len: {} bytes", image_data.len());
        }

        if image_data.is_empty() {
            warn!("❌ 纹理数据为空: file={}, index={}", file_index, image_index);
            return None;
        }

        // 🔧 检查纹理尺寸是否异常
        if image_info.width == 0 || image_info.height == 0 {
            warn!("❌ 纹理尺寸异常: {}x{}, file={}, index={}", 
                  image_info.width, image_info.height, file_index, image_index);
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

        if is_debug {
            warn!("  - BGRA→RGBA转换完成");
            // 检查前几个像素的Alpha值
            if rgba_data.len() >= 16 {
                warn!("  - 前4个像素RGBA值:");
                for i in 0..4 {
                    let offset = i * 4;
                    warn!("    像素{}: R={} G={} B={} A={}", 
                          i, 
                          rgba_data[offset], 
                          rgba_data[offset+1], 
                          rgba_data[offset+2], 
                          rgba_data[offset+3]);
                }
            }
        }

        // 🔧 创建Bevy Image
        let image = Image::new(
            Extent3d {
                width: image_info.width as u32,
                height: image_info.height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba_data,
            TextureFormat::Rgba8UnormSrgb,  // 🔧 使用sRGB格式（颜色纹理）
            Default::default(),
        );

        let handle = images.add(image);
        
        if is_debug {
            warn!("  ✅ Bevy Image创建成功，handle: {:?}", handle);
        }
        
        let texture_data = TextureData {
            handle: handle.clone(),
            width: image_info.width,
            height: image_info.height,
            offset_x: image_info.x,
            offset_y: image_info.y,
        };

        // 缓存
        self.texture_cache.insert(cache_key, texture_data.clone());

        if is_debug {
            warn!("  ✅ 纹理已缓存并返回");
        }

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
    show_border: bool,   // 🆕 显示纹理边框
    flip_texture_y: bool, // 🆕 翻转纹理Y轴（调试用）
    show_info: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            show_back: true,     // ✅ 默认显示Back层
            show_middle: false,  // ❌ 默认隐藏Middle层
            show_front: false,   // ❌ 默认隐藏Front层
            show_grid: false,
            show_border: false,  // 🆕 默认不显示边框
            flip_texture_y: false, // 🆕 默认不翻转（Bevy UV与传奇图像一致）
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
    width: f32,   // 纹理宽度
    height: f32,  // 纹理高度
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
#[derive(Resource)]
struct VisibleArea {
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
    front_end_y: i32,  // 🎨 Front层特殊：向下扩展更多格子
    zoom: f32,
}

impl Default for VisibleArea {
    fn default() -> Self {
        Self {
            start_x: -999999,  // 🔧 使用极端值，确保第一帧必定触发渲染
            end_x: -999999,
            start_y: -999999,
            end_y: -999999,
            front_end_y: -999999,
            zoom: -1.0,
        }
    }
}

/// UI文本标记
#[derive(Component)]
struct InfoText;

/// CellInfo悬停面板标记
#[derive(Component)]
struct CellInfoPanel;

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
            render_sprite_borders_system,  // 🆕 纹理边框绘制
            update_sprite_flip_system,     // 🆕 更新纹理翻转
            // UI更新
            update_info_text_system,
            update_cell_info_panel_system,  // 🆕 CellInfo悬停面板
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
    mut map_data: ResMut<MapData>,
) {
    // 设置黑色背景
    clear_color.0 = Color::srgb(0.0, 0.0, 0.0);

    // 初始化MLibrary资源
    let mlibrary = MLibraryAssets::new(PathBuf::from("Data"));
    commands.insert_resource(mlibrary);

    // 🔧 自动加载默认地图
    if let Ok(reader) = MapReader::new("Map/0.map") {
        *map_data = MapData::from_reader(reader, "Map/0.map".to_string());
        info!("✅ 自动加载默认地图: Map/0.map ({}x{})", map_data.width, map_data.height);
    } else {
        warn!("⚠️ 无法加载默认地图 Map/0.map，请按M键手动加载");
    }

    // 🔧 创建2D相机（设置到地图中心）
    let map_center_x = (map_data.width / 2) as f32 * CELL_WIDTH as f32;
    let map_center_y = -((map_data.height / 2) as f32 * CELL_HEIGHT as f32);  // Y轴翻转
    
    let mut camera = MapCamera::default();
    camera.target = Vec2::new(map_center_x, map_center_y);
    
    commands.spawn((
        Camera2d,
        Projection::from(OrthographicProjection {
            scale: 1.0,  // 初始zoom=1.0
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(map_center_x, map_center_y, 0.0),  // 初始位置设置到地图中心
        camera,
        Name::new("MapCamera"),
    ));
    
    info!("📍 相机初始位置: 地图中心 ({:.1}, {:.1})", map_center_x, map_center_y);

    // 🔧 加载中文字体（从assets/fonts目录）
    let font_handle = asset_server.load("fonts/NotoSansSC-Regular.ttf");

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
            Text::new("Bevy地图查看器 v2.2 - 启动中...\n\n[M] 加载地图\n[G] 网格\n[1/2/3] 图层\n[I] 信息\n[鼠标中键] 拖拽\n[滚轮] 缩放"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(0.1, 0.1, 0.1)),  // 深灰色文字（在浅色背景上更清晰）
            TextFont {
                font: font_handle.clone(),  // 使用中文字体
                font_size: 18.0,  // 稍微小一点，Noto Sans较宽
                ..default()
            },
            InfoText,
            Name::new("Info Text"),
        ));

        // 🖱️ CellInfo悬停面板（初始隐藏）
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,  // 初始隐藏
                width: Val::Px(650.0),
                height: Val::Px(320.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.16, 0.16, 0.16, 0.86)),  // 半透明深灰背景
            BorderColor::from(Color::srgb(0.4, 0.4, 0.4)),  // 边框颜色
            CellInfoPanel,
            Name::new("CellInfo Panel"),
        )).with_children(|panel| {
            panel.spawn((
                Text::new(""),  // 动态更新内容
                TextColor(Color::WHITE),
                TextFont {
                    font: font_handle.clone(),
                    font_size: 16.0,
                    ..default()
                },
                Name::new("CellInfo Text"),
            ));
        });
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

    // B键：切换纹理边框显示
    if keyboard.just_pressed(KeyCode::KeyB) {
        view_settings.show_border = !view_settings.show_border;
        info!("🔲 纹理边框: {}", if view_settings.show_border { "开启" } else { "关闭" });
    }

    // F键：切换纹理Y轴翻转（调试用）
    if keyboard.just_pressed(KeyCode::KeyF) {
        view_settings.flip_texture_y = !view_settings.flip_texture_y;
        info!("🔄 纹理Y翻转: {}", if view_settings.flip_texture_y { "开启" } else { "关闭" });
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
    mut camera_query: Query<(&mut MapCamera, &mut Transform)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok((mut camera, mut transform)) = camera_query.single_mut() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    // 开始拖拽（使用左键）
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(cursor_pos) = window.cursor_position() {
            camera.dragging = true;
            camera.drag_start = cursor_pos;
            camera.drag_start_camera = camera.target;
        }
    }

    // 结束拖拽
    if mouse_button.just_released(MouseButton::Left) {
        camera.dragging = false;
    }

    // 更新拖拽
    if camera.dragging {
        for event in mouse_motion.read() {
            let delta = event.delta;
            camera.target.x -= delta.x / camera.zoom;
            camera.target.y += delta.y / camera.zoom;  // Y轴翻转
        }
        
        // 🔧 关键修复：更新相机Transform（否则地图不会跟随移动！）
        transform.translation.x = camera.target.x;
        transform.translation.y = camera.target.y;
    } else {
        mouse_motion.clear();
    }
}

/// 相机缩放处理（使用Projection.scale）
fn camera_zoom_system(
    mut mouse_wheel: EventReader<MouseWheel>,
    mut camera_query: Query<(&mut MapCamera, &mut Transform, &mut Projection), With<Camera2d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok((mut camera, mut transform, mut projection)) = camera_query.single_mut() else {
        return;
    };

    let Ok(_window) = windows.single() else {
        return;
    };

    for event in mouse_wheel.read() {
        // 🔧 修复：滚轮向上缩小，向下放大（与Windows资源管理器一致）
        let zoom_delta = match event.unit {
            MouseScrollUnit::Line => -event.y,          // 反转方向
            MouseScrollUnit::Pixel => -event.y * 0.01,  // 反转方向
        };

        camera.zoom = (camera.zoom * (1.0 + zoom_delta * 0.1)).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    // 更新相机Transform位置
    transform.translation.x = camera.target.x;
    transform.translation.y = camera.target.y;
    
    // ✅ 使用Projection的scale来实现缩放
    // scale越大，看到的世界越大（缩小效果）
    // zoom小=看更多=scale大
    if let Projection::Orthographic(ref mut ortho) = *projection {
        ortho.scale = 1.0 / camera.zoom;
    }
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
    // ✅ 使用OrthographicProjection.scale时，世界坐标范围会根据scale变化
    // projection.scale = 1.0 / camera.zoom
    // zoom小(0.5) → projection.scale大(2.0) → 看到的世界范围更大
    let projection_scale = 1.0 / camera.zoom;
    let half_width = window.width() / 2.0 * projection_scale;
    let half_height = window.height() / 2.0 * projection_scale;

    let left = camera.target.x - half_width;
    let right = camera.target.x + half_width;
    let top = camera.target.y + half_height;
    let bottom = camera.target.y - half_height;

    // 🔧 动态缓冲区：projection.scale越大，渲染范围越大
    let base_buffer = 6;
    let buffer = (base_buffer as f32 * projection_scale).ceil() as i32;
    
    // 转换为地图格子坐标（扩大边界缓冲，Back层需要更多空间）
    let start_x = ((left / CELL_WIDTH as f32).floor() as i32 - buffer).max(0);
    let end_x = ((right / CELL_WIDTH as f32).ceil() as i32 + buffer).min(map_data.width - 1);
    let start_y = ((-top / CELL_HEIGHT as f32).floor() as i32 - buffer).max(0);
    let end_y = ((-bottom / CELL_HEIGHT as f32).ceil() as i32 + buffer).min(map_data.height - 1);
    
    // 🎨 Front层特殊处理：向下扩展更多格子（建筑物可能很高）
    // 根据projection scale动态调整
    let front_extra_cells = (20.0 * projection_scale).ceil() as i32;
    let front_end_y = (end_y + front_extra_cells).min(map_data.height - 1);

    // 🔍 检测可见区域或缩放是否变化
    let area_changed = visible_area.start_x != start_x
        || visible_area.end_x != end_x
        || visible_area.start_y != start_y
        || visible_area.end_y != end_y
        || visible_area.front_end_y != front_end_y
        || (visible_area.zoom - camera.zoom).abs() > 0.001;

    if !area_changed {
        return;  // ⚡ 可见区域未变化，跳过静态瓦片重建
    }

    // 更新可见区域缓存
    visible_area.start_x = start_x;
    visible_area.end_x = end_x;
    visible_area.start_y = start_y;
    visible_area.end_y = end_y;
    visible_area.front_end_y = front_end_y;
    visible_area.zoom = camera.zoom;

    // 🔍 调试信息
    let grid_width = end_x - start_x + 1;
    let grid_height = end_y - start_y + 1;
    info!("📐 可见区域更新: x=[{}, {}] y=[{}, {}] 范围={}x{} zoom={:.2}", 
          start_x, end_x, start_y, end_y, grid_width, grid_height, camera.zoom);
    info!("📐 世界坐标: left={:.1} right={:.1} top={:.1} bottom={:.1} 宽度={:.1} 高度={:.1}",
          left, right, top, bottom, right - left, top - bottom);
    info!("📐 窗口尺寸: {}x{} 世界半宽={:.1} 世界半高={:.1}",
          window.width(), window.height(), half_width, half_height);

    // 清除所有静态瓦片（但保留动画瓦片）
    for entity in static_tile_query.iter() {
        commands.entity(entity).despawn();
    }

    // ============ Back层渲染（静态） ============
    if view_settings.show_back {
        // 🔧 向下取偶数，确保覆盖所有可见的Back层格子
        let back_start_x = if start_x % 2 == 0 { start_x } else { start_x - 1 };
        let back_start_y = if start_y % 2 == 0 { start_y } else { start_y - 1 };
        
        info!("🟦 Back层范围: x=[{}, {}] y=[{}, {}]", 
              back_start_x, end_x, back_start_y, end_y);
        
        let mut back_tile_count = 0;
        for y in (back_start_y..=end_y).step_by(2) {
            for x in (back_start_x..=end_x).step_by(2) {
                if let Some(cell) = map_data.get_cell(x, y) {
                    // 🐛 调试特定坐标
                    let is_debug_coord = (x == 346 && y == 280) || (x == 350 && y == 278);
                    
                    if is_debug_coord {
                        warn!("🔍 调试坐标 ({}, {}):", x, y);
                        warn!("  - back_image: {:#010x} (原始值)", cell.back_image);
                        warn!("  - back_index: {}", cell.back_index);
                        warn!("  - back_image > 0: {}", cell.back_image > 0);
                    }
                    
                    if cell.back_image > 0 {
                        // 🔧 关键修复：传奇格式需要提取实际索引并减1
                        let texture_index = ((cell.back_image & 0x1FFFFFFF) - 1) as i32;
                        
                        if is_debug_coord {
                            warn!("  - 提取后texture_index: {}", texture_index);
                            warn!("  - 准备调用get_map_texture(back_index={}, texture_index={})", 
                                  cell.back_index, texture_index);
                        }
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.back_index,
                            texture_index,
                            &mut images,
                        ) {
                            if is_debug_coord {
                                warn!("  ✅ 纹理加载成功:");
                                warn!("    - 尺寸: {}x{}", texture_data.width, texture_data.height);
                                warn!("    - offset: ({}, {})", texture_data.offset_x, texture_data.offset_y);
                                warn!("    - handle: {:?}", texture_data.handle);
                            }
                            
                            // 🔧 Back层纹理通常是96x64（覆盖2x2格子）
                            // 但也要处理其他尺寸的纹理
                            let world_pos = map_to_world(x, y);
                            
                            // 🔧 Bevy Sprite锚点在中心
                            // 纹理左上角应该对齐到world_pos
                            let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let sprite_y = world_pos.y - texture_data.height as f32 / 2.0;

                            if is_debug_coord {
                                warn!("  - 世界坐标: ({}, {})", world_pos.x, world_pos.y);
                                warn!("  - Sprite坐标: ({}, {}) Z=0.0", sprite_x, sprite_y);
                            }

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    color: Color::WHITE,
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 0.0),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Back,
                                    is_animated: false,
                                    animation_index: 0,
                                    width: texture_data.width as f32,
                                    height: texture_data.height as f32,
                                },
                            ));
                            back_tile_count += 1;
                        } else {
                            if is_debug_coord {
                                error!("  ❌ 纹理加载失败! get_map_texture返回None");
                            }
                        }
                    }
                }
            }
        }
        info!("✅ Back层绘制完成: {} 个瓦片", back_tile_count);
    }

    // ============ Middle层渲染（仅静态瓦片） ============
    if view_settings.show_middle {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    // Middle静态瓦片
                    if cell.middle_image > 0 {
                        // 🔧 关键修复：传奇格式需要减1
                        let texture_index = (cell.middle_image - 1) as i32;
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.middle_index,
                            texture_index,
                            &mut images,
                        ) {
                            // 过滤非标准尺寸
                            if (texture_data.width == 48 && texture_data.height == 32) ||
                               (texture_data.width == 96 && texture_data.height == 64) {
                                let world_pos = map_to_world(x, y);
                                let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                                let sprite_y = world_pos.y - texture_data.height as f32 / 2.0;  // 🔧 Y轴向上，需要减去

                                commands.spawn((
                                    Sprite {
                                        image: texture_data.handle.clone(),
                                        color: Color::WHITE,  // 🔧 Middle层：不透明，覆盖Back层
                                        ..default()
                                    },
                                    Transform::from_xyz(sprite_x, sprite_y, 1.0),
                                    TileSprite {
                                        grid_x: x,
                                        grid_y: y,
                                        layer: TileLayer::Middle,
                                        is_animated: false,
                                        animation_index: 0,
                                        width: texture_data.width as f32,
                                        height: texture_data.height as f32,
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

    // ============ Front层渲染（统一处理静态、动画、门） ============
    // 🔧 关键修复：参考 map_viewer.rs，统一在一个循环中处理所有Front层瓦片
    // 不要因为有动画标志就跳过绘制！
    if view_settings.show_front {
        for y in start_y..=front_end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    // 提取基础索引
                    let mut index = (cell.front_image & 0x7FFF) - 1;
                    
                    // ⚠️ 过滤无效瓦片（参考 map_viewer.rs）
                    if index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                        continue;
                    }
                    
                    // 检查动画标志
                    let mut animation = cell.front_animation_frame;
                    let use_blend = (animation & 0x80) != 0;  // 混合模式标志
                    animation &= 0x7F;  // 清除标志位，获取真实帧数
                    
                    let has_animation = animation > 0;
                    let has_door = cell.door_index > 0;
                    
                    // 🎬 动画帧推进（如果有动画）
                    if has_animation {
                        let animation_tick = cell.front_animation_tick;
                        let total_frames = animation as i32 + (animation as i32 * animation_tick as i32);
                        let frame_offset = (map_data.animation_count % total_frames) / (1 + animation_tick as i32);
                        index += frame_offset;
                    }
                    
                    // 🚪 门动画处理（如果有门）
                    // 注意：目前简化处理，假设门是关闭的（door_frame = 0）
                    // TODO: 实现门的开关动画逻辑
                    // if has_door {
                    //     let door_frame = self.get_door_frame(cell.door_index);
                    //     if door_frame > 0 {
                    //         index += (door_frame + 1) * cell.door_offset as i32;
                    //     }
                    // }
                    
                    // 获取纹理
                    if let Some(texture_data) = mlibrary.get_map_texture(
                        cell.front_index,
                        index as i32,
                        &mut images,
                    ) {
                        let world_pos = map_to_world(x, y);
                        let mut sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                        
                        // 🔧 Y轴偏移计算（大型物体特殊处理）
                        let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
                                     (texture_data.width != 96 || texture_data.height != 64);
                        
                        let mut sprite_y = if is_large {
                            // 大型物体：底部对齐到格子底部
                            world_pos.y - CELL_HEIGHT as f32 + texture_data.height as f32 - texture_data.height as f32 / 2.0
                        } else {
                            // 标准瓦片
                            world_pos.y - texture_data.height as f32 / 2.0
                        };
                        
                        // 🔥 混合模式偏移（火焰等特效）
                        if use_blend {
                            sprite_x -= CELL_WIDTH as f32;
                            sprite_y += (CELL_HEIGHT * 4) as f32;  // Bevy Y轴向上
                        }
                        
                        // 🎨 颜色/混合模式选择
                        let sprite_color = if use_blend && !has_animation {
                            // 静态混合模式（如固定光效）：更亮
                            Color::srgba(1.5, 1.5, 1.5, 1.0)
                        } else if use_blend && has_animation {
                            // 动画混合模式（如火焰）：模拟ADD混合
                            Color::srgba(1.5, 1.5, 1.5, 0.8)
                        } else {
                            // 普通瓦片：Alpha混合
                            Color::srgba(1.0, 1.0, 1.0, 1.0)
                        };
                        
                        // Z坐标分层：门 > 动画 > 静态
                        let z_order = if has_door {
                            2.2
                        } else if has_animation {
                            2.1
                        } else {
                            2.0
                        };
                        
                        // 🔧 区分静态和动画瓦片：动画瓦片需要添加AnimatedTile组件
                        let base_index = (cell.front_image & 0x7FFF) - 1;
                        
                        if has_animation {
                            // 动画瓦片：添加AnimatedTile组件以便update_animated_tiles_system更新
                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    color: sprite_color,
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, z_order),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Front,
                                    is_animated: true,
                                    animation_index: cell.front_image as i16,
                                    width: texture_data.width as f32,
                                    height: texture_data.height as f32,
                                },
                                AnimatedTile {
                                    cell_x: x,
                                    cell_y: y,
                                    layer: TileLayer::Front,
                                    base_index: base_index as i16,
                                    frames: animation,
                                    offset: cell.front_animation_tick as i32,
                                },
                            ));
                        } else {
                            // 静态瓦片（包括门）：不需要AnimatedTile组件
                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    color: sprite_color,
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, z_order),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Front,
                                    is_animated: false,
                                    animation_index: if has_door { cell.door_index as i16 } else { 0 },
                                    width: texture_data.width as f32,
                                    height: texture_data.height as f32,
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

    let Ok((_camera_transform, camera)) = camera_query.single() else {
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
    let front_end_y = visible_area.front_end_y;  // 🎨 Front层扩展范围

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
                        // 🔧 关键修复：动画基础索引需要减1
                        let base_index = (cell.tile_animation_image - 1) as i32;
                        
                        // 计算当前动画帧
                        let current_frame = (map_data.animation_count / 10) % cell.tile_animation_frames as i32;
                        let image_index = base_index + cell.tile_animation_offset as i32 + current_frame;
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.middle_index,
                            image_index,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            let sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            let sprite_y = world_pos.y - texture_data.height as f32 / 2.0;  // 🔧 Y轴向上

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    color: Color::srgba(1.0, 1.0, 1.0, 0.9),  // 🔧 动画：ADD混合效果（发光）
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 1.1),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Middle,
                                    is_animated: true,
                                    animation_index: cell.tile_animation_image,
                                    width: texture_data.width as f32,
                                    height: texture_data.height as f32,
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
    // 🔧 处理Front层的动画（火焰、光效等）
    if view_settings.show_front {
        for y in start_y..=front_end_y {
            for x in start_x..=end_x {
                if let Some(cell) = map_data.get_cell(x, y) {
                    // 检查动画标志
                    let mut animation = cell.front_animation_frame;
                    let use_blend = (animation & 0x80) != 0;  // 混合模式标志
                    animation &= 0x7F;  // 清除标志位，获取真实帧数
                    
                    if animation > 0 {
                        // 提取基础索引
                        let base_index = (cell.front_image & 0x7FFF) - 1;
                        
                        // ⚠️ 过滤无效瓦片
                        if base_index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                            continue;
                        }
                        
                        // 🎬 计算当前动画帧
                        let animation_tick = cell.front_animation_tick;
                        let total_frames = animation as i32 + (animation as i32 * animation_tick as i32);
                        let frame_offset = (map_data.animation_count % total_frames) / (1 + animation_tick as i32);
                        let image_index = base_index + frame_offset;
                        
                        if let Some(texture_data) = mlibrary.get_map_texture(
                            cell.front_index,
                            image_index,
                            &mut images,
                        ) {
                            let world_pos = map_to_world(x, y);
                            let mut sprite_x = world_pos.x + texture_data.width as f32 / 2.0;
                            
                            // 🔧 Y轴偏移计算
                            let is_large = (texture_data.width != 48 || texture_data.height != 32) &&
                                         (texture_data.width != 96 || texture_data.height != 64);
                            
                            let mut sprite_y = if is_large {
                                world_pos.y - CELL_HEIGHT as f32 + texture_data.height as f32 - texture_data.height as f32 / 2.0
                            } else {
                                world_pos.y - texture_data.height as f32 / 2.0
                            };
                            
                            // 🔥 混合模式偏移
                            if use_blend {
                                sprite_x -= CELL_WIDTH as f32;
                                sprite_y += (CELL_HEIGHT * 4) as f32;
                            }
                            
                            // 🎨 动画混合模式颜色
                            let sprite_color = if use_blend {
                                Color::srgba(1.5, 1.5, 1.5, 0.8)  // ADD混合效果
                            } else {
                                Color::srgba(1.0, 1.0, 1.0, 1.0)  // 普通Alpha混合
                            };

                            commands.spawn((
                                Sprite {
                                    image: texture_data.handle.clone(),
                                    color: sprite_color,
                                    ..default()
                                },
                                Transform::from_xyz(sprite_x, sprite_y, 2.1),
                                TileSprite {
                                    grid_x: x,
                                    grid_y: y,
                                    layer: TileLayer::Front,
                                    is_animated: true,
                                    animation_index: cell.front_image as i16,
                                    width: texture_data.width as f32,
                                    height: texture_data.height as f32,
                                },
                                AnimatedTile {
                                    cell_x: x,
                                    cell_y: y,
                                    layer: TileLayer::Front,
                                    base_index: base_index as i16,
                                    frames: animation,
                                    offset: animation_tick as i32,
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

/// 绘制纹理边框系统
fn render_sprite_borders_system(
    mut gizmos: Gizmos,
    view_settings: Res<ViewSettings>,
    sprite_query: Query<(&Transform, &TileSprite)>,
) {
    if !view_settings.show_border {
        return;
    }

    let border_color = Color::srgba(1.0, 0.0, 0.0, 0.8); // 红色边框

    for (transform, tile_sprite) in sprite_query.iter() {
        let half_width = tile_sprite.width / 2.0;
        let half_height = tile_sprite.height / 2.0;
        
        let center = transform.translation;
        
        // 四个角的位置
        let top_left = Vec3::new(center.x - half_width, center.y + half_height, 10.0);
        let top_right = Vec3::new(center.x + half_width, center.y + half_height, 10.0);
        let bottom_left = Vec3::new(center.x - half_width, center.y - half_height, 10.0);
        let bottom_right = Vec3::new(center.x + half_width, center.y - half_height, 10.0);
        
        // 绘制四条边
        gizmos.line(top_left, top_right, border_color);       // 上边
        gizmos.line(top_right, bottom_right, border_color);   // 右边
        gizmos.line(bottom_right, bottom_left, border_color); // 下边
        gizmos.line(bottom_left, top_left, border_color);     // 左边
    }
}

/// 更新Sprite翻转状态系统
fn update_sprite_flip_system(
    view_settings: Res<ViewSettings>,
    mut sprite_query: Query<&mut Sprite, With<TileSprite>>,
) {
    // 🔧 只有当设置改变时才更新（通过Res<ViewSettings>的变化检测）
    if !view_settings.is_changed() {
        return;
    }

    for mut sprite in sprite_query.iter_mut() {
        sprite.flip_y = view_settings.flip_texture_y;
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
        // 🔧 Bevy的cursor_position Y轴向下，需要翻转
        let cursor_offset = Vec2::new(
            cursor_pos.x - screen_center.x,
            -(cursor_pos.y - screen_center.y),  // Y轴翻转
        );
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
        [B] 纹理边框: {}\n\
        [F] Y轴翻转: {}\n\
        [1] Back层: {}\n\
        [2] Middle层: {}\n\
        [3] Front层: {}\n\
        [I] 隐藏信息\n\
        [鼠标左键] 拖拽\n\
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
        if view_settings.show_border { "✓" } else { "✗" },
        if view_settings.flip_texture_y { "✓" } else { "✗" },
        if view_settings.show_back { "✓" } else { "✗" },
        if view_settings.show_middle { "✓" } else { "✗" },
        if view_settings.show_front { "✓" } else { "✗" },
    );
}

/// 🖱️ CellInfo悬停面板更新系统
fn update_cell_info_panel_system(
    map_data: Res<MapData>,
    camera_query: Query<&MapCamera>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panel_query: Query<&mut Node, With<CellInfoPanel>>,
    panel_children_query: Query<&Children, With<CellInfoPanel>>,
    mut text_query: Query<&mut Text, Without<InfoText>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Ok(camera) = camera_query.single() else {
        return;
    };

    let Ok(mut panel_node) = panel_query.single_mut() else {
        return;
    };

    // 获取CellInfo面板的子Text组件
    let Ok(children) = panel_children_query.single() else {
        return;
    };

    let Some(&text_entity) = children.first() else {
        return;
    };

    let Ok(mut text) = text_query.get_mut(text_entity) else {
        return;
    };

    // 获取鼠标位置对应的地图格子
    let Some(cursor_pos) = window.cursor_position() else {
        // 鼠标不在窗口内，隐藏面板
        panel_node.display = Display::None;
        return;
    };

    let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    // 🔧 Bevy的cursor_position Y轴向下，需要翻转
    let cursor_offset = Vec2::new(
        cursor_pos.x - screen_center.x,
        -(cursor_pos.y - screen_center.y),  // Y轴翻转
    );
    let world_pos = camera.target + cursor_offset / camera.zoom;
    let (grid_x, grid_y) = world_to_map(world_pos);

    // 获取单元格信息
    let Some(cell) = map_data.get_cell(grid_x, grid_y) else {
        // 格子不存在，隐藏面板
        panel_node.display = Display::None;
        return;
    };

    // 显示面板
    panel_node.display = Display::Flex;

    // 🎨 构建CellInfo文本（与ggez版本格式一致）
    let back_lib = if cell.back_index >= 0 { "Tiles" } else { "None" };
    let middle_lib = if cell.middle_index >= 0 { "Smtiles" } else { "None" };
    let front_lib = if cell.front_index >= 0 { "Objects" } else { "None" };

    let back_image_value = cell.back_image & 0x1FFFFFFF;
    let middle_image_value = cell.middle_image;
    let front_image_value = cell.front_image & 0x7FFF;

    // 🔍 计算实际的纹理索引（传奇格式需要-1）
    let back_texture_index = if back_image_value > 0 {
        (back_image_value - 1) as i32
    } else {
        -1
    };
    let middle_texture_index = if middle_image_value > 0 {
        (middle_image_value - 1) as i32
    } else {
        -1
    };
    let front_texture_index = if front_image_value > 0 {
        (front_image_value - 1) as i32
    } else {
        -1
    };

    text.0 = format!(
        "🗺️ 地图坐标: X={}, Y={}\n\n\
        📦 纹理索引信息 (原始值 → 实际索引):\n\
        ┌─────────────────────────────────────────┐\n\
        │ Back层:   {} → {}  (FileIndex: {})\n\
        │ Middle层: {} → {}  (FileIndex: {})\n\
        │ Front层:  {} → {}  (FileIndex: {})\n\
        └─────────────────────────────────────────┘\n\n\
        🏛️ 库文件: Back={} | Middle={} | Front={}\n\n\
        🚫 限制标记:\n\
        • Back HighWall:  {}\n\
        • Front LowWall:  {}\n\n\
        🚪 门: Offset={}  Index={}  Entity={}\n\n\
        💡 光照: {}     🎣 钓鱼: {}",
        grid_x, grid_y,
        back_image_value, back_texture_index, cell.back_index,
        middle_image_value, middle_texture_index, cell.middle_index,
        front_image_value, front_texture_index, cell.front_index,
        back_lib, middle_lib, front_lib,
        if (cell.back_image & 0x20000000) != 0 { "✓" } else { "✗" },
        if (cell.front_image & 0x8000) != 0 { "✓" } else { "✗" },
        cell.door_offset & 0x7F,
        cell.door_index & 0x7F,
        if (cell.door_offset & 0x80) != 0 || (cell.door_index & 0x80) != 0 { "✓" } else { "✗" },
        cell.light,
        "✗"
    );

    // 🖱️ 计算面板位置（跟随鼠标，边界自动翻转）
    let panel_width = 650.0;
    let panel_height = 320.0;
    let offset_x = 20.0;
    let offset_y = 20.0;
    let margin = 10.0;

    let mut panel_x = cursor_pos.x + offset_x;
    let mut panel_y = cursor_pos.y + offset_y;

    // 检查右边界，超出则翻转到鼠标左侧
    if panel_x + panel_width + margin > window.width() {
        panel_x = cursor_pos.x - panel_width - offset_x;
    }

    // 检查下边界，超出则翻转到鼠标上方
    if panel_y + panel_height + margin > window.height() {
        panel_y = cursor_pos.y - panel_height - offset_y;
    }

    // 检查左边界
    if panel_x < margin {
        panel_x = margin;
    }

    // 检查上边界（避开状态栏）
    let status_bar_bottom = 80.0;
    if panel_y < status_bar_bottom {
        panel_y = status_bar_bottom;
    }

    // 更新面板位置
    panel_node.left = Val::Px(panel_x);
    panel_node.top = Val::Px(panel_y);
}
