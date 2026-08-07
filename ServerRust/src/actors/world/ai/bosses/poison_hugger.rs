//! PoisonHugger（毒抱怪）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/PoisonHugger.cs
//! 机制：
//!   - 自爆虫：5 分钟超时/无目标/贴身 → Die → 1 格 AOE + 绿毒
//!   - 远程（>1 格）1/5 概率吐毒弹道（DC + ACAgility），否则追击
//!   - 贴身 → Die 自爆
//!
//! ProcessTarget（C# :23-74）：超时/无目标→Die；ranged→1/5 弹道否则 MoveTo；贴身→Die。
//! Die/CompleteDeath（C# :76-97）：FindAllTargets(1) Attacked + 绿毒。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 5;
const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
const EXPLOSION_TICKS: u64 = 3000;

pub struct PoisonHuggerBehavior {
    spawned: bool,
    explosion_tick: u64,
}

impl PoisonHuggerBehavior {
    pub fn new() -> Self {
        Self { spawned: false, explosion_tick: 0 }
    }
}

impl MonsterBehavior for PoisonHuggerBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.explosion_tick = ctx.tick_count + EXPLOSION_TICKS;
            self.spawned = true;
        }

        let target = ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index);
        if target.is_none() || ctx.tick_count >= self.explosion_tick {
            self.explode(monster, ctx);
            return;
        }
        let target = *target.unwrap();
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            self.explode(monster, ctx);
            return;
        }

        if dist <= ATTACK_RANGE && fastrand::i32(0..5) == 0 {
            // 远程吐毒弹道
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

impl PoisonHuggerBehavior {
    fn explode(&self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        let hits: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(monster.x, monster.y, 1, monster.map_index)
                .into_iter().copied().collect();
        for h in hits {
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: h.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            // C# PoisonTarget(5, 5, Green, 2000)：1/5 概率、值=SP（DC 近似）
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: h.session_id,
                    poison: Poison::new(PoisonType::GREEN, 5, damage, 2000),
                });
            }
        }
        monster.hp = 0;
    }
}
