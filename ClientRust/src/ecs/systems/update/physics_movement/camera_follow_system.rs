// ============================================================================
// Layer 4: Physics & Movement - CameraFollowSystem
// Priority: 420
// ============================================================================
//
// **职责**：
// - 摄像机跟随玩家逻辑
// - 目标追踪和平滑移动
// - 边界限制
//
// **数据流**：
// - 读取: LocalPlayer + Position (玩家位置)
// - 写入: Camera + Position (摄像机位置)
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::components::{LocalPlayer, Position, Camera};
use crate::ecs::systems::{System, priority};

/// 摄像机跟随系统
pub struct CameraFollowSystem;

impl System for CameraFollowSystem {
    fn priority(&self) -> u32 {
        priority::CAMERA_FOLLOW
    }

    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        // 获取玩家位置
        let player_pos = {
            let mut pos = None;
            for (_, (_, player_pos)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };

        // 如果找到玩家,更新摄像机位置
        if let Some((player_x, player_y)) = player_pos {
            for (_, (camera_pos, _camera)) in world.query_mut::<(&mut Position, &Camera)>() {
                // 直接跟随(可以后续改为平滑跟随)
                camera_pos.x = player_x;
                camera_pos.y = player_y;
            }
        }

        Ok(())
    }
}
