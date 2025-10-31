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

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use ggez::winit::event::MouseButton;
use ggez::input::keyboard::KeyInput;
use hecs::{World, Entity};
use std::time::Instant;
use std::sync::Arc;

use super::{Scene, SceneType};
use crate::network::{NetContext, handlers::GameEvent};
use crate::ecs::{
    components::{Position, Camera, Player, PlayerAction, MoveMode, Draggable, MouseInput, TimeTracker, RenderConfig, VisibleArea, PlayerAppearance, Inventory, MagicList, LearnableMagicList, LocalPlayer, PlayerData, TargetSelection, MirClass, MirGender, Equipment},
    systems::{
        // TODO: 很多系统已删除，GameScene需要重构以使用UpdateRenderParallelScheduler
        CameraSystem,
    },
    UpdateRenderParallelScheduler, ExecutionMode,  // 🆕 update/render调度器
    Coordinates, MapUtils,  // 坐标工具
    map_loader::MapLoader,
    ui::{ChatType, MainDialog, InventoryDialog, CharacterDialog, SkillBarDialog, ChatDialog, MagicLearningDialog, QuestDialog, TradeDialog, SkillsDialog, OptionsDialog, HotkeyHelpPanel},
};
use crate::objects::{MapReader};
use crate::graphics::libraries::initialize_all_libraries;

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
    
    ///  系统调度器 - 统一管理所有ECS系统
    system_scheduler: UpdateRenderParallelScheduler,
    
    /// UI字体名称 (保留用于后续字体切换功能)
    #[allow(dead_code)]
    ui_font_name: String,
}

impl GameScene {
    /// 创建新的游戏场景
    /// 
    /// # 架构设计 (完全ECS化)
    /// - GameScene只是一个场景编排器，不持有任何游戏数据
    /// - 不在构造函数中创建实体或加载资源
    /// - 所有实体创建由首次update调用的initialize()方法完成
    /// - Context和World通过Scene trait的方法参数自动传递
    /// 
    /// # 返回
    /// - `Self`: 游戏场景实例（纯粹的系统调度器）
    pub fn new() -> Self {
        println!("🎮 GameScene构造函数 - 创建空壳场景");
        println!("⏳ 等待首次update进行初始化...");
        
        Self {
            // 所有实体ID设为占位值，在首次update时初始化
            camera_entity: Entity::DANGLING,
            time_entity: Entity::DANGLING,
            config_entity: Entity::DANGLING,
            visible_area_entity: Entity::DANGLING,
            debug_counters_entity: Entity::DANGLING,
            main_dialog_entity: Entity::DANGLING,
            inventory_dialog_entity: Entity::DANGLING,
            character_dialog_entity: Entity::DANGLING,
            skillbar_entities: [Entity::DANGLING, Entity::DANGLING],
            chat_dialog_entity: Entity::DANGLING,
            magic_learning_dialog_entity: Entity::DANGLING,
            quest_dialog_entity: Entity::DANGLING,
            trade_dialog_entity: Entity::DANGLING,
            system_scheduler: UpdateRenderParallelScheduler::new(ExecutionMode::Sequential),
            ui_font_name: String::from("default"),
        }
    }
    
    /// 标记：场景是否已完成初始化
    fn is_initialized(&self) -> bool {
        self.camera_entity != Entity::DANGLING
    }
    
    /// 初始化场景实体（在首次update时调用）
    /// 
    /// Context和World由Scene trait的update方法传递，不需要在构造函数中传递
    fn initialize(&mut self, ctx: &mut Context, world: &mut World) -> GameResult {
        if self.is_initialized() {
            return Ok(());
        }
        
        println!("🎮 GameScene首次初始化...");
        
        // 初始化图形库
        println!("📚 正在初始化图形库...");
        initialize_all_libraries("Data").expect("初始化图形库失败");
        println!("✅ 图形库初始化完成");
        
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        
        // 创建相机实体
        self.camera_entity = world.spawn((
            Position { x: 0.0, y: 0.0 },
            Camera {
                zoom: 1.25,
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
        self.time_entity = world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));
        
        // 创建渲染配置实体
        self.config_entity = world.spawn((RenderConfig {
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
        self.visible_area_entity = world.spawn((VisibleArea::default(),));
        
        // 创建调试计数器实体
        self.debug_counters_entity = world.spawn((crate::ecs::components::DebugCounters::new(),));
        
        // 创建鼠标输入状态实体
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
        
        // 加载中文字体
        self.ui_font_name = Self::load_chinese_font(ctx)?;
        
        // 创建UI对话框实体
        self.main_dialog_entity = world.spawn((
            MainDialog::new(Coordinates::DESIGN_WIDTH, Coordinates::DESIGN_HEIGHT),
        ));
        
        self.inventory_dialog_entity = world.spawn((InventoryDialog::new(),));
        self.character_dialog_entity = world.spawn((CharacterDialog::new(),));
        
        self.skillbar_entities = [
            world.spawn((SkillBarDialog::new(0),)),
            world.spawn((SkillBarDialog::new(1),)),
        ];
        
        self.chat_dialog_entity = world.spawn((ChatDialog::new(0.0, screen_height - 300.0),));
        self.magic_learning_dialog_entity = world.spawn((MagicLearningDialog::new(),));
        self.quest_dialog_entity = world.spawn((QuestDialog::new(100.0, 100.0),));
        self.trade_dialog_entity = world.spawn((TradeDialog::new(300.0, 150.0),));
        
        world.spawn((SkillsDialog::new(),));
        world.spawn((OptionsDialog::new(),));
        
        // 创建按键帮助面板
        let mut hotkey_help = HotkeyHelpPanel::new();
        hotkey_help.set_font(self.ui_font_name.clone());
        world.spawn((hotkey_help,));
        
        println!("✅ GameScene初始化完成！");
        Ok(())
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
    // 网络事件处理 (委托给NetworkEventSystem)
    // ========================================================================
    
    /// 处理网络事件（委托给NetworkEventSystem）
    /// 
    /// GameScene作为场景编排器，不直接处理网络事件细节
    /// 所有网络事件处理逻辑都在NetworkEventSystem中
    pub fn handle_network_event(&mut self, world: &mut World, event: &GameEvent) {
        // 委托给NetworkEventSystem处理
        crate::ecs::systems::NetworkEventSystem::process_event(world, event);
    }
    
    // ========================================================================
    // UI 组件访问辅助方法
    // ========================================================================
    
    /// 获取聊天对话框的可变引用
    fn get_chat_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut ChatDialog> {
        world.query_one_mut::<&mut ChatDialog>(self.chat_dialog_entity).ok()
    }
    
    /// 获取主对话框的可变引用
    fn get_main_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut MainDialog> {
        world.query_one_mut::<&mut MainDialog>(self.main_dialog_entity).ok()
    }
    
    /// 加载中文字体
    fn load_chinese_font(ctx: &mut Context) -> GameResult<String> {
        use ggez::graphics::FontData;
        use std::path::Path;
        
        let font_configs = [
            ("C:/Windows/Fonts/msyh.ttc", "Microsoft YaHei"),
            ("C:/Windows/Fonts/simsun.ttc", "SimSun"),
        ];
        
        for (path, font_name) in &font_configs {
            if Path::new(path).exists() {
                match std::fs::read(path) {
                    Ok(bytes) => {
                        match FontData::from_vec(bytes) {
                            Ok(font_data) => {
                                ctx.gfx.add_font(*font_name, font_data);
                                println!("✅ 成功加载中文字体: {}", font_name);
                                return Ok(font_name.to_string());
                            }
                            Err(e) => {
                                println!("⚠️ 字体数据创建失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️ 字体文件读取失败: {}", e);
                    }
                }
            }
        }
        
        Ok(String::from("default"))
    }
}

impl Scene for GameScene {
    fn update(
        &mut self, 
        ctx: &mut Context, 
        world: &mut World,
        net_ctx: &Arc<NetContext>
    ) -> GameResult<Option<SceneType>> {
        // 🎯 首次update时初始化场景实体
        self.initialize(ctx, world)?;
        
        // 🆕 处理网络事件
        for event in net_ctx.try_recv() {
            self.handle_network_event(world, &event);
        }
        
        // 帧率限制
        let config = world.get::<&RenderConfig>(self.config_entity).unwrap();
        let max_fps = config.max_fps;
        drop(config);
        
        // 计算实际的帧时间（在更新TimeTracker之前）
        let delta_ms = if let Ok(time) = world.get::<&TimeTracker>(self.time_entity) {
            let elapsed = time.last_frame_time.elapsed();
            elapsed.as_millis().min(100) as u32 // 限制最大值防止卡顿时动画跳帧
        } else {
            16 // 默认约60fps
        };
        
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
        
        // 更新动画系统
        let show_animations = world
            .get::<&RenderConfig>(self.config_entity)
            .map(|c| c.show_animations)
            .unwrap_or(true);
        
        // 获取动画计数器（Layer 4动画系统需要）
        let animation_count: i32 = world
            .get::<&TimeTracker>(self.time_entity)
            .map(|t| t.animation_count)
            .unwrap_or(0);
        
        if show_animations {
            // ========================================================================
            // 🎯 Layer 4动画更新已统一到下面的五层架构部分
            // ========================================================================
        }
        
        // 🎯 更新鼠标按下时间计数器（用于长按检测）
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            if mouse_input.left_pressed {
                mouse_input.left_press_time += 1;
            }
            if mouse_input.right_pressed {
                mouse_input.right_press_time += 1;
            }
        }
        
        // 🎯 同步摄像机位置到玩家位置（确保摄像机始终跟随玩家）
        if let Some((_, (_, player_pos))) = world.query::<(&LocalPlayer, &Position)>().iter().next() {
            if let Ok(mut cam_pos) = world.get::<&mut Position>(self.camera_entity) {
                // � 使用 DebugCounters 组件记录日志（替代 unsafe static mut）
                if let Ok(mut debug) = world.get::<&mut crate::ecs::components::DebugCounters>(self.debug_counters_entity) {
                    if debug.should_log_sync() {
                        tracing::info!(
                            "📷 Camera同步: player=({:.1}, {:.1}), old_camera=({:.1}, {:.1})",
                            player_pos.x, player_pos.y, cam_pos.x, cam_pos.y
                        );
                    }
                }
                
                cam_pos.x = player_pos.x;
                cam_pos.y = player_pos.y;
            }
        }
        
        // 更新相机系统
        // TODO: CameraSystem需要实现System trait才能工作
        // CameraSystem::update(world);
        
        // ========================================
        // 使用UpdateRenderParallelScheduler执行update层系统
        // ========================================
        
        // 计算 delta_time（秒）
        let delta_time = delta_ms as f32 / 1000.0;
        
        // TODO: UpdateRenderParallelScheduler.update() 需要传入world和delta
        // 旧系统需要重构才能正常工作
        self.system_scheduler.update(world, delta_time)?;
        
        // 更新聊天对话框（用于光标闪烁）
        if let Some(chat_dialog) = self.get_chat_dialog_mut(world) {
            chat_dialog.update();
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 🎯 在绘制前同步摄像机位置到玩家位置（确保首帧就正确）
        // 必须在独立的作用域中完成，确保可变引用被drop
        {
            // 📊 使用 DebugCounters 组件记录绘制日志（替代 unsafe static mut）
            let should_log = if let Ok(mut debug) = world.get::<&mut crate::ecs::components::DebugCounters>(self.debug_counters_entity) {
                debug.should_log_draw()
            } else {
                false
            };
            
            let draw_count = if let Ok(debug) = world.get::<&crate::ecs::components::DebugCounters>(self.debug_counters_entity) {
                debug.get_draw_count()
            } else {
                0
            };
            
            // 查找LocalPlayer - 需要保存query以延长生命周期
            let mut player_query_iter = world.query::<(&LocalPlayer, &Position)>();
            let player_opt = player_query_iter.iter().next();
            
            // 调试：输出是否找到玩家
            if should_log {
                if let Some((_, (_, player_pos))) = player_opt.as_ref() {
                    tracing::info!(
                        "🎮 draw#{}: 找到LocalPlayer, pos=({:.1}, {:.1})",
                        draw_count, player_pos.x, player_pos.y
                    );
                } else {
                    tracing::warn!("⚠️ draw#{}: 未找到LocalPlayer!", draw_count);
                }
            }
            
            if let Some((_, (_, player_pos))) = player_opt {
                if let Ok(mut cam_pos) = world.get::<&mut Position>(self.camera_entity) {
                    // 计算距离用于调试
                    let distance = ((cam_pos.x - player_pos.x).powi(2) + (cam_pos.y - player_pos.y).powi(2)).sqrt();
                    
                    if should_log {
                        tracing::info!(
                            "📷 draw#{}: camera=({:.1}, {:.1}), distance={:.1}",
                            draw_count, cam_pos.x, cam_pos.y, distance
                        );
                    }
                    
                    // 🎯 强制同步Camera到玩家位置（确保draw前Camera在正确位置）
                    cam_pos.x = player_pos.x;
                    cam_pos.y = player_pos.y;
                    
                    if should_log && distance > 1.0 {
                        tracing::info!(
                            "🎯 draw#{} Camera同步: distance={:.1} -> player=({:.1}, {:.1})",
                            draw_count, distance, player_pos.x, player_pos.y
                        );
                    }
                }
            }
        } // 作用域结束，可变引用被drop
        
        // 获取相机组件（现在获取的是更新后的值）
        let (pos, camera) = {
            let pos = world.get::<&Position>(self.camera_entity).unwrap().clone();
            let camera = world.get::<&Camera>(self.camera_entity).unwrap().clone();
            (pos, camera)
        };
        
        let config = world.get::<&RenderConfig>(self.config_entity).ok().map(|r| (*r).clone()).unwrap();
        
        // ==================== 地图层: 使用窗口逻辑坐标 ====================
        // 设置画布使用窗口的逻辑分辨率(如 1280×960)
        // 这样 RenderSystem 的世界坐标转换才能正确工作
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            camera.screen_width,  
            camera.screen_height, 
        ));
        
        // 🎯 使用统一的渲染入口
        // TODO: RenderSystem已删除，需要使用UpdateRenderParallelScheduler的render方法
        // RenderSystem::draw_game_world(
        //     ctx,
        //     canvas,
        //     world,
        //     &pos,
        //     &camera,
        //     &config,
        //     self.visible_area_entity,
        //     self.debug_counters_entity,
        // )?;
        
        // ==================== UI 层: 使用设计坐标 1024×768 ====================
        // 设置画布使用设计分辨率坐标系,ggez 会自动缩放
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            Coordinates::DESIGN_WIDTH,   // 1024 (UI 设计分辨率)
            Coordinates::DESIGN_HEIGHT,  // 768 (UI 设计分辨率)
        ));
        
        // 🎯 使用 RenderSystem::draw_ui 统一渲染所有 UI（符合ECS设计原则）
        // 分层渲染: 调试UI -> 游戏UI -> 覆盖层UI
        // 优化说明: 移除所有参数,直接从ctx和world查询所需数据
        // TODO: RenderSystem已删除，需要重新实现UI渲染
        // RenderSystem::draw_ui(ctx, canvas, world)?;
        
        Ok(())
    }
    
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        input: KeyInput,
        _net_ctx: &Arc<NetContext>,
    ) -> GameResult<Option<SceneType>> {
        use ggez::winit::keyboard::KeyCode;
        
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            ..
        } = input.event
        {
            // Esc 键特殊处理 - 返回选择角色界面
            if keycode == KeyCode::Escape {
                return Ok(Some(SceneType::Select));
            }
            
            // H键 - 切换按键帮助面板
            // 优化说明: 从world查询HotkeyHelpPanel组件并修改状态
            if keycode == KeyCode::KeyH {
                for (_entity, hotkey_help) in world.query::<&mut HotkeyHelpPanel>().iter() {
                    hotkey_help.toggle();
                    tracing::info!("📖 按键帮助: {}", if hotkey_help.visible { "显示" } else { "隐藏" });
                    break; // 只需要第一个HotkeyHelpPanel
                }
                return Ok(None);
            }
            
            // ✅ 键盘快捷键处理（UI切换、物品拾取、技能释放等）
            // TODO: KeyboardShortcutSystem已删除
            // KeyboardShortcutSystem::process_keyboard(world, keycode, network_tx);
        }
        
        Ok(None)
    }
    
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

