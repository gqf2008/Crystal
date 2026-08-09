//! HellKnight（地狱骑士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellKnight.cs
//! 机制：
//!   - HellLord 的守门骑士：由 HellLord 按 stage 召唤（HellKnight1..4）
//!   - Summoned=true（Spawned 时设置，GetInfo.Extra 上报）
//!   - 死亡回调 Lord.KnightKilled() 推进 HellLord 阶段
//!
//! Die（C# :23-31）：Lord != null → Lord.KnightKilled(); base.Die()。
//!
//! 说明：跨怪物阶段推进（KnightKilled）由 tick.rs 的 Boss 互查通道处理；
//! 这里 on_die 发出哨兵召唤名 "HellLordAdvance"（monster_name_index 命中不到则忽略），
//! 上层可据此推进同地图 HellLord 阶段。骑士本体为普通近战追击怪。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const MELEE_RANGE: i32 = 1;

pub struct HellKnightBehavior {
    /// Spawned 标志（C# Summoned，GetInfo.Extra 上报；此处仅记录状态）
    #[allow(dead_code)]
    summoned: bool,
}

impl HellKnightBehavior {
    pub fn new() -> Self {
        Self { summoned: false }
    }
}

impl MonsterBehavior for HellKnightBehavior {
    /// C# Spawned：Summoned = true
    fn on_spawned(&mut self, _monster: &mut MonsterState) {
        self.summoned = true;
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# Die：Lord != null → Lord.KnightKilled()。此处发出阶段推进哨兵。
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 哨兵召唤名（monster_name_index 命中不到时 tick.rs 会忽略，无副作用），
        // 上层可监听该名称推进同地图 HellLord 阶段。
        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
            monster_name: "HellLordAdvance".to_string(),
            x: monster.x,
            y: monster.y,
            is_slave: false,
            summoner_oid: Some(monster.object_id),
        });
    }
}
