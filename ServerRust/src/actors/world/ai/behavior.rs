//! 怪物行为 trait（对齐 C# MonsterObject 虚方法集）
//!
//! 每个 Boss / 特殊怪物实现此 trait，与 C# 类继承 1:1 对齐。
//! 普通怪物用 DefaultBehavior（保留原 9 种 MonsterAiType，由 tick_monsters 处理）。

use crate::combat::poison::Poison;
use crate::actors::world::MonsterState;
use super::ctx::AiCtx;

/// 怪物行为接口（对齐 C# MonsterObject 的 virtual 方法）
/// 需要 Send + Sync：WorldActor 跨 await 点持有 monsters HashMap，要求 Box<dyn> 是 Send + Sync
pub trait MonsterBehavior: Send + Sync + 'static {
    /// 能否移动（EvilMir/HellLord/TreeQueen 返回 false，对齐 C# CanMove）
    fn can_move(&self) -> bool {
        true
    }

    /// 当前能否被攻击（EvilMir 睡眠期、HornedCommander 免疫期返回 false）
    fn is_attackable(&self) -> bool {
        true
    }

    /// 能否自然回血（HellLord/TreeQueen 返回 false，对齐 C# CanRegen）
    fn can_regen(&self) -> bool {
        true
    }

    /// 受击预处理（返回实际承伤；EvilMir 睡眠返 0、HellLord stage<4 返 0）
    fn on_attacked(&mut self, damage: i32) -> i32 {
        damage
    }

    /// 中毒预处理（返回 true 接受、false 拒绝；HellLord/TreeQueen 免疫返 false）
    fn on_poison(&mut self, _poison: Poison) -> bool {
        true
    }

    /// 每 tick AI 主入口（对齐 C# ProcessAI + ProcessTarget + Attack）
    ///
    /// `monster` 是该怪的可变引用（读写 hp/位置/定时器），
    /// `ctx` 提供玩家/怪物快照 + 输出队列（广播/法术场/召唤）。
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx);

    /// 死亡回调（对齐 C# Die；HornedCommander 清理 Slave/RockSpike）
    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {}

    /// 出生初始化（对齐 C# Spawned；TreeQueen 设定时器）
    fn on_spawned(&mut self, _monster: &mut MonsterState) {}

    /// 运行时 downcast 支持（跨行为回调，如 HellLord 的 Knight 死亡推进阶段）
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        None
    }
}
