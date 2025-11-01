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
/// 
/// 职责：
/// - 初始化 ECS World 和基础实体
/// - 管理系统调度器
/// - 控制帧率
/// 
/// **不负责**：
/// - ❌ 处理输入事件（由各个系统自己从 GlobalEvents 读取）
/// - ❌ 处理网络事件（由各个系统自己从 Network 读取）
pub struct MapViewerScene {
    /// 时间跟踪实体
    time_entity: Entity,
    /// 渲染配置实体
    config_entity: Entity,
    /// 系统调度器
    system_scheduler: SystemScheduler,
    /// 是否已初始化
    initialized: bool,
}

impl MapViewerScene {
    /// 打印帮助信息
    fn print_help() {
        println!("\n========================================");
        println!("           地图查看器 V3");
        println!("========================================");
        println!("\n📋 操作说明：");
        println!("  • ESC      - 退出程序");
        println!("  • 1/2/3    - 切换 Back/Middle/Front 层");
        println!("  • G        - 切换网格显示");
        println!("  • O        - 切换障碍物显示");
        println!("  • B        - 切换边框显示");
        println!("  • P        - 切换路径显示");
        println!("  • A        - 切换动画播放");
        println!("  • L        - 切换 LOD");
        println!("  • F9       - 切换怪物边框");
        println!("  • F10      - 切换 NPC 边框");
        println!("  • F11      - 切换特效边框");
        println!("  • +/-      - 增加/减少最大 FPS");
        println!("\n🖱️  鼠标操作：");
        println!("  • 中键拖拽  - 移动摄像机");
        println!("  • 滚轮     - 缩放");
        println!("  • 左键     - 选中/移动（未实现）");
        println!("  • 右键     - 操作（未实现）");
        println!("\n========================================\n");
    }
    
    /// 创建新的地图查看器场景
    pub fn new(_ctx: &mut Context) -> GameResult<Self> {
        // 打印帮助信息
        Self::print_help();
        
        // 初始化图形库
        tracing::info!("📚 正在初始化图形库...");
        initialize_all_libraries("Data").expect("初始化图形库失败");
        tracing::info!("✅ 图形库初始化完成");

        // 创建临时 World 来生成实体 ID（稍后会在 initialize_world 中正式创建）
        let mut temp_world = World::new();

        // 创建时间跟踪实体 ID
        let time_entity = temp_world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));

        // 创建渲染配置实体 ID
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

        Ok(Self {
            time_entity,
            config_entity,
            system_scheduler: Self::create_system_scheduler(),
            initialized: false,
        })
    }

    /// 创建系统调度器（只包含必要的系统）
    fn create_system_scheduler() -> SystemScheduler {
        use mir2_client::ecs::render::{MapRenderSystem, DebugSystem};
        use mir2_client::ecs::systems::logic::CameraFollowSystem;
        
        let mut scheduler = SystemScheduler::new();

        tracing::info!("🎯 初始化地图查看器系统...");

        // 添加逻辑系统
        scheduler
            .add_system(MovementSystem)              // 移动系统
            .add_system(AnimationSystem::new())      // 动画系统
            .add_system(CameraSystem::new())         // 相机系统（拖拽、缩放）→ 从 GlobalEvents 读取鼠标事件
            .add_system(CameraFollowSystem)          // 相机跟随
            .add_system(MapRenderSystem)             // 地图渲染系统
            .add_system(DebugSystem);                // 调试系统（键盘快捷键、FPS显示）→ 从 GlobalEvents 读取

        tracing::info!("✅ 地图查看器系统初始化完成！");
        tracing::info!("📋 所有系统都从 GlobalEvents 读取输入事件");
        tracing::info!("📋 网络事件由 MockNetwork 直接发送到 NetworkContext");
        scheduler
    }

    /// 在实际的 World 中创建所有实体
    fn initialize_world(&mut self, world: &mut World, screen_width: f32, screen_height: f32) {
        if self.initialized {
            return;
        }

        // 在实际的 World 中重新创建所有实体
        // 相机实体（由 CameraSystem 使用）
        world.spawn((
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

        // 时间跟踪实体
        self.time_entity = world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));

        // 渲染配置实体（由键盘输入系统修改）
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

        // 可见区域缓存实体
        world.spawn((VisibleArea::default(),));

        // 鼠标输入状态实体（由鼠标输入系统修改）
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

        // // 运行所有系统（update 阶段）
        // let delta_ms = 8.0; // 约 60fps
        self.system_scheduler.update(world, ctx.time.delta().as_secs_f32())?;

        Ok(None)
    }

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 运行所有系统（draw 阶段） - 传递 canvas 参数
        // 所有渲染（包括 FPS 显示）都由系统负责
        self.system_scheduler.draw(ctx, canvas, world)?;

        Ok(())
    }
}
