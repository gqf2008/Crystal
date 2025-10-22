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
    components::{Position, Camera, Player, PlayerAction, MoveMode, Draggable, MouseInput, TimeTracker, RenderConfig, VisibleArea, PlayerAppearance, Inventory, PlayerComp},
    systems::{CameraSystem, PlayerSystem, RenderSystem, AnimationSystem, NetworkSystem, MonsterSystem},
    map_helper::MapHelper,
    map_loader::MapLoader,
    ui::{MainDialog, MainDialogButton, InventoryDialog, InventoryAction, CharacterDialog, CharacterAction},
};
use crate::objects::{MapReader, PathFinder};
use crate::graphics::libraries::initialize_all_libraries;
use mir2_shared::Point;

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
    
    /// 网络同步系统
    network_system: NetworkSystem,
    
    /// UI字体名称
    ui_font_name: String,
    
    /// 主对话框
    main_dialog: MainDialog,
    
    /// 背包对话框
    inventory_dialog: InventoryDialog,
    
    /// 角色对话框
    character_dialog: CharacterDialog,
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
        ));
        
        println!("✅ 本地玩家已创建，使用默认外观和空背包");
        
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
        
        // 创建 UI 实体
        let screen = ctx.gfx.drawable_size();
        
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
        let _exp_bar_entity = world.spawn((crate::ecs::ui::ExpBar::new(screen.0, screen.1),));
        
        // 技能栏
        let _skill_bar_entity = world.spawn((crate::ecs::ui::SkillBar::default(),));
        
        // 聊天窗口
        let mut chat = crate::ecs::ui::ChatWindow::new(screen.0, screen.1);
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
        
        // 创建主对话框
        let screen = ctx.gfx.drawable_size();
        let main_dialog = MainDialog::new(screen.0, screen.1);
        
        // 创建背包对话框
        let inventory_dialog = InventoryDialog::new();
        
        println!("✅ 游戏场景初始化完成！");
        
        Ok(Self {
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            network_system: NetworkSystem::new(),
            ui_font_name,
            main_dialog,
            inventory_dialog,
            character_dialog: CharacterDialog::new(),
        })
    }
    
    /// 处理网络事件（由GameApp调用）
    pub fn handle_network_event(&mut self, world: &mut World, event: &GameEvent) {
        self.network_system.process_event(world, event);
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
    
    /// 处理双击寻路
    fn handle_double_click_pathfinding(&self, world: &mut World, screen_x: f32, screen_y: f32) {
        // 获取相机信息
        let (camera_x, camera_y, camera_width, camera_height) = {
            let pos = world.get::<&Position>(self.camera_entity).unwrap();
            let camera = world.get::<&Camera>(self.camera_entity).unwrap();
            (pos.x, pos.y, camera.screen_width, camera.screen_height)
        };
        
        // 屏幕坐标转世界坐标
        let world_x = camera_x + screen_x - camera_width / 2.0;
        let world_y = camera_y + screen_y - camera_height / 2.0;
        
        // 世界坐标转格子坐标
        let (target_grid_x, target_grid_y) = MapHelper::world_to_grid(world_x, world_y);
        
        println!("🎯 双击寻路: 屏幕({:.1}, {:.1}) -> 世界({:.1}, {:.1}) -> 格子({}, {})", 
                 screen_x, screen_y, world_x, world_y, target_grid_x, target_grid_y);
        
        // 获取地图数据
        let map_data = if let Some((_, data)) = world.query_mut::<&crate::ecs::components::MapData>().into_iter().next() {
            data.clone()
        } else {
            println!("❌ 无法获取地图数据");
            return;
        };
        
        // 检查目标是否可行走
        if !MapHelper::is_walkable(&map_data, target_grid_x, target_grid_y) {
            println!("❌ 目标位置不可行走");
            return;
        }
        
        // 获取玩家当前位置
        let player_query = world.query_mut::<(&mut Player, &Position)>();
        if let Some((_, (player, player_pos))) = player_query.into_iter().next() {
            let (start_grid_x, start_grid_y) = MapHelper::world_to_grid(player_pos.x, player_pos.y);
            
            println!("📍 玩家位置: 世界({:.1}, {:.1}) -> 格子({}, {})", 
                     player_pos.x, player_pos.y, start_grid_x, start_grid_y);
            
            // 创建寻路器
            let map_data_clone = map_data.clone();
            let pathfinder = PathFinder::new(
                map_data.width,
                map_data.height,
                Box::new(move |p: Point| !MapHelper::is_walkable(&map_data_clone, p.x, p.y))
            );
            
            // 执行寻路
            let start_point = Point { x: start_grid_x, y: start_grid_y };
            let target_point = Point { x: target_grid_x, y: target_grid_y };
            
            if let Some(path) = pathfinder.find_path(start_point, target_point) {
                println!("✅ 找到路径，长度: {}", path.len());
                // 转换 Vec<Point> 为 Vec<(i32, i32)>
                player.path = path.iter().map(|p| (p.x, p.y)).collect();
                player.path_index = 0;
                player.is_moving = true;
                player.move_mode = MoveMode::AutoPathfinding;
            } else {
                println!("❌ 无法找到路径");
            }
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
        
        // 更新相机系统
        CameraSystem::update(world);
        
        // 更新角色系统
        PlayerSystem::update(world);
        
        // 更新怪物系统
        let delta_time = 1.0 / max_fps as f32;
        MonsterSystem::update(world, delta_time);
        
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
        
        // 绘制FPS
        let time = world.get::<&TimeTracker>(self.time_entity).unwrap();
        let fps_text = Text::new(format!("FPS: {:.1}", time.fps));
        canvas.draw(
            &fps_text,
            DrawParam::default()
                .dest([10.0, 10.0])
                .color(Color::from_rgb(0, 255, 0)),
        );
        
        // 绘制操作提示（移到右上角）
        let hint_text = Text::new("[WASD/方向键] 移动  [Shift+WASD] 跑动  [鼠标] 点击移动  [Esc] 返回");
        canvas.draw(
            &hint_text,
            DrawParam::default()
                .dest([camera.screen_width - 500.0, 10.0])
                .color(Color::from_rgb(200, 200, 200)),
        );
        
        // 渲染 UI 系统
        crate::ecs::ui::UIRenderer::render(ctx, canvas, world)?;
        
        // 渲染主对话框
        self.main_dialog.draw(ctx, canvas)?;
        
        // 同步玩家背包数据到UI
        if let Some((_, inventory)) = world.query::<&Inventory>().iter().next() {
            self.inventory_dialog.set_gold(inventory.gold);
            self.inventory_dialog.set_weight(inventory.current_weight, inventory.max_weight);
        }
        
        // 同步玩家基本数据到角色对话框
        if let Some((_, (player_comp, inventory))) = world.query::<(&PlayerComp, &Inventory)>().iter().next() {
            // 基本信息
            self.character_dialog.level = player_comp.exp as u16; // TODO: 从经验计算等级
            self.character_dialog.experience = player_comp.exp;
            self.character_dialog.max_experience = 1000; // TODO: 使用真实最大经验值
            
            // 负重
            self.character_dialog.bag_weight = inventory.current_weight as i32;
            self.character_dialog.max_bag_weight = inventory.max_weight as i32;
            
            // TODO: HP/MP/装备等数据需要从服务器同步
            // 目前这些字段使用默认值或需要添加额外组件
        }
        
        // 渲染背包对话框
        self.inventory_dialog.draw(ctx, canvas)?;
        
        // 渲染角色对话框
        self.character_dialog.draw(ctx, canvas)?;
        
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
        use mir2_shared::enums::MirDirection;
        
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            ..
        } = input.event
        {
            // 检查是否按下 Shift 键（跑步）
            let running = input.mods.shift_key();
            
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
                
                // Ctrl + 方向键 = 攻击
                KeyCode::KeyW | KeyCode::ArrowUp if input.mods.control_key() => {
                    let _ = network_tx.send(NetworkCommand::Attack { 
                        direction: MirDirection::Up,
                        spell: mir2_shared::enums::Spell::None,
                    });
                }
                KeyCode::KeyS | KeyCode::ArrowDown if input.mods.control_key() => {
                    let _ = network_tx.send(NetworkCommand::Attack { 
                        direction: MirDirection::Down,
                        spell: mir2_shared::enums::Spell::None,
                    });
                }
                KeyCode::KeyA | KeyCode::ArrowLeft if input.mods.control_key() => {
                    let _ = network_tx.send(NetworkCommand::Attack { 
                        direction: MirDirection::Left,
                        spell: mir2_shared::enums::Spell::None,
                    });
                }
                KeyCode::KeyD | KeyCode::ArrowRight if input.mods.control_key() => {
                    let _ = network_tx.send(NetworkCommand::Attack { 
                        direction: MirDirection::Right,
                        spell: mir2_shared::enums::Spell::None,
                    });
                }
                
                // W 或上方向键 - 向上移动
                KeyCode::KeyW | KeyCode::ArrowUp => {
                    if running {
                        let _ = network_tx.send(NetworkCommand::Run { direction: MirDirection::Up });
                    } else {
                        let _ = network_tx.send(NetworkCommand::Walk { direction: MirDirection::Up });
                    }
                }
                // S 或下方向键 - 向下移动
                KeyCode::KeyS | KeyCode::ArrowDown => {
                    if running {
                        let _ = network_tx.send(NetworkCommand::Run { direction: MirDirection::Down });
                    } else {
                        let _ = network_tx.send(NetworkCommand::Walk { direction: MirDirection::Down });
                    }
                }
                // A 或左方向键 - 向左移动
                KeyCode::KeyA | KeyCode::ArrowLeft => {
                    if running {
                        let _ = network_tx.send(NetworkCommand::Run { direction: MirDirection::Left });
                    } else {
                        let _ = network_tx.send(NetworkCommand::Walk { direction: MirDirection::Left });
                    }
                }
                // D 或右方向键 - 向右移动
                KeyCode::KeyD | KeyCode::ArrowRight => {
                    if running {
                        let _ = network_tx.send(NetworkCommand::Run { direction: MirDirection::Right });
                    } else {
                        let _ = network_tx.send(NetworkCommand::Walk { direction: MirDirection::Right });
                    }
                }
                _ => {}
            }
        }
        
        Ok(None)
    }
    
    fn on_mouse_down(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 先检查角色对话框点击
        if button == MouseButton::Left {
            if let Some(action) = self.character_dialog.on_mouse_down(x, y) {
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
        
        // 再检查背包对话框点击
        if button == MouseButton::Left {
            if let Some(action) = self.inventory_dialog.on_mouse_down(x, y) {
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
        
        // 再检查主对话框按钮点击
        if button == MouseButton::Left {
            if let Some(clicked_button) = self.main_dialog.on_mouse_down(x, y) {
                println!("🖱️ 点击主对话框按钮: {:?}", clicked_button);
                match clicked_button {
                    MainDialogButton::Inventory => {
                        println!("📦 打开背包");
                        self.inventory_dialog.toggle();
                    }
                    MainDialogButton::Character => {
                        println!("👤 打开角色界面");
                        self.character_dialog.toggle();
                    }
                    MainDialogButton::Skills => {
                        println!("⚔️ 打开技能界面");
                        // TODO: 显示技能界面
                    }
                    MainDialogButton::Quest => {
                        println!("📜 打开任务界面");
                        // TODO: 显示任务界面
                    }
                    MainDialogButton::Options => {
                        println!("⚙️ 打开选项界面");
                        // TODO: 显示选项界面
                    }
                    MainDialogButton::Menu => {
                        println!("📋 打开菜单");
                        // TODO: 显示菜单
                    }
                    MainDialogButton::GameShop => {
                        println!("🛒 打开商城");
                        // TODO: 显示商城界面
                    }
                }
                return Ok(());
            }
        }
        
        // 更新鼠标状态并检测双击
        let mut double_click_detected = false;
        let mut double_click_x = 0.0;
        let mut double_click_y = 0.0;
        
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
            
            match button {
                MouseButton::Left => {
                    // 检测双击（300ms内的两次点击）
                    let now = Instant::now();
                    let time_since_last_click = now.duration_since(mouse_input.left_last_click_time);
                    
                    if time_since_last_click.as_millis() < 300 {
                        // 双击检测成功
                        double_click_detected = true;
                        double_click_x = x;
                        double_click_y = y;
                        mouse_input.left_double_clicked = true;
                        println!("🖱️ 检测到双击: ({:.1}, {:.1})", x, y);
                    } else {
                        mouse_input.left_double_clicked = false;
                    }
                    
                    mouse_input.left_last_click_time = now;
                    mouse_input.left_pressed = true;
                    mouse_input.left_press_time = 0;
                }
                MouseButton::Right => {
                    mouse_input.right_pressed = true;
                    mouse_input.right_press_time = 0;
                }
                _ => {}
            }
        }
        
        // 处理双击寻路
        if double_click_detected {
            self.handle_double_click_pathfinding(world, double_click_x, double_click_y);
        }
        
        Ok(())
    }
    
    fn on_mouse_up(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        _x: f32,
        _y: f32,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 更新鼠标状态
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            match button {
                MouseButton::Left => {
                    mouse_input.left_pressed = false;
                    mouse_input.left_press_time = 0;
                }
                MouseButton::Right => {
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
        _ctx: &mut Context,
        world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 更新鼠标位置
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
        }
        
        // 更新主对话框hover状态
        self.main_dialog.update_hover(x, y);
        
        // 更新角色对话框hover状态
        self.character_dialog.update_hover(x, y);
        
        // 更新背包对话框hover状态
        self.inventory_dialog.update_hover(x, y);
        
        Ok(())
    }
    
    fn on_resize(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        width: f32,
        height: f32,
    ) -> GameResult {
        // 更新相机尺寸
        if let Ok(mut camera) = world.get::<&mut Camera>(self.camera_entity) {
            camera.screen_width = width;
            camera.screen_height = height;
        }
        
        // 更新主对话框尺寸
        self.main_dialog.resize(width, height);
        
        println!("📐 窗口调整: {}x{}", width, height);
        
        Ok(())
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
