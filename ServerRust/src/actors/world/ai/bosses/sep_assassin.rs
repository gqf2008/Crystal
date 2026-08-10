//! SepAssassin（圣战刺客）behavior（简化）
//!
//! C# 参考：Server/MirObjects/Monsters/SepAssassin.cs
//! 机制：近战 + DoubleSlash：近战伤害 + 投射伤害（双重命中）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
/// C# AttackRange = 3（RangeAttack HeavenlySword LineAttack 用）
const ATTACK_RANGE: i32 = 3;

pub struct SepAssassinBehavior;

impl SepAssassinBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SepAssassinBehavior {
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
            // C# ProcessTarget：近战 4/5 Attack（DoubleSlash）/ 1/5 RangeAttack（HeavenlySword）
            if fastrand::i32(0..5) == 0 {
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                    attacker_oid: monster.object_id,
                    origin_x: monster.x,
                    origin_y: monster.y,
                    direction: dir,
                    range: ATTACK_RANGE,
                    damage,
                    spell_id: 0,
                });
                return;
            }
            // C# DoubleSlash：近战 + 投射
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            return;
        }

        // C# ProcessTarget：追击中（出近战范围）1/5 RangeAttack（HeavenlySword Line(3)）
        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick && fastrand::i32(0..5) == 0 {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                attacker_oid: monster.object_id,
                origin_x: monster.x,
                origin_y: monster.y,
                direction: dir,
                range: ATTACK_RANGE,
                damage,
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
    }
}
