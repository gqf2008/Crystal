use crate::components::network::Lifetime;
use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;

#[derive(ecs_macros::LogicSystem, Default)]
pub struct LifetimeCleanupSystem;

impl LogicSystem for LifetimeCleanupSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        let delta_ms = (dt.max(0.0) * 1000.0) as u32;
        if delta_ms == 0 {
            return Ok(());
        }

        let mut to_despawn: Vec<hecs::Entity> = Vec::new();
        for (e, lt) in ctx.world.query_mut::<&mut Lifetime>().into_iter() {
            if lt.update(delta_ms) {
                to_despawn.push(e);
            }
        }

        for e in to_despawn {
            let _ = ctx.world.despawn(e);
        }

        Ok(())
    }
}
