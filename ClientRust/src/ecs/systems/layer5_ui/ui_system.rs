// ============================================================================
// UI 系统 - 统一入口（已拆分为独立系统）
// ============================================================================
//
// 本文件保留用于向后兼容，实际功能已拆分为:
// - DialogManagerSystem: 对话框管理（打开/关闭/切换）
// - UIEventDispatcher: UI事件分发（处理游戏事件）
//
// ============================================================================

use hecs::{World, Entity};
use crate::ecs::ui::{ChatDialog, ChatType};
use crate::network::game_client::GameEvent;

// 重新导出拆分后的系统
pub use super::dialog_manager_system::DialogManagerSystem;
pub use super::ui_event_dispatcher::UIEventDispatcher;

/// UI 系统 (向后兼容入口)
pub struct UISystem;

impl UISystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 更新所有 UI 组件
    pub fn update(&mut self, _world: &mut World) {
        // UI 更新主要通过事件驱动,在 process_event 中处理
        // 这里保留用于未来需要主动更新的UI逻辑
    }
    
    /// 处理游戏事件并更新UI (委托给 UIEventDispatcher)
    pub fn process_event(world: &mut World, event: &GameEvent) {
        UIEventDispatcher::process_event(world, event);
    }
    
    /// 添加聊天消息 (委托给 UIEventDispatcher)
    pub fn add_chat_message(world: &mut World, entity: Entity, text: String, chat_type: ChatType) {
        UIEventDispatcher::add_chat_message(world, entity, text, chat_type);
    }
    
    /// 设置金币 (委托给 UIEventDispatcher)
    pub fn set_gold(world: &mut World, gold: u32) {
        UIEventDispatcher::set_gold(world, gold);
    }
    
    /// 切换对话框 (委托给 DialogManagerSystem)
    pub fn toggle_dialog(world: &mut World, dialog_type: crate::ecs::ui::DialogType) {
        DialogManagerSystem::toggle_dialog(world, dialog_type);
    }
    
    /// 关闭最上层对话框 (委托给 DialogManagerSystem)
    pub fn close_top_dialog(world: &mut World) {
        DialogManagerSystem::close_top_dialog(world);
    }
    
    /// 处理 UI 点击 (委托给 DialogManagerSystem)
    pub fn handle_click(
        world: &mut World,
        button: ggez::winit::event::MouseButton,
        ui_x: f32,
        ui_y: f32,
    ) -> bool {
        DialogManagerSystem::handle_click(world, button, ui_x, ui_y)
    }
    
    /// 更新 UI hover 状态 (委托给 DialogManagerSystem)
    pub fn update_hover(world: &mut World, ui_x: f32, ui_y: f32) {
        DialogManagerSystem::update_hover(world, ui_x, ui_y);
    }
}



