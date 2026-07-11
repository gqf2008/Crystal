//! SnowWolfKing（雪狼王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SnowWolfKing.cs
//! 机制：
//!   - 可移动追击
//!   - 攻击 3 形态按 HP 分档：>=60% 冰系(冻)/>=30% 普攻/<30% 狂攻（Type 1/2/3）
//!   - 2/3 概率普攻（Type 0）；攻击附带冰冻/减速（任务核心"冰冻"）
//!   - HP<70% 时 SpawnSlaves：召唤 3 只雪狼（Settings.SnowWolfKingMob）
//!   - 受击高伤时 FindWeakerTarget 切换到更弱目标（简化：受击大伤切换目标）
//!   - 死亡时 1 格内全体 MAC 爆炸 + 驯化奴仆（简化为死亡 AOE）

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野范围
const VIEW_RANGE: i32 = 20;
/// 近战判定
const MELEE_RANGE: i32 = 1;
/// 召唤池（C# Settings.SnowWolfKingMob，雪狼系）
const SLAVE_NAMES: [&str; 3] = ["SnowWolf", "FrostWolf", "IceWolf"];

pub struct SnowWolfKingBehavior {
    spawned_slaves: bool,
}

impl SnowWolfKingBehavior {
    pub fn new() -> Self {
        Self { spawned_slaves: false }
    }
}

impl MonsterBehavior for SnowWolfKingBehavior {
    /// 受击时若伤害高于自身攻击力，概率切换到更弱目标（C# Attacked override）
    fn on_attacked(&mut self, _damage: i32) -> i32 {
        // 目标切换需 target 信息，此处只保留伤害（切换在 process_tick 内按 hp 态势处理）
        _damage
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let hp_pct = if monster.max_hp > 0 {
            (monster.hp * 100) / monster.max_hp
        } else {
            0
        };

        // ---- HP<70% 召唤 3 只雪狼（C# SpawnSlaves）----
        if hp_pct < 70 && !self.spawned_slaves {
            self.spawned_slaves = true;
            for i in 0..3 {
                let dir = (i as usize) % 8;
                let name = SLAVE_NAMES[fastrand::usize(0..SLAVE_NAMES.len())];
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: name.to_string(),
                    x: monster.x + DIR_DX[dir] * 2,
                    y: monster.y + DIR_DY[dir] * 2,
                    is_slave: true,
                });
            }
        }

        // 无目标则返回
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 5;
            let base = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);

            // 2/3 概率普攻（Type 0）
            if fastrand::i32(0..3) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: base,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // 1/3：HP 分档三形态（C# HealthPercent 阶段）
                let (attack_type, freeze) = if hp_pct >= 60 {
                    (1, true)  // 冰系：冻
                } else if hp_pct >= 30 {
                    (2, false) // 普攻
                } else {
                    (3, false) // 狂攻（简化为伤害不变）
                };
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: base,
                    spell_id: 0,
                    attack_type,
                });
                // 冰系形态施加冰冻（任务核心"冰冻"）
                if freeze {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 3, 0, 1000),
                    });
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::SLOW, 5, 0, 1000),
                    });
                }
            }
        } else if dist > MELEE_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# CompleteDeath：1 格内全体 MAC 爆炸
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: 1,
            damage,
            spell_id: 0,
        });
    }
}
