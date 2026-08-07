//! HolyDeva（圣兽）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HolyDeva.cs
//! 机制：可被法师召唤（SummonHolyDeva）的远程圣兽
//!   - AttackRange=6，纯远程弹道 DC + DefenceType.MAC
//!   - 作为宠物（Master != null）：跟随主人（MoveTo Master），射程内即攻击
//!   - 非宠物（野生）：FearTime 5s 控制攻击/拉开，射程内维持，过近 WalkAway
//!
//! Attack（C# :23-44）：ObjectRangeAttack + DC MAC。
//! ProcessTarget（C# :46-97）：Master!=null→MoveTo Master；InRange && (Master || FearTime)→Attack。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
/// FearTime 持续（C# Envir.Time + 5000）
const FEAR_TICKS: u64 = 50;

pub struct HolyDevaBehavior {
    /// 下次恐惧结束 tick（C# FearTime）
    fear_end_tick: u64,
}

impl HolyDevaBehavior {
    pub fn new() -> Self {
        Self { fear_end_tick: 0 }
    }
}

impl MonsterBehavior for HolyDevaBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.pet_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let is_pet = monster.master_session.is_some();

        // 射程内攻击（宠物无条件攻击，野生需在 FearTime 内）
        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick
            && (is_pet || ctx.tick_count < self.fear_end_tick)
        {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            return;
        }

        // 刷新 FearTime（C# FearTime = Envir.Time + 5000）
        self.fear_end_tick = ctx.tick_count + FEAR_TICKS;

        // 走位：过近拉开，远了（>=AttackRange）维持/追近
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist < ATTACK_RANGE {
                // 过近：WalkAway 拉开（C# dist<AttackRange 走远离方向）
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                // 跟随/追近
                step_toward(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
