/// 简化的地图查看器 - 使用 GameScene 的 MapRenderer
/// 
/// 用途：独立测试地图渲染，验证 MapRenderer 是否正常工作
/// 
/// 运行方式：
/// ```bash
/// cargo run --bin simple_map_viewer -- 0
/// ```

use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Color, DrawParam, Canvas};
use ggez::input::keyboard::KeyInput;
use ggez::{Context, GameResult, ContextBuilder};
use ggez::conf::{WindowMode, WindowSetup};
use std::env;

// 引入 mir2_client 的模块
use mir2_client::scenes::game_scene::map_renderer::MapRenderer;
use mir2_client::scenes::game_scene::camera::Camera;
use mir2_client::objects::MapReader;  // MapReader 在 objects 模块中

struct MapViewerApp {
    map_renderer: MapRenderer,
    camera: Camera,
    map_name: String,
    camera_speed: f32,
}

impl MapViewerApp {
    fn new(ctx: &mut Context, map_path: &str) -> GameResult<Self> {
        println!("========================================");
        println!("🗺️  传奇地图查看器 (使用 GameScene MapRenderer)");
        println!("========================================");
        
        // 🔧 初始化图形库（必须！MapRenderer 需要纹理才能渲染）
        println!("📦 正在加载图形库...");
        use mir2_client::graphics::libraries::initialize_all_libraries;
        initialize_all_libraries("Data")
            .map_err(|e| {
                eprintln!("❌ 图形库加载失败: {}", e);
                ggez::GameError::CustomError(format!("Failed to load libraries: {}", e))
            })?;
        println!("✅ 图形库加载成功!");
        
        println!("📂 加载地图: {}", map_path);
        
        // 加载地图文件
        let map_reader = MapReader::new(map_path)
            .map_err(|e| {
                eprintln!("❌ 加载地图失败: {}", e);
                ggez::GameError::CustomError(format!("Failed to load map: {}", e))
            })?;
        
        println!("✅ 地图加载成功:");
        println!("   - 尺寸: {} x {} 格子", map_reader.width, map_reader.height);
        println!("   - 格子数: {} 个", map_reader.map_cells.len());
        println!("   - 像素: {:.0} x {:.0} 像素", 
            map_reader.width as f32 * 48.0,
            map_reader.height as f32 * 32.0);
        
        // 创建 MapRenderer
        let map_renderer = MapRenderer::from_reader(map_reader);
        
        println!("✅ MapRenderer 创建成功");
        
        // 创建摄像机（窗口大小）
        let (width, height) = ctx.gfx.drawable_size();
        let mut camera = Camera::new(width, height);
        
        // 摄像机居中在地图中心
        let map_center_x = (map_renderer.width as f32 * 48.0) / 2.0;
        let map_center_y = (map_renderer.height as f32 * 32.0) / 2.0;
        camera.follow_target(map_center_x, map_center_y);
        
        println!("🎥 摄像机初始化:");
        println!("   - 地图中心: ({:.1}, {:.1})", map_center_x, map_center_y);
        println!("   - 摄像机位置: ({:.1}, {:.1})", camera.x, camera.y);
        println!("   - 屏幕尺寸: {:.0} x {:.0}", width, height);
        println!("");
        println!("⌨️  操作说明:");
        println!("   - 方向键: 移动摄像机");
        println!("   - +/-: 缩放");
        println!("   - Home: 回到地图中心");
        println!("   - ESC: 退出");
        println!("========================================");
        
        Ok(Self {
            map_renderer,
            camera,
            map_name: map_path.to_string(),
            camera_speed: 100.0, // 增加速度以便更明显
        })
    }
}

impl EventHandler for MapViewerApp {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // 更新逻辑在这里
        // 键盘输入在 key_down_event 中处理
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        
        // 更新屏幕尺寸
        let (width, height) = ctx.gfx.drawable_size();
        self.camera.update_screen_size(width, height);
        
        // 渲染地图
        self.map_renderer.draw(ctx, &mut canvas, &self.camera)?;
        
        // 绘制信息文本
        let info = format!(
            "地图: {} | 相机: ({:.0}, {:.0}) | 缩放: {:.2}x | FPS: {:.0}",
            self.map_name,
            self.camera.x,
            self.camera.y,
            self.camera.zoom,
            ctx.time.fps()
        );
        
        let text = graphics::Text::new(info);
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([10.0, 10.0])
                .color(Color::WHITE),
        );
        
        // 🎯 绘制视觉辅助元素
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        
        // 1. 屏幕边框(显示可见区域边界)
        let border = graphics::Mesh::new_line(
            ctx,
            &[
                [5.0, 5.0],
                [width - 5.0, 5.0],
                [width - 5.0, height - 5.0],
                [5.0, height - 5.0],
                [5.0, 5.0],
            ],
            3.0,
            Color::from_rgb(0, 255, 255),
        )?;
        canvas.draw(&border, DrawParam::default());
        
        // 2. 十字准星(标记屏幕中心)
        let crosshair_size = 20.0;
        let h_line = graphics::Mesh::new_line(
            ctx,
            &[
                [center_x - crosshair_size, center_y],
                [center_x + crosshair_size, center_y],
            ],
            2.0,
            Color::from_rgb(255, 0, 0),
        )?;
        canvas.draw(&h_line, DrawParam::default());
        
        let v_line = graphics::Mesh::new_line(
            ctx,
            &[
                [center_x, center_y - crosshair_size],
                [center_x, center_y + crosshair_size],
            ],
            2.0,
            Color::from_rgb(255, 0, 0),
        )?;
        canvas.draw(&v_line, DrawParam::default());
        
        // 3. 中心点
        let center_point = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            [center_x, center_y],
            3.0,
            0.1,
            Color::from_rgb(255, 255, 0),
        )?;
        canvas.draw(&center_point, DrawParam::default());
        
        canvas.finish(ctx)?;
        Ok(())
    }
    
    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        use ggez::winit::keyboard::{PhysicalKey, KeyCode as KC};
        
        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            match keycode {
                KC::Equal | KC::NumpadAdd => {
                    // 放大
                    self.camera.zoom = (self.camera.zoom * 1.1).min(3.0);
                    println!("🔍 缩放: {:.2}x", self.camera.zoom);
                }
                KC::Minus | KC::NumpadSubtract => {
                    // 缩小
                    self.camera.zoom = (self.camera.zoom * 0.9).max(0.5);
                    println!("🔍 缩放: {:.2}x", self.camera.zoom);
                }
                KC::Escape => {
                    println!("👋 退出地图查看器");
                    std::process::exit(0);
                }
                KC::Home => {
                    // 回到地图中心
                    let map_center_x = (self.map_renderer.width as f32 * 48.0) / 2.0;
                    let map_center_y = (self.map_renderer.height as f32 * 32.0) / 2.0;
                    self.camera.follow_target(map_center_x, map_center_y);
                    println!("🏠 摄像机回到中心: ({:.1}, {:.1})", self.camera.x, self.camera.y);
                }
                KC::ArrowLeft => {
                    self.camera.x -= self.camera_speed * 5.0;
                    println!("⬅️ 摄像机向左: ({:.1}, {:.1})", self.camera.x, self.camera.y);
                }
                KC::ArrowRight => {
                    self.camera.x += self.camera_speed * 5.0;
                    println!("➡️ 摄像机向右: ({:.1}, {:.1})", self.camera.x, self.camera.y);
                }
                KC::ArrowUp => {
                    self.camera.y -= self.camera_speed * 5.0;
                    println!("⬆️ 摄像机向上: ({:.1}, {:.1})", self.camera.x, self.camera.y);
                }
                KC::ArrowDown => {
                    self.camera.y += self.camera_speed * 5.0;
                    println!("⬇️ 摄像机向下: ({:.1}, {:.1})", self.camera.x, self.camera.y);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn main() -> GameResult {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();
    
    let map_path = if args.len() > 1 {
        format!("Map/{}.map", args[1])
    } else {
        println!("用法: simple_map_viewer <地图编号>");
        println!("示例: simple_map_viewer 0");
        println!("使用默认地图: Map/0.map");
        "Map/0.map".to_string()
    };
    
    // 创建 ggez 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("simple_map_viewer", "Crystal")
        .window_setup(WindowSetup::default().title("传奇地图查看器 - MapRenderer 测试"))
        .window_mode(WindowMode::default().dimensions(1024.0, 768.0))
        .build()?;
    
    // 创建应用
    let app = MapViewerApp::new(&mut ctx, &map_path)?;
    
    // 运行事件循环
    event::run(ctx, event_loop, app)
}
