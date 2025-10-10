// Map Viewer - 独立地图绘制程序
// 功能:
// - 完整地图渲染 (Back/Middle/Front 三层)
// - 鼠标拖拽移动相机
// - 鼠标滚轮缩放
// - 显示坐标和FPS
//
// 运行: cargo run --example map_viewer --release

use ggez::{
    Context, ContextBuilder, GameResult,
    event::{self, EventHandler},
    graphics::{self, Canvas, Color, DrawParam, Text, TextFragment},
    conf::{WindowMode, WindowSetup},
};
use ggez::winit::event::MouseButton;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// 引入项目中的模块
use mir2_client::graphics::mlibrary::MLibrary;
use mir2_client::objects::{MapReader, CellInfo};

/// 相机系统
struct Camera {
    x: f32,           // 世界坐标 X
    y: f32,           // 世界坐标 Y
    zoom: f32,        // 缩放级别 (1.0 = 正常)
    
    // 拖拽状态
    dragging: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    drag_start_cam_x: f32,
    drag_start_cam_y: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_cam_x: 0.0,
            drag_start_cam_y: 0.0,
        }
    }
    
    /// 开始拖拽
    fn start_drag(&mut self, mouse_x: f32, mouse_y: f32) {
        self.dragging = true;
        self.drag_start_x = mouse_x;
        self.drag_start_y = mouse_y;
        self.drag_start_cam_x = self.x;
        self.drag_start_cam_y = self.y;
    }
    
    /// 更新拖拽
    fn update_drag(&mut self, mouse_x: f32, mouse_y: f32) {
        if self.dragging {
            let dx = mouse_x - self.drag_start_x;
            let dy = mouse_y - self.drag_start_y;
            self.x = self.drag_start_cam_x - dx / self.zoom;
            self.y = self.drag_start_cam_y - dy / self.zoom;
        }
    }
    
    /// 结束拖拽
    fn end_drag(&mut self) {
        self.dragging = false;
    }
    
    /// 缩放
    fn zoom_by(&mut self, delta: f32, mouse_x: f32, mouse_y: f32, screen_width: f32, screen_height: f32) {
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(0.25, 4.0);
        
        // 以鼠标位置为中心缩放
        let world_x = self.screen_to_world_x(mouse_x, screen_width);
        let world_y = self.screen_to_world_y(mouse_y, screen_height);
        
        self.x = world_x - (mouse_x - screen_width / 2.0) / self.zoom;
        self.y = world_y - (mouse_y - screen_height / 2.0) / self.zoom;
    }
    
    /// 屏幕坐标转世界坐标
    fn screen_to_world_x(&self, screen_x: f32, screen_width: f32) -> f32 {
        self.x + (screen_x - screen_width / 2.0) / self.zoom
    }
    
    fn screen_to_world_y(&self, screen_y: f32, screen_height: f32) -> f32 {
        self.y + (screen_y - screen_height / 2.0) / self.zoom
    }
    
    /// 世界坐标转屏幕坐标
    fn world_to_screen_x(&self, world_x: f32, screen_width: f32) -> f32 {
        (world_x - self.x) * self.zoom + screen_width / 2.0
    }
    
    fn world_to_screen_y(&self, world_y: f32, screen_height: f32) -> f32 {
        (world_y - self.y) * self.zoom + screen_height / 2.0
    }
}

/// 地图渲染器
struct MapRenderer {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
    map_libraries: Vec<Option<Arc<Mutex<MLibrary>>>>,
    animation_count: i32,
}

impl MapRenderer {
    // 传奇地图格子尺寸
    // 基础格子: 48x32 (逻辑坐标)
    // 实际瓦片: 96x64 (2x2格子，渲染在偶数坐标)
    const CELL_WIDTH: i32 = 48;   // 单个格子宽度
    const CELL_HEIGHT: i32 = 32;  // 单个格子高度
    
    fn new(reader: MapReader) -> Self {
        // 初始化地图库 (0-399)
        let mut map_libraries = vec![None; 400];
        
        // 加载常用的地图库
        Self::load_map_library(&mut map_libraries, "WemadeMir2/Tiles", 0);
        Self::load_map_library(&mut map_libraries, "WemadeMir2/SmTiles", 1);
        Self::load_map_library(&mut map_libraries, "WemadeMir3/Tiles", 2);
        Self::load_map_library(&mut map_libraries, "Shanda/Tiles", 4);
        Self::load_map_library(&mut map_libraries, "Shanda/SmTiles", 5);
        
        // 加载扩展库 (6-199)
        for i in 6..=199 {
            let path = format!("WemadeMir2/Tiles{}", i);
            Self::load_map_library(&mut map_libraries, &path, i);
        }
        
        Self {
            cells: reader.map_cells,
            width: reader.width,
            height: reader.height,
            map_libraries,
            animation_count: 0,
        }
    }
    
    fn load_map_library(libraries: &mut Vec<Option<Arc<Mutex<MLibrary>>>>, name: &str, index: usize) {
        let path = format!("Data/Map/{}", name);
        match MLibrary::open(&path) {
            Ok(lib) => {
                println!("✅ [{}] {}", index, name);
                libraries[index] = Some(Arc::new(Mutex::new(lib)));
            }
            Err(e) => {
                // 只在非 NotFound 错误时打印
                if e.kind() != std::io::ErrorKind::NotFound {
                    println!("⚠️  [{}] {} - {}", index, name, e);
                }
            }
        }
    }
    
    /// 获取地图库
    fn get_library(&self, index: i16) -> Option<Arc<Mutex<MLibrary>>> {
        let idx = index as usize;
        if idx < self.map_libraries.len() {
            self.map_libraries[idx].clone()
        } else {
            None
        }
    }
    
    /// 获取单元格
    fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            Some(&self.cells[x as usize][y as usize])
        } else {
            None
        }
    }
    
    /// 渲染地图
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, camera: &Camera, screen_width: f32, screen_height: f32) -> GameResult<()> {
        // 更新动画计数器
        self.animation_count = (self.animation_count + 1) % 1000;
        
        // 计算可见区域 (世界坐标转地图格子)
        let left = camera.screen_to_world_x(0.0, screen_width);
        let right = camera.screen_to_world_x(screen_width, screen_width);
        let top = camera.screen_to_world_y(0.0, screen_height);
        let bottom = camera.screen_to_world_y(screen_height, screen_height);
        
        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32 - 2).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32 + 2).min(self.width - 1);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32 - 2).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32 + 2).min(self.height - 1);
        
        // ========================================
        // BACK LAYER (地表层)
        // ========================================
        // 传奇地图特点：Back层只渲染偶数行列，通过大瓦片(48x32)覆盖相邻格子
        for y in (start_y..=end_y).step_by(2) {  // 只处理偶数行
            for x in (start_x..=end_x).step_by(2) {  // 只处理偶数列
                if let Some(cell) = self.get_cell(x, y) {
                    if cell.back_image > 0 && cell.back_index >= 0 {
                        // 屏蔽高位标记，获取实际图像索引
                        let masked_back = cell.back_image & 0x1FFFFFFF;
                        let index = (masked_back as usize).saturating_sub(1);
                        
                        // C# 公式: drawX = (x - User.X + OffSetX) * CellWidth - OffSetX
                        // 简化版：直接使用格子坐标 * 格子宽度
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;
                        
                        self.draw_tile(ctx, canvas, camera, screen_width, screen_height,
                                     cell.back_index as i32, index, world_x, world_y)?;
                    }
                }
            }
        }
        
        // ========================================
        // MIDDLE LAYER (建筑层)
        // ========================================
        // 渲染所有格子，不限奇偶
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    // 屏蔽 HighWall 标记 (0x20000000)
                    let middle_image_masked = cell.middle_image & 0x1FFFFFFF;
                    if middle_image_masked > 0 && cell.middle_index >= 0 {
                        let mut index = (middle_image_masked as i32).saturating_sub(1);
                        let animation = cell.middle_animation_frame;
                        
                        // 动画处理
                        if animation > 0 && animation < 255 {
                            let animation_tick = cell.middle_animation_tick;
                            let anim_frames = animation & 0x0f;
                            if anim_frames > 0 {
                                index += (self.animation_count % (anim_frames as i32 + (anim_frames as i32 * animation_tick as i32))) 
                                    / (1 + animation_tick as i32);
                            }
                        }
                        
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;
                        
                        self.draw_tile(ctx, canvas, camera, screen_width, screen_height,
                                     cell.middle_index as i32, index as usize, world_x, world_y)?;
                    }
                }
            }
        }
        
        // ========================================
        // FRONT LAYER (前景层)
        // ========================================
        // 渲染建筑顶部、树冠等前景物体
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    // 屏蔽高位标记 (0x8000 = LowWall)
                    let front_image = cell.front_image & 0x7FFF;
                    if front_image > 0 && cell.front_index >= 0 {
                        let mut index = (front_image as i32).saturating_sub(1);
                        let animation = cell.front_animation_frame & 0x7F;
                        
                        // 动画处理
                        if animation > 0 {
                            let animation_tick = cell.front_animation_tick;
                            index += (self.animation_count % (animation as i32 + (animation as i32 * animation_tick as i32))) 
                                / (1 + animation_tick as i32);
                        }
                        
                        // Front层不需要向上偏移，让图像的offset自然处理
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;
                        
                        self.draw_tile(ctx, canvas, camera, screen_width, screen_height,
                                     cell.front_index as i32, index as usize, world_x, world_y)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 绘制瓦片
    /// 
    /// C# 绘制逻辑详解:
    /// ```csharp
    /// // GameScene.cs DrawFloor() - 计算格子的屏幕坐标
    /// drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
    /// drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
    /// 
    /// // MLibrary.cs Draw(int index, int x, int y) - 直接绘制
    /// DXManager.Draw(mi.Image, 
    ///     new Rectangle(0, 0, mi.Width, mi.Height),  // 源矩形：完整纹理
    ///     new Vector3((float)x, (float)y, 0.0F),     // 目标位置：不应用offset
    ///     Color.White);
    /// ```
    /// 
    /// **关键发现**：
    /// 1. C# 的 Draw(index, x, y) 重载 **不应用 mi.X/mi.Y offset**
    /// 2. 只有 Draw(index, point, color, offSet=true) 才会应用 offset
    /// 3. 地图瓦片绘制使用前者，所以offset在格子坐标计算时已经体现
    /// 4. **但是**：对于 Middle/Front 层的大型物体（树、建筑），offset 用于定位
    fn draw_tile(&self, ctx: &mut Context, canvas: &mut Canvas, camera: &Camera, 
                 screen_width: f32, screen_height: f32,
                 lib_index: i32, image_index: usize, 
                 world_x: f32, world_y: f32) -> GameResult<()> {
        if let Some(map_lib) = self.get_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();
            
            // 使用纹理缓存（先获取纹理再读取 info，避免重复 lock）
            match lib.get_or_create_texture(ctx, image_index) {
                Ok(texture) => {
                    // 转换到屏幕坐标（不应用 offset）
                    // C# 逻辑：MLibrary.Draw(index, x, y) 直接绘制，不应用 mi.X/mi.Y
                    // 只有 Draw(index, point, color, offSet=true) 才应用 offset
                    let screen_x = camera.world_to_screen_x(world_x, screen_width);
                    let screen_y = camera.world_to_screen_y(world_y, screen_height);
                    
                    // 绘制纹理（使用 Alpha 混合）
                    canvas.draw(texture, DrawParam::default()
                        .dest([screen_x, screen_y])
                        .scale([camera.zoom, camera.zoom]));
                }
                Err(_e) => {
                    // 忽略加载错误（避免日志刷屏）
                }
            }
        }
        
        Ok(())
    }
}

/// 主程序状态
struct MapViewerState {
    camera: Camera,
    map_renderer: MapRenderer,
    screen_width: f32,
    screen_height: f32,
    fps_timer: Instant,
    frame_count: u32,
    fps: u32,
    map_name: String,
}

impl MapViewerState {
    fn new(ctx: &mut Context, map_path: &str) -> GameResult<Self> {
        println!("\n🗺️  加载地图: {}", map_path);
          let (screen_width, screen_height) = ctx.gfx.drawable_size();
        println!("🔍 初始窗口尺寸: {}x{}", screen_width, screen_height);
        // 加载地图
        let reader = MapReader::new(map_path)
            .map_err(|e| {
                eprintln!("❌ 加载地图失败: {}", e);
                ggez::GameError::ResourceLoadError(format!("Failed to load map: {}", e))
            })?;
        
        println!("✅ 地图加载成功: {}x{}", reader.width, reader.height);
        
        let map_renderer = MapRenderer::new(reader);
        
        // 相机初始位置设置为地图左上角附近（可以看到内容的位置）
        let mut camera = Camera::new();
        camera.x =screen_width/2.0;  // 屏幕宽度的一半
        camera.y = screen_height/2.0;  // 屏幕高度的一半
        camera.zoom = 1.0;
        
        println!("📍 相机初始位置: ({}, {})", camera.x, camera.y);
        println!("🎯 地图像素尺寸: {}x{} 像素", 
            map_renderer.width * MapRenderer::CELL_WIDTH,
            map_renderer.height * MapRenderer::CELL_HEIGHT);
        
      
        
        Ok(Self {
            camera,
            map_renderer,
            screen_width,
            screen_height,
            fps_timer: Instant::now(),
            frame_count: 0,
            fps: 0,
            map_name: map_path.to_string(),
        })
    }
}

impl EventHandler for MapViewerState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 更新FPS
        self.frame_count += 1;
        if self.fps_timer.elapsed().as_secs() >= 1 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer = Instant::now();
        }
        
        // 更新屏幕尺寸
        let (w, h) = ctx.gfx.drawable_size();
        self.screen_width = w;
        self.screen_height = h;
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 使用深灰色背景，更容易看清地图边界
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(32, 32, 32));
        
        // 设置混合模式为标准 Alpha 混合
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);
        
        // 绘制地图
        self.map_renderer.draw(ctx, &mut canvas, &self.camera, self.screen_width, self.screen_height)?;
        
        // 绘制UI信息
        self.draw_ui(ctx, &mut canvas)?;
        
        canvas.finish(ctx)?;
        Ok(())
    }
    
    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        if button == MouseButton::Left {
            self.camera.start_drag(x, y);
        }
        Ok(())
    }
    
    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> GameResult {
        if button == MouseButton::Left {
            self.camera.end_drag();
        }
        Ok(())
    }
    
    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult {
        self.camera.update_drag(x, y);
        Ok(())
    }
    
    fn mouse_wheel_event(&mut self, ctx: &mut Context, _x: f32, y: f32) -> GameResult {
        let mouse_pos = ctx.mouse.position();
        let mouse_x = mouse_pos.x;
        let mouse_y = mouse_pos.y;
        self.camera.zoom_by(y, mouse_x, mouse_y, self.screen_width, self.screen_height);
        Ok(())
    }
}

impl MapViewerState {
    fn draw_ui(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 计算当前鼠标悬停的地图格子坐标
        let mouse_pos = ctx.mouse.position();
        let mouse_x = mouse_pos.x;
        let mouse_y = mouse_pos.y;
        let world_x = self.camera.screen_to_world_x(mouse_x, self.screen_width);
        let world_y = self.camera.screen_to_world_y(mouse_y, self.screen_height);
        let grid_x = (world_x / MapRenderer::CELL_WIDTH as f32).floor() as i32;
        let grid_y = (world_y / MapRenderer::CELL_HEIGHT as f32).floor() as i32;
        
        // 构建UI文本
        let ui_text = format!(
            "Map: {}\nSize: {}x{}\nFPS: {}\nZoom: {:.2}x\nCamera: ({:.0}, {:.0})\nGrid: ({}, {})\n\n按住左键拖拽 | 滚轮缩放",
            self.map_name,
            self.map_renderer.width, self.map_renderer.height,
            self.fps,
            self.camera.zoom,
            self.camera.x, self.camera.y,
            grid_x, grid_y
        );
        
        // 绘制半透明背景
        let bg_rect = graphics::Rect::new(10.0, 10.0, 300.0, 180.0);
        let bg_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());
        
        // 绘制文本
        let text = Text::new(TextFragment {
            text: ui_text,
            color: Some(Color::WHITE),
            font: Some("LiberationMono-Regular".into()),
            scale: Some(graphics::PxScale::from(16.0)),
        });
        
        canvas.draw(&text, DrawParam::default().dest([20.0, 20.0]));
        
        Ok(())
    }
}

fn main() -> GameResult {
    // 从命令行参数获取地图路径
    let args: Vec<String> = std::env::args().collect();
    let map_path = if args.len() > 1 {
        args[1].clone()
    } else {
        // 默认地图
        "Map/0.map".to_string()
    };
    
    println!("\n╔══════════════════════════════════════════╗");
    println!("║     地图查看器 - Map Viewer v1.0        ║");
    println!("╚══════════════════════════════════════════╝");
    println!("\n📖 使用说明:");
    println!("  - 按住左键拖拽移动视角");
    println!("  - 鼠标滚轮缩放");
    println!("  - ESC 退出");
    println!("\n🎮 启动参数: cargo run --example map_viewer [地图路径]");
    println!("   示例: cargo run --example map_viewer Map/0.map\n");
    
    // 创建窗口
    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer", "Mir2")
        .window_setup(WindowSetup::default().title("Map Viewer - 传奇地图查看器"))
        .window_mode(WindowMode::default()
            .dimensions(1280.0, 960.0)
            .resizable(true))
        .build()?;
    
    // 创建状态
    let state = MapViewerState::new(&mut ctx, &map_path)?;
    
    // 运行
    event::run(ctx, event_loop, state)
}
