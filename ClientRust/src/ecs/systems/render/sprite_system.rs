use crate::ecs::systems::RenderSystem;
use ggez::{graphics::GraphicsContext, GameResult};

#[derive(ecs_macros::RenderSystem)]
pub struct SpriteRenderSystem;


impl RenderSystem for SpriteRenderSystem {
    fn draw(
        &mut self,
        _ctx: &mut GraphicsContext,
        _canvas: &mut ggez::graphics::Canvas,
        _world: &hecs::World,
    ) -> GameResult {
        // 在这里实现地图渲染逻辑
        Ok(())
    }
}
