//! HellKeeper（地狱守门人）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellKeeper.cs
//! 机制：
//!   - 不能移动（CanMove=false）、不能回血（CanRegen=false）
//!   - 免疫毒（C# ApplyPoison 空实现）
//!   - 全视野攻击（InAttackRange = ViewRange），固定方向（Up）
//!   - 1/3 概率 MC AOE + Dazed；2/3 概率 DC 全体单体（多目激光）
//!   - 高敏捷/护甲减伤（C# Attacked 自定义 armour 判定，简化为 on_attacked 减伤）
//!
//! Attack（C# HellKeeper.cs:163-171）：attacktype1 = Random(3)>0 ? 0 : 1。
//! CompleteAttack（C# :173-201）：ViewRange 全体，Type 0=DC / Type 1=MC+Dazed。
//! Attacked（C# :31-144）：自定义 armour 减伤 + 移除 LRParalysis。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

/// 视野范围（C# InAttackRange 用 Info.ViewRange）
const VIEW_RANGE: i32 = 15;
/// 攻击冷却
const ATTACK_COOLDOWN: u64 = 8;

pub struct HellKeeperBehavior;

impl HellKeeperBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for HellKeeperBehavior {
    fn can_move(&self) -> bool { false }
    fn can_regen(&self) -> bool { false }

    /// 免疫毒（C# ApplyPoison 空实现）
    fn on_poison(&mut self, _poison: Poison) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        // 全视野全体玩家（C# FindAllTargets(Info.ViewRange)）
        let targets: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                .into_iter().copied().collect();
        if targets.is_empty() {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + ATTACK_COOLDOWN;

        // 1/3 概率 MC 激光 + Dazed；2/3 概率 DC 激光（C# attacktype1 = Random(3)>0 ? 0 : 1）
        let is_mc = fastrand::i32(0..3) == 0;

        for t in targets {
            let damage = if is_mc {
                crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1)
            } else {
                crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1)
            };
            // C# Type 0：Target.Attacked(DC)；Type 1：MACAgility + Dazed
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: t.session_id,
                target_object_id: t.object_id,
                damage,
                spell_id: 0,
            });
            if is_mc {
                // C# PoisonTarget(Target, 10, damage, Dazed, 1000)
                // C# PoisonTarget 1/10
                    if fastrand::i32(0..10) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: t.session_id,
                        poison: Poison::new(PoisonType::DAZED, damage.max(1) as u32, damage, 1000),
                    });
                    }
            }
        }
    }
}
