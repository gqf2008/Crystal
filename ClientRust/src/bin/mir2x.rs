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
    conf::{Conf, NumSamples, WindowMode, WindowSetup},
    filesystem::Filesystem,
};

use mir2_client::ecs::{game_context::GameContextBuilder, runtime::ClientRuntime};
use mir2_client::ecs::{ime_handler, GameContext, GameState};

fn main() -> Result<()> {
    println!("\n========================================");
    println!("🎮 Crystal Mir2 Client - Mir2X (ECS版本)");
    println!("========================================\n");

    // 1. 初始化日志系统 (使用 ClientRuntime 统一方法)
    ClientRuntime::init_logging("info");
    tracing::info!("=== Crystal Mir2 Client (ECS版本) ===");

    // 2. 加载配置 (使用 ClientRuntime 统一方法)
    let settings = ClientRuntime::load_config(false)?;
    tracing::info!("配置加载完成: {:?}", settings.launcher.server_name);

    // 4. 初始化图像库系统 (使用 ClientRuntime 统一方法)
    tracing::info!("🔧 [mir2x.rs] 即将调用 init_graphics_libraries");
    if let Err(e) = ClientRuntime::init_graphics_libraries("Data") {
        tracing::error!("图像库初始化失败: {}", e);
        tracing::warn!("将继续运行,但部分图像可能无法显示");
    } else {
        tracing::info!("✅ [mir2x.rs] 图像库初始化成功");
    }

    // 5. 创建 GameContext (从配置读取窗口尺寸，但强制调整为4:3比例)
    let resolution = settings.resolution();
    tracing::info!(
        "🎨 请求的窗口分辨率: {}x{}",
        resolution.width,
        resolution.height
    );
    // UI纹理是4:3设计，需要强制窗口为4:3比例
    let initial_width = resolution.width as f32;
    let initial_height = (initial_width * 3.0 / 4.0).round(); // 强制4:3比例

    // 创建 Filesystem (使用与 ContextBuilder 相同的参数)
    let settings_clone = settings.clone();
    let (mut ctx, event_loop) = GameContextBuilder::new("mir2x", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!(
                    "Crystal - {} (ECS)",
                    settings.launcher.server_name
                ))
                .samples(NumSamples::Four) // 4x MSAA
                .vsync(true),
        )
        .window_mode(
            WindowMode::default()
                .dimensions(initial_width, initial_height)
                .min_dimensions(initial_width, initial_height)
                .max_dimensions(initial_width, initial_height)
                .resizable(false)
                .maximized(false)
                .resize_on_scale_factor_change(true),
        )
        .with_font(
            "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf",
            "AlibabaPuHuiTi",
        )
        .with_settings(settings_clone)
        .build()?;

    let (w, h) = ctx.drawable_size();
    tracing::info!("🎨 实际创建的窗口分辨率: {}x{}", w, h);

    // 8. 创建游戏应用
    let game = GameState::new(&mut ctx)?;

    // 9. 运行自定义事件循环 (完整支持 IME)
    tracing::info!("启动自定义事件循环 (IME 支持)");
    ime_handler::run(ctx, event_loop, game)?;
    Ok(())
}
