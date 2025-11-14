use crate::systems::RenderSystem;
use crate::game::GameResult;

#[derive(ecs_macros::RenderSystem)]
pub struct EffectRenderSystem;

impl RenderSystem for EffectRenderSystem {
    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        // TODO: 实现特效渲染
        Ok(())
    }
}