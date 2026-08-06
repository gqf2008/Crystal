// ============================================================================
// ReportDialogHybrid - 举报 / Bug 反馈对话框
// ============================================================================
//
// C# 参考: Client/MirScenes/Dialogs/ReportDialog.cs (75 行)
// - 背景: Index=1633, Library=Prguse, Movable=true, Sort=true, Location=Center
// - 关闭按钮: Prguse2[360/361/362]
// - ReportType 下拉: 选择类型 / 提交Bug / 举报玩家
// - MessageArea: 330x150 多行文本框 (8F 字体)
// - SendButton: C# 原版 SendButton_Click 抛 NotImplementedException
//   （即原版未实现发送逻辑）
//
// Rust 实现:
// - 与 C# 不同，发送按钮已接线：利用已存在的
//   `ClientPacketIds::ReportIssue` (SharedRust/src/enums.rs:213)
//   及 `client::ReportIssue { message }` 数据包，draw() 返回
//   `Some(message)`，由 UIRenderSystem 转发为
//   `NetworkEvent::ReportIssueRequest { issue }`。
// - 类型选择用 3 个按钮模拟下拉（对齐 C# 的 3 个 Items）。
// - 多行文本：单行输入 + 多行视觉模拟（按宽度折行显示），
//   输入用 get_char_pressed 支持 Unicode/中文。
//
// ============================================================================

use macroquad::prelude::*;

use super::native_ui_utils::{ButtonState, ButtonTextures};
use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use crate::utils::ime::set_ime_enabled;

/// 举报类型（对齐 C# ReportType.Items）
///
/// - 0: 选择类型（占位，不可发送）
/// - 1: 提交 Bug
/// - 2: 举报玩家
pub type ReportType = u8;

pub const REPORT_TYPE_NONE: ReportType = 0;
pub const REPORT_TYPE_BUG: ReportType = 1;
pub const REPORT_TYPE_PLAYER: ReportType = 2;

fn report_type_label(t: ReportType) -> &'static str {
    match t {
        REPORT_TYPE_BUG => "提交Bug",
        REPORT_TYPE_PLAYER => "举报玩家",
        _ => "选择类型",
    }
}

/// 举报 / Bug 反馈对话框
pub struct ReportDialogHybrid {
    visible: bool,
    /// 当前选中的举报类型
    report_type: ReportType,
    /// 消息正文（单行存储，显示时按宽度折行模拟多行）
    message: String,
    max_length: usize,

    // 布局
    position: Vec2,
    size: Vec2,

    // 关闭按钮（Prguse2[360/361/362]，对齐 C#）
    close_btn: ButtonTextures,

    // Backspace 连续删除
    backspace_timer: f64,
    backspace_repeat: bool,
}

impl Default for ReportDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportDialogHybrid {
    // 对齐 C# ReportDialog 的布局常量
    const TITLE_BAR_H: f32 = 26.0;
    const PADDING: f32 = 12.0;
    const TYPE_Y: f32 = 35.0; // C# ReportType.Location.Y
    const MSG_Y: f32 = 57.0; // C# MessageArea.Location.Y
    const MSG_W: f32 = 330.0; // C# MessageArea.Size.Width
    const MSG_H: f32 = 150.0; // C# MessageArea.Size.Height
    const SEND_W: f32 = 60.0;
    const SEND_H: f32 = 24.0;

    pub fn new() -> Self {
        Self {
            visible: false,
            report_type: REPORT_TYPE_NONE,
            message: String::new(),
            max_length: 500,
            position: Vec2::ZERO,
            // 对齐 C# 背景 1633 的近似尺寸（关闭按钮在 x=336，宽度约 360）
            size: vec2(360.0, 250.0),
            close_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 360),
            backspace_timer: 0.0,
            backspace_repeat: false,
        }
    }

    /// 显示对话框（屏幕居中，对齐 C# Location = Center）
    pub fn show(&mut self) {
        self.report_type = REPORT_TYPE_NONE;
        self.message.clear();
        self.visible = true;
        self.load_textures();
        self.center();
        set_ime_enabled(true);
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
        set_ime_enabled(false);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 探测背景纹理（Prguse[1633]）尺寸以对齐 C# 布局；绘制时用纯色矩形保持稳定。
    fn load_textures(&mut self) {
        if let Some(info) = crate::resources::LibraryName::Prguse.get_texture(1633) {
            self.size = vec2(
                self.size.x.max(info.width as f32),
                self.size.y.max(info.height as f32),
            );
        }
        self.center();
    }

    fn center(&mut self) {
        let sw = screen_width();
        let sh = screen_height();
        self.position = vec2((sw - self.size.x) / 2.0, (sh - self.size.y) / 2.0);
    }

    /// 鼠标是否落在对话框上
    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 处理键盘输入（Unicode/中文、Backspace、Ctrl+V、Escape）
    fn handle_keyboard(&mut self) {
        // Unicode/中文输入
        let mut pending: Vec<char> = Vec::new();
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                pending.push(ch);
            }
        }
        for ch in pending {
            if self.message.chars().count() < self.max_length {
                self.message.push(ch);
            }
        }

        // Ctrl+V 粘贴
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && is_key_pressed(KeyCode::V)
        {
            if let Some(clipboard) = miniquad::window::clipboard_get() {
                for ch in clipboard.chars() {
                    if !ch.is_control() && self.message.chars().count() < self.max_length {
                        self.message.push(ch);
                    }
                }
            }
        }

        // Escape 关闭
        if is_key_pressed(KeyCode::Escape) {
            self.close();
            return;
        }

        // Backspace 连续删除（与 TextInputDialog 一致）
        if is_key_down(KeyCode::Backspace) {
            let now = get_time();
            if is_key_pressed(KeyCode::Backspace) {
                self.message.pop();
                self.backspace_timer = now;
                self.backspace_repeat = false;
            } else {
                let delay = if self.backspace_repeat { 0.03 } else { 0.4 };
                if now - self.backspace_timer > delay {
                    self.message.pop();
                    self.backspace_timer = now;
                    self.backspace_repeat = true;
                }
            }
        } else {
            self.backspace_repeat = false;
        }
    }

    /// 更新并绘制。
    ///
    /// 返回值：点击发送按钮且内容有效时返回 `Some(message)`，
    /// 由调用方转发为 `NetworkEvent::ReportIssueRequest`。
    /// （C# 原版 SendButton_Click 为 NotImplementedException；此处补全发送逻辑。）
    pub fn draw(&mut self, mouse_pos: Vec2, left_clicked: bool) -> Option<String> {
        if !self.visible {
            return None;
        }

        self.handle_keyboard();

        let px = self.position.x;
        let py = self.position.y;
        let w = self.size.x;
        let h = self.size.y;

        // ===== 背景 =====
        draw_rectangle(px, py, w, h, Color::from_rgba(25, 25, 35, 235));

        // 标题栏
        draw_rectangle(
            px,
            py,
            w,
            Self::TITLE_BAR_H,
            Color::from_rgba(50, 50, 70, 255),
        );
        draw_text_cn(
            "举报 / 反馈",
            px + 12.0,
            py + 8.0,
            14.0,
            Color::from_rgba(255, 220, 100, 255),
        );

        // 边框
        let border = Color::from_rgba(120, 100, 50, 200);
        draw_rectangle_lines(px, py, w, h, 1.5, border);

        // ===== 关闭按钮（Prguse2[360/361/362]，C# Location=(336,3)） =====
        let close_pos = vec2(px + 336.0, py + 3.0);
        let close_size = self.close_btn.size;
        let close_rect = Rect::new(
            close_pos.x,
            close_pos.y,
            close_size.x.max(16.0),
            close_size.y.max(16.0),
        );
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn.draw(close_pos, close_state);
        if ButtonState::is_clicked(close_rect, mouse_pos) {
            self.close();
            return None;
        }

        // ===== 举报类型（3 个按钮模拟下拉） =====
        let type_y = py + Self::TYPE_Y;
        let labels = [
            (REPORT_TYPE_NONE, "选择类型"),
            (REPORT_TYPE_BUG, "提交Bug"),
            (REPORT_TYPE_PLAYER, "举报玩家"),
        ];
        let btn_w = 90.0;
        let btn_h = 20.0;
        for (i, (t, label)) in labels.iter().enumerate() {
            let bx = px + Self::PADDING + i as f32 * (btn_w + 6.0);
            let rect = Rect::new(bx, type_y, btn_w, btn_h);
            let selected = self.report_type == *t;
            let hover = rect.contains(mouse_pos);

            let bg = if selected {
                Color::from_rgba(90, 70, 30, 255)
            } else if hover {
                Color::from_rgba(60, 60, 80, 255)
            } else {
                Color::from_rgba(35, 35, 50, 255)
            };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                1.0,
                Color::from_rgba(90, 90, 110, 255),
            );

            let color = if selected {
                Color::from_rgba(255, 230, 120, 255)
            } else {
                WHITE
            };
            let m = measure_text_cn(label, 12.0);
            draw_text_cn(
                label,
                rect.x + (rect.w - m.width) / 2.0,
                rect.y + 15.0,
                12.0,
                color,
            );

            if hover && left_clicked {
                self.report_type = *t;
            }
        }

        // ===== 消息文本框（多行视觉模拟） =====
        let msg_x = px + Self::PADDING;
        let msg_y = py + Self::MSG_Y;
        let msg_rect = Rect::new(msg_x, msg_y, Self::MSG_W, Self::MSG_H);
        draw_rectangle(
            msg_rect.x,
            msg_rect.y,
            msg_rect.w,
            msg_rect.h,
            Color::from_rgba(15, 15, 25, 255),
        );
        draw_rectangle_lines(
            msg_rect.x,
            msg_rect.y,
            msg_rect.w,
            msg_rect.h,
            1.0,
            Color::from_rgba(80, 80, 100, 255),
        );

        // 按宽度折行显示（模拟多行）
        let font_size = 13.0; // 对齐 C# 8F 字体的视觉密度
        let line_h = font_size + 4.0;
        let inner_w = msg_rect.w - 8.0;
        let mut y = msg_y + font_size + 2.0;

        if self.message.is_empty() {
            draw_text_cn(
                "请输入举报/反馈内容...",
                msg_x + 4.0,
                y,
                font_size,
                Color::from_rgba(120, 120, 120, 255),
            );
        } else {
            // 按字符宽度累加折行
            let mut current = String::new();
            for ch in self.message.chars() {
                let probe = format!("{}{}", current, ch);
                if measure_text_cn(&probe, font_size).width > inner_w && !current.is_empty() {
                    draw_text_cn(&current, msg_x + 4.0, y, font_size, WHITE);
                    y += line_h;
                    if y > msg_y + Self::MSG_H {
                        break;
                    }
                    current = ch.to_string();
                } else {
                    current = probe;
                }
            }
            if !current.is_empty() && y <= msg_y + Self::MSG_H {
                draw_text_cn(&current, msg_x + 4.0, y, font_size, WHITE);
                y += line_h;
            }

            // 光标（最后一行末尾）
            if y <= msg_y + Self::MSG_H {
                let cursor_w = measure_text_cn(&current, font_size).width;
                let blink = ((get_time() * 2.0) as u64) % 2 == 0;
                if blink {
                    draw_line(
                        msg_x + 4.0 + cursor_w + 1.0,
                        y - font_size,
                        msg_x + 4.0 + cursor_w + 1.0,
                        y - 2.0,
                        1.0,
                        WHITE,
                    );
                }
            }
        }

        // ===== 发送按钮 =====
        // C# SendButton.Location = (260, 219)
        let send_x = px + 260.0;
        let send_y = py + h - Self::SEND_H - 7.0;
        let send_rect = Rect::new(send_x, send_y, Self::SEND_W, Self::SEND_H);

        let can_send = self.report_type != REPORT_TYPE_NONE && !self.message.trim().is_empty();
        let send_hover = send_rect.contains(mouse_pos);
        let send_bg = if !can_send {
            Color::from_rgba(50, 50, 55, 255)
        } else if send_hover {
            Color::from_rgba(70, 110, 60, 255)
        } else {
            Color::from_rgba(50, 90, 45, 255)
        };
        draw_rectangle(send_rect.x, send_rect.y, send_rect.w, send_rect.h, send_bg);
        draw_rectangle_lines(
            send_rect.x,
            send_rect.y,
            send_rect.w,
            send_rect.h,
            1.0,
            Color::from_rgba(90, 90, 110, 255),
        );

        let send_color = if can_send {
            WHITE
        } else {
            Color::from_rgba(150, 150, 150, 255)
        };
        let send_label = "发送";
        let sm = measure_text_cn(send_label, 13.0);
        draw_text_cn(
            send_label,
            send_rect.x + (send_rect.w - sm.width) / 2.0,
            send_rect.y + 16.0,
            13.0,
            send_color,
        );

        if send_hover && left_clicked && can_send {
            let issue = self.message.trim().to_string();
            // 拼上类型前缀，便于服务端/GM 区分
            let prefixed = format!("[{}] {}", report_type_label(self.report_type), issue);
            self.close();
            return Some(prefixed);
        }

        None
    }
}
