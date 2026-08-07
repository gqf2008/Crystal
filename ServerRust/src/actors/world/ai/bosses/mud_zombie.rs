//! MudZombie（泥僵尸）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/MudZombie.cs
//! 机制：
//!   - InAttackRange：Info.ViewRange 内（此处 12）
//!   - 距离<=2 且非同位：近战 ObjectAttack + LineAttack(damage, 2)，命中 1/5 绿毒（8s，值=SP，tick 2000）
//!   - 否则：远程 ObjectRangeAttack（MC，MAC 防御），攻速 +500ms，命中 1/5 绿毒

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const LINE_RANGE: i32 = 2;

pub struct MudZombieBehavior;

impl MudZombieBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for MudZombieBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        if dist > VIEW_RANGE {
            return;
        }

        if ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# ranged = 同位 || !InRange(..., 2)（切比雪夫距离 > 2）
            let ranged = dist == 0 || dist > 2;
            if !ranged {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                // C# LineAttack(damage, 2)：沿朝向 2 格逐格命中 + 绿毒
                let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize % 8;
                let mut hit_any = false;
                for i in 1..=LINE_RANGE {
                    let tx = monster.x + DIR_DX[dir] * i;
                    let ty = monster.y + DIR_DY[dir] * i;
                    if let Some(p) = ctx.players.iter()
                        .find(|p| p.map_index == monster.map_index && p.x == tx && p.y == ty && p.hp > 0)
                    {
                        hit_any = true;
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: p.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                        // PoisonTarget(5, 8, Green, 2000)：1/5 概率，值=SP
                        if fastrand::i32(0..5) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: p.session_id,
                                poison: Poison::new(PoisonType::GREEN, 8, damage, 2000),
                            });
                        }
                    }
                }
                if !hit_any {
                    // 至少打主目标（与 SpittingSpider 近似一致）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            } else {
                // C# 远程：攻速 +500ms（5 tick）
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
                        poison: Poison::new(PoisonType::GREEN, 8, damage, 2000),
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
