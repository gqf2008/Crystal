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
use ggez::event::{self, EventHandler};
use ggez::graphics::{Canvas, Color};
use ggez::input::keyboard::KeyInput;
use ggez::winit::event::MouseButton;
use ggez::{Context, ContextBuilder, GameResult};
use hecs::World;

use map_viewer::scene::MapViewerScene;

use mir2_client::ecs::components::InputEvent;
use mir2_client::ecs::scenes::Scene;
use mir2_client::network::{ NetworkBuilder};
use mir2_client::settings::ClientSettings;

/// 地图查看器应用
struct MapViewerApp {
    /// ECS World
    world: World,
    /// 地图查看器场景
    scene: MapViewerScene,
    /// 帧输入事件缓冲
    frame_input_events: Vec<InputEvent>,
}

impl MapViewerApp {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        tracing::info!("🗺️  Map Viewer V3 启动中...");

        // 创建 ECS World
        let mut world = World::new();

        // 创建默认配置
        let settings = ClientSettings::default();

        // 创建模拟网络（使用 mock 模式）
        let _net_ctx = NetworkBuilder::new(settings.network.clone())
            .mock(true) // 启用模拟网络
            .build()
            .expect("Failed to create mock network");

        // 注意：网络功能在此测试工具中暂时禁用
        // world.spawn_settings(settings);
        // world.spawn_network(net_ctx.clone());

        // 创建场景
        let scene = MapViewerScene::new(ctx)?;

        // 发送开始游戏请求，触发地图加载
        // let _ = net_ctx.send(GameEvent::StartGameRequest { character_index: 0 });
        tracing::info!("📤 地图查看器已启动（网络功能已禁用）");

        tracing::info!("✅ Map Viewer V3 启动完成");

        Ok(Self {
            world,
            scene,
            frame_input_events: Vec::new(),
        })
    }

    // 网络功能已禁用
}

impl EventHandler for MapViewerApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // ✅ 创建 GameContext 并传递给场景
        let mut game_ctx = mir2_client::ecs::GameContext::new(ctx, &mut self.world);
        
        // 移动事件到 GameContext
        game_ctx.input_events = std::mem::take(&mut self.frame_input_events);
        
        self.scene.update(&mut game_ctx)?;
        
        // 清理事件
        self.frame_input_events.clear();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(0, 0, 0));

        // 创建 GameContext 用于绘制
        let mut game_ctx = mir2_client::ecs::GameContext::new(ctx, &mut self.world);
        
        // 绘制场景
        self.scene.draw(&mut game_ctx, &mut canvas)?;

        // 提交画布
        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    ) -> GameResult {
        self.frame_input_events
            .push(InputEvent::MouseMove { x, y, dx, dy });
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        // MouseDown 事件已被移除
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        // MouseUp 事件已被移除
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) -> GameResult {
        self.frame_input_events
            .push(InputEvent::MouseWheel { x, y });
        Ok(())
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        _input: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        // KeyDown 事件已被移除 - 使用 ggez Context 的键盘状态
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, _input: KeyInput) -> GameResult {
        // KeyUp 事件已被移除
        Ok(())
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) -> GameResult {
        self.frame_input_events
            .push(InputEvent::Resize { width, height });
        tracing::info!("🖥️ 窗口调整大小: ({:.1}, {:.1})", width, height);
        Ok(())
    }
}

fn main() -> GameResult {
    // 创建 ggez 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer_v3", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title("Map Viewer V3 - Crystal")
                .vsync(true),  // 启用垂直同步，限制帧率到屏幕刷新率
        )
        .window_mode(
            WindowMode::default()
                .dimensions(1600.0, 1200.0)
                .resizable(true)
                .resize_on_scale_factor_change(true),
        )
        .build()?;

    // 创建应用
    let app = MapViewerApp::new(&mut ctx)?;

    // 运行事件循环
    event::run(ctx, event_loop, app)
}
