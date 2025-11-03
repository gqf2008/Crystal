// ============================================================================
// Layer 5: State Update - TileAnimationSystem
// Priority: 505
// ============================================================================
//
// **职责**：
// - 更新地图瓦片动画帧
// - 根据时间和帧数循环播放动画
//
// **逻辑来源**：
// - C# DrawObjects(): 动画瓦片帧计算
//   - index += (AnimationCount % (animation + (animation * animationTick))) / (1 + animationTick);
//   - AnimationCount 是全局计数器，每帧递增
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::GameContext;
use crate::ecs::components::{AnimatedTile, MapTile, RenderConfig};
use crate::ecs::systems::{System, priority};

/// 瓦片动画系统
/// 
/// 负责更新所有动画瓦片的当前帧索引
pub struct TileAnimationSystem {
    /// 全局动画计数器（模拟 C# 的 AnimationCount）
    animation_counter: u32,
    /// 累积时间（秒）
    accumulated_time: f32,
    /// 每次递增计数器需要的时间（秒）
    counter_interval: f32,
}

impl TileAnimationSystem {
    pub fn new() -> Self {
        Self {
            animation_counter: 0,
            accumulated_time: 0.0,
            counter_interval: 1.0 / 60.0, // 60 FPS 基准
        }
    }

    /// 计算动画帧偏移
    /// 
    /// C# 逻辑：
    /// ```csharp
    /// index += (AnimationCount % (animation + (animation * animationTick))) / (1 + animationTick);
    /// ```
    fn calculate_frame_offset(&self, frame_count: u8, frame_interval: u8) -> i32 {
        if frame_count == 0 {
            return 0;
        }

        let total_ticks = (frame_count as u32 + frame_count as u32 * frame_interval as u32);
        let divisor = 1 + frame_interval as u32;
        
        ((self.animation_counter % total_ticks) / divisor) as i32
    }
}

impl Default for TileAnimationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for TileAnimationSystem {
    fn priority(&self) -> u32 {
        priority::ANIMATION + 5 // 505, 稍微晚于角色动画
    }

    fn update(&mut self, ctx: &mut GameContext, delta_time: f32) -> GameResult {
        // 检查是否启用动画
        let animations_enabled = {
            let mut config_query = ctx.world.query::<&RenderConfig>();
            config_query
                .iter()
                .next()
                .map(|(_, cfg)| cfg.show_animations)
                .unwrap_or(true) // 默认启用
        };

        // 如果动画被禁用，直接返回
        if !animations_enabled {
            return Ok(());
        }

        // 累积时间
        self.accumulated_time += delta_time;

        // 每个计数器间隔递增计数器
        while self.accumulated_time >= self.counter_interval {
            self.animation_counter = self.animation_counter.wrapping_add(1);
            self.accumulated_time -= self.counter_interval;
        }

        // 更新所有动画瓦片的 image_index
        for (_, (tile, anim)) in ctx.world.query_mut::<(&mut MapTile, &AnimatedTile)>() {
            let frame_offset = self.calculate_frame_offset(anim.frame_count, anim.frame_interval);
            tile.image_index = anim.base_image_index + frame_offset;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_calculation() {
        let system = TileAnimationSystem::new();
        
        // 4帧，间隔1
        let offset = system.calculate_frame_offset(4, 1);
        assert!(offset >= 0 && offset < 4);
    }

    #[test]
    fn test_counter_wrapping() {
        let mut system = TileAnimationSystem::new();
        system.animation_counter = u32::MAX;
        
        // 应该正常溢出
        system.animation_counter = system.animation_counter.wrapping_add(1);
        assert_eq!(system.animation_counter, 0);
    }
}
