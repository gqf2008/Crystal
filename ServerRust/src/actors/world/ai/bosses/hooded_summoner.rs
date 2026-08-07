//! HoodedSummoner（兜帽召唤师）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HoodedSummoner.cs
//! 机制：
//!   - 全视野攻击（InAttackRange = ViewRange），风筝走位（FearTime 5s）
//!   - Attack 随机 6 选：0-3 远程 MC 弹道(MAC)；4/5 召唤 Slave（15s 冷却）
//!     召唤 type 0 → ScrollMob1/2；type 1 → ScrollMob3/4
//!   - 过近 WalkAway，远了追近
//!
//! Attack（C# :23-91）：Random(6)；0-3 RangedAttack；4/5 SpawnSlaves（SlaveSpawnTime 冷却）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
/// SlaveSpawn 冷却（C# Settings.Second * 15 = 15s = 150 ticks）
const SLAVE_COOLDOWN_TICKS: u64 = 150;
/// FearTime 持续（C# Envir.Time + 5000）
const FEAR_TICKS: u64 = 50;

/// 召唤物候选名（C# Settings.ScrollMob1..4，对齐默认配置）
const SCROLL_MOBS_A: [&str; 2] = ["Hugger", "BugBagMaggot"];
const SCROLL_MOBS_B: [&str; 2] = ["SpittingSpider", "PoisonHugger"];

pub struct HoodedSummonerBehavior {
    /// 下次可召唤 tick（C# SlaveSpawnTime）
    slave_spawn_tick: u64,
    fear_end_tick: u64,
}

impl HoodedSummonerBehavior {
    pub fn new() -> Self {
        Self { slave_spawn_tick: 0, fear_end_tick: 0 }
    }
}

impl MonsterBehavior for HoodedSummonerBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count < self.fear_end_tick
            && ctx.tick_count >= monster.next_attack_tick
        {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let roll = fastrand::i32(0..6);

            match roll {
                0..=3 => {
                    // 远程 MC 弹道（MAC）
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                }
                4 | 5 => {
                    if ctx.tick_count >= self.slave_spawn_tick {
                        // 召唤 Slave（C# SpawnSlaves，1 只）
                        self.slave_spawn_tick = ctx.tick_count + SLAVE_COOLDOWN_TICKS;
                        let mobs = if roll == 4 { &SCROLL_MOBS_A } else { &SCROLL_MOBS_B };
                        let name = mobs[fastrand::usize(0..2)];
                        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                            monster_name: name.to_string(),
                            x: target.x + fastrand::i32(-2..=2),
                            y: target.y + fastrand::i32(-2..=2),
                            is_slave: true,
                        });
                    } else {
                        // 冷却中：退化为远程弹道
                        let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            target_object_id: target.object_id,
                            damage,
                            spell_id: 0,
                        });
                    }
                }
                _ => {}
            }
            return;
        }

        // 刷新 FearTime（C# FearTime = Envir.Time + 5000）
        self.fear_end_tick = ctx.tick_count + FEAR_TICKS;

        // 走位：过近拉开，远了追近
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist < VIEW_RANGE {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                step_toward(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
