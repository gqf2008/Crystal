//! HumanWizard（人形法师 NPC）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HumanWizard.cs
//! 机制：可被召唤的人形法师（施放 ThunderBolt）
//!   - AttackRange=6，纯远程：ObjectMagic ThunderBolt + MC MAC
//!   - 宠物（Master!=null）：跟随主人 MoveTo Master，射程内即攻击；
//!     每秒消耗主人 10 MP，主人 MP<=0 则 Die
//!   - 野生（无 Master）：FearTime 5s 控制攻击/拉开，过近 WalkAway
//!
//! Attack（C# :24-45）：ObjectMagic(ThunderBolt) + DC→MC MAC（注：取 MC）。
//! ProcessAI（C# :47-58）：Master 每 1s ChangeMP(-10)，MP<=0 → Die。
//! ProcessTarget（C# :60-111）：Master→MoveTo Master；InRange&&(Master||FearTime)→Attack。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
/// FearTime 持续（C# Envir.Time + 5000）
const FEAR_TICKS: u64 = 50;
/// 雷电术 Spell ID（C# Spell.ThunderBolt）
const SPELL_THUNDER_BOLT: u8 = mir2_shared::enums::Spell::ThunderBolt as u8;

pub struct HumanWizardBehavior {
    fear_end_tick: u64,
}

impl HumanWizardBehavior {
    pub fn new() -> Self {
        Self { fear_end_tick: 0 }
    }
}

impl MonsterBehavior for HumanWizardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let is_pet = monster.master_session.is_some();

        // 射程内攻击（宠物无条件，野生需 FearTime 内）
        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick
            && (is_pet || ctx.tick_count < self.fear_end_tick)
        {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            // 雷电术：MC MAC（C# GetAttackPower MinMC/MaxMC + DefenceType.MAC）
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: SPELL_THUNDER_BOLT,
            });
            return;
        }

        // 刷新 FearTime
        self.fear_end_tick = ctx.tick_count + FEAR_TICKS;

        // 走位：宠物跟主人/过近拉开
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = if dist < ATTACK_RANGE {
                step_away(monster.x, monster.y, target.x, target.y)
            } else {
                step_toward(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + 2;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
