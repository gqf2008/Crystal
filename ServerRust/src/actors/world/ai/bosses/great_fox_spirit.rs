//! GreatFoxSpirit（巨狐之灵）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/GreatFoxSpirit.cs
//! 机制：
//!   - 不可移动（CanMove=false）
//!   - 4 阶段（按 HP 1/4 划分），每阶段外观变化（_stage 广播）
//!   - 周期传送玩家：>3 格目标 1/10 概率随机传送到自身邻格（冷却 10s）
//!   - AOE 攻击：近战 FindAllTargets(2) / 远程 FindAllTargets(7) 全体 MAC + Slow + Paralysis
//!
//! ProcessAI（C# :25-41）：按 HP 划分 _stage。
//! ProcessTarget（C# :52-90）：>3 格 1/10 概率传送。
//! Attack（C# :92-123）：ranged→FindAllTargets(7)；else FindAllTargets(2)。
//! CompleteAttack（C# :125-137）：Slow 15s + Paralysis 5s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 7;
const MELEE_RADIUS: i32 = 2;
/// 传送冷却（C# RecallTime = Time + 10000）
const RECALL_COOLDOWN_TICKS: u64 = 100;

pub struct GreatFoxSpiritBehavior {
    /// 当前阶段（0-3，C# _stage）
    stage: u8,
    /// 下次可传送的 tick（C# RecallTime）
    next_recall_tick: u64,
}

impl GreatFoxSpiritBehavior {
    pub fn new() -> Self {
        Self { stage: 0, next_recall_tick: 0 }
    }
}

impl MonsterBehavior for GreatFoxSpiritBehavior {
    fn can_move(&self) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // ---- 阶段计算（C# stage = 4 - HP/(MaxHP/4)）----
        if monster.max_hp >= 4 {
            let stage = (4 - monster.hp / (monster.max_hp / 4)).max(0).min(3) as u8;
            if stage > self.stage {
                self.stage = stage;
            }
        }

        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // ---- 周期传送玩家（C# >3 格 1/10 概率）----
        if dist > 3 && fastrand::i32(0..10) == 0 && ctx.tick_count >= self.next_recall_tick {
            self.next_recall_tick = ctx.tick_count + RECALL_COOLDOWN_TICKS;
            // C# FindAllTargets(30) 随机传送到自身邻格
            let targets: Vec<crate::actors::world::ai::PlayerSnap> =
                ctx.find_targets_in_range(monster.x, monster.y, 30, monster.map_index)
                    .into_iter().copied().collect();
            for t in targets {
                let td = max_distance(monster.x, monster.y, t.x, t.y);
                if td > 3 {
                    // 传送到自身随机邻格（近似 C# PointMove(CurrentLocation, random dir, 1)）
                    let off = fastrand::i32(-1..=1);
                    let off2 = fastrand::i32(-1..=1);
                    let nx = monster.x + off;
                    let ny = monster.y + off2;
                    ctx.out_moves.push((t.object_id, nx, ny, 0));
                    break; // C# 一次只传送一个
                }
            }
        }

        // ---- AOE 攻击 ----
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

        let ranged = dist > 2;
        let radius = if ranged { ATTACK_RANGE } else { MELEE_RADIUS };
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);

        let hits: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(monster.x, monster.y, radius, monster.map_index)
                .into_iter().copied().collect();
        for h in hits {
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: h.session_id,
                damage,
                spell_id: 0,
                attack_type: if ranged { 0 } else { 1 },
            });
            // C# Attack（GreatFoxSpirit.cs:118）：远程命中时对每个目标广播特效
            if ranged {
                ctx.out_effects.push((h.object_id, mir2_shared::enums::SpellEffect::GreatFoxSpirit));
            }
            // C# CompleteAttack: Slow 15s + Paralysis 5s
            // C# PoisonTarget 1/5
                if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: h.session_id,
                    poison: Poison::new(PoisonType::SLOW, 15, damage, 1000),
                });
                }
            // C# PoisonTarget 1/5
                if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: h.session_id,
                    poison: Poison::new(PoisonType::PARALYSIS, 5, damage, 1000),
                });
                }
        }
    }
}
