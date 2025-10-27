// ============================================================================
// Occlusion System - 遮挡检测系统
// ============================================================================
//
// 功能：
// - 检测角色是否被 Front 层瓦片遮挡
// - 动态调整遮挡瓦片的透明度
// - 平滑过渡透明度效果
//
// ============================================================================

use hecs::World;
use crate::ecs::components::*;
use crate::ecs::{Coordinates, CELL_WIDTH, CELL_HEIGHT};

/// 遮挡检测系统
pub struct OcclusionSystem {
    /// 遮挡检测范围（格子）
    check_range_x: i32,
    check_range_y: i32,
    /// 目标透明度
    target_alpha: f32,
    /// 过渡速度
    transition_speed: f32,
}

impl OcclusionSystem {
    pub fn new() -> Self {
        Self {
            check_range_x: 3,      // 左右各检测3格
            check_range_y: 5,      // 向上检测5格
            target_alpha: 0.4,     // 遮挡时的透明度
            transition_speed: 8.0, // 透明度过渡速度（每秒）
        }
    }

    /// 更新遮挡检测
    pub fn update(&self, world: &mut World, delta_time: f32) {
        // 🎯 第一步：收集所有玩家/角色的位置
        let player_positions = self.collect_player_positions(world);

        if player_positions.is_empty() {
            return;
        }

        // 🎯 第二步：检查所有 Front 层瓦片
        self.update_tile_occlusion(world, &player_positions, delta_time);
    }

    /// 收集所有需要检测遮挡的角色位置
    fn collect_player_positions(&self, world: &World) -> Vec<(i32, i32)> {
        let mut positions = Vec::new();

        // 收集玩家位置
        for (_entity, (pos, _player)) in world.query::<(&Position, &Player)>().iter() {
            let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
            positions.push((grid_x, grid_y));
        }

        // TODO: 也可以收集其他角色（NPC、怪物）的位置
        // for (_entity, (pos, _npc)) in world.query::<(&Position, &NPCData)>().iter() {
        //     let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
        //     positions.push((grid_x, grid_y));
        // }

        positions
    }

    /// 更新瓦片的遮挡状态
    fn update_tile_occlusion(
        &self,
        world: &mut World,
        player_positions: &[(i32, i32)],
        delta_time: f32,
    ) {
        // 遍历所有 Front 层瓦片
        for (_entity, (tile, occlusion)) in world.query::<(&MapTile, &mut TileOcclusion)>().iter() {
            if !matches!(tile.layer, TileLayer::Front) {
                continue;
            }

            // 检查是否有玩家被遮挡
            let is_occluding = self.check_tile_occlusion(tile, player_positions);

            // 平滑过渡透明度
            self.update_alpha(occlusion, is_occluding, delta_time);
        }
    }

    /// 检查瓦片是否遮挡任何玩家
    fn check_tile_occlusion(&self, tile: &MapTile, player_positions: &[(i32, i32)]) -> bool {
        for &(player_grid_x, player_grid_y) in player_positions {
            // 计算瓦片相对玩家的偏移
            let dx = (tile.grid_x - player_grid_x).abs();
            let dy = tile.grid_y - player_grid_y;

            // 🎯 遮挡条件：
            // 1. 水平距离 <= check_range_x 格
            // 2. 瓦片在玩家后方（dy > 0，瓦片Y坐标更大，在等距视角的后方）
            // 3. 垂直距离 <= check_range_y 格
            // 
            // 注意：在2D等距视角中，Y坐标越大，位置越靠后（远离屏幕顶部）
            // 所以 dy > 0 表示瓦片在玩家后方，建筑物会遮挡玩家
            if dx <= self.check_range_x && dy > 0 && dy <= self.check_range_y {
                return true;
            }
        }

        false
    }

    /// 平滑更新透明度
    fn update_alpha(&self, occlusion: &mut TileOcclusion, is_occluding: bool, delta_time: f32) {
        let target = if is_occluding {
            self.target_alpha
        } else {
            1.0
        };

        // 平滑插值
        let alpha_diff = target - occlusion.current_alpha;
        occlusion.current_alpha += alpha_diff * self.transition_speed * delta_time;

        // 限制范围
        occlusion.current_alpha = occlusion.current_alpha.clamp(self.target_alpha, 1.0);

        // 更新状态标志
        occlusion.is_occluding = is_occluding;
    }

    /// 获取瓦片的当前透明度
    pub fn get_tile_alpha(world: &World, entity: hecs::Entity) -> f32 {
        if let Ok(occlusion) = world.get::<&TileOcclusion>(entity) {
            occlusion.current_alpha
        } else {
            1.0 // 默认不透明
        }
    }

    /// 配置参数
    pub fn set_check_range(&mut self, range_x: i32, range_y: i32) {
        self.check_range_x = range_x;
        self.check_range_y = range_y;
    }

    pub fn set_target_alpha(&mut self, alpha: f32) {
        self.target_alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn set_transition_speed(&mut self, speed: f32) {
        self.transition_speed = speed.max(0.1);
    }
}

impl Default for OcclusionSystem {
    fn default() -> Self {
        Self::new()
    }
}
