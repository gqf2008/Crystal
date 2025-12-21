use macroquad::prelude::*;

use crate::network::NetworkEvent;
use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use crate::ui::additive::with_additive_blend;
use crate::ui::widgets::{begin_modal, draw_button};

use super::{CharacterInfo, SelectScene};

impl SelectScene {
    /// 绘制新建角色对话框
    pub(super) fn draw_new_character_dialog(&mut self) {
        // 原工程：NewCharacterDialog 背景为 Prguse[73]，控件布局为固定像素。
        let (dialog_w, dialog_h) = if let Some(info) = LibraryName::Prguse.get_texture(73) {
            (info.width as f32, info.height as f32)
        } else {
            // 经验值 fallback（避免资源缺失时 UI 完全不可用）
            (600.0, 470.0)
        };
        let dialog = begin_modal(dialog_w, dialog_h, 200);
        let dialog_x = dialog.x;
        let dialog_y = dialog.y;

        // 背景纹理（缺失则退化为纯色遮罩+提示）
        if let Some(info) = LibraryName::Prguse.get_texture(73) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + info.offset_x as f32, dialog_y + info.offset_y as f32, WHITE);
            }
        }

        // 标题：Title[20] at (206, 11)
        if let Some(info) = LibraryName::Title.get_texture(20) {
            if let Some(ref tex) = info.image {
                draw_texture(
                    tex,
                    dialog_x + 206.0 + info.offset_x as f32,
                    dialog_y + 11.0 + info.offset_y as f32,
                    WHITE,
                );
            }
        }

        // 角色预览（ChrSel 动画） at (120, 250)
        let (male_base, female_base) = match self.new_char_class {
            0 => (20, 300),  // Warrior
            1 => (40, 320),  // Wizard
            2 => (60, 340),  // Taoist
            3 => (80, 360),  // Assassin
            4 => (100, 140), // Archer
            _ => (20, 300),
        };
        let base = if self.new_char_gender == 0 { male_base } else { female_base };
        let frame_index = base as usize + self.animation_frame;
        if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
            if let Some(ref tex) = info.image {
                draw_texture(
                    tex,
                    dialog_x + 120.0 + info.offset_x as f32,
                    dialog_y + 250.0 + info.offset_y as f32,
                    WHITE,
                );
            }
        }

        // 原工程：法师额外叠加手上特效 (Index + 560) 使用 DrawBlend (SourceAlpha + One)
        if self.new_char_class == 1 {
            let overlay_index = frame_index + 560;
            if let Some(info) = LibraryName::ChrSel.get_texture(overlay_index) {
                if let Some(ref tex) = info.image {
                    with_additive_blend(|| {
                        draw_texture(
                            tex,
                            dialog_x + 120.0 + info.offset_x as f32,
                            dialog_y + 250.0 + info.offset_y as f32,
                            WHITE,
                        );
                    });
                }
            }
        }

        // 名称输入：原工程 TextBox 在 (325,268) 大小 240x20；这里不盖掉底图，仅绘制文字+光标。
        let name_x = dialog_x + 325.0;
        let name_y_baseline = dialog_y + 268.0 + 15.0;
        let name_font = 14.0;
        draw_text_cn(&self.new_char_name, name_x + 2.0, name_y_baseline, name_font, WHITE);
        if self.cursor_visible {
            let w = measure_text_cn(&self.new_char_name, name_font).width;
            let cx = name_x + 2.0 + w;
            draw_line(cx, dialog_y + 268.0 + 3.0, cx, dialog_y + 268.0 + 20.0 - 3.0, 1.0, WHITE);
        }

        // 职业按钮（Prguse）
        let class_btns: [(u8, f32, f32, usize); 5] = [
            (0, 323.0, 296.0, 2426),
            (1, 373.0, 296.0, 2429),
            (2, 423.0, 296.0, 2432),
            (3, 473.0, 296.0, 2435),
            (4, 523.0, 296.0, 2438),
        ];
        for (class, x, y, base_idx) in class_btns {
            let selected = self.new_char_class == class;
            let normal = base_idx + if selected { 1 } else { 0 };
            let hover = base_idx + 1;
            let pressed = base_idx + 2;
            if draw_button(
                LibraryName::Prguse,
                dialog_x + x,
                dialog_y + y,
                normal,
                hover,
                pressed,
                true,
            ) {
                self.new_char_class = class;
            }
        }

        // 性别按钮（Prguse）
        let male_selected = self.new_char_gender == 0;
        if draw_button(
            LibraryName::Prguse,
            dialog_x + 323.0,
            dialog_y + 343.0,
            2420 + if male_selected { 1 } else { 0 },
            2421,
            2422,
            true,
        ) {
            self.new_char_gender = 0;
        }
        let female_selected = self.new_char_gender == 1;
        if draw_button(
            LibraryName::Prguse,
            dialog_x + 373.0,
            dialog_y + 343.0,
            2423 + if female_selected { 1 } else { 0 },
            2424,
            2425,
            true,
        ) {
            self.new_char_gender = 1;
        }

        // OK/Cancel（Title）：OK 360/361/362 at (160,425)；Cancel 280/281/282 at (425,425)
        let name_len = self.new_char_name.trim().chars().count();
        let ok_enabled = (2..=16).contains(&name_len) && !self.character_op_in_flight;
        if draw_button(
            LibraryName::Title,
            dialog_x + 160.0,
            dialog_y + 425.0,
            360,
            361,
            362,
            ok_enabled,
        ) {
            self.on_create_character();
        }
        if draw_button(
            LibraryName::Title,
            dialog_x + 425.0,
            dialog_y + 425.0,
            280,
            281,
            282,
            true,
        ) {
            self.show_new_character = false;
        }
    }

    /// 处理创建角色
    pub(super) fn on_create_character(&mut self) {
        if self.new_char_name.trim().is_empty() {
            self.show_message("请输入角色名称！");
            return;
        }

        if self.new_char_name.chars().count() < 2 {
            self.show_message("角色名称至少需2个字符！");
            return;
        }

        if self.new_char_name.chars().count() > 16 {
            self.show_message("角色名称最多16个字符！");
            return;
        }

        self.ensure_network();

        if let Some(net) = self.net.as_ref() {
            if self.character_op_in_flight {
                self.show_message("正在处理上一个请求，请稍候...");
                return;
            }
            let name = self.new_char_name.trim().to_string();
            if let Err(e) = net.send(NetworkEvent::NewCharacterRequest {
                name: name.clone(),
                class: self.new_char_class,
                gender: self.new_char_gender,
            }) {
                self.show_message(&format!("发送 NewCharacterRequest 失败: {e}"));
                return;
            }
            self.character_op_in_flight = true;
            // 对齐 C#：发送后禁用 OK，等待服务器回包；不额外弹“正在创建”提示框。
        } else {
            // 离线回退（不推荐，仅用于无网络时演示 UI）
            let new_char = CharacterInfo {
                index: self.characters.len() as i32,
                name: self.new_char_name.clone(),
                level: 1,
                class: self.new_char_class,
                gender: self.new_char_gender,
                last_access: "刚刚".to_string(),
            };
            self.characters.push(new_char);
            println!("🎭 (offline) 创建角色成功: {}", self.new_char_name);

            self.show_new_character = false;
            self.show_message("Your character was created successfully.");
        }
    }
}
