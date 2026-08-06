// ============================================================================
// ChatNoticeDialogHybrid - 屏幕中央 transient 通知
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/ChatNoticeDialog.cs
// - 半透明屏幕中央通知，用于重要系统提示（如中毒、任务完成等）
// - 自动淡出（fade out）
// - 可叠加多个通知，排队显示
// - 两种 type：
//     type=0: 10F 字体, 文本框 Y 偏移 -6, 背景图 Prguse[1361]/Layout[1360]
//     type=1: 15F 字体, 文本框 Y 偏移  0, 背景图 Prguse[1363]/Layout[1362]
// - ViewTime=10000 (10 秒), 半透明背景图 Opacity=0.7F
// - 黄字 + 黑描边 (Yellow/Black)
//
// ============================================================================

use macroquad::prelude::*;

use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_with_outline, measure_text_cn};

/// 通知类型
///
/// 与 C# ShowNotice(text, type) 的 type 参数对应：
/// - type 0: 较小字体 (10F), 文本框上移 6px
/// - type 1: 较大字体 (15F), 文本框 Y 不偏移
pub const NOTICE_TYPE_SMALL: u8 = 0;
pub const NOTICE_TYPE_LARGE: u8 = 1;

/// C# ViewTime = 10000ms
pub const DEFAULT_NOTICE_DURATION: f32 = 10.0;

/// 单个通知条目
struct NoticeEntry {
    text: String,
    elapsed: f32,        // 已显示时间（秒）
    duration: f32,       // 总显示时长（秒）
    fade_out_start: f32, // 开始淡出的时间点
    notice_type: u8,     // 0=小字体, 1=大字体
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
    ///
    /// `notice_type`: 0=小字体(10F, 上移6px), 1=大字体(15F, 不偏移)，对应 C# type
    pub fn push_notice(&mut self, text: String, duration: f32, notice_type: u8) {
        let fade_out_start = duration * 0.7; // 最后 30% 时间淡出
        self.queue.push(NoticeEntry {
            text,
            elapsed: 0.0,
            duration,
            fade_out_start,
            notice_type,
        });
    }

    /// 推送默认时长（10 秒，对齐 C# ViewTime）的通知
    pub fn push_notice_default(&mut self, text: String) {
        self.push_notice(text, DEFAULT_NOTICE_DURATION, NOTICE_TYPE_LARGE);
    }

    /// 根据 type 选择字体大小（对齐 C# 10F/15F）
    fn font_size_for(notice_type: u8) -> f32 {
        match notice_type {
            NOTICE_TYPE_SMALL => 10.0,
            _ => 15.0,
        }
    }

    /// 根据 type 选择背景图索引（对齐 C# 1361/1363）
    fn bg_index_for(notice_type: u8) -> usize {
        match notice_type {
            NOTICE_TYPE_SMALL => 1361,
            _ => 1363,
        }
    }

    /// 根据 type 选择前景图索引（对齐 C# Layout 1360/1362）
    fn fg_index_for(notice_type: u8) -> usize {
        match notice_type {
            NOTICE_TYPE_SMALL => 1360,
            _ => 1362,
        }
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
                let fade_progress = (entry.elapsed - entry.fade_out_start)
                    / (entry.duration - entry.fade_out_start);
                (1.0 - fade_progress).max(0.0)
            } else {
                1.0
            };

            if alpha > 0.01 {
                let font_size = Self::font_size_for(entry.notice_type);
                // C#: 文本框 Size=660x40
                let box_w = 660.0;
                let box_h = 40.0;
                let box_x = (screen_w - box_w) / 2.0;
                // C#: Location = (ScreenWidth/2 - Size.Width/2, ScreenHeight/6 - Size.Height/2)
                let box_y = screen_h / 6.0 - box_h / 2.0;
                // C# type=0 文本框 Location.Y = -6 (相对父)，type=1 = 0
                let text_y_offset = if entry.notice_type == NOTICE_TYPE_SMALL {
                    -6.0
                } else {
                    0.0
                };

                // 背景贴图（C# Opacity=0.7F）。若资源缺失则回退到纯色矩形。
                let mut drew_bg = false;
                if let Some(info) =
                    LibraryName::Prguse.get_texture(Self::bg_index_for(entry.notice_type))
                {
                    if let Some(tex) = info.image {
                        // 缩放到框大小并应用半透明 (alpha ≈ 0.7)
                        let tint = Color::from_rgba(255, 255, 255, (alpha * 178.0) as u8);
                        draw_texture_ex(
                            &tex,
                            box_x,
                            box_y,
                            tint,
                            DrawTextureParams {
                                dest_size: Some(vec2(box_w, box_h)),
                                ..Default::default()
                            },
                        );
                        drew_bg = true;
                    }
                }
                if !drew_bg {
                    let bg_color = Color::from_rgba(20, 20, 20, (alpha * 180.0) as u8);
                    draw_rectangle(box_x, box_y, box_w, box_h, bg_color);

                    // 边框
                    let border_color = Color::from_rgba(200, 180, 100, (alpha * 200.0) as u8);
                    draw_line(box_x, box_y, box_x + box_w, box_y, 1.0, border_color);
                    draw_line(
                        box_x + box_w,
                        box_y,
                        box_x + box_w,
                        box_y + box_h,
                        1.0,
                        border_color,
                    );
                    draw_line(
                        box_x + box_w,
                        box_y + box_h,
                        box_x,
                        box_y + box_h,
                        1.0,
                        border_color,
                    );
                    draw_line(box_x, box_y + box_h, box_x, box_y, 1.0, border_color);
                }

                // 前景叠加贴图（C# Layout 1360/1362）
                if let Some(info) =
                    LibraryName::Prguse.get_texture(Self::fg_index_for(entry.notice_type))
                {
                    if let Some(tex) = info.image {
                        let tint = Color::from_rgba(255, 255, 255, (alpha * 255.0) as u8);
                        draw_texture_ex(
                            &tex,
                            box_x,
                            box_y,
                            tint,
                            DrawTextureParams {
                                dest_size: Some(vec2(box_w, box_h)),
                                ..Default::default()
                            },
                        );
                    }
                }

                // 文字：水平居中 + 真正垂直居中（C# VerticalCenter）
                let measured = measure_text_cn(&entry.text, font_size);
                let text_x = box_x + (box_w - measured.width) / 2.0;
                // draw_text_cn 的 y 是基线顶部；垂直居中
                let text_y = box_y + text_y_offset + (box_h - font_size) / 2.0;

                // C#: ForeColour=Yellow, OutLineColour=Black
                let yellow = Color::from_rgba(255, 255, 0, (alpha * 255.0) as u8);
                let black = Color::from_rgba(0, 0, 0, (alpha * 255.0) as u8);
                draw_text_with_outline(&entry.text, text_x, text_y, font_size, yellow, black);
            }

            // 通知到期，移除
            if entry.elapsed >= entry.duration {
                self.current = None;
            }
        }
    }
}
