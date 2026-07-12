//! IceGuard（冰守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/IceGuard.cs
//! 机制：
//!   - 近战（<=1 格）：DC + MAC 防御
//!   - 远程（>1 格，射程 8）：2/3 冰攻（MC + Slow + Frozen），1/3 火攻（MC 纯伤）
//!   - 远程冷却 +500ms
//!
//! Attack（C# :26-75）：!ranged→DC MAC；ranged→2/3 冰攻(Slow+Frozen) / 1/3 火攻。
//! CompleteRangeAttack（C# :78-94）：poison→PoisonTarget Slow 5s + Frozen 3s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 8;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct IceGuardBehavior;

impl IceGuardBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for IceGuardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        } else if dist <= ATTACK_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                // 远程冷却 +500ms（C# AttackSpeed + 500）
                monster.next_attack_tick = ctx.tick_count + 10;
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                let is_ice = fastrand::i32(0..3) > 0; // 2/3 冰攻
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: if is_ice { 0 } else { 1 },
                });
                if is_ice {
                    // Slow 5s + Frozen 3s（C# PoisonTarget Slow + Frozen）
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::SLOW, 5, 5, 1000),
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 3, 0, 1000),
                    });
                }
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
