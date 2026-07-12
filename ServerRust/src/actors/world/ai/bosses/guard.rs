//! Guard（守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Guard.cs
//! 机制：
//!   - 不可被攻击（IsAttackTarget 返 false，Attacked 抛异常，Struck 返 0，Die 空）
//!   - 不可回血（CanRegen=false）
//!   - 攻击范围=视野，攻击瞬秒怪物（damage=int.MaxValue if !Player）
//!   - 攻击玩家正常 DC，传送至目标背后攻击（C# 用 Target.Back 定位）
//!
//! Attack（C# :66-87）：damage = Target.Race!=Player ? int.MaxValue : DC；AC 防御。
//! 注意：C# Guard 只攻击红名/怪物目标（FindTarget 由基类红名逻辑驱动），此处简化为攻击范围内任意敌对目标。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

const VIEW_RANGE: i32 = 15;

pub struct GuardBehavior;

impl GuardBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GuardBehavior {
    fn can_regen(&self) -> bool { false }

    /// 守卫不可被攻击（C# IsAttackTarget 返 false）
    fn is_attackable(&self) -> bool { false }

    /// 守卫免疫一切伤害（C# Attacked 抛 NotSupportedException，Struck 返 0）
    fn on_attacked(&mut self, _damage: i32) -> i32 { 0 }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        // 全视野攻击（C# InAttackRange = ViewRange）
        let targets: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                .into_iter().copied().collect();
        if targets.is_empty() {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + 5;
        for t in targets {
            // 守卫对玩家用正常 DC（怪物秒杀逻辑不适用于玩家目标）
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: t.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        }
    }
}
