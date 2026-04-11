use macroquad::prelude::*;

use crate::network::NetworkEvent;
use crate::resources::LibraryName;
use crate::ui::widgets::{begin_modal, draw_button, draw_input_box};

use super::LoginScene;

#[derive(PartialEq, Clone, Copy)]
pub(super) enum ChangePasswordFocus {
    AccountId,
    CurrentPassword,
    NewPassword1,
    NewPassword2,
}

impl LoginScene {
    pub(super) fn open_change_password_dialog(&mut self) {
        self.show_change_password = true;
        self.cp_in_flight = false;

        // 原版支持自动填充账号/当前密码，这里按当前登录框内容预填。
        self.cp_account_id = self.account.clone();
        self.cp_current_password = self.password.clone();
        self.cp_new_password1.clear();
        self.cp_new_password2.clear();
        self.cp_focus = ChangePasswordFocus::AccountId;
    }

    pub(super) fn close_change_password_dialog(&mut self) {
        self.show_change_password = false;
        self.cp_in_flight = false;
    }

    fn can_submit_change_password(&self) -> bool {
        !self.cp_in_flight
            && !self.cp_account_id.is_empty()
            && !self.cp_current_password.is_empty()
            && !self.cp_new_password1.is_empty()
            && self.cp_new_password1 == self.cp_new_password2
    }

    fn submit_change_password(&mut self) {
        if !self.can_submit_change_password() {
            self.message_text = "请填写完整信息，并确认两次新密码一致".to_string();
            self.show_message = true;
            return;
        }

        self.ensure_network();
        let Some(net) = self.net.as_ref() else {
            self.message_text = "尚未连接服务器".to_string();
            self.show_message = true;
            return;
        };

        if !self.version_ok {
            self.message_text = "正在校验版本...".to_string();
            self.show_message = true;
            return;
        }

        self.cp_in_flight = true;
        if let Err(e) = net.send(NetworkEvent::ChangePasswordRequest {
            account_id: self.cp_account_id.clone(),
            current_password: self.cp_current_password.clone(),
            new_password: self.cp_new_password1.clone(),
        }) {
            self.cp_in_flight = false;
            self.message_text = format!("发送 ChangePasswordRequest 失败: {e}");
            self.show_message = true;
        } else {
            self.message_text = "正在修改密码...".to_string();
            self.show_message = true;
        }
    }

    pub(super) fn handle_change_password_text_input(&mut self) {
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii() && !ch.is_control() {
                match self.cp_focus {
                    ChangePasswordFocus::AccountId => {
                        if self.cp_account_id.len() < 20 {
                            self.cp_account_id.push(ch);
                        }
                    }
                    ChangePasswordFocus::CurrentPassword => {
                        if self.cp_current_password.len() < 20 {
                            self.cp_current_password.push(ch);
                        }
                    }
                    ChangePasswordFocus::NewPassword1 => {
                        if self.cp_new_password1.len() < 20 {
                            self.cp_new_password1.push(ch);
                        }
                    }
                    ChangePasswordFocus::NewPassword2 => {
                        if self.cp_new_password2.len() < 20 {
                            self.cp_new_password2.push(ch);
                        }
                    }
                }
            }
        }

        if is_key_pressed(KeyCode::Backspace) {
            match self.cp_focus {
                ChangePasswordFocus::AccountId => {
                    self.cp_account_id.pop();
                }
                ChangePasswordFocus::CurrentPassword => {
                    self.cp_current_password.pop();
                }
                ChangePasswordFocus::NewPassword1 => {
                    self.cp_new_password1.pop();
                }
                ChangePasswordFocus::NewPassword2 => {
                    self.cp_new_password2.pop();
                }
            }
        }

        if is_key_pressed(KeyCode::Tab) {
            self.cp_focus = match self.cp_focus {
                ChangePasswordFocus::AccountId => ChangePasswordFocus::CurrentPassword,
                ChangePasswordFocus::CurrentPassword => ChangePasswordFocus::NewPassword1,
                ChangePasswordFocus::NewPassword1 => ChangePasswordFocus::NewPassword2,
                ChangePasswordFocus::NewPassword2 => ChangePasswordFocus::AccountId,
            };
        }

        if is_key_pressed(KeyCode::Enter)
            && self.can_submit_change_password() {
                self.submit_change_password();
            }
    }

    pub(super) fn draw_change_password_dialog(&mut self) {
        // 背景 Prguse[50]
        let (dialog_w, dialog_h) = if let Some(info) = LibraryName::Prguse.get_texture(50) {
            (info.width as f32, info.height as f32)
        } else {
            (400.0, 280.0)
        };

        let dialog = begin_modal(dialog_w, dialog_h, 120);
        let dialog_x = dialog.x;
        let dialog_y = dialog.y;

        if let Some(info) = LibraryName::Prguse.get_texture(50) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x, dialog_y, WHITE);
            }
        } else {
            draw_rectangle(
                dialog_x,
                dialog_y,
                dialog_w,
                dialog_h,
                Color::from_rgba(30, 30, 40, 240),
            );
            draw_rectangle_lines(
                dialog_x,
                dialog_y,
                dialog_w,
                dialog_h,
                2.0,
                Color::from_rgba(120, 120, 140, 255),
            );
        }

        // 输入框：对齐原版坐标
        let input_w = 136.0;
        let input_h = 18.0;
        let input_x = dialog_x + 178.0;

        let y_account = dialog_y + 75.0;
        let y_current = dialog_y + 113.0;
        let y_new1 = dialog_y + 151.0;
        let y_new2 = dialog_y + 188.0;

        draw_input_box(
            input_x,
            y_account,
            input_w,
            input_h,
            &self.cp_account_id,
            false,
            self.cp_focus == ChangePasswordFocus::AccountId,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            input_x,
            y_current,
            input_w,
            input_h,
            &self.cp_current_password,
            true,
            self.cp_focus == ChangePasswordFocus::CurrentPassword,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            input_x,
            y_new1,
            input_w,
            input_h,
            &self.cp_new_password1,
            true,
            self.cp_focus == ChangePasswordFocus::NewPassword1,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            input_x,
            y_new2,
            input_w,
            input_h,
            &self.cp_new_password2,
            true,
            self.cp_focus == ChangePasswordFocus::NewPassword2,
            self.cursor_visible,
            14.0,
        );

        // 按钮：OK Title[107-109] at (80,236) / Cancel Title[110-112] at (222,236)
        let ok_enabled = self.can_submit_change_password();
        if draw_button(
            LibraryName::Title,
            dialog_x + 80.0,
            dialog_y + 236.0,
            107,
            108,
            109,
            ok_enabled,
        ) {
            self.submit_change_password();
        }

        if draw_button(
            LibraryName::Title,
            dialog_x + 222.0,
            dialog_y + 236.0,
            110,
            111,
            112,
            true,
        ) {
            self.close_change_password_dialog();
        }

        // 鼠标点输入框切换焦点
        let (mx, my) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left)
            && mx >= input_x && mx <= input_x + input_w {
                if my >= y_account && my <= y_account + input_h {
                    self.cp_focus = ChangePasswordFocus::AccountId;
                } else if my >= y_current && my <= y_current + input_h {
                    self.cp_focus = ChangePasswordFocus::CurrentPassword;
                } else if my >= y_new1 && my <= y_new1 + input_h {
                    self.cp_focus = ChangePasswordFocus::NewPassword1;
                } else if my >= y_new2 && my <= y_new2 + input_h {
                    self.cp_focus = ChangePasswordFocus::NewPassword2;
                }
            }
    }
}
