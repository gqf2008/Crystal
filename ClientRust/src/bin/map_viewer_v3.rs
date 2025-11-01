// ============================================================================
// Map Viewer V3 - 基于新 ECS 架构的地图查看器
// ============================================================================
//
// 功能：
// - 使用新的 SystemScheduler 调度器
// - 使用 GlobalEvents 事件系统
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

use mir2_client::ecs::components::{GlobalEvents, InputEvent};
use mir2_client::ecs::scenes::Scene;
use mir2_client::ecs::WorldExt;
use mir2_client::network::{handlers::GameEvent, NetworkBuilder};
use mir2_client::settings::ClientSettings;

/// 地图查看器应用
struct MapViewerApp {
    /// ECS World
    world: World,
    /// 地图查看器场景
    scene: MapViewerScene,
}

impl MapViewerApp {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        tracing::info!("🗺️  Map Viewer V3 启动中...");

        // 创建 ECS World
        let mut world = World::new();

        // 添加全局事件组件
        world.spawn_global_events(GlobalEvents::new());

        // 创建默认配置
        let settings = ClientSettings::default();

        // 创建模拟网络（使用 mock 模式）
        let net_ctx = NetworkBuilder::new(settings.network.clone())
            .mock(true) // 启用模拟网络
            .build()
            .expect("Failed to create mock network");

        world.spawn_settings(settings);
        world.spawn_network(net_ctx.clone());

        // 创建场景
        let scene = MapViewerScene::new(ctx)?;

        // 发送开始游戏请求，触发地图加载
        let _ = net_ctx.send(GameEvent::StartGameRequest { character_index: 0 });
        tracing::info!("📤 已发送 StartGameRequest，等待地图加载...");

        tracing::info!("✅ Map Viewer V3 启动完成");

        Ok(Self { world, scene })
    }

    #[inline]
    fn collect_network_events(&mut self) {
        let events = self.world.network().recv_categorized();
        self.world.global_events_mut().net_events = events;
    }
}

impl EventHandler for MapViewerApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.collect_network_events();
        // MapViewerApp 只负责收集事件到 GlobalEvents
        // 所有的逻辑处理由 Scene 完成
        self.scene.update(ctx, &mut self.world)?;

        // 清理每帧事件
        self.world.global_events_mut().clear_frame_events();

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(0, 0, 0));

        // 绘制场景
        self.scene.draw(ctx, &mut canvas, &self.world)?;

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
        self.world
            .global_events_mut()
            .input_events
            .push(InputEvent::MouseMove { x, y, dx, dy });
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.world
            .global_events_mut()
            .input_events
            .push(InputEvent::MouseDown { button, x, y });
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.world
            .global_events_mut()
            .input_events
            .push(InputEvent::MouseUp { button, x, y });
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) -> GameResult {
        self.world
            .global_events_mut()
            .input_events
            .push(InputEvent::MouseWheel { x, y });
        Ok(())
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: KeyInput,
        repeated: bool,
    ) -> GameResult {
        if let ggez::winit::keyboard::PhysicalKey::Code(code) = input.event.physical_key {
            self.world
                .global_events_mut()
                .input_events
                .push(InputEvent::KeyDown {
                    keycode: code,
                    repeat: repeated,
                    text: input.event.text,
                    timestamp: std::time::Instant::now(),
                });
        }
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if let ggez::winit::keyboard::PhysicalKey::Code(code) = input.event.physical_key {
            self.world
                .global_events_mut()
                .input_events
                .push(InputEvent::KeyUp {
                    keycode: code,
                    text: input.event.text,
                    timestamp: std::time::Instant::now(),
                });
        }
        Ok(())
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) -> GameResult {
        // 在高 DPI 显示器上，ggez 传递的是物理像素，需要转换为逻辑像素
        // let scale_factor = ctx.gfx.window().scale_factor() as f32;
        // let logical_width = width / scale_factor;
        // let logical_height = height / scale_factor;
        // self.current_scene
        //     .on_resize(ctx, &mut self.world, logical_width, logical_height)

        if let Some((_, events)) = self
            .world
            .query_mut::<&mut GlobalEvents>()
            .into_iter()
            .next()
        {
            events
                .input_events
                .push(InputEvent::Resize { width, height });
            tracing::info!("🖥️ 窗口调整大小: ({:.1}, {:.1})", width, height);
        }
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
