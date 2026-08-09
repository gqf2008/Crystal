//! TurtleKing（龟丞相）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TurtleKing.cs
//! 机制：
//!   - AttackRange=7, CloseRange=2，可移动追击
//!   - 5 阶段 HP：stage = HP/(MaxHP/5)，每掉一阶 SpawnSlaves（召唤 Turtle 系 8 只）
//!   - 近战：2/5 LineAttack 2 格 / 2/5 LineAttack 3 格 / 1/5 远程 MAC + Dazed
//!   - 远程：1/4 拉拽玩家到身前 / 1/4 自己瞬移到玩家身后 / 其余远程 MAC + Slow+Paralysis
//!
//! ProcessAI（C# TurtleKing.cs:25-41）：5 阶段 SpawnSlaves。
//! Attack（C# :42-123）：近战三形态 / 远程三形态。
//! CompleteRangeAttack（C# :138-156）：FindAllTargets(7) + Slow + Paralysis。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 远程攻击距离（C# AttackRange = 7）
const ATTACK_RANGE: i32 = 7;
/// 近战判定（C# CloseRange = 2）
const CLOSE_RANGE: i32 = 2;
/// 总阶段数（C# _stage = 5）
const TOTAL_STAGES: i32 = 5;
/// 每阶段召唤数（C# count = min(8, 30-SlaveList.Count)）
const SLAVES_PER_STAGE: usize = 8;
/// 召唤池（C# Settings.Turtle1..5）
const SLAVE_NAMES: [&str; 5] = [
    "GiantTurtle",
    "TurtleKing",
    "Turtle",
    "FinialTurtle",
    "GreatTurtle",
];

pub struct TurtleKingBehavior {
    stage: i32,
}

impl TurtleKingBehavior {
    pub fn new() -> Self {
        Self { stage: TOTAL_STAGES }
    }

    fn current_stage(monster: &MonsterState) -> i32 {
        if monster.max_hp < TOTAL_STAGES {
            return TOTAL_STAGES;
        }
        let per_stage = monster.max_hp / TOTAL_STAGES;
        if per_stage <= 0 {
            return TOTAL_STAGES;
        }
        monster.hp / per_stage
    }
}

impl MonsterBehavior for TurtleKingBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // ---- 5 阶段 HP 召唤（C# ProcessAI）----
        let cur_stage = Self::current_stage(monster);
        if cur_stage < self.stage {
            self.spawn_slaves(monster, ctx);
            self.stage = cur_stage;
        }

        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let in_close = dist <= CLOSE_RANGE;
            if in_close {
                // 近战三形态（C# Random(5)）
                let roll = fastrand::i32(0..5);
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                if roll < 2 {
                    // LineAttack 2 格
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else if roll < 4 {
                    // LineAttack 3 格
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                } else {
                    // 远程 MAC + Dazed
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# PoisonTarget 1/8
                        if fastrand::i32(0..8) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::DAZED, 3, damage, 1000),
                        });
                        }
                }
            } else {
                // 远程三形态（C# TurtleKing.cs:78-95：1/4 拉拽玩家 / 1/4 自身瞬移 / 1/2 远程）
                let roll = fastrand::i32(0..4);
                if roll == 0 {
                    // 拉拽玩家到身前（C# Target.Teleport(PointMove(CurrentLocation, Direction, 1))）
                    let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                    ctx.out_player_teleports.push((
                        target.session_id,
                        monster.x + DIR_DX[dir as usize],
                        monster.y + DIR_DY[dir as usize],
                        dir,
                    ));
                } else if roll == 1 {
                    // 自身瞬移到玩家前方 1 格（C# Teleport(PointMove(Target.Location, Target.Direction, 1))；
                    // PlayerSnap 无朝向，用乌龟→玩家方向近似）
                    let dir = direction_towards(target.x, target.y, monster.x, monster.y);
                    ctx.out_moves.push((
                        monster.object_id,
                        target.x + DIR_DX[dir as usize],
                        target.y + DIR_DY[dir as usize],
                        dir,
                    ));
                } else {
                    // 远程 MAC + Slow + Paralysis（C# CompleteRangeAttack）
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# PoisonTarget 1/5
                        if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::SLOW, 15, damage, 1000),
                        });
                        }
                    // C# PoisonTarget 1/5
                        if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::PARALYSIS, 5, damage, 1000),
                        });
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
}

impl TurtleKingBehavior {
    fn spawn_slaves(&self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        for i in 0..SLAVES_PER_STAGE {
            let dir = (i as usize) % 8;
            let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: name.to_string(),
                x: monster.x + DIR_DX[dir] * ((i / 8) as i32 + 1),
                y: monster.y + DIR_DY[dir] * ((i / 8) as i32 + 1),
                is_slave: true,
                summoner_oid: Some(monster.object_id),
            });
        }
    }
}
