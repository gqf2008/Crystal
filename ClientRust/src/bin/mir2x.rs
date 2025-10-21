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

use anyhow::Result;
use ggez::{
    conf::{WindowMode, WindowSetup, NumSamples},
    event,
    ContextBuilder,
};
use tracing::info;

use mir2_client::ecs::GameState;
use mir2_client::program::ClientRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n========================================");
    println!("🎮 Crystal Mir2 Client - Mir2X (ECS版本)");
    println!("========================================\n");

    // 1. 初始化日志系统 (使用 ClientRuntime 统一方法)
    ClientRuntime::init_logging("debug");
    tracing::info!("=== Crystal Mir2 Client (ECS版本) ===");
    
    // 2. 加载配置 (使用 ClientRuntime 统一方法)
    let settings = ClientRuntime::load_config(false)?;
    tracing::info!("配置加载完成: {:?}", settings.launcher.server_name);
    
    // 3. 创建 Tokio runtime (使用 ClientRuntime 统一方法)
    let runtime = ClientRuntime::create_tokio_runtime()?;
     let _guard = runtime.enter();
    tracing::info!("✅ Tokio runtime 创建成功");
    
    // 4. 初始化图像库系统 (使用 ClientRuntime 统一方法)
    if let Err(e) = ClientRuntime::init_graphics_libraries("Data") {
        tracing::error!("图像库初始化失败: {}", e);
        tracing::warn!("将继续运行,但部分图像可能无法显示");
    }
    
    // 5. 创建 ggez Context (使用配置中的分辨率)
    let res = settings.resolution();
    let window_width = res.width as f32;
    let window_height = res.height as f32;
    
    let (mut ctx, event_loop) = ContextBuilder::new("mir2x", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!("Crystal - {} (ECS)", settings.launcher.server_name))
                .samples(NumSamples::Four)  // 4x MSAA
                .vsync(true)  // 开启垂直同步，锁定 60 FPS
        )
        .window_mode(
            WindowMode::default()
                .dimensions(window_width, window_height)
                .resizable(false)
        )
        .build()?;
    
    info!(
        "Ggez Context 创建成功: {}x{} (vsync开启)",
        window_width, window_height
    );
    
    // 6. 添加中文字体支持 (使用 ClientRuntime 统一路径)
    ClientRuntime::load_font_to_context(&mut ctx, "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf", "AlibabaPuHuiTi")?;
    
    // 7. 启用文本输入 (IME)
    ctx.gfx.window().set_ime_allowed(true);
    tracing::info!("IME 文本输入已启用");
    
    // 8. 创建游戏应用 (传入配置和 runtime，像 CrystalGame 一样)
    let game = GameState::new(&mut ctx, settings)?;
    
    // 9. 运行游戏循环
    event::run(ctx, event_loop, game)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
