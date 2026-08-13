//! Deer（鹿，AI 1/2）behavior —— C# Deer.cs : HarvestMonster
//!
//! C# 机制：
//!   - 可采集（HarvestMonster 子类）：Drop() 为空，死亡保留尸体，采集出 monster_drops
//!   - AI 2（Deer/Deer1/Sheep）：RemainingSkinCount=5，1/7 概率 _runAway（不攻击、MoveSpeed-300、遇玩家逃跑）
//!   - AI 1（Hen/Pig/Bull）：RemainingSkinCount=2，不逃跑，被动
//!
//! 注：AI 1/2 由 from_db_ai 映射为 Passive；此处仅补可采集 + AI 2 逃跑分支（#2358）。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const VIEW_RANGE: i32 = 8;

pub struct DeerBehavior {
    /// C# _runAway：AI 2 的 Deer 1/7 概率逃跑；AI 1 不逃跑
    run_away: bool,
    /// 首次 process_tick 时按名字初始化逃跑分支（new() 无名字）
    initialized: bool,
}

impl Default for DeerBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl DeerBehavior {
    pub fn new() -> Self {
        Self {
            run_away: false,
            initialized: false,
        }
    }
}

impl MonsterBehavior for DeerBehavior {
    /// C# Deer : HarvestMonster —— 死亡保留尸体可采集
    fn is_harvestable(&self) -> bool {
        true
    }

    /// C# Deer.cs：AI 2（Deer/Deer1/Sheep）→ RemainingSkinCount=5；AI 1（Hen/Pig/Bull）→ 默认 2
    fn harvest_skin_count(&self, monster: &MonsterState) -> u8 {
        let nm = monster.name.to_lowercase();
        if nm.contains("deer") || nm.contains("sheep") {
            5
        } else {
            2
        }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.initialized {
            self.initialized = true;
            let nm = monster.name.to_lowercase();
            // C#：仅 AI 2（Deer/Deer1/Sheep）有 1/7 逃跑；AI 1（Hen/Pig/Bull）不逃跑
            if nm.contains("deer") || nm.contains("sheep") {
                self.run_away = fastrand::i32(0..7) == 0;
            }
        }
        if !self.run_away {
            return; // 被动：不索敌不攻击（C# 非逃跑 Deer FindTarget 不触发）
        }
        // C# ProcessTarget：朝远离目标方向移动（MoveSpeed-300 更快）
        if let Some(target) =
            ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index)
        {
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_away(monster.x, monster.y, target.x, target.y);
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
            }
        }
    }
}
