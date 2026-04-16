use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;

/// HUD system placeholder - displays health bar, status bar, quick bar
#[derive(ecs_macros::LogicSystem)]
pub struct HUDSystem;

impl LogicSystem for HUDSystem {
    fn update(&mut self, _ctx: &mut GameContext, _dt: f32) -> GameResult {
        // HUD (health bar, status bar, quick bar) renders via UIRenderSystem;
        // this LogicSystem exists as a scheduling placeholder for future state updates.
        Ok(())
    }
}

impl Default for HUDSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl HUDSystem {
    pub fn new() -> Self {
        Self
    }
}
