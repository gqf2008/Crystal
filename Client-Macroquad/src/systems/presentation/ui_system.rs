use crate::{
    game::{GameContext, GameResult},
    systems::LogicSystem,
};

#[derive(ecs_macros::LogicSystem)]
pub struct UISystem {
}

impl UISystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl LogicSystem for UISystem {
    fn update(&mut self, _ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 目前 UI 主要在 UIRenderSystem::draw 内完成“绘制 + 输入驱动”。
        // 这里保留 UISystem 作为表现层规划占位：后续可逐步把 UI 的状态更新/事件处理迁移到此。
        Ok(())
    }
}
