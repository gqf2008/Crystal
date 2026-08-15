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
//! ChangeHP（C# :123-131）：#2570 宠物受击不改自身 HP，镜像 ChangeMP 到主人（血条=主人蓝条）。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::ctx::MasterMpDrain;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const ATTACK_RANGE: i32 = 6;
const VIEW_RANGE: i32 = 15;
/// FearTime 持续（C# Envir.Time + 5000）
const FEAR_TICKS: u64 = 50;
/// 雷电术 Spell ID（C# Spell.ThunderBolt）
const SPELL_THUNDER_BOLT: u8 = mir2_shared::enums::Spell::ThunderBolt as u8;
/// #2570：吸取主人 MP 间隔（C# DecreaseMPTime = Envir.Time + 1000）
const MP_DRAIN_INTERVAL_TICKS: u64 = 10;
/// #2570：每次吸取量（C# ChangeMP(-10)）
const MP_DRAIN_PER_SECOND: i32 = 10;
/// #2570：跟主人保持距离（切比雪夫距离超过才移动，对齐 C# MoveTo 到达即停）
const FOLLOW_KEEP_DIST: i32 = 2;

pub struct HumanWizardBehavior {
    fear_end_tick: u64,
    /// #2570：C# DecreaseMPTime——下次吸取主人 MP 的 tick
    next_mp_drain_tick: u64,
    /// #2570：C# ChangeHP 镜像累积——宠物受击量（负值），process_tick 排水到主人 MP
    pending_master_mp_delta: i32,
}

impl Default for HumanWizardBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanWizardBehavior {
    pub fn new() -> Self {
        Self {
            fear_end_tick: 0,
            next_mp_drain_tick: 0,
            pending_master_mp_delta: 0,
        }
    }
}

impl MonsterBehavior for HumanWizardBehavior {
    /// #2570：C# HumanWizard.ChangeHP——宠物受击不扣自身 HP（HP 恒满，
    /// 永不因伤害死亡），改为镜像扣主人 MP；野生走默认扣血。
    fn on_attacked_with_monster(&mut self, monster: &mut MonsterState, damage: i32) -> i32 {
        if monster.master_session.is_some() {
            self.pending_master_mp_delta = self.pending_master_mp_delta.saturating_sub(damage);
            return 0;
        }
        damage
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let is_pet = monster.master_session.is_some();

        // ---- #2570 MP 维系 ----
        // 受击镜像排水（on_attacked_with_monster 累积的负伤害）
        if self.pending_master_mp_delta != 0 {
            let delta = std::mem::take(&mut self.pending_master_mp_delta);
            if let Some(master) = monster.master_session {
                ctx.out_master_mp_drains.push(MasterMpDrain {
                    pet_oid: monster.object_id,
                    master_session: master,
                    amount: delta,
                });
            }
        }
        // 每秒吸 10 MP（C# ProcessAI：DecreaseMPTime 间隔 1s；主人 MP<=0 宠物 Die 由 tick 消费判定）
        if is_pet && ctx.tick_count >= self.next_mp_drain_tick {
            self.next_mp_drain_tick = ctx.tick_count + MP_DRAIN_INTERVAL_TICKS;
            ctx.out_master_mp_drains.push(MasterMpDrain {
                pet_oid: monster.object_id,
                master_session: monster.master_session.unwrap(),
                amount: -MP_DRAIN_PER_SECOND,
            });
        }

        // ---- 目标选择 ----
        // 宠物：仅主人当前目标（协战）或自身被击反击（combat.rs 受击设 target_session）；
        // 野生：最近玩家（C# SearchArea）
        let target = if is_pet {
            ctx.master_target
                .filter(|t| t.map_index == monster.map_index && t.hp > 0)
                .or_else(|| {
                    ctx.players
                        .iter()
                        .find(|p| {
                            Some(p.session_id) == monster.target_session
                                && p.map_index == monster.map_index
                                && p.hp > 0
                        })
                        .copied()
                })
        } else {
            ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                .copied()
        };

        let dist = target
            .map(|t| max_distance(monster.x, monster.y, t.x, t.y))
            .unwrap_or(i32::MAX);

        // ---- 攻击（宠物无条件，野生需 FearTime 内；C# InAttackRange&&(Master||FearTime)）----
        if let Some(t) = target {
            monster.target_session = Some(t.session_id);
            if dist <= ATTACK_RANGE
                && ctx.tick_count >= monster.next_attack_tick
                && (is_pet || ctx.tick_count < self.fear_end_tick)
            {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                // 雷电术：MC MAC（C# GetAttackPower MinMC/MaxMC + DefenceType.MAC）
                let damage =
                    crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0)
                        .max(1);
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: t.session_id,
                        target_object_id: t.object_id,
                        damage,
                        spell_id: SPELL_THUNDER_BOLT,
                    });
                return;
            }
        }

        // 刷新 FearTime（野生攻击窗口）
        self.fear_end_tick = ctx.tick_count + FEAR_TICKS;

        // ---- 走位 ----
        // 宠物：跟主人为主（C# MoveTo(Master.CurrentLocation)），不追击目标；
        // 野生：过近拉开、远了追击
        if ctx.tick_count >= monster.next_move_tick {
            let step = if is_pet {
                // 主人在玩家快照内（快照剔除死亡/隐身/安全区玩家，缺失时原地待命）
                ctx.players
                    .iter()
                    .find(|p| Some(p.session_id) == monster.master_session)
                    .filter(|m| max_distance(monster.x, monster.y, m.x, m.y) > FOLLOW_KEEP_DIST)
                    .map(|m| step_toward(monster.x, monster.y, m.x, m.y))
            } else {
                match target {
                    Some(t) if dist >= ATTACK_RANGE => {
                        Some(step_toward(monster.x, monster.y, t.x, t.y))
                    }
                    Some(t) => Some(step_away(monster.x, monster.y, t.x, t.y)),
                    None => None,
                }
            };
            if let Some((nx, ny, dir)) = step {
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
        }
    }
}
