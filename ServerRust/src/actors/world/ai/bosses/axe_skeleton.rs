//! AxeSkeleton（掷斧骷髅）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/AxeSkeleton.cs
//! 机制：
//!   - 远程掷斧：AttackRange=6，ProjectileAttack 弹道
//!   - 风筝走位：在射程内时保持距离（远了追近，近了后退），FearTime 5s 控制攻击/逃跑切换
//!   - 贴身仍可攻击（InAttackRange 仅按距离判定，不分远近）
//!
//! Attack（C# :28-48）：ObjectRangeAttack + ProjectileAttack。
//! ProcessTarget（C# :50-101）：dist>=AttackRange 追击，否则远离（Walk 远离方向）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 攻击范围（C# AttackRange = 6）
const ATTACK_RANGE: i32 = 6;
/// 视野范围
const VIEW_RANGE: i32 = 15;

pub struct AxeSkeletonBehavior;

impl AxeSkeletonBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for AxeSkeletonBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // 射程内且冷却好了 → 掷斧
        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            return;
        }

        // 走位：远了追近，近了（<3）后退保持射程（C# ProcessTarget 风筝）
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                // 中距离维持不动（已在最佳射程）
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
