//! LeftGuard（左护卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/LeftGuard.cs（继承 RightGuard）
//! 机制：
//!   - 弓箭手型：可移动追击，AttackRange 由 RightGuard 基类提供（远距离弹道）
//!   - 近战（贴身）：ObjectAttack DC + DefenceType.MACAgility
//!   - 远程：ObjectRangeAttack DC 弹道 + DefenceType.MAC，延迟按距离（50ms/格）
//!     远程攻击冷却 +500ms（C# AttackTime + AttackSpeed + 500）
//!
//! Attack（C# LeftGuard.cs:13-49）：ranged 分支 + 弹道延迟。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 近战判定
const MELEE_RANGE: i32 = 1;

pub struct LeftGuardBehavior;

impl LeftGuardBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for LeftGuardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let in_melee = dist <= MELEE_RANGE;

        if ctx.tick_count < monster.next_attack_tick {
            // 远程时仍可移动拉开/贴近
            if !in_melee && ctx.tick_count >= monster.next_move_tick {
                // 弓箭手保持中距离：贴身时后退，远了追近
                let (nx, ny, dir) = if dist < 3 {
                    step_away(monster.x, monster.y, target.x, target.y)
                } else {
                    step_toward(monster.x, monster.y, target.x, target.y)
                };
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);

        if in_melee {
            // 近战：MACAgility
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        } else {
            // 远程弹道：MAC（C# AttackTime + AttackSpeed + 500，冷却更长）
            monster.next_attack_tick = ctx.tick_count + 10;
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
        }
    }
}
