//! # 战斗系统模块 (logic/combat)
//!
//! **优先级范围**: 300-399
//!
//! ## 模块职责
//!
//! 负责游戏中的战斗相关逻辑：
//! 1. 战斗计算（伤害、命中、暴击等）
//! 2. 技能释放与效果
//! 3. 生命/魔法回复
//!
//! ## 系统列表
//!
//! | 系统 | 优先级 | 依赖组件（读） | 依赖组件（写） | 职责 |
//! |------|--------|----------------|----------------|------|
//! | CombatSystem | 300 | Position, CombatStats, Target | Health, CombatState | 战斗逻辑与伤害计算 |
//! | SkillSystem | 310 | Position, SkillData, Mana | Health, Mana, SkillCooldown | 技能释放与效果 |
//! | HealthRegenSystem | 330 | Health, RegenRate, Time | Health, Mana | 生命/魔法回复 |
//!
//! ## 数据流
//!
//! ```text
//! 玩家输入/AI决策
//!         ↓
//! CombatSystem: 计算攻击 → 更新 Health
//!         ↓
//! SkillSystem: 技能效果 → 更新 Health/Mana
//!         ↓
//! HealthRegenSystem: 自然回复 → 更新 Health/Mana
//!         ↓
//! 后续系统检测死亡/状态变化
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::systems::logic::combat::{CombatSystem, DamageType};
//! use crate::components::{Health, CombatStats};
//!
//! // 创建战斗实体
//! world.spawn((
//!     Health::new(100),
//!     CombatStats {
//!         level: 10,
//!         attack_min: 5,
//!         attack_max: 15,
//!         defense: 10,
//!         ..Default::default()
//!     },
//! ));
//!
//! // CombatSystem 会处理战斗逻辑
//! ```
//!
//! ## 注意事项
//!
//! - CombatSystem 必须在 HealthRegenSystem 之前执行（避免重复回复）
//! - SkillSystem 依赖 CombatSystem 的伤害计算结果
//! - 死亡检测应该在战斗系统之后执行
// ============================================================================

pub mod skill_system;
pub mod combat_system;
pub mod regen_system;

pub use regen_system::HealthRegenSystem;
pub use skill_system::SkillSystem;
pub use combat_system::{CombatSystem, DamageType, CombatResult};
