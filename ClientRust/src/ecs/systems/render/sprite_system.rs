use crate::ecs::{GameContext, systems::DrawSystem};
use ggez::GameResult;
pub struct SpriteRenderSystem;

impl DrawSystem for SpriteRenderSystem {
    fn draw(
        &mut self,
        ctx: &mut GameContext,
        canvas: &mut ggez::graphics::Canvas,
    ) -> GameResult {
        // 在这里实现地图渲染逻辑
        Ok(())
    }
}