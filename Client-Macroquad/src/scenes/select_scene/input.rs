use macroquad::prelude::*;

use crate::game::GameResult;
// 消息框点击由渲染侧按钮处理；这里仅负责键盘关闭与阻止底层输入。

use super::SelectScene;

impl SelectScene {
    /// 处理文本输入
    pub(super) fn handle_text_input(&mut self) {
        while let Some(ch) = get_char_pressed() {
            if ch.is_control() {
                continue;
            }

            if self.show_new_character
                && self.new_char_name.chars().count() < 16 {
                    self.new_char_name.push(ch);
                }
        }

        if is_key_pressed(KeyCode::Backspace)
            && self.show_new_character {
                self.new_char_name.pop();
            }
    }

    pub(super) fn handle_scene_input(&mut self) -> GameResult {
        // 消息框优先级最高：ESC/Enter 关闭，并阻止底层 UI。
        if self.show_message_box {
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                self.show_message_box = false;
            }
            return Ok(());
        }

        // Credits 输入优先级最高：可用 ESC / 点击任意位置关闭，并阻止底层 UI。
        if self.credits_dialog.is_visible() {
            let _ = self.credits_dialog.handle_input();
            return Ok(());
        }

        // 处理文本输入
        self.handle_text_input();

        // ESC 关闭对话框
        if is_key_pressed(KeyCode::Escape) {
            if self.show_new_character {
                self.show_new_character = false;
            } else if self.show_delete_character {
                self.show_delete_character = false;
            }
        }

        // Enter 确认
        if is_key_pressed(KeyCode::Enter) {
            if self.show_new_character {
                self.on_create_character();
            } else if self.show_delete_character {
                self.on_delete_character();
            }
        }

        Ok(())
    }
}
