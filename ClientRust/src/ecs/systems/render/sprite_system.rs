use crate::ecs::systems::DrawSystem;
use ggez::{graphics::GraphicsContext, GameResult};
pub struct SpriteRenderSystem;

impl DrawSystem for SpriteRenderSystem {
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
