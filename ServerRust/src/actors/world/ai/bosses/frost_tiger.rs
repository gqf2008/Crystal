//! FrostTiger（霜虎）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FrostTiger.cs
//! 机制：AttackRange=6；近战 DC / 远程 MC（攻速+500ms）；
//!      远程命中后 1/8 毒：Info.Effect==0 → 出血 / ==1 → 减速（5s，tick 1000）
//! #1354：坐姿/隐身机制（ObjectSitDown）——出生 0~2 分钟随机计时，到点坐下（Hidden + 广播
//!       ObjectSitDown sitting=true），坐下禁攻禁移；被攻击/有仇恨时起身并重置计时。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct FrostTigerBehavior;

impl FrostTigerBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FrostTigerBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // #1354：初始坐下计时（C# 构造时 NewSitDownTime = Envir.Time + Random(0..2min)；0=未初始化）
        if monster.sit_down_tick == 0 {
            monster.sit_down_tick = ctx.tick_count + fastrand::u64(0..1200);
        }
        // C# ProcessAI：未死未坐且到点 → Sitting=true + Hidden=true + 广播 ObjectSitDown
        if !monster.sitting && ctx.tick_count >= monster.sit_down_tick {
            monster.sitting = true;
            monster.hidden = true;
            ctx.out_sit_down.push((
                monster.object_id,
                monster.x,
                monster.y,
                monster.direction,
                true,
            ));
        }
        // 坐下时禁攻禁移（C# CanAttack/CanMove/Walk=false）；被攻击/有仇恨 → 起身 + 重置计时
        if monster.sitting {
            if monster.provoked || monster.last_hitter_session.is_some() {
                monster.sitting = false;
                monster.hidden = false;
                monster.sit_down_tick = ctx.tick_count + fastrand::u64(0..1200);
                ctx.out_sit_down.push((
                    monster.object_id,
                    monster.x,
                    monster.y,
                    monster.direction,
                    false,
                ));
            } else {
                return;
            }
        }
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
        {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            if dist <= 1 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                // C# 1/8 毒：Effect==0 → Bleeding / ==1 → Slow（5s，tick 1000）
                if fastrand::i32(0..8) == 0 {
                    let ptype = if monster.effect == 0 {
                        PoisonType::BLEEDING
                    } else {
                        PoisonType::SLOW
                    };
                    ctx.out_poisons
                        .push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(ptype, 5, 0, 1000),
                        });
                }
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
