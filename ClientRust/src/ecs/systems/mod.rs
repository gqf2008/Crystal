// ============================================================================
// ECS Systems - 系统模块
// ============================================================================

pub mod camera_system;
pub mod movement_system;      // 🆕 移动系统（替代player_system）
pub mod pathfinding_system;   // 🆕 寻路系统
pub mod animation_system;
pub mod render_system;
pub mod network_system;
pub mod monster_system;
pub mod ui_system;
pub mod input_system;
pub mod magic_learning_system;
pub mod magic_cast_system;
pub mod item_system;
pub mod npc_system;
pub mod combat_system;
pub mod quest_system;
pub mod trade_system;
pub mod occlusion_system;

// 重新导出
pub use camera_system::CameraSystem;
pub use movement_system::MovementSystem;       // 🆕 移动系统
pub use pathfinding_system::PathfindingSystem; // 🆕 寻路系统
pub use animation_system::{AnimationSystem, DoorSystem, NPCActionSystem};
pub use render_system::RenderSystem;
pub use network_system::NetworkSystem;
pub use monster_system::MonsterSystem;
pub use ui_system::UISystem;
pub use input_system::InputSystem;
pub use magic_learning_system::MagicLearningSystem;
pub use magic_cast_system::MagicCastSystem;
pub use item_system::ItemSystem;
pub use npc_system::NPCSystem;
pub use combat_system::{CombatSystem, SkillEffectSystem, DamageType, CombatResult};
pub use quest_system::{QuestSystem, Quest, QuestLog, QuestState, QuestObjective, QuestReward};
pub use trade_system::{TradeSystem, ShopSystem, TradeWindow, TradeData, TradeState, ShopData, ShopItem};
pub use occlusion_system::OcclusionSystem;
