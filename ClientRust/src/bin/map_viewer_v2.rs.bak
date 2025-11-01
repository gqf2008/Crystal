// ============================================================================
// Map Viewer V2 - 简化版地图查看器（开发工具）
// ============================================================================
//
// ⚠️ 状态: 暂时禁用，等待重构
// 
// 原因: 依赖了已删除的组件和系统：
// - Player, PlayerAction, PlayerAppearance 等旧组件
// - RenderSystem, MapViewerScheduler 等旧系统
// - NetworkSync, PlayerInput 等网络组件
//
// 解决方案: 
// 1. 使用 #[cfg(feature = "dev-tools")] 条件编译
// 2. 或者重写使用新的 ECS 架构
// 3. 或者直接使用 mir2x 主程序的地图查看功能
//
// ============================================================================

// 暂时提供一个空的 main 函数，避免编译错误
fn main() {
    eprintln!("⚠️ map_viewer_v2 暂时禁用");
    eprintln!("请使用主程序: cargo run --bin mir2x");
    std::process::exit(1);
}

/*
// ============================================================================
// 以下代码已注释，等待重构
// ============================================================================

use ggez::winit::event::MouseButton;
use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{
        Canvas, Color, DrawParam,
        Text, TextFragment, FontData,
    },
    Context, ContextBuilder, GameResult,
};
use hecs::{Entity, World};
use mir2_client::graphics::libraries::initialize_all_libraries;
use mir2_client::objects::MapReader;
use std::time::Instant;
use std::path::Path as FilePath;

use mir2_client::ecs::{
    // ✅ 新架构：从 components 模块导入
    components::{
        Position, Camera, Draggable, RenderConfig, TimeTracker, VisibleArea,
        MapData, MapTile, TileLayer,
    },
    // Utilities
    Coordinates, MapUtils, MapLoader,
};

// ============================================================================
// 主应用程序
// ============================================================================

struct MapViewerV2 {
    world: World,
    camera_entity: Entity,
    time_entity: Entity,
    config_entity: Entity,
    visible_area_entity: Entity,
    ui_font_name: String,
    scheduler: MapViewerScheduler,  // 🆕 串行系统调度器
}

impl MapViewerV2 {
    fn new(ctx: &mut Context, map_path: &str) -> GameResult<Self> {
        // 初始化库
        println!("📚 正在初始化地图库...");
        initialize_all_libraries("Data").expect("初始化地图库失败");
        println!("✅ 地图库初始化完成");

        // 加载地图
        println!("🗺️  正在加载地图: {}", map_path);
        let reader = MapReader::new(map_path)?;
        println!("✅ 地图加载完成: {}x{}", reader.width, reader.height);

        // 创建 ECS 世界
        let mut world = World::new();

        // 加载地图瓦片到 ECS
        MapLoader::load_map(&mut world, reader)?;

        // 找到地图中心的无障碍位置作为出生点
        let map_data = world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone())
            .expect("地图数据未加载");
        
        let (spawn_grid_x, spawn_grid_y) = MapUtils::find_center_walkable_position(&map_data);
        let (spawn_x, spawn_y) = Coordinates::grid_to_world_center(spawn_grid_x, spawn_grid_y);
        
        println!("🎯 出生位置: 格子({}, {}) -> 世界坐标({:.1}, {:.1})", 
                 spawn_grid_x, spawn_grid_y, spawn_x, spawn_y);

        // 创建相机实体
        let screen = ctx.gfx.drawable_size();
        let camera_entity = world.spawn((
            Position { x: spawn_x, y: spawn_y },
            Camera {
                zoom: 1.0,
                screen_width: screen.0,
                screen_height: screen.1,
            },
            Draggable {
                is_dragging: false,
                drag_start_x: 0.0,
                drag_start_y: 0.0,
                drag_start_pos_x: 0.0,
                drag_start_pos_y: 0.0,
            },
        ));

        // 创建时间跟踪实体
        let time_entity = world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));

        // 创建渲染配置实体
        let config_entity = world.spawn((RenderConfig {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: false,
            show_obstacles: false,
            show_animations: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            max_fps: 160,
            enable_lod: true,
        },));

        // 创建可见区域缓存实体
        let visible_area_entity = world.spawn((VisibleArea::default(),));

        // 创建玩家角色实体
        let _player_entity = world.spawn((
            Player {
                direction: 4,
                action: PlayerAction::Stand,
                frame_index: 0,
                frame_time: 0,
                speed: 0.0,
                target_x: spawn_x,
                target_y: spawn_y,
                is_moving: false,
                path: Vec::new(),
                path_index: 0,
                move_mode: MoveMode::Idle,
                last_move_time: Instant::now(),
                move_delay: std::time::Duration::from_millis(600),
                waiting_server_confirm: false,
                collision_detected: false,
                collision_target_grid: None,
                can_run: true,
                last_run_time: Instant::now(),
                run_cooldown: std::time::Duration::from_millis(900),
            },
            Position { x: spawn_x, y: spawn_y },
            PlayerAppearance {
                class: mir2_shared::enums::MirClass::Warrior,
                gender: mir2_shared::enums::MirGender::Male,
                hair: 0,
                weapon: -1,
                armour: 0,
                weapon_effect: 0,
                wing_effect: 0,
            },
            MovementAnimation::new(
                (spawn_x / 48.0) as i32,
                (spawn_y / 32.0) as i32,
            ),
            LocalPlayer,
            NetworkSync {
                object_id: 1,
                last_update: Instant::now(),
                object_type: NetworkObjectType::Player,
            },
            PlayerInput::new(),
            MovementVelocity::with_speeds(DEFAULT_MAX_SPEED, DEFAULT_WALK_SPEED, DEFAULT_RUN_SPEED),
            Path::new(),
            Movement::new(),
            Prediction::new(Position { x: spawn_x, y: spawn_y }),
        ));

        // 创建鼠标输入状态实体
        let _mouse_input_entity = world.spawn((MouseInput {
            left_pressed: false,
            right_pressed: false,
            left_double_clicked: false,
            right_double_clicked: false,
            left_press_time: 0,
            right_press_time: 0,
            left_last_click_time: Instant::now() - std::time::Duration::from_secs(10),
            right_last_click_time: Instant::now() - std::time::Duration::from_secs(10),
            x: 0.0,
            y: 0.0,
        },));

        // 创建地图管理器组件
        world.spawn((MapManager::new(map_path.to_string()),));
        println!("🗺️  MapManager 已创建");

        // 加载中文字体
        let ui_font_name = Self::load_chinese_font(ctx)?;

        // 🆕 创建串行调度器
        let scheduler = MapViewerScheduler::new();
        println!("⚙️  MapViewer调度器已启动（串行执行5个系统）");

        Ok(Self {
            world,
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            ui_font_name,
            scheduler,
        })
    }

    /// 加载中文字体
    fn load_chinese_font(ctx: &mut Context) -> GameResult<String> {
        let font_configs = [
            ("C:/Windows/Fonts/msyh.ttc", "Microsoft YaHei"),
            ("C:/Windows/Fonts/simsun.ttc", "SimSun"),
            ("C:/Windows/Fonts/simhei.ttf", "SimHei"),
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", "WenQuanYi"),
            ("/System/Library/Fonts/PingFang.ttc", "PingFang"),
        ];

        for (path, font_name) in &font_configs {
            if FilePath::new(path).exists() {
                match std::fs::read(path) {
                    Ok(bytes) => {
                        match FontData::from_vec(bytes) {
                            Ok(font_data) => {
                                ctx.gfx.add_font(*font_name, font_data);
                                println!("✅ 字体加载成功: {}", font_name);
                                return Ok(font_name.to_string());
                            }
                            Err(e) => {
                                eprintln!("⚠️  字体解析失败 {}: {}", font_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️  字体读取失败 {}: {}", path, e);
                    }
                }
            }
        }

        println!("⚠️  未找到中文字体，使用默认字体");
        Ok("LiberationMono-Regular".to_string())
    }
}

impl EventHandler for MapViewerV2 {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // FPS 限制
        {
            let mut time = self.world.get::<&mut TimeTracker>(self.time_entity).unwrap();
            let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap();
            
            let target_frame_time = std::time::Duration::from_micros(1_000_000 / config.max_fps as u64);
            let elapsed = time.last_frame_time.elapsed();
            
            if elapsed < target_frame_time {
                std::thread::sleep(target_frame_time - elapsed);
            }
            time.last_frame_time = Instant::now();
        }

        // 动画计数和 FPS 统计
        {
            let mut time = self.world.get::<&mut TimeTracker>(self.time_entity).unwrap();
            time.animation_count = (time.animation_count + 1) % 10000;
            time.frame_count += 1;
            
            if time.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                time.fps = time.frame_count as f32 / time.last_fps_update.elapsed().as_secs_f32();
                time.frame_count = 0;
                time.last_fps_update = Instant::now();
            }
        }

        // 🚀 调用串行调度器（按优先级顺序执行所有系统）
        let delta_time = ctx.time.delta().as_secs_f32();
        self.scheduler.update(ctx, &mut self.world, delta_time)?;

        // 🗺️ 检查地图切换请求
        MapUpdateSystem::update(&mut self.world, ctx)?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);

        // 获取相机和配置
        let (pos, camera) = {
            let pos = self.world.get::<&Position>(self.camera_entity).unwrap().clone();
            let camera = self.world.get::<&Camera>(self.camera_entity).unwrap().clone();
            (pos, camera)
        };
        let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap().clone();

        // 渲染瓦片
        RenderSystem::draw_tiles(ctx, &mut canvas, &self.world, &pos, &camera, &config, self.visible_area_entity)?;

        // 渲染网格（调试）
        if config.show_grid {
            RenderSystem::draw_grid(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 渲染障碍物（调试）
        if config.show_obstacles {
            RenderSystem::draw_obstacles(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 渲染角色
        for (_entity, (player, player_pos)) in self.world.query::<(&Player, &Position)>().iter() {
            RenderSystem::draw_player_with_world(ctx, &mut canvas, &self.world, player, player_pos, &pos, &camera)?;
        }

        // 渲染路径（调试）
        if config.show_path {
            RenderSystem::draw_path(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 渲染碰撞调试
        RenderSystem::draw_collision_debug(ctx, &mut canvas, &self.world, &pos, &camera)?;

        // 渲染 UI
        self.draw_ui(ctx, &mut canvas)?;

        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) -> GameResult {
        if button == MouseButton::Middle {
            let mut draggable = self.world.get::<&mut Draggable>(self.camera_entity).unwrap();
            let pos = self.world.get::<&Position>(self.camera_entity).unwrap().clone();
            CameraSystem::start_drag(&mut draggable, &pos, x, y);
        }
        Ok(())
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> GameResult {
        if button == MouseButton::Middle {
            let mut draggable = self.world.get::<&mut Draggable>(self.camera_entity).unwrap();
            CameraSystem::end_drag(&mut draggable);
        }
        Ok(())
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult {
        let draggable = self.world.get::<&Draggable>(self.camera_entity).unwrap().clone();
        if draggable.is_dragging {
            let mut pos = self.world.get::<&mut Position>(self.camera_entity).unwrap();
            let camera = self.world.get::<&Camera>(self.camera_entity).unwrap().clone();
            CameraSystem::update_drag(&draggable, &mut pos, &camera, x, y);
        }
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) -> GameResult {
        let mouse_pos = _ctx.mouse.position();
        let mut pos = self.world.get::<&mut Position>(self.camera_entity).unwrap();
        let mut camera = self.world.get::<&mut Camera>(self.camera_entity).unwrap();
        CameraSystem::zoom(&mut pos, &mut camera, y, mouse_pos.x, mouse_pos.y);
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: ggez::input::keyboard::KeyInput, _repeat: bool) -> GameResult {
        use ggez::input::keyboard::KeyCode;
        use ggez::winit::keyboard::PhysicalKey;

        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            match keycode {
                KeyCode::Escape => ctx.request_quit(),
                KeyCode::KeyM => MapUpdateSystem::trigger_map_selection(&mut self.world),
                KeyCode::KeyG => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_grid = !config.show_grid;
                }
                KeyCode::KeyO => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_obstacles = !config.show_obstacles;
                }
                KeyCode::KeyB => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_borders = !config.show_borders;
                }
                KeyCode::KeyP => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_path = !config.show_path;
                }
                KeyCode::KeyA => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_animations = !config.show_animations;
                }
                KeyCode::KeyL => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.enable_lod = !config.enable_lod;
                }
                KeyCode::Digit1 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_back = !config.show_back;
                }
                KeyCode::Digit2 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_middle = !config.show_middle;
                }
                KeyCode::Digit3 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_front = !config.show_front;
                }
                KeyCode::F9 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_monster_borders = !config.show_monster_borders;
                }
                KeyCode::F10 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_npc_borders = !config.show_npc_borders;
                }
                KeyCode::F11 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_effect_borders = !config.show_effect_borders;
                }
                KeyCode::Equal | KeyCode::NumpadAdd => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.max_fps = (config.max_fps + 10).min(300);
                }
                KeyCode::Minus | KeyCode::NumpadSubtract => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                }
                KeyCode::F5 => {
                    // 打印调度器性能报告
                    self.scheduler.print_performance_report();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> GameResult {
        let (actual_width, actual_height) = ctx.gfx.drawable_size();
        println!("🖼️  窗口大小调整: 窗口={}x{}, 可绘制={}x{}", 
                 width, height, actual_width, actual_height);
        
        let mut camera = self.world.get::<&mut Camera>(self.camera_entity).unwrap();
        camera.screen_width = actual_width;
        camera.screen_height = actual_height;
        Ok(())
    }
}

impl MapViewerV2 {
    fn draw_ui(&self, _ctx: &Context, canvas: &mut Canvas) -> GameResult {
        let time = self.world.get::<&TimeTracker>(self.time_entity).unwrap();
        let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap();
        let pos = self.world.get::<&Position>(self.camera_entity).unwrap();
        let camera = self.world.get::<&Camera>(self.camera_entity).unwrap();
        
        let visible_count = if let Ok(visible_area) = self.world.get::<&VisibleArea>(self.visible_area_entity) {
            visible_area.visible_entities.len()
        } else {
            0
        };
        
        let frame_time = if time.fps > 0.0 { 1000.0 / time.fps } else { 0.0 };
        
        let ui_text = format!(
            "📊 性能: {:.1} FPS ({:.2}ms/帧) | 最大: {} FPS | LOD: {} | 调度器: 串行 ⚙️\n\
             🎨 渲染: {} 瓦片\n\
             📍 位置: ({:.0}, {:.0}) | 缩放: {:.2}x\n\
             🗺️  图层: Back={} Middle={} Front={}\n\
            \n\
             🎮 角色: [长按左键]走路 [长按右键]跑步 (自动避障寻路)\n\
             🖱️  地图: [中键拖拽]移动 [滚轮]缩放\n\
             🔧 调试: [G]网格 [O]障碍 [B]边框 [P]路径\n\
             👁️  显示: [1/2/3]图层 [A]动画 [L]LOD\n\
             🎯 边框: [F9]怪物 [F10]NPC [F11]特效\n\
             📈 调度器: [F5]性能报告\n\
             ⚙️  其他: [M]选择地图 [+/-]调整帧率 [ESC]退出",
            time.fps, frame_time, config.max_fps,
            if config.enable_lod { "开" } else { "关" },
            visible_count,
            pos.x, pos.y, camera.zoom,
            if config.show_back { "✓" } else { "✗" },
            if config.show_middle { "✓" } else { "✗" },
            if config.show_front { "✓" } else { "✗" },
        );

        let text = Text::new(
            TextFragment::new(ui_text)
                .font(&self.ui_font_name)
                .scale(24.0)
                .color(Color::from_rgb(255, 255, 0)),
        );

        canvas.draw(&text, DrawParam::default().dest([10.0, 10.0]).color(Color::WHITE));
        Ok(())
    }
}

// ============================================================================
// 主函数
// ============================================================================

fn main() -> GameResult {
    // 默认地图
    let default_map = "Map/0.map";

    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer_v2", "Crystal Team")
        .window_setup(WindowSetup::default().title("Map Viewer V2 - 串行调度器"))
        .window_mode(WindowMode::default().dimensions(1400.0, 900.0).resizable(true))
        .build()?;

    let state = MapViewerV2::new(&mut ctx, default_map)?;
    event::run(ctx, event_loop, state)
}
