//! FlameMage（火焰法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlameMage.cs（继承 RightGuard）
//! 机制：
//!   - 远程火焰弹：命中目标后溅射 2 格内全体（CompleteRangeAttack FindAllTargets(2)）
//!   - 风筝走位（继承 AxeSkeleton/RightGuard：远了追近，近了后退）
//!
//! CompleteRangeAttack（C# :14-30）：FindAllTargets(2, target.location) 全体 Attacked。
//! ProcessTarget（C# :32-83）：标准风筝（dist>=AttackRange 追，否则远离）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;

pub struct FlameMageBehavior;

impl FlameMageBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FlameMageBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            // 命中目标 + 溅射 2 格内全体
            let splash: Vec<crate::actors::world::ai::PlayerSnap> =
                ctx.find_targets_in_range(target.x, target.y, 2, monster.map_index)
                    .into_iter().copied().collect();
            for h in splash {
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: h.session_id,
                    target_object_id: h.object_id,
                    damage,
                    spell_id: 0,
                });
            }
            return;
        }

        // 风筝走位
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
