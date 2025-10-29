// ============================================================================
// Layer 6: Network Sync - ClientPredictionSystem  
// Priority: 595 (在NetworkSendSystem之前)
// ============================================================================
//
// **职责**：
// - 客户端位置预测
// - 服务器位置校正
// - 预测误差修正
//
// **逻辑来源**：
// - C# GameScene.UserLocation(): 服务器校正位置 (Line 2637+)
// - 校正阈值: >2格(96像素)时强制校正
// - 修正速度: 30% (correction_speed = 0.3)
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::systems::{System, priority};
use crate::ecs::components::{Position, PredictionState};

/// 客户端预测系统
/// 
/// 预测机制:
/// 1. 记录客户端预测的位置
/// 2. 接收服务器权威位置
/// 3. 如果偏差>2格(96像素),进行平滑校正
/// 4. 校正速度为30%(每帧移动30%距离)
pub struct ClientPredictionSystem;

impl System for ClientPredictionSystem {
    fn priority(&self) -> u32 {
        priority::NETWORK_SEND - 5 // 595: 在网络发送之前执行
    }

    fn update(&mut self, world: &mut World, delay_time: f32) -> GameResult {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        const CELL_WIDTH: f32 = 48.0;
        const CELL_HEIGHT: f32 = 32.0;
        const CORRECTION_THRESHOLD: f32 = 96.0; // 2格 = 96像素

        // 处理位置校正
        for (_id, (position, prediction)) in world.query_mut::<(&mut Position, &mut PredictionState)>() {
            // 清理过期的预测记录(超过1秒)
            prediction.cleanup_old_predictions(current_time, 1000);

            // 检查是否需要校正
            if let Some(correction_target) = prediction.correction_target {
                let target_x = correction_target.0 as f32 * CELL_WIDTH;
                let target_y = correction_target.1 as f32 * CELL_HEIGHT;

                // 计算当前位置与目标的距离
                let dx = target_x - position.x;
                let dy = target_y - position.y;
                let distance = (dx * dx + dy * dy).sqrt();

                // 如果距离大于阈值,执行平滑校正
                if distance > CORRECTION_THRESHOLD {
                    // 平滑插值: 每帧移动30%距离
                    let correction_amount = prediction.correction_speed * delay_time * 60.0; // 60fps标准
                    let move_distance = distance * correction_amount;

                    if move_distance < distance {
                        // 按方向移动
                        let ratio = move_distance / distance;
                        position.x += dx * ratio;
                        position.y += dy * ratio;
                    } else {
                        // 直接到达目标
                        position.x = target_x;
                        position.y = target_y;
                        prediction.clear_correction_target();
                    }
                } else {
                    // 距离小于阈值,直接修正
                    position.x = target_x;
                    position.y = target_y;
                    prediction.clear_correction_target();
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
    fn test_position_correction() {
        let mut world = World::new();
        let mut system = ClientPredictionSystem;

        let mut prediction = PredictionState::new();
        prediction.set_correction_target((5, 5)); // 目标格子位置(5,5)
        prediction.correction_speed = 0.3;

        let entity = world.spawn((
            Position { x: 0.0, y: 0.0 }, // 起始位置(0,0)
            prediction,
        ));

        // 目标位置: (5*48, 5*32) = (240, 160)
        // 距离: sqrt(240^2 + 160^2) = 288 > 96 (需要校正)

        system.update(&mut world, 0.016).unwrap(); // 1帧

        let position = world.get::<&Position>(entity).unwrap();
        // 应该向目标移动了一定距离
        assert!(position.x > 0.0);
        assert!(position.y > 0.0);
        assert!(position.x < 240.0);
        assert!(position.y < 160.0);
    }

    #[test]
    fn test_small_correction() {
        let mut world = World::new();
        let mut system = ClientPredictionSystem;

        let mut prediction = PredictionState::new();
        prediction.set_correction_target((2, 1)); // 目标(2,1) = (96, 32)

        let entity = world.spawn((
            Position { x: 90.0, y: 30.0 }, // 距离目标很近
            prediction,
        ));

        system.update(&mut world, 0.016).unwrap();

        let position = world.get::<&Position>(entity).unwrap();
        let prediction = world.get::<&PredictionState>(entity).unwrap();
        
        // 距离<96,应该直接修正
        assert!((position.x - 96.0).abs() < 0.01);
        assert!((position.y - 32.0).abs() < 0.01);
        assert!(prediction.correction_target.is_none());
    }

    #[test]
    fn test_no_correction_needed() {
        let mut world = World::new();
        let mut system = ClientPredictionSystem;

        let prediction = PredictionState::new(); // 无校正目标

        let entity = world.spawn((
            Position { x: 100.0, y: 100.0 },
            prediction,
        ));

        system.update(&mut world, 0.016).unwrap();

        let position = world.get::<&Position>(entity).unwrap();
        // 位置不应该改变
        assert_eq!(position.x, 100.0);
        assert_eq!(position.y, 100.0);
    }
}

