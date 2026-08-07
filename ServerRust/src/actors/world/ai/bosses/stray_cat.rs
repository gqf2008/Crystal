//! StrayCat（流浪猫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/StrayCat.cs
//! 机制（Attack）：
//!   - 近战（目标距离<=1 且 90% 概率）：
//!     - 90% 普通近战（DC）
//!     - 10% Type=1：目标等级<=怪+5 时推挤 1 格 + 直线 LineAttack(MC, 2)
//!   - 否则（距离>1 或 10% 概率）：Type=2 直线 LineAttack(MC, 2)
//! InAttackRange：2 格十字/对角（同 SpittingSpider）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const LINE_RANGE: i32 = 2;

/// C# InAttackRange（StrayCat）：x/y<=2 且 (x<=1&&y<=1)||(x==y||x%2==y%2)
fn in_cat_range(dx_abs: i32, dy_abs: i32) -> bool {
    if dx_abs > 2 || dy_abs > 2 {
        return false;
    }
    (dx_abs <= 1 && dy_abs <= 1) || (dx_abs == dy_abs || dx_abs % 2 == dy_abs % 2)
}

pub struct StrayCatBehavior;

impl StrayCatBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for StrayCatBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();

        if in_cat_range(dx, dy) && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            // C# range = !InRange(CurrentLocation, Target, 1)（切比雪夫距离 > 1）
            let ranged = max_distance(monster.x, monster.y, target.x, target.y) > 1;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);

            if !ranged && fastrand::i32(0..10) > 0 {
                if fastrand::i32(0..10) > 0 {
                    // 90% 普通近战（DC）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // 10% Type=1：目标等级<=怪+5 且推挤成功 → 直线 MC
                    if target.level as i32 <= monster.level + 5 {
                        ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                            session_id: target.session_id,
                            dir,
                            distance: 1,
                        });
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                            attacker_oid: monster.object_id,
                            origin_x: monster.x,
                            origin_y: monster.y,
                            direction: dir,
                            range: LINE_RANGE,
                            damage,
                            spell_id: 0,
                        });
                    }
                    // 注：C# 推挤返回>0 才发直线；Rust 无法回读推挤结果，此处近似为条件满足即推+线
                }
            } else {
                // Type=2：直线 MC
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                    attacker_oid: monster.object_id,
                    origin_x: monster.x,
                    origin_y: monster.y,
                    direction: dir,
                    range: LINE_RANGE,
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
