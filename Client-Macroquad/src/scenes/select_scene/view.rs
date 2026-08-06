use macroquad::prelude::*;

use crate::game::GameResult;
use crate::resources::LibraryName;
use crate::ui::additive::with_additive_blend;
use crate::ui::text_renderer::draw_text_cn;
use crate::ui::widgets::{draw_button, draw_mir_message_box_ok};

use super::SelectScene;

impl SelectScene {
    /// 绘制背景
    pub(super) fn draw_background(&self) {
        // 背景 Prguse[65]
        if let Some(info) = LibraryName::Prguse.get_texture(65) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, info.offset_x as f32, info.offset_y as f32, WHITE);
            }
        }

        // 标题 Title[40] at (468, 20)
        if let Some(info) = LibraryName::Title.get_texture(40) {
            if let Some(ref texture) = info.image {
                draw_texture(
                    texture,
                    468.0 + info.offset_x as f32,
                    20.0 + info.offset_y as f32,
                    WHITE,
                );
            }
        }

        // 服务器名称 at (432, 60)
        draw_text_cn("Legend of Mir 2", 460.0, 77.0, 17.0, WHITE);
    }

    /// 绘制角色预览动画
    pub(super) fn draw_character_preview(&self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];

                // 计算角色预览帧索引
                let base_index = if character.class == 4 {
                    if character.gender == 0 {
                        100
                    } else {
                        140
                    }
                } else {
                    20 + (character.class as usize * 20) + (character.gender as usize * 280)
                };
                let frame_index = base_index + self.animation_frame;

                if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
                    if let Some(ref texture) = info.image {
                        let scale = 1.2;
                        let x = 260.0 + info.offset_x as f32 * scale;
                        let y = 420.0 + info.offset_y as f32 * scale;
                        let w = texture.width() * scale;
                        let h = texture.height() * scale;
                        draw_texture_ex(
                            texture,
                            x,
                            y,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(Vec2::new(w, h)),
                                ..Default::default()
                            },
                        );
                    }
                }

                // 原工程：法师额外叠加手上特效 (Index + 560) 使用 DrawBlend (SourceAlpha + One)
                if character.class == 1 {
                    if let Some(info) = LibraryName::ChrSel.get_texture(frame_index + 560) {
                        if let Some(ref texture) = info.image {
                            let scale = 1.2;
                            let x = 260.0 + info.offset_x as f32 * scale;
                            let y = 420.0 + info.offset_y as f32 * scale;
                            let w = texture.width() * scale;
                            let h = texture.height() * scale;
                            with_additive_blend(|| {
                                draw_texture_ex(
                                    texture,
                                    x,
                                    y,
                                    WHITE,
                                    DrawTextureParams {
                                        dest_size: Some(Vec2::new(w, h)),
                                        ..Default::default()
                                    },
                                );
                            });
                        }
                    }
                }
            }
        }
    }

    /// 绘制角色信息
    pub(super) fn draw_character_info(&self) {
        if let Some(selected_idx) = self.selected_index {
            if selected_idx < self.characters.len() {
                let character = &self.characters[selected_idx];
                draw_text_cn("Last Online:", 200.0, 623.0, 14.0, WHITE);
                draw_text_cn(&character.last_access, 280.0, 623.0, 14.0, WHITE);
            }
        }
    }

    /// 绘制角色按钮
    pub(super) fn draw_character_buttons(&mut self) {
        let positions = [
            (637.0, 194.0),
            (637.0, 298.0),
            (637.0, 402.0),
            (637.0, 506.0),
        ];
        let (mx, my) = mouse_position();

        for (i, &(x, y)) in positions.iter().enumerate() {
            let has_character = i < self.characters.len();
            let is_selected = self.selected_index == Some(i);

            // 按钮尺寸
            let btn_w = 280.0;
            let btn_h = 90.0;
            let is_hovered = mx >= x && mx <= x + btn_w && my >= y && my <= y + btn_h;

            if has_character {
                let character = &self.characters[i];

                // 获取职业对应纹理索引
                let base_index = match character.class {
                    0 => 660,
                    1 => 661,
                    2 => 662,
                    3 => 663,
                    4 => 664,
                    _ => 660,
                };
                let texture_index = if is_selected {
                    base_index + 5
                } else {
                    base_index
                };

                if let Some(info) = LibraryName::Title.get_texture(texture_index) {
                    if let Some(ref texture) = info.image {
                        draw_texture(texture, x, y, WHITE);
                    }
                }

                // 绘制角色信息文字
                draw_text_cn(&character.name, x + 107.0, y + 18.0, 13.0, WHITE);
                draw_text_cn(
                    &format!("Lv.{}", character.level),
                    x + 107.0,
                    y + 37.0,
                    11.0,
                    LIGHTGRAY,
                );

                let class_name = match character.class {
                    0 => "战士",
                    1 => "法师",
                    2 => "道士",
                    3 => "刺客",
                    4 => "弓手",
                    _ => "未知",
                };
                draw_text_cn(class_name, x + 178.0, y + 37.0, 11.0, LIGHTGRAY);

                // 检测点击
                if !self.show_new_character
                    && !self.show_delete_character
                    && !self.show_message_box
                    && !self.credits_dialog.is_visible()
                    && is_hovered
                    && is_mouse_button_pressed(MouseButton::Left)
                {
                    self.selected_index = Some(i);
                }
            } else {
                // 空槽位 - Prguse[44]
                if let Some(info) = LibraryName::Prguse.get_texture(44) {
                    if let Some(ref texture) = info.image {
                        draw_texture(texture, x, y, WHITE);
                    }
                }

                // 检测点击空槽位
                if !self.show_new_character
                    && !self.show_delete_character
                    && !self.show_message_box
                    && !self.credits_dialog.is_visible()
                    && is_hovered
                    && is_mouse_button_pressed(MouseButton::Left)
                    && self.characters.len() < 4
                {
                    self.show_new_character = true;
                }
            }
        }
    }

    /// 绘制底部按钮
    pub(super) fn draw_bottom_buttons(&mut self) {
        let screen_w = screen_width();
        let x_point = (screen_w - 200.0) / 5.0;
        let y = screen_height() - 32.0;

        let any_dialog = self.show_new_character
            || self.show_delete_character
            || self.show_message_box
            || self.credits_dialog.is_visible();

        // 开始游戏 Title[340-342]
        if self.selected_index.is_some()
            && draw_button(
                LibraryName::Title,
                100.0 + x_point - x_point / 2.0 - 50.0,
                y,
                340,
                341,
                342,
                !any_dialog,
            )
        {
            self.request_start_game();
        }

        // 新建角色 Title[343-345]
        if draw_button(
            LibraryName::Title,
            100.0 + x_point * 2.0 - x_point / 2.0 - 50.0,
            y,
            343,
            344,
            345,
            !any_dialog,
        ) {
            if self.characters.len() < 4 {
                self.show_new_character = true;
                self.new_char_name.clear();
            } else {
                self.show_message("最多只能创建4个角色！");
            }
        }

        // 删除角色 Title[346-348]
        if draw_button(
            LibraryName::Title,
            100.0 + x_point * 3.0 - x_point / 2.0 - 50.0,
            y,
            346,
            347,
            348,
            !any_dialog,
        ) {
            if let Some(idx) = self.selected_index {
                if idx < self.characters.len() {
                    let character = &self.characters[idx];
                    self.delete_char_name = character.name.clone();
                    self.delete_char_index = character.index;
                    self.delete_confirm_input.clear();
                    self.show_delete_character = true;
                }
            }
        }

        // Credits Title[349-351]
        if draw_button(
            LibraryName::Title,
            100.0 + x_point * 4.0 - x_point / 2.0 - 50.0,
            y,
            349,
            350,
            351,
            !any_dialog,
        ) {
            // 对齐原版 C#：CreditsButton.Click 为空
        }

        // 退出 Title[352-354]
        if draw_button(
            LibraryName::Title,
            100.0 + x_point * 5.0 - x_point / 2.0 - 50.0,
            y,
            352,
            353,
            354,
            !any_dialog,
        ) {
            std::process::exit(0);
        }
    }

    pub(super) fn render_scene(&mut self) -> GameResult {
        clear_background(BLACK);

        // 绘制背景和角色
        self.draw_background();
        self.draw_character_preview();
        self.draw_character_info();

        // 绘制UI
        self.draw_character_buttons();
        self.draw_bottom_buttons();

        // 绘制对话框
        if self.show_new_character {
            self.draw_new_character_dialog();
        }

        if self.show_delete_character {
            self.draw_delete_character_dialog();
        }

        if self.show_message_box && draw_mir_message_box_ok(&self.message_text) {
            self.show_message_box = false;
        }

        // Credits 最上层
        if self.credits_dialog.is_visible() {
            self.credits_dialog.draw();
        }

        Ok(())
    }
}
