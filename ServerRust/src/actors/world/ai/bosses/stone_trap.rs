//! StoneTrap（石阵）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/StoneTrap.cs
//! 机制：弓手 Stonetrap 召唤的固定陷阱
//!   - Walk 返回 false、Turn 空、Attack 空（不可移动/不可攻击）
//!   - Process：主人跨图 / 15 格外 / DieTime（recall_at_tick）→ Die
//!   - 可被玩家攻击摧毁（hp 由伤害扣减）
//! 注意：C# 的"嘲讽附近怪物攻击自己"需要怪物互伤引擎（当前未支持，见 #1008）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

pub struct StoneTrapBehavior;

impl StoneTrapBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for StoneTrapBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 不可移动/不可攻击（C# Walk=false / Attack 空）
        monster.next_attack_tick = u64::MAX;
        monster.next_move_tick = u64::MAX;
        monster.ai_state = crate::actors::world::MonsterAiState::Idle;

        // C# Process：主人跨图 / 15 格外 → Die（DieTime 由 recall_at_tick 处理）
        if let Some(master) = monster.master_session {
            let near = ctx.players.iter().any(|p| {
                p.session_id == master
                    && p.map_index == monster.map_index
                    && ((p.x - monster.x).abs() + (p.y - monster.y).abs()) <= 15
            });
            if !near {
                monster.hp = 0;
            }
        }

        // C# ProcessAI：嘲讽视野内怪物攻击自己（怪物互伤，monster_targets 由上层应用）
        let view_range = monster.ai_profile.aggro_range.max(1) as i32;
        for snap in ctx.monsters.iter() {
            if snap.object_id == monster.object_id {
                continue;
            }
            if snap.map_index != monster.map_index || snap.hp <= 0 {
                continue;
            }
            let d = (snap.x - monster.x).abs() + (snap.y - monster.y).abs();
            if d <= view_range {
                ctx.out_monster_taunts.push((monster.object_id, snap.object_id));
            }
        }
    }
}
