//! IcePillar（冰柱）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/IcePillar.cs
//! 机制：CanMove=false、CanRegen=false、Struck=0、毒免疫；
//!      受击固定 ChangeHP(-1)（每次只扣 1 血）+ 1/3 CloseAttack（AOE1 + 1/5 冰冻，值=MC）；
//!      Die：FindAllTargets(7) AOE 伤害 + 每目标 1/5 冰冻（tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;
const DEATH_RADIUS: i32 = 7;

pub struct IcePillarBehavior;

impl IcePillarBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for IcePillarBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    /// C# Attacked：ChangeHP(-1) → 每次只受 1 点伤害
    fn on_attacked(&mut self, _damage: i32) -> i32 {
        1
    }

    fn on_poison(&mut self, _poison: Poison) -> bool {
        false // C# ApplyPoison 空实现
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Attacked 中 1/3 CloseAttack：AOE1 + 1/5 冰冻（用 last_hit_damage 触发）
        if monster.last_hit_damage > 0 {
            monster.last_hit_damage = 0;
            if fastrand::i32(0..3) == 0 {
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: AOE_RADIUS,
                    damage,
                    spell_id: 0,
                });
                let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                    .iter().map(|p| p.session_id).collect();
                for sid in nearby {
                    // C# PoisonTarget(5, MC, Frozen, 1000)：1/5、时长=MC 攻秒数、值=MC（DC 近似）
                    if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::FROZEN, damage.max(1) as u32, damage, 1000),
                        });
                    }
                }
            }
        }
    }

    /// C# Die + CompleteDeath：7 格 AOE 伤害 + 每目标 1/5 冰冻
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: DEATH_RADIUS,
            damage,
            spell_id: 0,
        });
        let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, DEATH_RADIUS, monster.map_index)
            .iter().map(|p| p.session_id).collect();
        for sid in nearby {
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::FROZEN, 5, damage, 1000),
                });
            }
        }
    }
}
