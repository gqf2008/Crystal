//! WitchDoctor（巫医）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/WitchDoctor.cs
//! 机制：
//!   - 1/5 概率随机传送到目标附近（TeleportRandom(10, AttackRange)）
//!   - 1/3 概率（HP<50%）自我治疗 HP/4
//!   - 否则远程弹道 MACAgility
//!   - AttackRange=6，风筝走位
//!
//! Attack（C# :23-66）：Random(5)==0→传送；hp<50&&Random(3)==0→治疗；否则弹道。
//! TeleportRandom（C# :122-137）：目标附近 ±AttackRange 随机点。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;

pub struct WitchDoctorBehavior;

impl WitchDoctorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for WitchDoctorBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + 8;
            let roll = fastrand::i32(0..5);
            let hp_pct = if monster.max_hp > 0 { monster.hp * 100 / monster.max_hp } else { 100 };

            if roll == 0 {
                // 传送（C# TeleportRandom：目标附近 ±AttackRange 随机点）
                // POC：用 move 槽近似传送，跳到目标附近随机格
                let off = fastrand::i32(-ATTACK_RANGE..=ATTACK_RANGE);
                let off2 = fastrand::i32(-ATTACK_RANGE..=ATTACK_RANGE);
                let nx = target.x + off;
                let ny = target.y + off2;
                ctx.out_moves.push((monster.object_id, nx, ny, monster.direction));
            } else if hp_pct < 50 && fastrand::i32(0..3) == 0 {
                // 自我治疗 HP/4（C# ChangeHP(HP/4)）
                let heal = monster.max_hp / 4;
                monster.hp = (monster.hp + heal).min(monster.max_hp);
            } else {
                // 远程弹道
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
            return;
        }

        // 风筝走位
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else if dist < 3 {
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
