use crate::ecs::systems::DrawSystem;
use ggez::GameResult;

pub struct CameraSystem;

impl DrawSystem for CameraSystem {
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult {
        // 在这里实现地图渲染逻辑
        Ok(())
    }
}