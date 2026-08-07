//! SpittingToad（吐毒蟾蜍）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SpittingToad.cs
//! 机制：召唤物——主人>15 格或离线 → 自毁（Die）；
//!      攻击：ObjectRangeAttack + RangeDamage（MAC，攻速+500ms），12 格十字/对角

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MASTER_RANGE: i32 = 15;

pub struct SpittingToadBehavior;

impl SpittingToadBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SpittingToadBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Process：主人>15 格或离线 → 自毁
        if let Some(master) = monster.master_session {
            let master_ok = ctx.players.iter()
                .find(|p| p.session_id == master && p.map_index == monster.map_index)
                .map(|p| max_distance(p.x, p.y, monster.x, monster.y) <= MASTER_RANGE)
                .unwrap_or(false);
            if !master_ok {
                monster.hp = 0;
                return;
            }
        }
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        // C# InAttackRange：12 格十字/对角（(x<=12&&y<=12) 恒真，实际无限制）
        if max_distance(monster.x, monster.y, target.x, target.y) > VIEW_RANGE {
            return;
        }
        let _ = (dx, dy);
        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
        }
    }
}
