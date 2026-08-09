//! RootSpider（根须蜘蛛）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/RootSpider.cs（继承 BugBagMaggot）
//! 机制：
//!   - 不可移动（继承 BugBagMaggot CanMove=false）
//!   - 攻击=召唤 BombSpider（自爆蜘蛛），最多 20 只，每 3s 一次
//!   - 召唤位置随 Direction（Up/UpRight/Right 三方向偏移）
//!
//! Attack（C# :16-54）：SlaveList>=20 跳过；否则 Spawn BombSpider。
//! 注意：与 BugBagMaggot 区别仅在召唤物名（BombSpider vs BugBat）。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 16;
const SUMMON_COOLDOWN: u64 = 30;

pub struct RootSpiderBehavior;

impl RootSpiderBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for RootSpiderBehavior {
    fn can_move(&self) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_summon_tick {
            return;
        }
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        monster.next_summon_tick = ctx.tick_count + SUMMON_COOLDOWN;
        // #1442：C# SlaveList.Count >= 20 跳过召唤
        if ctx.slave_count >= 20 {
            return;
        }

        // 召唤 BombSpider 在偏移位置（C# 按 Direction 选 Back/DownRight/DownLeft）
        let dir = monster.direction as usize % 8;
        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
            monster_name: "BombSpider".to_string(),
            x: monster.x + DIR_DX[dir],
            y: monster.y + DIR_DY[dir],
            is_slave: true,
            summoner_oid: Some(monster.object_id),
        });
    }
}
