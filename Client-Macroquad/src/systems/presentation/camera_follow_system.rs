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

use crate::components::{Camera, CameraMode, LocalPlayer, Position};
use crate::systems::LogicSystem;
use crate::compat::GameContext;
use crate::compat::GameResult;

/// 摄像机跟随系统
#[derive(ecs_macros::LogicSystem)]
pub struct CameraFollowSystem;

impl LogicSystem for CameraFollowSystem {
    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        // 获取玩家位置
        let player_pos = {
            let mut pos = None;
            for (_, (_, player_pos)) in ctx.world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };

        // 如果找到玩家,更新摄像机位置（仅在 FollowPlayer 模式下）
        if let Some((player_x, player_y)) = player_pos {
            for (_, (camera_pos, _camera, mode)) in ctx
                .world
                .query_mut::<(&mut Position, &Camera, &CameraMode)>()
            {
                // 仅在跟随模式下更新相机位置
                if *mode == CameraMode::FollowPlayer {
                    // 🎯 平滑跟随 (线性插值 lerp)
                    // lerp_factor越大跟随越快,但太大会丢失平滑效果
                    // 建议值: 5.0-15.0 之间
                    let lerp_factor = 10.0;
                    let smooth_speed = lerp_factor * delay_time;

                    // 限制smooth_speed最大为1.0,避免相机超过目标
                    let t = smooth_speed.min(1.0);

                    // 线性插值: camera = camera + (player - camera) * t
                    camera_pos.x += (player_x - camera_pos.x) * t;
                    camera_pos.y += (player_y - camera_pos.y) * t;
                }
            }
        }

        Ok(())
    }
}
