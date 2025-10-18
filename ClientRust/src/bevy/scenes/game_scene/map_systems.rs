// map_systems.rs - 地图加载和渲染系统
// 
// 功能说明:
// - 地图加载: 初始化地图数据、创建瓦片、添加地图对象
// - 图层管理: 创建多层地图图层实体
// - 对象生成: 根据地图对象数据生成 NPC、传送点等实体
// - 状态同步: 同步地图加载状态到 GameSceneState
// - 碰撞检测: 检查玩家是否可以通过瓦片
//
// 系统列表:
// 1. load_map_system - 地图加载系统
// 2. create_map_layers_system - 创建地图图层系统
// 3. spawn_map_objects_system - 生成地图对象系统
// 4. update_map_state_system - 更新地图状态系统
// 5. handle_map_collision_system - 地图碰撞检测系统

use bevy::prelude::*;
use super::{
    MapData, MapTile, MapObject, MapLayer, GameSceneState, 
    Player, NPC, InteractiveObject
};

/// 加载地图系统 - 初始化地图数据
/// 
/// 功能:
/// - 创建默认地图 (100×100)
/// - 初始化所有地面瓦片
/// - 设置边界为不可行走区域
/// - 添加地图对象 (NPC、传送点)
/// - 标记地图加载完成
pub fn load_map_system(
    mut map_data: ResMut<MapData>,
    mut game_state: ResMut<GameSceneState>,
) {
    // 如果地图已经加载，跳过
    if map_data.is_loaded {
        return;
    }
    
    info!("🗺️ 开始加载地图...");
    
    // 初始化地图数据
    *map_data = MapData::new(
        1,
        "Mirror World".to_string(),
        100,
        100,
    );
    
    // 初始化地面瓦片
    for y in 0..map_data.height {
        for x in 0..map_data.width {
            let mut tile = MapTile {
                tile_x: x,
                tile_y: y,
                layer: 0,
                tile_id: 1,  // 地面瓦片
                walkable: true,
            };
            
            // 添加一些不可行走的区域（例如树木、墙壁）
            if (x < 5) || (x > 95) || (y < 5) || (y > 95) {
                tile.walkable = false;
                tile.tile_id = 2;  // 不可行走的瓦片
            }
            
            map_data.set_tile(x, y, 0, tile);
        }
    }
    
    // 添加一些地图对象
    let npc = MapObject {
        object_id: 1,
        object_type: 1,  // NPC
        x: 50,
        y: 50,
        name: "村长".to_string(),
        properties: Default::default(),
    };
    map_data.add_object(npc);
    
    // 添加传送点
    let teleport = MapObject {
        object_id: 2,
        object_type: 3,  // 传送点
        x: 25,
        y: 25,
        name: "传送点".to_string(),
        properties: Default::default(),
    };
    map_data.add_object(teleport);
    
    map_data.is_loaded = true;
    
    // 更新游戏状态
    game_state.is_initialized = true;
    
    info!("🗺️ 地图加载完成 (100×100, 对象数: 2)");
}

/// 创建地图图层系统 - 生成地图图层实体
/// 
/// 功能:
/// - 为每个图层 (0-2) 创建实体
/// - 设置图层索引、Transform、可见性
/// - 记录图层创建日志
pub fn create_map_layers_system(
    mut commands: Commands,
    map_data: Res<MapData>,
) {
    if !map_data.is_loaded {
        return;
    }
    
    // 为每个图层创建实体
    for layer_idx in 0..3 {
        let _layer_entity = commands.spawn((
            MapLayer {
                layer_index: layer_idx as u32,
            },
            Transform::default(),
            Visibility::default(),
            Name::new(format!("MapLayer_{}", layer_idx)),
        )).id();
        
        info!("✅ 地图图层 {} 已创建", layer_idx);
    }
}

/// 生成地图对象系统 - 生成 NPC 和其他对象
/// 
/// 功能:
/// - 根据地图对象类型生成不同实体
/// - 类型 1: NPC (带对话功能)
/// - 类型 3: 传送点 (可交互对象)
/// - 设置对象位置、名称、交互范围
pub fn spawn_map_objects_system(
    mut commands: Commands,
    map_data: Res<MapData>,
) {
    if !map_data.is_loaded {
        return;
    }
    
    for object in &map_data.objects {
        match object.object_type {
            1 => {
                // NPC
                commands.spawn((
                    Transform::from_xyz(
                        object.x as f32 * 32.0,
                        object.y as f32 * 32.0,
                        1.0,
                    ),
                    NPC {
                        npc_id: object.object_id as i32,
                        name: object.name.clone(),
                        dialogue_id: None,
                    },
                    Visibility::Visible,
                    Name::new(format!("NPC_{}", object.name)),
                ));
                
                info!("👤 NPC 已生成: {} 在 ({}, {})", 
                    object.name, object.x, object.y);
            }
            3 => {
                // 传送点
                commands.spawn((
                    Transform::from_xyz(
                        object.x as f32 * 32.0,
                        object.y as f32 * 32.0,
                        0.5,
                    ),
                    InteractiveObject {
                        object_id: object.object_id as i32,
                        name: object.name.clone(),
                        object_type: "teleport".to_string(),
                        interaction_range: 32.0,
                    },
                    Visibility::Visible,
                    Name::new("Teleport"),
                ));
                
                info!("🚪 传送点已生成: {} 在 ({}, {})", 
                    object.name, object.x, object.y);
            }
            _ => {
                info!("ℹ️ 未知对象类型: {}", object.object_type);
            }
        }
    }
    
    info!("🎮 所有地图对象已生成 (总数: {})", map_data.objects.len());
}

/// 更新地图状态系统 - 监听地图加载完成
/// 
/// 功能:
/// - 检测地图加载完成状态
/// - 同步地图名称到 GameSceneState
/// - 标记游戏场景已初始化
pub fn update_map_state_system(
    map_data: Res<MapData>,
    mut game_state: ResMut<GameSceneState>,
) {
    if map_data.is_loaded && !game_state.is_initialized {
        game_state.current_map = map_data.map_name.clone();
        game_state.is_initialized = true;
        info!("🗺️ 地图状态已更新: {}", map_data.map_name);
    }
}

/// 处理地图碰撞检测系统
/// 
/// 功能:
/// - 将玩家世界坐标转换为地图瓦片坐标
/// - 检查目标瓦片是否可行走
/// - 如果不可行走则回退到上一个有效位置
/// - 记录碰撞日志
pub fn handle_map_collision_system(
    mut player_query: Query<&mut Transform, With<Player>>,
    map_data: Res<MapData>,
) {
    if !map_data.is_loaded {
        return;
    }
    
    for mut player_transform in player_query.iter_mut() {
        // 将世界坐标转换为地图坐标
        let tile_x = (player_transform.translation.x / 32.0) as u16;
        let tile_y = (player_transform.translation.y / 32.0) as u16;
        
        // 检查该瓦片是否可行走
        if !map_data.is_walkable(tile_x, tile_y) {
            // 回退到上一个有效位置
            player_transform.translation.x = ((tile_x - 1) as f32 * 32.0);
            
            info!("⚠️ 玩家碰撞检测: 不可通过的瓦片 ({}, {})", tile_x, tile_y);
        }
    }
}
