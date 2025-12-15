use crate::{
    components::{Camera, MapData, Position},
    game::{GameContext, GameResult},
    systems::LogicSystem,
};

/// 将相机位置限制在地图边界内（ECS 版本的 clamp_map_camera_position）。
#[derive(ecs_macros::LogicSystem, Default)]
pub struct CameraBoundsSystem;

impl LogicSystem for CameraBoundsSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let mut map_q = ctx.world.query::<&MapData>();
        let Some((_map_e, map)) = map_q.iter().next() else {
            return Ok(());
        };

        let (sw, sh) = ctx.drawable_size();

        // 取第一个 Camera + Position（约定：单例相机）
        let mut q = ctx.world.query::<(&mut Position, &Camera)>();
        let Some((_e, (pos, cam))) = q.iter().next() else {
            return Ok(());
        };

        let zoom = cam.zoom.max(0.0001);
        let half_w = (sw.max(1.0) / 2.0) / zoom;
        let half_h = (sh.max(1.0) / 2.0) / zoom;

        let map_w = map.width as f32 * 48.0;
        let map_h = map.height as f32 * 32.0;

        // 若视口比地图大，直接居中
        pos.x = if map_w <= half_w * 2.0 {
            map_w / 2.0
        } else {
            pos.x.clamp(half_w, map_w - half_w)
        };

        pos.y = if map_h <= half_h * 2.0 {
            map_h / 2.0
        } else {
            pos.y.clamp(half_h, map_h - half_h)
        };

        Ok(())
    }
}
