//! 对话框管理器 - 减少重复代码

use crate::{ecs::GameWorld, network::handlers::GameEvent};
use ggez::winit::keyboard::KeyCode;

/// 通用的对话框action trait
pub trait DialogWithValidation {
    /// 处理Tab键切换字段
    fn on_tab(&mut self);

    /// 处理Backspace删除
    fn on_backspace(&mut self);

    /// 处理字符输入
    fn on_char(&mut self, ch: char);

    /// 是否可以提交
    fn can_submit(&self) -> bool;

    /// 获取验证错误消息
    fn get_validation_error(&self) -> String;

    /// 构建网络命令
    fn build_network_command(&self) -> GameEvent;
}

/// 键盘处理结果
pub enum DialogKeyResult {
    /// 对话框已处理事件
    Handled,
    /// 对话框应该关闭
    Close,
    /// 验证失败,需要显示错误消息
    ValidationFailed(String),
    /// 发送命令失败
    SendError(String),
}

/// 处理标准对话框键盘输入的辅助函数
pub fn handle_dialog_keycode<D>(
    dialog: &mut D,
    keycode: &KeyCode,
    text: Option<&str>,
    world: &mut GameWorld,
    error_context: &str,
) -> DialogKeyResult
where
    D: DialogWithValidation,
{
    match keycode {
        KeyCode::Escape => DialogKeyResult::Close,
        KeyCode::Tab => {
            dialog.on_tab();
            DialogKeyResult::Handled
        }
        KeyCode::Backspace => {
            dialog.on_backspace();
            DialogKeyResult::Handled
        }
        KeyCode::Enter => {
            if dialog.can_submit() {
                let cmd = dialog.build_network_command();
                if let Err(e) = world.network().send(cmd) {
                    tracing::error!("❌ {}: {}", error_context, e);
                    return DialogKeyResult::SendError(format!(
                        "网络错误，无法发送{}请求",
                        error_context
                    ));
                }
                DialogKeyResult::Handled
            } else {
                let error_msg = dialog.get_validation_error();
                DialogKeyResult::ValidationFailed(error_msg)
            }
        }
        _ => {
            if let Some(text) = text {
                for ch in text.chars() {
                    dialog.on_char(ch);
                }
            }
            DialogKeyResult::Handled
        }
    }
}
