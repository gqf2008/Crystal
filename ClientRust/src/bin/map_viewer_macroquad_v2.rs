// ============================================================================
// 传奇2地图查看器 - Macroquad 版本 V2
// ============================================================================
//
// 说明：
// - 使用 macroquad 原生 API（不依赖 mir2_client::renderer 抽象层）
// - 实现 RenderTarget 虚拟分辨率（1600x1200 → 1024x768）
// - Camera2D 相机控制（拖拽、缩放）
// - 等待 MapReader 和 MLibrary 移植完成后实现真实地图渲染
//
// 运行方式：
// cargo run --bin map_viewer_macroquad_v2 --no-default-features --features backend-macroquad
// ============================================================================

use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;
use macroquad::texture::RenderTarget as MQRenderTarget;
use macroquad::text::draw_text_ex;

use macroquad_profiler::ProfilerParams;
use mir2_client::backends::macroquad::{LibraryManager, MeshMapRenderer};
use mir2_client::objects::MapReader; // 使用完整的 MapReader

// ============================================================================
// 常量配置
// ============================================================================

/// 窗口尺寸（实际显示窗口大小）
const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

/// 虚拟渲染尺寸（相机视野范围,大于窗口尺寸会缩小显示）
/// 1600x1200 的内容会被缩放到 1024x768 窗口显示
const RENDER_WIDTH: f32 = 1600.0;
const RENDER_HEIGHT: f32 = 1200.0;

/// 传奇2 瓦片尺寸
const TILE_WIDTH: f32 = 48.0;
const TILE_HEIGHT: f32 = 32.0;

/// 调试：首次渲染标志
static mut FIRST_RENDER: bool = true;

// ============================================================================
// 主程序
// ============================================================================

struct MapViewerState {
    
    /// 相机
    camera: Camera2D,
    
    /// 相机位置（世界坐标 - 相机看向的中心点）
    camera_position: Vec2,
    
    /// 缩放级别
    zoom: f32,
    
    /// 是否正在拖拽
    dragging: bool,
    
    /// 上次鼠标位置
    last_mouse_pos: Vec2,
    
    /// 是否显示网格
    show_grid: bool,
    
    /// 是否显示纹理边框
    show_texture_border: bool,
    
    /// 层级显示控制
    show_back_layer: bool,
    show_middle_layer: bool,
    show_front_layer: bool,
    
    /// FPS 计数
    frame_count: u32,
    fps_timer: f32,
    current_fps: u32,
    
    /// 渲染统计
    tiles_rendered: u32,
    
    /// 字体
    font: Option<Font>,
    
    /// 地图数据
    map_reader: Option<MapReader>,
    
    /// 精灵管理器（加载真实图块纹理）
    library_manager: LibraryManager,
    
    /// 地图渲染器（使用Mesh Batching优化）
    map_renderer: MeshMapRenderer,
    
    /// 鼠标世界坐标
    mouse_world_pos: Vec2,
    
    /// 鼠标对应的地图格子坐标
    mouse_tile_x: i32,
    mouse_tile_y: i32,
}

impl MapViewerState {
    async fn new() -> Result<Self, String> {
        // 创建 Camera2D (初始值,会在 update_camera 中更新)
        let camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(2.0 / RENDER_WIDTH, 2.0 / RENDER_HEIGHT),
            offset: vec2(0.0, 0.0),
            render_target: None,
            rotation: 0.0,
            viewport: None,
        };
        
        // 加载字体
        let font_data = include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let font = load_ttf_font_from_bytes(font_data)
            .map_err(|e| format!("加载字体失败: {}", e))?;
        
        // 加载地图（使用 n0.map - 新手村，有漂亮的地图动画）
        println!("🗺️ 正在加载地图...");
        let map_reader = match MapReader::new("Map/n0.map") {
            Ok(reader) => {
                println!("✅ 地图加载成功: {}x{}", reader.width, reader.height);
                Some(reader)
            }
            Err(e) => {
                println!("⚠️ 地图加载失败: {} (将显示占位符)", e);
                None
            }
        };
        
        // 创建库管理器并加载图块库
        println!("📦 正在加载图块库...");
        let library_manager = LibraryManager::new("Data");
        
        // 加载所有地图库 (MapLib_0 到 MapLib_399)
        if let Err(e) = library_manager.load_map_libraries() {
            println!("⚠️ 地图库加载失败: {}", e);
        }
        
        println!("✅ 图块库加载完成");
        
        // 如果地图加载成功，设置相机初始位置到地图中心
        let camera_position = if let Some(ref map) = map_reader {
            // 将相机设置到地图中心
            let center_x = (map.width as f32 / 2.0) * TILE_WIDTH;
            let center_y = (map.height as f32 / 2.0) * TILE_HEIGHT;
            println!("🎯 初始相机位置: ({:.0}, {:.0})", center_x, center_y);
            println!("📏 地图尺寸: {}x{} 格子", map.width, map.height);
            println!("📏 世界尺寸: {:.0}x{:.0} 像素", 
                map.width as f32 * TILE_WIDTH, 
                map.height as f32 * TILE_HEIGHT
            );
            vec2(center_x, center_y)
        } else {
            vec2(0.0, 0.0)
        };
        
        // 创建地图渲染器 (使用Mesh Batching)
        let map_renderer = MeshMapRenderer::new(TILE_WIDTH, TILE_HEIGHT);
        
        Ok(Self {
          
            camera,
            camera_position,
            zoom: 1.0,  // 初始缩放：1.0 = 正常大小
            dragging: false,
            last_mouse_pos: vec2(0.0, 0.0),
            show_grid: false,
            show_texture_border: false,
            show_back_layer: true,
            show_middle_layer: true,
            show_front_layer: true,
            frame_count: 0,
            fps_timer: 0.0,
            current_fps: 0,
            tiles_rendered: 0,
            font: Some(font),
            map_reader,
            library_manager,
            map_renderer,
            mouse_world_pos: vec2(0.0, 0.0),
            mouse_tile_x: 0,
            mouse_tile_y: 0,
        })
    }
    
    fn update(&mut self) {
        let dt = get_frame_time();
        
        // 更新地图渲染器（动画计数器）
        self.map_renderer.update(dt);
        
        // 计算鼠标世界坐标
        let mouse_screen = mouse_position();
        // 屏幕坐标 -> 渲染目标坐标
        let mouse_render_x = (mouse_screen.0 / WINDOW_WIDTH as f32) * RENDER_WIDTH;
        let mouse_render_y = (mouse_screen.1 / WINDOW_HEIGHT as f32) * RENDER_HEIGHT;
        
        // 渲染目标坐标 -> 世界坐标
        // 相机位置是世界坐标中心点，鼠标相对于渲染中心的偏移除以缩放后加上相机位置
        let mouse_offset_x = (mouse_render_x - RENDER_WIDTH / 2.0) / self.zoom;
        let mouse_offset_y = (mouse_render_y - RENDER_HEIGHT / 2.0) / self.zoom;
        
        self.mouse_world_pos.x = self.camera_position.x + mouse_offset_x;
        self.mouse_world_pos.y = self.camera_position.y + mouse_offset_y;
        
        // 世界坐标 -> 地图格子坐标
        self.mouse_tile_x = (self.mouse_world_pos.x / TILE_WIDTH).floor() as i32;
        self.mouse_tile_y = (self.mouse_world_pos.y / TILE_HEIGHT).floor() as i32;
        
                // FPS 计算
        self.frame_count += 1;
        self.fps_timer += dt;
        if self.fps_timer >= 1.0 {
            self.current_fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer -= 1.0;
        }
        
        // 键盘输入处理
        
        // 键盘输入
        if is_key_pressed(KeyCode::Escape) {
            std::process::exit(0);
        }
        
        if is_key_pressed(KeyCode::G) {
            self.show_grid = !self.show_grid;
            println!("🔲 网格显示: {}", if self.show_grid { "开启" } else { "关闭" });
        }
        
        if is_key_pressed(KeyCode::B) {
            self.show_texture_border = !self.show_texture_border;
            println!("🖼️ 纹理边框: {}", if self.show_texture_border { "开启" } else { "关闭" });
        }
        
        // 层级控制
        if is_key_pressed(KeyCode::Key1) {
            self.show_back_layer = !self.show_back_layer;
            println!("🗺️ Back层(背景): {}", if self.show_back_layer { "显示" } else { "隐藏" });
        }
        
        if is_key_pressed(KeyCode::Key2) {
            self.show_middle_layer = !self.show_middle_layer;
            println!("🗺️ Middle层(中间): {}", if self.show_middle_layer { "显示" } else { "隐藏" });
        }
        
        if is_key_pressed(KeyCode::Key3) {
            self.show_front_layer = !self.show_front_layer;
            println!("🗺️ Front层(前景): {}", if self.show_front_layer { "显示" } else { "隐藏" });
        }
        
        // D键：输出鼠标所在格子的详细数据
        if is_key_pressed(KeyCode::D) {
            if let Some(map) = &self.map_reader {
                if let Some(cell) = map.get_cell(self.mouse_tile_x, self.mouse_tile_y) {
                    cell.debug_cell_data(self.mouse_tile_x, self.mouse_tile_y);
                } else {
                    println!("⚠️ 格子({},{}) 超出地图范围", self.mouse_tile_x, self.mouse_tile_y);
                }
            }
        }
        
        // R键：重置相机到地图中心
        if is_key_pressed(KeyCode::R) {
            if let Some(map) = &self.map_reader {
                let center_x = (map.width as f32 / 2.0) * TILE_WIDTH;
                let center_y = (map.height as f32 / 2.0) * TILE_HEIGHT;
                self.camera_position = vec2(center_x, center_y);
                self.zoom = 1.0;
                println!("🎯 相机重置到地图中心: ({:.0}, {:.0})", center_x, center_y);
            }
        }
        
        // H键：显示帮助和当前状态
        if is_key_pressed(KeyCode::H) {
            println!("\n📊 当前状态:");
            println!("   相机位置: ({:.0}, {:.0})", self.camera_position.x, self.camera_position.y);
            println!("   缩放倍数: {:.2}x", self.zoom);
            if let Some(map) = &self.map_reader {
                println!("   地图大小: {}x{}", map.width, map.height);
                println!("   地图中心: ({:.0}, {:.0})", 
                    (map.width as f32 / 2.0) * TILE_WIDTH,
                    (map.height as f32 / 2.0) * TILE_HEIGHT
                );
            }
            println!("   鼠标格子: ({}, {})", self.mouse_tile_x, self.mouse_tile_y);
        }
        
        // 鼠标滚轮缩放
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            let zoom_factor = if wheel > 0.0 { 1.1 } else { 0.9 };
            self.zoom *= zoom_factor;
            // 限制缩放范围：最小0.3x（防止渲染过多格子导致卡顿），最大5.0x
            self.zoom = self.zoom.clamp(0.3, 5.0);
        }
        
        // 鼠标拖拽
        if is_mouse_button_pressed(MouseButton::Left) {
            self.dragging = true;
            self.last_mouse_pos = mouse_position().into();
        }
        
        if is_mouse_button_released(MouseButton::Left) {
            self.dragging = false;
        }
        
        if self.dragging {
            let current_pos: Vec2 = mouse_position().into();
            let delta = current_pos - self.last_mouse_pos;
            
            // 拖动逻辑：拖动地图（而不是拖动摄像机）
            // 1. delta 是屏幕坐标的变化 (1024x768)
            // 2. 现在渲染坐标和窗口坐标相同 (1024x768)，所以直接转换为世界坐标
            // 3. 鼠标往右拖 -> delta.x > 0 -> 想看左边的地图 -> 相机往左移 -> camera.x -= delta
            let screen_to_render_x = RENDER_WIDTH / WINDOW_WIDTH as f32;
            let screen_to_render_y = RENDER_HEIGHT / WINDOW_HEIGHT as f32;
            
            let world_delta_x = delta.x * screen_to_render_x / self.zoom;
            let world_delta_y = delta.y * screen_to_render_y / self.zoom;
            
            // 鼠标往右拖，相机往左移，看到左边的地图（拖动地图的效果）
            self.camera_position.x -= world_delta_x;
            self.camera_position.y -= world_delta_y;
            
            // 更新鼠标位置
            self.last_mouse_pos = current_pos;
        }
    }
    
    /// 更新相机参数
    fn update_camera(&mut self) {
        self.camera.target = self.camera_position;
        self.camera.zoom = vec2(
            2.0 / RENDER_WIDTH * self.zoom,
            2.0 / RENDER_HEIGHT * self.zoom
        );
    }
    
    fn draw(&mut self) {
        // 清空屏幕
        clear_background(Color::from_rgba(40, 40, 50, 255));
        
        // 更新并设置游戏相机
        self.update_camera();
        set_camera(&self.camera);
        
        // 绘制地图
        if self.map_reader.is_some() {
            self.draw_map();
        } else {
            self.draw_placeholder();
        }
        
        // 绘制网格
        if self.show_grid {
            self.draw_grid();
        }
        
        // 不切换相机,继续在同一相机空间绘制UI (这会让UI也跟随相机移动)
        self.draw_ui();
    }
    
    /// 绘制占位符（当没有地图时）
    fn draw_placeholder(&self) {
        // 绘制一个简单的棋盘格作为占位符
        let grid_size = 100.0;
        let start_x = ((self.camera_position.x - RENDER_WIDTH / 2.0) / grid_size).floor() as i32 - 1;
        let start_y = ((self.camera_position.y - RENDER_HEIGHT / 2.0) / grid_size).floor() as i32 - 1;
        let end_x = start_x + (RENDER_WIDTH / grid_size).ceil() as i32 + 2;
        let end_y = start_y + (RENDER_HEIGHT / grid_size).ceil() as i32 + 2;
        
        for y in start_y..end_y {
            for x in start_x..end_x {
                let color = if (x + y) % 2 == 0 {
                    Color::from_rgba(60, 60, 70, 255)
                } else {
                    Color::from_rgba(50, 50, 60, 255)
                };
                
                draw_rectangle(
                    x as f32 * grid_size,
                    y as f32 * grid_size,
                    grid_size,
                    grid_size,
                    color,
                );
            }
        }
        
        // 中心标记
        draw_circle(0.0, 0.0, 20.0, RED);
        draw_line(-50.0, 0.0, 50.0, 0.0, 3.0, WHITE);
        draw_line(0.0, -50.0, 0.0, 50.0, 3.0, WHITE);
    }
    
    /// 绘制真实地图 (参考 ggez 版本的 MapRenderSystem)
    fn draw_map(&mut self) {
        let map = match &self.map_reader {
            Some(m) => m,
            None => return,
        };
        
        // 使用 MapRenderer 渲染地图
        self.tiles_rendered = self.map_renderer.render(
            map,
            &self.library_manager,
            self.camera_position.x,
            self.camera_position.y,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            self.zoom,
            self.show_back_layer,
            self.show_middle_layer,
            self.show_front_layer,
            self.show_texture_border,
        );
        
        // 首次渲染标记
        unsafe {
            if FIRST_RENDER {
                FIRST_RENDER = false;
            }
        }
    }
    
    /// 绘制网格（地图格子网格，48x32）
    fn draw_grid(&self) {
        let map = match &self.map_reader {
            Some(m) => m,
            None => return,
        };
        
        // 计算视口范围
        let half_width = (RENDER_WIDTH / 2.0) / self.zoom;
        let half_height = (RENDER_HEIGHT / 2.0) / self.zoom;
        
        let view_left = self.camera_position.x - half_width;
        let view_right = self.camera_position.x + half_width;
        let view_top = self.camera_position.y - half_height;
        let view_bottom = self.camera_position.y + half_height;
        
        // 转换为格子坐标
        let start_x = ((view_left / TILE_WIDTH).floor() as i32).max(0);
        let start_y = ((view_top / TILE_HEIGHT).floor() as i32).max(0);
        let end_x = ((view_right / TILE_WIDTH).ceil() as i32 + 1).min(map.width);
        let end_y = ((view_bottom / TILE_HEIGHT).ceil() as i32 + 1).min(map.height);
        
        let grid_color = Color::from_rgba(255, 255, 0, 80); // 半透明黄色网格
        let text_color = Color::from_rgba(255, 255, 0, 255); // 不透明黄色文字
        
        // 绘制竖线和格子标注
        for grid_x in start_x..=end_x {
            let world_x = grid_x as f32 * TILE_WIDTH;
            
            let y1 = start_y as f32 * TILE_HEIGHT;
            let y2 = end_y as f32 * TILE_HEIGHT;
            
            draw_line(
                world_x,
                y1,
                world_x,
                y2,
                1.0,
                grid_color,
            );
        }
        
        // 绘制横线
        for grid_y in start_y..=end_y {
            let world_y = grid_y as f32 * TILE_HEIGHT;
            
            draw_line(
                start_x as f32 * TILE_WIDTH,
                world_y,
                end_x as f32 * TILE_WIDTH,
                world_y,
                1.0,
                grid_color,
            );
        }
        
        // 在每个格子中心绘制坐标
        // 只在缩放较大时显示,避免过于密集
        if self.zoom >= 0.8 {
            for grid_y in start_y..end_y {
                for grid_x in start_x..end_x {
                    // 每隔2个格子显示一次,避免太密集
                    if grid_x % 2 == 0 && grid_y % 2 == 0 {
                        let center_x = grid_x as f32 * TILE_WIDTH + TILE_WIDTH / 2.0;
                        let center_y = grid_y as f32 * TILE_HEIGHT + TILE_HEIGHT / 2.0;
                        
                        let label = format!("{},{}", grid_x, grid_y);
                        draw_text_ex(
                            &label,
                            center_x - 15.0,
                            center_y + 3.0,
                            TextParams {
                                font: self.font.as_ref(),
                                font_size: 10,
                                color: text_color,
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }
    
    /// 绘制 UI（屏幕空间，不受相机影响）
    fn draw_ui(&self) {
        // 标题
        draw_text_ex(
            "传奇2地图查看器 - Macroquad V2",
            10.0,
            35.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 32,
                color: WHITE,
                ..Default::default()
            },
        );
        
        // 状态信息
        let map_info = if let Some(ref map) = self.map_reader {
            format!("地图: {}x{} | ", map.width, map.height)
        } else {
            "地图: 未加载 | ".to_string()
        };
        
        let info = format!(
            "{}FPS: {} | 缩放: {:.1}x | 相机: ({:.0}, {:.0}) | 瓦片: {}",
            map_info,
            self.current_fps,
            self.zoom,
            self.camera_position.x,
            self.camera_position.y,
            self.tiles_rendered,
        );
        
        draw_text_ex(
            &info,
            10.0,
            75.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 20,
                color: WHITE,
                ..Default::default()
            },
        );
        
        // 层级状态
        let layer_status = format!(
            "Layers: Back[{}] Mid[{}] Front[{}] | Grid[{}] Border[{}]",
            if self.show_back_layer { "ON" } else { "  " },
            if self.show_middle_layer { "ON" } else { "  " },
            if self.show_front_layer { "ON" } else { "  " },
            if self.show_grid { "ON" } else { "  " },
            if self.show_texture_border { "ON" } else { "  " },
        );
        
        draw_text_ex(
            &layer_status,
            10.0,
            105.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 18,
                color: WHITE,
                ..Default::default()
            },
        );
        
        // 控制提示
        draw_text_ex(
            "控制: 拖拽移动 | 滚轮缩放 | 1/2/3 层级 | G 网格 | B 边框 | ESC 退出",
            10.0,
            135.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 16,
                color: WHITE,
                ..Default::default()
            },
        );
        
        // 鼠标位置信息
        self.draw_mouse_info();
    }
    
    /// 绘制鼠标位置和瓦片信息
    fn draw_mouse_info(&self) {
        let y_offset = 140.0;
        
        // 鼠标世界坐标
        let world_info = format!(
            "鼠标: 世界({:.1}, {:.1}) 格子({}, {})",
            self.mouse_world_pos.x,
            self.mouse_world_pos.y,
            self.mouse_tile_x,
            self.mouse_tile_y,
        );
        
        draw_text_ex(
            &world_info,
            10.0,
            y_offset,
            TextParams {
                font: self.font.as_ref(),
                font_size: 14,
                color: Color::from_rgba(255, 255, 100, 255),
                ..Default::default()
            },
        );
        
        // 获取瓦片信息
        if let Some(ref map) = self.map_reader {
            if let Some(cell) = map.get_cell(self.mouse_tile_x, self.mouse_tile_y) {
                let mut line_offset = 20.0;
                
                // Back 层信息 (总是显示,即使没有纹理)
                match cell.back_tile() {
                    Some((file_idx, img_idx)) => {
                        // 计算实际文件名: MapLib_100=Tiles, 101=Tiles2, 104=Tiles5
                        let actual_file = if file_idx == 100 {
                            "Tiles.Lib".to_string()
                        } else if file_idx >= 101 && file_idx <= 109 {
                            format!("Tiles{}.Lib", file_idx - 99)
                        } else if file_idx == 110 {
                            "SmTiles.Lib".to_string()
                        } else if file_idx >= 111 && file_idx <= 119 {
                            format!("SmTiles{}.Lib", file_idx - 109)
                        } else if file_idx == 190 {
                            "AniTiles1.Lib".to_string()
                        } else {
                            format!("Unknown_{}.Lib", file_idx)
                        };
                        
                        let back_info = format!(
                            "  Back:   MapLib_{}={}, 图像={}, back_image=0x{:X}",
                            file_idx, actual_file, img_idx, cell.back_image
                        );
                        draw_text_ex(
                            &back_info,
                            10.0,
                            y_offset + line_offset,
                            TextParams {
                                font: self.font.as_ref(),
                                font_size: 13,
                                color: WHITE,
                                ..Default::default()
                            },
                        );
                    }
                    None => {
                        let back_info = format!(
                            "  Back:   -1 (无纹理) back_image=0x{:X}",
                            cell.back_image
                        );
                        draw_text_ex(
                            &back_info,
                            10.0,
                            y_offset + line_offset,
                            TextParams {
                                font: self.font.as_ref(),
                                font_size: 13,
                                color: WHITE,
                                ..Default::default()
                            },
                        );
                    }
                }
                
                line_offset += 20.0;
                
                // Middle层信息 (总是显示)
                let middle_info = match cell.middle_tile() {
                    Some((file_idx, img_idx)) => {
                        format!(
                            "  Middle: MapLib_{} 图像={} (index={}, image=0x{:X})",
                            file_idx, img_idx, cell.middle_index, cell.middle_image
                        )
                    }
                    None => {
                        format!(
                            "  Middle: -1 (无纹理) (index={}, image=0x{:X})",
                            cell.middle_index, cell.middle_image
                        )
                    }
                };
                draw_text_ex(
                    &middle_info,
                    10.0,
                    y_offset + line_offset,
                    TextParams {
                        font: self.font.as_ref(),
                        font_size: 13,
                        color: WHITE,
                        ..Default::default()
                    },
                );
                
                line_offset += 20.0;
                
                // Front层信息 (总是显示)
                let front_info = match cell.front_tile() {
                    Some((file_idx, img_idx)) => {
                        format!(
                            "  Front:  MapLib_{} 图像={} (index={}, image=0x{:X})",
                            file_idx, img_idx, cell.front_index, cell.front_image
                        )
                    }
                    None => {
                        format!(
                            "  Front:  -1 (无纹理) (index={}, image=0x{:X})",
                            cell.front_index, cell.front_image
                        )
                    }
                };
                draw_text_ex(
                    &front_info,
                    10.0,
                    y_offset + line_offset,
                    TextParams {
                        font: self.font.as_ref(),
                        font_size: 13,
                        color: WHITE,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ============================================================================
// 入口函数
// ============================================================================

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2地图查看器 - Macroquad V2 (无VSync)".to_string(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: false,
        platform: Platform {
            swap_interval: Some(1),  // 0 = 关闭VSync, 1 = 60Hz, 2 = 30Hz
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // 初始化
    let mut state = MapViewerState::new().await.expect("初始化失败");
    
    println!("✅ 地图查看器启动成功");
    println!("📐 窗口尺寸: {}x{}", WINDOW_WIDTH, WINDOW_HEIGHT);
    println!("🖼️ 渲染尺寸: {}x{}", RENDER_WIDTH as i32, RENDER_HEIGHT as i32);
    println!("🎮 控制: 拖拽移动 | 滚轮缩放 | 1/2/3 层级 | G 网格 | B 边框 | ESC 退出");
    
    // 主循环
    loop {
        state.update();
        state.draw();
        
        macroquad_profiler::profiler(ProfilerParams::default());
        next_frame().await;
    }
}
