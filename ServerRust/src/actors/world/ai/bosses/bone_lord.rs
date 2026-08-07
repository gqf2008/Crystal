//! BoneLord（骨魔领主）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/BoneLord.cs
//! 机制：
//!   - HP 三阶段召唤：每损失 1/3 HP 召唤一次骨骼奴隶（最多 8 只，SlaveList 上限 40）
//!   - 近战贴身普攻；远程（>1 格）掷骨弹道 MACAgility
//!   - AttackRange=7，风筝追击
//!
//! ProcessTarget（C# :54-83）：stage=HP/(MaxHP/3)，stage<_stage → SpawnSlaves。
//! Attack（C# :21-52）：range→ObjectRangeAttack 弹道；否则 base.Attack。
//! SpawnSlaves（C# :85-121）：count=min(8, 40-SlaveList.Count)，随机 4 种骨怪。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 7;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct BoneLordBehavior {
    /// 当前阶段（3=满血，递减）。C# _stage 初值 3
    stage: u8,
    /// 已召唤标记
    called: [bool; 3],
}

impl BoneLordBehavior {
    pub fn new() -> Self {
        Self { stage: 3, called: [false; 3] }
    }
}

impl MonsterBehavior for BoneLordBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // HP 阶段召唤（C# stage = HP / (MaxHP/3)）
        if monster.max_hp > 0 {
            let third = monster.max_hp / 3;
            let cur_stage = if third > 0 {
                (monster.hp / third).clamp(0, 3) as u8
            } else {
                0
            };
            if cur_stage < self.stage {
                self.stage = cur_stage;
                let idx = self.stage as usize;
                if idx < 3 && !self.called[idx] {
                    self.called[idx] = true;
                    self.spawn_slaves(monster, ctx);
                    return;
                }
            }
        }

        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        } else if dist <= ATTACK_RANGE {
            // 远程掷骨弹道
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}

impl BoneLordBehavior {
    fn spawn_slaves(&self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let count = 8u8;
        let names = ["BoneMonster1", "BoneMonster2", "BoneMonster3", "BoneMonster4"];
        for i in 0..count {
            let dir = (i % 8) as usize;
            let nm = names[(i % 4) as usize];
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: nm.to_string(),
                x: monster.x + DIR_DX[dir] * 2,
                y: monster.y + DIR_DY[dir] * 2,
                is_slave: true,
            });
        }
    }
}
