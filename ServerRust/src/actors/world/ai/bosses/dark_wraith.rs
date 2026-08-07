//! DarkWraith（黑暗幽灵）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkWraith.cs
//! 机制：
//!   - 近战范围 1 格；不在近战 1/3 概率直线（4 格、DC*3、冷却 3-8s）；近战 2/3 普攻 / 1/3 直线
//!   - Attack：半径 1 内目标>1 且 1/2 → AOE 全部；否则普攻
//!   - LineAttack：DC*3，冷却 = 3s + random(0..5)*1s（存 behavior 内）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const ATTACK_RANGE: i32 = 4;
const LINE_RANGE: i32 = 4;

pub struct DarkWraithBehavior {
    next_line_tick: u64,
}

impl DarkWraithBehavior {
    pub fn new() -> Self {
        Self { next_line_tick: 0 }
    }
}

impl MonsterBehavior for DarkWraithBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
        let line_ready = ctx.tick_count >= self.next_line_tick;
        let dir = direction_towards(monster.x, monster.y, target.x, target.y);

        // C# ProcessTarget：不在近战范围 → 1/3 直线 / 移动
        if dist > 1 {
            if line_ready && ctx.tick_count >= monster.next_attack_tick && fastrand::i32(0..3) == 0 {
                self.next_line_tick = ctx.tick_count + 30 + fastrand::i32(0..5) as u64 * 10; // 3-8s
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                    attacker_oid: monster.object_id,
                    origin_x: monster.x,
                    origin_y: monster.y,
                    direction: dir,
                    range: LINE_RANGE,
                    damage: damage.saturating_mul(3),
                    spell_id: 0,
                });
                return;
            }
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, d2) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, d2));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            // C# 2/3 普攻 / 1/3 直线
            if fastrand::i32(0..3) > 0 {
                // C# Attack：半径 1 内目标>1 且 1/2 → AOE 全部；否则普攻
                let nearby = ctx.find_targets_in_range(monster.x, monster.y, 1, monster.map_index);
                if nearby.len() > 1 && fastrand::i32(0..2) > 0 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: 1,
                        damage,
                        spell_id: 0,
                    });
                } else {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            } else {
                self.next_line_tick = ctx.tick_count + 30 + fastrand::i32(0..5) as u64 * 10;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Line {
                    attacker_oid: monster.object_id,
                    origin_x: monster.x,
                    origin_y: monster.y,
                    direction: dir,
                    range: LINE_RANGE,
                    damage: damage.saturating_mul(3),
                    spell_id: 0,
                });
            }
        }
    }
}
