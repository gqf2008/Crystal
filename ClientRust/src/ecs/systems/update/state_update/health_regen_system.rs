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

use hecs::World;
use ggez::GameResult;
use crate::ecs::systems::{System, priority};
use crate::ecs::components::{Health, Mana, RegenTimer, BuffList, BuffType};

/// 生命恢复系统
/// 
/// 恢复规则 (参考C# HumanObject.ProcessRegen):
/// - HP恢复: 每10秒恢复 MaxHP * 3% + 1
/// - MP恢复: 每10秒恢复 MaxMP * 3% + 1
/// - Buff过期清理
/// - DoT伤害计算 (毒、流血等)
pub struct HealthRegenSystem;

impl System for HealthRegenSystem {
    fn priority(&self) -> u32 {
        priority::ANIMATION // 使用510优先级
    }

    fn update(&mut self, world: &mut World, delay_time: f32) -> GameResult {
        let delta_ms = (delay_time * 1000.0) as u64;

        // 1. HP/MP自动恢复
        for (_id, (health, mana, regen_timer)) in world.query_mut::<(&mut Health, &mut Mana, &mut RegenTimer)>() {
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
        for (_id, (health, buff_list)) in world.query_mut::<(&mut Health, &mut BuffList)>() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Buff;

    #[test]
    fn test_hp_regeneration() {
        let mut world = World::new();
        let mut system = HealthRegenSystem;

        let entity = world.spawn((
            Health { current: 50, max: 100 },
            Mana { current: 30, max: 100 },
            RegenTimer {
                hp_timer: 10000, // 已满10秒
                mp_timer: 0,
                hp_interval: 10000,
                mp_interval: 10000,
            },
        ));

        system.update(&mut world, 0.016).unwrap();

        let health = world.get::<&Health>(entity).unwrap();
        // 50 + (100 * 0.03 + 1) = 50 + 4 = 54
        assert_eq!(health.current, 54);
    }

    #[test]
    fn test_mp_regeneration() {
        let mut world = World::new();
        let mut system = HealthRegenSystem;

        let entity = world.spawn((
            Health { current: 100, max: 100 },
            Mana { current: 50, max: 100 },
            RegenTimer {
                hp_timer: 0,
                mp_timer: 10000, // 已满10秒
                hp_interval: 10000,
                mp_interval: 10000,
            },
        ));

        system.update(&mut world, 0.016).unwrap();

        let mana = world.get::<&Mana>(entity).unwrap();
        // 50 + (100 * 0.03 + 1) = 50 + 4 = 54
        assert_eq!(mana.current, 54);
    }

    #[test]
    fn test_buff_expiration() {
        let mut world = World::new();
        let mut system = HealthRegenSystem;

        world.spawn((
            Health { current: 100, max: 100 },
            BuffList {
                active_buffs: vec![
                    Buff::new(BuffType::Poison).with_duration(50), // 50ms后过期
                    Buff::new(BuffType::SpeedBoost).with_duration(200), // 200ms后过期
                ],
            },
        ));

        system.update(&mut world, 0.1).unwrap(); // 100ms更新

        // 验证50ms的buff已被移除,200ms的buff还在
        for (_id, buff_list) in world.query_mut::<&BuffList>() {
            assert_eq!(buff_list.active_buffs.len(), 1, "Should have 1 buff remaining");
            assert_eq!(buff_list.active_buffs[0].buff_type, BuffType::SpeedBoost);
        }
    }

    #[test]
    fn test_poison_damage() {
        let mut world = World::new();
        let mut system = HealthRegenSystem;

        let entity = world.spawn((
            Health { current: 100, max: 100 },
            BuffList {
                active_buffs: vec![
                    Buff::new(BuffType::Poison).with_duration(10000).with_strength(5), // 每秒5点伤害
                ],
            },
        ));

        system.update(&mut world, 1.0).unwrap(); // 1秒

        let health = world.get::<&Health>(entity).unwrap();
        assert_eq!(health.current, 95); // 100 - 5 = 95
    }
}

