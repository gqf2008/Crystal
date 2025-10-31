//! 地图更新系统
//! 
//! 职责：
//! - 监听地图切换请求（通过 MapManager 组件）
//! - 执行地图加载和世界重置
//! - 重新创建所有必要的实体（相机、玩家、配置等）

use hecs::World;
use ggez::{Context, GameResult};
use std::time::Instant;
use rfd::FileDialog;

use crate::ecs::components::{
    Position, Camera, Draggable, Player, PlayerAction, PlayerAppearance, MoveMode,
    MovementAnimation, MapData, RenderConfig, TimeTracker, VisibleArea,
    LocalPlayer, NetworkSync, NetworkObjectType, PlayerInput, MovementVelocity,
    Path, Movement, Prediction, GlobalEvents,
};
use crate::ecs::{Coordinates, MapUtils, MapLoader};
use crate::objects::MapReader;
use mir2_shared::enums::{MirClass, MirGender};

/// 地图管理组件
/// 
/// 用于控制地图加载和切换的状态
pub struct MapManager {
    /// 当前加载的地图路径
    pub current_map_path: String,
    /// 是否需要重新加载地图
    pub needs_reload: bool,
    /// 待加载的地图路径
    pub pending_map_path: Option<String>,
}

impl MapManager {
    pub fn new(map_path: String) -> Self {
        Self {
            current_map_path: map_path,
            needs_reload: false,
            pending_map_path: None,
        }
    }
}

/// 地图更新系统
/// 
/// 在每帧检查是否需要切换地图，如果需要则执行地图加载和实体重建
pub struct MapUpdateSystem {
    pub current_map_path: String,
    /// 是否需要重新加载地图
    pub needs_reload: bool,
    /// 待加载的地图路径
    pub pending_map_path: Option<String>,
}

impl MapUpdateSystem {
    pub fn new(initial_map_path: String) -> Self {
        Self {
            current_map_path: initial_map_path,
            needs_reload: false,
            pending_map_path: None,
        }
    }
}
impl MapUpdateSystem {
    /// 更新地图状态
    /// 
    /// 检查 MapManager 组件，如果 needs_reload 为 true，则加载新地图
    pub fn update(world: &mut World, ctx: &mut Context) -> GameResult {
        
        // 查询 MapManager 组件并立即获取数据
        let (needs_reload, pending_path) = {
            let mut query = world.query::<&MapManager>();
            if let Some((_, manager)) = query.iter().next() {
                (manager.needs_reload, manager.pending_map_path.clone())
            } else {
                return Ok(());  // 没有 MapManager 组件
            }
        };

        if !needs_reload {
            return Ok(());  // 不需要重新加载
        }

        if let Some(new_path) = pending_path {
            println!("🗺️  MapUpdateSystem: 正在加载地图 {}", new_path);
            
            // 加载新地图
            match MapReader::new(&new_path) {
                Ok(reader) => {
                    println!("✅ 地图读取成功: {}x{}", reader.width, reader.height);
                    
                    // 清空世界
                    world.clear();
                    
                    // 加载地图瓦片
                    if let Err(e) = MapLoader::load_map(world, reader) {
                        eprintln!("❌ 地图加载失败: {}", e);
                        return Err(e);
                    }
                    
                    // 找到出生点
                    let map_data = world.query_mut::<&MapData>()
                        .into_iter()
                        .next()
                        .map(|(_, data)| data.clone())
                        .expect("地图数据未加载");
                    
                    let (spawn_grid_x, spawn_grid_y) = MapUtils::find_center_walkable_position(&map_data);
                    let (spawn_x, spawn_y) = Coordinates::grid_to_world_center(spawn_grid_x, spawn_grid_y);
                    
                    println!("🎯 出生位置: 格子({}, {}) -> 世界坐标({:.1}, {:.1})", 
                             spawn_grid_x, spawn_grid_y, spawn_x, spawn_y);
                    
                    // 重新创建所有实体
                    Self::recreate_entities(world, ctx, spawn_x, spawn_y);
                    
                    // 重新创建 MapManager
                    world.spawn((MapManager {
                        current_map_path: new_path.clone(),
                        needs_reload: false,
                        pending_map_path: None,
                    },));
                    
                    println!("✅ 地图切换完成");
                }
                Err(e) => {
                    eprintln!("❌ 地图读取失败: {}", e);
                    // 重置标志
                    for (_, manager) in world.query::<&mut MapManager>().iter() {
                        manager.needs_reload = false;
                        manager.pending_map_path = None;
                        break;
                    }
                }
            }
        } else {
            // 没有待加载路径，重置标志
            for (_, manager) in world.query::<&mut MapManager>().iter() {
                manager.needs_reload = false;
                break;
            }
        }
        
        Ok(())
    }
    
    /// 触发地图选择对话框
    /// 
    /// 打开文件选择器，让用户选择新地图
    fn trigger_map_selection(world: &mut World) {
        if let Some(path) = FileDialog::new()
            .add_filter("Map files", &["map"])
            .set_directory("Map")
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            println!("📂 用户选择地图: {}", path_str);
            
            // 更新 MapManager
            for (_, manager) in world.query::<&mut MapManager>().iter() {
                manager.pending_map_path = Some(path_str.clone());
                manager.needs_reload = true;
                break;
            }
        }
    }
    
    /// 重新创建所有必要的实体
    fn recreate_entities(world: &mut World, ctx: &Context, spawn_x: f32, spawn_y: f32) {
        let screen = ctx.gfx.drawable_size();
        
        // 创建相机实体
        world.spawn((
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
        world.spawn((TimeTracker {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: Instant::now(),
            last_frame_time: Instant::now(),
        },));
        
        // 创建渲染配置实体
        world.spawn((RenderConfig {
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
        world.spawn((VisibleArea::default(),));
        
        // 创建玩家角色实体
        world.spawn((
            Player {
                direction: 4,
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
                last_move_time: Instant::now(),
                move_delay: std::time::Duration::from_millis(600),
                waiting_server_confirm: false,
                collision_detected: false,
                collision_target_grid: None,
                can_run: true,
                last_run_time: Instant::now(),
                run_cooldown: std::time::Duration::from_millis(900),
            },
            Position { x: spawn_x, y: spawn_y },
            PlayerAppearance {
                class: MirClass::Warrior,
                gender: MirGender::Male,
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
            LocalPlayer,
            NetworkSync {
                object_id: 1,
                last_update: Instant::now(),
                object_type: NetworkObjectType::Player,
            },
            PlayerInput::new(),
            MovementVelocity::with_speeds(
                crate::ecs::components::DEFAULT_MAX_SPEED,
                crate::ecs::components::DEFAULT_WALK_SPEED,
                crate::ecs::components::DEFAULT_RUN_SPEED,
            ),
            Path::new(),
            Movement::new(),
            Prediction::new(Position { x: spawn_x, y: spawn_y }),
        ));
        
        // 创建 GlobalEvents 组件 (新架构)
        world.spawn((GlobalEvents::new(),));
    }
}
