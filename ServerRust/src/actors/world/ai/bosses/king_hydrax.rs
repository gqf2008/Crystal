//! KingHydrax（海德拉王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/KingHydrax.cs
//! 机制：
//!   - 近战 DC 单体（ACAgility）
//!   - 远程 1/2 MC 弹道（Type0，命中 + Paralysis）；1/2 MC 弹道（Type1，延迟 + Green）
//!   - 死亡延迟 1s 后 SpawnSlaves（召唤 2 只 KingHydraxMob）
//!
//! Attack（C# :18-69）：近战/远程双弹道分支。
//! CompleteRangeAttack（C# :71-90）：poison→Green；else→Paralysis。
//! Die（C# :111-136）：CompleteDeath→SpawnSlaves(2)。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
/// C# Settings.KingHydraxMob（召唤物名）
const SLAVE_MOB_NAME: &str = "KingHydraxMob";

pub struct KingHydraxBehavior;

impl KingHydraxBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for KingHydraxBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            // 近战 DC 单体
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
        } else if dist <= VIEW_RANGE {
            // 远程双弹道
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 6;
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                if fastrand::i32(0..2) == 0 {
                    // Type0 即时弹道 + Paralysis
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# PoisonTarget 1/3
                        if fastrand::i32(0..3) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::PARALYSIS, 10, damage, 1000),
                        });
                        }
                } else {
                    // Type1 延迟弹道 + Green
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 1,
                    });
                    // C# PoisonTarget 1/2
                        if fastrand::i32(0..2) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::GREEN, 10, damage, 1000),
                        });
                        }
                }
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// 死亡召唤 2 只 KingHydraxMob（C# CompleteDeath→SpawnSlaves）
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        for i in 0..2 {
            // 召唤在身前/身边
            let (ox, oy) = match i {
                0 => (monster.x, monster.y + 1),
                _ => (monster.x + 1, monster.y),
            };
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: SLAVE_MOB_NAME.to_string(),
                x: ox,
                y: oy,
                is_slave: true,
            });
        }
    }
}
