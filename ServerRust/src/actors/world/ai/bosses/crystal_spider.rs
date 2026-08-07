//! CrystalSpider（水晶蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CrystalSpider.cs
//! 机制：
//!   - 十字/对角判定（3 格内 x==0||y==0||x==y）
//!   - 近战 base.Attack（DC 单体）
//!   - 远程 Type1 DC LineAttack(3)：沿朝向方向直线 3 格，命中 + Green 8s
//!
//! Attack（C# :28-53）：!ranged→base；ranged→Type1 LineAttack。
//! CompleteAttack（C# :55-67）：命中→Green 8s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
const LINE_RANGE: i32 = 3;

pub struct CrystalSpiderBehavior;

impl CrystalSpiderBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for CrystalSpiderBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let in_line = dx == 0 || dy == 0 || dx == dy;

        if in_line && dx.max(dy) <= LINE_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 6;
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            let melee = dx.max(dy) <= MELEE_RANGE;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);

            if melee {
                // 近战 base.Attack（Type0）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // Type1 LineAttack(3)：沿朝向方向击中线上目标 + Green
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, LINE_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                let mut hit_any = false;
                for h in hits {
                    let hd = direction_towards(monster.x, monster.y, h.x, h.y);
                    if hd == dir {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 1,
                        });
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: h.session_id,
                            poison: Poison::new(PoisonType::GREEN, 8, damage, 2000),
                        });
                        hit_any = true;
                    }
                }
                if !hit_any {
                    // 线上无目标也打主目标（保证有伤害反馈）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::GREEN, 8, damage, 2000),
                    });
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
