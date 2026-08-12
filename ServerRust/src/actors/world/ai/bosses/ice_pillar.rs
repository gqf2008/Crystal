//! IcePillar（冰柱）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/IcePillar.cs
//! 机制：CanMove=false、CanRegen=false、Struck=0、毒免疫；
//!      受击：敏捷闪避 + AC/MAC 护甲减伤（C# Attacked 自定义），通过后固定 ChangeHP(-1)；
//!      1/3 CloseAttack（AOE1 承伤值 + IcePillar 特效 + 1/5 冰冻，值=MC）；
//!      Die：FindAllTargets(7) AOE 伤害 + 每目标 1/5 冰冻（tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const AOE_RADIUS: i32 = 1;
const DEATH_RADIUS: i32 = 7;

pub struct IcePillarBehavior {
    /// C# CloseAttack 使用承伤值（Attacked 中 armour 判定通过后传入）
    last_incoming_damage: i32,
}

impl IcePillarBehavior {
    pub fn new() -> Self {
        Self { last_incoming_damage: 0 }
    }
}

impl MonsterBehavior for IcePillarBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    /// C# Attacked 自定义（IcePillar.cs）：敏捷闪避 + AC/MAC 护甲减伤；
    /// 通过后固定扣 1 血，并记录承伤供 CloseAttack 使用
    fn on_attacked_with_monster(&mut self, monster: &mut MonsterState, damage: i32) -> i32 {
        if damage <= 0 {
            return 0;
        }
        // 敏捷闪避（C# Random(Agility+1) > Accuracy → 0；无攻击者 Accuracy，近似 1/(Agility+1)）
        if monster.agility > 0 && fastrand::i32(0..=monster.agility) == 0 {
            return 0;
        }
        // 护甲减伤（C# 按 DefenceType 取 AC 或 MAC 单值；无类型信息，取两者均值）
        let armour = (monster.min_ac + monster.max_ac) / 2;
        let mac = (monster.min_mac + monster.max_mac) / 2;
        if damage <= (armour + mac) / 2 {
            return 0;
        }
        self.last_incoming_damage = damage;
        1
    }

    fn on_poison(&mut self, _poison: Poison) -> bool {
        false // C# ApplyPoison 空实现
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Attacked 中 1/3 CloseAttack：AOE1 + IcePillar 特效 + 1/5 冰冻（用 last_hit_damage 触发）
        if monster.last_hit_damage > 0 {
            monster.last_hit_damage = 0;
            if fastrand::i32(0..3) == 0 {
                // C# CloseAttack(damage)：伤害 = 承伤值（无记录时回退自身 DC）
                let damage = if self.last_incoming_damage > 0 {
                    self.last_incoming_damage
                } else {
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
                };
                self.last_incoming_damage = 0;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: AOE_RADIUS,
                    damage: damage.max(1),
                    spell_id: 0,
                });
                let nearby: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                        .into_iter().copied().collect();
                let mc_power = crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0).max(1);
                let sc_power = crate::combat::attack::get_attack_power(monster.min_sc, monster.max_sc, 0).max(1);
                for p in nearby {
                    // C# CloseAttack（IcePillar.cs:158）：每个命中目标广播 IcePillar 特效
                    ctx.out_effects.push((p.object_id, mir2_shared::enums::SpellEffect::IcePillar, 0, 0));
                    // C# PoisonTarget(5, MC, Frozen, 1000)：1/5、时长=MC 攻秒数、值=SC
                    if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: p.session_id,
                            poison: Poison::new(PoisonType::FROZEN, mc_power as u32, sc_power, 1000),
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
                    poison: Poison::new(PoisonType::FROZEN, 5, crate::actors::world::ai::helpers::poison_sc_value(monster), 1000),
                });
            }
        }
    }
}
