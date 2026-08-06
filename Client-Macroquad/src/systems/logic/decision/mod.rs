//! Layer 2: AI与决策层 (200-299)
//!
//! 所有系统都实现 System trait
//!
//! 优先级顺序：
//! - MonsterAISystem(200) - 怪物AI
//! - NpcAISystem(210) - NPC AI  
//! - NpcDialogueSystem(220) - NPC对话

pub mod monster_ai_system;
pub mod npc_ai_system;
pub mod npc_dialogue_system;

pub use monster_ai_system::MonsterAISystem;
pub use npc_ai_system::NpcAISystem;
pub use npc_dialogue_system::NpcDialogueSystem;
