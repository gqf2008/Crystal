// ============================================================================
// Layer 5: State Update - HealthRegenSystem
// Priority: 510
// ============================================================================
//
// **职责**：
// - HP/MP自动恢复
// - Buff效果计时
// - DoT(持续伤害)效果
//
// **逻辑来源**：
// - C# HumanObject.ProcessRegen(): HP/MP恢复 (Line 550-647)
// - C# HumanObject.ProcessPoison(): DoT效果处理 (Line 650+)
//
// ============================================================================

use crate::game::GameResult;
use crate::game::GameContext;
use crate::systems::LogicSystem;
use crate::components::{Health, Mana, RegenTimer, BuffList, BuffType};

/// 生命恢复系统
/// 
/// 恢复规则 (参考C# HumanObject.ProcessRegen):
/// - HP恢复: 每10秒恢复 MaxHP * 3% + 1
/// - MP恢复: 每10秒恢复 MaxMP * 3% + 1
/// - Buff过期清理
/// - DoT伤害计算 (毒、流血等)
#[derive(ecs_macros::LogicSystem)]
pub struct HealthRegenSystem;

impl LogicSystem for HealthRegenSystem {
    

    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        let delta_ms = (delay_time * 1000.0) as u64;

        // 1. HP/MP自动恢复
        for (_id, (health, mana, regen_timer)) in ctx.world.query_mut::<(&mut Health, &mut Mana, &mut RegenTimer)>() {
            // 死亡（HP=0）不自动回血；复活应由服务器/道具/技能驱动。
            if health.current <= 0 {
                continue;
            }
            regen_timer.update(delta_ms);

            // HP恢复: 3% max HP + 1 (每10秒)
            if regen_timer.should_regen_hp() && health.current < health.max {
                let hp_regen = ((health.max as f32) * 0.03) as i32 + 1;
                health.current = (health.current + hp_regen).min(health.max);
                regen_timer.reset_hp_timer();
            }

            // MP恢复: 3% max MP + 1 (每10秒)
            if regen_timer.should_regen_mp() && mana.current < mana.max {
                let mp_regen = ((mana.max as f32) * 0.03) as i32 + 1;
                mana.current = (mana.current + mp_regen).min(mana.max);
                regen_timer.reset_mp_timer();
            }
        }

        // 2. 处理Buff效果 (过期清理和DoT伤害)
        for (_id, (health, buff_list)) in ctx.world.query_mut::<(&mut Health, &mut BuffList)>() {
            if health.current <= 0 {
                continue;
            }
            // 清理过期buff
            buff_list.cleanup_expired(delta_ms);

            // 处理DoT效果
            for buff in &buff_list.active_buffs {
                // 根据Buff类型应用每帧效果
                match buff.buff_type {
                    BuffType::Poison => {
                        // 毒伤害: 基于强度每秒造成伤害
                        if let Some(strength) = buff.strength {
                            let damage = ((strength as f32) * delay_time) as i32;
                            health.current = health.current.saturating_sub(damage);
                        }
                    }
                    BuffType::Bleeding => {
                        // 流血: 基于强度每秒造成伤害
                        if let Some(strength) = buff.strength {
                            let damage = ((strength as f32) * delay_time) as i32;
                            health.current = health.current.saturating_sub(damage);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
