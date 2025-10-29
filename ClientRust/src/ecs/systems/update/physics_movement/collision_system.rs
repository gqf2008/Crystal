// ============================================================================
// Layer 4: Physics & Movement - CollisionSystem
// Priority: 410
// ============================================================================
//
// **职责**：
// - 碰撞检测
// - 地图边界限制
// - 实体间碰撞
//
// **逻辑来源**：
// - C# MapControl.ValidPoint(): 检查是否可移动
//   - (M2CellInfo[x, y].BackImage & 0x20000000) == 0
// - Map.HasTarget(): 检查是否有阻挡实体
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::components::{Position, movement::MovementVelocity, map::MapBounds};
use crate::ecs::systems::{System, priority};

/// 碰撞检测系统
pub struct CollisionSystem {
    /// 停止移动阈值(像素)
    stop_threshold: f32,
}

impl CollisionSystem {
    pub fn new() -> Self {
        Self {
            stop_threshold: 1.0,
        }
    }

    /// 检查位置是否在地图边界内
    fn is_within_bounds(x: f32, y: f32, bounds: &MapBounds) -> bool {
        x >= 0.0 && y >= 0.0 && x < bounds.width as f32 && y < bounds.height as f32
    }

    /// 将位置限制在地图边界内
    fn clamp_to_bounds(pos: &mut Position, bounds: &MapBounds) {
        pos.x = pos.x.clamp(0.0, (bounds.width - 1) as f32);
        pos.y = pos.y.clamp(0.0, (bounds.height - 1) as f32);
    }
}

impl Default for CollisionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for CollisionSystem {
    fn priority(&self) -> u32 {
        priority::COLLISION
    }

    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        // 获取地图边界
        let map_bounds = {
            let mut bounds = None;
            for (_, map_bounds) in world.query::<&MapBounds>().iter() {
                bounds = Some(MapBounds {
                    width: map_bounds.width,
                    height: map_bounds.height,
                });
                break;
            }
            bounds
        };

        // 如果没有地图边界,跳过碰撞检测
        let Some(bounds) = map_bounds else {
            return Ok(());
        };

        // 收集所有实体的位置(用于实体间碰撞检测)
        let mut entity_positions: Vec<(hecs::Entity, f32, f32)> = Vec::new();
        for (entity, pos) in world.query::<&Position>().iter() {
            entity_positions.push((entity, pos.x, pos.y));
        }

        // 检测并修正碰撞
        for (entity, (pos, vel)) in world.query_mut::<(&mut Position, &mut MovementVelocity)>() {
            // 1. 检查地图边界碰撞
            if !Self::is_within_bounds(pos.x, pos.y, &bounds) {
                // 越界,限制到边界内并停止移动
                Self::clamp_to_bounds(pos, &bounds);
                vel.stop();
                continue;
            }

            // 2. 检查是否即将越界
            let future_x = pos.x + vel.x * 0.1; // 预测未来位置
            let future_y = pos.y + vel.y * 0.1;
            
            if !Self::is_within_bounds(future_x, future_y, &bounds) {
                // 即将越界,停止向边界方向的移动
                if future_x < 0.0 || future_x >= bounds.width as f32 {
                    vel.x = 0.0;
                }
                if future_y < 0.0 || future_y >= bounds.height as f32 {
                    vel.y = 0.0;
                }
            }

            // 3. 实体间碰撞检测(简化版 - 可选)
            // 注: 传奇中怪物可以重叠,只有玩家和NPC不能重叠
            // 这里实现简化版,检测距离过近的实体
            const MIN_DISTANCE: f32 = 32.0; // 最小距离(像素)
            
            for &(other_entity, other_x, other_y) in &entity_positions {
                if entity == other_entity {
                    continue;
                }

                let dx = pos.x - other_x;
                let dy = pos.y - other_y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < MIN_DISTANCE * MIN_DISTANCE && dist_sq > 0.1 {
                    // 距离过近,推开
                    let dist = dist_sq.sqrt();
                    let push_strength = (MIN_DISTANCE - dist) * 0.5;
                    
                    pos.x += (dx / dist) * push_strength;
                    pos.y += (dy / dist) * push_strength;

                    // 限制回地图内
                    Self::clamp_to_bounds(pos, &bounds);
                }
            }

            // 4. 如果速度很小,完全停止(避免微小抖动)
            if vel.magnitude() < self.stop_threshold {
                vel.stop();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_check() {
        let bounds = MapBounds { width: 100, height: 100 };
        
        assert!(CollisionSystem::is_within_bounds(50.0, 50.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(-1.0, 50.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(50.0, -1.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(100.0, 50.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(50.0, 100.0, &bounds));
    }

    #[test]
    fn test_clamp_to_bounds() {
        let bounds = MapBounds { width: 100, height: 100 };
        let mut pos = Position { x: -10.0, y: 110.0 };
        
        CollisionSystem::clamp_to_bounds(&mut pos, &bounds);
        
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 99.0);
    }
}
