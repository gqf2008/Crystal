//! Trainer（训练木桩）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Trainer.cs
//! 机制：CanMove=false、Blocking、可被攻击；不移动、不反击（Die() 空）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

pub struct TrainerBehavior;

impl TrainerBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TrainerBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn process_tick(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // 训练木桩：无主动行为（不移动、不反击）
    }
}
