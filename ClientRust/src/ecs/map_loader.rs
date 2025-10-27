// ============================================================================
// Map Loader - 地图加载器
// ============================================================================
//
// 从 MapReader 加载地图数据到 ECS World
// 创建所有瓦片实体（MapTile + AnimatedTile + Door）
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::objects::{MapReader, CellInfo};
use std::time::Instant;

use crate::ecs::components::{
    MapData, MapTile, TileLayer, AnimatedTile, Door, DoorState, TileOcclusion,  // 🆕 添加 TileOcclusion
    CELL_WIDTH, CELL_HEIGHT,
};

pub struct MapLoader;

impl MapLoader {
    /// 从 MapReader 创建所有瓦片实体
    pub fn load_map(world: &mut World, reader: MapReader) -> GameResult<()> {
        let width = reader.width;
        let height = reader.height;
        let cells = reader.map_cells.clone();

        // 创建地图数据单例
        world.spawn((MapData {
            cells: cells.clone(),
            width,
            height,
        },));

        println!("📦 正在加载地图瓦片到 ECS...");
        let mut tile_count = 0;

        // 遍历所有格子，创建瓦片实体
        for x in 0..width {
            for y in 0..height {
                let cell = &cells[x as usize][y as usize];

                // Back 层 - 只加载偶数行列 (传奇地图特性：Back层使用大瓦片96x64覆盖4个格子)
                if x % 2 == 0 && y % 2 == 0 {
                    Self::load_back_tile(world, cell, x, y, &mut tile_count);
                }

                // Middle 层
                Self::load_middle_tile(world, cell, x, y, &mut tile_count);

                // Front 层
                Self::load_front_tile(world, cell, x, y, &mut tile_count);
            }
        }

        println!("✅ 加载完成: {} 个瓦片实体", tile_count);
        Ok(())
    }

    fn load_back_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
        let index = (cell.back_image & 0x1FFFFFFF) - 1;
        if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
            return;
        }

        // Back层只有静态瓦片，无动画（传奇地图特性）
        let tile = MapTile {
            grid_x: x,
            grid_y: y,
            layer: TileLayer::Back,
            library_index: cell.back_index,
            image_index: index,
            use_blend: false,
            brightness: 1.0,
            z_order: 0,  // 🎯 Back层最底层
        };

        world.spawn((tile,));
        *count += 1;
    }

    fn load_middle_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
        let index = (cell.middle_image & 0x7FFF) - 1;
        if index < 0 || cell.middle_index < 0 {
            return;
        }

        let mut animation = cell.middle_animation_frame;
        let use_blend = (animation & 0x0f) > 0;
        animation &= 0x0f;

        if animation > 0 {
            // 动画瓦片
            let tile = MapTile {
                grid_x: x,
                grid_y: y,
                layer: TileLayer::Middle,
                library_index: cell.middle_index,
                image_index: index,
                use_blend: use_blend && (animation == 10 || animation == 8),
                brightness: 1.0,
                z_order: 1000,  // 🎯 Middle层中间层
            };

            let anim = AnimatedTile {
                frame_count: animation,
                frame_interval: cell.middle_animation_tick,
                base_image_index: index,
            };

            world.spawn((tile, anim));
        } else {
            // 静态瓦片
            let tile = MapTile {
                grid_x: x,
                grid_y: y,
                layer: TileLayer::Middle,
                library_index: cell.middle_index,
                image_index: index,
                use_blend: false,
                brightness: 1.0,
                z_order: 1000,  // 🎯 Middle层中间层
            };

            world.spawn((tile,));
        }

        *count += 1;
    }

    fn load_front_tile(world: &mut World, cell: &CellInfo, x: i32, y: i32, count: &mut i32) {
        let index = (cell.front_image & 0x7FFF) - 1;
        if index < 0 || cell.front_index < 0 || cell.front_index == 200 {
            return;
        }

        let mut animation = cell.front_animation_frame;
        let use_blend = (animation & 0x80) != 0;
        animation &= 0x7F;

        let has_animation = animation > 0;
        let has_door = cell.door_index > 0;

        // 创建瓦片
        let tile = MapTile {
            grid_x: x,
            grid_y: y,
            layer: TileLayer::Front,
            library_index: cell.front_index,
            image_index: index,
            use_blend,
            brightness: if use_blend && !has_animation { 1.5 } else { 1.0 },
            z_order: 2000,  // 🎯 Front层最上层
        };

        let mut builder = hecs::EntityBuilder::new();
        builder.add(tile);
        
        // 🆕 添加遮挡组件（用于动态透明度控制）
        builder.add(TileOcclusion::new());

        // 添加动画组件
        if has_animation {
            let anim = AnimatedTile {
                frame_count: animation,
                frame_interval: cell.front_animation_tick,
                base_image_index: index,
            };
            builder.add(anim);
        }

        // 添加门组件
        if has_door {
            let door = Door {
                door_index: cell.door_index,
                door_offset: cell.door_offset as i32,
                state: DoorState::Closed,
                current_frame: 0,
                last_tick: Instant::now(),
            };
            builder.add(door);
        }

        world.spawn(builder.build());
        *count += 1;
    }
    
    /// 生成测试怪物
    /// 
    /// 在地图上随机生成一些测试怪物,确保生成在可行走位置
    pub fn spawn_test_monsters(world: &mut World, map_data: &MapData, count: usize) {
        use crate::ecs::components::{
            Position, MonsterData, AIState, Health, Animation, Sprite,
        };
        use crate::ecs::{Coordinates, MapUtils};
        use mir2_shared::MirAction;
        use rand::Rng;
        
        println!("🐲 正在生成 {} 个测试怪物...", count);
        
        let mut rng = rand::thread_rng();
        let mut spawned = 0;
        
        // 尝试生成怪物,最多重试次数
        let max_attempts = count * 10;
        let mut attempts = 0;
        
        while spawned < count && attempts < max_attempts {
            attempts += 1;
            
            // 随机网格位置（避开地图边缘）
            let grid_x = rng.gen_range(10..map_data.width - 10);
            let grid_y = rng.gen_range(10..map_data.height - 10);
            
            // 检查是否可行走
            if !MapUtils::is_walkable(map_data, grid_x, grid_y) {
                continue;
            }
            
            // 转换为世界坐标
            let (x, y) = Coordinates::grid_to_world_center(grid_x, grid_y);
            
            // 随机AI类型
            let ai_type = (spawned % 3) as u8 + 1; // 1=近战, 2=远程, 3=巡逻
            
            // 怪物数据
            let monster_name = match ai_type {
                1 => "骷髅战士",
                2 => "弓箭骷髅",
                3 => "巡逻卫兵",
                _ => "怪物",
            };
            
            // 创建怪物实体
            world.spawn((
                Position { x, y },
                MonsterData {
                    id: spawned as u32 + 1,
                    name: monster_name.to_string(),
                    monster_index: 0,  // 使用第一个怪物模型
                    ai_mode: ai_type,
                    ai_type,
                    spawn_x: x,
                    spawn_y: y,
                },
                AIState::default(),
                Health { 
                    current: 100, 
                    max: 100 
                },
                Animation {
                    action: MirAction::Standing,
                    direction: 0,
                    frame_count: 4,
                    frame_index: 0,
                    frame_interval: 200,
                    frame_timer: 0,
                    loop_animation: true,
                },
                Sprite {
                    library: 0,     // 怪物库索引
                    index: 0,       // 贴图索引
                    frame: 0,       // 当前帧
                    blend_mode: crate::ecs::components::SpriteBlendMode::Alpha,
                },
            ));
            
            spawned += 1;
        }
        
        println!("✅ 成功生成 {} 个测试怪物 (尝试次数: {})", spawned, attempts);
    }
}



