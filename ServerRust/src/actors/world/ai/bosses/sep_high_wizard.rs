//! SepHighWizard（圣战高阶法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SepHighWizard.cs
//! 机制：远程魔法（MinMC/MaxMC）；攻击顺序：
//!   - 排斥（1 格内低等级目标推 4，10-30s 冷却）
//!   - 目标<=2 且 1/3：FlameField + 投射
//!   - 1/3：FireBang + 投射
//!   - HP<=80% 且 1/4：Vampirism + 投射
//!   - 否则：GreatFireBall + SinglePushAttack（推 3，等级门控）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct SepHighWizardBehavior {
    next_repulsion_tick: u64,
}

impl SepHighWizardBehavior {
    pub fn new() -> Self {
        Self { next_repulsion_tick: 0 }
    }
}

impl MonsterBehavior for SepHighWizardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            // C# SepHighWizard：魔法伤害 MinMC/MaxMC
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, monster.luck).max(1);

            // 排斥：1 格内低等级目标推 4，10-30s 冷却
            if ctx.tick_count >= self.next_repulsion_tick {
                let nearby: Vec<&crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, 1, monster.map_index);
                let pushed: Vec<(u64, u8)> = nearby.iter()
                    .filter(|p| (p.level as i32) < monster.level)
                    .map(|p| {
                        let dir = direction_towards(monster.x, monster.y, p.x, p.y);
                        (p.session_id, dir)
                    }).collect();
                if !pushed.is_empty() {
                    // C# RepulsionTime = Time + Second * Random(10,30)
                    self.next_repulsion_tick = ctx.tick_count + 100 + fastrand::i32(0..20) as u64 * 10;
                    for (sid, dir) in pushed {
                        ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                            session_id: sid,
                            dir,
                            distance: 4,
                        });
                    }
                    return;
                }
            }

            // C# 目标<=2 且 1/3：FlameField + 投射
            if dist <= 2 && fastrand::i32(0..3) == 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                return;
            }

            // C# 1/3：FireBang + 投射
            if fastrand::i32(0..3) == 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                return;
            }

            // C# HP<=80% 且 1/4：Vampirism + 投射
            let percent_hp = if monster.max_hp > 0 { (monster.hp as f32 / monster.max_hp as f32) * 100.0 } else { 100.0 };
            if percent_hp <= 80.0 && fastrand::i32(0..4) == 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                return;
            }

            // C# GreatFireBall + SinglePushAttack（推 1）
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            // C# SinglePushAttack：目标等级<=怪+5 才推 3 格
            if (target.level as i32) <= monster.level + 5 {
                ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                    session_id: target.session_id,
                    dir: direction_towards(monster.x, monster.y, target.x, target.y),
                    distance: 3,
                });
            }
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

    /// #1782：行为构建 + 排斥冷却初始为 0（可立即触发）
    #[test]
    fn test_sep_high_wizard_behavior_builds() {
        let b = SepHighWizardBehavior::new();
        assert_eq!(b.next_repulsion_tick, 0);
    }
}
