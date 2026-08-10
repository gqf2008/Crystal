//! ManectricClaw（雷兽之爪）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ManectricClaw.cs
//! 机制：
//!   - AttackRange=3
//!   - 远程或 5s 冷却到：ObjectRangeAttack → IceThrust：前方 3x3（j<=1 近 DC / j==2 远 MC）
//!     + 每命中 1/5 减速（4s）+ 1/5 冰冻（2s，tick 1000）
//!   - 否则：base.Attack 近战

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const ATTACK_RANGE: i32 = 3;
const THRUST_COOLDOWN: u64 = 50; // 5s

pub struct ManectricClawBehavior {
    next_thrust_tick: u64,
}

impl ManectricClawBehavior {
    pub fn new() -> Self {
        Self { next_thrust_tick: 0 }
    }
}

impl MonsterBehavior for ManectricClawBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# ranged || 冷却到 → IceThrust
            if dist > 1 || ctx.tick_count >= self.next_thrust_tick {
                self.next_thrust_tick = ctx.tick_count + THRUST_COOLDOWN;
                // C# IceThrust：3 列（prevdir/dir/nextdir 起点）× 3 深 = 9 格；j<=1 近 DC / j==2 远 MC
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                monster.direction = dir;
                let cells3 = ice_thrust_cells(monster.x, monster.y, dir, 3);
                let all: Vec<(i32, i32)> = cells3.iter().map(|&(x, y, _)| (x, y)).collect();
                let near: Vec<(i32, i32)> = cells3.iter().filter(|&&(_, _, j)| j <= 1).map(|&(x, y, _)| (x, y)).collect();
                let far: Vec<(i32, i32)> = cells3.iter().filter(|&&(_, _, j)| j == 2).map(|&(x, y, _)| (x, y)).collect();
                let near_dmg = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                let far_dmg = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Cells {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    cells: near,
                    damage: near_dmg,
                    spell_id: 0,
                    attack_type: 2,
                });
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Cells {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    cells: far,
                    damage: far_dmg,
                    spell_id: 0,
                    attack_type: 2,
                });
                // C# PoisonTarget(5, 4, Slow, 1000) / (5, 2, Frozen, 1000)：每命中 1/5
                let hit: Vec<u64> = ctx.find_targets_in_cells(&all, monster.map_index)
                    .iter().map(|p| p.session_id).collect();
                for sid in hit {
                    if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::SLOW, 4, 0, 1000),
                        });
                    }
                    if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::FROZEN, 2, 0, 1000),
                        });
                    }
                }
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
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
