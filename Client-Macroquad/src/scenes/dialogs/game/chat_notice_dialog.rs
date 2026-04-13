// ============================================================================
// ChatNoticeDialogHybrid - 屏幕中央 transient 通知
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/ChatNoticeDialog.cs
// - 半透明屏幕中央通知，用于重要系统提示（如中毒、任务完成等）
// - 自动淡出（fade out）
// - 可叠加多个通知，排队显示
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};

/// 单个通知条目
struct NoticeEntry {
    text: String,
    elapsed: f32,       // 已显示时间（秒）
    duration: f32,      // 总显示时长（秒）
    fade_out_start: f32, // 开始淡出的时间点
}

/// 屏幕中央 transient 通知对话框
#[derive(Default)]
pub struct ChatNoticeDialogHybrid {
    queue: Vec<NoticeEntry>,
    /// 当前显示的通知（队首）
    current: Option<NoticeEntry>,
}

impl ChatNoticeDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 推送一个通知到队列
    pub fn push_notice(&mut self, text: String, duration: f32) {
        let fade_out_start = duration * 0.7; // 最后 30% 时间淡出
        self.queue.push(NoticeEntry {
            text,
            elapsed: 0.0,
            duration,
            fade_out_start,
        });
    }

    /// 推送默认时长（3秒）的通知
    pub fn push_notice_default(&mut self, text: String) {
        self.push_notice(text, 3.0);
    }

    /// 更新并绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, delta: f32) {
        // 如果当前没有显示的通知，从队列取一个
        if self.current.is_none() {
            if let Some(next) = self.queue.drain(..1).next() {
                self.current = Some(next);
            }
        }

        // 更新并绘制当前通知
        if let Some(ref mut entry) = self.current {
            entry.elapsed += delta;

            // 计算透明度（淡出阶段）
            let alpha = if entry.elapsed >= entry.fade_out_start {
                let fade_progress = (entry.elapsed - entry.fade_out_start) / (entry.duration - entry.fade_out_start);
                (1.0 - fade_progress).max(0.0)
            } else {
                1.0
            };

            if alpha > 0.01 {
                // 精确测量文字宽度以居中
                let measured = measure_text_cn(&entry.text, 16.0);
                let box_w = measured.width.max(100.0) + 30.0;
                let box_h = 40.0;
                let box_x = (screen_w - box_w) / 2.0;
                let box_y = screen_h * 0.3;

                let bg_color = Color::from_rgba(20, 20, 20, (alpha * 180.0) as u8);
                draw_rectangle(box_x, box_y, box_w, box_h, bg_color);

                // 边框
                let border_color = Color::from_rgba(200, 180, 100, (alpha * 200.0) as u8);
                draw_line(box_x, box_y, box_x + box_w, box_y, 1.0, border_color);
                draw_line(box_x + box_w, box_y, box_x + box_w, box_y + box_h, 1.0, border_color);
                draw_line(box_x + box_w, box_y + box_h, box_x, box_y + box_h, 1.0, border_color);
                draw_line(box_x, box_y + box_h, box_x, box_y, 1.0, border_color);

                // 文字居中
                let text_color = Color::from_rgba(255, 255, 200, (alpha * 255.0) as u8);
                let text_x = (screen_w - measured.width) / 2.0;
                draw_text_cn(&entry.text, text_x, box_y + 22.0, 16.0, text_color);
            }

            // 通知到期，移除
            if entry.elapsed >= entry.duration {
                self.current = None;
            }
        }
    }
}
