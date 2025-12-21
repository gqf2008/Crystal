use macroquad::prelude::*;

use crate::network::NetworkEvent;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use crate::ui::widgets::{begin_modal, draw_button};

use super::SelectScene;

impl SelectScene {
    /// 绘制删除角色对话框
    pub(super) fn draw_delete_character_dialog(&mut self) {
        // 原工程：SelectScene 删除确认使用 MirMessageBox（Prguse[360] 背景，Title Yes/No 按钮）。
        let (dialog_w, dialog_h) = if let Some(info) = LibraryName::Prguse.get_texture(360) {
            (info.width as f32, info.height as f32)
        } else {
            (460.0, 210.0)
        };
        let dialog = begin_modal(dialog_w, dialog_h, 200);
        let dialog_x = dialog.x;
        let dialog_y = dialog.y;

        // 背景纹理 Prguse[360]
        if let Some(info) = LibraryName::Prguse.get_texture(360) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + info.offset_x as f32, dialog_y + info.offset_y as f32, WHITE);
            }
        }

        // 文本区域：C# Label at (35,35) size (390,110)
        draw_text_cn(
            &format!("确定要删除角色 {} 吗?", self.delete_char_name),
            dialog_x + 35.0,
            dialog_y + 60.0,
            14.0,
            WHITE,
        );

        // Yes/No 按钮：Title Yes(206/207/208) at (260,157)；No(210/211/212) at (360,157)
        if draw_button(
            LibraryName::Title,
            dialog_x + 260.0,
            dialog_y + 157.0,
            206,
            207,
            208,
            true,
        ) {
            self.on_delete_character();
        }
        if draw_button(
            LibraryName::Title,
            dialog_x + 360.0,
            dialog_y + 157.0,
            210,
            211,
            212,
            true,
        ) {
            self.show_delete_character = false;
        }
    }

    /// 处理删除角色
    pub(super) fn on_delete_character(&mut self) {
        self.ensure_network();

        if let Some(net) = self.net.as_ref() {
            if self.character_op_in_flight {
                self.show_message("正在处理上一个请求，请稍候...");
                return;
            }
            if let Err(e) = net.send(NetworkEvent::DeleteCharacterRequest {
                index: self.delete_char_index,
            }) {
                self.show_message(&format!("发送 DeleteCharacterRequest 失败: {e}"));
                return;
            }
            self.character_op_in_flight = true;
            self.show_message(&format!("正在删除角色: {}", self.delete_char_name));
        } else {
            // 离线回退
            if let Some(pos) = self
                .characters
                .iter()
                .position(|c| c.index == self.delete_char_index)
            {
                let name = self.characters[pos].name.clone();
                self.characters.remove(pos);
                println!("✅ (offline) 角色已删除: {}", name);

                if self.characters.is_empty() {
                    self.selected_index = None;
                } else if let Some(idx) = self.selected_index {
                    if idx >= self.characters.len() {
                        self.selected_index = Some(self.characters.len() - 1);
                    }
                }
            }
        }

        self.show_delete_character = false;
    }
}
