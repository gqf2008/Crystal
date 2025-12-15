use crate::{
    game::{GameContext, GameResult},
    systems::LogicSystem,
};

/// 帧结束系统：清理死亡实体并清空 EventBus 本帧事件。
#[derive(ecs_macros::LogicSystem, Default)]
pub struct FrameEndSystem;

impl LogicSystem for FrameEndSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        ctx.cleanup_dead_entities();
        ctx.events_mut().clear_frame();
        Ok(())
    }
}
