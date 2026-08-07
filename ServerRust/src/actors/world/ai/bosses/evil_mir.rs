//! EvilMir（邪恶巨龙）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/EvilMir.cs
//! 机制：不能移动、睡眠周期、全图视野、1/8 概率全屏大招 vs 普攻、攻击带绿毒+麻痹、
//! 睡眠期完全免疫、DragonLink 死亡=睡眠 5 分钟（简化：死亡即真死，DragonLink 留 TODO）

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;

/// 睡眠唤醒延迟（5 分钟，单位 tick；100ms/tick → 5min=3000 ticks）
pub(crate) const SLEEP_DURATION_TICKS: u64 = 3000;
/// 攻击冷却（tick）
const EVIL_MIR_ATTACK_COOLDOWN: u64 = 8;
/// 行动间隔（tick）
#[allow(dead_code)]
const EVIL_MIR_ACTION_INTERVAL: u64 = 3;

pub struct EvilMirBehavior {
    sleeping: bool,
    wake_up_tick: u64,
    mass_attack: bool,
}

impl EvilMirBehavior {
    pub fn new() -> Self {
        Self {
            sleeping: false,
            wake_up_tick: 0,
            mass_attack: false,
        }
    }

    /// DragonLink 模式下死亡=睡眠 5 分钟（C# EvilMir.Die：不真死，睡眠后满血苏醒）
    pub(crate) fn sleep_on_death(&mut self, tick_count: u64) {
        self.sleeping = true;
        self.wake_up_tick = tick_count + SLEEP_DURATION_TICKS;
    }

}

impl MonsterBehavior for EvilMirBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn is_attackable(&self) -> bool {
        !self.sleeping
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        // 睡眠期完全免疫（C# EvilMir.cs:152）
        if self.sleeping {
            0
        } else {
            damage
        }
    }

    fn on_poison(&mut self, _poison: Poison) -> bool {
        false // EvilMir 免疫毒（C# 未显式 ApplyPoison override，但 Boss 默认高抗）
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 睡眠唤醒检查（C# EvilMir.cs:77-87）
        if self.sleeping {
            if ctx.tick_count >= self.wake_up_tick {
                self.sleeping = false;
                monster.hp = monster.max_hp; // 唤醒满血
            }
            return;
        }

        // 行动间隔检查
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        // 全图视野找目标（C# InAttackRange 用 ViewRange）
        let target = match ctx.nearest_target(monster.x, monster.y, 30, monster.map_index) {
            Some(t) => *t,
            None => return,
        };

        // 攻击冷却
        monster.next_attack_tick = ctx.tick_count + EVIL_MIR_ATTACK_COOLDOWN;
        monster.ai_state = crate::actors::world::MonsterAiState::Attack;

        // 1/8 概率全屏大招 vs 普攻（C# EvilMir.cs:115）
        let is_mass = fastrand::i32(0..8) == 0;
        self.mass_attack = is_mass;

        // C# EvilMir.Attack：DragonLink 时 MaxDC + (DragonLevel-1)*10（按意图修正 C# 运算符优先级）
        let dragon_bonus = if ctx.dragon_level > 0 {
            (ctx.dragon_level as i32 - 1) * 10
        } else {
            0
        };
        let damage = crate::combat::attack::get_attack_power(
            monster.min_dmg, monster.max_dmg + dragon_bonus, monster.luck);
        let final_damage = if is_mass { damage } else { (damage as f32 * 0.75) as i32 }.max(1);

        // C# EvilMir.cs:139-150：MAC 伤害 + 绿毒 + 麻痹
        // 全屏大招：FindAllTargets(17)；普攻：FindAllTargets(2, target)
        let (center_x, center_y, radius) = if is_mass {
            (monster.x, monster.y, 17)
        } else {
            (target.x, target.y, 2)
        };

        // 广播攻击
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
            center_x,
            center_y,
            radius,
            damage: final_damage,
            spell_id: 0,
        });

        // 对命中目标施毒（绿毒 15s + 麻痹 5s）。先收集 session_id 避免借用冲突。
        let hit_sessions: Vec<u64> = ctx.find_targets_in_range(center_x, center_y, radius, monster.map_index)
            .iter().map(|p| p.session_id).collect();
        for sid in hit_sessions {
            // C# PoisonTarget 1/5
                if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::GREEN, 15, damage, 2000),
                });
                }
            // C# PoisonTarget 1/5
                if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: Poison::new(PoisonType::PARALYSIS, 5, damage, 1000),
                });
                }
        }
    }

    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // 真死路径（非 DragonLink）：无需特殊处理
    }
}
