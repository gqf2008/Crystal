// ============================================================================
// ChatDialog - 聊天窗口
// ============================================================================
// 
// 【功能说明】
// 1. 显示聊天消息列表
// 2. 聊天输入框（按 Enter 显示/发送，ESC 取消）
// 3. 滚动条控制
// 4. 支持不同聊天频道（全体、组队、公会等）
// 
// 【位置】
// - 原工程：MainDialog.X + 230, ScreenHeight - 97
// - 尺寸：根据分辨率 (800: Prguse[2201], 1024+: Prguse[2221])
// 
// ============================================================================

use egui_macroquad::egui;
use crate::resources::LibraryName;
use crate::scenes::dialogs::Dialog;

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub text: String,
    pub color: egui::Color32,
    pub timestamp: String,
}

/// 聊天对话框
pub struct ChatDialog {
    visible: bool,
    resolution_index: usize,
    position: egui::Pos2,
    
    // 聊天消息列表
    messages: Vec<ChatMessage>,
    scroll_offset: usize,
    
    // 输入框
    input_text: String,
    input_visible: bool,
}

impl ChatDialog {
    /// 创建聊天对话框
    pub fn new(main_dialog_x: f32, screen_height: f32, resolution_index: usize) -> Self {
        // 原工程：MainDialog.X + 230, ScreenHeight - 97
        let position = egui::pos2(main_dialog_x + 230.0, screen_height - 97.0);
        
        Self {
            visible: true,
            resolution_index,
            position,
            messages: Vec::new(),
            scroll_offset: 0,
            input_text: String::new(),
            input_visible: false,
        }
    }
    
    /// 添加聊天消息
    pub fn add_message(&mut self, text: impl Into<String>, color: egui::Color32) {
        self.messages.push(ChatMessage {
            text: text.into(),
            color,
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        });
        
        // 自动滚动到最新消息
        if self.messages.len() > 10 {
            self.scroll_offset = self.messages.len() - 10;
        }
    }
    
    /// 显示输入框
    pub fn show_input(&mut self) {
        self.input_visible = true;
        self.input_text.clear();
    }
    
    /// 隐藏输入框
    pub fn hide_input(&mut self) {
        self.input_visible = false;
        self.input_text.clear();
    }
    
    /// 绘制聊天窗口
    fn draw_chat(&self, ui: &mut egui::Ui, ctx: &egui::Context) -> egui::Rect {
        // 获取背景纹理索引 (800分辨率用2201，其他用2221)
        let bg_index = if self.resolution_index == 0 { 2201 } else { 2221 };
        
        // 绘制主背景
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(ui.cursor().min, bg_size);
                
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                ui.allocate_rect(bg_rect, egui::Sense::hover());
                return bg_rect;
            } else {
                println!("⚠️ ChatDialog: 纹理 {} 存在但 egui_texture 为空", bg_index);
            }
        } else {
            println!("⚠️ ChatDialog: 无法加载背景纹理 Prguse[{}]", bg_index);
        }
        
        // 如果纹理加载失败，绘制一个临时背景便于调试
        let default_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
        let default_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(default_width, 70.0));
        
        // 绘制半透明黑色背景作为占位符
        ui.painter().rect_filled(
            default_rect,
            2.0,
            egui::Color32::from_black_alpha(180),
        );
        
        ui.allocate_rect(default_rect, egui::Sense::hover());
        default_rect
    }
    
    /// 绘制聊天消息
    fn draw_messages(&self, ui: &mut egui::Ui, base_rect: &egui::Rect) {
        let message_area_x = 5.0;
        let message_area_y = 5.0;
        let line_height = 12.0;
        
        // 显示最近的消息（最多4行）
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + 4).min(self.messages.len());
        
        for (i, msg) in self.messages[start_idx..end_idx].iter().enumerate() {
            let y = message_area_y + (i as f32 * line_height);
            
            ui.painter().text(
                egui::pos2(base_rect.min.x + message_area_x, base_rect.min.y + y),
                egui::Align2::LEFT_TOP,
                &msg.text,
                egui::FontId::monospace(10.0),
                msg.color,
            );
        }
    }
    
    /// 绘制滚动条按钮
    fn draw_scroll_buttons(&self, ui: &mut egui::Ui, ctx: &egui::Context, base_rect: &egui::Rect) {
        // 滚动条位置（原工程：622/398 是 CountBar，619/395 是 PositionBar）
        let countbar_x = if self.resolution_index == 0 { 398.0 } else { 622.0 };
        let posbar_x = if self.resolution_index == 0 { 395.0 } else { 619.0 };
        let scroll_x = if self.resolution_index == 0 { 394.0 } else { 618.0 };
        
        // CountBar - 滚动条背景轨道 (Prguse[2012])
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 2012) {
            if let Some(texture) = info.egui_texture {
                let size = texture.size_vec2();
                let rect = egui::Rect::from_min_size(
                    egui::pos2(base_rect.min.x + countbar_x, base_rect.min.y + 16.0),
                    size,
                );
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // PositionBar - 滚动条滑块 (Prguse[2015, 2016, 2017])
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 2015) {
            if let Some(texture) = info.egui_texture {
                let size = texture.size_vec2();
                let rect = egui::Rect::from_min_size(
                    egui::pos2(base_rect.min.x + posbar_x, base_rect.min.y + 16.0),
                    size,
                );
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        
        // Home 按钮 (2018, 2019, 2020)
        self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 1.0, 2018, 2019, 2020);
        
        // Up 按钮 (2021, 2022, 2023)
        self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 9.0, 2021, 2022, 2023);
        
        // Down 按钮 (2024, 2025, 2026)
        self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 39.0, 2024, 2025, 2026);
        
        // End 按钮 (2027, 2028, 2029)
        self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 45.0, 2027, 2028, 2029);
    }
    
    /// 绘制单个滚动按钮
    fn draw_scroll_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        base_rect: &egui::Rect,
        x: f32,
        y: f32,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
    ) {
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, normal_idx) {
            if let Some(texture) = info.egui_texture {
                let size = texture.size_vec2();
                let btn_rect = egui::Rect::from_min_size(
                    egui::pos2(base_rect.min.x + x, base_rect.min.y + y),
                    size,
                );
                
                let response = ui.interact(
                    btn_rect,
                    egui::Id::new(format!("scroll_btn_{}", normal_idx)),
                    egui::Sense::click(),
                );
                
                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = LibraryName::Prguse.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_texture) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            btn_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        }
    }
    
    /// 绘制输入框
    fn draw_input(&mut self, ui: &mut egui::Ui, base_rect: &egui::Rect) {
        if !self.input_visible {
            return;
        }
        
        // 输入框位置和尺寸（原工程：Location = (1, 54), Size = (627 or 403, 13)）
        let input_x = 1.0;
        let input_y = 54.0;
        let input_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
        let input_height = 13.0;
        
        let input_rect = egui::Rect::from_min_size(
            egui::pos2(base_rect.min.x + input_x, base_rect.min.y + input_y),
            egui::vec2(input_width, input_height),
        );
        
        // 绘制深灰色背景（DarkGray = RGB(169, 169, 169)）
        ui.painter().rect_filled(
            input_rect,
            1.0,
            egui::Color32::from_rgb(169, 169, 169),
        );
        
        // 绘制文本输入框
        let text_edit = egui::TextEdit::singleline(&mut self.input_text)
            .desired_width(input_width - 4.0)
            .font(egui::FontId::monospace(10.0))
            .text_color(egui::Color32::BLACK);
        
        let response = ui.put(
            egui::Rect::from_min_size(
                egui::pos2(input_rect.min.x + 2.0, input_rect.min.y + 1.0),
                egui::vec2(input_width - 4.0, input_height - 2.0),
            ),
            text_edit,
        );
        
        // 自动获取焦点
        response.request_focus();
        
        // 处理回车发送
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if !self.input_text.is_empty() {
                // 发送消息
                println!("📤 发送聊天: {}", self.input_text);
                // TODO: 发送到服务器
                // Network.Enqueue(new C.Chat { Message = self.input_text });
                
                // 添加到本地消息列表
                self.add_message(
                    format!("[我] {}", self.input_text),
                    egui::Color32::WHITE,
                );
                
                self.input_text.clear();
            }
            self.input_visible = false;
        }
        
        // ESC 取消
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.input_visible = false;
            self.input_text.clear();
        }
    }
}

impl Dialog for ChatDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !self.visible {
            *open = false;
            return;
        }
        
        egui::Area::new(egui::Id::new("chat_dialog"))
            .fixed_pos(self.position)
            .movable(false)
            .interactable(true)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                // 绘制背景并获取矩形
                let base_rect = self.draw_chat(ui, ctx);
                
                // 绘制消息（传入基础矩形）
                self.draw_messages(ui, &base_rect);
                
                // 绘制滚动条按钮
                self.draw_scroll_buttons(ui, ctx, &base_rect);
                
                // 绘制输入框（传入基础矩形）
                self.draw_input(ui, &base_rect);
            });
        
        *open = self.visible;
    }
}
