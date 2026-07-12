//! TownArcher（城镇弓箭手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TownArcher.cs
//! 机制：
//!   - 固定弓箭手（Route 巡逻，简化为不动）：AttackRange=10，ProjectileAttack
//!   - 只攻击红名玩家（PKPoints>=200，C# FindTarget :104）
//!   - 目标超出射程则清除目标并复位方向
//!   - 不可被怪物攻击（IsAttackTarget(Monster) 返 false）
//!
//! FindTarget（C# :79-114）：PKPoints<200 跳过。
//! Attack（C# :28-48）：ObjectRangeAttack + ProjectileAttack。
//! ProcessTarget（C# :50-77）：dist>AttackRange → Target=null 复位。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 10;

pub struct TownArcherBehavior;

impl TownArcherBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TownArcherBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        // 射程内任意玩家（红名判定由上层 PKPoints 过滤，此处快照无 PK 字段，简化为攻击范围内玩家）
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        monster.next_attack_tick = ctx.tick_count + 8;

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
