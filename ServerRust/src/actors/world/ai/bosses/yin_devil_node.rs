//! YinDevilNode（阴魔节点）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/YinDevilNode.cs
//! 机制：静态（CanMove=false）；周期对 7 格内友军（此处近似所有玩家）加 buff：
//!      AI==41 → BlessedArmour（MaxAC=目标等级/7+4）；否则 → UltimateEnhancer（MaxDC≈MaxMC，目标等级/7+4）
//!      持续 5s

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::buff::{BuffInstance, BuffType};

const VIEW_RANGE: i32 = 12;
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
        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let nearby: Vec<(u64, u16)> = ctx.find_targets_in_range(monster.x, monster.y, BUFF_RADIUS, monster.map_index)
                .iter().map(|p| (p.session_id, p.level)).collect();
            if nearby.is_empty() {
                return;
            }
            for (sid, level) in nearby {
                // C#：type = AI==41 ? BlessedArmour : UltimateEnhancer；stats = level/7 + 4
                let bonus = level as i32 / 7 + 4;
                let buff_type = // C# Info.AI==41（Rust from_db_ai 映射 40|41 → Summoner）
                if monster.ai_profile.ai_type == crate::actors::world::MonsterAiType::Summoner {
                    BuffType::AcDefenseBoost { bonus }
                } else {
                    BuffType::McBoost { bonus }
                };
                ctx.out_player_buffs.push((
                    sid,
                    BuffInstance::new(buff_type, 50, 10), // 5s（10 tick/s）
                ));
            }
        }
    }
}
