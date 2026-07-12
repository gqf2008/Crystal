//! Jar1（坛子怪）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Jar1.cs
//! 机制：
//!   - 不可移动、不可回血（CanMove=false, CanRegen=false）
//!   - 死亡后 1s 召唤一只随机同级怪物（CompleteDeath → SpawnSlave）
//!   - 召唤规则：Level∈[self.Level-10, self.Level]，非 Boss，排除攻城 AI
//!
//! Die（C# :30-35）：DelayedAction(Die, +1000)。
//! CompleteDeath/SpawnSlave（C# :37-72）：随机 validMonsters 生成。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

pub struct Jar1Behavior {
    /// 是否已触发死亡召唤
    died: bool,
    /// 死亡时刻
    die_tick: u64,
}

impl Jar1Behavior {
    pub fn new() -> Self {
        Self { died: false, die_tick: 0 }
    }
}

impl MonsterBehavior for Jar1Behavior {
    fn can_move(&self) -> bool { false }
    fn can_regen(&self) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 死亡检测 → 延迟 1s 召唤
        if monster.hp <= 0 {
            if !self.died {
                self.died = true;
                self.die_tick = ctx.tick_count;
            }
            // C# DelayedAction(Die, +1000) → 10 ticks 后召唤
            if ctx.tick_count >= self.die_tick + 10 {
                // 召唤一只随机同级怪物（简化：用通用僵尸名，POC 无法查 MonsterInfoList 过滤）
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: "Zombie".to_string(),
                    x: monster.x,
                    y: monster.y,
                    is_slave: false,
                });
                // 防止重复召唤
                self.die_tick = u64::MAX;
            }
        }
        // 活着时坛子怪不动不攻击（C# 无 Attack override，基类近战但 CanMove=false 固守）
    }
}
