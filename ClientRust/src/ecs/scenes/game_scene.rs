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
use ggez::input::keyboard::{KeyInput, KeyCode};
use hecs::{World, Entity};
use std::time::Instant;
use tokio::sync::mpsc;

use super::{Scene, SceneType};
use crate::network::NetworkCommand;
use crate::ecs::{
    components::{Position, Camera, Player, PlayerAction, MoveMode, Draggable, MouseInput, TimeTracker, RenderConfig, VisibleArea},
    systems::{CameraSystem, PlayerSystem, RenderSystem, AnimationSystem},
    map_helper::MapHelper,
    map_loader::MapLoader,
};
use crate::objects::MapReader;
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
        
        println!("✅ 游戏场景初始化完成！");
        
        Ok(Self {
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            ui_font_name,
        })
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
        
        // 渲染角色
        for (_entity, (player, player_pos)) in world.query::<(&Player, &Position)>().iter() {
            RenderSystem::draw_player_with_world(ctx, canvas, world, player, player_pos, &pos, &camera)?;
        }
        
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
        let hint_text = Text::new("[左键长按] 走动  [右键长按] 跑动  [Esc] 返回选择角色");
        canvas.draw(
            &hint_text,
            DrawParam::default()
                .dest([camera.screen_width - 450.0, 10.0])
                .color(Color::from_rgb(200, 200, 200)),
        );
        
        // 渲染 UI 系统
        crate::ecs::ui::UIRenderer::render(ctx, canvas, world)?;
        
        Ok(())
    }
    
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        input: KeyInput,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        use ggez::winit::keyboard::KeyCode;
        
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            ..
        } = input.event
        {
            match keycode {
                // Esc 返回选择角色
                KeyCode::Escape => {
                    return Ok(Some(SceneType::Select));
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
    ) -> GameResult {
        // 更新鼠标状态
        if let Some((_, mut mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
            
            match button {
                MouseButton::Left => {
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
        
        Ok(())
    }
    
    fn on_mouse_up(
        &mut self,
        _ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        // 更新鼠标状态
        if let Some((_, mut mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
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
        if let Some((_, mut mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = x;
            mouse_input.y = y;
        }
        
        Ok(())
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
