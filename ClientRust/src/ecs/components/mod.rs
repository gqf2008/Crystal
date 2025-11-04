// ============================================================================
// Components - ECS 组件定义
// 参考 C# Client/MirObjects/ 的对象属性
// ============================================================================

// 子模块声明
pub mod debug;
pub mod core;
pub mod combat;
pub mod player;
pub mod actor;
pub mod item;
pub mod spell;
pub mod map;
pub mod render;
pub mod input;
pub mod network;
pub mod character_select;  // 🆕 角色选择组件

// 🆕 新架构组件
pub mod movement;        // 移动相关组件
pub mod prediction;      // 预测和插值组件 (被 MapUpdateSystem 使用)
// pub mod animation_state; // ❌ 已删除 - 渲染系统直接使用 PlayerAction
pub mod sound;           // 音效组件
pub mod particle;        // 粒子组件
pub mod events;          // 🆕 全局事件组件
// pub mod state_machine;   // ❌ 已移动到 player.rs

// ============================================================================
// 统一导出所有组件
// ============================================================================

// 调试组件
pub use debug::*;

// 核心组件
pub use core::*;

// 战斗组件
pub use combat::*;

// 玩家组件
pub use player::*;

// 怪物/NPC组件
pub use actor::*;

// 物品组件
pub use item::*;

// 技能组件
pub use spell::*;

// 地图组件
pub use map::*;

// 渲染组件
pub use render::*;

// 输入组件
pub use input::*;

// 网络组件
pub use network::*;

// 角色选择组件
pub use character_select::*;  // 🆕 角色选择组件导出

// 🆕 新架构组件导出
pub use movement::*;
pub use prediction::*;  // ✅ 被 MapUpdateSystem 使用
// pub use animation_state::*;  // ❌ 已删除
pub use sound::*;
pub use particle::*;
pub use events::*;  // 🆕 全局事件组件导出
// pub use state_machine::*;  // ❌ 已移动到 player.rs (由 player 模块导出)

// ============================================================================
// 其他系统组件的重新导出
// ============================================================================

// 注意: QuestLog 和 TradeWindow 等 UI 组件已从 layer5_ui 移除
// 将来如果需要，应该在新的 UI 系统中重新实现

// ============================================================================
// 公共导出 - 确保外部可以使用
// ============================================================================

// 导出效果混合模式
pub use crate::objects::SpriteBlendMode;
