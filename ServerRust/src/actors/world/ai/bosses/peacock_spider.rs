//! PeacockSpider（孔雀蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/PeacockSpider.cs
//! 机制：
//!   - 近战（dist<=3，3/4 概率）：
//!     - 毒云（20s 冷却）：伤害 + FindAllTargets(3) AOE + 1/2 绿毒（2-6s）
//!     - 普通（2/3）：伤害 + 1/2 眩晕毒（2-6s）
//!     - 前撞（1/3）：TriangleAttack(damage, 3, 2)（9 格锥）
//!   - 远程（dist>3 或 1/4）：1/5 ProjectileAttack + 1/4 麻痹毒（5s，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 3;
const POISON_COOLDOWN: u64 = 200; // 20s

pub struct PeacockSpiderBehavior {
    next_poison_tick: u64,
}

impl PeacockSpiderBehavior {
    pub fn new() -> Self {
        Self { next_poison_tick: 0 }
    }
}

impl MonsterBehavior for PeacockSpiderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# !ranged && Random.Next(4) > 0：近战 3/4
            if dist <= MELEE_RANGE && fastrand::i32(0..4) > 0 {
                // 毒云优先（20s 冷却）
                if ctx.tick_count >= self.next_poison_tick {
                    self.next_poison_tick = ctx.tick_count + POISON_COOLDOWN;
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: MELEE_RANGE,
                        damage,
                        spell_id: 0,
                    });
                    let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, MELEE_RANGE, monster.map_index)
                        .iter().map(|p| p.session_id).collect();
                    for sid in nearby {
                        // PoisonTarget(2, random(2..6), Green, 1000)：1/2、2-6s
                        if fastrand::i32(0..2) == 0 {
                            let dur = fastrand::i32(2..6) as u32;
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: sid,
                                poison: Poison::new(PoisonType::GREEN, dur, damage, 1000),
                            });
                        }
                    }
                } else if fastrand::i32(0..3) > 0 {
                    // 普通：伤害 + 1/2 眩晕毒（2-6s）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    if fastrand::i32(0..2) == 0 {
                        let dur = fastrand::i32(2..6) as u32;
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::DAZED, dur, 0, 1000),
                        });
                    }
                } else {
                    // 前撞：TriangleAttack(damage, 3, 2, 500, ACAgility, false)（9 格锥）
                    let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                    monster.direction = dir;
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Triangle {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        direction: dir,
                        distance: 3,
                        limit_width: 2,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            } else if fastrand::i32(0..5) == 0 {
                // 远程：1/5 ProjectileAttack + 1/4 麻痹毒（5s，tick 1000）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                if fastrand::i32(0..4) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::PARALYSIS, 5, 0, 1000),
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
