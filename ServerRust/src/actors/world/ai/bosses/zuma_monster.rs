//! ZumaMonster（祖玛卫士系）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ZumaMonster.cs
//! 机制：
//!   - 默认石化（Stoned=true）：不可移动、不可攻击、不可被攻击、免疫毒/buff/推开
//!   - 玩家靠近 2 格内 Wake（现身）并 WakeAll（14格内同伴一起醒）
//!   - 现身后为标准近战怪（移动+攻击）
//!
//! ProcessAI（C# :59-73）：Envir.Time>ActionTime 时检测 FindNearby(2) 切换 Stoned。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

/// 石化判定：玩家靠近几格内唤醒（C# FindNearby(2)）
const WAKE_RANGE: i32 = 2;
/// 视野范围
const VIEW_RANGE: i32 = 15;
/// 近战范围
const MELEE_RANGE: i32 = 1;

pub struct ZumaMonsterBehavior {
    /// 当前是否石化（true=石化免疫，false=活跃）
    stoned: bool,
}

impl ZumaMonsterBehavior {
    pub fn new() -> Self {
        Self { stoned: true }
    }
}

impl MonsterBehavior for ZumaMonsterBehavior {
    /// 石化期不可移动（C# CanMove = base && !Stoned）
    fn can_move(&self) -> bool {
        !self.stoned
    }

    /// 石化期不可被攻击（C# IsAttackTarget = !Stoned && ...）
    fn is_attackable(&self) -> bool {
        !self.stoned
    }

    /// 石化期免疫伤害（C# IsAttackTarget 返 false）
    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.stoned { 0 } else { damage }
    }

    /// 石化期免疫毒（C# ApplyPoison: if(Stoned) return）
    fn on_poison(&mut self, _poison: Poison) -> bool {
        !self.stoned
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // ---- 石化/唤醒切换（C# ProcessAI：FindNearby(2)）----
        if self.stoned {
            let has_near = ctx.nearest_target(monster.x, monster.y, WAKE_RANGE, monster.map_index).is_some();
            if has_near {
                // C# Wake() + WakeAll(14)：自身唤醒（同伴唤醒由各怪自身检测完成）
                self.stoned = false;
                return;
            }
            return; // 石化期无动作
        }

        // ---- 活跃期：标准近战追击+攻击 ----
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        if dist <= MELEE_RANGE {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + 5;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        } else if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
