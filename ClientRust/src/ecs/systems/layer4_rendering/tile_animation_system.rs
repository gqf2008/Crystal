// ============================================================================
// Tile Animation System - 地图瓦片动画系统 (Layer 4)
// ============================================================================
//
// 职责：
// - 更新地图瓦片动画（水流、火焰、岩浆等）
// - 根据全局动画计数器计算瓦片帧偏移
// - 纯渲染层逻辑，不包含游戏规则
//
// 替代：
// - deprecated/AnimationSystem::update_tiles()
//
// ============================================================================

use hecs::World;
use crate::ecs::components::{MapTile, AnimatedTile};

/// 地图瓦片动画系统
pub struct TileAnimationSystem;

impl TileAnimationSystem {
    /// 更新地图瓦片动画
    /// 
    /// # 参数
    /// - world: ECS世界
    /// - animation_count: 全局动画计数器（每帧递增）
    /// 
    /// # 动画计算公式
    /// ```text
    /// total_frames = frame_count + (frame_count * frame_interval)
    /// frame_offset = (animation_count % total_frames) / (1 + frame_interval)
    /// final_image_index = base_image_index + frame_offset
    /// ```
    /// 
    /// # 示例
    /// - frame_count = 4, frame_interval = 2
    /// - total_frames = 4 + 8 = 12
    /// - animation_count = 0-2: frame 0
    /// - animation_count = 3-5: frame 1
    /// - animation_count = 6-8: frame 2
    /// - animation_count = 9-11: frame 3
    /// - animation_count = 12: 循环回 frame 0
    pub fn update(world: &mut World, animation_count: i32) {
        for (_entity, (tile, anim)) in world.query_mut::<(&mut MapTile, &AnimatedTile)>() {
            // 计算总帧数（包括间隔）
            let total_frames = anim.frame_count as i32 
                + (anim.frame_count as i32 * anim.frame_interval as i32);
            
            // 计算当前应该显示的帧偏移
            let frame_offset = (animation_count % total_frames) / (1 + anim.frame_interval as i32);
            
            // 更新瓦片图像索引
            tile.image_index = anim.base_image_index + frame_offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tile_animation_cycle() {
        let mut world = World::new();
        
        // 创建一个4帧的瓦片动画，间隔为2
        world.spawn((
            MapTile { 
                image_index: 100, 
                file_index: 0, 
                x: 0, 
                y: 0, 
                layer: 0 
            },
            AnimatedTile {
                base_image_index: 100,
                frame_count: 4,
                frame_interval: 2,
            },
        ));
        
        // animation_count = 0: frame 0
        TileAnimationSystem::update(&mut world, 0);
        for (_, tile) in world.query_mut::<&MapTile>() {
            assert_eq!(tile.image_index, 100);
        }
        
        // animation_count = 3: frame 1
        TileAnimationSystem::update(&mut world, 3);
        for (_, tile) in world.query_mut::<&MapTile>() {
            assert_eq!(tile.image_index, 101);
        }
        
        // animation_count = 6: frame 2
        TileAnimationSystem::update(&mut world, 6);
        for (_, tile) in world.query_mut::<&MapTile>() {
            assert_eq!(tile.image_index, 102);
        }
        
        // animation_count = 12: 循环回 frame 0
        TileAnimationSystem::update(&mut world, 12);
        for (_, tile) in world.query_mut::<&MapTile>() {
            assert_eq!(tile.image_index, 100);
        }
    }
}
