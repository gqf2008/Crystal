//! DeathCrawler（死亡爬行者）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DeathCrawler.cs
//! 机制：
//! - ApplyNegativeEffects：受击 1/3 → ObjectEffect(DeathCrawlerBreath) + 攻击者绿毒（5/5/2000）
//! - CompleteDeath：死亡时对 1 格内**所有**目标必然施绿毒（5 伤害/5s/tick 2000）
//!   （#1364：修正此前 1/5 概率 + 攻击力伤害的错误）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;
/// C# 受击吐息毒概率（1/3）
const BREATH_CHANCE: i32 = 3;
/// C# PoisonTarget(5, 5, Green, 2000)：固定 5 伤害
const BREATH_POISON_DAMAGE: i32 = 5;
const BREATH_POISON_DURATION: u32 = 5;
const BREATH_POISON_TICK_MS: u64 = 2000;

/// #1364：C# PoisonTarget(5, 5, PoisonType.Green, 2000)——受击/死亡吐息毒
fn breath_poison() -> Poison {
    Poison::new(
        PoisonType::GREEN,
        BREATH_POISON_DURATION,
        BREATH_POISON_DAMAGE,
        BREATH_POISON_TICK_MS,
    )
}

pub struct DeathCrawlerBehavior {
    /// #1364：受击触发的吐息毒待发次数（on_attacked 记，process_tick 消费）
    breath_pending: u32,
}

impl DeathCrawlerBehavior {
    pub fn new() -> Self {
        Self { breath_pending: 0 }
    }
}

impl MonsterBehavior for DeathCrawlerBehavior {
    /// #1364：C# ApplyNegativeEffects——受击 1/3 概率触发吐息毒（特效+毒在 process_tick 应用）
    fn on_attacked(&mut self, damage: i32) -> i32 {
        if fastrand::i32(0..BREATH_CHANCE) == 0 {
            self.breath_pending = self.breath_pending.saturating_add(1);
        }
        damage
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // #1364：消费受击吐息毒——广播特效 + 对 1 格内最近玩家施绿毒（近似攻击者，C# 攻击者通常贴身）
        if self.breath_pending > 0 {
            self.breath_pending -= 1;
            ctx.out_effects.push((
                monster.object_id,
                mir2_shared::enums::SpellEffect::DeathCrawlerBreath,
            ));
            if let Some(attacker) = ctx
                .nearest_target(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                .copied()
            {
                ctx.out_poisons
                    .push(crate::actors::world::ai::PoisonPlayer {
                        session_id: attacker.session_id,
                        poison: breath_poison(),
                    });
            }
        }
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// #1364：C# CompleteDeath——对 1 格内**所有**目标必然施绿毒（5 伤害/5s/tick 2000）
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let nearby: Vec<u64> = ctx
            .find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
            .iter()
            .map(|p| p.session_id)
            .collect();
        for sid in nearby {
            ctx.out_poisons
                .push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: breath_poison(),
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breath_poison_matches_csharp_values() {
        // C# DeathCrawler.cs：PoisonTarget(target, 5, 5, PoisonType.Green, 2000)
        let p = breath_poison();
        assert_eq!(p.p_type, PoisonType::GREEN);
        assert_eq!(p.duration_s, 5);
        assert_eq!(p.value, 5);
        assert_eq!(p.tick_ms, 2000);
    }
}
