// ============================================================================
// ECS Systems - 五层架构系统
// ============================================================================
//
// 架构设计：
// Layer 1: 输入与网络层 - 捕获输入，接收网络数据
// Layer 2: 核心逻辑层 - 游戏规则，物理模拟
// Layer 3: 表现状态层 - 动画/音效决策
// Layer 4: 渲染层     - 纯渲染，不含逻辑
// Layer 5: UI层       - UI事件处理
//
// 数据流：Layer 1 → Layer 2 → Layer 3 → Layer 4 → Layer 5
//
// ============================================================================

// 五层架构模块
pub mod layer1_input;
pub mod layer2_logic;
pub mod layer3_presentation;
pub mod layer4_rendering;
pub mod layer5_ui;

// 重新导出各层系统
pub use layer1_input::{InputCollectingSystem, ClientNetworkSystem};
pub use layer2_logic::{
    LocalPredictionSystem, MovementSystemV2, ReconciliationSystem, InterpolationSystem,
    MonsterSystem, NPCSystem, CombatSystem, MagicCastSystem,
};
pub use layer3_presentation::{AnimationStateSystem, NPCActionSystem, MonsterAnimationStateSystem};
pub use layer4_rendering::{
    RenderSystem, CameraSystem, OcclusionSystem,
    AnimationPlaybackSystem, TileAnimationSystem, MovementInterpolationSystem,
};
pub use layer5_ui::{
    UISystem, ItemSystem, QuestSystem, TradeSystem, MagicLearningSystem,
    KeyboardShortcutSystem, MouseEventSystem,  // 🆕 输入事件处理系统
    DialogManagerSystem, UIEventDispatcher,    // 🆕 UI系统拆分后的子系统
};

// ============================================================================
// 废弃系统（已全部删除）
// ============================================================================
// 
// 状态说明：
// - ✅ PathfindingSystem: 已删除 → LocalPredictionSystem (Layer 2)
// - ✅ MovementSystem: 已删除 → MovementSystemV2 (Layer 2)
// - ✅ InputSystem: 已删除 → KeyboardShortcutSystem + MouseEventSystem (Layer 5)
// - ✅ NetworkSystem: 已删除 → ClientNetworkSystem (Layer 1)
// - ✅ AnimationSystem: 已删除 → AnimationStateSystem (Layer 3) + AnimationPlaybackSystem (Layer 4)
// - ✅ DoorSystem: 已删除（仅在 map_viewer 中使用，已注释）
//
// 清理完成：2025-10-28
// ============================================================================

// 重新导出其他系统的特殊类型
pub use layer2_logic::combat_system::{SkillEffectSystem, DamageType, CombatResult};
pub use layer5_ui::quest_system::{Quest, QuestState, QuestObjective, QuestReward};
pub use layer5_ui::trade_system::{ShopSystem, TradeData, TradeState, ShopData, ShopItem};
