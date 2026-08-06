// ============================================================================
// TimerDialogHybrid - 服务器倒计时对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/TimerDialog.cs (~246 行)
// - 显示服务器推送的倒计时（蛋形计时器风格）
// - 固定在屏幕右下角
// - 多个计时器同时显示
// - 倒计时到期自动消失
//
// ============================================================================

use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;

/// 单个计时器条目
#[derive(Debug, Clone)]
pub struct TimerEntry {
    pub timer_id: u8,
    pub remaining_seconds: u32,
}

/// 服务器倒计时对话框
#[derive(Default)]
pub struct TimerDialogHybrid {
    pub visible: bool,
    pub timers: Vec<TimerEntry>,
}

impl TimerDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置/更新一个计时器
    pub fn set_timer(&mut self, timer_id: u8, seconds: u32) {
        if let Some(entry) = self.timers.iter_mut().find(|t| t.timer_id == timer_id) {
            entry.remaining_seconds = seconds;
        } else {
            self.timers.push(TimerEntry {
                timer_id,
                remaining_seconds: seconds,
            });
        }
        self.visible = true;
    }

    /// 移除到期的计时器
    pub fn remove_timer(&mut self, timer_id: u8) {
        self.timers.retain(|t| t.timer_id != timer_id);
        if self.timers.is_empty() {
            self.visible = false;
        }
    }

    /// 每帧更新倒计时（由 render 阶段调用）
    pub fn update_delta(&mut self, delta: f32) {
        for entry in &mut self.timers {
            entry.remaining_seconds = entry.remaining_seconds.saturating_sub(delta as u32);
        }
        // 清理到期计时器
        self.timers.retain(|t| t.remaining_seconds > 0);
        if self.timers.is_empty() {
            self.visible = false;
        }
    }

    /// 格式化秒为 HH:MM:SS
    fn format_time(seconds: u32) -> String {
        let h = seconds / 3600;
        let m = (seconds % 3600) / 60;
        let s = seconds % 60;
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        }
    }

    /// 绘制倒计时对话框
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, delta: f32) {
        self.update_delta(delta);

        if !self.visible || self.timers.is_empty() {
            return;
        }

        // 固定在屏幕右下角
        let padding = 10.0;
        let item_height = 28.0;
        let item_width = 100.0;
        let total_height = self.timers.len() as f32 * item_height + padding * 2.0;

        let base_x = screen_w - item_width - padding;
        let base_y = screen_h - total_height - padding;

        // 背景
        let bg_color = Color::from_rgba(30, 30, 30, 200);
        draw_rectangle(base_x, base_y, item_width, total_height, bg_color);

        // 边框
        let border_color = Color::from_rgba(150, 120, 50, 200);
        draw_line(
            base_x,
            base_y,
            base_x + item_width,
            base_y,
            1.0,
            border_color,
        );
        draw_line(
            base_x + item_width,
            base_y,
            base_x + item_width,
            base_y + total_height,
            1.0,
            border_color,
        );
        draw_line(
            base_x + item_width,
            base_y + total_height,
            base_x,
            base_y + total_height,
            1.0,
            border_color,
        );
        draw_line(
            base_x,
            base_y + total_height,
            base_x,
            base_y,
            1.0,
            border_color,
        );

        // 每个计时器
        for (i, entry) in self.timers.iter().enumerate() {
            let y = base_y + padding + i as f32 * item_height;
            let time_str = Self::format_time(entry.remaining_seconds);

            // 倒计时文字（黄色）
            let text_color = if entry.remaining_seconds <= 10 {
                Color::from_rgba(255, 80, 80, 255) // 最后10秒变红
            } else {
                Color::from_rgba(255, 220, 50, 255) // 正常黄色
            };

            draw_text_cn(&time_str, base_x + 5.0, y + 18.0, 14.0, text_color);
        }
    }
}
