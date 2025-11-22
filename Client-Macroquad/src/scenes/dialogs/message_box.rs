// 公共消息框组件
use macroquad::prelude::*;
use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::Dialog;

/// 消息框按钮类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    OkCancel,
    YesNo,
}

/// 消息框结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    Cancel,
    Yes,
    No,
}

/// 消息框组件
pub struct MessageBox {
    pub title: String,
    pub text: String,
    pub buttons: MessageBoxButtons,
    id: String,
    bg_size: egui::Vec2,
    position: Option<egui::Pos2>,
    result:Option<MessageBoxResult>
}

impl MessageBox {
    /// 创建新消息框(带自定义 ID)
    pub fn new_with_id(title: &str, text: &str, buttons: MessageBoxButtons, id: &str) -> Self {
        let bg_size = if let Some(info) = LibraryName::Prguse.get_texture(360) {
            egui::vec2(info.width as f32, info.height as f32)
        } else {
            egui::vec2(460.0, 200.0)
        };

        
        
        Self {
            title: title.to_string(),
            text: text.to_string(),
            buttons,
            id: format!("message_box_{}", id),
            bg_size,
            position: None,
            result: None,
        }
    }
    
    /// 创建新消息框(使用时间戳生成唯一 ID)
    pub fn new(title: &str, text: &str, buttons: MessageBoxButtons) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let id = format!("{:x}", timestamp);
        Self::new_with_id(title, text, buttons, &id)
    }
    
    /// 设置窗口位置
    pub fn with_position(mut self, pos: egui::Pos2) -> Self {
        self.position = Some(pos);
        self
    }
    
    pub fn result(&self) -> Option<MessageBoxResult> {
        self.result
    }
    
    /// 处理结果并触发回调
    fn handle_result(&mut self, result: MessageBoxResult, open: &mut bool) {
        self.result = Some(result);
        // 关闭窗口
        *open = false;
    }
}

impl Dialog for MessageBox {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        self.result=None;
        // ESC键关闭
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.handle_result(MessageBoxResult::Cancel, open);
            return;
        }

        // 计算居中位置
        let default_pos = egui::pos2(
            (screen_width() / screen_dpi_scale() - self.bg_size.x) / 2.0,
            (screen_height() / screen_dpi_scale() - self.bg_size.y) / 2.0
        );

        // 使用 Area 来实现可拖动的自定义窗口
        egui::Area::new(egui::Id::new(&self.id))
            .movable(true)
            .interactable(true)
            .default_pos(self.position.unwrap_or(default_pos))
            .show(ctx, |ui| {
            // 分配固定大小的区域
            let (rect, _response) = ui.allocate_exact_size(self.bg_size, egui::Sense::hover());
            
            // === 第1层：背景层（静态） ===
            // 在窗口区域绘制背景纹理
            if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) {
                if let Some(ref handle) = info.egui_texture {
                    ui.painter().image(
                        handle.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            
            // === 第2层：交互层（动态，使用 egui 组件 + 绝对定位） ===
            let base_pos = rect.min;
            
            // 标题 - 使用 put 在指定位置放置 Label
            let title_rect = egui::Rect::from_min_size(
                base_pos + egui::vec2(20.0, 15.0),
                egui::vec2(420.0, 25.0)
            );
            ui.put(title_rect, egui::Label::new(
                egui::RichText::new(&self.title)
                    .size(18.0)
                    .color(egui::Color32::WHITE)
            ));
            
            // 文本内容 - 使用 put 在指定位置放置 Label
            let text_rect = egui::Rect::from_min_size(
                base_pos + egui::vec2(20.0, 50.0),
                egui::vec2(420.0, 90.0)
            );
            ui.put(text_rect, egui::Label::new(
                egui::RichText::new(&self.text)
                    .size(14.0)
                    .color(egui::Color32::WHITE)
            ));
            
            // 按钮 - Y坐标固定在140，根据交互状态显示不同纹理
            let button_y = 158.;
            match self.buttons {
                MessageBoxButtons::Ok => {
                    // 单个OK按钮居中 - X位置190
                    // OK按钮纹理: 200(正常), 201(悬停), 202(按下)
                    if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, 200) {
                        if let Some(ref normal_tex) = tex_info.egui_texture {
                            let button_rect = egui::Rect::from_min_size(
                                base_pos + egui::vec2(190.0, button_y),
                                normal_tex.size_vec2()
                            );
                            
                            let response = ui.allocate_rect(button_rect, egui::Sense::click());
                            
                            // 根据交互状态选择纹理
                            let tex_index = if response.is_pointer_button_down_on() {
                                202 // 按下
                            } else if response.hovered() {
                                201 // 悬停
                            } else {
                                200 // 正常
                            };
                            
                            if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, tex_index) {
                                if let Some(ref tex) = tex_info.egui_texture {
                                    ui.painter().image(
                                        tex.id(),
                                        button_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }
                            
                            if response.clicked() {
                                self.handle_result(MessageBoxResult::Ok, open);
                            }
                        }
                    }
                }
                MessageBoxButtons::OkCancel => {
                    // OK按钮 - X位置140
                    // OK按钮纹理: 200(正常), 201(悬停), 202(按下)
                    if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, 200) {
                        if let Some(ref normal_tex) = tex_info.egui_texture {
                            let ok_rect = egui::Rect::from_min_size(
                                base_pos + egui::vec2(140.0, button_y),
                                normal_tex.size_vec2()
                            );
                            
                            let ok_response = ui.allocate_rect(ok_rect, egui::Sense::click());
                            
                            let ok_tex_index = if ok_response.is_pointer_button_down_on() {
                                202
                            } else if ok_response.hovered() {
                                201
                            } else {
                                200
                            };
                            
                            if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, ok_tex_index) {
                                if let Some(ref tex) = tex_info.egui_texture {
                                    ui.painter().image(
                                        tex.id(),
                                        ok_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }
                            
                            if ok_response.clicked() {
                                self.handle_result(MessageBoxResult::Ok, open);
                            }
                        }
                    }
                    
                    // Cancel按钮 - X位置230
                    // Cancel按钮纹理: 203(正常), 204(悬停), 205(按下)
                    if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, 203) {
                        if let Some(ref normal_tex) = tex_info.egui_texture {
                            let cancel_rect = egui::Rect::from_min_size(
                                base_pos + egui::vec2(230.0, button_y),
                                normal_tex.size_vec2()
                            );
                            
                            let cancel_response = ui.allocate_rect(cancel_rect, egui::Sense::click());
                            
                            let cancel_tex_index = if cancel_response.is_pointer_button_down_on() {
                                205
                            } else if cancel_response.hovered() {
                                204
                            } else {
                                203
                            };
                            
                            if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, cancel_tex_index) {
                                if let Some(ref tex) = tex_info.egui_texture {
                                    ui.painter().image(
                                        tex.id(),
                                        cancel_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }
                            
                            if cancel_response.clicked() {
                                self.handle_result(MessageBoxResult::Cancel, open);
                            }
                        }
                    }
                }
                MessageBoxButtons::YesNo => {
                    // Yes按钮 - X位置140
                    // Yes按钮纹理: 206(正常), 207(悬停), 208(按下)
                    if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, 206) {
                        if let Some(ref normal_tex) = tex_info.egui_texture {
                            let yes_rect = egui::Rect::from_min_size(
                                base_pos + egui::vec2(140.0, button_y),
                                normal_tex.size_vec2()
                            );
                            
                            let yes_response = ui.allocate_rect(yes_rect, egui::Sense::click());
                            
                            let yes_tex_index = if yes_response.is_pointer_button_down_on() {
                                208
                            } else if yes_response.hovered() {
                                207
                            } else {
                                206
                            };
                            
                            if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, yes_tex_index) {
                                if let Some(ref tex) = tex_info.egui_texture {
                                    ui.painter().image(
                                        tex.id(),
                                        yes_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }
                            
                            if yes_response.clicked() {
                                self.handle_result(MessageBoxResult::Yes, open);
                            }
                        }
                    }
                    
                    // No按钮 - X位置230
                    // No按钮纹理: 210(正常), 211(悬停), 212(按下)
                    if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, 210) {
                        if let Some(ref normal_tex) = tex_info.egui_texture {
                            let no_rect = egui::Rect::from_min_size(
                                base_pos + egui::vec2(230.0, button_y),
                                normal_tex.size_vec2()
                            );
                            
                            let no_response = ui.allocate_rect(no_rect, egui::Sense::click());
                            
                            let no_tex_index = if no_response.is_pointer_button_down_on() {
                                212
                            } else if no_response.hovered() {
                                211
                            } else {
                                210
                            };
                            
                            if let Some(tex_info) = LibraryName::Title.get_egui_texture(ctx, no_tex_index) {
                                if let Some(ref tex) = tex_info.egui_texture {
                                    ui.painter().image(
                                        tex.id(),
                                        no_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }
                            
                            if no_response.clicked() {
                                self.handle_result(MessageBoxResult::No, open);
                            }
                        }
                    }
                }
            }
        });
    }
}
