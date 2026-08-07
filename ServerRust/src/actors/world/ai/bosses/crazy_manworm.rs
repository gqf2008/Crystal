//! CrazyManworm（狂化人面虫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CrazyManworm.cs
//! 机制：
//!   - 2/3 概率 DC 物理近战（ACAgility，Type 0）
//!   - 1/3 概率 MC 魔法近战（ACAgility，Type 1）
//!   - 标准追击走位
//!
//! Attack（C# :13-47）：Random(3)>0→DC Type0；else MC Type1。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MELEE_RANGE: i32 = 1;

pub struct CrazyManwormBehavior;

impl CrazyManwormBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for CrazyManwormBehavior {
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
                // C# Random(3)>0 → DC（Type0）；else MC（Type1）
                let use_mc = fastrand::i32(0..3) == 0;
                let damage = if use_mc {
                    crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1)
                } else {
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
                };
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: if use_mc { 1 } else { 0 },
                });
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
