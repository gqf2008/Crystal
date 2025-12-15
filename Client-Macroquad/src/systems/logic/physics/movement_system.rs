// ============================================================================
// Layer 4: Physics & Movement - MovementSystem
// Priority: 400
// ============================================================================
//
// **职责**：
// - 实体移动更新 (格子对齐系统)
// - 路径跟随
// - 方向计算
// - 到达检测
//
// **逻辑来源**：
// - C# PlayerObject.ProcessFrames(): 移动动画同步 (Line 2424+)
// - C# MapObject: 格子坐标系统 (48x32像素)
// - C# Movement: 当前格子位置
//
// ============================================================================

use crate::game::GameContext;
use crate::game::GameResult;
use crate::components::{Player, Position, movement::{MovementVelocity, Path}};
use crate::systems::LogicSystem;
use mir2_shared::enums::MirDirection;

/// 移动系统 - 实现格子对齐的移动逻辑
#[derive(ecs_macros::LogicSystem)]
pub struct MovementSystem;

/// 格子尺寸常量 (与C#保持一致)
const CELL_WIDTH: f32 = 48.0;
const CELL_HEIGHT: f32 = 32.0;
const ARRIVAL_THRESHOLD: f32 = 5.0; // 到达阈值(像素)

impl MovementSystem {
    /// 根据移动向量计算8方向
    /// 
    /// C# 参考: MapObject.PointToDirection()
    /// ```
    ///     7  0  1
    ///      \ | /
    ///    6 - · - 2
    ///      / | \
    ///     5  4  3
    /// ```
    fn calculate_direction(dx: f32, dy: f32) -> MirDirection {
        use MirDirection::*;
        
        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return Up; // 静止，保持当前方向
        }
        
        let angle = dy.atan2(dx).to_degrees();
        
        // 将角度转换为0-360度
        let normalized_angle = if angle < 0.0 {
            angle + 360.0
        } else {
            angle
        };
        
        // 8方向划分 (每个方向45度)
        // 0度 = 东 (右), 90度 = 南 (下), 180度 = 西 (左), 270度 = 北 (上)
        match normalized_angle as i32 {
            337..=360 | 0..=22 => Right,      // 东 (右) = 2
            23..=67 => DownRight,             // 东南 = 3
            68..=112 => Down,                 // 南 (下) = 4
            113..=157 => DownLeft,            // 西南 = 5
            158..=202 => Left,                // 西 (左) = 6
            203..=247 => UpLeft,              // 西北 = 7
            248..=292 => Up,                  // 北 (上) = 0
            293..=336 => UpRight,             // 东北 = 1
            _ => Up,
        }
    }

}

impl LogicSystem for MovementSystem {
    

    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        // 🎯 处理有Player组件的实体（玩家、NPC等）
        for (_, (position, velocity, path, player, player_input)) in ctx.world.query_mut::<(
            &mut Position,
            &mut MovementVelocity,
            &mut Path,
            &mut Player,
            &mut crate::components::PlayerInput,
        )>() {
            use crate::components::MovementMode;
            
            // 🎯 统一处理所有移动模式的动画状态
            // 检查是否有velocity (移动中)
            let has_velocity = velocity.x.abs() > 0.01 || velocity.y.abs() > 0.01;
            
            if player_input.movement_mode == MovementMode::DirectFollow {
                // DirectFollow模式: 直接用velocity更新位置
                if has_velocity {
                    // 移动中: 更新位置和方向
                    player.direction = Self::calculate_direction(velocity.x, velocity.y);
                    
                    // 🎯 关键修复：只在有velocity时才更新position
                    position.x += velocity.x * delay_time;
                    position.y += velocity.y * delay_time;
                    
                    // 🔥 不再设置 player.action，由 PlayerStateSystem 根据 move_to 决定
                    // 这样碰撞时即使 velocity=0，动画仍会继续
                } else {
                    // 🎯 静止时停止velocity
                    velocity.stop();
                    
                    // 🔥 不再设置 player.action，由 PlayerStateSystem 决定
                }
                continue;
            }
            
            // Pathfinding模式: 只检查path，velocity由MovementSystem自己计算
            if !path.is_valid {
                tracing::warn!("❌ MovementSystem: path无效, 停止 (mode={:?}, waypoints={}, current={}, valid={})", 
                    player_input.movement_mode, path.waypoints.len(), path.current_index, path.is_valid);
                velocity.stop();
                continue;
            }

            if let Some(target) = path.current_waypoint() {
                // 转换格子坐标到像素坐标
                let target_x = target.0 as f32 * CELL_WIDTH;
                let target_y = target.1 as f32 * CELL_HEIGHT;

                // 计算方向和距离
                let dx = target_x - position.x;
                let dy = target_y - position.y;
                let distance = (dx * dx + dy * dy).sqrt();

                // 到达检测
                if distance < ARRIVAL_THRESHOLD {
                    // 对齐到格子中心
                    position.x = target_x;
                    position.y = target_y;
                    
                    // 移动到下一个路径点
                    if !path.advance() {
                        // 🎯 路径结束: 只清理物理状态，不修改 player.action
                        // player.action 由 PlayerControlSystem 独占管理
                        velocity.stop();
                        path.clear();
                        
                        // 清除移动目标，触发 PlayerControlSystem 下一帧设置 Stand
                        use crate::components::MovementMode;
                        player_input.move_to = None;
                        player_input.movement_mode = MovementMode::None;
                        
                        tracing::info!("✅ 到达目的地，清除移动目标");
                    }
                } else {
                    // 🎯 计算8方向
                    player.direction = Self::calculate_direction(dx, dy);
                    
                    // ✅ 根据 Player.action 判断速度（统一数据源）
                    use crate::components::PlayerAction;
                    let speed = if player.action == PlayerAction::Run {
                        velocity.run_speed
                    } else {
                        velocity.walk_speed
                    };
                    
                    // PlayerControlSystem 已设置 player.action，这里只负责移动
                    
                    // 设置速度方向 (归一化)
                    velocity.set((dx / distance) * speed, (dy / distance) * speed);

                    // 应用速度到位置
                    let move_x = velocity.x * delay_time;
                    let move_y = velocity.y * delay_time;
                    position.x += move_x;
                    position.y += move_y;
                    
                    // 调试:打印移动信息
                    tracing::info!(
                        "✅ MovementSystem移动: pos=({:.1},{:.1}) target=({:.1},{:.1}) dist={:.1} dt={:.3} move=({:.2},{:.2})",
                        position.x, position.y, target_x, target_y, distance, delay_time, move_x, move_y
                    );
                }
            }
        }

        // 🎯 处理没有Player组件的通用实体（怪物、道具等）
        // 先收集所有有Player组件的实体ID
        let player_entities: Vec<_> = ctx.world.query::<&Player>()
            .iter()
            .map(|(entity, _)| entity)
            .collect();

        for (entity, (position, velocity, path)) in ctx.world.query_mut::<(
            &mut Position,
            &mut MovementVelocity,
            &mut Path,
        )>() {
            // 跳过已经有Player组件的实体（避免重复处理）
            if player_entities.contains(&entity) {
                continue;
            }
            
            if !path.is_valid {
                velocity.stop();
                continue;
            }

            if let Some(target) = path.current_waypoint() {
                let target_x = target.0 as f32 * CELL_WIDTH;
                let target_y = target.1 as f32 * CELL_HEIGHT;

                let dx = target_x - position.x;
                let dy = target_y - position.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < ARRIVAL_THRESHOLD {
                    position.x = target_x;
                    position.y = target_y;
                    
                    if !path.advance() {
                        velocity.stop();
                    }
                } else {
                    let speed = velocity.max_speed;
                    
                    velocity.set(
                        (dx / distance) * speed,
                        (dy / distance) * speed
                    );

                    position.x += velocity.x * delay_time;
                    position.y += velocity.y * delay_time;
                }
            }
        }

        Ok(())
    }
}
