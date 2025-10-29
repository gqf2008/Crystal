// ============================================================================
// Layer 3: Combat & Skills (优先级 300-399)
// ============================================================================
//
// **职责**：处理战斗和技能释放的核心逻辑
//
// **系统列表**：
// - SkillSystem (300) - 技能施放、冷却管理、MP消耗
// - CombatSystem (310) - 伤害计算、命中判定、暴击处理
//
// **输入依赖**：
// - Layer 1: PlayerInput (攻击/施法事件)
// - Layer 2: AI决策系统产生的攻击意图
//
// **输出影响**：
// - 修改Health/Mana组件
// - 发布NetworkCommand
// - 触发音效/特效(Layer 7)
//
// ============================================================================

pub mod skill_system;
pub mod combat_system;

pub use skill_system::SkillSystem;
pub use combat_system::{CombatSystem, DamageType, CombatResult};
