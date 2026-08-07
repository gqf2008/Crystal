//! AxePlant（斧头植物）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/AxePlant.cs
//! 机制：InAttackRange 3 格十字/对角（x==y||x%2==y%2，x/y<=3）；
//!      Attack：TriangleAttack(3, 1, 800)（窄锥=Line 3 近似）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const LINE_RANGE: i32 = 3;

/// C# InAttackRange：3 格十字/对角
fn in_axe_range(dx_abs: i32, dy_abs: i32) -> bool {
    dx_abs <= 3 && dy_abs <= 3 && (dx_abs == dy_abs || dx_abs % 2 == dy_abs % 2)
}

pub struct AxePlantBehavior;

impl AxePlantBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for AxePlantBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();

        if in_axe_range(dx, dy) && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                attacker_oid: monster.object_id,
                origin_x: monster.x,
                origin_y: monster.y,
                direction: dir,
                range: LINE_RANGE,
                damage,
                spell_id: 0,
            });
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
