// ============================================================================
// 游戏场景 - 基于地图查看器扩展
// ============================================================================
//
// 功能：
// - 地图渲染（复用地图查看器系统）
// - 角色控制
// - NPC/怪物显示
// - UI系统
// - 网络同步
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
    systems::{CameraSystem, PlayerSystem, RenderSystem, AnimationSystem, NetworkSystem, MonsterSystem, UISystem, MagicLearningSystem, QuestSystem},
    map_helper::MapHelper,
    map_loader::MapLoader,
    ui::{MainDialogButton, InventoryAction, CharacterAction, ChatType, MainDialogComp, InventoryDialogComp, CharacterDialogComp, SkillBarComp, ChatDialogComp, MagicLearningDialogComp, QuestDialogComp, TradeDialogComp},
};
use crate::objects::{MapReader, PathFinder};
use crate::graphics::libraries::initialize_all_libraries;
use mir2_shared::Point;

// UI 设计分辨率 (所有 UI 元素都基于此设计)
const DESIGN_WIDTH: f32 = 1024.0;
const DESIGN_HEIGHT: f32 = 768.0;

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
    
    /// UI 实体引用
    main_dialog_entity: Entity,
    inventory_dialog_entity: Entity,
    character_dialog_entity: Entity,
    skillbar_entities: [Entity; 2],
    chat_dialog_entity: Entity,
    magic_learning_dialog_entity: Entity, // 🆕 技能学习对话框
    quest_dialog_entity: Entity,          // 🆕 任务对话框
    trade_dialog_entity: Entity,          // 🆕 交易窗口
    
    /// 对话框管理器 - 统一管理所有对话框的显示/隐藏、快捷键等
    dialog_manager: crate::ecs::ui::DialogManager,
    
    /// 网络同步系统
    network_system: NetworkSystem,
    
    /// UI 系统
    ui_system: UISystem,
    
    /// UI字体名称
    ui_font_name: String,
}

impl GameScene {
    pub fn new(ctx: &mut Context, world: &mut World) -> GameResult<Self> {
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
        
        let (spawn_grid_x, spawn_grid_y) = MapHelper::find_center_walkable_position(&map_data);
        let (spawn_x, spawn_y) = MapHelper::grid_to_world(spawn_grid_x, spawn_grid_y);
        
        println!("🧙 出生位置: 格子({}, {}) -> 世界坐标({:.1}, {:.1})", 
                 spawn_grid_x, spawn_grid_y, spawn_x, spawn_y);
        
        // 创建相机实体
        // 使用 drawable_size() 获取窗口尺寸,ggez 会自动处理 DPI 缩放
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        tracing::info!("📐 窗口尺寸: {}x{} | UI设计: {}x{}", 
                      screen_width, screen_height, 
                      DESIGN_WIDTH, DESIGN_HEIGHT);
        let camera_entity = world.spawn((
            Position { x: spawn_x, y: spawn_y },
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
        
        // 创建玩家角色实体
        let _player_entity = world.spawn((
            Player {
                direction: 4,  // 朝下
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
            },
            Position { x: spawn_x, y: spawn_y },
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
            MainDialogComp::new(DESIGN_WIDTH, DESIGN_HEIGHT),
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
            dialog_manager: crate::ecs::ui::DialogManager::new(),  // 🆕 初始化对话框管理器
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
    
    /// 将窗口逻辑坐标转换为 UI 设计坐标系（1024×768）
    /// 将窗口坐标转换为 UI 设计坐标 (1024×768)
    /// ggez 会自动处理 DPI 缩放,我们只需要使用 drawable_size()
    fn window_to_ui_coords(&self, ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let (window_width, window_height) = ctx.gfx.drawable_size();
        
        // UI 固定使用设计分辨率 1024×768
        let design_width = DESIGN_WIDTH;
        let design_height = DESIGN_HEIGHT;
        
        // 计算4:3视口
        let aspect_ratio = 4.0 / 3.0;
        let current_ratio = window_width / window_height;
        
        let (viewport_width, viewport_height) = if current_ratio > aspect_ratio {
            (window_height * aspect_ratio, window_height)
        } else {
            (window_width, window_width / aspect_ratio)
        };
        
        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;
        
        // 转换：窗口坐标 -> 视口坐标 -> 设计坐标
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;
        
        let design_x = (viewport_x / viewport_width) * design_width;
        let design_y = (viewport_y / viewport_height) * design_height;
        
        (design_x, design_y)
    }
    
    /// 获取主对话框的可变引用
    fn get_main_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut MainDialogComp> {
        world.query_one_mut::<&mut MainDialogComp>(self.main_dialog_entity).ok()
    }
    
    /// 获取背包对话框的可变引用
    fn get_inventory_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut InventoryDialogComp> {
        world.query_one_mut::<&mut InventoryDialogComp>(self.inventory_dialog_entity).ok()
    }
    
    /// 获取角色对话框的可变引用
    fn get_character_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut CharacterDialogComp> {
        world.query_one_mut::<&mut CharacterDialogComp>(self.character_dialog_entity).ok()
    }
    
    /// 获取聊天对话框的可变引用
    fn get_chat_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut ChatDialogComp> {
        world.query_one_mut::<&mut ChatDialogComp>(self.chat_dialog_entity).ok()
    }
    
    /// 获取技能学习对话框的可变引用
    fn get_magic_learning_dialog_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut MagicLearningDialogComp> {
        world.query_one_mut::<&mut MagicLearningDialogComp>(self.magic_learning_dialog_entity).ok()
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
    
    // /// 处理双击寻路
    // fn handle_double_click_pathfinding(&self, world: &mut World, screen_x: f32, screen_y: f32) {
    //     // 获取相机信息
    //     let (camera_x, camera_y, camera_width, camera_height) = {
    //         let pos = world.get::<&Position>(self.camera_entity).unwrap();
    //         let camera = world.get::<&Camera>(self.camera_entity).unwrap();
    //         (pos.x, pos.y, camera.screen_width, camera.screen_height)
    //     };
        
    //     // 屏幕坐标转世界坐标
    //     let world_x = camera_x + screen_x - camera_width / 2.0;
    //     let world_y = camera_y + screen_y - camera_height / 2.0;
        
    //     // 世界坐标转格子坐标
    //     let (target_grid_x, target_grid_y) = MapHelper::world_to_grid(world_x, world_y);
        
    //     println!("🎯 双击寻路: 屏幕({:.1}, {:.1}) -> 世界({:.1}, {:.1}) -> 格子({}, {})", 
    //              screen_x, screen_y, world_x, world_y, target_grid_x, target_grid_y);
        
    //     // 获取地图数据
    //     let map_data = if let Some((_, data)) = world.query_mut::<&crate::ecs::components::MapData>().into_iter().next() {
    //         data.clone()
    //     } else {
    //         println!("❌ 无法获取地图数据");
    //         return;
    //     };
        
    //     // 检查目标是否可行走
    //     if !MapHelper::is_walkable(&map_data, target_grid_x, target_grid_y) {
    //         println!("❌ 目标位置不可行走");
    //         return;
    //     }
        
    //     // 获取玩家当前位置
    //     let player_query = world.query_mut::<(&mut Player, &Position)>();
    //     if let Some((_, (player, player_pos))) = player_query.into_iter().next() {
    //         let (start_grid_x, start_grid_y) = MapHelper::world_to_grid(player_pos.x, player_pos.y);
            
    //         println!("📍 玩家位置: 世界({:.1}, {:.1}) -> 格子({}, {})", 
    //                  player_pos.x, player_pos.y, start_grid_x, start_grid_y);
            
    //         // 创建寻路器
    //         let map_data_clone = map_data.clone();
    //         let pathfinder = PathFinder::new(
    //             map_data.width,
    //             map_data.height,
    //             Box::new(move |p: Point| !MapHelper::is_walkable(&map_data_clone, p.x, p.y))
    //         );
            
    //         // 执行寻路
    //         let start_point = Point { x: start_grid_x, y: start_grid_y };
    //         let target_point = Point { x: target_grid_x, y: target_grid_y };
            
    //         if let Some(path) = pathfinder.find_path(start_point, target_point) {
    //             println!("✅ 找到路径，长度: {}", path.len());
    //             // 转换 Vec<Point> 为 Vec<(i32, i32)>
    //             player.path = path.iter().map(|p| (p.x, p.y)).collect();
    //             player.path_index = 0;
    //             player.is_moving = true;
    //             player.move_mode = MoveMode::AutoPathfinding;
    //         } else {
    //             println!("❌ 无法找到路径");
    //         }
    //     }
    // }
    
    /// 施放技能栏中的技能
    fn cast_spell_in_slot(
        &mut self,
        world: &mut World,
        slot: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use crate::ecs::systems::MagicCastSystem;
        use crate::ecs::components::{LocalPlayer, MagicList};
        
        // 从技能栏获取技能 (通过 key_slot 查找)
        let spell = {
            let mut spell_opt = None;
            for (_, (_, magic_list)) in world.query::<(&LocalPlayer, &MagicList)>().iter() {
                // 查找绑定到该槽位的技能
                if let Some(learned_magic) = magic_list.get_by_slot(slot as u8) {
                    spell_opt = Some(learned_magic.spell);
                }
                break;
            }
            spell_opt
        };
        
        if let Some(spell_type) = spell {
            // 施放技能
            MagicCastSystem::cast_spell(world, spell_type, network_tx);
        } else {
            println!("⚠️ 技能栏 F{} 未绑定技能", slot + 1);
        }
    }
}

impl Scene for GameScene {
    fn update(
        &mut self, 
        _ctx: &mut Context, 
        world: &mut World,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>
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
        
        // 更新相机系统
        CameraSystem::update(world);
        
        // 更新角色系统（会处理双击事件）
        PlayerSystem::update(world);
        
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
        if let Some(mut chat_dialog) = self.get_chat_dialog_mut(world) {
            chat_dialog.dialog.update();
        }
        
        // 更新主对话框（用于输入框光标闪烁）
        if let Some(mut main_dialog) = self.get_main_dialog_mut(world) {
            main_dialog.dialog.update();
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 获取相机组件
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
            DESIGN_WIDTH,   // 1024 (UI 设计分辨率)
            DESIGN_HEIGHT,  // 768 (UI 设计分辨率)
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
                .dest([DESIGN_WIDTH - 500.0, 10.0])
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
            match keycode {
                // 空格键 = 拾取物品
                KeyCode::Space => {
                    // 获取玩家位置
                    if let Some((_, pos)) = world.query::<&Position>().iter().next() {
                        let (grid_x, grid_y) = pos.to_grid();
                        let _ = network_tx.send(NetworkCommand::PickupItem { 
                            location: (grid_x, grid_y),
                        });
                    }
                }
                
                // Esc 返回选择角色
                KeyCode::Escape => {
                    return Ok(Some(SceneType::Select));
                }
                
                // K键 - 打开技能学习对话框
                KeyCode::KeyK => {
                    if let Some(dialog) = self.get_magic_learning_dialog_mut(world) {
                        dialog.dialog.toggle();
                        // 更新可学习技能列表
                        MagicLearningSystem::update_available_magics(world);
                        println!("📖 打开技能学习对话框");
                    }
                }
                
                // Q键 - 打开任务对话框
                KeyCode::KeyQ => {
                    // 先获取任务列表
                    let active_quests = QuestSystem::get_active_quests(world);
                    
                    // 再更新UI
                    for (_, dialog) in world.query_mut::<&mut QuestDialogComp>() {
                        if dialog.is_open {
                            dialog.close();
                        } else {
                            dialog.open();
                            dialog.update_active_quests(active_quests);
                            println!("📜 打开任务对话框");
                        }
                        break;
                    }
                }
                
                // T键 - 打开交易窗口 (测试用，实际应由交易请求触发)
                KeyCode::KeyT => {
                    for (_, dialog) in world.query_mut::<&mut TradeDialogComp>() {
                        if dialog.is_open {
                            dialog.close();
                            println!("🚫 关闭交易窗口");
                        } else {
                            // 测试：创建虚拟交易数据
                            use crate::ecs::systems::TradeData;
                            let test_trade = TradeData::new(999, "测试玩家".to_string());
                            dialog.open(test_trade);
                            println!("🤝 打开交易窗口 (测试)");
                        }
                        break;
                    }
                }
                
                // F1-F8 技能快捷键
                KeyCode::F1 => self.cast_spell_in_slot(world, 0, network_tx),
                KeyCode::F2 => self.cast_spell_in_slot(world, 1, network_tx),
                KeyCode::F3 => self.cast_spell_in_slot(world, 2, network_tx),
                KeyCode::F4 => self.cast_spell_in_slot(world, 3, network_tx),
                KeyCode::F5 => self.cast_spell_in_slot(world, 4, network_tx),
                KeyCode::F6 => self.cast_spell_in_slot(world, 5, network_tx),
                KeyCode::F7 => self.cast_spell_in_slot(world, 6, network_tx),
                KeyCode::F8 => self.cast_spell_in_slot(world, 7, network_tx),
                
                // 1-8 数字键 - 使用物品栏物品 (对应背包前8个格子)
                KeyCode::Digit1 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 0, network_tx);
                }
                KeyCode::Digit2 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 1, network_tx);
                }
                KeyCode::Digit3 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 2, network_tx);
                }
                KeyCode::Digit4 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 3, network_tx);
                }
                KeyCode::Digit5 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 4, network_tx);
                }
                KeyCode::Digit6 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 5, network_tx);
                }
                KeyCode::Digit7 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 6, network_tx);
                }
                KeyCode::Digit8 => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::use_item(world, 7, network_tx);
                }
                
                // Z键 - 整理背包
                KeyCode::KeyZ => {
                    use crate::ecs::systems::ItemSystem;
                    ItemSystem::organize_inventory(world);
                }
                
                // N键 - 与最近的NPC对话
                KeyCode::KeyN => {
                    use crate::ecs::systems::NPCSystem;
                    if let Some(npc_id) = NPCSystem::find_nearest_npc(world) {
                        NPCSystem::click_npc(world, npc_id, network_tx);
                    } else {
                        println!("⚠️ 附近没有NPC");
                    }
                }
                
                // Tab键 - 切换目标
                KeyCode::Tab => {
                    use crate::ecs::systems::MagicCastSystem;
                    MagicCastSystem::cycle_target(world);
                }
                
                // 🎯 B键 - 切换显示纹理边框（调试用）
                KeyCode::KeyB => {
                    let mut config = world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_borders = !config.show_borders;
                    println!("🖼️ 纹理边框 (B): {}", if config.show_borders { "显示" } else { "隐藏" });
                }
                
                // 🎯 G键 - 切换显示网格（调试用）
                KeyCode::KeyG => {
                    let mut config = world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_grid = !config.show_grid;
                    println!("📐 网格 (G): {}", if config.show_grid { "显示" } else { "隐藏" });
                }
                
                // 🎯 O键 - 切换显示障碍物（调试用）
                KeyCode::KeyO => {
                    let mut config = world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_obstacles = !config.show_obstacles;
                    println!("🚧 障碍物 (O): {}", if config.show_obstacles { "显示" } else { "隐藏" });
                }
                
                // 🎯 P键 - 切换显示寻路路径（调试用）
                KeyCode::KeyP => {
                    let mut config = world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_path = !config.show_path;
                    println!("🗺️ 寻路路径 (P): {}", if config.show_path { "显示" } else { "隐藏" });
                }
                
                // 📂 对话框快捷键 (使用 DialogManager)
                KeyCode::KeyI => {
                    self.dialog_manager.toggle(crate::ecs::ui::DialogType::Inventory);
                    // 同步更新对话框组件状态
                    if let Some(mut inv_dialog) = self.get_inventory_dialog_mut(world) {
                        inv_dialog.is_open = self.dialog_manager.is_visible(crate::ecs::ui::DialogType::Inventory);
                    }
                    println!("📦 快捷键: 切换背包 (I)");
                }
                KeyCode::KeyC => {
                    self.dialog_manager.toggle(crate::ecs::ui::DialogType::Character);
                    // 同步更新对话框组件状态
                    if let Some(mut char_dialog) = self.get_character_dialog_mut(world) {
                        char_dialog.is_open = self.dialog_manager.is_visible(crate::ecs::ui::DialogType::Character);
                    }
                    println!("👤 快捷键: 切换角色 (C)");
                }
                KeyCode::KeyS => {
                    self.dialog_manager.toggle(crate::ecs::ui::DialogType::Skills);
                    println!("⚔️ 快捷键: 切换技能 (S) (待实现)");
                }
                
                _ => {}
            }
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
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 转换为 UI 设计坐标 (1024×768) 用于 UI 点击检测
        let (ui_x, ui_y) = self.window_to_ui_coords(ctx, x, y);
        
        // ==================== UI 层点击检测 (使用设计坐标) ====================
        
        // 先检查角色对话框点击
        if button == MouseButton::Left {
            if let Some(mut char_dialog) = self.get_character_dialog_mut(world) {
                if let Some(action) = char_dialog.dialog.on_mouse_down(ui_x, ui_y) {
                    match action {
                        CharacterAction::Close => {
                            println!("👤 角色对话框关闭");
                        }
                        CharacterAction::SwitchTab(tab) => {
                            println!("👤 切换到标签页: {:?}", tab);
                        }
                        CharacterAction::EquipmentClick(slot) => {
                            println!("👤 点击装备槽: {:?}", slot);
                            // TODO: 处理装备操作
                        }
                    }
                    return Ok(());
                }
            }
        }
        
        // 再检查背包对话框点击
        if button == MouseButton::Left {
            if let Some(mut inv_dialog) = self.get_inventory_dialog_mut(world) {
                if let Some(action) = inv_dialog.dialog.on_mouse_down(ui_x, ui_y) {
                    match action {
                        InventoryAction::Close => {
                            println!("🎒 背包关闭");
                        }
                        InventoryAction::SelectSlot(slot) => {
                            println!("🎒 选中背包格子: {}", slot);
                            // TODO: 处理物品选择
                        }
                        _ => {}
                    }
                    return Ok(());
                }
            }
        }
        
        // 再检查主对话框按钮点击
        if button == MouseButton::Left {
            let clicked_button = {
                if let Some(mut main_dialog) = self.get_main_dialog_mut(world) {
                    main_dialog.dialog.on_mouse_down(ui_x, ui_y)
                } else {
                    None
                }
            };
            
            if let Some(clicked_button) = clicked_button {
                println!("🖱️ 点击主对话框按钮: {:?}", clicked_button);
                
                // 使用 DialogManager 统一管理对话框显示/隐藏
                match clicked_button {
                    MainDialogButton::Inventory => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::Inventory);
                        // 同步更新对话框组件状态
                        if let Some(mut inv_dialog) = self.get_inventory_dialog_mut(world) {
                            inv_dialog.is_open = self.dialog_manager.is_visible(crate::ecs::ui::DialogType::Inventory);
                        }
                        println!("📦 切换背包对话框");
                    }
                    MainDialogButton::Character => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::Character);
                        // 同步更新对话框组件状态
                        if let Some(mut char_dialog) = self.get_character_dialog_mut(world) {
                            char_dialog.is_open = self.dialog_manager.is_visible(crate::ecs::ui::DialogType::Character);
                        }
                        println!("👤 切换角色对话框");
                    }
                    MainDialogButton::Skills => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::Skills);
                        println!("⚔️ 切换技能对话框 (待实现)");
                    }
                    MainDialogButton::Quest => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::Quest);
                        println!("📜 切换任务对话框 (待实现)");
                    }
                    MainDialogButton::Options => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::Options);
                        println!("⚙️ 切换选项对话框 (待实现)");
                    }
                    MainDialogButton::Menu => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::Menu);
                        println!("📋 切换菜单对话框 (待实现)");
                    }
                    MainDialogButton::GameShop => {
                        self.dialog_manager.toggle(crate::ecs::ui::DialogType::GameShop);
                        println!("🛒 切换商城对话框 (待实现)");
                    }
                }
                return Ok(());
            }
        }
        
        // 更新鼠标状态（支持左键和右键）
        // ggez 提供的 x,y 是窗口逻辑坐标，与 camera.screen_width/height 相同
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
            
            match button {
                MouseButton::Left => {
                    mouse_input.left_pressed = true;
                    mouse_input.left_press_time = 0;  // 🎯 重置按下时间，用于长按检测
                }
                MouseButton::Right => {
                    mouse_input.right_pressed = true;
                    mouse_input.right_press_time = 0;  // 🎯 重置按下时间，用于长按检测
                }
                _ => {}
            }
        }
        
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
        // 更新鼠标状态并检测双击
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            // 🎯 更新鼠标位置（防止快速点击时位置不准确）
            mouse_input.x = x;
            mouse_input.y = y;
            
            match button {
                MouseButton::Left => {
                    // 🎯 双击检测：如果按下时间不太长(< 30帧,约500ms)且距离上次点击 < 500ms
                    if mouse_input.left_press_time < 30 {
                        let now = Instant::now();
                        let time_since_last_click = now.duration_since(mouse_input.left_last_click_time);
                        
                        if time_since_last_click < std::time::Duration::from_millis(500) {
                            // 双击！
                            mouse_input.left_double_clicked = true;
                            println!("👆👆 左键双击事件触发 at ({:.1}, {:.1})", x, y);
                            // 重置上次点击时间，防止三击被识别为两次双击
                            mouse_input.left_last_click_time = now - std::time::Duration::from_secs(10);
                        } else {
                            // 第一次点击
                            mouse_input.left_last_click_time = now;
                            mouse_input.left_double_clicked = false;
                        }
                    }
                    mouse_input.left_pressed = false;
                    mouse_input.left_press_time = 0;
                }
                MouseButton::Right => {
                    // 🎯 双击检测：右键
                    if mouse_input.right_press_time < 30 {
                        let now = Instant::now();
                        let time_since_last_click = now.duration_since(mouse_input.right_last_click_time);
                        
                        if time_since_last_click < std::time::Duration::from_millis(500) {
                            // 双击！
                            mouse_input.right_double_clicked = true;
                            println!("👆👆 右键双击事件触发 at ({:.1}, {:.1})", x, y);
                            mouse_input.right_last_click_time = now - std::time::Duration::from_secs(10);
                        } else {
                            // 第一次点击
                            mouse_input.right_last_click_time = now;
                            mouse_input.right_double_clicked = false;
                        }
                    }
                    mouse_input.right_pressed = false;
                    mouse_input.right_press_time = 0;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    fn on_mouse_move(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 转换为 UI 设计坐标 (1024×768) 用于 UI hover 检测
        let (ui_x, ui_y) = self.window_to_ui_coords(ctx, x, y);
        
        // 更新鼠标位置 (游戏逻辑使用窗口坐标,会被转换为世界坐标)
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;  // 窗口坐标,用于地图点击
            mouse_input.y = y;
        }
        
        // 更新主对话框 hover 状态 (使用 UI 坐标)
        if let Some(mut main_dialog) = self.get_main_dialog_mut(world) {
            main_dialog.dialog.update_hover(ui_x, ui_y);
        }
        
        // 更新角色对话框 hover 状态 (使用 UI 坐标)
        if let Some(mut char_dialog) = self.get_character_dialog_mut(world) {
            char_dialog.dialog.update_hover(ui_x, ui_y);
        }
        
        // 更新背包对话框 hover 状态 (使用 UI 坐标)
        if let Some(mut inv_dialog) = self.get_inventory_dialog_mut(world) {
            inv_dialog.dialog.update_hover(ui_x, ui_y);
        }
        
        Ok(())
    }
    
    fn on_mouse_wheel(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        _x: f32,
        y: f32,
    ) -> GameResult {
        // 滚轮缩放游戏画面（不影响UI）
        const ZOOM_SPEED: f32 = 0.1;
        const MIN_ZOOM: f32 = 0.5;
        const MAX_ZOOM: f32 = 2.0;
        
        if let Ok(mut camera) = world.get::<&mut Camera>(self.camera_entity) {
            // y > 0 向上滚动（放大），y < 0 向下滚动（缩小）
            let zoom_delta = y * ZOOM_SPEED;
            camera.zoom = (camera.zoom + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
            println!("🔍 缩放: {:.1}x", camera.zoom);
        }
        
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
        if let Some(mut main_dialog) = self.get_main_dialog_mut(world) {
            main_dialog.dialog.resize(width, height);
        }
        
        println!("📐 窗口调整: {}x{}", width, height);
        
        Ok(())
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
