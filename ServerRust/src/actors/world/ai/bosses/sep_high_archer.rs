//! SepHighArcher（圣战高阶弓手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SepHighArcher.cs
//! 机制：远程弓手；攻击分支：
//!   - 目标<=2 且 1/3：BackStep 后跳（反向最多 3 格）
//!   - 目标持有 PoisonShot buff 且 1/2：CrippleShot——目标+3×3 区域受伤（MAC）+ 绿毒，消耗 buff
//!   - 否则：PoisonShot 投射（延迟 ACAgility）+ 5/10 给目标上 PoisonShot buff（10s）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
/// C# AttackRange = 6
const ATTACK_RANGE: i32 = 6;
const POISON_BUFF_TICKS: u64 = 100; // C# Second*10 = 10s

/// C# CrippleShot 3×3 区域毒值：damage / 25 + 4
fn cripple_area_poison_value(damage: i32) -> i32 {
    damage / 25 + 4
}

pub struct SepHighArcherBehavior {
    /// 目标持有 PoisonShot buff 的到期 tick（0=未激活；C# Buff 10s）
    poison_shot_active_until: u64,
}

impl SepHighArcherBehavior {
    pub fn new() -> Self {
        Self { poison_shot_active_until: 0 }
    }
}

impl MonsterBehavior for SepHighArcherBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);

            // C# 目标<=2 且 1/3：BackStep 后跳（反向最多 3 格；boss_moves 应用时校验 walkable）
            if dist <= 2 && fastrand::i32(0..3) == 0 {
                // C# BackStep：ObjectBackStep 广播 + 直接落位（#1801）
                let back = direction_towards(monster.x, monster.y, target.x, target.y) as i32 + 4;
                let dir = (back.rem_euclid(8)) as u8;
                ctx.out_backsteps.push((monster.object_id, dir, 3));
                return;
            }

            let poison_active = ctx.tick_count < self.poison_shot_active_until;
            if poison_active && fastrand::i32(0..2) == 0 {
                // C# CrippleShot：目标 + 3×3 区域受伤（MAC）+ 绿毒，消耗 PoisonShot buff
                self.poison_shot_active_until = 0;
                // C# PoisonTarget(Target, 5, 8, Green, 2000)：主目标额外 1/5、8s（值=SC，用 damage 近似）
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::GREEN, 8, damage, 2000),
                    });
                }
                for p in ctx.players.iter().filter(|p| p.map_index == monster.map_index && p.hp > 0
                    && max_distance(target.x, target.y, p.x, p.y) <= 1) {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: p.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 2,
                    });
                    // C# 3×3 区域绿毒：value = damage/25+4，tick 2000ms
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: p.session_id,
                        poison: Poison::new(
                            PoisonType::GREEN,
                            (fastrand::i32(1..3) + 1) as u32 * 7,
                            cripple_area_poison_value(damage),
                            2000,
                        ),
                    });
                }
                return;
            }

            // C# PoisonShot：投射（延迟 ACAgility）+ 5/10 上 buff（10s）
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            if fastrand::i32(0..10) <= 4 {
                self.poison_shot_active_until = ctx.tick_count + POISON_BUFF_TICKS;
            }
            return;
        }

        // C# ProcessTarget：出攻击范围才 MoveTo（不风筝）
        if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
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

    /// #1786：CrippleShot 区域毒值 = damage/25+4
    #[test]
    fn test_cripple_area_poison_value() {
        assert_eq!(cripple_area_poison_value(100), 8);
        assert_eq!(cripple_area_poison_value(25), 5);
        assert_eq!(cripple_area_poison_value(0), 4);
        assert_eq!(cripple_area_poison_value(250), 14);
    }

    /// #1786：行为构建 + buff 初始未激活
    #[test]
    fn test_sep_high_archer_builds() {
        let b = SepHighArcherBehavior::new();
        assert_eq!(b.poison_shot_active_until, 0);
    }
}
