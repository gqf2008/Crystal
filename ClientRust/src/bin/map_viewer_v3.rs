// ============================================================================
// Map Viewer V3 - 基于新 ECS 架构的地图查看器
// ============================================================================
//
// 功能：
// - 使用新的 SystemScheduler 调度器
// - 使用 GameContext 事件系统
// - 使用 MockNetwork 模拟网络（无需真实服务器）
// - 支持地图浏览、缩放、拖拽
//
// 用法：
//   cargo run --bin map_viewer_v3
//
// 控制：
//   鼠标拖拽 - 移动地图
//   滚轮     - 缩放
//   ESC      - 退出
//   G        - 切换网格显示
//   O        - 切换障碍显示
//
// ============================================================================

mod map_viewer;

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::EventHandler;
use ggez::graphics::{Canvas, Color};
use ggez::GameResult;

use map_viewer::scene::MapViewerScene;
use mir2_client::ecs::components::InputEvent;
use mir2_client::ecs::game_context::GameContextBuilder;
use mir2_client::ecs::scenes::Scene;
use mir2_client::ecs::{ime_handler, GameContext};
use mir2_client::settings::ClientSettings;

/// 地图查看器应用
struct MapViewerApp {
    /// 地图查看器场景
    scene: MapViewerScene,
}

impl MapViewerApp {
    fn new(ctx: &mut GameContext) -> GameResult<Self> {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        tracing::info!("🗺️  Map Viewer V3 启动中...");
        
        // 输出实际屏幕尺寸
        let screen_rect = ctx.gfx.drawable_size();
        tracing::info!("📐 ggez 实际渲染尺寸: {}x{}", screen_rect.0, screen_rect.1);
        
        // 创建场景
        let scene = MapViewerScene::new(ctx)?;

        // 🎮 发送开始游戏请求，触发 MockNetwork 加载地图
        use mir2_client::network::handlers::GameEvent;
        let net_ctx = ctx.world.network();
        if let Err(e) = net_ctx.send(GameEvent::StartGameRequest { character_index: 0 }) {
            tracing::error!("❌ 发送 StartGameRequest 失败: {}", e);
        } else {
            tracing::info!("📤 已发送 StartGameRequest，等待 MockNetwork 加载地图");
        }

        tracing::info!("✅ Map Viewer V3 启动完成");

        Ok(Self { scene })
    }

    // 网络功能已禁用
}

impl EventHandler<GameContext> for MapViewerApp {
    fn update(&mut self, ctx: &mut GameContext) -> GameResult {
        // ✅ 创建 GameContext 并传递给场景
        ctx.collect_network_events();
        self.scene.update(ctx)?;
        ctx.clear_frame_events();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut GameContext) -> GameResult {
        let (ctx, world) = ctx.split_gfx_world();
        self.scene.draw(ctx, world)?;
        Ok(())
    }

    fn mouse_enter_or_leave(
        &mut self,
        ctx: &mut GameContext,
        entered: bool,
    ) -> Result<(), ggez::GameError> {
        tracing::debug!(
            "🖱️ 鼠标{}",
            if entered {
                "进入窗口"
            } else {
                "离开窗口"
            }
        );
        ctx.push_input_event(InputEvent::MouseEnterOrLeave { entered });

        Ok(())
    }

    fn mouse_wheel_event(
        &mut self,
        ctx: &mut GameContext,
        x: f32,
        y: f32,
    ) -> Result<(), ggez::GameError> {
        tracing::debug!("🖱️ 鼠标滚轮: ({:.1}, {:.1})", x, y);
        ctx.push_input_event(InputEvent::MouseWheel { x, y });

        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        ctx: &mut GameContext,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    ) -> Result<(), ggez::GameError> {
        // 取消详细日志,避免刷屏
        // tracing::debug!("🖱️ 鼠标移动: ({:.1}, {:.1}), Δ({:.1}, {:.1})", x, y, dx, dy);
        ctx.push_input_event(InputEvent::MouseMove { x, y, dx, dy });

        Ok(())
    }

    // 注意: 鼠标按钮事件不需要推送到InputEvent,系统会直接从ctx.mouse读取状态
    // PlayerControlSystem 使用 ctx.mouse.button_pressed() 来检测按钮状态
    // 键盘事件也不需要推送,DebugSystem 使用 ctx.input().key_pressed() 直接检测

    fn text_input_event(
        &mut self,
        ctx: &mut GameContext,
        character: char,
    ) -> Result<(), ggez::GameError> {
        tracing::debug!("⌨️ 输入字符: '{}'", character);
        ctx.push_input_event(InputEvent::Ime {
            character,
            timestamp: std::time::Instant::now(),
        });
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut GameContext, width: f32, height: f32) -> Result<(), ggez::GameError> {
        tracing::info!("🔄 窗口尺寸改变: 新尺寸 = {:.1}x{:.1}", width, height);
        ctx.push_input_event(InputEvent::Resize { width, height });
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let mut settings = ClientSettings::default();
    settings.network.use_mock = true;
    let (mut ctx, event_loop) = GameContextBuilder::new("map_viewer_v3", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title("Map Viewer V3 - Crystal")
                .vsync(true), // 启用垂直同步，限制帧率到屏幕刷新率
        )
        .window_mode(
            WindowMode::default()
                .dimensions(1600.0, 1200.0)
                .resizable(true)
                .resize_on_scale_factor_change(true),
        )
        .with_font(
            "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf",
            "AlibabaPuHuiTi",
        )
        .with_settings(settings)
        .build()?;

    // 创建应用
    let app = MapViewerApp::new(&mut ctx)?;

    // 运行事件循环
    ime_handler::run(ctx, event_loop, app)
}
