//! 怪物行为 trait（对齐 C# MonsterObject 虚方法集）
//!
//! 每个 Boss / 特殊怪物实现此 trait，与 C# 类继承 1:1 对齐。
//! 普通怪物用 DefaultBehavior（保留原 9 种 MonsterAiType，由 tick_monsters 处理）。

use super::ctx::AiCtx;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;

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

    /// 是否可被怪物/宠物攻击（C# IsAttackTarget(MonsterObject)）——默认同玩家版；
    /// TownArcher/Siege 等对怪物恒 false（玩家可打）。#1984
    fn is_attackable_by_monster(&self) -> bool {
        self.is_attackable()
    }

    /// 能否自然回血（HellLord/TreeQueen 返回 false，对齐 C# CanRegen）
    fn can_regen(&self) -> bool {
        true
    }

    /// 受击预处理（返回实际承伤；EvilMir 睡眠返 0、HellLord stage<4 返 0）
    fn on_attacked(&mut self, damage: i32) -> i32 {
        damage
    }

    /// #1920：受击预处理（带 monster 引用，供需要读自身属性的行为使用，如 HellKeeper 护甲减伤）
    /// 默认转发 on_attacked，保持现有覆盖行为不变。
    fn on_attacked_with_monster(&mut self, _monster: &mut MonsterState, damage: i32) -> i32 {
        self.on_attacked(damage)
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

    /// #1399：死亡后是否保留尸体等待复活/苏醒（DemonGuard 延迟复活、DragonStatue 睡眠）
    fn keep_corpse_for_revive(&self) -> bool {
        false
    }

    /// #2108：是否可被采集（C# HarvestMonster 子类：死亡后保留尸体 + Harvest 交互）
    fn is_harvestable(&self) -> bool {
        false
    }

    /// #2358：可采集尸体皮肤次数（C# HarvestMonster.RemainingSkinCount：默认 2；Deer AI 2 = 5）
    fn harvest_skin_count(&self, _monster: &MonsterState) -> u8 {
        crate::actors::world::HARVEST_SKIN_COUNT
    }

    /// #1399：标记死亡已广播；返回是否首次（tick.rs 死亡处理首次发 ObjectDied，避免每 tick 重复）
    fn mark_death_announced(&mut self) -> bool {
        false
    }

    /// 出生初始化（对齐 C# Spawned；TreeQueen 设定时器）
    fn on_spawned(&mut self, _monster: &mut MonsterState) {}

    /// 运行时 downcast 支持（跨行为回调，如 HellLord 的 Knight 死亡推进阶段）
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::MonsterBehavior;

    /// #1980：is_attackable 钩子——石化/隐身 Boss 初始不可被选中（玩家攻击目标校验用）
    #[test]
    fn stoned_and_hidden_behaviors_report_not_attackable() {
        let zuma = crate::actors::world::ai::bosses::zuma_monster::ZumaMonsterBehavior::new();
        assert!(!zuma.is_attackable(), "石化祖玛初始不可攻击");
        let earth = crate::actors::world::ai::bosses::earth_golem::EarthGolemBehavior::new();
        assert!(!earth.is_attackable(), "石化土傀儡初始不可攻击");
        let dig = crate::actors::world::ai::bosses::dig_out_zombie::DigOutZombieBehavior::new();
        assert!(!dig.is_attackable(), "钻地僵尸初始隐身不可攻击");
        let evil = crate::actors::world::ai::bosses::evil_centipede::EvilCentipedeBehavior::new();
        assert!(!evil.is_attackable(), "地蜈蚣初始隐身不可攻击");
    }

    /// #1980：默认行为可攻击（绝大多数 Boss）
    #[test]
    fn default_behavior_is_attackable() {
        let d = crate::actors::world::ai::DefaultBehavior::new();
        assert!(d.is_attackable());
        assert!(d.is_attackable_by_monster());
    }

    /// #1984：TownArcher/Siege 对怪物不可攻击、对玩家可攻击（C# IsAttackTarget 双版本）
    #[test]
    fn town_archer_and_siege_immune_to_monsters() {
        let archer = crate::actors::world::ai::bosses::town_archer::TownArcherBehavior::new();
        assert!(archer.is_attackable(), "城镇弓箭手对玩家可攻击");
        assert!(
            !archer.is_attackable_by_monster(),
            "城镇弓箭手对怪物不可攻击"
        );
        let siege = crate::actors::world::ai::bosses::siege::SiegeBehavior::new();
        assert!(siege.is_attackable(), "攻城建筑对玩家可攻击");
        assert!(!siege.is_attackable_by_monster(), "攻城建筑对怪物不可攻击");
    }
}
