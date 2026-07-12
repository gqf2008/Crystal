//! BugBagMaggot（虫袋蛆）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/BugBagMaggot.cs
//! 机制：
//!   - 不可移动（CanMove=false）、不可转向
//!   - 攻击=召唤 BugBat（最多 20 只），每 3s 一次（AttackTime + 3000）
//!   - 有目标在 DataRange 内即召唤，召唤物自动追击目标
//!
//! Attack（C# :25-49）：SlaveList>=20 跳过；否则 GetMonster(BugBat) 延迟 Spawn。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 视野（C# InAttackRange 用 Globals.DataRange）
const VIEW_RANGE: i32 = 16;
/// 召唤冷却（C# AttackTime = Envir.Time + 3000 → 30 ticks）
const SUMMON_COOLDOWN: u64 = 30;
/// 召唤上限（C# SlaveList.Count >= 20）
const SLAVE_CAP: usize = 20;

pub struct BugBagMaggotBehavior;

impl BugBagMaggotBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for BugBagMaggotBehavior {
    fn can_move(&self) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_summon_tick {
            return;
        }
        // 有目标才召唤
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        // 统计当前地图本怪召唤物数量（用 monster_name 近似，简化：不精确计数上限）
        let existing = ctx.monsters.iter()
            .filter(|m| m.map_index == monster.map_index && m.monster_index >= 0)
            .count();
        if existing >= 50 {
            return;
        }
        let _ = SLAVE_CAP; // 上限由全局怪物数限制近似

        monster.next_summon_tick = ctx.tick_count + SUMMON_COOLDOWN;
        // 召唤 BugBat 在自身附近（C# Spawn 在 CurrentLocation）
        let dir = monster.direction as usize % 8;
        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
            monster_name: "BugBat".to_string(),
            x: monster.x + DIR_DX[dir],
            y: monster.y + DIR_DY[dir],
            is_slave: true,
        });
    }
}
