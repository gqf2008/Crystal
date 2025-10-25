// ============================================================================
// ECS Systems - 系统模块
// ============================================================================

pub mod camera;
pub mod player;
pub mod animation;
pub mod render;
pub mod network;
pub mod monster;
pub mod ui_system;
pub mod input_system;  // 🆕 输入系统
pub mod coordinate_system;  // 🆕 坐标系统
pub mod magic_learning_system;
pub mod magic_cast_system;
pub mod item_system;
pub mod npc_system;
pub mod combat_system;
pub mod quest_system;
pub mod trade_system;

// 重新导出
pub use camera::CameraSystem;
pub use player::PlayerSystem;
pub use animation::{AnimationSystem, DoorSystem};
pub use render::RenderSystem;
pub use network::NetworkSystem;
pub use monster::MonsterSystem;
pub use ui_system::UISystem;
pub use input_system::InputSystem;  // 🆕 输入系统
pub use coordinate_system::CoordinateSystem;  // 🆕 坐标系统
pub use magic_learning_system::MagicLearningSystem;
pub use magic_cast_system::MagicCastSystem;
pub use item_system::ItemSystem;
pub use npc_system::NPCSystem;
pub use combat_system::{CombatSystem, SkillEffectSystem, DamageType, CombatResult};
pub use quest_system::{QuestSystem, Quest, QuestLog, QuestState, QuestObjective, QuestReward};
pub use trade_system::{TradeSystem, ShopSystem, TradeWindow, TradeData, TradeState, ShopData, ShopItem};
