//! EvilCentipede（触角恶魔/地蜈蚣）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/EvilCentipede.cs
//! 机制：
//!   - 不能移动（CanMove=false）
//!   - 默认隐身（钻地），玩家靠近 3 格内现身，玩家离开 7 格内再次隐身
//!   - 隐身期免疫（IsAttackTarget 返 false）+ 满血（ProcessAI 每帧 SetHP(MaxHP)）
//!   - 现身期：7 格内全体玩家 MAC 攻击 + 绿毒 + 麻痹
//!
//! ProcessAI（C# EvilCentipede.cs:35-66）：每 2s 检测附近玩家切换 Visible。
//! CompleteAttack（C# :92-102）：FindAllTargets(7) 逐个 Attack。
//! Attack（C# :118-127）：MAC 伤害 + Green 15s + Paralysis 5s。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

/// 现身检测间隔（C# VisibleTime = Envir.Time + 2000，单位 tick=100ms）
const VISIBILITY_CHECK_TICKS: u64 = 20;
/// 现身触发距离（C# FindNearby(3) 隐身时）
const APPEAR_RANGE: i32 = 3;
/// 隐身判定距离（C# FindNearby(7) 现身时）
const DISAPPEAR_RANGE: i32 = 7;
/// 攻击范围（C# FindAllTargets(7)）
const ATTACK_RANGE: i32 = 7;

pub struct EvilCentipedeBehavior {
    /// 当前是否现身（钻地=false=隐身）
    visible: bool,
    /// 下次检测可见性的 tick
    next_visibility_tick: u64,
    spawned: bool,
}

impl EvilCentipedeBehavior {
    pub fn new() -> Self {
        Self {
            visible: false,
            next_visibility_tick: 0,
            spawned: false,
        }
    }
}

impl MonsterBehavior for EvilCentipedeBehavior {
    fn can_move(&self) -> bool { false }
    fn can_regen(&self) -> bool { false } // 隐身即满血，无需自然回血

    /// 隐身期不可被攻击（C# IsAttackTarget 返 Visible && ...）
    fn is_attackable(&self) -> bool {
        self.visible
    }

    /// 隐身期完全免疫伤害（C# IsAttackTarget 返 false → Attacked 不生效）
    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.visible { damage } else { 0 }
    }

    /// 免疫毒（钻地隐身期无法被施毒）
    fn on_poison(&mut self, _poison: Poison) -> bool { false }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.next_visibility_tick = ctx.tick_count + VISIBILITY_CHECK_TICKS;
            self.spawned = true;
        }

        // ---- 可见性切换（C# ProcessAI 每 2s 检测）----
        if ctx.tick_count >= self.next_visibility_tick {
            self.next_visibility_tick = ctx.tick_count + VISIBILITY_CHECK_TICKS;
            let detect_range = if self.visible { DISAPPEAR_RANGE } else { APPEAR_RANGE };
            let has_near = ctx.nearest_target(monster.x, monster.y, detect_range, monster.map_index).is_some();
            if !self.visible && has_near {
                // 现身（C# Visible = true + Broadcast ObjectShow + 满血）
                self.visible = true;
                monster.hp = monster.max_hp;
            } else if self.visible && !has_near {
                // 隐身（C# Visible = false + SetHP(MaxHP)）
                self.visible = false;
                monster.hp = monster.max_hp;
            }
        }

        // 隐身期每 tick 保持满血（C# if(!Visible) SetHP(Stats[HP])）
        if !self.visible {
            monster.hp = monster.max_hp;
            return;
        }

        // ---- 现身期攻击：7 格内全体玩家 MAC + 毒（C# CompleteAttack + Attack）----
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        let has_target = ctx.nearest_target(monster.x, monster.y, DISAPPEAR_RANGE, monster.map_index).is_some();
        if !has_target {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;

        // 先收集目标避免借用冲突
        let targets: Vec<crate::actors::world::ai::PlayerSnap> =
            ctx.find_targets_in_range(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
                .into_iter().copied().collect();

        for t in targets {
            let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
            // C# DefenceType.MAC
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: t.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            // Green 15s（C# PoisonTarget(Target, 5, 15, Green, 2000)）
            // C# PoisonTarget 1/5
                if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: t.session_id,
                    poison: Poison::new(PoisonType::GREEN, 15, damage, 2000),
                });
                }
            // Paralysis 5s（C# PoisonTarget(Target, 15, 5, Paralysis, 2000)）
            // C# PoisonTarget 1/15
                if fastrand::i32(0..15) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: t.session_id,
                    poison: Poison::new(PoisonType::PARALYSIS, 5, damage, 1000),
                });
                }
        }
    }
}
