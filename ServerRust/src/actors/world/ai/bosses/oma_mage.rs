//! OmaMage（奥玛法师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/OmaMage.cs
//! 机制：
//!   - 近战 Type0 DC（ACAgility）
//!   - 远程 MC 弹道（MACAgility）→ 命中后 Slow 6s + Frozen 9s
//!   - 全视野攻击范围（ViewRange）
//!
//! Attack（C# :18-58）：近战/远程分支。
//! CompleteRangeAttack（C# :60-72）：命中→Slow 6s + Frozen 9s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct OmaMageBehavior;

impl OmaMageBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for OmaMageBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            if dist <= MELEE_RANGE {
                // 近战 Type0 DC
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 远程 MC 弹道 + Slow + Frozen
                monster.next_attack_tick = ctx.tick_count + 11; // C# +500 额外冷却
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // C# PoisonTarget Slow 6s + Frozen 9s
                // C# PoisonTarget 1/6
                    if fastrand::i32(0..6) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::SLOW, 5, damage, 2000),
                    });
                    }
                // C# PoisonTarget 1/9
                    if fastrand::i32(0..9) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 5, damage, 2000),
                    });
                    }
            }
            return;
        }

        // 追击
        if dist > VIEW_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
