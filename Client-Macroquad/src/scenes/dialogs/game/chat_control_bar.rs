// ============================================================================
// ChatControlBarHybrid - 聊天控制栏（混合版本）
// ============================================================================
//
// 【功能说明】
// 1. 聊天频道切换按钮（全体、喊话、私聊、夫妻、师徒、组队、行会）
// 2. 功能按钮（大小调整、设置、交易、举报）
// 3. 显示当前选中的聊天频道
//
// 【实现方式】
// - 使用 macroquad 原生 draw_* 函数绘制
// - 不需要拖拽（控制栏跟随 ChatDialog）
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;

/// 聊天频道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFilterHybrid {
    All,     // 全部
    Shout,   // 喊话 (!)
    Whisper, // 私聊 (/)
    Lover,   // 夫妻 (:))
    Mentor,  // 师徒 (!#)
    Group,   // 组队 (!!)
    Guild,   // 行会 (!~)
}

impl ChatFilterHybrid {
    /// 获取聊天前缀
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::All => "",
            Self::Shout => "!",
            Self::Whisper => "/",
            Self::Lover => ":)",
            Self::Mentor => "!#",
            Self::Group => "!!",
            Self::Guild => "!~",
        }
    }

    /// 获取显示名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Shout => "喊话",
            Self::Whisper => "私聊",
            Self::Lover => "夫妻",
            Self::Mentor => "师徒",
            Self::Group => "组队",
            Self::Guild => "行会",
        }
    }
}

/// 聊天控制栏（混合版本）
pub struct ChatControlBarHybrid {
    /// 分辨率索引
    resolution_index: usize,
    /// 位置
    position: Vec2,
    /// 是否可见
    visible: bool,
    /// 当前选中的聊天频道
    active_filter: ChatFilterHybrid,
    /// 是否显示行会聊天（对齐 C# Settings.ShowGuildChat）
    show_guild_chat: bool,
    /// 是否显示举报按钮（对齐 C#：默认 false）
    report_visible: bool,
    /// 背景纹理
    bg_texture: Option<Texture2D>,
    /// 当前尺寸
    current_size: Vec2,
}

impl ChatControlBarHybrid {
    /// 创建聊天控制栏
    pub fn new(main_dialog_x: f32, screen_height: f32, resolution_index: usize) -> Self {
        // 位置：MainDialog.X + 230, ScreenHeight - 112
        let position = vec2(main_dialog_x + 230.0, screen_height - 112.0);
        
        // 默认尺寸
        let default_width = if resolution_index == 0 { 372.0 } else { 596.0 };

        Self {
            resolution_index,
            position,
            visible: true,
            active_filter: ChatFilterHybrid::All,
            show_guild_chat: true,
            report_visible: false,
            bg_texture: None,
            current_size: vec2(default_width, 15.0),
        }
    }

    /// 显示控制栏
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// 隐藏控制栏
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 设置位置（当 ChatDialog 改变大小时需要同步更新）
    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    /// 获取位置
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn get_size(&self) -> Vec2 {
        self.current_size
    }

    /// 切换聊天频道
    pub fn set_filter(&mut self, filter: ChatFilterHybrid) {
        self.active_filter = filter;
    }

    /// 获取当前聊天前缀
    pub fn get_chat_prefix(&self) -> &'static str {
        self.active_filter.prefix()
    }

    /// 获取当前频道
    pub fn get_active_filter(&self) -> ChatFilterHybrid {
        self.active_filter
    }

    /// 检查点是否在控制栏内
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        point.x >= self.position.x
            && point.x <= self.position.x + self.current_size.x
            && point.y >= self.position.y
            && point.y <= self.position.y + self.current_size.y
    }

    /// 异步加载纹理
    pub  fn load_textures(&mut self) {
        // 预加载背景纹理
        let bg_index = if self.resolution_index == 0 { 2035 } else { 2034 };
        if let Some(texture) = LibraryName::Prguse.get_texture(bg_index) {
            self.current_size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 预加载按钮纹理
        let button_indices = [
            2036, 2037, 2038, // All
            2039, 2040, 2041, // Shout
            2042, 2043, 2044, // Whisper
            2045, 2046, 2047, // Lover
            2048, 2049, 2050, // Mentor
            2051, 2052, 2053, // Group
            2054, 2055, 2056, // Guild
            2004, 2005, 2006, // Trade
            2057, 2058, 2059, // Size
            2060, 2061, 2062, // Settings
            2063, 2064, 2065, // Report
        ];
        for idx in button_indices {
            let _ = LibraryName::Prguse.get_texture(idx);
        }
    }

    /// 更新和绘制
    /// 返回：(size_button_clicked, settings_button_clicked)
    pub fn update_and_draw(&mut self) -> (bool, bool) {
        if !self.visible {
            return (false, false);
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 获取背景纹理
        let bg_index = if self.resolution_index == 0 { 2035 } else { 2034 };
        if let Some(texture) = LibraryName::Prguse.get_texture(bg_index) {
            self.current_size = vec2(texture.width as f32, texture.height as f32);
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
            }
        }

        // 绘制背景
        self.draw_background();

        // 绘制频道选择按钮
        self.draw_filter_buttons(mouse_pos);

        // 绘制功能按钮
        let (size_clicked, settings_clicked) = self.draw_function_buttons(mouse_pos);

        (size_clicked, settings_clicked)
    }

    /// 绘制背景
    fn draw_background(&self) {
        if let Some(ref texture) = self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        } else {
            // 降级：绘制临时背景
            draw_rectangle(
                self.position.x,
                self.position.y,
                self.current_size.x,
                self.current_size.y,
                Color::from_rgba(50, 50, 50, 200),
            );
        }
    }

    /// 绘制频道选择按钮
    fn draw_filter_buttons(&mut self, mouse_pos: Vec2) {
        // 按钮配置：(x_offset, 频道, 基础纹理索引)
        let button_configs = [
            (12.0, ChatFilterHybrid::All, 2036usize),
            (34.0, ChatFilterHybrid::Shout, 2039usize),
            (56.0, ChatFilterHybrid::Whisper, 2042usize),
            (78.0, ChatFilterHybrid::Lover, 2045usize),
            (100.0, ChatFilterHybrid::Mentor, 2048usize),
            (122.0, ChatFilterHybrid::Group, 2051usize),
            (144.0, ChatFilterHybrid::Guild, 2054usize),
        ];

        for (x_offset, filter, base_index) in button_configs {
            let is_selected = filter == self.active_filter;
            if self.draw_button(mouse_pos, x_offset, 1.0, base_index, is_selected) {
                if filter == ChatFilterHybrid::Guild {
                    self.show_guild_chat = !self.show_guild_chat;
                }
                self.active_filter = filter;
            }
        }

        // TradeButton - 位置固定 (166, 1)
        self.draw_button(mouse_pos, 166.0, 1.0, 2004, false);

        // ReportButton - 对齐 C#：默认 Visible=false
        if self.report_visible {
            let report_x = if self.resolution_index != 0 { 552.0 } else { 328.0 };
            self.draw_button(mouse_pos, report_x, 1.0, 2063, false);
        }
    }

    /// 绘制功能按钮
    /// 返回：(size_button_clicked, settings_button_clicked)
    fn draw_function_buttons(&self, mouse_pos: Vec2) -> (bool, bool) {
        // SizeButton - 位置根据分辨率变化
        let size_btn_x = if self.resolution_index != 0 { 574.0 } else { 350.0 };
        let size_clicked = self.draw_button(mouse_pos, size_btn_x, 1.0, 2057, false);

        // SettingsButton - 位置根据分辨率变化
        let settings_btn_x = if self.resolution_index != 0 { 596.0 } else { 372.0 };
        let settings_clicked = self.draw_button(mouse_pos, settings_btn_x, 1.0, 2060, false);

        (size_clicked, settings_clicked)
    }

    /// 绘制可点击按钮（返回是否被点击）
    fn draw_button(&self, mouse_pos: Vec2, x_offset: f32, y_offset: f32, base_index: usize, is_selected: bool) -> bool {
        let btn_pos = vec2(self.position.x + x_offset, self.position.y + y_offset);

        // 获取按钮纹理尺寸
        let btn_size = if let Some(texture) = LibraryName::Prguse.get_texture(base_index) {
            vec2(texture.width as f32, texture.height as f32)
        } else {
            vec2(20.0, 13.0) // 默认按钮尺寸
        };

        let btn_rect = Rect::new(btn_pos.x, btn_pos.y, btn_size.x, btn_size.y);
        let is_hovered = btn_rect.contains(mouse_pos);
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

        // 根据状态选择纹理索引
        let texture_idx = if is_selected || is_pressed {
            base_index + 2 // 已选中/按下状态
        } else if is_hovered {
            base_index + 1 // 悬停状态
        } else {
            base_index // 正常状态
        };

        // 绘制按钮
        if let Some(texture) = LibraryName::Prguse.get_texture(texture_idx) {
            if let Some(ref tex) = texture.image {
                draw_texture_ex(
                    tex,
                    btn_pos.x,
                    btn_pos.y,
                    WHITE,
                    DrawTextureParams::default(),
                );
            }
        } else {
            // 降级：绘制临时按钮
            let color = if is_selected || is_pressed {
                Color::from_rgba(100, 150, 100, 255)
            } else if is_hovered {
                Color::from_rgba(80, 80, 100, 255)
            } else {
                Color::from_rgba(60, 60, 70, 255)
            };
            draw_rectangle(btn_pos.x, btn_pos.y, btn_size.x, btn_size.y, color);
        }

        // 返回是否被点击
        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }
}
