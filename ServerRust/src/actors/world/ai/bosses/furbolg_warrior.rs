//! FurbolgWarrior（熊人战士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FurbolgWarrior.cs
//! 机制：
//!   - 棋盘格近战范围（最远 2 格）
//!   - 远程（>1 格）：前方 2 格穿透，每格 1/10 暴击（+50% 伤害）
//!   - 近战：半圆 6 方向横扫（PreviousDir 起 6 格），每向 1/10 暴击
//!
//! Attack（C# :30-138）：ranged→方向上 2 格；else 半圆 6 方向。

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

pub struct FurbolgWarriorBehavior;

impl FurbolgWarriorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FurbolgWarriorBehavior {
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

        let base = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        let ranged = dx > 1 || dy > 1;

        if ranged {
            // 前方 2 格穿透（C# i=1..=2）
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            for i in 1..=2 {
                let cx = monster.x + DIR_DX[dir as usize] * i;
                let cy = monster.y + DIR_DY[dir as usize] * i;
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(cx, cy, 0, monster.map_index)
                        .into_iter().copied().collect();
                let mut dmg = base;
                if fastrand::i32(0..10) == 0 {
                    dmg += base / 2; // 暴击 +50%
                }
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage: dmg,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            }
        } else {
            // 半圆 6 方向横扫（C# PreviousDir 起 6 向）
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            let mut d = (dir + 7) % 8; // PreviousDir
            for _ in 0..6 {
                let cx = monster.x + DIR_DX[d as usize];
                let cy = monster.y + DIR_DY[d as usize];
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(cx, cy, 0, monster.map_index)
                        .into_iter().copied().collect();
                let mut dmg = base;
                if fastrand::i32(0..10) == 0 {
                    dmg += base / 2;
                }
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage: dmg,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
                d = (d + 1) % 8; // NextDir
            }
        }
    }
}
