//! RedMoonEvil（红月恶魔，AI 13）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RedMoonEvil.cs
//! 机制：不可移动、不可回血；每 AttackSpeed 周期 AoE 攻击视野内所有目标
//!   - ProcessTarget：FindAllTargets(ViewRange) → 一次 ObjectAttack + 逐个伤害
//!   - CanMove=false / CanRegen=false

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

pub struct RedMoonEvilBehavior;

impl RedMoonEvilBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for RedMoonEvilBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 不可移动（C# CanMove=false）
        monster.next_move_tick = u64::MAX;
        monster.ai_state = crate::actors::world::MonsterAiState::Idle;

        // C# ProcessTarget：每 AttackSpeed 周期 AoE 攻击视野内所有目标（一次 ObjectAttack）
        if ctx.tick_count >= monster.next_attack_tick {
            let view_range = monster.ai_profile.aggro_range.max(1) as i32;
            let targets = ctx.find_targets_in_range(monster.x, monster.y, view_range, monster.map_index);
            if targets.is_empty() {
                return;
            }
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: monster.x,
                center_y: monster.y,
                radius: view_range,
                damage,
                spell_id: 0,
            });
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
        }
    }
}
