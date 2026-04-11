use crate::{
    game::{GameContext, GameResult},
    systems::LogicSystem,
};

#[derive(ecs_macros::LogicSystem)]
pub struct HUDSystem {
}

impl Default for HUDSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl HUDSystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl LogicSystem for HUDSystem {
    fn update(&mut self, _ctx: &mut GameContext, _dt: f32) -> GameResult {
        // MainDialog 当前仍使用模拟数据字段（hp/mp/gold 等），暂未与 ECS 绑定。
        // 这里先作为表现层规划占位。
        Ok(())
    }
}
