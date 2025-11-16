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
        // 获取背景纹理索引
        let bg_index = if self.resolution_index == 0 { 2201 } else { 2221 };
        
        let base_rect = if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, bg_index) {
            if let Some(bg_texture) = info.egui_texture {
                let bg_size = bg_texture.size_vec2();
                let bg_rect = egui::Rect::from_min_size(ui.cursor().min, bg_size);
                
                // 先绘制主背景纹理
                ui.painter().image(
                    bg_texture.id(),
                    bg_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                
                // 立即在主背景上绘制深色消息背景（关键：使用同一个 painter）
                let message_area_x = 5.0;
                let message_area_y = 5.0;
                let message_area_width = if self.resolution_index == 0 { 380.0 } else { 600.0 };
                let message_area_height = 40.0;
                let msg_bg_rect = egui::Rect::from_min_size(
                    egui::pos2(bg_rect.min.x + message_area_x, bg_rect.min.y + message_area_y),
                    egui::vec2(message_area_width, message_area_height),
                );
                
                // 使用白色背景覆盖主背景纹理的消息区域
                ui.painter().rect_filled(
                    msg_bg_rect,
                    0.0,
                    egui::Color32::WHITE, // 白色背景
                );
                
                ui.allocate_rect(bg_rect, egui::Sense::hover());
                bg_rect
            } else {
                // 纹理加载失败，绘制默认背景
                let default_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
                let r = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(default_width, 70.0));
                ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(50, 50, 50));
                ui.allocate_rect(r, egui::Sense::hover());
                r
            }
        } else {
            // 纹理不存在，绘制默认背景
            let default_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
            let r = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(default_width, 70.0));
            ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(50, 50, 50));
            ui.allocate_rect(r, egui::Sense::hover());
            r
        };
        
        base_rect
    }
    
    /// 绘制消息区域深色背景（在主背景之后绘制，覆盖主背景）
    #[allow(dead_code)]
    fn draw_message_background(&self, ui: &mut egui::Ui, base_rect: &egui::Rect) {
        let message_area_x = 5.0;
        let message_area_y = 5.0;
        let message_area_width = if self.resolution_index == 0 { 380.0 } else { 600.0 };
        let message_area_height = 40.0;
        
        let msg_bg_rect = egui::Rect::from_min_size(
            egui::pos2(base_rect.min.x + message_area_x, base_rect.min.y + message_area_y),
            egui::vec2(message_area_width, message_area_height),
        );
        
        // 直接使用 ui.painter() 在当前上下文绘制
        ui.painter().rect_filled(
            msg_bg_rect,
            0.0,
            egui::Color32::from_rgb(30, 30, 30),
        );
    }
    
    /// 绘制聊天消息显示框（上方区域）
    fn draw_messages(&self, ui: &mut egui::Ui, base_rect: &egui::Rect) {
        // 消息显示区域：位置(5, 5)，高度约40像素（显示4行，每行10像素）
        let message_area_x = 5.0;
        let message_area_y = 5.0;
        let line_height = 10.0;
        
        // 显示最近的消息（最多4行）
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + 4).min(self.messages.len());
        
        for (i, msg) in self.messages[start_idx..end_idx].iter().enumerate() {
            let y = message_area_y + 2.0 + (i as f32 * line_height);
            
            ui.painter().text(
                egui::pos2(base_rect.min.x + message_area_x + 3.0, base_rect.min.y + y),
                egui::Align2::LEFT_TOP,
                &msg.text,
                egui::FontId::proportional(9.0),
                msg.color,
            );
        }
    }
    
    /// 绘制滚动条按钮
    fn draw_scroll_buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, base_rect: &egui::Rect) {
        // 先检测消息区域的鼠标滚轮事件
        let msg_width = if self.resolution_index == 0 { 380.0 } else { 600.0 };
        let msg_rect = egui::Rect::from_min_size(
            egui::pos2(base_rect.min.x + 5.0, base_rect.min.y + 5.0),
            egui::vec2(msg_width, 40.0)
        );
        
        let msg_response = ui.interact(
            msg_rect,
            egui::Id::new("chat_messages_scroll"),
            egui::Sense::hover(),
        );
        
        if msg_response.hovered() {
            ctx.input(|i| {
                let scroll_delta = i.smooth_scroll_delta.y;
                if scroll_delta.abs() > 0.1 {
                    let max_scroll = self.messages.len().saturating_sub(4);
                    let delta_lines = (-scroll_delta / 10.0).round() as i32;
                    let new_offset = (self.scroll_offset as i32 + delta_lines)
                        .clamp(0, max_scroll as i32) as usize;
                    self.scroll_offset = new_offset;
                }
            });
        }
        
        // 滚动条位置（原工程：622/398 是 CountBar，619/395 是 PositionBar）
        let countbar_x = if self.resolution_index == 0 { 398.0 } else { 622.0 };
        let posbar_x = if self.resolution_index == 0 { 395.0 } else { 619.0 };
        let scroll_x = if self.resolution_index == 0 { 394.0 } else { 618.0 };
        
        // CountBar - 滚动条背景轨道 (Prguse[2012])
        let countbar_height = if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 2012) {
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
                size.y
            } else {
                30.0
            }
        } else {
            30.0
        };
        
        // 先绘制4个按钮
        // Home 按钮 (2018, 2019, 2020) - 滚动到顶部
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 1.0, 2018, 2019, 2020) {
            self.scroll_offset = 0;
        }
        
        // Up 按钮 (2021, 2022, 2023) - 向上滚动
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 9.0, 2021, 2022, 2023) {
            if self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        }
        
        // Down 按钮 (2024, 2025, 2026) - 向下滚动
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 39.0, 2024, 2025, 2026) {
            let max_scroll = self.messages.len().saturating_sub(4);
            if self.scroll_offset < max_scroll {
                self.scroll_offset += 1;
            }
        }
        
        // End 按钮 (2027, 2028, 2029) - 滚动到底部
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 45.0, 2027, 2028, 2029) {
            let max_scroll = self.messages.len().saturating_sub(4);
            self.scroll_offset = max_scroll;
        }
        
        // 最后绘制可拖动滑块（在最上层，确保能接收拖动事件）
        // PositionBar - 可拖动滚动条滑块 (Prguse[2015, 2016, 2017])
        if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 2015) {
            if let Some(texture) = info.egui_texture {
                let size = texture.size_vec2();
                
                // 计算滑块位置（基于滚动偏移）
                let max_scroll = self.messages.len().saturating_sub(4);
                let scroll_range = countbar_height - size.y;
                let slider_y = if max_scroll > 0 {
                    16.0 + (self.scroll_offset as f32 / max_scroll as f32) * scroll_range
                } else {
                    16.0
                };
                
                let rect = egui::Rect::from_min_size(
                    egui::pos2(base_rect.min.x + posbar_x, base_rect.min.y + slider_y),
                    size,
                );
                
                // 拖动交互 - 使用 click_and_drag 提高优先级
                let id = egui::Id::new("chat_scroll_slider");
                let response = ui.interact(
                    rect,
                    id,
                    egui::Sense::click_and_drag(),
                );
                
                // 处理拖动 - 使用绝对位置计算
                if (response.dragged() || response.drag_started()) && max_scroll > 0 {
                    // 获取鼠标位置
                    if let Some(pointer_pos) = ctx.pointer_latest_pos() {
                        // 计算鼠标在滚动轨道中的相对位置
                        let track_start_y = base_rect.min.y + 16.0;
                        let relative_y = (pointer_pos.y - track_start_y).clamp(0.0, scroll_range);
                        
                        // 根据相对位置计算滚动偏移
                        let scroll_ratio = relative_y / scroll_range;
                        self.scroll_offset = (scroll_ratio * max_scroll as f32).round() as usize;
                        self.scroll_offset = self.scroll_offset.min(max_scroll);
                    }
                }
                
                // 也支持点击滚动条轨道直接跳转
                if response.clicked() && max_scroll > 0 {
                    if let Some(pointer_pos) = ctx.pointer_interact_pos() {
                        let track_start_y = base_rect.min.y + 16.0;
                        let relative_y = (pointer_pos.y - track_start_y).clamp(0.0, scroll_range);
                        let scroll_ratio = relative_y / scroll_range;
                        self.scroll_offset = (scroll_ratio * max_scroll as f32).round() as usize;
                        self.scroll_offset = self.scroll_offset.min(max_scroll);
                    }
                }
                
                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    2017  // pressed
                } else if response.hovered() {
                    2016  // hover
                } else {
                    2015  // normal
                };
                
                if let Some(bar_info) = LibraryName::Prguse.get_egui_texture(ctx, texture_idx) {
                    if let Some(bar_texture) = bar_info.egui_texture {
                        ui.painter().image(
                            bar_texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        }
        
        // Home 按钮 (2018, 2019, 2020) - 滚动到顶部
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 1.0, 2018, 2019, 2020) {
            self.scroll_offset = 0;
        }
        
        // Up 按钮 (2021, 2022, 2023) - 向上滚动
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 9.0, 2021, 2022, 2023) {
            if self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        }
        
        // Down 按钮 (2024, 2025, 2026) - 向下滚动
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 39.0, 2024, 2025, 2026) {
            let max_scroll = self.messages.len().saturating_sub(4);
            if self.scroll_offset < max_scroll {
                self.scroll_offset += 1;
            }
        }
        
        // End 按钮 (2027, 2028, 2029) - 滚动到底部
        if self.draw_scroll_button(ui, ctx, base_rect, scroll_x, 45.0, 2027, 2028, 2029) {
            let max_scroll = self.messages.len().saturating_sub(4);
            self.scroll_offset = max_scroll;
        }
    }
    
    /// 绘制单个滚动按钮（返回是否被点击）
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
    ) -> bool {
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
                
                return response.clicked();
            }
        }
        false
    }
    
    /// 绘制输入框（下方区域，独立于消息显示框）
    fn draw_input(&mut self, ui: &mut egui::Ui, base_rect: &egui::Rect, ctx: &egui::Context) {
        // 输入框位置和尺寸（原工程：Location = (1, 54), Size = (627 or 403, 13)）
        // 这是下方的输入区域，与上方的消息显示区域分离
        let input_x = 1.0;
        let input_y = 54.0;
        let input_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
        let input_height = 13.0;
        
        let input_rect = egui::Rect::from_min_size(
            egui::pos2(base_rect.min.x + input_x, base_rect.min.y + input_y),
            egui::vec2(input_width, input_height),
        );
        
        // 绘制白色背景，让输入框更明显
        ui.painter().rect_filled(
            input_rect,
            1.0,
            egui::Color32::WHITE,
        );
        
        // 使用 allocate_ui_at_rect 在指定位置放置交互式输入框
        let mut text_rect = input_rect;
        text_rect.min.x += 2.0;
        text_rect.min.y += 1.0 - 2.0; // 光标向上移动3像素
        text_rect.max.x -= 2.0;
        text_rect.max.y -= 1.0;
        
        // 临时修改 UI 样式，设置光标颜色为黑色
        let mut style = (*ctx.style()).clone();
        style.visuals.text_cursor.stroke = egui::Stroke::new(2.0, egui::Color32::BLACK); // 黑色粗光标
        ctx.set_style(style);
        
        let text_edit = egui::TextEdit::singleline(&mut self.input_text)
            .desired_width(text_rect.width())
            .frame(false)
            .font(egui::FontId::proportional(9.0)) // 字体从10改为9，使用 Proportional 支持中文
            .text_color(egui::Color32::BLACK)
            .cursor_at_end(true); // 光标放在末尾
        
        let response = ui.put(text_rect, text_edit);
        
        // 点击输入框时获取焦点，不再需要手动维护焦点
        // egui 会自动处理点击后的焦点获取
        
        // 处理回车发送
        if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
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
                // 发送后重新获取焦点，继续输入
                response.request_focus();
            }
        }
        
        // ESC 清空输入框但不隐藏
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
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
                // 绘制背景纹理
                let base_rect = self.draw_chat(ui, ctx);
                
                // 绘制消息
                self.draw_messages(ui, &base_rect);
                
                // 绘制滚动条
                self.draw_scroll_buttons(ui, ctx, &base_rect);
                
                // 绘制输入框
                self.draw_input(ui, &base_rect, ctx);
            });        *open = self.visible;
    }
}
