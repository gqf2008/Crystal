//! Behemoth（巨兽）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Behemoth.cs
//! 机制：
//!   - AttackRange=10，可移动追击
//!   - 近战（贴身）：3/5 普攻 swipe / 1/5 FireCircle 自身 1 格 AOE / 1/5 推开 4 格 + Dazed
//!     攻击后 Bleeding 15s
//!   - 远程：1/2 追击；1/2 二选一：SpawnSlaves（投掷 huggers） / 远程 DC*3 弹道 + Paralysis
//!   - 死亡时所有 slave 一起死
//!
//! Attack（C# Behemoth.cs:22-100）：近战三形态 / 远程 SpawnSlaves+弹道。
//! SpawnSlaves（C# :168-202）：count = min(8, targets*5 - SlaveList.Count)。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 远程攻击距离（C# AttackRange = 10）
const ATTACK_RANGE: i32 = 10;
/// 近战判定
const MELEE_RANGE: i32 = 1;
/// 召唤池（C# Settings.BehemothMonster1..3，huggers 系）
const SLAVE_NAMES: [&str; 3] = ["BehemothHugger", "Crawler", "BoulderSpirit"];

pub struct BehemothBehavior;

impl BehemothBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for BehemothBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let base = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);

            if dist <= MELEE_RANGE {
                // ---- 近战三形态（C# Random(5)）----
                let roll = fastrand::i32(0..5);
                if roll < 3 {
                    // swipe 普攻
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: base,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else if roll == 3 {
                    // FireCircle：自身 1 格 AOE
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: 1,
                        damage: base,
                        spell_id: 0,
                    });
                } else {
                    // Push back 4 格（C# Behemoth.cs:158 t.Pushed(this, Direction, 4)）+ Dazed
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage: 1,
                        spell_id: 0,
                        attack_type: 1,
                    });
                    ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                        session_id: target.session_id,
                        dir: monster.direction,
                        distance: 4,
                    });
                    // C# PoisonTarget(3, 15, Dazed, 1000)：1/3、15s
                    if fastrand::i32(0..3) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::DAZED, 15, base, 1000),
                        });
                    }
                }
                // 近战命中后 Bleeding 15s（C# PoisonTarget(15, 5, Bleeding)：1/15 概率、值=SP）
                if fastrand::i32(0..15) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::BLEEDING, 15, base, 1000),
                    });
                }
            } else {
                // ---- 远程：1/2 追击；1/2 SpawnSlaves / 弹道 ----
                if fastrand::i32(0..2) == 0 {
                    // 追击
                    let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                    ctx.out_moves.push((monster.object_id, nx, ny, dir));
                    monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                } else if fastrand::i32(0..2) == 0 {
                    // SpawnSlaves：投掷 huggers（数量 = min(8, targets*5)）
                    let targets_count = ctx.find_targets_in_range(monster.x, monster.y, ATTACK_RANGE, monster.map_index).len();
                    let count = (targets_count * 5).min(8);
                    for i in 0..count {
                        let dir = (i as usize) % 8;
                        let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
                        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                            monster_name: name.to_string(),
                            x: monster.x + DIR_DX[dir] * 2,
                            y: monster.y + DIR_DY[dir] * 2,
                            is_slave: true,
                            summoner_oid: Some(monster.object_id),
                        });
                    }
                } else {
                    // 远程 DC*3 弹道 + Paralysis（C# CompleteRangeAttack）
                    let damage = (base * 3).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# FindAllTargets(AttackRange) 全体 Paralysis
                    let hit_sessions: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
                        .iter().map(|p| p.session_id).collect();
                    for sid in hit_sessions {
                        // C# PoisonTarget 1/15
                            if fastrand::i32(0..15) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: sid,
                                poison: Poison::new(PoisonType::PARALYSIS, 5, damage, 1000),
                            });
                            }
                    }
                }
            }
        } else if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C# Die：所有 slave 一起死（由 is_slave 统一清理，调用方处理）
    }
}
