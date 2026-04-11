use crate::{
    components::TimeTracker,
    game::{GameContext, GameResult},
    systems::LogicSystem,
};

/// 帧时间推进系统：统一推进 TimeTracker（frame_count / animation_count 等）。
#[derive(ecs_macros::LogicSystem, Default)]
pub struct TimeTickSystem {
    animation_accum: f32,
}

impl LogicSystem for TimeTickSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        let now = std::time::Instant::now();

        // 只维护第一个 TimeTracker（约定：单例）。
        let mut q = ctx.world.query::<&mut TimeTracker>();
        let Some(tt) = q.iter().next() else {
            return Ok(());
        };

        tt.frame_count = tt.frame_count.wrapping_add(1);
        tt.last_frame_time = now;

        // 与旧 GameScene 行为保持一致：每 100ms 推进一次 animation_count。
        self.animation_accum += dt;
        while self.animation_accum >= 0.1 {
            tt.animation_count = tt.animation_count.wrapping_add(1);
            self.animation_accum -= 0.1;
        }

        Ok(())
    }
}
