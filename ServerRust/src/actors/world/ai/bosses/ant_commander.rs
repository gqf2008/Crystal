//! AntCommander（蚁后指挥官）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/AntCommander.cs
//! 机制：
//!   - 近战（dist<=1）：
//!     - 3/6 普通近战（DC，无毒）
//!     - 2/6 Type=1 双倍伤害（DC*2）+ 1/4 Slow + 1/2 Dazed（5s，tick 1000）
//!     - 1/6 远程 MC（攻速+500ms）+ 1/5 绿毒（7s，tick 1000）
//!   - 远程（dist>1）：MC（攻速+500ms）+ 1/5 绿毒（7s，tick 1000）
//! 说明：C# CompleteAttack 里 target.Attacked 调两次（疑似 bug），按意图实现单次命中

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct AntCommanderBehavior;

impl AntCommanderBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for AntCommanderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            if dist <= 1 {
                let roll = fastrand::i32(0..6);
                match roll {
                    // 3/6 普通近战（C# case 0/3/4，毒标记 false）
                    0 | 3 | 4 => {
                        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                    }
                    // 2/6 Type=1 双倍伤害 + Slow/Dazed 毒
                    1 | 5 => {
                        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage: damage.saturating_mul(2),
                            spell_id: 0,
                            attack_type: 1,
                        });
                        // PoisonTarget(4, 5, Slow, 1000)：1/4
                        if fastrand::i32(0..4) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: target.session_id,
                                poison: Poison::new(PoisonType::SLOW, 5, poison_sc_value(monster), 1000),
                            });
                        }
                        // PoisonTarget(2, 5, Dazed, 1000)：1/2
                        if fastrand::i32(0..2) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: target.session_id,
                                poison: Poison::new(PoisonType::DAZED, 5, poison_sc_value(monster), 1000),
                            });
                        }
                    }
                    // 1/6 远程（C# case 2）
                    _ => {
                        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            target_object_id: target.object_id,
                            damage,
                            spell_id: 0,
                        });
                        // PoisonTarget(5, 7, Green, 1000)：1/5、7s
                        if fastrand::i32(0..5) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: target.session_id,
                                poison: Poison::new(PoisonType::GREEN, 7, poison_sc_value(monster), 1000),
                            });
                        }
                    }
                }
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::GREEN, 7, poison_sc_value(monster), 1000),
                    });
                }
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
