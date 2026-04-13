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
use crate::utils::ime::{set_ime_enabled, set_ime_position};
use super::ChatOptionSettingsHybrid;
use super::native_ui_utils::DragHelper;

/// 全局聊天发送桥接：ChatDialog 设置此标志，ui_system 消费并发送网络事件。
static PENDING_CHAT_MESSAGE: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();

fn pending_chat() -> &'static std::sync::Mutex<Option<String>> {
    PENDING_CHAT_MESSAGE.get_or_init(|| std::sync::Mutex::new(None))
}

/// 检查是否有待发送的聊天消息（由 ui_system 调用）
pub fn take_pending_chat_message() -> Option<String> {
    pending_chat().lock().unwrap().take()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMessageKind {
    Normal,
    Whisper,
    Shout,
    System,
    Lover,
    Mentor,
    Group,
    Guild,
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub text: String,
    pub color: Color,
    pub timestamp: String,
    pub kind: ChatMessageKind,
}

impl ChatMessage {
    /// 解析消息中的特殊标记并渲染
    /// 支持的标记:
    ///   [物品:名称] -> 带下划线的蓝色文字，点击可查看详情
    ///   [坐标:x,y] -> 绿色文字，点击触发自动寻路
    ///   :smile: -> 表情符号
    /// 返回渲染的文本宽度
    fn draw_with_markup(&self, x: f32, y: f32, font_size: f32, mouse_pos: Vec2, _line_rect: Rect) -> f32 {
        let mut cursor_x = x;
        let text = &self.text;
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            // 检测 [ 开头的标记
            if ch == '[' {
                let mut tag_content = String::new();
                let mut found_close = false;
                for c in chars.by_ref() {
                    if c == ']' {
                        found_close = true;
                        break;
                    }
                    tag_content.push(c);
                }

                if found_close {
                    // 解析标记类型
                    if tag_content.starts_with("物品:") || tag_content.starts_with("Item:") {
                        let item_name = tag_content.split_once(':').map(|x| x.1).unwrap_or("");
                        let item_color = Color::from_rgba(150, 100, 255, 255); // 紫色
                        let w = measure_text_cn(item_name, font_size).width;
                        let item_rect = Rect::new(cursor_x, y - font_size + 2.0, w, font_size);

                        draw_text_cn(item_name, cursor_x, y, font_size, item_color);
                        // 下划线
                        draw_line(cursor_x, y + 1.0, cursor_x + w, y + 1.0, 1.0, item_color);

                        // 悬停高亮
                        if item_rect.contains(mouse_pos) {
                            draw_rectangle(cursor_x, y - font_size + 2.0, w, font_size, Color::from_rgba(150, 100, 255, 40));
                        }

                        cursor_x += w;
                        continue;
                    } else if tag_content.starts_with("坐标:") || tag_content.starts_with("Coord:") {
                        let coord_str = tag_content.split_once(':').map(|x| x.1).unwrap_or("");
                        let coord_color = Color::from_rgba(100, 255, 100, 255); // 绿色
                        let w = measure_text_cn(&format!("[{}]", coord_str), font_size).width;
                        let coord_rect = Rect::new(cursor_x, y - font_size + 2.0, w, font_size);

                        draw_text_cn(&format!("[{}]", coord_str), cursor_x, y, font_size, coord_color);
                        draw_line(cursor_x, y + 1.0, cursor_x + w, y + 1.0, 1.0, coord_color);

                        if coord_rect.contains(mouse_pos) {
                            draw_rectangle(cursor_x, y - font_size + 2.0, w, font_size, Color::from_rgba(100, 255, 100, 40));
                        }

                        cursor_x += w;
                        continue;
                    } else {
                        // 未知标记，原样显示
                        let full_tag = format!("[{}]", tag_content);
                        let w = measure_text_cn(&full_tag, font_size).width;
                        draw_text_cn(&full_tag, cursor_x, y, font_size, self.color);
                        cursor_x += w;
                        continue;
                    }
                } else {
                    // 没有找到 ]，把 [ 当作普通字符
                    let w = measure_text_cn("[", font_size).width;
                    draw_text_cn("[", cursor_x, y, font_size, self.color);
                    cursor_x += w;
                    continue;
                }
            }

            // 检测表情标记 :xxx:
            if ch == ':' {
                let mut emoji_name = String::new();
                let mut found_close = false;
                let mut temp_chars: Vec<char> = Vec::new();
                for c in chars.by_ref() {
                    if c == ':' {
                        found_close = true;
                        break;
                    }
                    if c.is_ascii_alphanumeric() || c == '_' {
                        emoji_name.push(c);
                    } else {
                        temp_chars.push(c);
                        break;
                    }
                }

                if found_close && !emoji_name.is_empty() {
                    // 渲染表情
                    let emoji = emoji_to_char(&emoji_name);
                    let w = measure_text_cn(emoji, font_size).width;
                    draw_text_cn(emoji, cursor_x, y, font_size, self.color);
                    cursor_x += w;

                    // 消费 temp_chars（不应该有，但如果有的话要处理）
                    for c in temp_chars {
                        let w = measure_text_cn(&c.to_string(), font_size).width;
                        draw_text_cn(&c.to_string(), cursor_x, y, font_size, self.color);
                        cursor_x += w;
                    }
                    continue;
                } else {
                    // 不是表情，输出冒号和已收集的字符
                    let w = measure_text_cn(":", font_size).width;
                    draw_text_cn(":", cursor_x, y, font_size, self.color);
                    cursor_x += w;
                    for c in temp_chars {
                        let w = measure_text_cn(&c.to_string(), font_size).width;
                        draw_text_cn(&c.to_string(), cursor_x, y, font_size, self.color);
                        cursor_x += w;
                    }
                    continue;
                }
            }

            // 普通字符
            let s = ch.to_string();
            let w = measure_text_cn(&s, font_size).width;
            draw_text_cn(&s, cursor_x, y, font_size, self.color);
            cursor_x += w;
        }

        cursor_x - x
    }

    /// 检查点击是否命中了消息中的特殊标记
    /// 返回 Some(ItemLink) 或 Some(CoordLink(x, y)) 或 None
    fn hit_test_markup(&self, text_start_x: f32, text_y: f32, font_size: f32, mouse_pos: Vec2) -> Option<ChatLink> {
        let mut cursor_x = text_start_x;
        let text = &self.text;
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '[' {
                let mut tag_content = String::new();
                let mut found_close = false;
                for c in chars.by_ref() {
                    if c == ']' {
                        found_close = true;
                        break;
                    }
                    tag_content.push(c);
                }

                if found_close {
                    if tag_content.starts_with("物品:") || tag_content.starts_with("Item:") {
                        let item_name = tag_content.split_once(':').map(|x| x.1).unwrap_or("");
                        let w = measure_text_cn(item_name, font_size).width;
                        let item_rect = Rect::new(cursor_x, text_y - font_size + 2.0, w, font_size);
                        if item_rect.contains(mouse_pos) {
                            return Some(ChatLink::Item(item_name.to_string()));
                        }
                        cursor_x += w;
                        continue;
                    } else if tag_content.starts_with("坐标:") || tag_content.starts_with("Coord:") {
                        let coord_str = tag_content.split_once(':').map(|x| x.1).unwrap_or("");
                        let display = format!("[{}]", coord_str);
                        let w = measure_text_cn(&display, font_size).width;
                        let coord_rect = Rect::new(cursor_x, text_y - font_size + 2.0, w, font_size);
                        if coord_rect.contains(mouse_pos) {
                            // 解析坐标
                            let parts: Vec<&str> = coord_str.split(',').collect();
                            if parts.len() == 2 {
                                if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<f32>(), parts[1].trim().parse::<f32>()) {
                                    return Some(ChatLink::Coord(x, y));
                                }
                            }
                        }
                        cursor_x += w;
                        continue;
                    } else {
                        let full_tag = format!("[{}]", tag_content);
                        let w = measure_text_cn(&full_tag, font_size).width;
                        cursor_x += w;
                        continue;
                    }
                } else {
                    let w = measure_text_cn("[", font_size).width;
                    cursor_x += w;
                    continue;
                }
            }

            // 跳过表情（不影响链接检测）
            if ch == ':' {
                let mut emoji_name = String::new();
                let mut found_close = false;
                for c in chars.by_ref() {
                    if c == ':' {
                        found_close = true;
                        break;
                    }
                    if !c.is_ascii_alphanumeric() && c != '_' {
                        break;
                    }
                    emoji_name.push(c);
                }
                if found_close && !emoji_name.is_empty() {
                    let w = measure_text_cn(emoji_to_char(&emoji_name), font_size).width;
                    cursor_x += w;
                    continue;
                } else {
                    let w = measure_text_cn(":", font_size).width;
                    cursor_x += w;
                    continue;
                }
            }

            let s = ch.to_string();
            let w = measure_text_cn(&s, font_size).width;
            cursor_x += w;
        }

        None
    }
}

/// 聊天消息中的可点击链接
#[derive(Debug, Clone)]
pub enum ChatLink {
    /// 物品链接，包含物品名称
    Item(String),
    /// 坐标链接，包含世界坐标 (x, y)
    Coord(f32, f32),
}

/// 将表情名称转换为对应的 Unicode 字符
fn emoji_to_char(name: &str) -> &'static str {
    match name {
        "smile" | "happy" => "😊",
        "sad" | "cry" => "😢",
        "angry" => "😡",
        "laugh" | "lol" => "😄",
        "surprised" => "😮",
        "cool" => "😎",
        "wink" => "😉",
        "love" | "heart" => "❤️",
        "star" => "⭐",
        "fire" => "🔥",
        "thumbsup" | "good" => "👍",
        "thumbsdown" | "bad" => "👎",
        _ => "?",
    }
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
    /// 滚动条背景 CountBar 纹理 (Prguse[2012/2013/2014])
    count_bar_texture: Option<Texture2D>,
    /// 滚动条滑块 PositionBar 纹理 (Prguse[2015/2016/2017])
    position_bar_textures: [Option<Texture2D>; 3],
    /// PositionBar 当前 Y（相对对话框）
    position_bar_y: f32,
    /// 是否正在拖动 PositionBar
    position_bar_dragging: bool,
    /// 拖动时鼠标到 PositionBar 顶部的偏移
    position_bar_drag_offset_y: f32,
    /// 拖拽辅助器
    #[allow(dead_code)]
    drag_helper: DragHelper,
    /// Backspace 按键重复计时器
    backspace_timer: f64,
    /// Backspace 是否在重复模式
    backspace_repeat: bool,
    /// 对话框是否有"焦点"（用于模拟 C# KeyPress 只在控件聚焦时触发）
    has_focus: bool,
    /// 是否启用透明聊天（对齐 C# Settings.TransparentChat）
    transparent_chat: bool,
    /// 聊天选项/过滤设置（对齐 C# Settings.Filter*Chat/TransparentChat）
    chat_option_settings: ChatOptionSettingsHybrid,
    /// ChatControlBar 设置的聊天前缀（对齐 C# ChatDialog.ChatPrefix）
    chat_prefix: String,
    /// 最近一次私聊前缀（对齐 C# LastPM 行为：按 '/' 预填）
    last_pm: String,
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
            count_bar_texture: None,
            position_bar_textures: [None, None, None],
            position_bar_y: 16.0,
            position_bar_dragging: false,
            position_bar_drag_offset_y: 0.0,
            drag_helper: DragHelper::new(),
            backspace_timer: 0.0,
            backspace_repeat: false,
            has_focus: false,
            transparent_chat: false,
            chat_option_settings: ChatOptionSettingsHybrid::default(),
            chat_prefix: String::new(),
            last_pm: String::new(),
        }
    }

    /// 加载演示用的测试消息（用于 debug/test 场景）
    pub fn load_demo_messages(&mut self) {
        self.messages.push(ChatMessage {
            text: "欢迎使用传奇2！:smile:".to_string(),
            color: Color::from_rgba(255, 255, 255, 255),
            timestamp: "12:00".to_string(),
            kind: ChatMessageKind::System,
        });
        self.messages.push(ChatMessage {
            text: "出售 [物品:裁决之杖] 1000金币，需要的私聊".to_string(),
            color: Color::from_rgba(255, 200, 100, 255),
            timestamp: "12:01".to_string(),
            kind: ChatMessageKind::Shout,
        });
        self.messages.push(ChatMessage {
            text: "组队刷祖玛寺庙，来 [坐标:340,280] 集合 :thumbsup:".to_string(),
            color: Color::from_rgba(100, 255, 100, 255),
            timestamp: "12:02".to_string(),
            kind: ChatMessageKind::Group,
        });
        self.messages.push(ChatMessage {
            text: "有人看到 boss 了吗？在 [坐标:150,200] :fire:".to_string(),
            color: Color::from_rgba(255, 100, 100, 255),
            timestamp: "12:03".to_string(),
            kind: ChatMessageKind::Guild,
        });
    }

    pub fn apply_chat_option_settings(&mut self, settings: ChatOptionSettingsHybrid) {
        self.transparent_chat = settings.transparent_chat;
        self.chat_option_settings = settings;
        self.clamp_scroll_offset();
    }

    fn is_message_visible(&self, kind: ChatMessageKind) -> bool {
        match kind {
            ChatMessageKind::Normal => !self.chat_option_settings.filter_normal,
            ChatMessageKind::Whisper => !self.chat_option_settings.filter_whisper,
            ChatMessageKind::Shout => !self.chat_option_settings.filter_shout,
            ChatMessageKind::System => !self.chat_option_settings.filter_system,
            ChatMessageKind::Lover => !self.chat_option_settings.filter_lover,
            ChatMessageKind::Mentor => !self.chat_option_settings.filter_mentor,
            ChatMessageKind::Group => !self.chat_option_settings.filter_group,
            ChatMessageKind::Guild => !self.chat_option_settings.filter_guild,
        }
    }

    fn visible_message_indices(&self) -> Vec<usize> {
        let mut indices = Vec::with_capacity(self.messages.len());
        for (i, msg) in self.messages.iter().enumerate() {
            if self.is_message_visible(msg.kind) {
                indices.push(i);
            }
        }
        indices
    }

    fn max_scroll_start_for_visible_count(&self, visible_count: usize) -> usize {
        if visible_count <= self.line_count {
            0
        } else {
            visible_count.saturating_sub(self.line_count)
        }
    }

    fn clamp_scroll_offset(&mut self) {
        let visible_count = self
            .messages
            .iter()
            .filter(|m| self.is_message_visible(m.kind))
            .count();
        let max_start = self.max_scroll_start_for_visible_count(visible_count);
        self.scroll_offset = self.scroll_offset.min(max_start);
    }

    /// 设置聊天前缀（由 ChatControlBar 驱动）
    pub fn set_chat_prefix(&mut self, prefix: &str) {
        if self.chat_prefix != prefix {
            self.chat_prefix.clear();
            self.chat_prefix.push_str(prefix);
        }
    }

    /// 显示对话框
    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    pub fn get_position(&self) -> Vec2 {
        self.position
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

    /// 激活输入框（用于 Enter 键打开聊天）
    pub fn activate_input(&mut self) {
        if self.visible && !self.input_active {
            self.input_active = true;
            // 启用 IME 输入法
            set_ime_enabled(true);
        }
    }

    /// 取消激活输入框
    pub fn deactivate_input(&mut self) {
        if self.input_active {
            self.input_active = false;
            // 禁用 IME 输入法
            set_ime_enabled(false);
        }
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
        self.add_message_with_kind(text, color, ChatMessageKind::Normal);
    }

    pub fn add_message_with_kind(
        &mut self,
        text: impl Into<String>,
        color: Color,
        kind: ChatMessageKind,
    ) {
        // 对齐 C#：只有当当前视图在底部时才跟随新消息自动滚动
        let visible_count_before = self
            .messages
            .iter()
            .filter(|m| self.is_message_visible(m.kind))
            .count();
        let bottom_start_before = self.max_scroll_start_for_visible_count(visible_count_before);
        let was_at_bottom = self.scroll_offset >= bottom_start_before;

        let new_visible = self.is_message_visible(kind);

        self.messages.push(ChatMessage {
            text: text.into(),
            color,
            timestamp: chrono::Local::now().format("%H:%M").to_string(),
            kind,
        });

        if was_at_bottom && new_visible {
            let visible_count_after = visible_count_before + 1;
            self.scroll_offset = self.max_scroll_start_for_visible_count(visible_count_after);
        } else {
            self.clamp_scroll_offset();
        }
    }

    /// 切换窗口大小
    pub fn change_size(&mut self, _screen_height: f32) {
        // 对齐 C#：保持 DisplayRectangle.Bottom 不变，切换 Index 后用新 Size.Height 反推 Location.Y
        let bottom = self.position.y + self.current_size.y;

        self.window_size = (self.window_size + 1) % 3;
        self.line_count = match self.window_size {
            0 => 4,
            1 => 7,
            2 => 11,
            _ => 4,
        };

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
            self.bg_texture = texture.image;
        }

        self.position.y = bottom - self.current_size.y;
        self.clamp_scroll_offset();
    }

    /// 异步加载纹理
    pub  fn load_textures(&mut self) {
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

        // 简单焦点：点击聊天框获得焦点；点击外部失去焦点（输入框激活时不强制失焦）
        if is_mouse_button_pressed(MouseButton::Left) {
            if self.contains(mouse_pos) {
                self.has_focus = true;
            } else if !self.input_active {
                self.has_focus = false;
            }
        }

        // 对齐 C# KeyPress：当对话框聚焦（或鼠标悬停）且未激活输入框时，按 '@'/'!' 直接打开输入框
        if !self.input_active && (self.has_focus || self.contains(mouse_pos)) {
            while let Some(ch) = get_char_pressed() {
                if ch == '@' || ch == '!' {
                    self.activate_input();
                    if !self.chat_prefix.is_empty() {
                        self.input_text = self.chat_prefix.clone();
                    } else {
                        self.input_text = ch.to_string();
                    }
                    break;
                }
            }
        }

        // 对齐 C#：未激活输入框时，按 Enter/Space 打开输入；按 '/' 预填上一次私聊目标
        if !self.input_active {
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                self.activate_input();
                if !self.chat_prefix.is_empty() {
                    self.input_text = self.chat_prefix.clone();
                }
            } else if is_key_pressed(KeyCode::Slash) {
                self.activate_input();
                if !self.last_pm.is_empty() {
                    self.input_text = format!("{} ", self.last_pm);
                } else {
                    self.input_text = "/".to_string();
                }
            }
        }

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

        // CountBar (对齐 C#：WindowSize 对应 2012/2013/2014)
        let count_bar_index = match self.window_size {
            0 => 2012,
            1 => 2013,
            2 => 2014,
            _ => 2012,
        };
        if let Some(texture) = LibraryName::Prguse.get_texture(count_bar_index) {
            self.count_bar_texture = texture.image;
        }

        // PositionBar (2015/2016/2017)
        for (i, idx) in [2015usize, 2016, 2017].iter().enumerate() {
            if self.position_bar_textures[i].is_none() {
                if let Some(texture) = LibraryName::Prguse.get_texture(*idx) {
                    self.position_bar_textures[i] = texture.image;
                }
            }
        }

        // C# 没有 Movable=true；这里暂时不启用拖拽，避免与原版行为不一致

        // 绘制背景
        self.draw_background();

        // 绘制消息（含点击行快速私聊）
        self.draw_messages(mouse_pos);

        // 绘制滚动条
        self.draw_scroll_buttons(mouse_pos);

        // 绘制输入框
        // 之前仅在 input_active 或已有文本时绘制，会导致无法点击激活（看不到光标/无法输入）。
        // 这里保持输入框始终可见：未激活时无光标，点击后激活并显示光标。
        self.draw_input();
    }

    /// 绘制背景
    fn draw_background(&self) {
        let bg_color = if self.transparent_chat {
            Color::new(1.0, 1.0, 1.0, 0.8)
        } else {
            WHITE
        };

        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                bg_color,
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
                if self.transparent_chat {
                    Color::new(0.0, 0.0, 0.0, 0.15)
                } else {
                    WHITE
                },
            );
        } else {
            // 降级
            let default_width = if self.resolution_index == 0 { 403.0 } else { 627.0 };
            draw_rectangle(
                self.position.x,
                self.position.y,
                default_width,
                70.0,
                if self.transparent_chat {
                    Color::from_rgba(50, 50, 50, 200)
                } else {
                    Color::from_rgba(50, 50, 50, 255)
                },
            );
        }
    }

    /// 绘制消息（使用中文字体，支持物品/坐标链接和表情）
    fn draw_messages(&mut self, mouse_pos: Vec2) {
        let msg_x = self.position.x + 8.0;
        let msg_y = self.position.y + 7.0;
        let line_height = 14.0;
        let font_size = 12.0;

        let msg_width = if self.resolution_index == 0 { 380.0 } else { 600.0 };

        let visible_indices = self.visible_message_indices();
        let visible_count = visible_indices.len();
        let max_start = self.max_scroll_start_for_visible_count(visible_count);
        let start_row = self.scroll_offset.min(max_start);
        let end_row = (start_row + self.line_count).min(visible_count);

        // 收集所有行的链接点击结果
        let mut clicked_link: Option<ChatLink> = None;

        for (i, row) in (start_row..end_row).enumerate() {
            let msg_idx = visible_indices[row];
            let msg = &self.messages[msg_idx];
            let y = msg_y + (i as f32 * line_height) + 12.0;

            // 使用 markup 渲染
            msg.draw_with_markup(msg_x, y, font_size, mouse_pos, Rect::new(msg_x, msg_y + (i as f32 * line_height), msg_width, line_height));

            // 检测链接点击
            if is_mouse_button_pressed(MouseButton::Left) {
                let line_rect = Rect::new(msg_x, msg_y + (i as f32 * line_height), msg_width, line_height);
                if line_rect.contains(mouse_pos) {
                    if let Some(link) = msg.hit_test_markup(msg_x, y, font_size, mouse_pos) {
                        clicked_link = Some(link);
                    }
                }
            }
        }

        // 处理链接点击
        match clicked_link {
            Some(ChatLink::Item(ref name)) => {
                println!("🔗 点击物品链接: {}", name);
                // 可以触发物品详情弹窗
            }
            Some(ChatLink::Coord(x, y)) => {
                println!("🔗 点击坐标链接: ({}, {})", x, y);
                // 可以触发自动寻路
            }
            None => {}
        }

        // 对齐 C#：点击聊天行，自动打开输入框并预填私聊"/name "
        // 只有在没有点击链接的情况下才触发私聊
        if clicked_link.is_none() && is_mouse_button_pressed(MouseButton::Left) {
            let rel_y = mouse_pos.y - msg_y;
            if rel_y >= 0.0 {
                let line_i = (rel_y / line_height).floor() as i32;
                if line_i >= 0 && (line_i as usize) < self.line_count {
                    let row = start_row + line_i as usize;
                    if row < visible_count {
                        let line_rect = Rect::new(msg_x, msg_y + (line_i as f32 * line_height), msg_width, line_height);
                        if line_rect.contains(mouse_pos) {
                            let msg_idx = visible_indices[row];
                            let clicked_text = self.messages[msg_idx].text.clone();
                            let mut name_part = clicked_text
                                .split([':', ' '])
                                .next()
                                .unwrap_or("")
                                .to_string();
                            name_part.retain(|c| c.is_ascii_alphanumeric());
                            if !name_part.is_empty() {
                                self.activate_input();
                                self.input_text = format!("/{} ", name_part);
                            }
                        }
                    }
                }
            }
        }
    }

    /// 绘制滚动条按钮
    fn draw_scroll_buttons(&mut self, mouse_pos: Vec2) {
        let scroll_x = if self.resolution_index == 0 { 394.0 } else { 618.0 };

        let visible_count = self
            .messages
            .iter()
            .filter(|m| self.is_message_visible(m.kind))
            .count();
        let max_start = self.max_scroll_start_for_visible_count(visible_count);
        self.scroll_offset = self.scroll_offset.min(max_start);

        // Home 按钮
        if self.draw_scroll_button(mouse_pos, scroll_x, 1.0, 2018, 2019, 2020) {
            self.scroll_offset = 0;
        }

        // Up 按钮
        if self.draw_scroll_button(mouse_pos, scroll_x, 9.0, 2021, 2022, 2023)
            && self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }

        // Down 按钮
        let down_y = match self.window_size {
            0 => 39.0,
            1 => 39.0 + 48.0,
            2 => 39.0 + 96.0,
            _ => 39.0,
        };
        if self.draw_scroll_button(mouse_pos, scroll_x, down_y, 2024, 2025, 2026) {
            self.scroll_offset = (self.scroll_offset + 1).min(max_start);
        }

        // End 按钮
        let end_y = match self.window_size {
            0 => 45.0,
            1 => 45.0 + 48.0,
            2 => 45.0 + 96.0,
            _ => 45.0,
        };
        if self.draw_scroll_button(mouse_pos, scroll_x, end_y, 2027, 2028, 2029) {
            self.scroll_offset = max_start;
        }

        // CountBar / PositionBar（对齐 C#：CountBar at (622/398,16)，PositionBar at (619/395,16+offset)）
        let count_x = if self.resolution_index == 0 { 398.0 } else { 622.0 };
        let pos_x = if self.resolution_index == 0 { 395.0 } else { 619.0 };
        let bar_y = 16.0;

        if let Some(ref count_tex) = self.count_bar_texture {
            draw_texture_ex(
                count_tex,
                self.position.x + count_x,
                self.position.y + bar_y,
                WHITE,
                DrawTextureParams::default(),
            );

            let count_h = count_tex.height();
            let (pos_w, pos_h) = self
                .position_bar_textures[0]
                .as_ref()
                .map(|t| (t.width(), t.height()))
                .unwrap_or((12.0, 12.0));

            // 如果不在拖动，则按当前 scroll_offset 更新滑块位置
            let positions = max_start + 1;
            if !self.position_bar_dragging && positions > 1 {
                let h = (count_h - pos_h).max(0.0);
                let step = h / (positions as f32 - 1.0);
                self.position_bar_y = bar_y + step * (self.scroll_offset as f32);
            }

            let pos_rect = Rect::new(
                self.position.x + pos_x,
                self.position.y + self.position_bar_y,
                pos_w,
                pos_h,
            );
            let hovered = pos_rect.contains(mouse_pos);
            let pressed = hovered && is_mouse_button_down(MouseButton::Left);

            // 开始拖动
            if hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.position_bar_dragging = true;
                self.position_bar_drag_offset_y = mouse_pos.y - pos_rect.y;
            }

            // 拖动更新
            if self.position_bar_dragging {
                if is_mouse_button_down(MouseButton::Left) {
                    let min_y = self.position.y + bar_y;
                    let max_y = self.position.y + bar_y + (count_h - pos_h);
                    let new_y = (mouse_pos.y - self.position_bar_drag_offset_y).clamp(min_y, max_y);
                    self.position_bar_y = new_y - self.position.y;

                    let positions = max_start + 1;
                    if positions > 1 {
                        let h = (count_h - pos_h).max(0.0);
                        let step = if h > 0.0 { h / (positions as f32 - 1.0) } else { 1.0 };
                        let idx = ((self.position_bar_y - bar_y) / step).floor() as i32;
                        let idx = idx.clamp(0, max_start as i32);
                        self.scroll_offset = idx as usize;
                    }
                } else {
                    self.position_bar_dragging = false;
                }
            }

            // 绘制 PositionBar
            let tex = if pressed {
                self.position_bar_textures[2].as_ref().or(self.position_bar_textures[0].as_ref())
            } else if hovered {
                self.position_bar_textures[1].as_ref().or(self.position_bar_textures[0].as_ref())
            } else {
                self.position_bar_textures[0].as_ref()
            };
            if let Some(tex) = tex {
                draw_texture_ex(
                    tex,
                    pos_rect.x,
                    pos_rect.y,
                    WHITE,
                    DrawTextureParams::default(),
                );
            }
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
            if scroll_delta > 0.0 {
                // C#：StartIndex -= count
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            } else if scroll_delta < 0.0 {
                self.scroll_offset = (self.scroll_offset + 1).min(max_start);
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
            // 光标闪烁（对齐常见输入框体验）
            let blink_on = ((get_time() * 2.0) as i64 % 2) == 0;
            if blink_on {
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
        }

        // 检测点击激活
        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        if input_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left) {
            if !self.input_active {
                self.input_active = true;
                // 启用 IME 输入法
                set_ime_enabled(true);
                // 设置 IME 候选窗口位置到输入框下方
                let dpi = miniquad::window::dpi_scale();
                let ime_x = (input_rect.x * dpi) as i32;
                let ime_y = ((input_rect.y + input_rect.h + 2.0) * dpi) as i32;
                set_ime_position(ime_x, ime_y);
            }
        } else if !input_rect.contains(mouse_pos) && is_mouse_button_pressed(MouseButton::Left)
            && self.input_active {
                self.input_active = false;
                // 禁用 IME 输入法（用于游戏控制）
                set_ime_enabled(false);
            }

        // 处理键盘输入（支持中文）
        if self.input_active {
            // 每帧更新 IME 位置（输入框可能被拖动，或光标位置变化）
            let cursor_x = input_rect.x + 3.0 + measure_text_cn(&self.input_text, 14.0).width;
            let dpi = miniquad::window::dpi_scale();
            let ime_x = (cursor_x * dpi) as i32;
            let ime_y = ((input_rect.y + input_rect.h + 2.0) * dpi) as i32;
            set_ime_position(ime_x, ime_y);
            
            // 获取输入的字符（支持中文和其他Unicode字符）
            // 注意：macroquad 的 get_char_pressed() 在同一帧内可能以"后进先出"顺序吐出多个字符，
            // 这会导致 IME 一次性提交的文本显示为倒序。这里先收集后再反向追加，保证显示顺序正确。
            let mut pending_chars: Vec<char> = Vec::new();
            while let Some(ch) = get_char_pressed() {
                if !ch.is_control() {
                    pending_chars.push(ch);
                }
            }
            for ch in pending_chars.into_iter().rev() {
                self.input_text.push(ch);
            }
            
            // Ctrl+V 粘贴（支持中文输入的备用方案）
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::V)
            {
                if let Some(clipboard_text) = miniquad::window::clipboard_get() {
                    // 过滤控制字符，保留中文和其他可打印字符
                    for ch in clipboard_text.chars() {
                        if !ch.is_control() {
                            self.input_text.push(ch);
                        }
                    }
                }
            }
            
            // Enter 发送消息
            if is_key_pressed(KeyCode::Enter) && !self.input_text.is_empty() {
                let message = self.input_text.clone();
                println!("📤 发送聊天: {}", message);
                self.add_message(format!("[我] {}", message), WHITE);

                // 记录私聊前缀（用于下次按 '/' 预填）
                if message.starts_with('/') {
                    if let Some(first) = message.split_whitespace().next() {
                        if first.len() > 1 {
                            self.last_pm = first.to_string();
                        }
                    }
                }

                // 设置待发送消息（由 ui_system 消费并发送网络事件）
                *pending_chat().lock().unwrap() = Some(message);

                self.input_text.clear();
            }
            // Escape 取消输入
            if is_key_pressed(KeyCode::Escape) {
                self.input_text.clear();
                self.input_active = false;
                // 禁用 IME 输入法
                set_ime_enabled(false);
            }
            // Backspace 删除字符（支持按住连续删除）
            if is_key_down(KeyCode::Backspace) {
                let now = get_time();
                if is_key_pressed(KeyCode::Backspace) {
                    // 首次按下，立即删除一个字符
                    if !self.input_text.is_empty() {
                        self.input_text.pop();
                    }
                    self.backspace_timer = now;
                    self.backspace_repeat = false;
                } else {
                    // 按住状态
                    let delay = if self.backspace_repeat { 0.03 } else { 0.4 }; // 首次延迟 400ms，之后 30ms
                    if now - self.backspace_timer > delay {
                        if !self.input_text.is_empty() {
                            self.input_text.pop();
                        }
                        self.backspace_timer = now;
                        self.backspace_repeat = true;
                    }
                }
            } else {
                self.backspace_repeat = false;
            }
        }
    }
}
