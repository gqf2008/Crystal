//! YinDevilNode（阴魔节点）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/YinDevilNode.cs
//! 机制：静态（CanMove=false）；周期对 7 格内友军（此处近似所有玩家）加 buff：
//!      AI==41 → BlessedArmour（MaxAC=目标等级/7+4）；否则 → UltimateEnhancer（MaxDC≈MaxMC，目标等级/7+4）
//!      持续 5s

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::MonsterState;

const BUFF_RADIUS: i32 = 7;

pub struct YinDevilNodeBehavior;

impl YinDevilNodeBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for YinDevilNodeBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
        // C# FindFriendsNearby(7)（MonsterObject.cs:2257-2291）：附近有非攻击目标友军（此处=其他存活怪物）才触发
        let friends: Vec<crate::actors::world::ai::ctx::MonsterSnap> = ctx
            .monsters
            .iter()
            .filter(|m| {
                m.object_id != monster.object_id
                    && m.hp > 0
                    && m.map_index == monster.map_index
                    && (m.x - monster.x).abs() <= BUFF_RADIUS
                    && (m.y - monster.y).abs() <= BUFF_RADIUS
            })
            .cloned()
            .collect();
        if friends.is_empty() {
            return;
        }
        // C# CompleteAttack（YinDevilNode.cs:23-43）：给 7 格内友军加 Buff
        //   AI==41 → BlessedArmour（MaxAC = 目标等级/7+4）；否则 → UltimateEnhancer（MaxDC = 目标等级/7+4）；持续 5s
        let is_blessed =
            monster.ai_profile.ai_type == crate::actors::world::MonsterAiType::Summoner;
        for f in &friends {
            let bonus = f.level / 7 + 4;
            let buff = if is_blessed {
                crate::actors::world::MonsterBuff {
                    dc_min: 0,
                    dc_max: 0,
                    ac_min: bonus,
                    ac_max: bonus,
                    mac_min: 0,
                    mac_max: 0,
                    remaining_ticks: 50, // 5s（10 tick/s）
                }
            } else {
                crate::actors::world::MonsterBuff {
                    dc_min: bonus,
                    dc_max: bonus,
                    ac_min: 0,
                    ac_max: 0,
                    mac_min: 0,
                    mac_max: 0,
                    remaining_ticks: 50,
                }
            };
            ctx.out_monster_buffs.push((f.object_id, buff));
        }
    }
}
