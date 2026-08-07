//! MinotaurKing（牛头王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/MinotaurKing.cs（继承 RightGuard，AttackRange=6）
//! 机制：
//!   - 继承 RightGuard 的远程弹道机制，但 CompleteRangeAttack 为 AOE：
//!     远程弹道命中后在目标点 FindAllTargets(3) 范围内全体受击（MinotaurKing 独有）
//!   - 近战同 RightGuard
//!
//! CompleteRangeAttack（C# :20-34）：FindAllTargets(3, target.CurrentLocation) 逐个 Attacked。
//! Attack 继承 RightGuard（AttackRange override 为 6）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// C# override AttackRange = 6
const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// 远程 AOE 命中半径（C# FindAllTargets(3)）
const SPLASH_RADIUS: i32 = 3;

pub struct MinotaurKingBehavior;

impl MinotaurKingBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for MinotaurKingBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            if dist <= MELEE_RANGE {
                // 近战：继承 RightGuard 的 MACAgility
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 远程：命中原目标 + 目标点 3 格内 AOE（C# CompleteRangeAttack 独有）
                monster.next_attack_tick = ctx.tick_count + 10;
                // 先以 Aoe 在目标点施加溅射（含主目标）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: SPLASH_RADIUS,
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
