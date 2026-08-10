//! Mandrill（山魈）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Mandrill.cs（继承 MutatedManworm）
//! 机制：近战 DC（ACAgility）+ 受击反制：承伤>自身 DC 且 1/2 → FindWeakerTarget → TeleportToTarget

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
    /// C# Attacked 反制：承伤>自身 DC 且 1/2 → FindWeakerTarget → TeleportToTarget（目标背后 1 格）
    fn maybe_teleport_to_weaker(monster: &mut MonsterState, ctx: &mut AiCtx, view_range: i32) {
        if monster.last_hit_damage <= 0 {
            return;
        }
        let dmg = monster.last_hit_damage;
        monster.last_hit_damage = 0;
        let own = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
        if dmg <= own || fastrand::i32(0..2) != 0 {
            return;
        }
        let current = monster.target_session;
        let candidates = ctx.find_targets_in_range(monster.x, monster.y, view_range, monster.map_index);
        if candidates.len() < 2 {
            return;
        }
        // C# FindWeakerTarget 按 MinDC 选更弱目标
        let weaker = candidates.iter()
            .filter(|p| current != Some(p.session_id))
            .min_by_key(|p| p.min_dc)
            .copied();
        if let Some(w) = weaker {
            monster.target_session = Some(w.session_id);
            // C# TeleportToTarget：目标背后 1 格（目标 + 反方向）
            let dir = direction_towards(monster.x, monster.y, w.x, w.y) as usize;
            let back = (dir + 4) % 8;
            let tx = w.x + DIR_DX[back];
            let ty = w.y + DIR_DY[back];
            ctx.out_monster_teleports.push((monster.object_id, tx, ty));
        }
    }

pub struct MandrillBehavior;

impl MandrillBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for MandrillBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        maybe_teleport_to_weaker(monster, ctx, VIEW_RANGE);
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
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
