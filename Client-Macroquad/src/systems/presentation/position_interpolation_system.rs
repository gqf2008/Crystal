use macroquad::prelude::get_time;

use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;

/// 平滑远程位置更新：将 `Position` 插值到服务器下发的目标坐标。
#[derive(Default, ecs_macros::LogicSystem)]
pub struct PositionInterpolationSystem;

impl LogicSystem for PositionInterpolationSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        use crate::components::{Position, PositionInterpolation};

        let now = get_time();
        let mut done: Vec<hecs::Entity> = Vec::new();

        for eref in ctx.world.iter() {
            let (Some(mut pos), Some(interp)) = (
                eref.get::<&mut Position>(),
                eref.get::<&PositionInterpolation>(),
            ) else {
                continue;
            };
            let duration = interp.duration.max(0.0001) as f64;
            let t = ((now - interp.start_time) / duration).clamp(0.0, 1.0) as f32;

            pos.x = interp.start_x + (interp.target_x - interp.start_x) * t;
            pos.y = interp.start_y + (interp.target_y - interp.start_y) * t;

            if t >= 1.0 {
                pos.x = interp.target_x;
                pos.y = interp.target_y;
                done.push(eref.entity());
            }
        }

        for e in done {
            let _ = ctx.world.remove_one::<PositionInterpolation>(e);
        }

        Ok(())
    }
}
