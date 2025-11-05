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
    MapData, RenderConfig, TimeTracker, VisibleArea,
    LocalPlayer, NetworkSync, NetworkObjectType, PlayerInput, MovementVelocity,
    Path, Movement, Prediction,
};
use crate::ecs::{Coord, GameContext, MapLoader, MapUtils};
use crate::objects::MapReader;
use mir2_shared::enums::{MirClass, MirGender};

// 使用 map_load_system 中定义的 MapManager
use super::map_load_system::MapManager;

/// 地图切换请求组件
/// 
/// 用于存储 M 键触发的地图切换请求
pub struct MapSwitchRequest {
    /// 待加载的地图文件路径
    pub map_path: String,
}

/// 地图更新系统
/// 
/// 在每帧检查是否需要切换地图，如果需要则执行地图加载和实体重建
pub struct MapUpdateSystem;

impl MapUpdateSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 内部更新逻辑
    /// 
    /// 检查 MapSwitchRequest 组件，如果存在则执行地图切换
    fn do_update(world: &mut World) -> GameResult {
        
        // 查询 MapSwitchRequest 组件
        let map_path = {
            let mut query = world.query::<&MapSwitchRequest>();
            if let Some((_, request)) = query.iter().next() {
                Some(request.map_path.clone())
            } else {
                None
            }
        };

        if let Some(new_path) = map_path {
            tracing::info!("🗺️  MapUpdateSystem: 正在加载地图 {}", new_path);
            
            // 在清空世界前,保存需要保留的组件和相机状态
            use crate::ecs::WorldExt;
            let net_ctx = (*world.network()).clone();
            let settings = (*world.settings()).clone();
            
            // 保存相机位置和状态
            let (camera_pos, camera_zoom, screen_width, screen_height) = world.query::<(&Position, &Camera)>()
                .into_iter()
                .next()
                .map(|(_, (pos, cam))| (pos.clone(), cam.zoom, cam.screen_width, cam.screen_height))
                .unwrap_or((Position { x: 800.0, y: 600.0 }, 1.0, 1600.0, 1200.0));
            
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
                    let (spawn_x, spawn_y) = Coord::grid_to_world_center(spawn_grid_x, spawn_grid_y);
                    
                    println!("🎯 出生位置: 格子({}, {}) -> 世界坐标({:.1}, {:.1})", 
                             spawn_grid_x, spawn_grid_y, spawn_x, spawn_y);
                    
                    // 重新创建所有实体 - 使用保存的相机位置和缩放
                    Self::recreate_entities(world, spawn_x, spawn_y, screen_width, screen_height, camera_pos, camera_zoom);
                    
                    // 恢复全局组件 - 使用固定实体ID
                    use crate::ecs::{NETWORK_ENTITY, SETTING_ENTITY};
                    if let Some(entity_id) = SETTING_ENTITY {
                        world.spawn_at(entity_id, (settings,));
                    }
                    if let Some(entity_id) = NETWORK_ENTITY {
                        world.spawn_at(entity_id, (net_ctx,));
                    }
                    
                    // 重新创建 MapManager（使用文件名）
                    let map_file = std::path::Path::new(&new_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    world.spawn((MapManager {
                        current_map_index: -1,
                        current_map_file: map_file,
                        current_map_title: String::from("未知地图"),
                        is_loading: false,
                    },));
                    
                    tracing::info!("✅ 地图切换完成: {}", new_path);
                }
                Err(e) => {
                    tracing::error!("❌ 地图读取失败: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// 触发地图选择对话框
    /// 
    /// 打开文件选择器，让用户选择新地图
    pub fn trigger_map_selection(world: &mut World) {
        if let Some(path) = FileDialog::new()
            .add_filter("Map files", &["map"])
            .set_directory("Map")
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            tracing::info!("📂 用户选择地图: {}", path_str);
            
            // 删除旧的 MapSwitchRequest（如果存在）
            let to_remove: Vec<_> = world.query::<&MapSwitchRequest>()
                .iter()
                .map(|(e, _)| e)
                .collect();
            for entity in to_remove {
                let _ = world.despawn(entity);
            }
            
            // 创建新的地图切换请求
            world.spawn((MapSwitchRequest {
                map_path: path_str,
            },));
        }
    }
    
    /// 重新创建所有必要的实体
    fn recreate_entities(
        world: &mut World, 
        spawn_x: f32, 
        spawn_y: f32, 
        screen_width: f32, 
        screen_height: f32,
        camera_pos: Position,
        camera_zoom: f32,
    ) {
        use crate::ecs::components::{CameraMode};
        
        // 创建相机实体 - 使用保存的相机位置和缩放
        world.spawn((
            camera_pos,
            Camera {
                zoom: camera_zoom,
                screen_width,
                screen_height,
            },
            CameraMode::Manual,  // 地图查看器使用手动模式
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
            show_static_tiles: true,
            show_animated_tiles: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_player_debug: false,
            show_path: false,
            max_fps: 160,
            enable_lod: true,
            enable_camera_drag: true,  // 地图查看器启用鼠标拖拽功能
        },));
        
        // 创建可见区域缓存实体
        world.spawn((VisibleArea::default(),));
        
        // 创建玩家角色实体
        world.spawn((
            Player {
                direction: 4,
                action: PlayerAction::Stand,
                is_moving: false,
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
        
        // 输入事件现在通过 GameContext 传递，无需单独组件
        
        // // 创建鼠标输入状态实体（地图查看器需要）
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
    }
}

// ============================================================================
// System Trait 实现
// ============================================================================

use crate::ecs::systems::LogicSystem;

impl LogicSystem for MapUpdateSystem {
    

    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        Self::do_update(&mut ctx.world)
    }
}
