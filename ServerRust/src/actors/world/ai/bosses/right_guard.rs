//! RightGuard（右护卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RightGuard.cs
//! 机制：
//!   - 远程范围攻击型护卫：AttackRange=8，可移动追击
//!   - 近战（贴身）：ObjectAttack DC + DefenceType.MACAgility
//!   - 远程：ObjectRangeAttack DC 弹道 + DefenceType.MAC，延迟按距离（50ms/格）
//!     远程攻击冷却 +500ms
//!
//! Attack（C# :26-60）：!ranged→DC MACAgility；ranged→DC MAC（AttackTime+500）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 8;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct RightGuardBehavior;

impl RightGuardBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for RightGuardBehavior {
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
                // 近战：MACAgility
                monster.next_attack_tick = ctx.tick_count + 6;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 远程弹道：MAC，冷却 +500ms（C# AttackTime + AttackSpeed + 500）
                monster.next_attack_tick = ctx.tick_count + 10;
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

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
