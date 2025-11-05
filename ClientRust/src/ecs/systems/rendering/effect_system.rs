use crate::ecs::{GameWorld, systems::RenderSystem};
use ggez::{GameResult, graphics::GraphicsContext};

#[derive(ecs_macros::RenderSystem)]
pub struct EffectRenderSystem;

impl RenderSystem for EffectRenderSystem {
    fn draw(
        &mut self,
        _ctx: &mut GraphicsContext,
        _canvas: &mut ggez::graphics::Canvas,
        _world: &GameWorld,
    ) -> GameResult {
        // 在这里实现地图渲染逻辑
        Ok(())
    }
}