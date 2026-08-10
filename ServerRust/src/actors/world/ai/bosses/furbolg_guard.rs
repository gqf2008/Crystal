//! FurbolgGuard（兽人守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FurbolgGuard.cs
//! 机制：
//!   - AttackRange=8
//!   - 近战（dist<=1）：LineAttack(damage, 3)；若 dist<=2 且 1/3 → 带推挤 2 + JumpBack(3)（反向 3 格 ObjectBackStep）
//!   - 远程：ProjectileAttack（Range MC 近似）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 8;
const LINE_RANGE: i32 = 3;

pub struct FurbolgGuardBehavior;

impl FurbolgGuardBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FurbolgGuardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            if dist <= 1 {
                // C# 近战：LineAttack(3)；1/3 时带推挤 + JumpBack(3)
                if dist <= 2 && fastrand::i32(0..3) == 0 {
                    // C# LineAttack push：距离 = distance-1 = 2（MonsterObject.cs:3631）
                    ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                        session_id: target.session_id,
                        dir,
                        distance: 2,
                    });
                    // C# JumpBack(3)：反向 3 格（out_backsteps 应用时校验 walkable + 广播 ObjectBackStep）
                    let back_dir = (dir as i32 + 4).rem_euclid(8) as u8;
                    ctx.out_backsteps.push((monster.object_id, back_dir, 3));
                }
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

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
