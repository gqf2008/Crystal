//! OmaWitchDoctor（奥玛巫医）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/OmaWitchDoctor.cs
//! 机制：
//!   - 近战（dist<=1）：DC（ACAgility）
//!   - 远程且直线判定（6 格内 x==0||y==0||x==y）：LineAttack(MC, 6, MACAgility)，攻速+500ms
//!   - 远程非直线：RangeDamage（MC，MAC），攻速+500ms
//!   - ProcessTarget：不在直线射程才移动

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const LINE_RANGE: i32 = 6;

/// C# InLineAttackRange：6 格内 (x==0)||(y==0)||(x==y)
fn in_line_range(dx_abs: i32, dy_abs: i32) -> bool {
    dx_abs <= LINE_RANGE && dy_abs <= LINE_RANGE && (dx_abs == 0 || dy_abs == 0 || dx_abs == dy_abs)
}

pub struct OmaWitchDoctorBehavior;

impl OmaWitchDoctorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for OmaWitchDoctorBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            if dist <= 1 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else if in_line_range(dx, dy) {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                    attacker_oid: monster.object_id,
                    origin_x: monster.x,
                    origin_y: monster.y,
                    direction: dir,
                    range: LINE_RANGE,
                    damage,
                    spell_id: 0,
                });
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
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

        // C# ProcessTarget：不在直线射程才移动
        if ctx.tick_count >= monster.next_move_tick && !in_line_range(dx, dy) {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
