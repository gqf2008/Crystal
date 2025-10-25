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
    conf::{NumSamples, WindowMode, WindowSetup},
    ContextBuilder,
};

use mir2_client::ecs::runtime::ClientRuntime;
use mir2_client::ecs::{ime_handler, GameState};

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

    // 3. 创建 Tokio runtime (使用 ClientRuntime 统一方法)
    let runtime = ClientRuntime::create_tokio_runtime()?;
    let _guard = runtime.enter();
    tracing::info!("✅ Tokio runtime 创建成功");

    // 4. 初始化图像库系统 (使用 ClientRuntime 统一方法)
    println!("🔧🔧🔧 [mir2x.rs] 即将调用 init_graphics_libraries");
    tracing::info!("🔧 [mir2x.rs] 即将调用 init_graphics_libraries");
    
    if let Err(e) = ClientRuntime::init_graphics_libraries("Data") {
        println!("❌❌❌ [mir2x.rs] 图像库初始化失败: {}", e);
        tracing::error!("图像库初始化失败: {}", e);
        tracing::warn!("将继续运行,但部分图像可能无法显示");
    } else {
        println!("✅✅✅ [mir2x.rs] 图像库初始化成功返回");
        tracing::info!("✅ [mir2x.rs] 图像库初始化成功");
    }

    // 5. 创建 ggez Context (从配置读取窗口尺寸，但强制调整为4:3比例)
    let resolution = settings.resolution();
    tracing::info!(
        "🎨 请求的窗口分辨率: {}x{}",
        resolution.width,
        resolution.height
    );
    // UI纹理是4:3设计，需要强制窗口为4:3比例
    let initial_width = resolution.width as f32;
    let initial_height = (initial_width * 3.0 / 4.0).round(); // 强制4:3比例

    let (mut ctx, event_loop) = ContextBuilder::new("mir2x", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!(
                    "Crystal - {} (ECS)",
                    settings.launcher.server_name
                ))
                .samples(NumSamples::Four) // 4x MSAA
                .vsync(true), // 开启垂直同步，锁定 60 FPS
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
        .build()?;
    let (w, h) = ctx.gfx.drawable_size();
    tracing::info!("🎨 实际创建的窗口分辨率: {}x{}", w, h);

    // 6. 添加中文字体支持 (使用 ClientRuntime 统一路径)
    ClientRuntime::load_font_to_context(
        &mut ctx,
        "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf",
        "AlibabaPuHuiTi",
    )?;
    // 8. 创建游戏应用 (传入配置和 runtime，像 CrystalGame 一样)
    let game = GameState::new(&mut ctx, settings)?;

    // 9. 运行自定义事件循环 (完整支持 IME)
    tracing::info!("启动自定义事件循环 (IME 支持)");
    ime_handler::run_with_ime(ctx, event_loop, game)?;
    Ok(())
}
