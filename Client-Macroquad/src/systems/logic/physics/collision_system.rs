// ============================================================================
// Layer 4: Physics & Movement - CollisionSystem
// Priority: 410 (在MovementSystem之后执行,检查并修正位置)
// ============================================================================
//
// **职责**：
// - 碰撞检测 (检查MovementSystem移动后的位置)
// - 地图边界限制
// - 停止velocity以阻止继续移动到障碍物
//
// **逻辑来源**：
// - C# MapControl.ValidPoint(): 检查是否可移动
//   - (M2CellInfo[x, y].BackImage & 0x20000000) == 0
// - Map.HasTarget(): 检查是否有阻挡实体
//
// ============================================================================

use crate::game::GameResult;
use crate::game::GameContext;
use crate::components::{Position, movement::{MovementVelocity, Path}, map::MapBounds};
use crate::systems::LogicSystem;

/// 碰撞检测系统
#[derive(ecs_macros::LogicSystem)]
pub struct CollisionSystem {
    /// 停止移动阈值(像素)
    #[allow(dead_code)]
    stop_threshold: f32,
}

impl CollisionSystem {
    pub fn new() -> Self {
        Self {
            stop_threshold: 1.0,
        }
    }

    /// 检查位置是否在地图边界内
    #[allow(dead_code)]
    fn is_within_bounds(x: f32, y: f32, bounds: &MapBounds) -> bool {
        x >= 0.0 && y >= 0.0 && x < bounds.width as f32 && y < bounds.height as f32
    }

    /// 将位置限制在地图边界内
    #[allow(dead_code)]
    fn clamp_to_bounds(pos: &mut Position, bounds: &MapBounds) {
        pos.x = pos.x.clamp(0.0, (bounds.width - 1) as f32);
        pos.y = pos.y.clamp(0.0, (bounds.height - 1) as f32);
    }

    fn is_walkable(cells: &Vec<Vec<crate::resources::map_reader::CellInfo>>, width: i32, height: i32, gx: i32, gy: i32) -> bool {
        if gx < 0 || gy < 0 || gx >= width || gy >= height {
            return false;
        }
        let x = gx as usize;
        let y = gy as usize;
        if x >= cells.len() || y >= cells[x].len() {
            return false;
        }
        cells[x][y].is_walkable()
    }

    fn nearest_walkable(
        cells: &Vec<Vec<crate::resources::map_reader::CellInfo>>,
        width: i32,
        height: i32,
        target: (i32, i32),
        max_radius: i32,
    ) -> Option<(i32, i32)> {
        if Self::is_walkable(cells, width, height, target.0, target.1) {
            return Some(target);
        }

        for r in 1..=max_radius {
            for dx in -r..=r {
                for dy in [-r, r] {
                    let gx = target.0 + dx;
                    let gy = target.1 + dy;
                    if Self::is_walkable(cells, width, height, gx, gy) {
                        return Some((gx, gy));
                    }
                }
            }
            for dy in (-r + 1)..=(r - 1) {
                for dx in [-r, r] {
                    let gx = target.0 + dx;
                    let gy = target.1 + dy;
                    if Self::is_walkable(cells, width, height, gx, gy) {
                        return Some((gx, gy));
                    }
                }
            }
        }

        None
    }
}

impl Default for CollisionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicSystem for CollisionSystem {
    

    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        use crate::components::map::MapData;
        
        // 获取地图数据
        let (map_width, map_height, map_cells) = {
            let mut width = None;
            let mut height = None;
            let mut cells = None;
            
            for (_, map_data) in ctx.world.query::<&MapData>().iter() {
                width = Some(map_data.width);
                height = Some(map_data.height);
                cells = Some(map_data.cells.clone());
                break;
            }
            
            (width, height, cells)
        };

        let Some(width) = map_width else { return Ok(()); };
        let Some(height) = map_height else { return Ok(()); };
        let Some(cells) = map_cells else { return Ok(()); };

        // 检查移动方向的下一个格子是否有障碍物
        // 重要：MovementSystem 已经更新了 position，本系统负责“阻止进一步进入阻挡”和“把已经进入阻挡的情况拉回”。
        use crate::components::{MovementMode, PlayerInput, Player};

        for (_entity, (pos, vel, player_input, path, _player)) in ctx.world.query_mut::<(
            &mut Position,
            &mut MovementVelocity,
            Option<&mut PlayerInput>,
            Option<&mut Path>,
            Option<&Player>,
        )>() {
            // 🎯 检查velocity是否为零或接近零
            // 注意: 如果velocity为零,说明没有移动,不需要检查碰撞
            if vel.x.abs() < 0.01 && vel.y.abs() < 0.01 {
                continue;
            }
            
            // 记录是否为玩家（用于日志输出）
            let is_player = player_input.is_some();
            
            // 1) 若“当前位置已经在阻挡格子里”，说明上一帧已经穿进去了。
            //    这种情况必须立刻拉回，否则会出现“进房子出不来”。
            let (cur_gx, cur_gy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
            if !Self::is_walkable(&cells, width, height, cur_gx, cur_gy) {
                // 尝试回滚到上一帧位置（假设 MovementSystem 的更新是 pos += vel * dt）
                let prev_x = pos.x - vel.x * delay_time;
                let prev_y = pos.y - vel.y * delay_time;
                let (prev_gx, prev_gy) = crate::coord::Coord::world_to_grid(prev_x, prev_y);

                if Self::is_walkable(&cells, width, height, prev_gx, prev_gy) {
                    pos.x = prev_x;
                    pos.y = prev_y;
                } else if let Some((gx, gy)) = Self::nearest_walkable(&cells, width, height, (cur_gx, cur_gy), 8) {
                    let (wx, wy) = crate::coord::Coord::grid_to_world_center(gx, gy);
                    pos.x = wx;
                    pos.y = wy;
                }

                vel.stop();
                if let Some(p) = path {
                    p.clear();
                }
                if let Some(input) = player_input {
                    // 进入阻挡属于严重异常：直接停止自动移动，避免继续顶墙/越陷越深
                    input.move_to = None;
                    input.movement_mode = MovementMode::None;
                    input.run = false;
                }

                if is_player {
                    tracing::warn!(
                        "🛑 位置落在阻挡格，已回滚/拉回：grid=({}, {}) pos=({:.1},{:.1})",
                        cur_gx,
                        cur_gy,
                        pos.x,
                        pos.y
                    );
                }
                continue;
            }

            // 2) 预测下一帧的位置（用于提前阻止进入阻挡格）
            let next_x = pos.x + vel.x * delay_time;
            let next_y = pos.y + vel.y * delay_time;

            // 使用统一的 world_to_grid（不要手写 /48 /32，避免边界误差）
            let (grid_x, grid_y) = crate::coord::Coord::world_to_grid(next_x, next_y);
            
            // 边界检查
            if grid_x < 0 || grid_y < 0 || grid_x >= width || grid_y >= height {
                vel.stop();
                if let Some(p) = path {
                    p.clear();
                }
                if let Some(input) = player_input {
                    // 边界外直接停掉移动指令
                    input.move_to = None;
                    input.movement_mode = MovementMode::None;
                    input.run = false;
                }
                tracing::warn!(
                    "🛑 边界碰撞：停止移动 - NextGrid({}, {}), CurPos({:.1}, {:.1})",
                    grid_x,
                    grid_y,
                    pos.x,
                    pos.y
                );
                continue;
            }

            // 数组范围检查
            if (grid_x as usize) >= cells.len() || (grid_y as usize) >= cells[grid_x as usize].len() {
                continue;
            }

            // 检查下一个位置的格子是否有障碍物
            let cell = &cells[grid_x as usize][grid_y as usize];
            let has_obstacle = !cell.is_walkable();
            
            if has_obstacle {
                // 下一个位置有障碍物：立即停止，并清掉当前 path，避免继续沿错误路径推进。
                vel.stop();

                if let Some(p) = path {
                    p.clear();
                }

                if let Some(input) = player_input {
                    match input.movement_mode {
                        MovementMode::Pathfinding => {
                            // 保留 move_to：下一帧 PathfindingSystem 会重新算路（绕开障碍）
                        }
                        MovementMode::DirectFollow => {
                            // DirectFollow：避免一直“顶墙”导致抖动，直接停止跟随
                            input.move_to = None;
                            input.movement_mode = MovementMode::None;
                            input.run = false;
                        }
                        MovementMode::None => {}
                    }
                }

                if is_player {
                    tracing::warn!("🛑 碰撞！停止velocity但保持动画 - NextGrid({}, {}), CurPos({:.1}, {:.1})", 
                                   grid_x, grid_y, pos.x, pos.y);
                }
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
