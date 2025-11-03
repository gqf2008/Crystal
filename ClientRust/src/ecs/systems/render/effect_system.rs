use crate::ecs::systems::DrawSystem;
use ggez::{GameResult, graphics::GraphicsContext};

pub struct EffectRenderSystem;

impl DrawSystem for EffectRenderSystem {
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