//! RedFoxman（红狐人）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RedFoxman.cs
//! 机制：
//!   - AttackRange=6 远程风筝（<6 远离，>=6 接近）
//!   - 目标贴身（dist<=1）且 10s 冷却：TeleportRandom(40, 14) 随机传送 ±14 格（C# Random.Next(1)==0 恒真）
//!   - 攻击：ObjectRangeAttack + RangeDamage（MAC）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const TELEPORT_RANGE: i32 = 14;
const TELEPORT_COOLDOWN: u64 = 100; // 10s

pub struct RedFoxmanBehavior {
    next_teleport_tick: u64,
}

impl RedFoxmanBehavior {
    pub fn new() -> Self {
        Self { next_teleport_tick: 0 }
    }
}

impl MonsterBehavior for RedFoxmanBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // C# ProcessTarget：贴身且冷却到 → TeleportRandom(40, 14)
        if dist <= 1 && ctx.tick_count >= self.next_teleport_tick {
            self.next_teleport_tick = ctx.tick_count + TELEPORT_COOLDOWN;
            let (w, h) = ctx.map_size;
            let tx = (monster.x + fastrand::i32(-TELEPORT_RANGE..=TELEPORT_RANGE)).clamp(0, w - 1);
            let ty = (monster.y + fastrand::i32(-TELEPORT_RANGE..=TELEPORT_RANGE)).clamp(0, h - 1);
            ctx.out_monster_teleports.push((monster.object_id, tx, ty));
            return;
        }

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# 风筝：>=6 接近，<6 远离
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else {
                step_away(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
