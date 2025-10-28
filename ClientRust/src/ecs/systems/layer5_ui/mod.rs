// ============================================================================
// Layer 5: UI 层
// ============================================================================
//
// 职责：
// - UI 事件处理
// - UI 数据更新
// - 对话框管理
// - 物品/任务/交易系统
//
// 特点：
// - 事件驱动
// - 不负责 UI 渲染（渲染由 Layer 4 的 RenderSystem::draw_ui 负责）
//
// ============================================================================

pub mod ui_system;              // UI系统（向后兼容入口）
pub mod dialog_manager_system;  // 🆕 对话框管理系统（从ui_system拆分）
pub mod ui_event_dispatcher;    // 🆕 UI事件分发系统（从ui_system拆分）
pub mod item_system;
pub mod quest_system;
pub mod trade_system;
pub mod magic_learning_system;
pub mod keyboard_shortcut_system;  // 🆕 键盘快捷键系统
pub mod mouse_event_system;        // 🆕 鼠标事件系统

pub use ui_system::UISystem;
pub use dialog_manager_system::DialogManagerSystem;  // 🆕 导出
pub use ui_event_dispatcher::UIEventDispatcher;      // 🆕 导出
pub use item_system::ItemSystem;
pub use quest_system::QuestSystem;
pub use trade_system::TradeSystem;
pub use magic_learning_system::MagicLearningSystem;
pub use keyboard_shortcut_system::KeyboardShortcutSystem;
pub use mouse_event_system::MouseEventSystem;

