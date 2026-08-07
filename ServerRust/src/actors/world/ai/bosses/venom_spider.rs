//! VenomSpider（毒液蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/VenomSpider.cs
//! 机制：
//!   - InAttackRange 特殊十字判定（2 格：x<=1&&y<=1 或 同奇偶）
//!   - Attack：DC LineAttack(2) + MACAgility，命中后 8s 绿毒
//!
//! Attack（C# :27-46）：DC LineAttack(2, MACAgility)。
//! CompleteAttack（C# :48-59）：命中 → Green 8s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
/// LineAttack 距离（C# LineAttack(damage, 2)）
const LINE_RANGE: i32 = 2;

pub struct VenomSpiderBehavior;

impl VenomSpiderBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for VenomSpiderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= LINE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // LineAttack(2)：朝目标方向直线 2 格命中 + 主目标
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let dx = DIR_DX[dir as usize];
                let dy = DIR_DY[dir as usize];
                let hits: Vec<crate::actors::world::ai::PlayerSnap> = ctx
                    .find_targets_in_range(monster.x, monster.y, LINE_RANGE, monster.map_index)
                    .into_iter().copied()
                    .filter(|p| {
                        let rx = p.x - monster.x;
                        let ry = p.y - monster.y;
                        (rx == 0 && dy == 0) || (ry == 0 && dx == 0)
                            || (rx.signum() == dx.signum() && ry.signum() == dy.signum() && rx.abs() == ry.abs())
                    })
                    .collect();
                if hits.is_empty() {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    // C# PoisonTarget(8,5,Green,1000)：1/8
                    if fastrand::i32(0..8) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
                        });
                    }
                } else {
                    for h in hits {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                        // C# PoisonTarget(8,5,Green,1000)：1/8
                        if fastrand::i32(0..8) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: h.session_id,
                                poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
                            });
                        }
                    }
                }
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
