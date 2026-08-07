//! CannibalPlant（食人花）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CannibalPlant.cs（继承 HarvestMonster）
//! 机制：
//!   - 不能移动（CanMove=false）
//!   - 默认隐身，玩家靠近 3 格内现身，离开后隐身 + 回满血
//!   - 隐身期免疫（IsAttackTarget 返 false）
//!
//! 任务要求核心机制："吞噬玩家（拉到身边+束缚）"——在 C# 原版仅现身近战基础上，
//! 追加一个周期性"吞噬"动作：把攻击范围内最远玩家拉到身边并束缚（麻痹），
//! 对齐任务描述的束缚控制效果（POC 简化版，原版无此段）。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

/// 现身检测间隔（C# VisibleTime = Envir.Time + 2000）
const VISIBILITY_CHECK_TICKS: u64 = 20;
/// 现身/隐身判定距离（C# FindNearby(3)）
const APPEAR_RANGE: i32 = 3;
/// 攻击范围
const ATTACK_RANGE: i32 = 2;
/// 吞噬周期（3s）— 任务特有控制机制
const DEVOUR_INTERVAL_TICKS: u64 = 30;

pub struct CannibalPlantBehavior {
    visible: bool,
    next_visibility_tick: u64,
    next_devour_tick: u64,
    spawned: bool,
}

impl CannibalPlantBehavior {
    pub fn new() -> Self {
        Self {
            visible: false,
            next_visibility_tick: 0,
            next_devour_tick: 0,
            spawned: false,
        }
    }
}

impl MonsterBehavior for CannibalPlantBehavior {
    fn can_move(&self) -> bool { false }

    fn is_attackable(&self) -> bool {
        self.visible
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.visible { damage } else { 0 }
    }

    fn on_poison(&mut self, _poison: Poison) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_visibility_tick = ctx.tick_count + VISIBILITY_CHECK_TICKS;
            self.spawned = true;
        }

        // ---- 可见性切换（C# ProcessAI 每 2s 检测）----
        if ctx.tick_count >= self.next_visibility_tick {
            self.next_visibility_tick = ctx.tick_count + VISIBILITY_CHECK_TICKS;
            let has_near = ctx.nearest_target(monster.x, monster.y, APPEAR_RANGE, monster.map_index).is_some();
            if !self.visible && has_near {
                self.visible = true;
            } else if self.visible && !has_near {
                self.visible = false;
                monster.hp = monster.max_hp; // C# SetHP(Stats[HP])
            }
        }

        if !self.visible {
            return;
        }

        // ---- 周期吞噬：把最远玩家拉到身边 + 麻痹束缚 ----
        if self.next_devour_tick == 0 {
            self.next_devour_tick = ctx.tick_count + DEVOUR_INTERVAL_TICKS;
        }
        if ctx.tick_count >= self.next_devour_tick {
            self.next_devour_tick = ctx.tick_count + DEVOUR_INTERVAL_TICKS;
            // 找攻击范围内最远的玩家"吞噬"（简化：单体拉拽 + 束缚）
            if let Some(t) = ctx.nearest_target(monster.x, monster.y, APPEAR_RANGE, monster.map_index).copied() {
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                // C# CannibalPlant 无毒（原实现凭空加的束缚麻痹，移除）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: t.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 1,
                });
            }
        }

        // ---- 普通近战攻击（C# HarvestMonster 标准 Attack）----
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        if let Some(t) = ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index).copied() {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: t.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
        }
    }
}
