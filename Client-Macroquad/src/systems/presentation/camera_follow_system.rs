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
use crate::game::GameContext;
use crate::game::GameResult;
use crate::systems::LogicSystem;

/// 摄像机跟随系统
#[derive(ecs_macros::LogicSystem)]
pub struct CameraFollowSystem;

impl LogicSystem for CameraFollowSystem {
    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        // 获取玩家位置
        let player_pos = {
            let mut pos = None;
            for (_, player_pos) in ctx.world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };

        // 如果找到玩家,更新摄像机位置（仅在 FollowPlayer 模式下）
        if let Some((player_x, player_y)) = player_pos {
            for (camera_pos, camera, mode) in ctx
                .world
                .query_mut::<(&mut Position, &Camera, &CameraMode)>()
            {
                // 仅在跟随模式下更新相机位置
                if *mode != CameraMode::FollowPlayer {
                    continue;
                }

                // 首次进入游戏时，相机可能还在 (0,0) 或与玩家相距非常远。
                // 此时如果只做 lerp，会导致很长时间看不到角色（甚至被裁剪系统剔除），
                // 体感上像是“人物/装备/坐骑都没了”。
                let far_enough = (player_x - camera_pos.x).abs()
                    > camera.screen_width.max(1.0) * 6.0
                    || (player_y - camera_pos.y).abs() > camera.screen_height.max(1.0) * 6.0;

                // 只要相机和玩家距离异常大，就直接对齐。
                // 这能覆盖：
                // - 相机初始化在地图中心，但玩家出生点很远
                // - 服务器/Mock 下发位置后，首帧视野不在玩家附近
                // - 大跨度传送（可接受直接跳转视野）
                if far_enough {
                    camera_pos.x = player_x;
                    camera_pos.y = player_y;
                    continue;
                }

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

        Ok(())
    }
}
