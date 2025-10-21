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
    MapData, MapTile, TileLayer, AnimatedTile, Door, DoorState,
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
}
