use macroquad::prelude::*;

use chrono::{Datelike, TimeZone, Utc};

use crate::network::NetworkEvent;
use crate::resources::LibraryName;
use crate::ui::widgets::{
    begin_modal, draw_button, draw_input_box, draw_multiline_text_cn,
};

use super::LoginScene;

#[derive(PartialEq, Clone, Copy)]
pub(super) enum NewAccountFocus {
    AccountId,
    Password1,
    Password2,
    UserName,
    BirthDate,
    Question,
    Answer,
    Email,
}

impl LoginScene {
    pub(super) fn open_new_account_dialog(&mut self) {
        self.show_new_account = true;
        self.na_in_flight = false;

        self.na_account_id.clear();
        self.na_password1.clear();
        self.na_password2.clear();
        self.na_user_name.clear();
        self.na_birth_date.clear();
        self.na_question.clear();
        self.na_answer.clear();
        self.na_email.clear();
        self.na_focus = NewAccountFocus::AccountId;
    }

    pub(super) fn close_new_account_dialog(&mut self) {
        self.show_new_account = false;
        self.na_in_flight = false;
    }

    fn parse_birth_date_to_dotnet_binary(text: &str) -> Option<i64> {
        let s = text.trim();
        if s.is_empty() {
            return Some(0);
        }

        let formats = ["%Y-%m-%d", "%Y/%m/%d", "%m/%d/%Y", "%d/%m/%Y"];
        let mut date = None;
        for fmt in formats {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
                date = Some(d);
                break;
            }
        }
        let date = date?;

        let dt = Utc
            .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
            .single()?;

        // .NET DateTime ticks at Unix epoch
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = dt.timestamp() * 10_000_000 + unix_epoch_ticks;
        Some(ticks)
    }

    fn can_submit_new_account(&self) -> bool {
        if self.na_in_flight {
            return false;
        }
        if self.na_account_id.is_empty() || self.na_password1.is_empty() {
            return false;
        }
        if self.na_password1 != self.na_password2 {
            return false;
        }
        if self.na_account_id.len() > 20 || self.na_password1.len() > 20 {
            return false;
        }
        if self.na_user_name.len() > 20 {
            return false;
        }
        if self.na_birth_date.len() > 10 {
            return false;
        }
        if self.na_question.len() > 30 || self.na_answer.len() > 30 {
            return false;
        }
        if self.na_email.len() > 50 {
            return false;
        }
        if !self.na_birth_date.trim().is_empty() {
            if Self::parse_birth_date_to_dotnet_binary(&self.na_birth_date).is_none() {
                return false;
            }
        }

        true
    }

    fn submit_new_account(&mut self) {
        if !self.can_submit_new_account() {
            self.message_text = "请填写完整信息（两次密码需一致，生日格式需正确）".to_string();
            self.show_message = true;
            return;
        }

        let birth_date_binary = match Self::parse_birth_date_to_dotnet_binary(&self.na_birth_date) {
            Some(v) => v,
            None => {
                self.message_text = "生日格式不正确（建议 YYYY-MM-DD）".to_string();
                self.show_message = true;
                return;
            }
        };

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

        self.na_in_flight = true;
        if let Err(e) = net.send(NetworkEvent::NewAccountRequest {
            account_id: self.na_account_id.clone(),
            password: self.na_password1.clone(),
            birth_date: birth_date_binary,
            username: self.na_user_name.clone(),
            secret_question: self.na_question.clone(),
            secret_answer: self.na_answer.clone(),
            email: self.na_email.clone(),
        }) {
            self.na_in_flight = false;
            self.message_text = format!("发送 NewAccountRequest 失败: {e}");
            self.show_message = true;
        } else {
            self.message_text = "正在创建账号...".to_string();
            self.show_message = true;
        }
    }

    pub(super) fn handle_new_account_text_input(&mut self) {
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii() && !ch.is_control() {
                match self.na_focus {
                    NewAccountFocus::AccountId => {
                        if self.na_account_id.len() < 20 {
                            self.na_account_id.push(ch);
                        }
                    }
                    NewAccountFocus::Password1 => {
                        if self.na_password1.len() < 20 {
                            self.na_password1.push(ch);
                        }
                    }
                    NewAccountFocus::Password2 => {
                        if self.na_password2.len() < 20 {
                            self.na_password2.push(ch);
                        }
                    }
                    NewAccountFocus::UserName => {
                        if self.na_user_name.len() < 20 {
                            self.na_user_name.push(ch);
                        }
                    }
                    NewAccountFocus::BirthDate => {
                        if self.na_birth_date.len() < 10 {
                            self.na_birth_date.push(ch);
                        }
                    }
                    NewAccountFocus::Question => {
                        if self.na_question.len() < 30 {
                            self.na_question.push(ch);
                        }
                    }
                    NewAccountFocus::Answer => {
                        if self.na_answer.len() < 30 {
                            self.na_answer.push(ch);
                        }
                    }
                    NewAccountFocus::Email => {
                        if self.na_email.len() < 50 {
                            self.na_email.push(ch);
                        }
                    }
                }
            }
        }

        if is_key_pressed(KeyCode::Backspace) {
            match self.na_focus {
                NewAccountFocus::AccountId => {
                    self.na_account_id.pop();
                }
                NewAccountFocus::Password1 => {
                    self.na_password1.pop();
                }
                NewAccountFocus::Password2 => {
                    self.na_password2.pop();
                }
                NewAccountFocus::UserName => {
                    self.na_user_name.pop();
                }
                NewAccountFocus::BirthDate => {
                    self.na_birth_date.pop();
                }
                NewAccountFocus::Question => {
                    self.na_question.pop();
                }
                NewAccountFocus::Answer => {
                    self.na_answer.pop();
                }
                NewAccountFocus::Email => {
                    self.na_email.pop();
                }
            }
        }

        if is_key_pressed(KeyCode::Tab) {
            self.na_focus = match self.na_focus {
                NewAccountFocus::AccountId => NewAccountFocus::Password1,
                NewAccountFocus::Password1 => NewAccountFocus::Password2,
                NewAccountFocus::Password2 => NewAccountFocus::UserName,
                NewAccountFocus::UserName => NewAccountFocus::BirthDate,
                NewAccountFocus::BirthDate => NewAccountFocus::Question,
                NewAccountFocus::Question => NewAccountFocus::Answer,
                NewAccountFocus::Answer => NewAccountFocus::Email,
                NewAccountFocus::Email => NewAccountFocus::AccountId,
            };
        }

        if is_key_pressed(KeyCode::Enter) {
            if self.can_submit_new_account() {
                self.submit_new_account();
            }
        }
    }

    pub(super) fn draw_new_account_dialog(&mut self) {
        // 背景 Prguse[63]
        let (dialog_w, dialog_h) = if let Some(info) = LibraryName::Prguse.get_texture(63) {
            (info.width as f32, info.height as f32)
        } else {
            (600.0, 470.0)
        };

        let dialog = begin_modal(dialog_w, dialog_h, 120);
        let dialog_x = dialog.x;
        let dialog_y = dialog.y;

        if let Some(info) = LibraryName::Prguse.get_texture(63) {
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

        // 输入框：对齐原版坐标（相对弹窗）
        let input_h = 18.0;
        let x_main = dialog_x + 226.0;
        let w_main = 136.0;
        let w_wide = 190.0;

        let y_account = dialog_y + 103.0;
        let y_p1 = dialog_y + 129.0;
        let y_p2 = dialog_y + 155.0;
        let y_name = dialog_y + 189.0;
        let y_birth = dialog_y + 215.0;
        let y_q = dialog_y + 250.0;
        let y_a = dialog_y + 276.0;
        let y_email = dialog_y + 311.0;

        draw_input_box(
            x_main,
            y_account,
            w_main,
            input_h,
            &self.na_account_id,
            false,
            self.na_focus == NewAccountFocus::AccountId,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_p1,
            w_main,
            input_h,
            &self.na_password1,
            true,
            self.na_focus == NewAccountFocus::Password1,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_p2,
            w_main,
            input_h,
            &self.na_password2,
            true,
            self.na_focus == NewAccountFocus::Password2,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_name,
            w_main,
            input_h,
            &self.na_user_name,
            false,
            self.na_focus == NewAccountFocus::UserName,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_birth,
            w_main,
            input_h,
            &self.na_birth_date,
            false,
            self.na_focus == NewAccountFocus::BirthDate,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_q,
            w_wide,
            input_h,
            &self.na_question,
            false,
            self.na_focus == NewAccountFocus::Question,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_a,
            w_wide,
            input_h,
            &self.na_answer,
            false,
            self.na_focus == NewAccountFocus::Answer,
            self.cursor_visible,
            14.0,
        );
        draw_input_box(
            x_main,
            y_email,
            w_main,
            input_h,
            &self.na_email,
            false,
            self.na_focus == NewAccountFocus::Email,
            self.cursor_visible,
            14.0,
        );

        // Description 区
        let desc_x = dialog_x + 15.0;
        let desc_y = dialog_y + 340.0;
        let desc_w = 300.0;
        let desc_h = 70.0;
        draw_rectangle(desc_x, desc_y, desc_w, desc_h, Color::from_rgba(20, 20, 25, 160));
        draw_rectangle_lines(
            desc_x,
            desc_y,
            desc_w,
            desc_h,
            1.0,
            Color::from_rgba(100, 100, 120, 255),
        );

        let desc_text = match self.na_focus {
            NewAccountFocus::AccountId => {
                " Description: Account ID.\n Accepted characters: A-Z a-z 0-9.\n Length: 1~20."
            }
            NewAccountFocus::Password1 | NewAccountFocus::Password2 => {
                " Description: Password.\n Accepted characters: ASCII.\n Length: 1~20."
            }
            NewAccountFocus::UserName => " Description: User Name.\n Length: 0~20. Optional.",
            NewAccountFocus::BirthDate => {
                " Description: Birth Date.\n Format: YYYY-MM-DD.\n Length: <=10. Optional."
            }
            NewAccountFocus::Question => " Description: Secret Question.\n Length: 0~30. Optional.",
            NewAccountFocus::Answer => " Description: Secret Answer.\n Length: 0~30. Optional.",
            NewAccountFocus::Email => " Description: E-Mail.\n Length: 0~50. Optional.",
        };
        draw_multiline_text_cn(desc_text, desc_x + 6.0, desc_y + 18.0, 12.0, LIGHTGRAY);

        // 按钮：OK Title[200-202] at (135,425) / Cancel Title[203-205] at (409,425)
        let ok_enabled = self.can_submit_new_account();
        if draw_button(
            LibraryName::Title,
            dialog_x + 135.0,
            dialog_y + 425.0,
            200,
            201,
            202,
            ok_enabled,
        ) {
            self.submit_new_account();
        }

        if draw_button(
            LibraryName::Title,
            dialog_x + 409.0,
            dialog_y + 425.0,
            203,
            204,
            205,
            true,
        ) {
            self.close_new_account_dialog();
        }

        // 鼠标点输入框切换焦点
        let (mx, my) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) {
            // 主宽度输入框
            if mx >= x_main && mx <= x_main + w_wide {
                if my >= y_account && my <= y_account + input_h {
                    self.na_focus = NewAccountFocus::AccountId;
                } else if my >= y_p1 && my <= y_p1 + input_h {
                    self.na_focus = NewAccountFocus::Password1;
                } else if my >= y_p2 && my <= y_p2 + input_h {
                    self.na_focus = NewAccountFocus::Password2;
                } else if my >= y_name && my <= y_name + input_h {
                    self.na_focus = NewAccountFocus::UserName;
                } else if my >= y_birth && my <= y_birth + input_h {
                    self.na_focus = NewAccountFocus::BirthDate;
                } else if my >= y_q && my <= y_q + input_h && mx <= x_main + w_wide {
                    self.na_focus = NewAccountFocus::Question;
                } else if my >= y_a && my <= y_a + input_h && mx <= x_main + w_wide {
                    self.na_focus = NewAccountFocus::Answer;
                } else if my >= y_email && my <= y_email + input_h && mx <= x_main + w_main {
                    self.na_focus = NewAccountFocus::Email;
                }
            }
        }
    }
}
