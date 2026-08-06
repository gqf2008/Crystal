// ============================================================================
// RollDialogHybrid - 骰子/占卜小游戏对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/RollDialog.cs (~199 行)
// - 显示骰子结果（1-6 或更高）
// - 带滚动动画
// - 屏幕中央弹出，3秒后自动关闭
//
// ============================================================================

use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;

/// 骰子对话框
pub struct RollDialogHybrid {
    pub visible: bool,
    result: u32,
    elapsed: f32,
    /// 是否正在滚动动画中
    is_rolling: bool,
    /// 滚动阶段（0-1 表示动画进度）
    roll_phase: f32,
    /// 当前显示的伪随机结果（动画期间）
    display_value: u32,
}

impl Default for RollDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            result: 0,
            elapsed: 0.0,
            is_rolling: false,
            roll_phase: 0.0,
            display_value: 1,
        }
    }
}

impl RollDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示骰子结果
    pub fn show_roll(&mut self, value: u32) {
        self.result = value;
        self.elapsed = 0.0;
        self.is_rolling = true;
        self.roll_phase = 0.0;
        self.display_value = 1;
        self.visible = true;
    }

    /// 更新并绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, delta: f32) -> bool {
        if !self.visible {
            return false;
        }

        self.elapsed += delta;

        // 滚动动画（前 1.5 秒）
        if self.is_rolling {
            self.roll_phase += delta * 4.0;
            if self.roll_phase >= 1.0 {
                self.is_rolling = false;
                self.display_value = self.result;
            } else {
                // 随机滚动数字
                self.display_value = (self.elapsed * 15.0) as u32 % 6 + 1;
            }
        }

        // 3 秒后自动关闭
        if self.elapsed > 3.0 {
            self.visible = false;
            return false;
        }

        // 计算透明度（最后 1 秒淡出）
        let alpha = if self.elapsed > 2.0 {
            1.0 - (self.elapsed - 2.0)
        } else {
            1.0
        }
        .clamp(0.0, 1.0);

        // 背景
        let box_w = 120.0;
        let box_h = 120.0;
        let box_x = (screen_w - box_w) / 2.0;
        let box_y = (screen_h - box_h) / 2.0;

        let bg_color = Color::from_rgba(30, 30, 30, (alpha * 220.0) as u8);
        draw_rectangle(box_x, box_y, box_w, box_h, bg_color);

        // 边框（金色）
        let border_color = Color::from_rgba(200, 170, 50, (alpha * 255.0) as u8);
        draw_line(box_x, box_y, box_x + box_w, box_y, 2.0, border_color);
        draw_line(
            box_x + box_w,
            box_y,
            box_x + box_w,
            box_y + box_h,
            2.0,
            border_color,
        );
        draw_line(
            box_x + box_w,
            box_y + box_h,
            box_x,
            box_y + box_h,
            2.0,
            border_color,
        );
        draw_line(box_x, box_y + box_h, box_x, box_y, 2.0, border_color);

        // 标题
        draw_text_cn(
            "🎲 掷骰子",
            box_x + 20.0,
            box_y + 20.0,
            14.0,
            Color::from_rgba(255, 220, 100, (alpha * 255.0) as u8),
        );

        // 骰子结果（大字）
        let result_text = format!("{}", self.display_value);
        draw_text_cn(
            &result_text,
            box_x + 35.0,
            box_y + 50.0,
            40.0,
            Color::from_rgba(255, 255, 255, (alpha * 255.0) as u8),
        );

        // 提示文字
        if !self.is_rolling {
            draw_text_cn(
                "结果",
                box_x + 35.0,
                box_y + 95.0,
                12.0,
                Color::from_rgba(200, 200, 200, (alpha * 255.0) as u8),
            );
        }

        true
    }
}
