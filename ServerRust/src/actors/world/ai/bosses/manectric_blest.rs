//! ManectricBlest（雷兽祝福）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ManectricBlest.cs
//! 机制：每 5 次攻击 → Type=2 AOE3（MC，MAC）+ 每目标 1/5 冰冻（5s，tick 1000）；
//!      其余 2/3 base.Attack / 1/3 旋风（绕自身 8 向排除正面，每格 1 目标，不计数）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 3;

pub struct ManectricBlestBehavior {
    attack_count: u32,
}

impl ManectricBlestBehavior {
    pub fn new() -> Self {
        Self { attack_count: 0 }
    }
}

impl MonsterBehavior for ManectricBlestBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# _attackCount >= 5：AOE3 + 1/5 冰冻
            if self.attack_count >= 5 {
                self.attack_count = 0;
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
                    if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::FROZEN, 5, 0, 1000),
                        });
                    }
                }
                return;
            }
            // C# Random.Next(3)：0/1 → base.Attack（计数+1）/ 2 → 旋风（不计数）
            let roll = fastrand::i32(0..3);
            if roll < 2 {
                self.attack_count += 1;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // C# 旋风（ManectricBlest.cs:70-93）：绕自身 8 向逐格，排除正面格，每格最多 1 个目标（DC，Agility）
                let facing = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                for d in 0..8usize {
                    if d == facing { continue; } // C# tar == Front → continue
                    let cx = monster.x + DIR_DX[d];
                    let cy = monster.y + DIR_DY[d];
                    if let Some(p) = ctx.players.iter().find(|p| {
                        p.map_index == monster.map_index && p.hp > 0 && p.x == cx && p.y == cy
                    }) {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: p.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 1,
                        });
                    }
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
