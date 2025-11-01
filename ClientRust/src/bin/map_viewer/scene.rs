// ============================================================================
// Map Viewer Scene - 简化的地图查看器场景
// ============================================================================
//
// 基于 GameScene 简化而来，专门用于地图查看和调试
// 
// 特性：
// - 使用新的 ECS 系统架构
// - 使用 SystemScheduler 调度器
// - 集成 MockNetServer 模拟网络
// - 只包含必要的系统（渲染、相机、输入）
//
// ============================================================================

use ggez::graphics::Canvas;
use ggez::{Context, GameResult};
use hecs::{Entity, World};
use std::time::Instant;

use mir2_client::ecs::components::{
    Camera, Draggable, MouseInput, Position, RenderConfig, TimeTracker, VisibleArea,
};
use mir2_client::ecs::scenes::{Scene, SceneType};
use mir2_client::ecs::systems::{
    AnimationSystem, CameraSystem, MovementSystem, SystemScheduler,
};
use mir2_client::graphics::libraries::initialize_all_libraries;

/// 地图查看器场景
pub struct MapViewerScene {
    /// 相机实体
    camera_entity: Entity,
    /// 时间跟踪实体
    time_entity: Entity,
    /// 渲染配置实体
    config_entity: Entity,
    /// 可见区域缓存实体
    visible_area_entity: Entity,
    /// 系统调度器
    system_scheduler: SystemScheduler,
    /// 是否已初始化
    initialized: bool,
}

impl MapViewerScene {
    /// 创建新的地图查看器场景
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        // 初始化图形库
        tracing::info!("📚 正在初始化图形库...");
        initialize_all_libraries("Data").expect("初始化图形库失败");
        tracing::info!("✅ 图形库初始化完成");

        let (screen_width, screen_height) = ctx.gfx.drawable_size();

        // 创建临时 World 来生成实体（稍后会在 spawn 中正式创建）
        let mut temp_world = World::new();

        // 创建相机实体
        let camera_entity = temp_world.spawn((
            Position { x: 50.0, y: 50.0 }, // 默认位置在地图中心
            Camera {
                zoom: 1.0,
                screen_width,
                screen_height,
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
        let time_entity = temp_world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));

        // 创建渲染配置实体
        let config_entity = temp_world.spawn((RenderConfig {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: true,
            show_obstacles: true,
            show_animations: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            max_fps: 60,
            enable_lod: true,
        },));

        // 创建可见区域缓存实体
        let visible_area_entity = temp_world.spawn((VisibleArea::default(),));

        // 创建鼠标输入状态实体
        temp_world.spawn((MouseInput {
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

        Ok(Self {
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            system_scheduler: Self::create_system_scheduler(),
            initialized: false,
        })
    }

    /// 创建系统调度器（只包含必要的系统）
    fn create_system_scheduler() -> SystemScheduler {
        use mir2_client::ecs::render::MapRenderSystem;
        use mir2_client::ecs::systems::logic::{
            CameraFollowSystem, DebugSystem, PlayerControlSystem,
        };
        
        let mut scheduler = SystemScheduler::new();

        tracing::info!("🎯 初始化地图查看器系统...");

        // 添加逻辑系统
        scheduler
            .add_system(PlayerControlSystem::new())  // 玩家控制（输入处理）
            .add_system(MovementSystem)              // 移动系统
            .add_system(AnimationSystem::new())      // 动画系统
            .add_system(CameraSystem::new())         // 相机系统（拖拽、缩放）
            .add_system(CameraFollowSystem::new())   // 相机跟随
            .add_system(MapRenderSystem)             // 地图渲染系统
            .add_system(DebugSystem::new());         // 调试系统（FPS、坐标）

        tracing::info!("✅ 地图查看器系统初始化完成！");
        scheduler
    }

    /// 在实际的 World 中创建所有实体
    fn initialize_world(&mut self, world: &mut World, screen_width: f32, screen_height: f32) {
        if self.initialized {
            return;
        }

        // 在实际的 World 中重新创建所有实体
        self.camera_entity = world.spawn((
            Position { x: 50.0, y: 50.0 },
            Camera {
                zoom: 1.0,
                screen_width,
                screen_height,
            },
            Draggable {
                is_dragging: false,
                drag_start_x: 0.0,
                drag_start_y: 0.0,
                drag_start_pos_x: 0.0,
                drag_start_pos_y: 0.0,
            },
        ));

        self.time_entity = world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));

        self.config_entity = world.spawn((RenderConfig {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: true,
            show_obstacles: true,
            show_animations: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            max_fps: 60,
            enable_lod: true,
        },));

        self.visible_area_entity = world.spawn((VisibleArea::default(),));

        world.spawn((MouseInput {
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

        self.initialized = true;
        tracing::info!("✅ MapViewerScene World 初始化完成");
    }
}

impl Scene for MapViewerScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut Context, world: &mut World) -> GameResult<Option<SceneType>> {
        // 首次更新时初始化 World
        if !self.initialized {
            let (screen_width, screen_height) = ctx.gfx.drawable_size();
            self.initialize_world(world, screen_width, screen_height);
        }

        // 帧率限制
        let config = world.get::<&RenderConfig>(self.config_entity).unwrap();
        let max_fps = config.max_fps;
        drop(config);

        if let Ok(mut time) = world.get::<&mut TimeTracker>(self.time_entity) {
            let target_frame_time = std::time::Duration::from_secs_f32(1.0 / max_fps as f32);
            let elapsed = time.last_frame_time.elapsed();

            if elapsed < target_frame_time {
                return Ok(None);
            }

            time.last_frame_time = Instant::now();
            time.animation_count += 1;
            time.frame_count += 1;

            if time.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                time.fps = time.frame_count as f32 / time.last_fps_update.elapsed().as_secs_f32();
                time.frame_count = 0;
                time.last_fps_update = Instant::now();
            }
        }

        // 运行所有系统（update 阶段）
        let delta_ms = 8.0; // 约 60fps
        self.system_scheduler.update(world, delta_ms)?;

        Ok(None)
    }

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 运行所有系统（draw 阶段）
        self.system_scheduler.draw(ctx, world)?;

        // 显示 FPS
        if let Ok(time) = world.get::<&TimeTracker>(self.time_entity) {
            let fps_text = format!("FPS: {:.1}", time.fps);
            let text = ggez::graphics::Text::new(fps_text);
            canvas.draw(
                &text,
                ggez::graphics::DrawParam::default().dest([10.0, 10.0]),
            );
        }

        Ok(())
    }
}
