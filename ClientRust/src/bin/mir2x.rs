// ============================================================================
// Mir2X - 完整传奇2客户端 (基于 ECS + 网络)
// ============================================================================
//
// 功能:
// - 完整的游戏客户端
// - 网络连接到服务器
// - 登录/角色选择
// - 游戏场景
// - 多人在线同步
//
// 运行: cargo run --bin mir2x --release
//
// ============================================================================

use ggez::{
    conf::{WindowMode, WindowSetup},
    event,
    Context, ContextBuilder, GameResult,
};

use mir2_client::ecs::GameState;

fn main() -> GameResult {
    println!("\n========================================");
    println!("🎮 Crystal Mir2 Client - Mir2X");
    println!("========================================\n");
    
    // 创建 GGEZ 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("mir2x", "gqf2008")
        .window_setup(WindowSetup::default().title("传奇2 Rust客户端 - Mir2X"))
        .window_mode(WindowMode::default().dimensions(1280.0, 720.0).resizable(true))
        .build()?;

    // 创建游戏应用
    let game = GameState::new(&mut ctx)?;

    // 运行游戏循环
    event::run(ctx, event_loop, game)
}
