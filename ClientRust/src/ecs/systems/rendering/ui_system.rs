use crate::ecs::{Coord, systems::RenderSystem};
use ggez::{GameResult, graphics::GraphicsContext};

#[derive(ecs_macros::RenderSystem)]
pub struct UIRenderSystem;

impl RenderSystem for UIRenderSystem {
    fn draw(
        &mut self,
        _ctx: &mut GraphicsContext,
        canvas: &mut ggez::graphics::Canvas,
        _world: &hecs::World,
    ) -> GameResult {
        
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            Coord::DESIGN_WIDTH,   // 1024 (UI 设计分辨率)
            Coord::DESIGN_HEIGHT,  // 768 (UI 设计分辨率)
        ));
        // 在这里实现地图渲染逻辑
        Ok(())
    }
}