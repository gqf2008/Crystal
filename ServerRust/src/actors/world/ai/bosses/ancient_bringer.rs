//! AncientBringer（远古召唤者）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/AncientBringer.cs
//! 机制：AttackRange=12，远近双模式 + 召唤蝙蝠
//!   - 近战（<=2）：4/5 DC PoisonLineAttack(2,ACAgility)；1/5 DC*2 PoisonLineAttack(2,ACAgility,poison)
//!     → poison 时附加 Paralysis 5s
//!   - 远程（>2）：9/10 MC 弹道(ACAgility) + 目标点 4 格 AOE；
//!     1/10 MC*2 + 目标点 5 格 AOE + 召唤 6 只 AncientBat
//!
//! Attack（C# :28-90）：!ranged→4/5 普通 / 1/5 DC*2+poison(Paralysis)；
//!   ranged→9/10 MC AOE(4) / 1/10 MC*2 AOE(5)+SpawnSlaves。
//! CompleteRangeAttack（C# :92-107）：FindAllTargets(range, target.loc) 逐个 Attacked。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// C# AttackRange = 12
const ATTACK_RANGE: i32 = 12;
const VIEW_RANGE: i32 = 15;
/// 近战判定（C# InRange(,2)）
const MELEE_RANGE: i32 = 2;
/// PoisonLineAttack 距离（C# PoisonLineAttack(2)）
const LINE_RANGE: i32 = 2;
/// 召唤蝙蝠上限（C# Min(6, 40-SlaveList.Count)）
const MAX_BATS: usize = 6;
/// 召唤物名（C# Settings.AncientBatName）
const BAT_NAME: &str = "AncientBat";

pub struct AncientBringerBehavior;

impl AncientBringerBehavior {
    pub fn new() -> Self { Self }
}

impl MonsterBehavior for AncientBringerBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            if dist <= MELEE_RANGE {
                // 近战 PoisonLineAttack(2)：4/5 普通 / 1/5 DC*2 + Paralysis
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let strong = fastrand::i32(0..5) == 0;
                let max_dc = if strong { monster.max_dmg * 2 } else { monster.max_dmg };
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, max_dc, 0).max(1);
                // 直线 2 格（含主目标）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                let dx = DIR_DX[dir as usize];
                let dy = DIR_DY[dir as usize];
                let hits: Vec<crate::actors::world::ai::PlayerSnap> = ctx
                    .find_targets_in_range(monster.x, monster.y, LINE_RANGE, monster.map_index)
                    .into_iter().copied()
                    .filter(|p| {
                        let rx = p.x - monster.x;
                        let ry = p.y - monster.y;
                        (rx == 0 && dy == 0) || (ry == 0 && dx == 0)
                            || (rx.signum() == dx.signum() && ry.signum() == dy.signum() && rx.abs() == ry.abs())
                    })
                    .collect();
                let targets = if hits.is_empty() { vec![target] } else { hits };
                for h in targets {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: h.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                    // 强攻附加 Paralysis 5s（C# PoisonTarget 5,5,Paralysis,2000）
                    if strong {
                        // C# PoisonTarget 1/5
                            if fastrand::i32(0..5) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: h.session_id,
                                poison: Poison::new(PoisonType::PARALYSIS, 5, poison_sc_value(monster), 2000),
                            });
                            }
                    }
                }
            } else {
                // 远程：9/10 MC AOE(4)；1/10 MC*2 AOE(5) + 召唤蝙蝠
                let rare = fastrand::i32(0..10) == 0;
                monster.next_attack_tick = ctx.tick_count + if rare { 12 } else { 8 };
                let max_mc = if rare { monster.max_mac * 2 } else { monster.max_mac };
                let damage = crate::combat::attack::get_attack_power(monster.min_mac, max_mc, 0).max(1);
                let splash = if rare { 5 } else { 4 };
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: splash,
                    damage,
                    spell_id: 0,
                });
                // 1/10 召唤 AncientBat（C# SpawnSlaves）
                if rare {
                    // #1441：C# SpawnSlaves count = min(6, 40 - SlaveList.Count)
                    for _ in 0..slave_spawn_count(MAX_BATS, ctx.slave_count, 40) {
                        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                            monster_name: BAT_NAME.to_string(),
                            x: target.x + fastrand::i32(-3..=3),
                            y: target.y + fastrand::i32(-3..=3),
                            is_slave: true,
                            summoner_oid: Some(monster.object_id),
                        });
                    }
                }
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
