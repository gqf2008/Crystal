use crate::ecs::HybridSystem;
use ggez::GameResult;

/// DebugSystem 是唯一的混合系统，既实现了 System 又实现了 DrawSystem
///
pub struct DebugSystem;

impl HybridSystem for DebugSystem {
    fn priority(&self) -> u32 {
       u32::MAX-1
    }
    fn update(&mut self, world: &mut hecs::World, delay_time: f32) -> GameResult {
        // 在这里实现调试信息的更新逻辑
        Ok(())
    }
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
