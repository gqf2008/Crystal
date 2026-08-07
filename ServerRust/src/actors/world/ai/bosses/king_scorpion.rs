//! KingScorpion（蝎子王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/KingScorpion.cs
//! 机制：
//!   - 棋盘格近战范围（x==y||x%2==y%2，最远 2 格）
//!   - 攻击两形态：前方 2 格有目标或 1/5 概率 → MC LineAttack(2) MACAgility；否则 DC LineAttack(2) ACAgility
//!
//! Attack（C# :28-78）：range||Random(5)==0→MC 直线；else DC 直线。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

fn in_range(dx_abs: i32, dy_abs: i32) -> bool {
    if dx_abs > 2 || dy_abs > 2 {
        return false;
    }
    (dx_abs <= 1 && dy_abs <= 1) || (dx_abs == dy_abs || dx_abs % 2 == dy_abs % 2)
}

pub struct KingScorpionBehavior;

impl KingScorpionBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for KingScorpionBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        if !in_range(dx, dy) {
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

        // 前方 2 格有目标 or 1/5 → MC 直线；否则 DC 直线
        let use_mc = fastrand::i32(0..5) == 0;
        let dir = direction_towards(monster.x, monster.y, target.x, target.y);
        let cx = monster.x + DIR_DX[dir as usize] * 2;
        let cy = monster.y + DIR_DY[dir as usize] * 2;
        let hits: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(cx, cy, 2, monster.map_index)
                .into_iter().copied().collect();
        let damage = if use_mc {
            crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1)
        } else {
            crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
        };
        if hits.is_empty() {
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: if use_mc { 1 } else { 0 },
            });
        } else {
            for h in &hits {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: h.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: if use_mc { 1 } else { 0 },
                });
            }
        }
    }
}
