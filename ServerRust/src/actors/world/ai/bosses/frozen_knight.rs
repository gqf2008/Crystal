//! FrozenKnight（冰霜骑士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FrozenKnight.cs
//! 机制：
//!   - InAttackRange：2 格十字/对角（同 SpittingSpider）
//!   - 近战且 2/3：Halfmoon 弧形（DC，用 AOE 半径 1 近似）
//!   - 否则：远程 MC + CompleteRangeAttack：FindAllTargets(2, 目标位置) AOE（以目标为中心半径 2）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MELEE_AOE_RADIUS: i32 = 1;
const RANGE_AOE_RADIUS: i32 = 2;

/// C# InAttackRange：2 格十字/对角
fn in_knight_range(dx_abs: i32, dy_abs: i32) -> bool {
    if dx_abs > 2 || dy_abs > 2 {
        return false;
    }
    (dx_abs <= 1 && dy_abs <= 1) || (dx_abs == dy_abs || dx_abs % 2 == dy_abs % 2)
}

pub struct FrozenKnightBehavior;

impl FrozenKnightBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FrozenKnightBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if in_knight_range(dx, dy) && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# !range && Random.Next(3) > 0：近战 2/3 Halfmoon / 1/3 远程
            let melee = dist <= 1 && fastrand::i32(0..3) > 0;
            if melee {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: MELEE_AOE_RADIUS,
                    damage,
                    spell_id: 0,
                });
            } else {
                // C# 远程：RangeDamage(MC) + CompleteRangeAttack FindAllTargets(2, target)
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: RANGE_AOE_RADIUS,
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
