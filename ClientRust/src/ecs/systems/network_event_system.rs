// ============================================================================
// NetworkEventSystem - 处理网络事件并更新ECS组件
// ============================================================================
//
// 职责：
// - 监听网络事件（MapInformation, UserInformation, ObjectPlayer等）
// - 根据事件创建/更新ECS实体和组件
// - 是连接网络层和ECS世界的桥梁
//
// 架构优势：
// - 场景无需关心网络事件细节
// - 所有实体创建逻辑集中管理
// - 符合ECS的单一职责原则
//
// ============================================================================

use hecs::World;
use crate::network::handlers::GameEvent;
use crate::ecs::components::*;
use crate::ecs::map_loader::MapLoader;
use crate::objects::MapReader;

/// 网络事件系统
pub struct NetworkEventSystem;

impl NetworkEventSystem {
    /// 处理网络事件并更新World
    pub fn process_event(world: &mut World, event: &GameEvent) {
        match event {
            // ========================================================================
            // 地图相关事件
            // ========================================================================
            GameEvent::MapChanged { file_name, .. } => {
                Self::handle_map_information(world, file_name);
            }
            
            // ========================================================================
            // 玩家数据事件 (进入游戏后服务器发送)
            // ========================================================================
            GameEvent::PlayerLocationChanged { x, y } => {
                Self::handle_player_location(world, *x, *y);
            }
            
            GameEvent::HealthChanged { current, max } => {
                Self::handle_health_changed(world, *current, *max);
            }
            
            GameEvent::ManaChanged { current, max } => {
                Self::handle_mana_changed(world, *current, *max);
            }
            
            GameEvent::GoldChanged { amount } => {
                Self::handle_gold_changed(world, *amount);
            }
            
            GameEvent::ExperienceGained { amount } => {
                Self::handle_experience_gained(world, *amount);
            }
            
            GameEvent::LevelUp { new_level } => {
                Self::handle_level_up(world, *new_level);
            }
            
            _ => {
                // 其他事件暂不处理
            }
        }
    }
    
    // ========================================================================
    // 私有方法：具体事件处理逻辑
    // ========================================================================
    
    /// 处理地图信息事件：加载地图并创建实体
    fn handle_map_information(world: &mut World, map_file_name: &str) {
        tracing::info!("🗺️ 加载地图: {}", map_file_name);
        
        let map_path = format!("Map/{}.map", map_file_name);
        match MapReader::new(&map_path) {
            Ok(reader) => {
                tracing::info!("✅ 地图加载成功: {}x{}", reader.width, reader.height);
                
                // 加载地图瓦片到ECS
                if let Err(e) = MapLoader::load_map(world, reader) {
                    tracing::error!("❌ 地图瓦片加载失败: {}", e);
                    return;
                }
                
                // 生成测试怪物（使用临时作用域避免借用冲突）
                {
                    let map_data_opt = world.query::<&MapData>().iter().next().map(|(_, data)| data.clone());
                    if let Some(map_data) = map_data_opt {
                        MapLoader::spawn_test_monsters(world, &map_data, 15);
                        tracing::info!("✅ 已生成 15 只测试怪物");
                    }
                }
            }
            Err(e) => {
                tracing::error!("❌ 地图文件读取失败: {}", e);
            }
        }
    }
    
    /// 处理玩家位置更新
    fn handle_player_location(world: &mut World, x: i32, y: i32) {
        tracing::info!("📍 玩家位置更新: ({}, {})", x, y);
        
        // 检查是否已存在本地玩家实体
        let has_local_player = world.query::<&LocalPlayer>().iter().next().is_some();
        
        if !has_local_player {
            // 首次收到位置信息，创建本地玩家实体
            Self::create_local_player(world, x, y);
        } else {
            // 更新已存在的本地玩家位置
            for (_, (pos, _)) in world.query::<(&mut Position, &LocalPlayer)>().iter() {
                let (world_x, world_y) = crate::ecs::Coordinates::grid_to_world_center(x, y);
                pos.x = world_x;
                pos.y = world_y;
                tracing::info!("✅ 本地玩家位置已更新到世界坐标: ({:.1}, {:.1})", world_x, world_y);
            }
        }
    }
    
    /// 创建本地玩家实体（从CharacterList读取角色数据）
    fn create_local_player(world: &mut World, grid_x: i32, grid_y: i32) {
        // 从World读取选中的角色数据
        let (player_name, player_class, player_gender, player_level) = {
            world.query::<&CharacterList>()
                .iter()
                .next()
                .and_then(|(_, char_list)| char_list.get_selected())
                .map(|character| (
                    character.name.clone(),
                    character.class,
                    character.gender,
                    character.level,
                ))
                .unwrap_or_else(|| {
                    tracing::warn!("⚠️ 未找到选中角色，使用默认值");
                    ("勇士".to_string(), MirClass::Warrior, MirGender::Female, 1)
                })
        };
        
        let (world_x, world_y) = crate::ecs::Coordinates::grid_to_world_center(grid_x, grid_y);
        
        tracing::info!("🧙 创建本地玩家: {} (Lv.{}, {:?}, {:?}) at ({:.1}, {:.1})", 
                      player_name, player_level, player_class, player_gender, world_x, world_y);
        
        // 创建玩家实体
        world.spawn((
            Player {
                direction: 4,
                action: PlayerAction::Stand,
                frame_index: 0,
                frame_time: 0,
                speed: 0.0,
                target_x: world_x,
                target_y: world_y,
                is_moving: false,
                path: Vec::new(),
                path_index: 0,
                move_mode: MoveMode::Idle,
                last_move_time: std::time::Instant::now(),
                move_delay: std::time::Duration::from_millis(700),
                waiting_server_confirm: false,
                collision_detected: false,
                collision_target_grid: None,
                can_run: false,
                last_run_time: std::time::Instant::now(),
                run_cooldown: std::time::Duration::from_millis(900),
            },
            Position { x: world_x, y: world_y },
            MovementAnimation::new(grid_x, grid_y),
            PlayerAppearance {
                class: player_class,
                gender: player_gender,
                hair: 0,
                weapon: -1,
                armour: 0,
                weapon_effect: 0,
                wing_effect: 0,
            },
            Inventory::default(),
            Equipment::new(),
            LocalPlayer,
            PlayerData {
                id: 1,
                name: player_name,
                class: player_class,
                gender: player_gender,
                level: player_level,
                exp: 0,
                max_experience: 1000,
                gold: 0,
                credit: 0,
            },
            Health::new(100),
            Mana::new(100),
            MagicList::new(),
            LearnableMagicList::new(),
            TargetSelection::new(),
            NetworkSync {
                object_id: 0,
                last_update: std::time::Instant::now(),
                object_type: NetworkObjectType::Player,
            },
        ));
        
        tracing::info!("✅ 本地玩家实体已创建");
    }
    
    /// 处理生命值变化
    fn handle_health_changed(world: &mut World, current: u32, max: u32) {
        for (_, (health, _)) in world.query::<(&mut Health, &LocalPlayer)>().iter() {
            health.current = current as i32;
            health.max = max as i32;
            tracing::info!("💚 生命值更新: {}/{}", current, max);
        }
    }
    
    /// 处理魔法值变化
    fn handle_mana_changed(world: &mut World, current: u32, max: u32) {
        for (_, (mana, _)) in world.query::<(&mut Mana, &LocalPlayer)>().iter() {
            mana.current = current as i32;
            mana.max = max as i32;
            tracing::info!("💙 魔法值更新: {}/{}", current, max);
        }
    }
    
    /// 处理金币变化
    fn handle_gold_changed(world: &mut World, amount: u32) {
        for (_, (player_data, _)) in world.query::<(&mut PlayerData, &LocalPlayer)>().iter() {
            player_data.gold = amount;
            tracing::info!("💰 金币更新: {}", amount);
        }
    }
    
    /// 处理经验值获得
    fn handle_experience_gained(world: &mut World, amount: i64) {
        for (_, (player_data, _)) in world.query::<(&mut PlayerData, &LocalPlayer)>().iter() {
            player_data.exp += amount;
            tracing::info!("⭐ 获得经验: {} (当前: {})", amount, player_data.exp);
        }
    }
    
    /// 处理等级提升
    fn handle_level_up(world: &mut World, new_level: u16) {
        for (_, (player_data, _)) in world.query::<(&mut PlayerData, &LocalPlayer)>().iter() {
            player_data.level = new_level;
            tracing::info!("🎉 等级提升: Lv.{}", new_level);
        }
    }
}
