use crate::systems::RenderSystem;
use crate::game::GameResult;

#[derive(ecs_macros::RenderSystem)]
pub struct UIRenderSystem;

impl RenderSystem for UIRenderSystem {
    fn draw(&mut self, _world: &hecs::World) -> GameResult {
        // TODO: macroquad UI 渲染
        // 使用 macroquad::ui 或直接绘制
        Ok(())
    }
}