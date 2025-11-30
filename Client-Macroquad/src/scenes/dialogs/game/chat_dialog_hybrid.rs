// ============================================================================
// ChatDialogHybrid - 聊天窗口（混合版本）
// ============================================================================
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 使用 DragHelper 实现拖拽功能
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use super::native_ui_utils::DragHelper;

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub text: String,
    pub color: Color,
    pub timestamp: String,
}

/// 聊天对话框（混合版本）
pub struct ChatDialogHybrid {
    /// 分辨率索引
    resolution_index: usize,
    /// 窗口位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 聊天消息列表
    messages: Vec<ChatMessage>,
    /// 滚动偏移
    scroll_offset: usize,
    /// 输入文本
    input_text: String,
    /// 输入框是否激活
    input_active: bool,
    /// 窗口大小：0=小(4行), 1=中(7行), 2=大(11行)
    window_size: usize,
    /// 当前行数
    line_count: usize,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 当前尺寸
    current_size: Vec2,
    /// 拖拽辅助器
    drag_helper: DragHelper,
}

impl ChatDialogHybrid {
    /// 创建聊天对话框
    pub fn new(main_dialog_x: f32, screen_height: f32, resolution_index: usize) -> Self {
        let position = vec2(main_dialog_x + 230.0, screen_height - 97.0);

        Self {
            resolution_index,
            position,
            visible: true,
            messages: Vec::new(),
            scroll_offset: 0,
            input_text: String::new(),
            input_active: false,
            window_size: 0,
            line_count: 4,
            bg_texture: None,
            current_size: vec2(627.0, 70.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 输入框是否激活（用于判断是否应该消耗键盘输入）
    pub fn is_input_active(&self) -> bool {
        self.input_active
    }

    /// 设置位置
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    /// 检查点是否在对话框内
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        point.x >= self.position.x
            && point.x <= self.position.x + self.current_size.x
            && point.y >= self.position.y
            && point.y <= self.position.y + self.current_size.y
    }

    /// 添加聊天消息
    pub fn add_message(&mut self, text: impl Into<String>, color: Color) {
        self.messages.push(ChatMessage {
            text: text.into(),
            color,
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
        });

        // 自动滚动到最新消息
        if self.messages.len() > self.line_count {
            self.scroll_offset = self.messages.len() - self.line_count;
        }
    }

    /// 切换窗口大小
    pub fn change_size(&mut self, screen_height: f32) {
        self.window_size = (self.window_size + 1) % 3;

        self.line_count = match self.window_size {
            0 => 4,
            1 => 7,
            2 => 11,
            _ => 4,
        };

        let y_offset = match self.window_size {
            0 => 97.0,
            1 => 97.0 + 48.0,
            2 => 97.0 + 96.0,
            _ => 97.0,
        };

        self.position.y = screen_height - y_offset;
        self.bg_texture = None; // 重新加载纹理
    }

    /// 异步加载纹理
    pub async fn load_textures(&mut self) {
        // 预加载聊天窗口纹理
        for idx in [2201, 2204, 2207, 2221, 2224, 2227] {
            let _ = LibraryName::Prguse.get_texture(idx);
        }
        // 预加载滚动条纹理
        for idx in [2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027, 2028, 2029] {
            let _ = LibraryName::Prguse.get_texture(idx);
        }
    }

    /// 更新和绘制
    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 获取背景纹理
        let bg_index = match (self.window_size, self.resolution_index) {
            (0, 0) => 2201,
            (0, _) => 2221,
            (1, 0) => 2204,
            (1, _) => 2224,
            (2, 0) => 2207,
            (2, _) => 2227,
            _ => if self.resolution_index == 0 { 2201 } else { 2221 },
        };

        if let Some(texture) = LibraryName::Prguse.get_texture(bg_index) {
            self.current_size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 使用 DragHelper 实现拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.current_size.x, 20.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制消息
        self.draw_messages();

        // 绘制滚动条
        self.draw_scroll_buttons(mouse_pos);

        // 绘制输入框
        self.draw_input();
    }

    /// 绘制背景
    fn draw_background(&self) {
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );

            // 消息区域白色背景
            let msg_width = if self.resolution_index == 0 { 380.0 } else { 600.0 };
            let msg_height = (self.line_count as f32 * 10.0) + 4.0;
            draw_rectangle(
                self.position.x + 5.0,
                self.position.y + 5.0,
                msg_width,
                msg_height,
                WHITE,
            );
        } else {
            // 降级
            let default_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
            draw_rectangle(
                self.position.x,
                self.position.y,
                default_width,
                70.0,
                Color::from_rgba(50, 50, 50, 255),
            );
        }
    }

    /// 绘制消息（使用中文字体）
    fn draw_messages(&self) {
        let msg_x = self.position.x + 8.0;
        let msg_y = self.position.y + 7.0;
        let line_height = 14.0;

        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + self.line_count).min(self.messages.len());

        for (i, msg) in self.messages[start_idx..end_idx].iter().enumerate() {
            let y = msg_y + (i as f32 * line_height);
            draw_text_cn(&msg.text, msg_x, y + 12.0, 12.0, msg.color);
        }
    }

    /// 绘制滚动条按钮
    fn draw_scroll_buttons(&mut self, mouse_pos: Vec2) {
        let scroll_x = if self.resolution_index == 0 { 394.0 } else { 618.0 };

        // Home 按钮
        if self.draw_scroll_button(mouse_pos, scroll_x, 1.0, 2018, 2019, 2020) {
            self.scroll_offset = 0;
        }

        // Up 按钮
        if self.draw_scroll_button(mouse_pos, scroll_x, 9.0, 2021, 2022, 2023) {
            if self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        }

        // Down 按钮
        let down_y = match self.window_size {
            0 => 39.0,
            1 => 39.0 + 48.0,
            2 => 39.0 + 96.0,
            _ => 39.0,
        };
        if self.draw_scroll_button(mouse_pos, scroll_x, down_y, 2024, 2025, 2026) {
            let max_scroll = self.messages.len().saturating_sub(self.line_count);
            if self.scroll_offset < max_scroll {
                self.scroll_offset += 1;
            }
        }

        // End 按钮
        let end_y = match self.window_size {
            0 => 45.0,
            1 => 45.0 + 48.0,
            2 => 45.0 + 96.0,
            _ => 45.0,
        };
        if self.draw_scroll_button(mouse_pos, scroll_x, end_y, 2027, 2028, 2029) {
            let max_scroll = self.messages.len().saturating_sub(self.line_count);
            self.scroll_offset = max_scroll;
        }

        // 处理鼠标滚轮
        let msg_width = if self.resolution_index == 0 { 380.0 } else { 600.0 };
        let msg_height = (self.line_count as f32 * 10.0) + 4.0;
        let msg_rect = Rect::new(
            self.position.x + 5.0,
            self.position.y + 5.0,
            msg_width,
            msg_height,
        );

        if msg_rect.contains(mouse_pos) {
            let scroll_delta = mouse_wheel().1;
            if scroll_delta.abs() > 0.1 {
                let max_scroll = self.messages.len().saturating_sub(self.line_count);
                let delta_lines = (-scroll_delta / 10.0).round() as i32;
                let new_offset = (self.scroll_offset as i32 + delta_lines)
                    .clamp(0, max_scroll as i32) as usize;
                self.scroll_offset = new_offset;
            }
        }
    }

    /// 绘制单个滚动按钮
    fn draw_scroll_button(
        &self,
        mouse_pos: Vec2,
        x: f32,
        y: f32,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
    ) -> bool {
        let button_pos = vec2(self.position.x + x, self.position.y + y);

        if let Some(texture) = LibraryName::Prguse.get_texture(normal_idx) {
            let button_rect = Rect::new(button_pos.x, button_pos.y, texture.width as f32, texture.height as f32);
            let is_hovered = button_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let texture_idx = if is_pressed {
                pressed_idx
            } else if is_hovered {
                hover_idx
            } else {
                normal_idx
            };

            if let Some(btn_texture) = LibraryName::Prguse.get_texture(texture_idx) {
                if let Some(ref tex) = btn_texture.image {
                    draw_texture_ex(
                        tex,
                        button_pos.x,
                        button_pos.y,
                        WHITE,
                        DrawTextureParams::default(),
                    );
                }
            }

            return is_hovered && is_mouse_button_pressed(MouseButton::Left);
        }

        false
    }

    /// 绘制输入框（使用 egui 处理 IME 输入）
    fn draw_input(&mut self) {
        let input_y = match self.window_size {
            0 => 54.0,
            1 => 54.0 + 48.0,
            2 => 54.0 + 96.0,
            _ => 54.0,
        };
        let input_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
        let input_height = 13.0;

        let input_rect = Rect::new(
            self.position.x + 1.0,
            self.position.y + input_y,
            input_width,
            input_height,
        );

        // 白色背景
        draw_rectangle(
            input_rect.x,
            input_rect.y,
            input_rect.w,
            input_rect.h,
            WHITE,
        );

        // 绘制输入文本（使用中文字体）
        if !self.input_text.is_empty() {
            draw_text_cn(
                &self.input_text,
                input_rect.x + 3.0,
                input_rect.y + 12.0,
                14.0,
                BLACK,
            );
        }

        // 绘制光标
        if self.input_active {
            let cursor_x = input_rect.x + 3.0 + measure_text_cn(&self.input_text, 14.0).width;
            draw_line(
                cursor_x,
                input_rect.y + 2.0,
                cursor_x,
                input_rect.y + input_height - 2.0,
                1.0,
                BLACK,
            );
        }

        // 检测点击激活
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        if input_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left) {
            self.input_active = true;
        } else if !input_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left) {
            self.input_active = false;
        }

        // 处理键盘输入（支持中文）
        if self.input_active {
            // 获取输入的字符（支持中文和其他Unicode字符）
            while let Some(ch) = get_char_pressed() {
                // 过滤控制字符，但允许所有可打印字符（包括中文）
                if !ch.is_control() {
                    self.input_text.push(ch);
                }
            }
            
            // Enter 发送消息
            if is_key_pressed(KeyCode::Enter) && !self.input_text.is_empty() {
                println!("📤 发送聊天: {}", self.input_text);
                self.add_message(format!("[我] {}", self.input_text), WHITE);
                self.input_text.clear();
            }
            // Escape 取消输入
            if is_key_pressed(KeyCode::Escape) {
                self.input_text.clear();
                self.input_active = false;
            }
            // Backspace 删除字符
            if is_key_pressed(KeyCode::Backspace) && !self.input_text.is_empty() {
                self.input_text.pop();
            }
        }
    }
}
