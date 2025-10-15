/// 测试角色移动和动画的简化客户端
/// 直接进入 GameScene,跳过登录流程
use ggez::{event, Context, GameResult};
use mir2_client::scenes::game_scene::GameScene;
use mir2_client::scenes::Scene;
use std::sync::Arc;
use tokio::sync::Mutex;

struct TestGameState {
    game_scene: GameScene,
}

impl TestGameState {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        println!("🚀 创建测试游戏场景...");
        
        // 创建 GameScene
        let mut game_scene = GameScene::new(ctx)?;
        
        // 模拟进入游戏 (设置测试数据)
        game_scene.set_test_mode();
        
        println!("✅ 测试场景创建完成!");
        
        Ok(Self { game_scene })
    }
}

impl event::EventHandler<ggez::GameError> for TestGameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.game_scene.update(ctx)?;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.game_scene.draw(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.game_scene.handle_mouse_button(button, true, x, y)?;
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.game_scene.handle_mouse_button(button, false, x, y)?;
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        repeated: bool,
    ) -> GameResult {
        if let Some(keycode) = input.keycode {
            self.game_scene.handle_key_down(keycode)?;
        }
        Ok(())
    }
}

fn main() -> GameResult {
    println!("========================================");
    println!("🎮 角色移动测试客户端");
    println!("========================================\n");

    // 初始化 ggez
    let (mut ctx, event_loop) = ggez::ContextBuilder::new("mir2_movement_test", "Crystal Team")
        .window_mode(ggez::conf::WindowMode::default().dimensions(1024.0, 768.0))
        .build()?;

    // 创建测试状态
    let state = TestGameState::new(&mut ctx)?;

    // 运行游戏循环
    event::run(ctx, event_loop, state)
}
