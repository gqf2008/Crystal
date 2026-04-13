// ============================================================================
// NoticeDialogHybrid - 服务器公告/通知对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/NoticeDialog.cs (~373 行)
// - 显示服务器公告（富文本、多行）
// - 弹出式对话框，可关闭
// - 首次登录时自动弹出
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;

/// 服务器公告对话框
pub struct NoticeDialogHybrid {
    pub visible: bool,
    /// 公告文本内容（每行一条）
    pub lines: Vec<String>,
    /// 滚动偏移
    scroll_offset: f32,
    /// 是否正在拖拽滚动
    is_dragging: bool,
    drag_start_y: f32,
    drag_start_scroll: f32,
}

impl Default for NoticeDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            lines: Vec::new(),
            scroll_offset: 0.0,
            is_dragging: false,
            drag_start_y: 0.0,
            drag_start_scroll: 0.0,
        }
    }
}

impl NoticeDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置公告文本
    pub fn set_notice(&mut self, text: String) {
        self.lines = text.lines().map(|s| s.to_string()).collect();
        self.scroll_offset = 0.0;
        if !self.lines.is_empty() {
            self.visible = true;
        }
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 切换显示/隐藏
    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else if !self.lines.is_empty() {
            self.visible = true;
        }
    }

    /// 处理鼠标输入并绘制
    pub fn draw(&mut self, mouse_pos: Vec2, mouse_wheel: f32, left_clicked: bool, left_released: bool) -> bool {
        if !self.visible || self.lines.is_empty() {
            return false;
        }

        let dialog_w = 500.0;
        let dialog_h = 350.0;
        let dialog_x = (screen_width() - dialog_w) / 2.0;
        let dialog_y = (screen_height() - dialog_h) / 2.0;

        let title_bar_h = 30.0;
        let content_x = dialog_x + 15.0;
        let content_y = dialog_y + title_bar_h + 5.0;
        let content_w = dialog_w - 30.0;
        let content_h = dialog_h - title_bar_h - 45.0; // 留出底部关闭按钮空间

        let mouse_over_dialog = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        // 滚动（鼠标滚轮）
        if mouse_over_dialog && mouse_wheel != 0.0 {
            self.scroll_offset = (self.scroll_offset - mouse_wheel * 20.0).max(0.0);
        }

        // 拖拽滚动
        if left_clicked && mouse_over_dialog && mouse_pos.y >= content_y && mouse_pos.y <= content_y + content_h {
            self.is_dragging = true;
            self.drag_start_y = mouse_pos.y;
            self.drag_start_scroll = self.scroll_offset;
        }
        if self.is_dragging && !left_released {
            let dy = self.drag_start_y - mouse_pos.y;
            self.scroll_offset = (self.drag_start_scroll + dy).max(0.0);
        }
        if left_released {
            self.is_dragging = false;
        }

        // 背景
        let bg_color = Color::from_rgba(25, 25, 35, 230);
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, bg_color);

        // 标题栏
        let title_color = Color::from_rgba(50, 50, 70, 255);
        draw_rectangle(dialog_x, dialog_y, dialog_w, title_bar_h, title_color);
        draw_text_cn("服务器公告", dialog_x + 15.0, dialog_y + 8.0, 16.0, Color::from_rgba(255, 220, 100, 255));

        // 内容区域裁剪
        // 先绘制内容区域背景
        let content_bg = Color::from_rgba(15, 15, 25, 200);
        draw_rectangle(content_x, content_y, content_w, content_h, content_bg);

        // 逐行绘制公告文本
        let line_height = 18.0;
        let mut y = content_y + 5.0 - self.scroll_offset;

        for line in &self.lines {
            // 跳过屏幕外的行
            if y + line_height < content_y || y > content_y + content_h {
                y += line_height;
                continue;
            }

            // 简单处理长行换行（按字符数，避免 UTF-8 字节切片 panic）
            let max_chars_per_line = (content_w / 8.0) as usize;
            let chars: Vec<char> = line.chars().collect();
            if chars.len() > max_chars_per_line {
                let mut start = 0;
                while start < chars.len() {
                    let end = (start + max_chars_per_line).min(chars.len());
                    let sub: String = chars[start..end].iter().collect();
                    draw_text_cn(&sub, content_x + 5.0, y, 14.0, Color::from_rgba(220, 220, 220, 255));
                    y += line_height;
                    start = end;
                }
            } else {
                draw_text_cn(line, content_x + 5.0, y, 14.0, Color::from_rgba(220, 220, 220, 255));
                y += line_height;
            }
        }

        // 边框
        let border_color = Color::from_rgba(120, 100, 50, 200);
        draw_line(dialog_x, dialog_y, dialog_x + dialog_w, dialog_y, 1.5, border_color);
        draw_line(dialog_x + dialog_w, dialog_y, dialog_x + dialog_w, dialog_y + dialog_h, 1.5, border_color);
        draw_line(dialog_x + dialog_w, dialog_y + dialog_h, dialog_x, dialog_y + dialog_h, 1.5, border_color);
        draw_line(dialog_x, dialog_y + dialog_h, dialog_x, dialog_y, 1.5, border_color);

        // 关闭按钮
        let close_btn_w = 60.0;
        let close_btn_h = 25.0;
        let close_btn_x = dialog_x + (dialog_w - close_btn_w) / 2.0;
        let close_btn_y = dialog_y + dialog_h - close_btn_h - 10.0;

        let mouse_over_close = mouse_pos.x >= close_btn_x && mouse_pos.x <= close_btn_x + close_btn_w
            && mouse_pos.y >= close_btn_y && mouse_pos.y <= close_btn_y + close_btn_h;

        let close_color = if mouse_over_close {
            Color::from_rgba(180, 60, 60, 255)
        } else {
            Color::from_rgba(120, 40, 40, 255)
        };
        draw_rectangle(close_btn_x, close_btn_y, close_btn_w, close_btn_h, close_color);
        draw_text_cn("关闭", close_btn_x + 15.0, close_btn_y + 7.0, 14.0, Color::from_rgba(255, 255, 255, 255));

        // 点击关闭按钮
        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over_dialog
    }
}
