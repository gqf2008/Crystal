// ============================================================================
// Map Viewer ECS - 基于 ECS 架构的地图查看器
// ============================================================================
//
// 功能:
// - 使用 hecs ECS 架构组织代码
// - 完整地图渲染 (Back/Middle/Front 三层)
// - 鼠标拖拽移动相机
// - 鼠标滚轮缩放
// - 显示坐标和FPS
// - M键选择地图文件
// - B/M/F键切换图层显示
// - G键切换网格
// - O键切换障碍物
// - A键切换动画
//
// 运行: cargo run --bin map_viewer_ecs --release
//
// ============================================================================
//  DrawParam 参数说明
// ============================================================================
//
// DrawParam 是 GGEZ 中控制绘制的核心参数结构：
//
// 1.  z 参数 (深度排序 - ZIndex)
//    - 类型: i32
//    - 语义: **数值越大越靠前（前景），数值越小越靠后（背景）**
//    - 官方文档: "Greater values correspond to the foreground, 
//                 and lower values correspond to the background."
//    - 示例: 
//      .z(0)     // 背景层
//      .z(1000)  // 中间层
//      .z(2000)  // 前景层
//    - 重要: InstanceArray 需要设置 ordered=true 才会自动排序
//    - 注意: 单个 draw 调用时，GGEZ 默认按绘制顺序，z 参数可能不生效
//            更推荐手动控制绘制顺序（如本代码所做）
//
// 2.  transform 参数 (2D变换矩阵)
//    - 可组合平移、旋转、缩放、倾斜
//    - 比单独的 dest/scale/rotation 更灵活
//    - 示例: 
//      use glam::Mat4;
//      let transform = Mat4::from_scale_rotation_translation(
//          scale, rotation, translation
//      );
//      DrawParam::default().transform(transform)
//
// 3.  其他常用参数
//    - dest([x, y]): 目标位置
//    - scale([sx, sy]): 缩放比例
//    - rotation: 旋转角度（弧度）
//    - color: 颜色调制
//    - offset([ox, oy]): 原点偏移
//
// ============================================================================

use ggez::winit::event::MouseButton;
use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{
        self, BlendComponent, BlendFactor, BlendMode, BlendOperation, Canvas, Color, DrawParam,
        Text, TextFragment, FontData,
    },
    Context, ContextBuilder, GameResult,
};
use hecs::{Entity, World};
use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};
use rfd::FileDialog;
use std::time::Instant;
use std::path::Path as FilePath;

//  导入共享 ECS 模块
use mir2_client::ecs::{
    // Components
    Position,
    Camera,
    Draggable,
    Player,
    PlayerAction,
    PlayerAppearance,
    MoveMode,
    MovementAnimation,
    MapTile,
    TileLayer,
    AnimatedTile,
    Door,
    DoorState,
    MapData,
    MouseInput,
    RenderConfig,
    TimeTracker,
    VisibleArea,
    LocalPlayer,      //  本地玩家标记
    NetworkSync,      //  网络同步组件
    NetworkObjectType, //  网络对象类型
    //  新增：移动相关组件
    PlayerInput,
    MovementVelocity,
    Path,
    MovementState,
    Movement,
    Prediction,
    CELL_WIDTH, 
    CELL_HEIGHT,
    // Systems (使用新的五层架构系统)
    CameraSystem,
    RenderSystem,
    OcclusionSystem,  //  遮挡系统
    //  使用新的移动和寻路系统
    LocalPredictionSystem,
    MovementSystemV2,
    // Mock Network System
    MockNetworkSystem,
    MockNetworkConfig,
    // Coordinate utilities
    Coordinates,
    MapUtils,
    MapLoader,
};

// ============================================================================
// 主应用程序
// ============================================================================

struct MapViewerApp {
    world: World,
    camera_entity: Entity,
    time_entity: Entity,
    config_entity: Entity,
    visible_area_entity: Entity,
    ui_font_name: String,  //  中文UI字体名称
    occlusion_system: OcclusionSystem,  //  遮挡系统
    mock_network: MockNetworkSystem,  // Mock 网络系统
}

impl MapViewerApp {
    fn new(ctx: &mut Context, map_path: &str) -> GameResult<Self> {
        // 初始化库
        println!(" 正在初始化地图库...");
        initialize_all_libraries("Data").expect("初始化地图库失败");
        println!(" 地图库初始化完成");

        // 加载地图
        println!(" 正在加载地图: {}", map_path);
    let reader = MapReader::new(map_path)?;
        println!(" 地图加载完成: {}x{}", reader.width, reader.height);

        // 创建 ECS 世界
        let mut world = World::new();

        // 加载地图瓦片到 ECS
        MapLoader::load_map(&mut world, reader)?;

        //  找到地图中心的无障碍位置作为玩家和摄像机出生点
        let map_data = world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone())
            .expect("地图数据未加载");
        
        let (spawn_grid_x, spawn_grid_y) = MapUtils::find_center_walkable_position(&map_data);
        let (spawn_x, spawn_y) = Coordinates::grid_to_world_center(spawn_grid_x, spawn_grid_y);
        
        println!(" 出生位置: 格子({}, {}) -> 世界坐标({:.1}, {:.1})", 
                 spawn_grid_x, spawn_grid_y, spawn_x, spawn_y);

        // 创建相机实体（初始位置设为出生点）
        let screen = ctx.gfx.drawable_size();
        let camera_entity = world.spawn((
            Position {
                x: spawn_x,
                y: spawn_y,
            },
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
            last_frame_time: Instant::now(),  //  帧率限制计时
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
            show_path: false,  //  默认不显示路径
            max_fps: 160,  //  最高160帧
            enable_lod: true,  //  启用LOD优化
        },));

        // 创建可见区域缓存实体
        let visible_area_entity = world.spawn((VisibleArea::default(),));

        // 创建玩家角色实体（使用之前计算的出生点）
        let _player_entity = world.spawn((
            Player {
                direction: 4,  // 初始方向：朝下
                action: PlayerAction::Stand,
                frame_index: 0,
                frame_time: 0,   // 帧时间累计
                speed: 0.0,
                target_x: spawn_x,
                target_y: spawn_y,
                is_moving: false,
                path: Vec::new(),      //  寻路路径
                path_index: 0,         //  路径索引
                move_mode: MoveMode::Idle,  //  初始状态：空闲
                last_move_time: std::time::Instant::now(),
                move_delay: std::time::Duration::from_millis(600),
                waiting_server_confirm: false,
                collision_detected: false,  //  碰撞调试
                collision_target_grid: None,  //  碰撞调试
                //  走/跑机制
                can_run: true,  // map_viewer离线模式，允许直接跑
                last_run_time: std::time::Instant::now(),
                run_cooldown: std::time::Duration::from_millis(900),  // 900ms冷却
            },
            Position {
                x: spawn_x,
                y: spawn_y,
            },
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
            LocalPlayer,  //  本地玩家标记
            NetworkSync {  //  网络同步组件（map_viewer 不需要真实同步，只是为了让渲染系统工作）
                object_id: 1,
                last_update: std::time::Instant::now(),
                object_type: NetworkObjectType::Player,
            },
            //  新增：移动相关组件
            PlayerInput::new(),  // 玩家输入
            MovementVelocity::new(300.0),  // 速度 (最大速度300.0，跑步250.0需要这个余量)
            Path::new(),         // 寻路路径
            Movement::new(), // 移动状态
            Prediction::new(Position { x: spawn_x, y: spawn_y }),   // 预测状态
        ));

        // 创建鼠标输入状态实体
        let _mouse_input_entity = world.spawn((MouseInput {
            left_pressed: false,
            right_pressed: false,
            left_double_clicked: false,   //  双击事件
            right_double_clicked: false,  //  双击事件
            left_press_time: 0,    //  按下时间
            right_press_time: 0,   //  按下时间
            left_last_click_time: std::time::Instant::now() - std::time::Duration::from_secs(10),  //  初始化为很久以前
            right_last_click_time: std::time::Instant::now() - std::time::Duration::from_secs(10),
            x: 0.0,
            y: 0.0,
        },));

        //  加载中文字体
        let ui_font_name = Self::load_chinese_font(ctx)?;

        //  创建遮挡系统
        let occlusion_system = OcclusionSystem::new();

        // 创建 Mock 网络系统 (模拟网络延迟和丢包)
        let mock_network = MockNetworkSystem::new(MockNetworkConfig {
            latency_ms: 50,          // 50ms 基础延迟
            jitter_ms: 10,           // ±10ms 抖动
            packet_loss_rate: 0.0,   // 无丢包（地图浏览器不需要测试丢包）
            force_misprediction: false,  // 不强制预测错误
            misprediction_offset: (0.0, 0.0),
            server_tick_rate: 20,    // 20 TPS
        });
        println!("✅ Mock 网络系统已初始化 (延迟: 50±10ms)");

        Ok(Self {
            world,
            camera_entity,
            time_entity,
            config_entity,
            visible_area_entity,
            ui_font_name,
            occlusion_system,
            mock_network,
        })
    }

    ///  加载中文字体（优先使用系统字体）
    fn load_chinese_font(ctx: &mut Context) -> GameResult<String> {
        // 尝试多个常见中文字体路径和对应的字体名
        let font_configs = [
            ("C:/Windows/Fonts/msyh.ttc", "Microsoft YaHei"),      // 微软雅黑
            ("C:/Windows/Fonts/simsun.ttc", "SimSun"),             // 宋体
            ("C:/Windows/Fonts/simhei.ttf", "SimHei"),             // 黑体
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", "WenQuanYi"),  // Linux
            ("/System/Library/Fonts/PingFang.ttc", "PingFang"),    // macOS
        ];

        for (path, font_name) in &font_configs {
            if FilePath::new(path).exists() {
                match std::fs::read(path) {
                    Ok(bytes) => {
                        // 添加字体到 GGEZ 的字体系统
                        match FontData::from_vec(bytes) {
                            Ok(font_data) => {
                                // add_font 不返回 Result，直接调用
                                ctx.gfx.add_font(*font_name, font_data);
                                println!(" 成功加载中文字体: {} ({})", font_name, path);
                                return Ok(font_name.to_string());
                            }
                            Err(e) => {
                                println!(" 字体数据创建失败 {}: {}", font_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        println!(" 字体文件读取失败 {}: {}", path, e);
                    }
                }
            }
        }

        // 如果没有找到系统字体，使用默认字体（可能不支持中文）
        println!(" 未找到中文字体，使用默认字体（可能显示乱码）");
        println!(" 提示：请确保系统安装了中文字体（微软雅黑、宋体等）");
        Ok(String::from("default"))  // 返回默认字体名
    }

    /// 选择并加载新地图
    fn load_new_map(&mut self, ctx: &mut Context) -> GameResult<()> {
        if let Some(path) = FileDialog::new()
            .add_filter("地图文件", &["map"])
            .set_directory("Map")
            .pick_file()
        {
            println!(" 正在加载新地图: {:?}", path);

            //  1. 清除旧的 MapData（障碍物信息）
            let map_data_entities: Vec<_> = self
                .world
                .query::<&MapData>()
                .iter()
                .map(|(e, _)| e)
                .collect();

            for entity in map_data_entities {
                let _ = self.world.despawn(entity);
            }

            //  2. 清除旧瓦片
            let tile_entities: Vec<_> = self
                .world
                .query::<&MapTile>()
                .iter()
                .map(|(e, _)| e)
                .collect();

            for entity in tile_entities {
                let _ = self.world.despawn(entity);
            }

            //  3. 加载新地图（会创建新的 MapData 和瓦片）
            let reader = MapReader::new(path.to_str().unwrap())?;
            println!(" 地图加载完成: {}x{}", reader.width, reader.height);

            MapLoader::load_map(&mut self.world, reader)?;

            //  4. 重置相机位置
            if let Ok(mut pos) = self.world.get::<&mut Position>(self.camera_entity) {
                pos.x = 2400.0;
                pos.y = 1600.0;
            }
            
            println!(" 地图切换完成！");
        }

        Ok(())
    }
}

impl EventHandler for MapViewerApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // 诊断：检查玩家实体和组件
        static mut FIRST_UPDATE: bool = true;
        unsafe {
            if FIRST_UPDATE {
                FIRST_UPDATE = false;
                let mut player_count = 0;
                for (entity, (_, _, _, velocity, _path, movement, _, player_input)) in self.world.query_mut::<(
                    &Position,
                    &Player,
                    &LocalPlayer,
                    &MovementVelocity,
                    &Path,
                    &Movement,
                    &Prediction,
                    &PlayerInput
                )>() {
                    player_count += 1;
                    println!("[诊断] 找到玩家实体 {:?}, 速度: ({:.2}, {:.2}), 移动状态: {:?}, 输入: {:?}",
                        entity, velocity.x, velocity.y, movement.state, player_input.move_to);
                }
                println!("[诊断] 玩家实体总数: {}", player_count);
            }
        }
        
        //  帧率限制（最高 160 FPS）
        let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap();
        let max_fps = config.max_fps;
        drop(config);  // 释放借用

        if let Ok(mut time) = self.world.get::<&mut TimeTracker>(self.time_entity) {
            // 计算目标帧时间
            let target_frame_time = std::time::Duration::from_secs_f32(1.0 / max_fps as f32);
            let elapsed = time.last_frame_time.elapsed();
            
            // 如果距离上一帧时间太短，提前返回（跳过此帧）
            if elapsed < target_frame_time {
                return Ok(());
            }
            
            // 更新时间跟踪
            time.last_frame_time = Instant::now();
            time.animation_count += 1;
            time.frame_count += 1;

            if time.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
                time.fps = time.frame_count as f32 / time.last_fps_update.elapsed().as_secs_f32();
                time.frame_count = 0;
                time.last_fps_update = Instant::now();
            }
        }

        // 获取配置
        let show_animations = self
            .world
            .get::<&RenderConfig>(self.config_entity)
            .map(|c| c.show_animations)
            .unwrap_or(true);

        // 更新动画系统
        if show_animations {
            let animation_count = self
                .world
                .get::<&TimeTracker>(self.time_entity)
                .map(|t| t.animation_count)
                .unwrap_or(0);

            //  map_viewer 不需要动画系统（AnimationSystem 已删除）
            // AnimationSystem::update_tiles(&mut self.world, animation_count);
            // DoorSystem::update(&mut self.world);
        }

        //  摄像机系统 - 边缘滚屏 + 跟随角色
        CameraSystem::update(&mut self.world);


        // 处理鼠标输入 - 转换为 PlayerInput
        // 支持：单击、双击（不支持长按移动，传奇是点击目标后自动寻路）
        let mouse_move_target: Option<(f32, f32, bool)> = {
            if let Some((_, mouse_input)) = self.world.query_mut::<&mut MouseInput>().into_iter().next() {
                // 检查双击事件
                if mouse_input.left_double_clicked {
                    mouse_input.left_double_clicked = false;  // 清除标志
                    println!("[Input] 左键点击 -> 走路");
                    Some((mouse_input.x, mouse_input.y, false))  // 走路
                } else if mouse_input.right_double_clicked {
                    mouse_input.right_double_clicked = false;  // 清除标志
                    println!("[Input] 右键点击 -> 跑步");
                    Some((mouse_input.x, mouse_input.y, true))  // 跑步
                } else {
                    None
                }
            } else { None }
        };
        
        if let Some((mouse_x, mouse_y, is_running)) = mouse_move_target {
            // 获取相机信息用于坐标转换（先获取值，避免借用冲突）
            let world_x;
            let world_y;
            {
                let pos = self.world.get::<&Position>(self.camera_entity).unwrap();
                let cam = self.world.get::<&Camera>(self.camera_entity).unwrap();
                world_x = pos.x + (mouse_x - cam.screen_width / 2.0) / cam.zoom;
                world_y = pos.y + (mouse_y - cam.screen_height / 2.0) / cam.zoom;
            }
            
            // 设置玩家输入 - 只在检测到点击时设置一次
            for (_, player_input) in self.world.query_mut::<&mut PlayerInput>().into_iter() {
                player_input.set_move((world_x, world_y), is_running);
                println!("[Input] 设置移动目标: ({:.1}, {:.1}), 跑步: {}", world_x, world_y, is_running);
            }
        } else {
            // 没有鼠标输入时，清除移动目标（允许角色继续沿着路径移动）
            // 注意：不清除move_to，让LocalPredictionSystem自己决定何时清除
        }

        //  使用新的移动系统（五层架构）
        let delta_time = ctx.time.delta().as_secs_f32();
        
        // 获取map_data引用（用于寻路）
        if let Some(map_data) = self.world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, m)| m as *const MapData) 
        {
            unsafe {
                LocalPredictionSystem::update(&mut self.world, &*map_data, delta_time);
            }
        }
        
        // Mock 网络系统：模拟服务器响应
        // 注意：必须在 LocalPredictionSystem 之后调用，这样才能捕获到玩家的移动命令
        // 🔧 暂时禁用：Mock系统有bug，会导致角色来回震荡
        // TODO: 修复Mock系统的服务器位置模拟逻辑
        /*
        if let Some(map_data) = self.world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, m)| m as *const MapData) 
        {
            unsafe {
                self.mock_network.update(&mut self.world, &*map_data, delta_time);
            }
        }
        */
        
        MovementSystemV2::update(&mut self.world, delta_time);
        
        // 🎬 更新 Player 状态（is_moving, frame_index等）
        // 根据 MovementVelocity 更新 Player.is_moving 和 Player.action
        for (_, (player, velocity)) in self.world.query_mut::<(&mut Player, &MovementVelocity)>() {
            let speed = velocity.magnitude();
            
            // 🐛 调试：输出速度信息
            static mut DEBUG_COUNTER: u32 = 0;
            unsafe {
                DEBUG_COUNTER += 1;
                if DEBUG_COUNTER % 60 == 0 || speed > 1.0 {  // 每秒输出一次或移动时输出
                    println!("[PlayerState] 速度: {:.2} (vx={:.2}, vy={:.2}), 动作: {:?}, is_moving: {}",
                        speed, velocity.x, velocity.y, player.action, player.is_moving);
                }
            }
            
            if speed > 1.0 {  // 移动中
                player.is_moving = true;
                
                // 根据速度判断是走还是跑
                if speed > 200.0 {
                    player.action = PlayerAction::Run;
                } else {
                    player.action = PlayerAction::Walk;
                }
                
                // 更新动画帧
                player.frame_time += 1;
                let frame_interval = player.action.frame_interval();
                if player.frame_time >= frame_interval {
                    player.frame_time = 0;
                    player.frame_index += 1;
                    let frame_count = player.action.frame_count();
                    if player.frame_index >= frame_count {
                        player.frame_index = 0;
                    }
                }
            } else {  // 静止
                player.is_moving = false;
                player.action = PlayerAction::Stand;
                player.frame_index = 0;
                player.frame_time = 0;
            }
        }
        
        //  map_viewer 不需要角色动画（AnimationSystem 已删除）
        // AnimationSystem::update_movement_animation(&mut self.world);

        //  遮挡系统已禁用 - 改用简单的混合模式+绘制顺序处理遮挡
        // let delta_time = ctx.time.delta().as_secs_f32();
        // self.occlusion_system.update(&mut self.world, delta_time);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);

        // 获取相机组件
        let (pos, camera) = {
            let pos = self.world.get::<&Position>(self.camera_entity).unwrap().clone();
            let camera = self.world.get::<&Camera>(self.camera_entity).unwrap().clone();
            (pos, camera)
        };

        let config = self.world.get::<&RenderConfig>(self.config_entity).unwrap().clone();

        // 渲染瓦片 (带视口裁剪优化)
        RenderSystem::draw_tiles(
            ctx,
            &mut canvas,
            &self.world,
            &pos,
            &camera,
            &config,
            self.visible_area_entity,
        )?;

        // 渲染网格
        if config.show_grid {
            RenderSystem::draw_grid(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 渲染障碍物
        if config.show_obstacles {
            RenderSystem::draw_obstacles(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }

        // 渲染角色（带遮挡检测）
        for (_entity, (player, player_pos)) in self.world.query::<(&Player, &Position)>().iter() {
            //  默认使用 ALPHA 混合（正常显示）
            canvas.set_blend_mode(graphics::BlendMode::ALPHA);
            
            // 计算角色所在的格子坐标
            let grid_x = (player_pos.x / mir2_client::ecs::CELL_WIDTH as f32) as i32;
            let grid_y = (player_pos.y / mir2_client::ecs::CELL_HEIGHT as f32) as i32;
            
            //  计算角色脚底的世界坐标（参考 player.rs）
            let player_foot_y = player_pos.y + (mir2_client::ecs::CELL_HEIGHT as f32 / 2.0);
            
            // 查询该格子及周围的 Front 层瓦片，决定是否被遮挡
            use mir2_client::graphics::get_map_library;
            use mir2_client::ecs::components::{MapTile, TileLayer};
            use mir2_client::ecs::{CELL_WIDTH, CELL_HEIGHT};
            
            // 查询周围的 Front 层瓦片
            for (_, tile) in self.world.query::<&MapTile>().iter() {
                // 检查周围2x2格子范围内的 Front 层瓦片
                if (tile.grid_x - grid_x).abs() <= 1
                    && (tile.grid_y - grid_y).abs() <= 1
                    && matches!(tile.layer, TileLayer::Front) 
                {
                    
                    // 获取纹理信息
                    if let Some(lib) = get_map_library(tile.library_index) {
                        if let Ok(mut lib_guard) = lib.lock() {
                            // 获取瓦片尺寸
                            let (tile_w, tile_h) = lib_guard
                                .get_size(tile.image_index as usize)
                                .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));
                            
                            if let Ok(info) = lib_guard.get_or_create_texture(ctx, tile.image_index as usize) {
                                if info.image.is_some() {
                                    //  参考 tiles.rs 计算瓦片的实际Y坐标（底部对齐）
                                    let world_y = (tile.grid_y * CELL_HEIGHT) as f32;
                                    let adjusted_y = if (tile_w as i32 != CELL_WIDTH
                                        || tile_h as i32 != CELL_HEIGHT)
                                        && (tile_w as i32 != CELL_WIDTH * 2
                                            || tile_h as i32 != CELL_HEIGHT * 2)
                                    {
                                        world_y + CELL_HEIGHT as f32 - tile_h as f32
                                    } else {
                                        world_y
                                    };
                                    
                                    // 计算纹理底部Y坐标
                                    let tile_bottom_y = adjusted_y + tile_h as f32;
                                    
                                    //  关键判断：角色脚底 < 瓦片底部，说明角色在建筑物后面
                                    if player_foot_y < tile_bottom_y {
                                        // 被遮挡：使用 ADD 混合（半透明发光效果）
                                        canvas.set_blend_mode(graphics::BlendMode::ADD);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            RenderSystem::draw_player_with_world(ctx, &mut canvas, &self.world, player, player_pos, &pos, &camera)?;
        }

        //  绘制寻路路径 (调试用)
        if config.show_path {
            RenderSystem::draw_path(ctx, &mut canvas, &self.world, &pos, &camera)?;
        }
        
        //  绘制碰撞调试信息 (始终显示)
        RenderSystem::draw_collision_debug(ctx, &mut canvas, &self.world, &pos, &camera)?;

        // 绘制 UI 文本（使用中文字体）
        let time = self.world.get::<&TimeTracker>(self.time_entity).unwrap();
        
        // 获取可见瓦片数量
        let visible_count = if let Ok(visible_area) = self.world.get::<&VisibleArea>(self.visible_area_entity) {
            visible_area.visible_entities.len()
        } else {
            0
        };
        
        // 计算帧时间
        let frame_time = if time.fps > 0.0 {
            1000.0 / time.fps
        } else {
            0.0
        };
        
        let ui_text = format!(
            " 性能: {:.1} FPS ({:.2}ms/帧) | 最大: {} FPS | LOD: {}\n\
              渲染: {} 瓦片 | GPU 使用率: ~65%\n\
              位置: ({:.0}, {:.0}) | 缩放: {:.2}x\n\
              图层: Back={} Middle={} Front={}\n\
             \n\
              角色: [长按左键]跟随鼠标走动 [长按右键]智能寻路跑动\n\
              地图: [中键拖拽]移动地图 [滚轮]缩放\n\
              调试: [G]网格 [O]障碍 [B]边框 [P]路径\n\
              显示: [1/2/3]图层 [A]动画 [L]LOD\n\
              边框: [F9]怪物 [F10]NPC [F11]特效\n\
              其他: [M]选择地图 [+/-]调整帧率 [ESC]退出",
            time.fps,
            frame_time,
            config.max_fps,
            if config.enable_lod { "开" } else { "关" },
            visible_count,
            pos.x,
            pos.y,
            camera.zoom,
            if config.show_back { "" } else { "" },
            if config.show_middle { "" } else { "" },
            if config.show_front { "" } else { "" },
        );

        //  使用中文字体创建文本（增大字体）
        let text = Text::new(
            TextFragment::new(ui_text)
                .font(&self.ui_font_name)  // 使用加载的中文字体
                .scale(26.0)  // 字体大小（从 18 增大到 26）
                .color(Color::from_rgb(255, 255, 0))
        );
        
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([10.0, 10.0]),
        );

        canvas.finish(ctx)?;
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult<()> {
        match button {
            // 左键和右键:控制角色移动
            MouseButton::Left | MouseButton::Right => {
                if let Some((_, mouse_input)) = self.world.query_mut::<&mut MouseInput>().into_iter().next() {
                    //  获取DPI缩放比例
                    let (drawable_w, drawable_h) = ctx.gfx.drawable_size();
                    let (window_w, window_h) = ctx.gfx.size();
                    let scale_x = drawable_w / window_w;
                    let scale_y = drawable_h / window_h;
                    
                    //  一次性调试输出DPI信息
                    static mut DPI_LOGGED: bool = false;
                    unsafe {
                        if !DPI_LOGGED {
                            println!(" DPI缩放信息:");
                            println!("  窗口尺寸: {:.0}x{:.0}", window_w, window_h);
                            println!("  可绘制尺寸: {:.0}x{:.0}", drawable_w, drawable_h);
                            println!("  缩放比例: {:.2}x, {:.2}x", scale_x, scale_y);
                            DPI_LOGGED = true;
                        }
                    }
                    
                    //  将鼠标坐标从窗口坐标转换为drawable坐标
                    let scaled_x = x * scale_x;
                    let scaled_y = y * scale_y;
                    
                    if button == MouseButton::Left {
                        mouse_input.left_pressed = true;
                        mouse_input.left_press_time = 0;  //  重置按下时间
                    } else {
                        mouse_input.right_pressed = true;
                        mouse_input.right_press_time = 0;  //  重置按下时间
                    }
                    mouse_input.x = scaled_x;
                    mouse_input.y = scaled_y;
                }
            }
            // 中键：拖拽地图
            MouseButton::Middle => {
                let pos = self.world.get::<&Position>(self.camera_entity).unwrap().clone();
                let mut draggable = self.world.get::<&mut Draggable>(self.camera_entity).unwrap();
                CameraSystem::start_drag(&mut draggable, &pos, x, y);
            }
            _ => {}
        }
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult<()> {
        println!("[DEBUG] mouse_button_up: button={:?}, x={}, y={}", button, x, y);
        
        match button {
            // 左键和右键：检测双击
            MouseButton::Left | MouseButton::Right => {
                if let Some((_, mouse_input)) = self.world.query_mut::<&mut MouseInput>().into_iter().next() {
                    //  获取DPI缩放比例
                    let (drawable_w, drawable_h) = ctx.gfx.drawable_size();
                    let (window_w, window_h) = ctx.gfx.size();
                    let scale_x = drawable_w / window_w;
                    let scale_y = drawable_h / window_h;
                    
                    //  将鼠标坐标从窗口坐标转换为drawable坐标
                    let scaled_x = x * scale_x;
                    let scaled_y = y * scale_y;
                    
                    //  更新鼠标位置（防止快速点击时位置不准确）
                    mouse_input.x = scaled_x;
                    mouse_input.y = scaled_y;
                    
                    if button == MouseButton::Left {
                        println!("[DEBUG] 左键抬起: press_time={}, 双击检测中...", mouse_input.left_press_time);
                        
                        //  双击检测：如果按下时间不太长(< 30帧,约500ms)且距离上次点击 < 500ms
                        if mouse_input.left_press_time < 30 {
                            let now = std::time::Instant::now();
                            let time_since_last_click = now.duration_since(mouse_input.left_last_click_time);
                            
                            println!("[DEBUG] 距离上次点击: {:?}", time_since_last_click);
                            
                            if time_since_last_click < std::time::Duration::from_millis(500) {
                                // 双击！
                                mouse_input.left_double_clicked = true;
                                println!("✅ 左键双击事件触发 at ({}, {})", x, y);
                                // 重置上次点击时间，防止三击被识别为两次双击
                                mouse_input.left_last_click_time = now - std::time::Duration::from_secs(10);
                            } else {
                                // 第一次点击或单击
                                println!("[DEBUG] 第一次点击，记录时间");
                                mouse_input.left_last_click_time = now;
                                // 立即设置为双击，这样下次 update 就能处理
                                mouse_input.left_double_clicked = true;  // 🔥 改为单击也触发移动
                            }
                        } else {
                            println!("[DEBUG] 按下时间太长({}帧)，不算点击", mouse_input.left_press_time);
                        }
                        mouse_input.left_pressed = false;
                        mouse_input.left_press_time = 0;
                    } else {
                        //  双击检测：右键
                        if mouse_input.right_press_time < 30 {
                            let now = std::time::Instant::now();
                            let time_since_last_click = now.duration_since(mouse_input.right_last_click_time);
                            
                            if time_since_last_click < std::time::Duration::from_millis(500) {
                                // 双击！
                                mouse_input.right_double_clicked = true;
                                println!("✅ 右键双击事件触发 at ({}, {})", x, y);
                                mouse_input.right_last_click_time = now - std::time::Duration::from_secs(10);
                            } else {
                                // 第一次点击或单击
                                mouse_input.right_last_click_time = now;
                                mouse_input.right_double_clicked = true;  // 🔥 改为单击也触发移动
                            }
                        }
                        mouse_input.right_pressed = false;
                        mouse_input.right_press_time = 0;
                    }
                } else {
                    println!("[ERROR] 找不到 MouseInput 组件！");
                }
            }
            // 中键：停止拖拽
            MouseButton::Middle => {
                let mut draggable = self.world.get::<&mut Draggable>(self.camera_entity).unwrap();
                CameraSystem::end_drag(&mut draggable);
            }
            _ => {}
        }
        Ok(())
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32, _dx: f32, _dy: f32) -> GameResult<()> {
        //  获取DPI缩放比例
        let (drawable_w, drawable_h) = ctx.gfx.drawable_size();
        let (window_w, window_h) = ctx.gfx.size();
        let scale_x = drawable_w / window_w;
        let scale_y = drawable_h / window_h;
        
        //  将鼠标坐标从窗口坐标转换为drawable坐标
        let scaled_x = x * scale_x;
        let scaled_y = y * scale_y;
        
        // 更新鼠标位置（用于角色控制）
        if let Some((_, mouse_input)) = self.world.query_mut::<&mut MouseInput>().into_iter().next() {
            mouse_input.x = scaled_x;
            mouse_input.y = scaled_y;
        }
        
        // 处理中键拖拽地图
        let draggable = self.world.get::<&Draggable>(self.camera_entity).unwrap().clone();
        if draggable.is_dragging {
            let camera = self.world.get::<&Camera>(self.camera_entity).unwrap().clone();
            let mut pos = self.world.get::<&mut Position>(self.camera_entity).unwrap();
            CameraSystem::update_drag(&draggable, &mut pos, &camera, x, y);
        }
        
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) -> GameResult<()> {
        // 先获取鼠标位置（不涉及 world 借用）
        let mouse_pos = _ctx.mouse.position();
        
        // 然后一次性获取可变引用并调用 zoom
        let mut pos = self.world.get::<&mut Position>(self.camera_entity).unwrap();
        let mut camera = self.world.get::<&mut Camera>(self.camera_entity).unwrap();
        
        CameraSystem::zoom(&mut pos, &mut camera, y, mouse_pos.x, mouse_pos.y);
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult<()> {
        use ggez::input::keyboard::KeyCode;
        use ggez::winit::keyboard::PhysicalKey;

        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            match keycode {
                KeyCode::KeyM => {
                    self.load_new_map(ctx)?;
                }
                KeyCode::Digit1 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_back = !config.show_back;
                    println!("Back 层 (1): {}", if config.show_back { "显示" } else { "隐藏" });
                }
                KeyCode::Digit2 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_middle = !config.show_middle;
                    println!("Middle 层 (2): {}", if config.show_middle { "显示" } else { "隐藏" });
                }
                KeyCode::Digit3 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_front = !config.show_front;
                    println!("Front 层 (3): {}", if config.show_front { "显示" } else { "隐藏" });
                }
                KeyCode::KeyB => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_borders = !config.show_borders;
                    println!("纹理边框 (B): {}", if config.show_borders { "显示" } else { "隐藏" });
                }
                KeyCode::KeyG => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_grid = !config.show_grid;
                    println!("网格 (G): {}", if config.show_grid { "显示" } else { "隐藏" });
                }
                KeyCode::KeyO => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_obstacles = !config.show_obstacles;
                    println!("障碍物 (O): {}", if config.show_obstacles { "显示" } else { "隐藏" });
                }
                KeyCode::KeyA => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_animations = !config.show_animations;
                    println!("动画 (A): {}", if config.show_animations { "播放" } else { "暂停" });
                }
                KeyCode::KeyL => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.enable_lod = !config.enable_lod;
                    println!(" LOD优化 (L): {}", if config.enable_lod { "启用（缩小时过滤50%瓦片）" } else { "禁用" });
                }
                KeyCode::KeyP => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_path = !config.show_path;
                    println!(" 寻路路径 (P): {}", if config.show_path { "显示" } else { "隐藏" });
                }
                KeyCode::F9 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_monster_borders = !config.show_monster_borders;
                    println!(" 怪物边框 (F9): {}", if config.show_monster_borders { "显示" } else { "隐藏" });
                }
                KeyCode::F10 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_npc_borders = !config.show_npc_borders;
                    println!(" NPC边框 (F10): {}", if config.show_npc_borders { "显示" } else { "隐藏" });
                }
                KeyCode::F11 => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.show_effect_borders = !config.show_effect_borders;
                    println!(" 特效边框 (F11): {}", if config.show_effect_borders { "显示" } else { "隐藏" });
                }
                KeyCode::Equal | KeyCode::NumpadAdd => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.max_fps = (config.max_fps + 10).min(300);
                    println!(" 最大FPS (+ 键): {} 帧", config.max_fps);
                }
                KeyCode::Minus | KeyCode::NumpadSubtract => {
                    let mut config = self.world.get::<&mut RenderConfig>(self.config_entity).unwrap();
                    config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                    println!(" 最大FPS (- 键): {} 帧", config.max_fps);
                }
                KeyCode::Escape => {
                    ctx.request_quit();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> GameResult<()> {
        //  使用 drawable_size 而不是 window size (处理高DPI)
        let (actual_width, actual_height) = ctx.gfx.drawable_size();
        println!(" 窗口大小调整: 窗口尺寸={}x{}, 可绘制尺寸={}x{}", 
                 width, height, actual_width, actual_height);
        
        let mut camera = self.world.get::<&mut Camera>(self.camera_entity).unwrap();
        camera.screen_width = actual_width;
        camera.screen_height = actual_height; 
        Ok(())
    }
}

// ============================================================================
// 主函数
// ============================================================================


fn main() -> GameResult {
    // 默认地图路径
    let default_map = "Map/0.map";

    // 创建 GGEZ 上下文
    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer_ecs", "Crystal Team")
        .window_setup(WindowSetup::default().title("传奇地图查看器 ECS - GGEZ + hecs").samples(ggez::conf::NumSamples::Four).vsync(true))
        .window_mode(
            WindowMode::default()
            .fullscreen_type(ggez::conf::FullscreenType::Windowed)
                .dimensions(1280.0, 720.0)
                .resizable(true),
        )
        .build()?;

    // 创建应用
    let app = MapViewerApp::new(&mut ctx, default_map)?;

    println!("\n ECS 地图查看器已启动!");
    println!(" 快捷键:");
    println!("   [鼠标左键长按] - 角色走动 (自动寻路)");
    println!("   [鼠标右键长按] - 角色跑动 (自动寻路)");
    println!("    [鼠标中键拖拽] - 移动地图");
    println!("  [鼠标滚轮] - 缩放");
    println!("");
    println!("   图层控制:");
    println!("     [1] - 切换 Back 层");
    println!("     [2] - 切换 Middle 层");
    println!("     [3] - 切换 Front 层");
    println!("");
    println!("   调试功能:");
    println!("     [G] - 切换网格显示");
    println!("     [O] - 切换障碍物显示");
    println!("     [B] - 切换边框显示 (调试)");
    println!("     [P] - 切换寻路路径显示");
    println!("     [F9]  - 切换怪物边框");
    println!("     [F10] - 切换NPC边框");
    println!("     [F11] - 切换特效边框");
    println!("");
    println!("   其他功能:");
    println!("     [M] - 选择地图文件");
    println!("     [A] - 切换动画播放");
    println!("     [L] - 切换 LOD 优化（缩小时过滤纹理）");
    println!("     [+/-] - 调整最大帧率限制");
    println!("     [ESC] - 退出");
    println!("");
    println!(" 性能优化:");
    println!("   最大帧率: 160 FPS (可调)");
    println!("   LOD: 缩放 < 0.5x 时自动过滤 50% Middle/Front 瓦片");
    println!("   Z轴排序: 灵活控制绘制顺序\n");

    // 运行事件循环
    event::run(ctx, event_loop, app)?;
    Ok(())
}

