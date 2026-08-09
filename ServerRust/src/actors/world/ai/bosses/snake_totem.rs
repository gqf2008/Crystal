//! SnakeTotem（蛇图腾）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SnakeTotem.cs
//! 机制：静态（CanMove=false）；主人>15 格或离线 → 自毁；
//!      周期召唤 CharmedSnake 小兵（MaxMinions=PetLevel+1 近似 2，10s 冷却近似）
//!      （死亡连带小兵清理依赖 slave_list，此处 out_summons 用 is_slave=true）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const MASTER_RANGE: i32 = 15;
const SUMMON_COOLDOWN: u64 = 100; // 10s

pub struct SnakeTotemBehavior {
    next_summon_tick: u64,
}

impl SnakeTotemBehavior {
    pub fn new() -> Self {
        Self { next_summon_tick: 0 }
    }
}

impl MonsterBehavior for SnakeTotemBehavior {
    fn can_move(&self) -> bool {
        false
    }

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
        // C# ProcessAI：保持 Minions（10s 冷却近似）
        if ctx.tick_count >= self.next_summon_tick {
            self.next_summon_tick = ctx.tick_count + SUMMON_COOLDOWN;
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: "CharmedSnake".to_string(),
                x: monster.x,
                y: monster.y,
                is_slave: true,
                summoner_oid: Some(monster.object_id),
            });
        }
    }
}
