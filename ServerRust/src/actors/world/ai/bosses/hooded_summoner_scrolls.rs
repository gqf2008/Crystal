//! HoodedSummonerScrolls（兜帽召唤卷轴）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HoodedSummonerScrolls.cs
//! 机制：远程风筝；按 Info.Effect：
//!   0 战士卷：远程伤害（MACAgility）
//!   1 道士卷：远程 + AOE1 目标 + 1/7 绿毒（5s）；死亡 AOE1 + 1/7 绿毒
//!   2 法师卷：远程伤害
//!   3 刺客卷：远程伤害
//!   默认：base.Attack（近战）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct HoodedSummonerScrollsBehavior;

impl HoodedSummonerScrollsBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for HoodedSummonerScrollsBehavior {
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
            match monster.effect {
                1 => {
                    // C# 道士卷：RangeDamage(毒标记) → AOE1 + 1/7 绿毒
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: target.x,
                        center_y: target.y,
                        radius: AOE_RADIUS,
                        damage,
                        spell_id: 0,
                    });
                    let nearby: Vec<u64> = ctx.find_targets_in_range(target.x, target.y, AOE_RADIUS, monster.map_index)
                        .iter().map(|p| p.session_id).collect();
                    for sid in nearby {
                        // C# PoisonTarget(7, 5, Green)：1/7、5s
                        if fastrand::i32(0..7) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: sid,
                                poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
                            });
                        }
                    }
                }
                _ => {
                    // C# 0/2/3/default：远程伤害（MACAgility）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                }
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# 风筝：>=view 接近，<view 远离
            let (nx, ny, dir) = if dist >= VIEW_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else {
                step_away(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# Die（仅 Effect==1）：CompleteDeath AOE1 + 1/7 绿毒
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if monster.effect != 1 {
            return;
        }
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
            if fastrand::i32(0..7) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
                });
            }
        }
    }
}
