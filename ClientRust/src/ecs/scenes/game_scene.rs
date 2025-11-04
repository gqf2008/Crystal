// ============================================================================
// 游戏场景 - 主游戏界面
// ============================================================================
//
// 职责：
// - 场景生命周期管理（初始化、更新、绘制）
// - 系统协调（Camera、Player、Render、Animation、Network、Monster、UI、Input）
// - 事件分发（键盘、鼠标事件委托给 InputSystem）
// - 网络事件处理（委托给 NetworkSystem）
//
// 架构特点：
// - 纯场景编排，不包含业务逻辑
// - 所有输入处理委托给 InputSystem
// - 坐标转换使用 CoordinateSystem
// - UI 管理委托给 UISystem
//
// 重构历史：
// - 从 1207 行减少到 681 行（-43.6%）
// - 移除 DialogManager，使用组件管理对话框状态
// - 移除所有输入处理逻辑到 InputSystem
// - 移除坐标转换逻辑到 CoordinateSystem
//
// ============================================================================

use ggez::graphics::Canvas;
use ggez::input::keyboard::KeyInput;
use ggez::GameResult;
use hecs::{Entity, World};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

use super::{Scene, SceneType};
use crate::ecs::render::{DebugSystem, UIRenderSystem};
use crate::ecs::systems::{
    // Layer 5
    // CharacterAnimationSystem,  // ❌ 已删除 - 未使用
    CameraSystem, // ✅ V2 版本

    CollisionSystem,
    CombatSystemV2 as CombatSystem,
    HealthRegenSystem,
    // Layer 2
    MonsterAISystem,
    // Layer 4
    MovementSystem,
    NpcDialogueSystem,
    ParticleSystem,
    // Layer 1
    PlayerControlSystem,
    // Layer 3
    SkillSystem,
    SoundSystem,
    SystemScheduler,   // ✅ V1 调度器
};
use crate::ecs::GameContext;
use crate::ecs::{
    components::{
        Camera, CameraMode, Draggable, Equipment, Inventory, LearnableMagicList, LocalPlayer,
        MagicList, MirClass, MirGender, MoveMode, Player, PlayerAction,
        PlayerAppearance, PlayerData, Position, RenderConfig, TargetSelection, TimeTracker,
        VisibleArea,
    },
    map_loader::MapLoader,

    ui::{
        CharacterDialog, ChatDialog, ChatType, HotkeyHelpPanel, InventoryDialog,
        MagicLearningDialog, MainDialog, OptionsDialog, QuestDialog, SkillBarDialog, SkillsDialog,
        TradeDialog,
    },
    // 🔒 暂时注释掉旧调度器
    // UpdateRenderParallelScheduler, ExecutionMode,
    Coord,
    MapUtils, // 坐标工具
};
use crate::graphics::libraries::initialize_all_libraries;
use crate::network::{handlers::GameEvent, NetContext};
use crate::objects::MapReader;

/// 游戏场景
pub struct GameScene {
    /// 相机实体
    camera_entity: Entity,

    /// 时间跟踪实体
    time_entity: Entity,

    /// 渲染配置实体
    config_entity: Entity,

    /// 可见区域缓存实体
    visible_area_entity: Entity,

    /// 调试计数器实体
    debug_counters_entity: Entity,

    /// UI 实体引用 (保留用于后续功能扩展)
    main_dialog_entity: Entity,
    #[allow(dead_code)]
    inventory_dialog_entity: Entity,
    #[allow(dead_code)]
    character_dialog_entity: Entity,
    #[allow(dead_code)]
    skillbar_entities: [Entity; 2],
    chat_dialog_entity: Entity,
    #[allow(dead_code)]
    magic_learning_dialog_entity: Entity,
    #[allow(dead_code)]
    quest_dialog_entity: Entity,
    #[allow(dead_code)]
    trade_dialog_entity: Entity,

    /// 网络同步系统 (TODO: 已删除，需重构)
    // network_system: ClientNetworkSystem,

    ///  系统调度器 V1 - 管理旧版 System trait 系统
    system_scheduler: SystemScheduler,

    /// UI字体名称 (保留用于后续字体切换功能)
    #[allow(dead_code)]
    ui_font_name: String,
}

impl GameScene {
    pub fn spawn(ctx: &mut GameContext) -> Self {
        // 初始化图形库
        println!("📚 正在初始化图形库...");
        initialize_all_libraries("Data").expect("初始化图形库失败");
        println!("✅ 图形库初始化完成");
        let world = &mut ctx.world;
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        // 创建相机实体
        let camera_entity = world.spawn((
            Position { x: 0.0, y: 0.0 },
            Camera {
                zoom: 1.25,
                screen_width,
                screen_height,
            },
            CameraMode::FollowPlayer, // 默认跟随玩家模式
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
            show_static_tiles: true,
            show_animated_tiles: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            show_player_debug: false, // F2键切换
            max_fps: 160,
            enable_lod: true,
            enable_camera_drag: false, // 正常游戏禁用拖拽
        },));

        // 创建可见区域缓存实体
        let visible_area_entity = world.spawn((VisibleArea::default(),));

        // 创建调试计数器实体
        let debug_counters_entity = world.spawn((crate::ecs::components::DebugCounters::new(),));

        // // 创建鼠标输入状态实体
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

        let ui_font_name = "AlibabaPuHuiTi".to_string();

        // 创建UI对话框实体
        let main_dialog_entity =
            world.spawn((MainDialog::new(Coord::DESIGN_WIDTH, Coord::DESIGN_HEIGHT),));

        let inventory_dialog_entity = world.spawn((InventoryDialog::new(),));
        let character_dialog_entity = world.spawn((CharacterDialog::new(),));

        let skillbar_entities = [
            world.spawn((SkillBarDialog::new(0),)),
            world.spawn((SkillBarDialog::new(1),)),
        ];

        let chat_dialog_entity = world.spawn((ChatDialog::new(0.0, screen_height - 300.0),));
        let magic_learning_dialog_entity = world.spawn((MagicLearningDialog::new(),));
        let quest_dialog_entity = world.spawn((QuestDialog::new(100.0, 100.0),));
        let trade_dialog_entity = world.spawn((TradeDialog::new(300.0, 150.0),));

        world.spawn((SkillsDialog::new(),));
        world.spawn((OptionsDialog::new(),));

        // 创建按键帮助面板
        let mut hotkey_help = HotkeyHelpPanel::new();
        hotkey_help.set_font(ui_font_name.clone());
        world.spawn((hotkey_help,));

        Self {
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            debug_counters_entity,
            main_dialog_entity,
            inventory_dialog_entity,
            character_dialog_entity,
            skillbar_entities,
            chat_dialog_entity,
            magic_learning_dialog_entity,
            quest_dialog_entity,
            trade_dialog_entity,
            system_scheduler: Self::create_system_scheduler(),

            ui_font_name,
        }
    }


    /// 创建并初始化所有 ECS 系统 (V1)
    ///
    /// 按照六层架构顺序添加系统：
    /// 1. 输入与网络 (50-199)
    /// 2. AI 与决策 (200-299)
    /// 3. 战斗与技能 (300-399)
    /// 4. 移动与物理 (400-499)
    /// 5. 状态更新 (500-599)
    /// 6. 网络同步 (600-699)
    /// 7. 事件清理 (900) - 最低优先级
    fn create_system_scheduler() -> SystemScheduler {
        let mut scheduler = SystemScheduler::new();

        tracing::info!("🎯 初始化 ECS 系统...");

        // ===== Layer 1: 输入与网络 (50-199) =====
        // PlayerControlSystem 已迁移到 V2 (零拷贝)
        scheduler
            .add_system(MonsterAISystem)
            .add_system(NpcDialogueSystem::new())
            .add_system(SkillSystem)
            .add_system(CombatSystem)
            .add_system(MovementSystem)
            .add_system(CollisionSystem::new())
            // 注意: CharacterAnimationSystem 已移除（未使用）
            .add_system(ParticleSystem)
            .add_system(HealthRegenSystem)
            .add_system(SoundSystem::new())
            .add_system(DebugSystem::new());

        tracing::info!("✅ ECS 系统初始化完成！");
        scheduler
    }

    /// 创建新的游戏场景
    ///
    /// # 架构设计 (完全ECS化)
    /// - GameScene只是一个场景编排器，不持有任何游戏数据
    /// - 不在构造函数中创建实体或加载资源
    /// - 所有实体创建由NetworkEventSystem在update循环中处理服务器事件时完成
    /// - 只初始化系统调度器
    ///
    /// # 返回
    /// - `Self`: 游戏场景实例（纯粹的系统调度器）

    // ========================================================================
    // UI 组件访问辅助方法
    // ========================================================================

    /// 获取聊天对话框的可变引用
    fn get_chat_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut ChatDialog> {
        world
            .query_one_mut::<&mut ChatDialog>(self.chat_dialog_entity)
            .ok()
    }

    /// 获取主对话框的可变引用
    fn get_main_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut MainDialog> {
        world
            .query_one_mut::<&mut MainDialog>(self.main_dialog_entity)
            .ok()
    }
}

impl Scene for GameScene {
    fn update(&mut self, game_ctx: &mut crate::ecs::GameContext) -> GameResult<Option<SceneType>> {
        // 帧率限制
        let config = game_ctx
            .world
            .get::<&RenderConfig>(self.config_entity)
            .unwrap();
        let max_fps = config.max_fps;
        drop(config);

        // 计算实际的帧时间（在更新TimeTracker之前）
        let delta_ms = if let Ok(time) = game_ctx.world.get::<&TimeTracker>(self.time_entity) {
            let elapsed = time.last_frame_time.elapsed();
            elapsed.as_millis().min(100) as u32 // 限制最大值防止卡顿时动画跳帧
        } else {
            16 // 默认约60fps
        };

        if let Ok(mut time) = game_ctx.world.get::<&mut TimeTracker>(self.time_entity) {
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

        // 计算 delta_time（秒）
        let delta_time = delta_ms as f32 / 1000.0;
        self.system_scheduler.update(game_ctx, delta_time)?;

        Ok(None)
    }

    fn draw(&mut self, ctx: &mut GameContext, canvas: &mut Canvas) -> GameResult {
        let (ctx, world) = ctx.split_gfx_world();
        self.system_scheduler.draw(ctx, canvas, world)?;
        Ok(())
    }

    // fn on_key_down(
    //     &mut self,
    //     _ctx: &mut Context,
    //     world: &mut World,
    //     input: KeyInput,
    // ) -> GameResult<Option<SceneType>> {
    //     use ggez::winit::keyboard::KeyCode;

    //     if let ggez::winit::event::KeyEvent {
    //         physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
    //         ..
    //     } = input.event
    //     {
    //         // Esc 键特殊处理 - 返回选择角色界面
    //         if keycode == KeyCode::Escape {
    //             return Ok(Some(SceneType::Select));
    //         }

    //         // H键 - 切换按键帮助面板
    //         // 优化说明: 从world查询HotkeyHelpPanel组件并修改状态
    //         if keycode == KeyCode::KeyH {
    //             for (_entity, hotkey_help) in world.query::<&mut HotkeyHelpPanel>().iter() {
    //                 hotkey_help.toggle();
    //                 tracing::info!(
    //                     "📖 按键帮助: {}",
    //                     if hotkey_help.visible {
    //                         "显示"
    //                     } else {
    //                         "隐藏"
    //                     }
    //                 );
    //                 break; // 只需要第一个HotkeyHelpPanel
    //             }
    //             return Ok(None);
    //         }

    //         // ✅ 键盘快捷键处理（UI切换、物品拾取、技能释放等）
    //         // TODO: KeyboardShortcutSystem已删除
    //         // KeyboardShortcutSystem::process_keyboard(world, keycode, network_tx);
    //     }

    //     Ok(None)
    // }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
