// ============================================================================
// ChatOptionDialogHybrid - 聊天设置/过滤对话框（混合版本）
// ============================================================================
//
// 【C# 原版参考】
// - Client/MirScenes/Dialogs/ChatOptionDialog.cs
// - 背景: Title[466] (Filter Tab), Title[467] (Chat Tab)
// - Size: 224x180
// - Tabs:
//   - FilterTabButton: Title[463]/Pressed[462] at (8,8)
//   - ChatTabButton:   Title[464]/Pressed[465] at (78,8)
// - CloseButton: Prguse2[360/361/362] at (198,3)
// - Filter buttons (Prguse):
//   - All: 2087/2086 at (74,47)
//   - General: 2071/2070 at (40,69)
//   - Whisper: 2075/2074 at (40,92)
//   - Shout: 2073/2072 at (40,115)
//   - System: 2085/2084 at (40,138)
//   - Lover: 2077/2076 at (135,69)
//   - Mentor: 2079/2078 at (135,92)
//   - Group: 2081/2080 at (135,115)
//   - Guild: 2083/2082 at (135,138)
// - Transparency buttons (Title):
//   - Off: Index 471 Hover 472 Pressed 470 at (45,90)
//   - On:  Index 474 Hover 475 Pressed 473 at (115,90)
//
// ============================================================================

use macroquad::prelude::*;

use crate::resources::LibraryName;

use super::native_ui_utils::DragHelper;

#[derive(Debug, Clone, Copy, Default)]
pub struct ChatOptionSettingsHybrid {
    // 与 C# Settings.Filter*Chat 语义一致：true = 过滤(隐藏)该类别
    pub filter_normal: bool,
    pub filter_whisper: bool,
    pub filter_shout: bool,
    pub filter_system: bool,
    pub filter_lover: bool,
    pub filter_mentor: bool,
    pub filter_group: bool,
    pub filter_guild: bool,
    pub transparent_chat: bool,
}

pub struct ChatOptionDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,

    // 对齐 C#：Hide/Show 不会重置 Location；仅首次显示时做一次默认居中。
    position_initialized: bool,

    // 0 = Filter, 1 = Chat
    tab: u8,

    // C# 的 AllFiltersOff
    all_filters_off: bool,
    settings: ChatOptionSettingsHybrid,

    // 背景纹理 Title[466/467]
    bg_textures: [Option<Texture2D>; 2],
    // Close Prguse2[360/361/362]
    close_textures: [Option<Texture2D>; 3],

    drag_helper: DragHelper,
}

impl Default for ChatOptionDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatOptionDialogHybrid {
    pub fn new() -> Self {
        Self {
            position: vec2(300.0, 200.0),
            visible: false,
            size: vec2(224.0, 180.0),
            position_initialized: false,
            tab: 0,
            all_filters_off: true,
            settings: ChatOptionSettingsHybrid::default(),
            bg_textures: [None, None],
            close_textures: [None, None, None],
            drag_helper: DragHelper::new(),
        }
    }

    pub fn open(&mut self) {
        if !self.position_initialized {
            let dpi = screen_dpi_scale();
            let screen_w = screen_width() / dpi;
            let screen_h = screen_height() / dpi;

            self.position = vec2(
                (screen_w - self.size.x) / 2.0,
                (screen_h - self.size.y) / 2.0,
            );

            // 保险：避免出屏
            self.position.x = self
                .position
                .x
                .clamp(0.0, (screen_w - self.size.x).max(0.0));
            self.position.y = self
                .position
                .y
                .clamp(0.0, (screen_h - self.size.y).max(0.0));

            self.position_initialized = true;
        }

        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    pub fn get_settings(&self) -> ChatOptionSettingsHybrid {
        self.settings
    }

    pub fn load_textures(&mut self) {
        // 背景
        for (i, idx) in [466usize, 467].iter().enumerate() {
            if let Some(texture) = LibraryName::Title.get_texture(*idx) {
                self.size = vec2(texture.width as f32, texture.height as f32);
                self.bg_textures[i] = texture.image;
            }
        }

        // Close button
        for (i, idx) in [360usize, 361, 362].iter().enumerate() {
            if let Some(texture) = LibraryName::Prguse2.get_texture(*idx) {
                self.close_textures[i] = texture.image;
            }
        }

        // 预加载 Tab 与透明按钮（Title）
        for idx in [462usize, 463, 464, 465, 470, 471, 472, 473, 474, 475] {
            let _ = LibraryName::Title.get_texture(idx);
        }

        // 预加载过滤按钮（Prguse）
        for idx in [
            2086usize, 2087, // all
            2070, 2071, // general
            2074, 2075, // whisper
            2072, 2073, // shout
            2084, 2085, // system
            2076, 2077, // lover
            2078, 2079, // mentor
            2080, 2081, // group
            2082, 2083, // guild
        ] {
            let _ = LibraryName::Prguse.get_texture(idx);
        }
    }

    /// 更新并绘制
    /// 返回是否有设置变更（用于外层同步）
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut changed = false;

        // 拖拽区域：顶部 30px
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 背景
        self.draw_background();

        // Tabs
        if self.draw_tabs(mouse_pos) {
            changed = true;
        }

        // Close
        if self.draw_close(mouse_pos) {
            self.close();
        }

        // Content
        match self.tab {
            0 => {
                if self.draw_filter_tab(mouse_pos) {
                    changed = true;
                }
            }
            1 => {
                if self.draw_chat_tab(mouse_pos) {
                    changed = true;
                }
            }
            _ => {}
        }

        changed
    }

    fn draw_background(&self) {
        let bg = match self.tab {
            0 => self.bg_textures[0].as_ref(),
            1 => self.bg_textures[1].as_ref(),
            _ => self.bg_textures[0].as_ref(),
        };

        if let Some(tex) = bg {
            draw_texture_ex(
                tex,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }
    }

    fn draw_tabs(&mut self, mouse_pos: Vec2) -> bool {
        let mut changed = false;

        // 对齐 C# SwitchTab：不同 tab 下 Index/PressedIndex 交换
        // FilterTab at (8,8)
        let (filter_idx, filter_pressed) = if self.tab == 0 {
            (463, 462)
        } else {
            (462, 463)
        };
        if self.draw_title_button(mouse_pos, 8.0, 8.0, filter_idx, None, filter_pressed)
            && self.tab != 0
        {
            self.tab = 0;
            changed = true;
        }

        // ChatTab at (78,8)
        let (chat_idx, chat_pressed) = if self.tab == 0 {
            (464, 465)
        } else {
            (465, 464)
        };
        if self.draw_title_button(mouse_pos, 78.0, 8.0, chat_idx, None, chat_pressed)
            && self.tab != 1
        {
            self.tab = 1;
            changed = true;
        }

        changed
    }

    fn draw_close(&self, mouse_pos: Vec2) -> bool {
        let x = self.position.x + 198.0;
        let y = self.position.y + 3.0;

        let (w, h) = self.close_textures[0]
            .as_ref()
            .map(|t| (t.width(), t.height()))
            .unwrap_or((24.0, 21.0));

        let rect = Rect::new(x, y, w, h);
        let hovered = rect.contains(mouse_pos);
        let pressed = hovered && is_mouse_button_down(MouseButton::Left);

        let tex = if pressed {
            self.close_textures[2]
                .as_ref()
                .or(self.close_textures[0].as_ref())
        } else if hovered {
            self.close_textures[1]
                .as_ref()
                .or(self.close_textures[0].as_ref())
        } else {
            self.close_textures[0].as_ref()
        };

        if let Some(tex) = tex {
            draw_texture_ex(tex, x, y, WHITE, DrawTextureParams::default());
        }

        hovered && is_mouse_button_pressed(MouseButton::Left)
    }

    fn draw_filter_tab(&mut self, mouse_pos: Vec2) -> bool {
        let mut changed = false;

        // All button
        let all_idx = if self.all_filters_off { 2087 } else { 2086 };
        if self.draw_prguse_toggle_button(mouse_pos, 74.0, 47.0, all_idx) {
            self.toggle_all_filters();
            changed = true;
        }

        // Left column
        if self.draw_prguse_toggle_button(
            mouse_pos,
            40.0,
            69.0,
            if self.settings.filter_normal {
                2070
            } else {
                2071
            },
        ) {
            self.settings.filter_normal = !self.settings.filter_normal;
            self.check_all_filters();
            changed = true;
        }

        if self.draw_prguse_toggle_button(
            mouse_pos,
            40.0,
            92.0,
            if self.settings.filter_whisper {
                2074
            } else {
                2075
            },
        ) {
            self.settings.filter_whisper = !self.settings.filter_whisper;
            self.check_all_filters();
            changed = true;
        }

        if self.draw_prguse_toggle_button(
            mouse_pos,
            40.0,
            115.0,
            if self.settings.filter_shout {
                2072
            } else {
                2073
            },
        ) {
            self.settings.filter_shout = !self.settings.filter_shout;
            self.check_all_filters();
            changed = true;
        }

        if self.draw_prguse_toggle_button(
            mouse_pos,
            40.0,
            138.0,
            if self.settings.filter_system {
                2084
            } else {
                2085
            },
        ) {
            self.settings.filter_system = !self.settings.filter_system;
            self.check_all_filters();
            changed = true;
        }

        // Right column
        if self.draw_prguse_toggle_button(
            mouse_pos,
            135.0,
            69.0,
            if self.settings.filter_lover {
                2076
            } else {
                2077
            },
        ) {
            self.settings.filter_lover = !self.settings.filter_lover;
            self.check_all_filters();
            changed = true;
        }

        if self.draw_prguse_toggle_button(
            mouse_pos,
            135.0,
            92.0,
            if self.settings.filter_mentor {
                2078
            } else {
                2079
            },
        ) {
            self.settings.filter_mentor = !self.settings.filter_mentor;
            self.check_all_filters();
            changed = true;
        }

        if self.draw_prguse_toggle_button(
            mouse_pos,
            135.0,
            115.0,
            if self.settings.filter_group {
                2080
            } else {
                2081
            },
        ) {
            self.settings.filter_group = !self.settings.filter_group;
            self.check_all_filters();
            changed = true;
        }

        if self.draw_prguse_toggle_button(
            mouse_pos,
            135.0,
            138.0,
            if self.settings.filter_guild {
                2082
            } else {
                2083
            },
        ) {
            self.settings.filter_guild = !self.settings.filter_guild;
            self.check_all_filters();
            changed = true;
        }

        changed
    }

    fn draw_chat_tab(&mut self, mouse_pos: Vec2) -> bool {
        let mut changed = false;

        // Off button visual indices depend on state (对齐 C# UpdateTransparency)
        let (off_idx, off_hover) = if self.settings.transparent_chat {
            (470, 470)
        } else {
            (471, 472)
        };
        if self.draw_title_button(mouse_pos, 45.0, 90.0, off_idx, Some(off_hover), 470)
            && self.settings.transparent_chat
        {
            self.settings.transparent_chat = false;
            changed = true;
        }

        // On button visual indices
        let (on_idx, on_hover) = if self.settings.transparent_chat {
            (474, 475)
        } else {
            (473, 473)
        };
        if self.draw_title_button(mouse_pos, 115.0, 90.0, on_idx, Some(on_hover), 473)
            && !self.settings.transparent_chat
        {
            self.settings.transparent_chat = true;
            changed = true;
        }

        changed
    }

    fn toggle_all_filters(&mut self) {
        if self.all_filters_off {
            self.settings.filter_normal = true;
            self.settings.filter_whisper = true;
            self.settings.filter_shout = true;
            self.settings.filter_system = true;
            self.settings.filter_lover = true;
            self.settings.filter_mentor = true;
            self.settings.filter_group = true;
            self.settings.filter_guild = true;
        } else {
            self.settings.filter_normal = false;
            self.settings.filter_whisper = false;
            self.settings.filter_shout = false;
            self.settings.filter_system = false;
            self.settings.filter_lover = false;
            self.settings.filter_mentor = false;
            self.settings.filter_group = false;
            self.settings.filter_guild = false;
        }

        self.all_filters_off = !self.all_filters_off;
    }

    fn check_all_filters(&mut self) {
        self.all_filters_off = !self.settings.filter_normal
            && !self.settings.filter_whisper
            && !self.settings.filter_shout
            && !self.settings.filter_system
            && !self.settings.filter_lover
            && !self.settings.filter_mentor
            && !self.settings.filter_group
            && !self.settings.filter_guild;
    }

    fn draw_prguse_toggle_button(&self, mouse_pos: Vec2, x: f32, y: f32, idx: usize) -> bool {
        let pos = vec2(self.position.x + x, self.position.y + y);
        if let Some(texture) = LibraryName::Prguse.get_texture(idx) {
            let (w, h) = (texture.width as f32, texture.height as f32);
            let rect = Rect::new(pos.x, pos.y, w, h);
            let hovered = rect.contains(mouse_pos);

            if let Some(tex) = texture.image.as_ref() {
                draw_texture_ex(tex, pos.x, pos.y, WHITE, DrawTextureParams::default());
            }

            return hovered && is_mouse_button_pressed(MouseButton::Left);
        }

        false
    }

    fn draw_title_button(
        &self,
        mouse_pos: Vec2,
        x: f32,
        y: f32,
        normal_idx: usize,
        hover_idx: Option<usize>,
        pressed_idx: usize,
    ) -> bool {
        let pos = vec2(self.position.x + x, self.position.y + y);
        let base_tex = LibraryName::Title.get_texture(normal_idx);
        let (w, h) = base_tex
            .as_ref()
            .map(|t| (t.width as f32, t.height as f32))
            .unwrap_or((60.0, 20.0));

        let rect = Rect::new(pos.x, pos.y, w, h);
        let hovered = rect.contains(mouse_pos);
        let pressed = hovered && is_mouse_button_down(MouseButton::Left);

        let idx = if pressed {
            pressed_idx
        } else if hovered {
            hover_idx.unwrap_or(normal_idx)
        } else {
            normal_idx
        };

        if let Some(texture) = LibraryName::Title.get_texture(idx) {
            if let Some(tex) = texture.image.as_ref() {
                draw_texture_ex(tex, pos.x, pos.y, WHITE, DrawTextureParams::default());
            }
        }

        hovered && is_mouse_button_pressed(MouseButton::Left)
    }
}
