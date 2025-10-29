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

// 🆕 新架构组件
pub mod movement;        // 移动相关组件
pub mod prediction;      // 预测和插值组件
pub mod animation_state; // 动画状态组件
pub mod sound;           // 音效组件
pub mod particle;        // 粒子组件

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

// 🆕 新架构组件导出
pub use movement::*;
pub use prediction::*;
pub use animation_state::*;
pub use sound::*;
pub use particle::*;

// ============================================================================
// 其他系统组件的重新导出
// ============================================================================

// QuestLog 组件在 quest_system.rs 中定义，在这里重新导出
pub use crate::ecs::systems::layer5_ui::quest_system::QuestLog;

// TradeWindow 组件在 trade_system.rs 中定义，在这里重新导出
pub use crate::ecs::systems::layer5_ui::trade_system::TradeWindow;

// ============================================================================
// 公共导出 - 确保外部可以使用
// ============================================================================

// 导出效果混合模式
pub use crate::objects::SpriteBlendMode;
