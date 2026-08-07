//! BoneSpearman（骷髅枪兵，AI 29）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/BoneSpearman.cs
//! 机制：十字/对角判定（2 格内），LineAttack(damage, 2, 250)

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 10;
const LINE_RANGE: i32 = 2;

pub struct BoneSpearmanBehavior;

impl BoneSpearmanBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for BoneSpearmanBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        // C# InAttackRange：x/y <=2 且 (x<=1&&y<=1 || x==y || x%2==y%2)
        let in_range = dx.max(dy) <= 2 && ((dx <= 1 && dy <= 1) || dx == dy || dx % 2 == dy % 2);

        if in_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            // C# LineAttack(damage, 2)
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

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
