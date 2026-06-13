// ============================================================================
// NpcDialogHybrid - NPC 对话框（对齐 C# NPCDialog 的最小可用实现）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/NPCDialogs.cs -> NPCDialog
// - 背景：Prguse[384/385]
// - 关闭按钮：Prguse2[360-362]
// - 滚动按钮：Prguse2[197-199] / [207-209]
// - 文字区：x=8 y=34 每行 18px，高度 8 行
//
// 目前实现：
// - 显示 NPC dialog 文本（按 \n 分行）
// - 解析并渲染可点击选项：<text/@Action> 和 <<text/@Action>>
// - 点击 @Exit 关闭；其他 action 交给上层发送 CallNPC key

use macroquad::prelude::*;

use crate::resources::LibraryName;
use crate::scenes::dialogs::game::native_ui_utils::{ButtonState, ButtonTextures};
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};

#[derive(Debug, Clone)]
pub enum NpcDialogAction {
    None,
    Close,
    /// action 形如 "@Shop"，上层需要格式化成 key: "[@Shop]" 并发给服务器
    ClickAction { action: String },
    /// 链接：对齐 C# 的 ((text/url))
    OpenLink { url: String },
    // ===== PR #1169: Warehouse password actions =====
    /// 玩家在 NPC 对话框里点击了"输入仓库密码"按钮。
    /// 上层需要弹出 ShowTextInput(TextInputKind::UnlockStorage)。
    StorageUnlock,
    /// 玩家在 NPC 对话框里点击了"删除仓库密码"按钮。
    /// 上层需要弹出 ShowTextInput(TextInputKind::RemoveStoragePassword)。
    StorageRemovePassword,
}

#[derive(Debug, Clone)]
enum Segment {
    Plain(String),
    Action { text: String, action: String },
    Colored { text: String, color_name: String },
    Link { text: String, url: String },
    // PR #1126: KR-style NPC link (master C# 解析 `[MONSTER:idx|Name]`,
    // `[NPC:idx|Name]`, `[ITEM:idx|Name]` 格式 — 服务器 LinkFormatter 把
    // `<$MONSTER:5>` 转成 `[MONSTER:5|Beetle]` 后客户端显示)
    KrLink {
        link_type: KrLinkType,
        index: i32,
        display_name: String,
    },
}

/// PR #1126: KR NPC link 类型 (对齐 master LinkFormatter)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KrLinkType {
    Monster,
    Npc,
    Item,
}

impl KrLinkType {
    fn label(self) -> &'static str {
        match self {
            Self::Monster => "怪物",
            Self::Npc => "NPC",
            Self::Item => "物品",
        }
    }
}

#[derive(Debug, Clone)]
struct BigButton {
    text: String,
    action: String,
    color_name: String,
}

pub struct NpcDialogHybrid {
    visible: bool,

    // 窗口位置（可拖动）
    pos: Vec2,
    window_dragging: bool,
    window_drag_offset: Vec2,

    // 文本
    lines: Vec<String>,
    index: usize,
    maximum_lines: usize,

    // 大按钮（对齐 C# BigButtonDialog：<<text/@Action>>）
    big_buttons: Vec<BigButton>,
    big_scroll_offset: usize,

    // 纹理
    bg_texture_small: Option<Texture2D>,
    bg_texture_scroll: Option<Texture2D>,
    bg_size: Vec2,

    // BigButtonDialog 纹理（Title 836-840 + footer 837）
    big_bg_single: Option<Texture2D>,
    big_bg_top: Option<Texture2D>,
    big_bg_mid: Option<Texture2D>,
    big_bg_bottom: Option<Texture2D>,
    big_bg_footer: Option<Texture2D>,

    close_btn: ButtonTextures,
    up_btn: ButtonTextures,
    down_btn: ButtonTextures,

    scroll_bar_btn: ButtonTextures,
    scroll_dragging: bool,
    scroll_drag_offset_y: f32,

    big_btn: ButtonTextures,
}

impl Default for NpcDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcDialogHybrid {
    // 默认位置（对齐旧实现：左上角）
    const DEFAULT_POS_X: f32 = 0.0;
    const DEFAULT_POS_Y: f32 = 0.0;

    // 顶部可拖动区域高度（简化实现：标题栏/顶部背景）
    const DRAG_BAR_H: f32 = 32.0;

    const TEXT_X: f32 = 8.0;
    const TEXT_Y: f32 = 34.0;
    const LINE_STEP_Y: f32 = 18.0;

    const CLOSE_X: f32 = 413.0;
    const CLOSE_Y: f32 = 3.0;

    const UP_X: f32 = 417.0;
    const UP_Y: f32 = 34.0;
    const DOWN_X: f32 = 417.0;
    const DOWN_Y: f32 = 175.0;

    // PositionBar（对齐 C#：y in [47, 155]）
    const SCROLL_BAR_X: f32 = 417.0;
    const SCROLL_BAR_MIN_Y: f32 = 47.0;
    const SCROLL_BAR_MAX_Y: f32 = 155.0;

    const FONT_SIZE: f32 = 14.0;

    // BigButtonDialog 参考（C#）：MaximumRows=8, button location x=97, y=7+i*40
    const BIG_MAX_ROWS: usize = 8;
    const BIG_BUTTON_X: f32 = 97.0;
    const BIG_BUTTON_Y0: f32 = 7.0;
    const BIG_BUTTON_STEP_Y: f32 = 40.0;
    const BIG_UP_BTN_Y: f32 = 17.0;
    const BIG_DOWN_BTN_BOTTOM_PAD: f32 = 57.0;

    pub fn new() -> Self {
        Self {
            visible: false,

            pos: vec2(Self::DEFAULT_POS_X, Self::DEFAULT_POS_Y),
            window_dragging: false,
            window_drag_offset: vec2(0.0, 0.0),

            lines: Vec::new(),
            index: 0,
            maximum_lines: 8,

            big_buttons: Vec::new(),
            big_scroll_offset: 0,

            bg_texture_small: None,
            bg_texture_scroll: None,
            bg_size: vec2(450.0, 220.0),

            big_bg_single: None,
            big_bg_top: None,
            big_bg_mid: None,
            big_bg_bottom: None,
            big_bg_footer: None,

            close_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 360),
            up_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 197),
            down_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 207),

            scroll_bar_btn: ButtonTextures::load_from_indices(LibraryName::Prguse2, [205, 206, 206]),
            scroll_dragging: false,
            scroll_drag_offset_y: 0.0,

            big_btn: ButtonTextures::load_from_library(LibraryName::Title, 841),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.ensure_textures_loaded();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.scroll_dragging = false;
        self.big_scroll_offset = 0;
        self.window_dragging = false;
    }

    pub fn rect(&self) -> Rect {
        let w = self.bg_size.x.max(450.0);
        let mut h = self.bg_size.y.max(220.0);

        if self.has_big_buttons() {
            let panel_h = self.big_panel_height();
            let y = self.big_panel_y();
            h = h.max(y + panel_h);
        }

        Rect::new(self.pos.x, self.pos.y, w, h)
    }

    fn title_drag_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x, r.y, r.w, Self::DRAG_BAR_H.min(r.h))
    }

    fn clamp_pos_to_screen(&mut self) {
        let r = self.rect();
        let sw = screen_width();
        let sh = screen_height();

        // 保证窗口至少留一点在屏幕内，避免拖丢
        let min_visible = 20.0;
        let min_x = -(r.w - min_visible);
        let max_x = (sw - min_visible).max(min_x);
        let min_y = 0.0;
        let max_y = (sh - min_visible).max(min_y);

        self.pos.x = self.pos.x.clamp(min_x, max_x);
        self.pos.y = self.pos.y.clamp(min_y, max_y);
    }

    pub fn is_mouse_over(&self, mouse_pos: Vec2) -> bool {
        self.visible && self.rect().contains(mouse_pos)
    }

    pub fn new_dialog(&mut self, dialog: impl AsRef<str>) {
        let dialog = dialog.as_ref();
        let raw_lines: Vec<String> = dialog
            .split('\n')
            .map(|x| x.trim().to_string())
            .collect();

        let (lines, big_buttons) = Self::extract_big_buttons(raw_lines);
        self.lines = lines;
        self.big_buttons = big_buttons;
        self.big_scroll_offset = 0;
        self.index = 0;
        self.show();
    }

    fn ensure_textures_loaded(&mut self) {
        if self.bg_texture_small.is_some() || self.bg_texture_scroll.is_some() {
            // 仍需要确保 BigButtonDialog 纹理
        } else {
            // 对齐 C#：Index=384/385（Prguse）
            if let Some(info) = LibraryName::Prguse.get_texture(384) {
                self.bg_texture_small = info.image;
                self.bg_size = vec2(info.width as f32, info.height as f32);
            }
            if let Some(info) = LibraryName::Prguse.get_texture(385) {
                self.bg_texture_scroll = info.image;
                // 优先以带滚动版本的尺寸为准（避免切换时抖动）
                self.bg_size = vec2(info.width as f32, info.height as f32);
            }
        }

        // BigButtonDialog（Title）
        if self.big_bg_footer.is_none() {
            self.big_bg_single = LibraryName::Title.get_texture(836).and_then(|i| i.image);
            self.big_bg_footer = LibraryName::Title.get_texture(837).and_then(|i| i.image);
            self.big_bg_top = LibraryName::Title.get_texture(838).and_then(|i| i.image);
            self.big_bg_mid = LibraryName::Title.get_texture(839).and_then(|i| i.image);
            self.big_bg_bottom = LibraryName::Title.get_texture(840).and_then(|i| i.image);
        }
    }

    fn has_scroll(&self) -> bool {
        self.lines.len() > self.maximum_lines
    }

    fn has_big_buttons(&self) -> bool {
        !self.big_buttons.is_empty()
    }

    fn big_panel_y(&self) -> f32 {
        // 对齐 C#：无文本时 y=27，否则 y=Size.Height-33
        if self.lines.is_empty() {
            27.0
        } else {
            self.bg_size.y.max(220.0) - 33.0
        }
    }

    fn big_panel_height(&self) -> f32 {
        // 对齐 C#：背景块若干行 + footer
        // 这里按纹理高度累加；若纹理缺失，使用近似值。
        let count = self.big_panel_row_count();
        let row_h = self
            .big_bg_mid
            .as_ref()
            .map(|t| t.height())
            .unwrap_or(40.0);
        let footer_h = self
            .big_bg_footer
            .as_ref()
            .map(|t| t.height())
            .unwrap_or(18.0);
        (count as f32) * row_h + footer_h
    }

    fn big_panel_row_count(&self) -> usize {
        let minimum_buttons = if self.lines.is_empty() { 4 } else { 0 };
        let count = minimum_buttons.max(self.big_buttons.len());
        count.clamp(1, Self::BIG_MAX_ROWS)
    }

    fn big_visible_buttons(&self) -> usize {
        self.big_buttons.len().min(Self::BIG_MAX_ROWS)
    }

    fn extract_big_buttons(mut lines: Vec<String>) -> (Vec<String>, Vec<BigButton>) {
        let mut buttons: Vec<BigButton> = Vec::new();

        // 扫描每一行：抽取 <<...>>
        for line in lines.iter_mut() {
            if line.is_empty() {
                continue;
            }

            // 循环找出所有 << >>
            while let (Some(start), Some(end)) = (line.find("<<"), line.find(">>").map(|i| i + 2)) {
                let inner = line[start + 2..end].to_string();
                if let Some(btn) = Self::parse_big_button_inner(&inner) {
                    buttons.push(btn);
                }
                // 移除 <<...>>
                line.replace_range(start..end, "");
            }

            *line = line.trim().to_string();
        }

        // 移除空行（对齐 C#：如果移除 big button 后该行为空则删掉）
        lines.retain(|x| !x.trim().is_empty());
        (lines, buttons)
    }

    fn parse_big_button_inner(inner: &str) -> Option<BigButton> {
        // inner: "text/@Action" 或 "text/@Action/Color"
        let mut parts = inner.split('/');
        let text = parts.next()?.to_string();
        let action_raw = parts.next()?.to_string();
        if !action_raw.starts_with('@') {
            return None;
        }
        let color_name = parts.next().unwrap_or("RoyalBlue").to_string();
        Some(BigButton {
            text,
            action: action_raw,
            color_name,
        })
    }

    fn clamp_index(&mut self) {
        if self.lines.len() <= self.maximum_lines {
            self.index = 0;
            return;
        }
        let max = self.lines.len() - self.maximum_lines;
        if self.index > max {
            self.index = max;
        }
    }

    fn scroll_bar_y_from_index(&self) -> f32 {
        if !self.has_scroll() {
            return Self::SCROLL_BAR_MIN_Y;
        }
        let denom = (self.lines.len() - self.maximum_lines).max(1) as f32;
        let t = (self.index as f32) / denom;
        Self::SCROLL_BAR_MIN_Y + t * (Self::SCROLL_BAR_MAX_Y - Self::SCROLL_BAR_MIN_Y)
    }

    fn index_from_scroll_bar_y(&self, bar_y: f32) -> usize {
        if !self.has_scroll() {
            return 0;
        }
        let y = bar_y.clamp(Self::SCROLL_BAR_MIN_Y, Self::SCROLL_BAR_MAX_Y);
        let t = (y - Self::SCROLL_BAR_MIN_Y) / (Self::SCROLL_BAR_MAX_Y - Self::SCROLL_BAR_MIN_Y);
        let max = (self.lines.len() - self.maximum_lines).max(1);
        (t * (max as f32)).floor().clamp(0.0, max as f32) as usize
    }

    fn parse_segments(line: &str) -> Vec<Segment> {
        // 解析：
        // - <text/@Action> 和 <<text/@Action>>
        // - {text/Color}
        // - ((text/url))
        // 说明：这里不引入 regex，按最小可用做一次线性扫描。

        fn flush_plain(out: &mut Vec<Segment>, buf: &mut String) {
            if !buf.is_empty() {
                out.push(Segment::Plain(std::mem::take(buf)));
            }
        }

        let mut out = Vec::new();
        let mut plain = String::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 处理 ((...))
            if i + 1 < chars.len() && chars[i] == '(' && chars[i + 1] == '(' {
                let mut j = i + 2;
                while j + 1 < chars.len() {
                    if chars[j] == ')' && chars[j + 1] == ')' {
                        break;
                    }
                    j += 1;
                }
                if j + 1 < chars.len() && chars[j] == ')' && chars[j + 1] == ')' {
                    flush_plain(&mut out, &mut plain);
                    let inner: String = chars[i + 2..j].iter().collect();
                    if let Some(seg) = Self::parse_link_inner(&inner) {
                        out.push(seg);
                    } else {
                        out.push(Segment::Plain(format!("(({}))", inner)));
                    }
                    i = j + 2;
                    continue;
                }
            }

            // 处理 <<...>>
            if i + 1 < chars.len() && chars[i] == '<' && chars[i + 1] == '<' {
                // 找到 >>
                let mut j = i + 2;
                while j + 1 < chars.len() {
                    if chars[j] == '>' && chars[j + 1] == '>' {
                        break;
                    }
                    j += 1;
                }
                if j + 1 < chars.len() && chars[j] == '>' && chars[j + 1] == '>' {
                    flush_plain(&mut out, &mut plain);
                    let inner: String = chars[i + 2..j].iter().collect();
                    if let Some(seg) = Self::parse_action_inner(&inner) {
                        out.push(seg);
                    } else {
                        out.push(Segment::Plain(format!("<<{}>>", inner)));
                    }
                    i = j + 2;
                    continue;
                }
            }

            // 处理 <...>
            if chars[i] == '<' {
                let mut j = i + 1;
                while j < chars.len() {
                    if chars[j] == '>' {
                        break;
                    }
                    j += 1;
                }
                if j < chars.len() && chars[j] == '>' {
                    flush_plain(&mut out, &mut plain);
                    let inner: String = chars[i + 1..j].iter().collect();
                    if let Some(seg) = Self::parse_action_inner(&inner) {
                        out.push(seg);
                    } else {
                        out.push(Segment::Plain(format!("<{}>", inner)));
                    }
                    i = j + 1;
                    continue;
                }
            }

            // PR #1126: 处理 [...]  KR NPC link (master C# LinkFormatter 输出)
            if chars[i] == '[' {
                let mut j = i + 1;
                while j < chars.len() {
                    if chars[j] == ']' {
                        break;
                    }
                    j += 1;
                }
                if j < chars.len() && chars[j] == ']' {
                    flush_plain(&mut out, &mut plain);
                    let inner: String = chars[i + 1..j].iter().collect();
                    if let Some(seg) = Self::parse_kr_link_inner(&inner) {
                        out.push(seg);
                    } else {
                        out.push(Segment::Plain(format!("[{}]", inner)));
                    }
                    i = j + 1;
                    continue;
                }
            }

            // 处理 {...}
            if chars[i] == '{' {
                let mut j = i + 1;
                while j < chars.len() {
                    if chars[j] == '}' {
                        break;
                    }
                    j += 1;
                }
                if j < chars.len() && chars[j] == '}' {
                    flush_plain(&mut out, &mut plain);
                    let inner: String = chars[i + 1..j].iter().collect();
                    if let Some(seg) = Self::parse_color_inner(&inner) {
                        out.push(seg);
                    } else {
                        out.push(Segment::Plain(format!("{{{}}}", inner)));
                    }
                    i = j + 1;
                    continue;
                }
            }

            plain.push(chars[i]);
            i += 1;
        }

        flush_plain(&mut out, &mut plain);
        out
    }

    fn parse_action_inner(inner: &str) -> Option<Segment> {
        // inner: "text/@Action" 或 "text/@Action/Color"（Color 忽略）
        let mut parts = inner.split('/');
        let text = parts.next()?.to_string();
        let action = parts.next()?.to_string();
        if !action.starts_with('@') {
            return None;
        }
        Some(Segment::Action { text, action })
    }

    /// PR #1126: 解析 `[TYPE:idx|Name]` 格式
    /// 例:`[MONSTER:5|Beetle]` → KrLink { link_type: Monster, index: 5, display_name: "Beetle" }
    fn parse_kr_link_inner(inner: &str) -> Option<Segment> {
        // inner: "TYPE:idx|Name" (master C# 走 Regex;我们用 split)
        let mut parts = inner.splitn(2, '|');
        let left = parts.next()?;
        let name = parts.next()?.to_string();
        let mut type_and_idx = left.splitn(2, ':');
        let type_str = type_and_idx.next()?;
        let idx_str = type_and_idx.next()?;
        let index: i32 = idx_str.parse().ok()?;
        let link_type = match type_str {
            "MONSTER" => KrLinkType::Monster,
            "NPC" => KrLinkType::Npc,
            "ITEM" => KrLinkType::Item,
            _ => return None,
        };
        Some(Segment::KrLink {
            link_type,
            index,
            display_name: name,
        })
    }

    fn parse_color_inner(inner: &str) -> Option<Segment> {
        // inner: "text/ColorName"
        let mut parts = inner.split('/');
        let text = parts.next()?.to_string();
        let color_name = parts.next()?.to_string();
        if color_name.trim().is_empty() {
            return None;
        }
        Some(Segment::Colored { text, color_name })
    }

    fn parse_link_inner(inner: &str) -> Option<Segment> {
        // inner: "text/url"
        let mut parts = inner.split('/');
        let text = parts.next()?.to_string();
        let url = parts.next()?.to_string();
        if url.trim().is_empty() {
            return None;
        }
        Some(Segment::Link { text, url })
    }

    fn color_from_name(name: &str) -> Color {
        match name.trim().to_ascii_lowercase().as_str() {
            "red" => RED,
            "yellow" => YELLOW,
            "white" => WHITE,
            "green" => GREEN,
            "blue" => BLUE,
            "skyblue" => SKYBLUE,
            "orange" => ORANGE,
            "pink" => PINK,
            "purple" => PURPLE,
            "gray" | "grey" => GRAY,
            _ => WHITE,
        }
    }

    pub fn update_and_draw(&mut self) -> NpcDialogAction {
        self.update_and_draw_with_input(true)
    }

    /// `input_enabled=false` 时：仍绘制窗口，但不响应点击/滚轮/ESC/拖拽。
    ///
    /// 用途：当窗口被其他 dialog 覆盖时，避免“看起来在下面但仍能吃输入”。
    pub fn update_and_draw_with_input(&mut self, input_enabled: bool) -> NpcDialogAction {
        if !self.visible {
            return NpcDialogAction::None;
        }

        // 对齐 C#：ESC 关闭对话框
        if input_enabled && is_key_pressed(KeyCode::Escape) {
            self.hide();
            return NpcDialogAction::Close;
        }

        self.ensure_textures_loaded();

        let (mx, my) = mouse_position();
        let mouse_pos = vec2(mx, my);
        // 窗口拖拽（标题区域）
        // 注意：避免与 close 按钮冲突；仅左键按住时生效。
        if input_enabled {
            let rect0 = self.rect();
            let close_rect0 = Rect::new(
                rect0.x + Self::CLOSE_X,
                rect0.y + Self::CLOSE_Y,
                self.close_btn.size.x,
                self.close_btn.size.y,
            );
            let drag_rect0 = self.title_drag_rect();

            if is_mouse_button_pressed(MouseButton::Left)
                && drag_rect0.contains(mouse_pos)
                && !close_rect0.contains(mouse_pos)
            {
                self.window_dragging = true;
                self.window_drag_offset = mouse_pos - self.pos;
                // 拖动窗口时不应继续拖滚动条
                self.scroll_dragging = false;
            }
            if self.window_dragging {
                if is_mouse_button_down(MouseButton::Left) {
                    self.pos = mouse_pos - self.window_drag_offset;
                    self.clamp_pos_to_screen();
                } else {
                    self.window_dragging = false;
                }
            }
        } else {
            // 失去输入权限时，立即终止拖拽，避免“松开”在别处导致跳动
            self.window_dragging = false;
            self.scroll_dragging = false;
        }

        let rect = self.rect();

        // 鼠标滚轮滚动（对齐 C# MouseWheel）
        if input_enabled && self.is_mouse_over(mouse_pos) && self.has_scroll() {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                // macroquad: wheel 上为正
                let count = wheel.round() as i32;
                if count != 0 {
                    // C#：_index -= count
                    let next = (self.index as i32) - count;
                    self.index = next.max(0) as usize;
                    self.clamp_index();
                }
            }
        }

        // 背景
        let bg = if self.has_scroll() {
            self.bg_texture_scroll.as_ref().or(self.bg_texture_small.as_ref())
        } else {
            self.bg_texture_small.as_ref().or(self.bg_texture_scroll.as_ref())
        };

        if let Some(bg) = bg {
            draw_texture(bg, rect.x, rect.y, WHITE);
        } else {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.0, 0.0, 0.0, 0.75));
        }

        // 关闭按钮
        let close_rect = Rect::new(
            rect.x + Self::CLOSE_X,
            rect.y + Self::CLOSE_Y,
            self.close_btn.size.x,
            self.close_btn.size.y,
        );
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn.draw(vec2(close_rect.x, close_rect.y), close_state);
        if input_enabled && ButtonState::is_clicked(close_rect, mouse_pos) {
            self.hide();
            return NpcDialogAction::Close;
        }

        // 滚动按钮
        if self.has_scroll() {
            let up_rect = Rect::new(
                rect.x + Self::UP_X,
                rect.y + Self::UP_Y,
                self.up_btn.size.x,
                self.up_btn.size.y,
            );
            let up_state = ButtonState::from_mouse(up_rect, mouse_pos);
            self.up_btn.draw(vec2(up_rect.x, up_rect.y), up_state);
            if input_enabled && ButtonState::is_clicked(up_rect, mouse_pos)
                && self.index > 0 {
                    self.index -= 1;
                }

            let down_rect = Rect::new(
                rect.x + Self::DOWN_X,
                rect.y + Self::DOWN_Y,
                self.down_btn.size.x,
                self.down_btn.size.y,
            );
            let down_state = ButtonState::from_mouse(down_rect, mouse_pos);
            self.down_btn.draw(vec2(down_rect.x, down_rect.y), down_state);
            if input_enabled && ButtonState::is_clicked(down_rect, mouse_pos)
                && self.index + self.maximum_lines < self.lines.len() {
                    self.index += 1;
                }

            // PositionBar（可拖拽）
            let bar_y = rect.y + self.scroll_bar_y_from_index();
            let bar_rect = Rect::new(
                rect.x + Self::SCROLL_BAR_X,
                bar_y,
                self.scroll_bar_btn.size.x,
                self.scroll_bar_btn.size.y,
            );

            // 开始拖拽
            if input_enabled
                && is_mouse_button_pressed(MouseButton::Left)
                && bar_rect.contains(mouse_pos)
            {
                self.scroll_dragging = true;
                self.scroll_drag_offset_y = mouse_pos.y - bar_rect.y;
            }
            // 结束拖拽
            if self.scroll_dragging && (!input_enabled || !is_mouse_button_down(MouseButton::Left)) {
                self.scroll_dragging = false;
            }

            let mut draw_bar_rect = bar_rect;
            if self.scroll_dragging {
                let desired_y = mouse_pos.y - self.scroll_drag_offset_y;
                let desired_local_y = (desired_y - rect.y)
                    .clamp(Self::SCROLL_BAR_MIN_Y, Self::SCROLL_BAR_MAX_Y);
                self.index = self.index_from_scroll_bar_y(desired_local_y);
                self.clamp_index();
                draw_bar_rect.y = rect.y + desired_local_y;
            }

            let bar_state = ButtonState::from_mouse(draw_bar_rect, mouse_pos);
            self.scroll_bar_btn
                .draw(vec2(draw_bar_rect.x, draw_bar_rect.y), bar_state);
        }

        // 文本
        let last_line = (self.index + self.maximum_lines).min(self.lines.len());
        let mut action_clicked: Option<String> = None;
        let mut link_clicked: Option<String> = None;
        // PR #1126: 当前 hover 的 KR NPC link (用于 tooltip 渲染)
        let mut kr_link_hovered: Option<(KrLinkType, i32, String)> = None;

        for (row, i) in (self.index..last_line).enumerate() {
            let y = rect.y + Self::TEXT_Y + (row as f32) * Self::LINE_STEP_Y;
            let x0 = rect.x + Self::TEXT_X;
            let line = &self.lines[i];

            let segments = Self::parse_segments(line);
            let mut x = x0;

            for seg in segments {
                match seg {
                    Segment::Plain(text) => {
                        draw_text_cn(&text, x, y + 14.0, Self::FONT_SIZE, WHITE);
                        x += measure_text_cn(&text, Self::FONT_SIZE).width;
                    }
                    Segment::Action { text, action } => {
                        let dims = measure_text_cn(&text, Self::FONT_SIZE);
                        let span_rect = Rect::new(x, y, dims.width, dims.height.max(Self::LINE_STEP_Y));

                        let hovered = span_rect.contains(mouse_pos);
                        let color = if hovered { RED } else { YELLOW };
                        draw_text_cn(&text, x, y + 14.0, Self::FONT_SIZE, color);

                        // 点击
                        if input_enabled && hovered && is_mouse_button_pressed(MouseButton::Left) {
                            action_clicked = Some(action.clone());
                        }

                        x += dims.width;
                    }
                    Segment::Colored { text, color_name } => {
                        let color = Self::color_from_name(&color_name);
                        draw_text_cn(&text, x, y + 14.0, Self::FONT_SIZE, color);
                        x += measure_text_cn(&text, Self::FONT_SIZE).width;
                    }
                    Segment::Link { text, url } => {
                        let dims = measure_text_cn(&text, Self::FONT_SIZE);
                        let span_rect = Rect::new(x, y, dims.width, dims.height.max(Self::LINE_STEP_Y));
                        let hovered = span_rect.contains(mouse_pos);
                        let color = if hovered { RED } else { YELLOW };
                        draw_text_cn(&text, x, y + 14.0, Self::FONT_SIZE, color);
                        if input_enabled && hovered && is_mouse_button_pressed(MouseButton::Left) {
                            link_clicked = Some(url);
                        }
                        x += dims.width;
                    }
                    // PR #1126: KR NPC link 渲染 (黄色/红色 hover 颜色 + tooltip)
                    Segment::KrLink { link_type, index, display_name } => {
                        let label = format!("[{}:{}]", link_type.label(), display_name);
                        let dims = measure_text_cn(&label, Self::FONT_SIZE);
                        let span_rect = Rect::new(x, y, dims.width, dims.height.max(Self::LINE_STEP_Y));
                        let hovered = span_rect.contains(mouse_pos);
                        let color = if hovered { RED } else { Color::from_rgba(255, 200, 100, 255) };
                        draw_text_cn(&label, x, y + 14.0, Self::FONT_SIZE, color);
                        // Track hovered link for tooltip rendering
                        if hovered {
                            kr_link_hovered = Some((link_type, index, display_name.clone()));
                        }
                        x += dims.width;
                    }
                }
            }
        }

        // BigButtonDialog（绘制在对话框下方/底部）
        if self.has_big_buttons() {
            let panel_y = rect.y + self.big_panel_y();
            let panel_x = rect.x + 1.0;
            let panel_w = self
                .big_bg_top
                .as_ref()
                .or(self.big_bg_mid.as_ref())
                .or(self.big_bg_bottom.as_ref())
                .or(self.big_bg_single.as_ref())
                .map(|t| t.width())
                .unwrap_or(rect.w);

            let rows = self.big_panel_row_count();
            let row_h = self
                .big_bg_mid
                .as_ref()
                .map(|t| t.height())
                .unwrap_or(40.0);
            let footer_h = self
                .big_bg_footer
                .as_ref()
                .map(|t| t.height())
                .unwrap_or(18.0);

            // 背景行
            for i in 0..rows {
                let tex = if rows == 1 {
                    self.big_bg_single.as_ref()
                } else if i == 0 {
                    self.big_bg_top.as_ref()
                } else if i + 1 == rows {
                    self.big_bg_bottom.as_ref()
                } else {
                    self.big_bg_mid.as_ref()
                };

                let y = panel_y + (i as f32) * row_h;
                if let Some(tex) = tex {
                    draw_texture(tex, panel_x, y, WHITE);
                } else {
                    draw_rectangle(panel_x, y, panel_w, row_h, Color::new(0.0, 0.0, 0.0, 0.55));
                }
            }
            // footer
            let footer_y = panel_y + (rows as f32) * row_h;
            if let Some(tex) = self.big_bg_footer.as_ref() {
                draw_texture(tex, panel_x - 1.0, footer_y, WHITE);
            } else {
                draw_rectangle(panel_x - 1.0, footer_y, panel_w, footer_h, Color::new(0.0, 0.0, 0.0, 0.55));
            }

            let panel_h = (rows as f32) * row_h + footer_h;
            let panel_rect = Rect::new(panel_x, panel_y, panel_w, panel_h);

            // big button 滚轮
            if input_enabled && panel_rect.contains(mouse_pos) {
                let wheel = mouse_wheel().1;
                if wheel != 0.0 {
                    let count = wheel.round() as i32;
                    if count > 0 {
                        if self.big_scroll_offset > 0 {
                            self.big_scroll_offset -= 1;
                        }
                    } else if count < 0
                        && self.big_scroll_offset + Self::BIG_MAX_ROWS < self.big_buttons.len() {
                            self.big_scroll_offset += 1;
                        }
                }
            }

            // big button up/down（当按钮数量 > 8）
            if self.big_buttons.len() > Self::BIG_MAX_ROWS {
                let up_rect = Rect::new(
                    panel_x + panel_w - 26.0,
                    panel_y + Self::BIG_UP_BTN_Y,
                    self.up_btn.size.x,
                    self.up_btn.size.y,
                );
                let up_state = ButtonState::from_mouse(up_rect, mouse_pos);
                self.up_btn.draw(vec2(up_rect.x, up_rect.y), up_state);
                if input_enabled && ButtonState::is_clicked(up_rect, mouse_pos)
                    && self.big_scroll_offset > 0 {
                        self.big_scroll_offset -= 1;
                    }

                let down_rect = Rect::new(
                    panel_x + panel_w - 26.0,
                    panel_y + panel_h - Self::BIG_DOWN_BTN_BOTTOM_PAD,
                    self.down_btn.size.x,
                    self.down_btn.size.y,
                );
                let down_state = ButtonState::from_mouse(down_rect, mouse_pos);
                self.down_btn.draw(vec2(down_rect.x, down_rect.y), down_state);
                if input_enabled && ButtonState::is_clicked(down_rect, mouse_pos)
                    && self.big_scroll_offset + Self::BIG_MAX_ROWS < self.big_buttons.len() {
                        self.big_scroll_offset += 1;
                    }
            }

            // 绘制按钮本体
            let visible = self.big_visible_buttons();
            for i in 0..visible {
                let btn_idx = i + self.big_scroll_offset;
                if btn_idx >= self.big_buttons.len() {
                    break;
                }
                let btn = &self.big_buttons[btn_idx];

                let x = panel_x + Self::BIG_BUTTON_X;
                let y = panel_y + Self::BIG_BUTTON_Y0 + (i as f32) * Self::BIG_BUTTON_STEP_Y;
                let w = self.big_btn.size.x.max(237.0);
                let h = self.big_btn.size.y.max(32.0);
                let btn_rect = Rect::new(x, y, w, h);
                let state = ButtonState::from_mouse(btn_rect, mouse_pos);
                self.big_btn.draw(vec2(x, y), state);

                // 文字（居中 + 阴影）
                let text_color = Self::color_from_name(&btn.color_name);
                let dims = measure_text_cn(&btn.text, 16.0);
                let tx = x + (w - dims.width) / 2.0;
                let ty = y + (h / 2.0) + 6.0;
                draw_text_cn(&btn.text, tx + 2.0, ty + 2.0, 16.0, BLACK);
                draw_text_cn(&btn.text, tx, ty, 16.0, text_color);

                if ButtonState::is_clicked(btn_rect, mouse_pos) {
                    action_clicked = Some(btn.action.clone());
                }
            }
        }

        if let Some(action) = action_clicked {
            if action == "@Exit" {
                self.hide();
                return NpcDialogAction::Close;
            }
            // PR #1169: Warehouse password action intercepts.
            // 触发规则:在 NPC 脚本里写 `@StorageUnlock` 或 `@StorageRemovePassword`
            // (用现有 <<text/@Action>> 标签),本层拦截并转为专用 NpcDialogAction,
            // 由 actions.rs 弹 ShowTextInput。不发给服务器。
            // (SetStoragePassword 需要 current + new 两段 input,
            //  留待后续 PR — 当前 TextInputKind 是单字段)
            match action.as_str() {
                "@StorageUnlock" => return NpcDialogAction::StorageUnlock,
                "@StorageRemovePassword" => return NpcDialogAction::StorageRemovePassword,
                _ => {}
            }
            return NpcDialogAction::ClickAction { action };
        }

        if let Some(url) = link_clicked {
            return NpcDialogAction::OpenLink { url };
        }

        // PR #1126: 渲染 KR NPC link tooltip (master C# AttackInfoLabel 等价)
        if let Some((link_type, index, display_name)) = kr_link_hovered {
            let tip = match link_type {
                KrLinkType::Monster => {
                    // 简单版:显示 name + index;NewMonsterInfo 数据后续由
                    // 工具提示系统读 ui_state.tooltip_cache。
                    format!("怪物: {} (#{})", display_name, index)
                }
                KrLinkType::Npc => {
                    format!("NPC: {} (#{})", display_name, index)
                }
                KrLinkType::Item => {
                    format!("物品: {} (#{})", display_name, index)
                }
            };
            let tip_x = mouse_pos.x + 16.0;
            let tip_y = mouse_pos.y + 16.0;
            let w = tip.chars().count() as f32 * 8.0 + 8.0;
            let h = 20.0;
            draw_rectangle(tip_x, tip_y, w, h, Color::from_rgba(0, 0, 0, 200));
            draw_text_cn(&tip, tip_x + 4.0, tip_y + 4.0, 12.0, Color::from_rgba(255, 200, 100, 255));
        }

        NpcDialogAction::None
    }
}
