//! TrollBomber（巨魔投弹手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TrollBomber.cs（继承 AxeSkeleton）
//! 机制：
//!   - 继承 AxeSkeleton 远程掷弹（AttackRange=6，风筝走位）
//!   - CompleteRangeAttack 独有：投弹命中后目标点 2 格 AOE，
//!     主目标全额伤害，溅射目标半额伤害（炸弹 AOE）
//!
//! CompleteRangeAttack（C# :12-28）：FindAllTargets(2, target.loc)；
//!   主目标 damage，其他 damage/2。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 继承 AxeSkeleton AttackRange=6
const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
/// 投弹 AOE 半径（C# FindAllTargets(2)）
const SPLASH_RADIUS: i32 = 2;

pub struct TrollBomberBehavior;

impl TrollBomberBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for TrollBomberBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            // 投弹：主目标全额 + 目标点 2 格 AOE 半额（C# CompleteRangeAttack）
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let full = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            let half = (full / 2).max(1);

            // 主目标全额
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage: full,
                spell_id: 0,
            });
            // 目标点 AOE 半额（含主目标二次受击近似溅射）
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: target.x,
                center_y: target.y,
                radius: SPLASH_RADIUS,
                damage: half,
                spell_id: 0,
            });
            return;
        }

        // 风筝走位（继承 AxeSkeleton）
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
