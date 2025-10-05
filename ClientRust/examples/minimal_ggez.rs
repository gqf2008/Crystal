// 最简ggez示例 - 仅验证ggez是否工作
// 不依赖项目其他模块

use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::conf::{WindowMode, WindowSetup};
use ggez::graphics::{self, Color, Canvas, Text};

fn main() -> GameResult {
    println!("=== 最简Ggez示例 ===");
    
    let (mut ctx, event_loop) = ContextBuilder::new("minimal", "Crystal")
        .window_setup(WindowSetup::default().title("最简Ggez示例"))
        .window_mode(WindowMode::default().dimensions(400.0, 300.0))
        .build()?;
    
    let game = MinimalGame::new();
    
    event::run(ctx, event_loop, game)
}

struct MinimalGame {
    frame_count: u32,
}

impl MinimalGame {
    fn new() -> Self {
        Self { frame_count: 0 }
    }
}

impl EventHandler for MinimalGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if ctx.keyboard.is_key_pressed(ggez::input::keyboard::KeyCode::Escape) {
            ctx.request_quit();
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(30, 60, 90));
        
        let text = Text::new(format!(
            "Ggez 工作正常!\n\n\
             帧数: {}\n\
             FPS: {:.1}\n\n\
             按 ESC 退出",
            self.frame_count,
            ctx.time.fps(),
        ));
        
        canvas.draw(&text, graphics::DrawParam::default().dest([50.0, 50.0]));
        
        canvas.finish(ctx)?;
        self.frame_count += 1;
        Ok(())
    }
}
