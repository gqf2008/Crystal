use crate::ecs::systems::{DrawSystem,System};
use ggez::GameResult;

/// DebugSystem 是唯一的混合系统，既实现了 System 又实现了 DrawSystem
pub struct DebugSystem;

impl System for DebugSystem {
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
       
        Ok(())
    }
}

impl DrawSystem for DebugSystem {
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
