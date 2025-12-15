use crate::{
    components::{CameraMode, RenderConfig},
    game::{GameContext, GameResult, KeyCode},
    systems::LogicSystem,
};

/// Space 按下时启用相机拖拽/缩放并切到 Manual；松开时回到 FollowPlayer。
///
/// 说明：这是当前 GameScene 的交互约定（Space+拖拽/滚轮=地图），
/// 通过 ECS 系统化后，Scene.update 不需要再手动切相机模式。
#[derive(ecs_macros::LogicSystem, Default)]
pub struct CameraSpaceGateSystem;

impl LogicSystem for CameraSpaceGateSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let space_down = ctx.input().key_down(KeyCode::Space);

        // 1) RenderConfig：决定 CameraSystem 是否允许拖拽/缩放。
        let mut cfg_q = ctx.world.query::<&mut RenderConfig>();
        if let Some((_e, cfg)) = cfg_q.iter().next() {
            cfg.enable_camera_drag = space_down;
        }

        // 2) CameraMode：Space 按住 = Manual；否则 = FollowPlayer。
        let mut mode_q = ctx.world.query::<&mut CameraMode>();
        if let Some((_e, mode)) = mode_q.iter().next() {
            *mode = if space_down {
                CameraMode::Manual
            } else {
                CameraMode::FollowPlayer
            };
        }

        Ok(())
    }
}
