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
use ggez::graphics::{Canvas, Color, Text, DrawParam};
use ggez::winit::event::MouseButton;
use ggez::input::keyboard::KeyInput;
use hecs::{World, Entity};
use std::time::Instant;
use tokio::sync::mpsc;

use super::{Scene, SceneType};
use crate::network::{NetworkCommand, game_client::GameEvent};
use crate::ecs::{
    components::{Position, Camera, Player, PlayerAction, MoveMode, Draggable, MouseInput, TimeTracker, RenderConfig, VisibleArea, PlayerAppearance, Inventory, MagicList, LearnableMagicList, LocalPlayer, PlayerComp, TargetSelection, MirClass, MirGender, Equipment, QuestLog, TradeWindow},
    systems::{CameraSystem, PlayerSystem, RenderSystem, AnimationSystem, NetworkSystem, MonsterSystem, UISystem, InputSystem, CoordinateSystem},
    map_helper::MapHelper,
    map_loader::MapLoader,
    ui::{ChatType, MainDialogComp, InventoryDialogComp, CharacterDialogComp, SkillBarComp, ChatDialogComp, MagicLearningDialogComp, QuestDialogComp, TradeDialogComp, SkillsDialogComp, OptionsDialogComp},
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
    
    /// 网络同步系统
    network_system: NetworkSystem,
    
    /// UI 系统
    ui_system: UISystem,
    
    /// UI字体名称 (保留用于后续字体切换功能)
    #[allow(dead_code)]
    ui_font_name: String,
}

impl GameScene {
    /// 创建新的游戏场景
    /// 
    /// # 功能
    /// - 初始化地图库
    /// - 加载地图数据
    /// - 创建相机、玩家、UI 等实体
    /// - 初始化各个系统
    /// 
    /// # 参数
    /// - `ctx`: ggez 上下文
    /// - `world`: ECS 世界
    /// 
    /// # 返回
    /// - `Ok(GameScene)`: 成功创建的游戏场景
    /// - `Err`: 初始化失败的错误信息
    pub fn new(ctx: &mut Context, world: &mut World, player_grid_location: Option<(i32, i32)>) -> GameResult<Self> {
        println!("🗺️ 游戏场景初始化中...");
        
        // 初始化地图库
        println!("📚 正在初始化地图库...");
        initialize_all_libraries("Data").expect("初始化地图库失败");
        println!("✅ 地图库初始化完成");
        
        // 加载地图
        let map_path = "Map/0.map";
        println!("🗺️ 正在加载地图: {}", map_path);
        let reader = MapReader::new(map_path)?;
        println!("✅ 地图加载完成: {}x{}", reader.width, reader.height);
        
        // 加载地图瓦片到 ECS
        MapLoader::load_map(world, reader)?;
        
        // 找到出生点
        let map_data = world.query_mut::<&crate::ecs::components::MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone())
            .expect("地图数据未加载");
        
        // ✅ 使用服务器发送的玩家位置，如果没有则使用地图中心
        let (player_grid_x, player_grid_y) = if let Some((x, y)) = player_grid_location {
            println!("✅ 使用服务器玩家位置: 格子({}, {})", x, y);
            (x, y)
        } else {
            let (x, y) = MapHelper::find_center_walkable_position(&map_data);
            println!("⚠️ 未找到服务器位置，使用地图中心: 格子({}, {})", x, y);
            (x, y)
        };
        
        let (player_world_x, player_world_y) = MapHelper::grid_to_world(player_grid_x, player_grid_y);
        println!("� 玩家世界坐标: ({:.1}, {:.1})", player_world_x, player_world_y);
        
        // 创建相机实体
        // 使用 drawable_size() 获取窗口尺寸,ggez 会自动处理 DPI 缩放
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        tracing::info!("📐 窗口尺寸: {}x{} | UI设计: {}x{}", 
                      screen_width, screen_height, 
                      CoordinateSystem::DESIGN_WIDTH, CoordinateSystem::DESIGN_HEIGHT);
        let camera_entity = world.spawn((
            Position { x: player_world_x, y: player_world_y },  // 📍 使用玩家真实位置
            Camera {
                zoom: 1.25,  // 初始缩放1.75x，让角色看起来更大
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
            show_path: false,
            max_fps: 160,
            enable_lod: true,
        },));
        
        // 创建可见区域缓存实体
        let visible_area_entity = world.spawn((VisibleArea::default(),));
        
        // 生成测试怪物
        println!("👹 正在生成测试怪物...");
        MapLoader::spawn_test_monsters(world, &map_data, 15);
        println!("✅ 已生成 15 只测试怪物");
        
        // ✅ 使用服务器发送的玩家真实位置创建角色实体
        // 创建玩家角色实体
        let _player_entity = world.spawn((
            Player {
                direction: 4,  // 朝下
                action: PlayerAction::Stand,
                frame_index: 0,
                frame_time: 0,
                speed: 0.0,
                target_x: player_world_x,  // 📍 使用真实位置
                target_y: player_world_y,  // 📍 使用真实位置
                is_moving: false,
                path: Vec::new(),
                path_index: 0,
                move_mode: MoveMode::Idle,
                last_move_time: std::time::Instant::now(),
                move_delay: std::time::Duration::from_millis(600), // 服务器MoveDelay
                waiting_server_confirm: false,  // 🎯 初始不等待确认
            },
            Position { x: player_world_x, y: player_world_y },  // 📍 使用真实位置
            PlayerAppearance::default(),  // 默认外观（战士男）
            Inventory::default(),  // 默认背包（40格）
            Equipment::new(),  // 装备栏
            LocalPlayer,  // 本地玩家标记
            PlayerComp {
                id: 1,
                name: "勇士".to_string(),
                class: MirClass::Warrior,
                gender: MirGender::Male,
                exp: 750,
                gold: 100,
            },
            MagicList::new(),  // 已学技能列表
            LearnableMagicList::new(),  // 可学技能列表
            TargetSelection::new(),  // 目标选择
            QuestLog::new(),  // ✅ 任务日志（用于任务系统）
            TradeWindow::new(),  // ✅ 交易窗口（用于玩家交易）
            crate::ecs::components::NetworkSync {  // ✅ 网络同步标记（立即允许渲染）
                object_id: 0,
                last_update: std::time::Instant::now(),
                object_type: crate::ecs::components::NetworkObjectType::Player,
            },
        ));
        
        println!("✅ 本地玩家已创建，包含任务日志和交易窗口组件");
        
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
        
        // 角色状态
        let _status_entity = world.spawn((
            crate::ecs::ui::CharacterStatus {
                name: "勇士".to_string(),
                level: 10,
                health: 120,
                max_health: 150,
                mana: 45,
                max_mana: 80,
                experience: 750,
                max_experience: 1000,
            },
        ));
        
        // 血条
        let _health_bar_entity = world.spawn((crate::ecs::ui::HealthBar::default(),));
        
        // 魔法条
        let _mana_bar_entity = world.spawn((crate::ecs::ui::ManaBar::default(),));
        
        // 经验条
        let _exp_bar_entity = world.spawn((crate::ecs::ui::ExpBar::new(screen_width, screen_height),));
        
        // 技能栏
        let _skill_bar_entity = world.spawn((crate::ecs::ui::SkillBar::default(),));
        
        // 聊天窗口
        let mut chat = crate::ecs::ui::ChatWindow::new(screen_width, screen_height);
        // 添加一些测试消息
        chat.add_message(crate::ecs::ui::ChatMessage {
            sender: "系统".to_string(),
            content: "欢迎来到传奇世界！".to_string(),
            msg_type: crate::ecs::ui::ChatMessageType::System,
            timestamp: Instant::now(),
        });
        chat.add_message(crate::ecs::ui::ChatMessage {
            sender: "GM".to_string(),
            content: "游戏测试中...".to_string(),
            msg_type: crate::ecs::ui::ChatMessageType::Normal,
            timestamp: Instant::now(),
        });
        let _chat_entity = world.spawn((chat,));
        
        // 加载中文字体
        let ui_font_name = Self::load_chinese_font(ctx)?;
        
        // 创建主对话框实体
        // UI 使用固定设计分辨率 1024×768
        let main_dialog_entity = world.spawn((
            MainDialogComp::new(CoordinateSystem::DESIGN_WIDTH, CoordinateSystem::DESIGN_HEIGHT),
        ));
        
        // 创建背包对话框实体
        let inventory_dialog_entity = world.spawn((
            InventoryDialogComp::new(),
        ));
        
        // 创建角色对话框实体
        let character_dialog_entity = world.spawn((
            CharacterDialogComp::new(),
        ));
        
        // 创建两个技能栏实体
        let skillbar_entities = [
            world.spawn((SkillBarComp::new(0),)),
            world.spawn((SkillBarComp::new(1),)),
        ];
        
        // 创建聊天对话框实体
        let chat_dialog_entity = world.spawn((
            ChatDialogComp::new(0.0, screen_height - 300.0), // 屏幕底部
        ));
        
        // 创建技能学习对话框实体
        let magic_learning_dialog_entity = world.spawn((
            MagicLearningDialogComp::new(),
        ));
        
        // 创建任务对话框实体
        let quest_dialog_entity = world.spawn((
            QuestDialogComp::new(100.0, 100.0),
        ));
        
        // 创建技能对话框实体
        let _skills_dialog_entity = world.spawn((
            SkillsDialogComp::new(),
        ));
        
        // 创建选项对话框实体
        let _options_dialog_entity = world.spawn((
            OptionsDialogComp::new(),
        ));
        
        // 创建交易窗口实体
        let trade_dialog_entity = world.spawn((
            TradeDialogComp::new(300.0, 150.0),
        ));
        
        // 添加欢迎消息
        UISystem::add_chat_message(
            world,
            chat_dialog_entity,
            "欢迎来到传奇世界！".to_string(),
            ChatType::System,
        );
        UISystem::add_chat_message(
            world,
            chat_dialog_entity,
            "游戏测试中...".to_string(),
            ChatType::Normal,
        );
        
        println!("✅ 游戏场景初始化完成！");
        
        Ok(Self {
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            network_system: NetworkSystem::new(),
            ui_system: UISystem::new(),
            ui_font_name,
            main_dialog_entity,
            inventory_dialog_entity,
            character_dialog_entity,
            skillbar_entities,
            chat_dialog_entity,
            magic_learning_dialog_entity,
            quest_dialog_entity,
            trade_dialog_entity,
        })
    }
    
    /// 处理网络事件（由GameApp调用）
    pub fn handle_network_event(&mut self, world: &mut World, event: &GameEvent) {
        self.network_system.process_event(world, event);
    }
    
    // ========================================================================
    // UI 组件访问辅助方法
    // ========================================================================
    
    /// 获取聊天对话框的可变引用
    fn get_chat_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut ChatDialogComp> {
        world.query_one_mut::<&mut ChatDialogComp>(self.chat_dialog_entity).ok()
    }
    
    /// 获取主对话框的可变引用
    fn get_main_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut MainDialogComp> {
        world.query_one_mut::<&mut MainDialogComp>(self.main_dialog_entity).ok()
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
        _ctx: &mut Context, 
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) -> GameResult<Option<SceneType>> {
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
        
        // 更新动画系统
        let show_animations = world
            .get::<&RenderConfig>(self.config_entity)
            .map(|c| c.show_animations)
            .unwrap_or(true);
        
        if show_animations {
            let animation_count = world
                .get::<&TimeTracker>(self.time_entity)
                .map(|t| t.animation_count)
                .unwrap_or(0);
            
            AnimationSystem::update(world, animation_count);
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
                // 🐛 添加调试日志
                static mut SYNC_COUNTER: u32 = 0;
                unsafe {
                    SYNC_COUNTER += 1;
                    if SYNC_COUNTER == 1 || SYNC_COUNTER % 300 == 0 {  // 首次和每300帧
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
        CameraSystem::update(world);
        
        // 更新角色系统（会处理双击事件）- 传递 network_tx 用于位置同步
        PlayerSystem::update(world, Some(network_tx));
        
        // 🎯 重置双击标志（在PlayerSystem处理完之后清除，避免重复触发）
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            if mouse_input.left_double_clicked {
                mouse_input.left_double_clicked = false;
            }
            if mouse_input.right_double_clicked {
                mouse_input.right_double_clicked = false;
            }
        }
        
        // 更新怪物系统
        let delta_time = 1.0 / max_fps as f32;
        MonsterSystem::update(world, delta_time);
        
        // 更新聊天对话框（用于光标闪烁）
        if let Some(chat_dialog) = self.get_chat_dialog_mut(world) {
            chat_dialog.dialog.update();
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 🎯 在绘制前同步摄像机位置到玩家位置（确保首帧就正确）
        // 必须在独立的作用域中完成，确保可变引用被drop
        {
            static mut DRAW_COUNT: u32 = 0;
            unsafe { DRAW_COUNT += 1; }
            
            // 查找LocalPlayer - 需要保存query以延长生命周期
            let mut player_query_iter = world.query::<(&LocalPlayer, &Position)>();
            let player_opt = player_query_iter.iter().next();
            
            // 调试：输出是否找到玩家
            unsafe {
                if DRAW_COUNT <= 3 {
                    if let Some((_, (_, player_pos))) = player_opt.as_ref() {
                        tracing::info!(
                            "🎮 draw#{}: 找到LocalPlayer, pos=({:.1}, {:.1})",
                            DRAW_COUNT, player_pos.x, player_pos.y
                        );
                    } else {
                        tracing::warn!("⚠️ draw#{}: 未找到LocalPlayer!", DRAW_COUNT);
                    }
                }
            }
            
            if let Some((_, (_, player_pos))) = player_opt {
                if let Ok(mut cam_pos) = world.get::<&mut Position>(self.camera_entity) {
                    // 计算距离用于调试
                    let distance = ((cam_pos.x - player_pos.x).powi(2) + (cam_pos.y - player_pos.y).powi(2)).sqrt();
                    
                    unsafe {
                        if DRAW_COUNT <= 3 {
                            tracing::info!(
                                "📷 draw#{}: camera=({:.1}, {:.1}), distance={:.1}",
                                DRAW_COUNT, cam_pos.x, cam_pos.y, distance
                            );
                        }
                    }
                    
                    // 🎯 强制同步Camera到玩家位置（确保draw前Camera在正确位置）
                    cam_pos.x = player_pos.x;
                    cam_pos.y = player_pos.y;
                    
                    unsafe {
                        if DRAW_COUNT <= 3 && distance > 1.0 {
                            tracing::info!(
                                "🎯 draw#{} Camera同步: distance={:.1} -> player=({:.1}, {:.1})",
                                DRAW_COUNT, distance, player_pos.x, player_pos.y
                            );
                        }
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
        
        let config = world.get::<&RenderConfig>(self.config_entity).unwrap().clone();
        
        // ==================== 地图层: 使用窗口逻辑坐标 ====================
        // 设置画布使用窗口的逻辑分辨率(如 1280×960)
        // 这样 RenderSystem 的世界坐标转换才能正确工作
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            camera.screen_width,  
            camera.screen_height, 
        ));
        
        // 渲染地图瓦片
        RenderSystem::draw_tiles(
            ctx,
            canvas,
            world,
            &pos,
            &camera,
            &config,
            self.visible_area_entity,
        )?;
        
        // 渲染怪物
        RenderSystem::draw_monsters(ctx, canvas, world, &pos, &camera)?;
        
        // 渲染角色
        for (_entity, (player, player_pos)) in world.query::<(&Player, &Position)>().iter() {
            RenderSystem::draw_player_with_world(ctx, canvas, world, player, player_pos, &pos, &camera)?;
        }
        
        // 渲染怪物血条和名称
        RenderSystem::draw_monster_info(ctx, canvas, world, &pos, &camera)?;
        
        // 🎯 绘制网格 (调试用 - G键切换)
        if config.show_grid {
            RenderSystem::draw_grid(ctx, canvas, world, &pos, &camera)?;
        }
        
        // 🎯 绘制障碍物 (调试用 - O键切换)
        if config.show_obstacles {
            RenderSystem::draw_obstacles(ctx, canvas, world, &pos, &camera)?;
        }
        
        // 🎯 绘制寻路路径 (调试用 - P键切换)
        if config.show_path {
            RenderSystem::draw_path(ctx, canvas, world, &pos, &camera)?;
        }
        
        // ==================== UI 层: 使用设计坐标 1024×768 ====================
        // 设置画布使用设计分辨率坐标系,ggez 会自动缩放
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            CoordinateSystem::DESIGN_WIDTH,   // 1024 (UI 设计分辨率)
            CoordinateSystem::DESIGN_HEIGHT,  // 768 (UI 设计分辨率)
        ));
        
        // 绘制FPS
        let time = world.get::<&TimeTracker>(self.time_entity).unwrap();
        let fps_text = Text::new(format!("FPS: {:.1}", time.fps));
        canvas.draw(
            &fps_text,
            DrawParam::default()
                .dest([10.0, 10.0])
                .color(Color::from_rgb(0, 255, 0)),
        );
        
        // 绘制操作提示（移到右上角，使用设计坐标系）
        let hint_text = Text::new("[WASD/方向键] 移动  [Shift+WASD] 跑动  [鼠标] 点击移动  [Esc] 返回");
        canvas.draw(
            &hint_text,
            DrawParam::default()
                .dest([CoordinateSystem::DESIGN_WIDTH - 500.0, 10.0])
                .color(Color::from_rgb(200, 200, 200)),
        );
        
        // 🎯 只使用 UISystem 渲染所有 UI组件（移除UIRenderer避免重复绘制）
        self.ui_system.draw(ctx, canvas, world, 0)?; // TODO: 传递正确的 current_time
        
        Ok(())
    }
    
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        input: KeyInput,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
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
            
            // ✅ 所有其他键盘输入交给 InputSystem 处理
            InputSystem::process_keyboard(world, keycode, network_tx);
        }
        
        Ok(None)
    }
    
    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // ✅ 使用 CoordinateSystem 转换为 UI 设计坐标 (1024×768)
        let (ui_x, ui_y) = CoordinateSystem::window_to_ui_coords(ctx, x, y);
        
        // ✅ 委托给 InputSystem 处理所有鼠标点击逻辑
        InputSystem::process_mouse_click(world, button, ui_x, ui_y, x, y, network_tx);
        
        Ok(())
    }
    
    fn on_mouse_up(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // ✅ 委托给 InputSystem 处理鼠标抬起和双击检测
        InputSystem::process_mouse_up(world, button, x, y);
        
        Ok(())
    }
    
    fn on_mouse_move(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // ✅ 使用 CoordinateSystem 转换为 UI 设计坐标 (1024×768)
        let (ui_x, ui_y) = CoordinateSystem::window_to_ui_coords(ctx, x, y);
        
        // ✅ 委托给 InputSystem 处理鼠标移动
        InputSystem::process_mouse_move(world, x, y, ui_x, ui_y);
        
        Ok(())
    }
    
    fn on_mouse_wheel(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // ✅ 委托给 InputSystem 处理滚轮缩放
        InputSystem::process_mouse_wheel(world, self.camera_entity, x, y);
        
        Ok(())
    }
    
    fn on_resize(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        width: f32,
        height: f32,
    ) -> GameResult {
        // 忽略无效尺寸(避免启动时的闪烁)
        if width <= 1.0 || height <= 1.0 {
            println!("⚠️ 忽略无效窗口尺寸: {}x{}", width, height);
            return Ok(());
        }
        
        // 更新相机尺寸
        if let Ok(mut camera) = world.get::<&mut Camera>(self.camera_entity) {
            camera.screen_width = width;
            camera.screen_height = height;
        }
        
        // 更新主对话框尺寸
        if let Some(main_dialog) = self.get_main_dialog_mut(world) {
            main_dialog.dialog.resize(width, height);
        }
        
        println!("📐 窗口调整: {}x{}", width, height);
        
        Ok(())
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
