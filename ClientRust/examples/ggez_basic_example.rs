// Ggez 基础示例 - 演示窗口创建、纹理加载、精灵渲染
// 
// 运行命令: cargo run --example ggez_basic_example
//
// 功能演示:
// 1. 创建 800x600 窗口
// 2. 加载纹理 (从内存创建测试图案)
// 3. 渲染精灵 (旋转、缩放、透明度)
// 4. 文本渲染
// 5. 形状绘制 (矩形、线条)
// 6. 帧率显示

use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Color, Image};
use ggez::conf::{WindowMode, WindowSetup};
use std::time::Instant;

fn main() -> GameResult {
    println!("=== Ggez 基础示例 ===");
    println!("ESC 键退出");
    
    // 1. 创建 ggez Context (替代 winit + wgpu 的复杂初始化)
    let (mut ctx, event_loop) = ContextBuilder::new("ggez_example", "Crystal")
        .window_setup(WindowSetup::default().title("Ggez 示例 - Crystal"))
        .window_mode(WindowMode::default().dimensions(800.0, 600.0))
        .build()?;
    
    // 2. 创建游戏状态
    let mut game = ExampleGame::new(&mut ctx)?;
    
    // 3. 运行事件循环 (处理输入、更新、渲染)
    event::run(ctx, event_loop, game)
}

struct ExampleGame {
    test_texture: Image,       // 测试纹理
    rotation: f32,             // 旋转角度
    scale: f32,                // 缩放比例
    alpha: f32,                // 透明度
    frame_count: u32,          // 帧计数
    start_time: Instant,       // 启动时间
}

impl ExampleGame {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        // 创建测试纹理 (64x64 渐变图案)
        let test_texture = create_test_texture(ctx)?;
        
        Ok(Self {
            test_texture,
            rotation: 0.0,
            scale: 1.0,
            alpha: 1.0,
            frame_count: 0,
            start_time: Instant::now(),
        })
    }
}

impl EventHandler for ExampleGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 动画效果
        self.rotation += 0.02;  // 旋转
        
        // 缩放来回变化 (0.5 -> 2.0)
        let time = self.start_time.elapsed().as_secs_f32();
        self.scale = 1.0 + (time * 2.0).sin() * 0.5;
        
        // 透明度来回变化 (0.3 -> 1.0)
        self.alpha = 0.65 + (time * 3.0).sin().abs() * 0.35;
        
        // 检查退出键
        if ctx.keyboard.is_key_pressed(ggez::input::keyboard::KeyCode::Escape) {
            ctx.request_quit();
        }
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 1. 清屏 (深蓝色背景)
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::from_rgb(20, 30, 60));
        
        // 2. 绘制旋转的测试纹理 (中心)
        let center_x = 400.0;
        let center_y = 300.0;
        
        canvas.draw(
            &self.test_texture,
            graphics::DrawParam::default()
                .dest([center_x, center_y])
                .rotation(self.rotation)
                .scale([self.scale, self.scale])
                .offset([0.5, 0.5])  // 中心点旋转
                .color(Color::from_rgba(255, 255, 255, (self.alpha * 255.0) as u8))
        );
        
        // 3. 绘制多个静态精灵 (环绕中心)
        for i in 0..8 {
            let angle = (i as f32) * std::f32::consts::PI / 4.0 + self.rotation * 0.5;
            let radius = 150.0;
            let x = center_x + angle.cos() * radius;
            let y = center_y + angle.sin() * radius;
            
            canvas.draw(
                &self.test_texture,
                graphics::DrawParam::default()
                    .dest([x, y])
                    .scale([0.5, 0.5])
                    .offset([0.5, 0.5])
                    .color(Color::from_rgb(255, 180, 100))
            );
        }
        
        // 4. 绘制矩形 (UI 框架演示)
        let rect = graphics::Rect::new(50.0, 50.0, 200.0, 100.0);
        let rect_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(2.0),
            rect,
            Color::from_rgb(100, 200, 255)
        )?;
        canvas.draw(&rect_mesh, graphics::DrawParam::default());
        
        // 5. 绘制填充矩形 (UI 背景演示)
        let filled_rect = graphics::Rect::new(50.0, 170.0, 200.0, 80.0);
        let filled_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            filled_rect,
            Color::from_rgba(50, 50, 50, 200)
        )?;
        canvas.draw(&filled_mesh, graphics::DrawParam::default());
        
        // 6. 绘制文本
        let fps = self.frame_count as f32 / self.start_time.elapsed().as_secs_f32();
        let text = graphics::Text::new(format!(
            "Ggez 渲染示例\n\
             帧率: {:.1} FPS\n\
             旋转: {:.2} rad\n\
             缩放: {:.2}x\n\
             透明度: {:.2}",
            fps, self.rotation, self.scale, self.alpha
        ));
        
        canvas.draw(
            &text,
            graphics::DrawParam::default()
                .dest([60.0, 60.0])
                .color(Color::WHITE)
        );
        
        // 7. 底部提示文本
        let hint_text = graphics::Text::new("按 ESC 键退出");
        canvas.draw(
            &hint_text,
            graphics::DrawParam::default()
                .dest([300.0, 550.0])
                .color(Color::from_rgb(200, 200, 200))
        );
        
        // 8. 完成渲染
        canvas.finish(ctx)?;
        
        self.frame_count += 1;
        Ok(())
    }
}

/// 创建测试纹理 (64x64 渐变图案)
fn create_test_texture(ctx: &mut Context) -> GameResult<Image> {
    let width = 64u16;
    let height = 64u16;
    let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 4);
    
    // 生成彩色渐变图案
    for y in 0..height {
        for x in 0..width {
            // 径向渐变效果
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let intensity = ((32.0 - distance) / 32.0).max(0.0);
            
            // RGB 颜色基于位置
            let r = ((x as f32 / 64.0) * 255.0) as u8;
            let g = ((y as f32 / 64.0) * 255.0) as u8;
            let b = (intensity * 255.0) as u8;
            let a = 255u8;
            
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }
    
    // 从 RGBA 数据创建纹理 (对比 wgpu 需要 TextureDescriptor + Queue.write_texture)
    Image::from_rgba8(ctx, width, height, &pixels)
}
