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
use ggez::GameResult;
use hecs::{Entity, World};
use mir2_client::ecs::components::movement::{MovementVelocity, Path};
use mir2_client::ecs::components::{
    Camera, CameraMode, Draggable, PlayerInput, Position, RenderConfig, TimeTracker,
    VisibleArea,
};
use mir2_client::ecs::components::{LocalPlayer, Player, PlayerAction, PlayerAppearance};
use mir2_client::ecs::render::{CharacterRenderSystem, DebugSystem, MapRenderSystem};
use mir2_client::ecs::scenes::{Scene, SceneType};
use mir2_client::ecs::systems::logic::map_load_system::MapManager;
use mir2_client::ecs::systems::logic::{
    CameraFollowSystem, CollisionSystem, MapLoadSystem, MapUpdateSystem, TileAnimationSystem,
    PathfindingSystem,
};
use mir2_client::ecs::systems::{MovementSystem, SystemScheduler};
use mir2_client::ecs::{CameraSystem, GameContext, PlayerControlSystem};
use mir2_client::graphics::libraries::initialize_all_libraries;
use mir2_shared::enums::{MirClass, MirGender};
use std::time::Instant;
/// 地图查看器场景
///
/// 职责：
/// - 初始化 ECS World 和基础实体
/// - 管理双系统调度器 (V1 + V2)
/// - 控制帧率
///
/// **架构**:
/// - V1 系统: 使用 GameContext (新架构)
/// - V2 系统: 使用 GameContext 零拷贝 (新架构)
pub struct MapViewerScene {
    /// 时间跟踪实体
    time_entity: Entity,
    /// 渲染配置实体
    config_entity: Entity,
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
        println!("  • M        - 打开地图选择对话框");
        println!("  • 1/2/3    - 切换 Front/Middle/Back 层 (前景/中间/背景)");
        println!("  • G        - 切换网格显示");
        println!("  • O        - 切换障碍物显示");
        println!("  • B        - 切换边框显示");
        println!("  • P        - 切换路径显示");
        println!("  • A        - 切换动画播放");
        println!("  • S        - 切换静态瓦片显示");
        println!("  • D        - 切换动画瓦片显示");
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
    pub fn new(_ctx: &mut GameContext) -> GameResult<Self> {
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
            show_grid: false,
            show_obstacles: false,
            show_animations: true,
            show_static_tiles: true,
            show_animated_tiles: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_player_debug: false, // F2键切换
            show_path: false,
            max_fps: 60,
            enable_lod: true,
            enable_camera_drag: true, // 地图查看器启用鼠标拖拽功能
        },));

        Ok(Self {
            time_entity,
            config_entity,
            system_scheduler: Self::create_system_scheduler(),
            initialized: false,
        })
    }

    /// 创建系统调度器（只包含必要的系统 - V1旧版）
    fn create_system_scheduler() -> SystemScheduler {
        let mut scheduler = SystemScheduler::new();

        tracing::info!("🎯 初始化地图查看器系统 (V1)...");

        // 添加逻辑系统 (V1)
        // 清晰的单向数据流: Input → State → Collision → Movement → Animation
        scheduler
            // 1. PlayerControlSystem: 输入处理
            //    - 检测鼠标输入,设置 PlayerInput
            //    - 设置 mouse_pressed 标志
            .add_system(PlayerControlSystem::new())
            
            // 2. PathfindingSystem: 寻路系统
            //    - 处理双击寻路，计算路径
            //    - 根据 MovementMode 决定是否使用寻路
            .add_system(PathfindingSystem::new())
            
            // 3. CollisionSystem: 碰撞检测
            //    - 检查地图障碍物
            //    - velocity=0 停止位移，但不影响动画
            .add_system(CollisionSystem::new())
            
            // 5. MovementSystem: 位置更新
            //    - 根据velocity更新position
            //    - 只管位移，不管动画状态
            .add_system(MovementSystem)
            
            // 注意: CharacterAnimationSystem 已移除 (未使用)
            // 渲染系统直接使用 PlayerAction.frame_interval()
            
            .add_system(TileAnimationSystem::new()) // 瓦片动画系统
            .add_system(MapUpdateSystem::new()) // 地图更新系统 (M键切换地图)
            .add_system(MapLoadSystem) // 地图加载系统 → 从 GameContext 读取 MapChanged 事件
            .add_system(CameraSystem::new())
            .add_system(CameraFollowSystem) // 相机跟随
            .add_system(MapRenderSystem) // 地图渲染系统
            .add_system(CharacterRenderSystem)
            .add_system(DebugSystem); // 角色渲染系统

        tracing::info!("✅ 地图查看器 V1 系统初始化完成！");
        scheduler
    }

    /// 在实际的 World 中创建所有实体
    fn initialize_world(&mut self, world: &mut World, screen_width: f32, screen_height: f32) {
        if self.initialized {
            return;
        }

        // 🎯 玩家的世界坐标（盟重土城传送点）
        let player_world_x = (332 * 48) as f32;
        let player_world_y = (327 * 32) as f32;

        // 在实际的 World 中重新创建所有实体
        // 相机实体（由 CameraSystem 使用）
        // ✅ 相机初始位置应该对准玩家，这样缩放时才不会偏移
        world.spawn((
            Position {
                x: player_world_x,  // 相机位置 = 玩家位置
                y: player_world_y,
            },
            Camera {
                zoom: 1.0,
                screen_width,
                screen_height,
            },
            CameraMode::FollowPlayer, // 🎯 改为跟随玩家模式
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

        // 渲染配置实体(由键盘输入系统修改)
        self.config_entity = world.spawn((RenderConfig {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: false,
            show_obstacles: false,
            show_animations: true,
            show_static_tiles: true,
            show_animated_tiles: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_player_debug: false, // F2键切换
            show_path: false,
            max_fps: 60,
            enable_lod: true,
            enable_camera_drag: true, // 地图查看器启用鼠标拖拽功能
        },));

        // 可见区域缓存实体
        world.spawn((VisibleArea::default(),));

        // 事件通过 GameContext 传递

        // // 鼠标输入状态实体（由鼠标输入系统修改）
        // world.spawn((MouseInput {
        //     left_pressed: false,
        //     right_pressed: false,
        //     left_double_clicked: false,
        //     right_double_clicked: false,
        //     left_press_time: 0,
        //     right_press_time: 0,
        //     left_last_click_time: Instant::now() - std::time::Duration::from_secs(10),
        //     right_last_click_time: Instant::now() - std::time::Duration::from_secs(10),
        //     x: 0.0,
        //     y: 0.0,
        // },));

        // 地图管理器组件（用于地图状态跟踪）

        world.spawn((MapManager {
            current_map_index: 0,
            current_map_file: "0".to_string(),
            current_map_title: "比奇城".to_string(),
            is_loading: false,
        },));

        // 🆕 创建测试玩家

        let player_entity = world.spawn((
            // 位置：盟重土城传送点（已知的安全位置）
            Position {
                x: (332 * 48) as f32, // 传送点 X - 盟重土城中心传送点位置
                y: (327 * 32) as f32, // 传送点 Y
            },
            // 🆕 移动速度组件（MovementSystem 需要）
            // 🎯 调整速度以匹配动画播放速度
            // Walk: 3帧间隔(0.3s/循环) → 设置为60px/s
            // Run: 2帧间隔(0.2s/循环) → 设置为120px/s (加快)
            MovementVelocity::with_speeds(120.0, 60.0, 120.0),
            // 🆕 路径组件（MovementSystem 需要）
            Path::new(),
            // 玩家核心状态
            Player {
                direction: 0,
                action: PlayerAction::Stand,
            },
            // 玩家外观
            PlayerAppearance {
                class: MirClass::Warrior,
                gender: MirGender::Male,
                hair: 1,  // 使用索引1的头发 (0可能无效)
                weapon: 1, // 武器索引1 (测试武器渲染)
                armour: 1, // 测试armour=1 (0可能是裸体/透明)
                weapon_effect: 0, // 0 = 无特效
                wing_effect: 0,
            },
            // 🆕 玩家输入组件（由 PlayerControlSystem 写入）
            PlayerInput::default(),
            // 本地玩家标记
            LocalPlayer,
        ));

        tracing::info!(
            "🎮 已创建测试玩家实体: entity={:?}, grid=(332, 327), pixel=({}, {})",
            player_entity,
            332 * 48,
            327 * 32
        );

        self.initialized = true;
        tracing::info!("✅ MapViewerScene World 初始化完成");
    }
}

impl Scene for MapViewerScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut GameContext) -> GameResult<Option<SceneType>> {
        // tracing::info!("🔄 MapViewerScene::update() called");
        
        // 首次更新时初始化 World
        if !self.initialized {
            tracing::info!("🔧 Initializing World...");
            let (screen_width, screen_height) = ctx.drawable_size();
            self.initialize_world(&mut ctx.world, screen_width, screen_height);
        }

        // tracing::info!("📊 Updating FPS stats...");
        // 更新 FPS 统计（不控制帧率）
        if let Ok(mut time) = ctx.world.get::<&mut TimeTracker>(self.time_entity) {
            time.animation_count += 1;
            time.frame_count += 1;

            if time.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                time.fps = time.frame_count as f32 / time.last_fps_update.elapsed().as_secs_f32();
                time.frame_count = 0;
                time.last_fps_update = Instant::now();
            }
        }

      //  tracing::info!("🎮 Running system scheduler update...");
        let dt = ctx.time.delta().as_secs_f32();
        self.system_scheduler.update(ctx, dt)?;

        // tracing::info!("✅ MapViewerScene::update() completed");
        Ok(None)
    }

    fn draw(&mut self, ctx: &mut GameContext, canvas: &mut Canvas) -> GameResult {
      //  tracing::info!("🎨 MapViewerScene::draw() called");
         let (ctx, world) = ctx.split_gfx_world();
        self.system_scheduler.draw(ctx, canvas, world)?;
    //    tracing::info!("✅ MapViewerScene::draw() completed");
        Ok(())
    }
}
