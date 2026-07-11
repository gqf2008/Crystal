//! 默认怪物行为（普通怪物的占位 behavior）
//!
//! 普通怪物（9 种 MonsterAiType）的 AI 逻辑保留在 tick_monsters 内联处理，
//! 因为它们逻辑简单且与借用模型深度耦合。Boss 走专属 behavior。
//! DefaultBehavior 的 process_tick 为空 —— tick_monsters 对非 Boss 怪物
//! 不调用 behavior.process_tick，而是直接走原有内联逻辑。

use crate::actors::world::MonsterState;
use super::behavior::MonsterBehavior;
use super::ctx::AiCtx;

/// 默认行为（普通怪物）
pub struct DefaultBehavior;

impl DefaultBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for DefaultBehavior {
    fn process_tick(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // 普通怪物的 AI 由 tick_monsters 内联处理，此处为空。
        // （tick_monsters 通过 monster_index 判断：Boss 走 behavior.process_tick，
        //   非 Boss 走原有 MonsterAiType 逻辑）
    }
}
