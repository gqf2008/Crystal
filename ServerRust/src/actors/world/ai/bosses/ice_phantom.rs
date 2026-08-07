//! IcePhantom（冰幻影）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/IcePhantom.cs
//! 机制：
//!   - AttackRange=5，可移动追击
//!   - 近战（<=1）：DC + DefenceType.ACAgility
//!   - 远程（>1）：MC 弹道 + DefenceType.MAC（冰系）
//!   - 标准追击型法师怪（可移动，无隐身——C# IcePhantom 仅为近/远双模式）
//!
//! Attack（C# :26-61）：!ranged→DC ACAgility；ranged→MC MAC。
//! ProcessTarget（C# :64-81）：InRange→Attack；否则 MoveTo。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 5;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct IcePhantomBehavior;

impl IcePhantomBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for IcePhantomBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = if dist <= MELEE_RANGE {
                // 近战 DC ACAgility
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
            } else {
                // 远程 MC MAC（冰系）
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1)
            };

            if dist <= MELEE_RANGE {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
