// Scene Demo - 场景系统演示

use client_macroquad::game_state::GameState;

#[macroquad::main("场景系统演示")]
async fn main() {
    println!("🚀 启动场景系统演示...");
    
    match GameState::new().await {
        Ok(game_state) => {
            if let Err(e) = game_state.run().await {
                eprintln!("❌ 运行时错误: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("❌ 初始化失败: {:?}", e);
        }
    }
    
    println!("✅ 程序退出");
}
