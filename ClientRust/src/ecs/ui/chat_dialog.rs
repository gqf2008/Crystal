// ============================================================================
// 聊天对话框
// ============================================================================
//
// 功能:
// - 消息历史显示(可滚动)
// - 多频道支持(综合/私聊/组队/公会)
// - 文本输入和发送
// - 消息类型颜色区分
//
// 参考: Client/MirScenes/Dialogs/MainDialogs.cs (ChatDialog)
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Rect, DrawParam, Text, TextFragment};

/// 聊天类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatType {
    Normal,       // 普通聊天(白色)
    Whisper,      // 私聊(蓝色)
    Group,        // 组队(绿色)
    Guild,        // 公会(紫色)
    Shout,        // 喊话(黄色)
    System,       // 系统消息(红色)
    Hint,         // 提示(深绿色)
    Announcement, // 公告(蓝底白字)
}

impl ChatType {
    /// 获取文字颜色
    pub fn text_color(&self) -> Color {
        match self {
            ChatType::Normal => Color::WHITE,
            ChatType::Whisper => Color::from_rgb(100, 149, 237),
            ChatType::Group => Color::from_rgb(144, 238, 144),
            ChatType::Guild => Color::from_rgb(186, 85, 211),
            ChatType::Shout => Color::from_rgb(255, 255, 100),
            ChatType::System => Color::from_rgb(255, 100, 100),
            ChatType::Hint => Color::from_rgb(34, 139, 34),
            ChatType::Announcement => Color::WHITE,
        }
    }
    
    /// 获取背景颜色
    pub fn background_color(&self) -> Option<Color> {
        match self {
            ChatType::Announcement => Some(Color::from_rgba(0, 0, 255, 128)),
            _ => None,
        }
    }
}

/// 聊天消息
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub text: String,
    pub chat_type: ChatType,
    pub timestamp: u64,
}

/// 聊天对话框
pub struct ChatDialog {
    /// 是否可见
    visible: bool,
    
    /// 位置和尺寸
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    
    /// 消息历史
    messages: Vec<ChatMessage>,
    
    /// 滚动偏移量
    scroll_offset: usize,
    
    /// 最大显示行数
    max_visible_lines: usize,
    
    /// 输入框文本
    input_text: String,
    
    /// 输入框是否激活
    input_active: bool,
    
    /// 悬停状态
    hovered: bool,
    
    /// 光标闪烁计时器 (帧数)
    cursor_blink_timer: u32,
    
    /// 光标是否可见 (每30帧切换一次)
    cursor_visible: bool,
}

/// 聊天对话框操作
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatAction {
    /// 关闭
    Close,
    
    /// 发送消息
    SendMessage(String),
    
    /// 滚动到顶部
    ScrollHome,
    
    /// 向上滚动
    ScrollUp,
    
    /// 向下滚动
    ScrollDown,
    
    /// 滚动到底部
    ScrollEnd,
    
    /// 激活输入框
    ActivateInput,
}

impl ChatDialog {
    /// 创建新聊天对话框
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            visible: true, // 聊天框默认可见
            x,
            y,
            width: 400.0,
            height: 150.0,
            messages: Vec::new(),
            scroll_offset: 0,
            max_visible_lines: 10,
            input_text: String::new(),
            input_active: false,
            hovered: false,
            cursor_blink_timer: 0,
            cursor_visible: true,
        }
    }
    
    /// 添加消息
    pub fn add_message(&mut self, text: String, chat_type: ChatType) {
        self.messages.push(ChatMessage {
            text,
            chat_type,
            timestamp: 0, // TODO: 使用真实时间戳
        });
        
        // 自动滚动到底部
        if self.messages.len() > self.max_visible_lines {
            self.scroll_offset = self.messages.len() - self.max_visible_lines;
        }
    }
    
    /// 清空消息
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }
    
    /// 切换显示/隐藏
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    /// 显示
    pub fn show(&mut self) {
        self.visible = true;
    }
    
    /// 隐藏
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 更新（每帧调用，用于光标闪烁）
    pub fn update(&mut self) {
        if self.input_active {
            self.cursor_blink_timer += 1;
            if self.cursor_blink_timer >= 30 {  // 每30帧切换一次（约0.5秒）
                self.cursor_blink_timer = 0;
                self.cursor_visible = !self.cursor_visible;
            }
        } else {
            self.cursor_blink_timer = 0;
            self.cursor_visible = true;
        }
    }
    
    /// 激活输入框
    pub fn activate_input(&mut self) {
        self.input_active = true;
        self.cursor_blink_timer = 0;
        self.cursor_visible = true;
    }
    
    /// 取消输入
    pub fn deactivate_input(&mut self) {
        self.input_active = false;
        self.input_text.clear();
    }
    
    /// 检查输入框是否激活
    pub fn is_input_active(&self) -> bool {
        self.input_active
    }
    
    /// 输入字符
    pub fn input_char(&mut self, ch: char) {
        if !self.input_active {
            return;
        }
        
        if self.input_text.len() < 100 {
            self.input_text.push(ch);
        }
    }
    
    /// 删除字符
    pub fn backspace(&mut self) {
        if !self.input_active {
            return;
        }
        
        self.input_text.pop();
    }
    
    /// 获取输入文本
    pub fn get_input(&self) -> &str {
        &self.input_text
    }
    
    /// 检查点击
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> Option<ChatAction> {
        if !self.visible {
            return None;
        }
        
        // 检查输入框点击
        if self.is_in_input_box(x, y) {
            self.input_active = true;
            return Some(ChatAction::ActivateInput);
        }
        
        // 检查滚动按钮
        if self.is_in_scroll_up_button(x, y) {
            return Some(ChatAction::ScrollUp);
        }
        
        if self.is_in_scroll_down_button(x, y) {
            return Some(ChatAction::ScrollDown);
        }
        
        if self.is_in_scroll_home_button(x, y) {
            return Some(ChatAction::ScrollHome);
        }
        
        if self.is_in_scroll_end_button(x, y) {
            return Some(ChatAction::ScrollEnd);
        }
        
        None
    }
    
    /// 更新悬停状态
    pub fn update_hover(&mut self, x: f32, y: f32) {
        if !self.visible {
            self.hovered = false;
            return;
        }
        
        let rect = Rect::new(self.x, self.y, self.width, self.height);
        self.hovered = rect.contains([x, y]);
    }
    
    /// 处理滚轮滚动
    pub fn on_mouse_wheel(&mut self, delta: f32) {
        if !self.hovered || !self.visible {
            return;
        }
        
        if delta > 0.0 {
            // 向上滚动
            if self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        } else if delta < 0.0 {
            // 向下滚动
            let max_offset = self.messages.len().saturating_sub(self.max_visible_lines);
            if self.scroll_offset < max_offset {
                self.scroll_offset += 1;
            }
        }
    }
    
    /// 滚动到顶部
    pub fn scroll_home(&mut self) {
        self.scroll_offset = 0;
    }
    
    /// 滚动到底部
    pub fn scroll_end(&mut self) {
        let max_offset = self.messages.len().saturating_sub(self.max_visible_lines);
        self.scroll_offset = max_offset;
    }
    
    /// 向上滚动一行
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }
    
    /// 向下滚动一行
    pub fn scroll_down(&mut self) {
        let max_offset = self.messages.len().saturating_sub(self.max_visible_lines);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }
    
    // 辅助方法: 检查各个区域
    fn is_in_input_box(&self, x: f32, y: f32) -> bool {
        let rect = Rect::new(self.x + 5.0, self.y + self.height - 20.0, self.width - 60.0, 15.0);
        rect.contains([x, y])
    }
    
    fn is_in_scroll_up_button(&self, x: f32, y: f32) -> bool {
        let rect = Rect::new(self.x + self.width - 20.0, self.y + 20.0, 15.0, 15.0);
        rect.contains([x, y])
    }
    
    fn is_in_scroll_down_button(&self, x: f32, y: f32) -> bool {
        let rect = Rect::new(self.x + self.width - 20.0, self.y + self.height - 55.0, 15.0, 15.0);
        rect.contains([x, y])
    }
    
    fn is_in_scroll_home_button(&self, x: f32, y: f32) -> bool {
        let rect = Rect::new(self.x + self.width - 20.0, self.y + 5.0, 15.0, 15.0);
        rect.contains([x, y])
    }
    
    fn is_in_scroll_end_button(&self, x: f32, y: f32) -> bool {
        let rect = Rect::new(self.x + self.width - 20.0, self.y + self.height - 40.0, 15.0, 15.0);
        rect.contains([x, y])
    }
    
    /// 渲染
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        
        // 背景
        let background = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x, self.y, self.width, self.height),
            Color::from_rgba(20, 20, 20, 200),
        )?;
        canvas.draw(&background, DrawParam::default());
        
        // 边框
        let border = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            Rect::new(self.x, self.y, self.width, self.height),
            Color::from_rgb(100, 100, 100),
        )?;
        canvas.draw(&border, DrawParam::default());
        
        // 标题
        let title = Text::new("聊天");
        canvas.draw(
            &title,
            DrawParam::default()
                .dest([self.x + 5.0, self.y + 3.0])
                .color(Color::from_rgb(200, 200, 200)),
        );
        
        // 消息列表
        let line_height = 12.0;
        let start_y = self.y + 25.0;
        let visible_messages = self.messages
            .iter()
            .skip(self.scroll_offset)
            .take(self.max_visible_lines);
        
        for (i, msg) in visible_messages.enumerate() {
            let y = start_y + (i as f32 * line_height);
            
            // 背景色(如果有)
            if let Some(bg_color) = msg.chat_type.background_color() {
                let msg_bg = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(self.x + 5.0, y - 2.0, self.width - 30.0, line_height),
                    bg_color,
                )?;
                canvas.draw(&msg_bg, DrawParam::default());
            }
            
            // 消息文本
            let text = Text::new(&msg.text);
            canvas.draw(
                &text,
                DrawParam::default()
                    .dest([self.x + 8.0, y])
                    .color(msg.chat_type.text_color())
                    .scale([0.8, 0.8]),
            );
        }
        
        // 滚动条背景
        let scrollbar_bg = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + self.width - 18.0, self.y + 20.0, 15.0, self.height - 80.0),
            Color::from_rgba(40, 40, 40, 255),
        )?;
        canvas.draw(&scrollbar_bg, DrawParam::default());
        
        // 滚动条按钮
        let button_color = Color::from_rgb(80, 80, 80);
        
        // Home按钮(↑↑)
        let home_btn = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + self.width - 18.0, self.y + 5.0, 15.0, 15.0),
            button_color,
        )?;
        canvas.draw(&home_btn, DrawParam::default());
        
        // Up按钮(↑)
        let up_btn = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + self.width - 18.0, self.y + 22.0, 15.0, 15.0),
            button_color,
        )?;
        canvas.draw(&up_btn, DrawParam::default());
        
        // Down按钮(↓)
        let down_btn = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + self.width - 18.0, self.y + self.height - 55.0, 15.0, 15.0),
            button_color,
        )?;
        canvas.draw(&down_btn, DrawParam::default());
        
        // End按钮(↓↓)
        let end_btn = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + self.width - 18.0, self.y + self.height - 38.0, 15.0, 15.0),
            button_color,
        )?;
        canvas.draw(&end_btn, DrawParam::default());
        
        // 输入框
        let input_box = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + 5.0, self.y + self.height - 20.0, self.width - 60.0, 15.0),
            if self.input_active {
                Color::from_rgba(60, 60, 60, 255)
            } else {
                Color::from_rgba(40, 40, 40, 255)
            },
        )?;
        canvas.draw(&input_box, DrawParam::default());
        
        // 输入框边框
        let input_border = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            Rect::new(self.x + 5.0, self.y + self.height - 20.0, self.width - 60.0, 15.0),
            Color::from_rgb(100, 100, 100),
        )?;
        canvas.draw(&input_border, DrawParam::default());
        
        // 输入文本
        if !self.input_text.is_empty() || self.input_active {
            let input_display = if self.input_active && self.input_text.is_empty() {
                "输入聊天内容..."
            } else {
                &self.input_text
            };
            
            let input_text_obj = Text::new(input_display);
            canvas.draw(
                &input_text_obj,
                DrawParam::default()
                    .dest([self.x + 8.0, self.y + self.height - 18.0])
                    .color(if self.input_active && self.input_text.is_empty() {
                        Color::from_rgba(150, 150, 150, 200)
                    } else {
                        Color::WHITE
                    })
                    .scale([0.7, 0.7]),
            );
            
            // 绘制闪烁光标（只在输入激活且有实际文本时显示）
            if self.input_active && !self.input_text.is_empty() && self.cursor_visible {
                // 简单估算文本宽度：每个字符约6像素宽（缩放0.7后）
                let char_count = self.input_text.chars().count();
                let text_width = char_count as f32 * 6.0 * 0.7;
                let cursor_x = self.x + 8.0 + text_width;
                let cursor_y = self.y + self.height - 18.0;
                
                let cursor = ggez::graphics::Mesh::new_rectangle(
                    ctx,
                    ggez::graphics::DrawMode::fill(),
                    Rect::new(cursor_x, cursor_y, 1.5, 12.0),
                    Color::WHITE,
                )?;
                canvas.draw(&cursor, DrawParam::default());
            }
        }
        
        // 发送按钮
        let send_btn = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            Rect::new(self.x + self.width - 50.0, self.y + self.height - 20.0, 45.0, 15.0),
            Color::from_rgb(70, 70, 70),
        )?;
        canvas.draw(&send_btn, DrawParam::default());
        
        let send_text = Text::new("发送");
        canvas.draw(
            &send_text,
            DrawParam::default()
                .dest([self.x + self.width - 40.0, self.y + self.height - 18.0])
                .color(Color::WHITE)
                .scale([0.7, 0.7]),
        );
        
        Ok(())
    }
}
