use crate::components::{FloatingText, Position};
use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;

/// 漂浮文本系统：更新上浮位置并清理过期实体
#[derive(ecs_macros::LogicSystem)]
pub struct FloatingTextSystem;

impl Default for FloatingTextSystem {
    fn default() -> Self {
        Self
    }
}

impl LogicSystem for FloatingTextSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        let now = macroquad::prelude::get_time();

        let mut to_remove: Vec<hecs::Entity> = Vec::new();

        for (entity, (pos, ft)) in ctx.world.query_mut::<(&mut Position, &FloatingText)>() {
            // 过期清理
            if now >= ft.start_time + ft.duration {
                to_remove.push(entity);
                continue;
            }

            // 上浮（只改 y，避免影响碰撞/寻路）
            pos.y -= ft.rise_speed * dt;
        }

        for e in to_remove {
            let _ = ctx.world.despawn(e);
        }

        Ok(())
    }
}
