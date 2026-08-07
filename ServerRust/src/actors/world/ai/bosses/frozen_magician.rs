//! FrozenMagician（冰霜法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FrozenMagician.cs
//! 机制：
//!   - 近战范围 1；不在近战 1/2 概率远程（否则移动）
//!   - 近战 2/3 普攻 / 1/3 远程
//!   - 远程：2/3 普通 MC / 1/3 MC*1.5，冷却 +1000ms

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const ATTACK_RANGE: i32 = 9;

pub struct FrozenMagicianBehavior;

impl FrozenMagicianBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FrozenMagicianBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist > 1 {
            // C# 不在近战：1/2 概率远程 / 移动
            if ctx.tick_count >= monster.next_attack_tick && fastrand::i32(0..2) == 0 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 10;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                let dmg = if fastrand::i32(0..3) == 0 { (damage as f32 * 1.5) as i32 } else { damage };
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage: dmg.max(1),
                    spell_id: 0,
                });
                return;
            }
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# 2/3 普攻 / 1/3 远程
            if fastrand::i32(0..3) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 10;
                let dmg = if fastrand::i32(0..3) == 0 { (damage as f32 * 1.5) as i32 } else { damage };
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage: dmg.max(1),
                    spell_id: 0,
                });
            }
        }
    }
}
