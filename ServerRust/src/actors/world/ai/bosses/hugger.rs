//! Hugger（抱抱怪/自爆虫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Hugger.cs
//! 机制：
//!   - 自爆怪：贴近目标/超时/无目标 → Die → 范围 1 格 AOE + 绿毒 5s
//!   - ExplosionTime = 出生后 5 分钟强制爆炸
//!   - 贴身攻击：InAttackRange 则 Attack，目标死则自爆
//!
//! ProcessTarget（C# :16-47）：Target==null||超时→Die；InAttackRange→Attack（目标死→Die）。
//! CompleteDeath（C# :55-69）：FindAllTargets(1) Attacked + PoisonTarget(5,5,Green,2000)。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;
const EXPLOSION_TICKS: u64 = 3000;

pub struct HuggerBehavior {
    spawned: bool,
    explosion_tick: u64,
}

impl HuggerBehavior {
    pub fn new() -> Self {
        Self { spawned: false, explosion_tick: 0 }
    }
}

impl MonsterBehavior for HuggerBehavior {
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
            // 贴身攻击（C# Attack 后目标死则 Die）
            self.explode(monster, ctx);
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

impl HuggerBehavior {
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
            // C# PoisonTarget(5,5,Green,2000)：1/5
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
