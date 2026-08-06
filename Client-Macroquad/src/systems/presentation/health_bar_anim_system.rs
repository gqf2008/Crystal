// ============================================================================
// Layer 3: Presentation - HealthBarAnimSystem
// Priority: systems::priority::HEALTH_BAR_ANIM
// ============================================================================
//
// **职责**：
// - 为怪物血条提供“掉血动画”（显示值平滑下降到真实 Health.current）
// - 仅影响渲染显示，不修改真实战斗血量
//
// ============================================================================

use crate::components::{Health, HealthBarAnim, Monster};
use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;

#[derive(ecs_macros::LogicSystem, Default)]
pub struct HealthBarAnimSystem;

impl LogicSystem for HealthBarAnimSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 1) 确保怪物都有 HealthBarAnim（初始化为当前血量）。
        let mut to_init: Vec<(hecs::Entity, f32)> = Vec::new();
        for (e, (_m, hp)) in ctx.world.iter().filter_map(|e| {
            let m = e.get::<&Monster>()?;
            let hp = e.get::<&Health>()?;
            Some((e.entity(), (m, hp)))
        }) {
            if ctx.world.get::<&HealthBarAnim>(e).is_err() {
                to_init.push((e, hp.current.max(0) as f32));
            }
        }
        for (e, v) in to_init {
            let _ = ctx.world.insert_one(e, HealthBarAnim { displayed: v });
        }

        // 2) 平滑下降：displayed 以固定速度向 target 逼近；回血/满血则直接对齐。
        // 速度策略：按 max 血量缩放，保证“看得到动画”但不拖泥带水。
        let dt = dt.max(0.0);
        for (_m, hp, anim) in ctx
            .world
            .query_mut::<(&Monster, &Health, &mut HealthBarAnim)>()
        {
            let max = hp.max.max(1) as f32;
            let target = (hp.current.max(0) as f32).min(max);

            // 若当前显示比目标低（回血/初始化顺序），直接追上避免“慢慢加血”的怪异感。
            if anim.displayed <= target {
                anim.displayed = target;
                continue;
            }

            // 掉血动画：以 max*6/秒 的速度下降（满血到 0 大约 0.17s）。
            let drop_speed = (max * 6.0).max(60.0);
            anim.displayed = (anim.displayed - drop_speed * dt)
                .max(target)
                .clamp(0.0, max);
        }

        Ok(())
    }
}
