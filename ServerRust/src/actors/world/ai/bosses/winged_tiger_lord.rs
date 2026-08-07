//! WingedTigerLord（飞虎王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/WingedTigerLord.cs
//! 机制：
//!   - 连招系统：stomp/tornado 标志位被设置后下一次攻击执行该连招
//!   - 近战 Random(2)：0=双连斩（Type0，两次 DC）；1=双手斩（Type1，一次 DC）
//!     * 近战后 1/5 设 stomp=true；1/2 设 tornado=true
//!   - stomp 连招（Type2）：周围 8 格 AOE DC + Paralysis
//!   - tornado 连招（Type0 远程）：目标周围 1 格 AOE DC + Dazed
//!
//! Attack（C# :31-146）：连招标志优先；近战随机双类型。
//! CompleteRangeAttack（C# :148-161）：Dazed。
//! CompleteAttack（C# :163-182）：Stomp→Paralysis。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 5;
const MELEE_RANGE: i32 = 1;

pub struct WingedTigerLordBehavior {
    /// 下次近战执行 stomp 连招（C# stomp 标志）
    pending_stomp: bool,
    /// 下次远程执行 tornado 连招（C# tornado 标志）
    pending_tornado: bool,
}

impl WingedTigerLordBehavior {
    pub fn new() -> Self {
        Self { pending_stomp: false, pending_tornado: false }
    }
}

impl MonsterBehavior for WingedTigerLordBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // ---- 远程 tornado 连招 ----
        if dist > MELEE_RANGE && self.pending_tornado && dist <= ATTACK_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 8;
                self.pending_tornado = false;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // C# FindAllTargets(1, Target.CurrentLocation) AOE
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(target.x, target.y, 1, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        target_object_id: h.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# PoisonTarget Dazed
                    let poison_time = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(2);
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: h.session_id,
                        poison: Poison::new(PoisonType::DAZED, 2, poison_time, 2000),
                    });
                }
            }
            return;
        }

        // 远程且无 tornado：等待接近
        if dist > MELEE_RANGE {
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + 2;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        // ---- 近战攻击 ----
        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 6;

            // stomp 连招优先
            if self.pending_stomp {
                self.pending_stomp = false;
                monster.next_attack_tick = ctx.tick_count + 8;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // C# 周围 8 格 AOE
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, MELEE_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 2,
                    });
                    // C# PoisonTarget 1/2
                        if fastrand::i32(0..2) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: h.session_id,
                            poison: Poison::new(PoisonType::PARALYSIS, 5, damage, 2000),
                        });
                        }
                }
                return;
            }

            // 普通近战
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            match fastrand::i32(0..2) {
                0 => {
                    // Type0 双连斩（两次 DC）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    let damage2 = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: damage2,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
                _ => {
                    // Type1 双手斩（一次 DC）
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
            }

            // 设置连招标志（C# Random(5)==0→stomp；Random(2)==0→tornado）
            if fastrand::i32(0..5) == 0 {
                self.pending_stomp = true;
            }
            if fastrand::i32(0..2) == 0 {
                self.pending_tornado = true;
            }
        }
    }
}
