//! SepHighTaoist（圣战高阶道士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SepHighTaoist.cs
//! 机制：
//!   - 毒轮换：目标无毒→Green（SC，value=power/15+4）/ 只有 Green→Red / 只有 Red→Green
//!   - Curse（目标无 Curse 且 1/8）：Slow 5s
//!   - MassHealing（HP<=90% 且 1/8）：TriangleAttack（Aoe 近似）
//!   - 无宠物时召唤 Shinsu（PetLevel 3 / MaxPetLevel 7）
//!   - SoulFireBall + HalfmoonAttack（Aoe 近似）
//! 近似说明：AI 无目标毒/buff 列表，毒轮换与 Curse buff 检查用行为状态/概率近似。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::{AiCtx, BossSummon};
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

/// C# 毒轮换状态：0=无 1=Green 2=Red
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PoisonStage {
    None = 0,
    Green = 1,
    Red = 2,
}

/// C# 绿毒 value = power / 15 + 4
fn taoist_green_poison_value(power: i32) -> i32 {
    power / 15 + 4
}

pub struct SepHighTaoistBehavior {
    /// 目标毒轮换状态（C# 查目标 PoisonList；AI 无该数据，用状态近似）
    poison_stage: PoisonStage,
    /// 是否已召唤 Shinsu（C# Pets.Count<1；AI 无法感知宠物死亡，近似为仅一次）
    has_summoned: bool,
}

impl SepHighTaoistBehavior {
    pub fn new() -> Self {
        Self { poison_stage: PoisonStage::None, has_summoned: false }
    }
}

impl MonsterBehavior for SepHighTaoistBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);

            // C# 毒轮换：无→Green / Green→Red / Red→Green（毒分支优先）
            // 约 3/4 概率施毒（近似 C# 毒优先；其余走双毒回落路径的 Curse/MassHealing/召唤/SoulFireBall）
            if fastrand::i32(0..4) != 0 {
                // C# 用 SC（道士灵魂）；MonsterState 无 SC 字段，以 DC 近似
                let power = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                let (stage, p_type) = match self.poison_stage {
                    PoisonStage::None => (PoisonStage::Green, PoisonType::GREEN),
                    PoisonStage::Green => (PoisonStage::Red, PoisonType::RED),
                    PoisonStage::Red => (PoisonStage::Green, PoisonType::GREEN),
                };
                let value = if p_type == PoisonType::GREEN { taoist_green_poison_value(power) } else { 0 };
                self.poison_stage = stage;
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(
                        p_type,
                        power as u32 + (fastrand::i32(0..3) + 1) as u32 * 7,
                        value,
                        2000,
                    ),
                });
                return;
            }

            // C# MassHealing（HP<=90% 且 1/8）：TriangleAttack（Aoe 半径 1 近似）
            let percent_hp = if monster.max_hp > 0 { (monster.hp as f32 / monster.max_hp as f32) * 100.0 } else { 100.0 };
            if percent_hp <= 90.0 && fastrand::i32(0..8) == 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: 1,
                    damage,
                    spell_id: 0,
                });
                return;
            }

            // C# 无宠物时召唤 Shinsu（Pets.Count<1；AI 无法感知宠物死亡，近似仅一次）
            if !self.has_summoned {
                self.has_summoned = true;
                ctx.out_summons.push(BossSummon {
                    monster_name: "Shinsu".to_string(), // C# Settings.ShinsuName
                    x: monster.x,
                    y: monster.y,
                    is_slave: true,
                    summoner_oid: Some(monster.object_id),
                });
                return;
            }

            // C# Curse（目标无 Curse 且 1/8）：Slow 5s（buff 检查近似为概率）
            if fastrand::i32(0..8) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::SLOW, 5, 1, 1000),
                });
                return;
            }

            // C# SoulFireBall + HalfmoonAttack（Aoe 半径 1 近似）
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: target.x,
                center_y: target.y,
                radius: 1,
                damage,
                spell_id: 0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1788：道士绿毒 value = power/15+4
    #[test]
    fn test_taoist_green_poison_value() {
        assert_eq!(taoist_green_poison_value(150), 14);
        assert_eq!(taoist_green_poison_value(30), 6);
        assert_eq!(taoist_green_poison_value(0), 4);
    }

    /// #1788：行为构建 + 毒轮换初始 None + 未召唤
    #[test]
    fn test_sep_high_taoist_builds() {
        let b = SepHighTaoistBehavior::new();
        assert_eq!(b.poison_stage, PoisonStage::None);
        assert!(!b.has_summoned);
    }
}
