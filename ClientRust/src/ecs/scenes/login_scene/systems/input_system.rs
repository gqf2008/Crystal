//! 输入处理系统 - 统一处理鼠标和键盘事件
//! 
//! 替代原来的on_mouse_xxx和on_key_xxx方法

use hecs::World;
use super::super::components::*;
use super::super::ui::{button_helpers, input_helpers};

/// 处理鼠标移动事件（更新悬停状态）
pub fn handle_mouse_move(world: &mut World, mouse_x: f32, mouse_y: f32) {
    button_helpers::update_hover(world, mouse_x, mouse_y);
}

/// 处理鼠标点击事件
pub fn handle_mouse_click(world: &World, mouse_x: f32, mouse_y: f32) -> Option<ButtonAction> {
    button_helpers::handle_click(world, mouse_x, mouse_y)
}

/// 处理字符输入
pub fn handle_char_input(world: &mut World, ch: char) {
    input_helpers::handle_char_input(world, ch);
}

/// 处理退格键
pub fn handle_backspace(world: &mut World) {
    input_helpers::handle_backspace(world);
}

/// 处理Tab键
pub fn handle_tab(world: &mut World) {
    input_helpers::handle_tab(world);
}

/// 处理Enter键
pub fn handle_enter(world: &World) -> Option<ButtonAction> {
    // 查找当前启用的OK按钮
    for (_entity, (button, clickable)) in world.query::<(&Button, &Clickable)>().iter() {
        if clickable.enabled && button.enabled {
            match button.action {
                ButtonAction::Login
                | ButtonAction::NewAccountOk
                | ButtonAction::ChangePasswordOk => {
                    return Some(button.action);
                }
                _ => {}
            }
        }
    }
    None
}

/// 处理Escape键
pub fn handle_escape() -> bool {
    // 返回true表示需要关闭当前对话框
    true
}

/// 点击输入框聚焦
pub fn handle_input_click(world: &mut World, mouse_x: f32, mouse_y: f32) -> bool {
    // 先查找被点击的输入框
    let clicked_field = {
        let mut result = None;
        for (_entity, (bounds, input_field)) in world.query::<(&Bounds, &InputField)>().iter() {
            if bounds.contains(mouse_x, mouse_y) {
                result = Some(input_field.field_type);
                break;
            }
        }
        result
    };
    
    // 然后聚焦（避免借用冲突）
    if let Some(field_type) = clicked_field {
        input_helpers::focus_field(world, field_type);
        tracing::debug!("🎯 点击输入框: {:?}", field_type);
        return true;
    }
    
    false
}
