//! ManectricKing（雷电王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ManectricKing.cs
//! 机制：
//!   - HP<20% 周期 MassAttack：FindAllTargets(7) 全体 MC 弹道（按距离延迟）
//!   - 近战 AttackRange=3 的十字/对角判定：
//!     * 贴身 1/3 概率 Type1 DC LineAttack（推回）
//!     * 其余 Type0 MC LineAttack(3)
//!   - 风筝走位（恐惧期远离，否则追击）
//!
//! Attack（C# :31-91）：HP<20&&MassAttackTime<Time→全体弹道；贴身1/3 DC线；else MC线。
//! MassAttackTime = Time + 2000 + Random(5)*1000。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 3;
const MASS_RADIUS: i32 = 7;

pub struct ManectricKingBehavior {
    /// 下次 MassAttack 的 tick（C# MassAttackTime）
    next_mass_tick: u64,
}

impl ManectricKingBehavior {
    pub fn new() -> Self {
        Self { next_mass_tick: 0 }
    }
}

impl MonsterBehavior for ManectricKingBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        let hp_pct = if monster.max_hp > 0 { monster.hp * 100 / monster.max_hp } else { 100 };

        // ---- HP<20% MassAttack：全体 MC 弹道 ----
        if hp_pct < 20 && ctx.tick_count >= self.next_mass_tick {
            // C# MassAttackTime = Time + 2000 + Random(5)*1000
            self.next_mass_tick = ctx.tick_count + 20 + fastrand::u64(0..5) * 10;
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
            let targets: Vec<crate::actors::world::ai::PlayerSnap> =
                ctx.find_targets_in_range(monster.x, monster.y, MASS_RADIUS, monster.map_index)
                    .into_iter().copied().collect();
            for t in targets {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: t.session_id,
                    target_object_id: t.object_id,
                    damage,
                    spell_id: 0,
                });
            }
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            return;
        }

        // ---- 攻击范围判定（十字/对角，AttackRange=3）----
        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let close = dist <= ATTACK_RANGE - 1;
            if close && fastrand::i32(0..3) == 0 {
                // Type1 DC LineAttack（带推回，用 attack_type=1 标记）
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 1,
                });
            } else {
                // Type0 MC LineAttack(3)
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                    ctx.find_targets_in_range(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
                        .into_iter().copied().collect();
                for h in hits {
                    let hd = direction_towards(monster.x, monster.y, h.x, h.y);
                    if hd == dir {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                    }
                }
            }
            return;
        }

        // 风筝走位：太近远离，太远追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 2 {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                return;
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
