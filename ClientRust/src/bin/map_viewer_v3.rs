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
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::winit::event::MouseButton;
use ggez::{Context, ContextBuilder, GameResult};
use hecs::World;

use map_viewer::scene::MapViewerScene;

use mir2_client::ecs::components::{GlobalEvents, InputEvent, MouseInput};
use mir2_client::ecs::scenes::Scene;
use mir2_client::ecs::WorldExt;
use mir2_client::network::{NetworkBuilder, handlers::GameEvent};
use mir2_client::objects::MapReader;
use mir2_client::settings::ClientSettings;

/// 地图查看器应用
struct MapViewerApp {
    /// ECS World
    world: World,
    /// 地图查看器场景
    scene: MapViewerScene,
    /// 鼠标位置
    mouse_pos: (f32, f32),
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
            .mock(true)  // 启用模拟网络
            .build()
            .expect("Failed to create mock network");

        world.spawn_settings(settings);
        world.spawn_network(net_ctx.clone());

        // 创建场景
        let scene = MapViewerScene::new(ctx)?;

        // 发送开始游戏请求，触发地图加载
        let _ = net_ctx.send(GameEvent::StartGameRequest {
            character_index: 0,
        });
        tracing::info!("📤 已发送 StartGameRequest，等待地图加载...");

        tracing::info!("✅ Map Viewer V3 启动完成");

        Ok(Self {
            world,
            scene,
            mouse_pos: (0.0, 0.0),
        })
    }

    /// 处理网络事件
    fn handle_network_events(&mut self) {
        // 收集网络事件
        let game_events = {
            let net_ctx = self.world.network();
            net_ctx.recv_all()
        };

        // 处理事件
        for event in game_events {
            match event {
                GameEvent::MapChanged { file_name, .. } => {
                    tracing::info!("🗺️  收到地图变更事件: {}", file_name);
                    
                    // 加载地图
                    match MapReader::new(&file_name) {
                        Ok(map_reader) => {
                            tracing::info!(
                                "✅ 成功加载地图: {} ({}x{})",
                                file_name,
                                map_reader.width,
                                map_reader.height
                            );
                            
                            // 使用 MapLoader 加载地图瓦片到 ECS
                            use mir2_client::ecs::MapLoader;
                            if let Err(e) = MapLoader::load_map(&mut self.world, map_reader) {
                                tracing::error!("❌ 加载地图瓦片失败: {:?}", e);
                            } else {
                                tracing::info!("✅ 地图瓦片已加载到 ECS");
                            }
                        }
                        Err(e) => {
                            tracing::error!("❌ 加载地图失败 {}: {:?}", file_name, e);
                        }
                    }
                }
                GameEvent::Connected => {
                    tracing::info!("✅ 网络已连接");
                }
                GameEvent::StartGame { .. } => {
                    tracing::info!("🎮 游戏开始");
                }
                _ => {
                    tracing::debug!("📥 收到网络事件: {:?}", event);
                }
            }
        }
    }

    /// 处理输入事件
    fn handle_input_events(&mut self, ctx: &mut Context) {
        // 先收集需要处理的事件
        let input_events = {
            let global_events = self.world.global_events();
            global_events.input_events.clone()
        };

        // 然后处理事件
        for event in input_events {
            match event {
                InputEvent::KeyDown { keycode, .. } => {
                    if keycode == KeyCode::Escape {
                        tracing::info!("👋 用户按下 ESC，退出程序");
                        ctx.request_quit();
                    }
                }
                InputEvent::MouseMove { x, y, .. } => {
                    self.mouse_pos = (x, y);

                    // 更新 MouseInput 组件
                    if let Some((_, mouse_input)) =
                        self.world.query_mut::<&mut MouseInput>().into_iter().next()
                    {
                        mouse_input.x = x;
                        mouse_input.y = y;
                    }
                }
                InputEvent::MouseDown { button, x, y } => {
                    if button == MouseButton::Left {
                        if let Some((_, mouse_input)) =
                            self.world.query_mut::<&mut MouseInput>().into_iter().next()
                        {
                            mouse_input.left_pressed = true;
                            mouse_input.left_press_time = 0;
                            mouse_input.x = x;
                            mouse_input.y = y;
                        }
                    } else if button == MouseButton::Right {
                        if let Some((_, mouse_input)) =
                            self.world.query_mut::<&mut MouseInput>().into_iter().next()
                        {
                            mouse_input.right_pressed = true;
                            mouse_input.right_press_time = 0;
                            mouse_input.x = x;
                            mouse_input.y = y;
                        }
                    }
                }
                InputEvent::MouseUp { button, .. } => {
                    if button == MouseButton::Left {
                        if let Some((_, mouse_input)) =
                            self.world.query_mut::<&mut MouseInput>().into_iter().next()
                        {
                            mouse_input.left_pressed = false;
                        }
                    } else if button == MouseButton::Right {
                        if let Some((_, mouse_input)) =
                            self.world.query_mut::<&mut MouseInput>().into_iter().next()
                        {
                            mouse_input.right_pressed = false;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl EventHandler for MapViewerApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 处理网络事件
        self.handle_network_events();

        // 处理输入事件
        self.handle_input_events(ctx);

        // 更新场景
        self.scene.update(ctx, &mut self.world)?;

        // 清理事件（在每帧结束时）
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
}

fn main() -> GameResult {
    // 创建 ggez 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer_v3", "Crystal")
        .window_setup(WindowSetup::default().title("Map Viewer V3 - Crystal").vsync(false))
        .window_mode(WindowMode::default().dimensions(1600.0, 1200.0))
        .build()?;

    // 创建应用
    let app = MapViewerApp::new(&mut ctx)?;

    // 运行事件循环
    event::run(ctx, event_loop, app)
}
