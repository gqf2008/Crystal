//! FlamingWooma（烈焰沃玛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlamingWooma.cs
//! 机制：
//!   - 近战用 MACAgility 防御判定（区别于普通怪的 ACAgility）
//!   - 其余标准追击+攻击
//!
//! Attack（C# :12-33）：DefenceType.MACAgility（魔法闪避判定）。
//! 注意：Rust 端 attack_type 字段编码防御类型，用 2 标记 MACAgility（由战斗层解读）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;

pub struct FlamingWoomaBehavior;

impl FlamingWoomaBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FlamingWoomaBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 2, // MACAgility（C# DefenceType.MACAgility）
                });
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
