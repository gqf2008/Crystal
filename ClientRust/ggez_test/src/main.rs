// 独立ggez测试
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::conf::{WindowMode, WindowSetup};
use ggez::graphics::{self, Color, Canvas, Text, DrawParam};

fn main() -> GameResult {
    println!("启动Ggez测试...");
    
    let (mut ctx, event_loop) = ContextBuilder::new("ggez_test", "Crystal")
        .window_setup(WindowSetup::default().title("Ggez测试"))
        .window_mode(WindowMode::default().dimensions(500.0, 400.0))
        .build()?;
    
    println!("Ggez Context创建成功!");
    println!("按ESC退出");
    
    let game = TestGame::new();
    event::run(ctx, event_loop, game)
}

struct TestGame {
    frame: u32,
}

impl TestGame {
    fn new() -> Self {
        Self { frame: 0 }
    }
}

impl EventHandler for TestGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if ctx.keyboard.is_key_pressed(ggez::input::keyboard::KeyCode::Escape) {
            println!("用户退出");
            ctx.request_quit();
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 创建画布
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(40, 70, 100));
        
        // 绘制文本
        let text = Text::new(format!(
            "✓ Ggez 工作正常!\n\n\
             帧数: {}\n\
             FPS: {:.1}\n\n\
             按 ESC 退出",
            self.frame,
            ctx.time.fps(),
        ));
        
        canvas.draw(&text, DrawParam::default().dest([50.0, 50.0]).color(Color::WHITE));
        
        // 完成渲染
        canvas.finish(ctx)?;
        
        self.frame += 1;
        Ok(())
    }
}
