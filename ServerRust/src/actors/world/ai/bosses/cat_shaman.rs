//! CatShaman（猫巫师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CatShaman.cs
//! 机制：
//!   - 近战贴身 DC（ACAgility）；远程（>1 格）MC 弹道
//!   - 远程 1/5 概率附加红毒（PoisonTarget(5,5,Red,1000)）
//!   - AttackRange=6，风筝走位
//!
//! Attack（C# :26-72）：ranged→4/5 普通 MC 弹道 / 1/5 MC 弹道+红毒。
//! CompleteRangeAttack（C# :75-89）：poison && damage>0 → Red 毒。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct CatShamanBehavior;

impl CatShamanBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for CatShamanBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        } else if dist <= ATTACK_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 8;
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // 1/5 红毒（C# Random(5)==0）
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::RED, 5, damage, 1000),
                    });
                }
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
