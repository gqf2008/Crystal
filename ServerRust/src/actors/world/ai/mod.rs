//! 怪物 AI 行为模块
//!
//! 架构：每个 Boss / 特殊怪物实现 `MonsterBehavior` trait（对齐 C# 类继承）。
//! 普通怪物用 `DefaultBehavior`（AI 逻辑保留在 tick_monsters 内联）。
//! Boss 通过名称匹配注册（对齐 C# Settings 里的字符串配置）。

pub mod behavior;
pub mod ctx;
pub mod default;
pub mod registry;
pub mod helpers;
pub mod bosses;

pub use behavior::MonsterBehavior;
pub use ctx::{AiCtx, PlayerSnap, MonsterSnap, AttackAction, SpellFieldSpawn, BossSummon, PoisonPlayer};
pub use default::DefaultBehavior;
pub use registry::{make_behavior, is_registered_boss, is_static_object, is_passive_object};
