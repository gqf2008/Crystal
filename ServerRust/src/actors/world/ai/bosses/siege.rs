//! Siege（攻城器械）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Siege.cs
//! 机制：FindTarget/ProcessSearch/ProcessTarget 全空 → 永不攻击；CanRegen=false；Effect 3/4/5 静态
//! 实现：静态不可攻击结构（can_move=false + can_regen=false + 空行为）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::MonsterState;

pub struct SiegeBehavior;

impl Default for SiegeBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl SiegeBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SiegeBehavior {
    /// 不可被怪物攻击（C# Siege.cs IsAttackTarget(MonsterObject)=false）；玩家可攻击
    fn is_attackable_by_monster(&self) -> bool {
        false
    }
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C#：无索敌、无攻击、无移动（仅可被破坏）
    }
}
