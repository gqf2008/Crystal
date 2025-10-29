use crate::ecs::{Coordinates, systems::DrawSystem};
use ggez::GameResult;


pub struct UIRenderSystem;

impl DrawSystem for UIRenderSystem {
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult {
        
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            Coordinates::DESIGN_WIDTH,   // 1024 (UI 设计分辨率)
            Coordinates::DESIGN_HEIGHT,  // 768 (UI 设计分辨率)
        ));
        // 在这里实现地图渲染逻辑
        Ok(())
    }
}