use macroquad::prelude::*;

use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};

pub fn draw_input_box(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: &str,
    is_password: bool,
    is_focused: bool,
    cursor_visible: bool,
    font_size: f32,
) {
    // 绘制背景
    let bg_color = if is_focused {
        Color::from_rgba(40, 40, 50, 255)
    } else {
        Color::from_rgba(30, 30, 40, 255)
    };
    draw_rectangle(x, y, width, height, bg_color);

    // 绘制边框
    let border_color = if is_focused {
        Color::from_rgba(100, 150, 200, 255)
    } else {
        Color::from_rgba(60, 60, 80, 255)
    };
    draw_rectangle_lines(x, y, width, height, 1.0, border_color);

    // 绘制文本
    let display_text = if is_password {
        "*".repeat(text.len())
    } else {
        text.to_string()
    };

    let text_y = y + height / 2.0 + 5.0;
    draw_text_cn(&display_text, x + 5.0, text_y, font_size, WHITE);

    // 绘制光标
    if is_focused && cursor_visible {
        let text_width = measure_text_cn(&display_text, font_size).width;
        let cursor_x = x + 5.0 + text_width;
        draw_line(cursor_x, y + 3.0, cursor_x, y + height - 3.0, 1.0, WHITE);
    }
}

pub fn draw_button(
    library: LibraryName,
    x: f32,
    y: f32,
    normal_idx: usize,
    hover_idx: usize,
    pressed_idx: usize,
    enabled: bool,
) -> bool {
    let (mx, my) = mouse_position();

    let btn_size = if let Some(info) = library.get_texture(normal_idx) {
        (info.width as f32, info.height as f32)
    } else {
        (80.0, 25.0)
    };

    let is_hovered = enabled && mx >= x && mx <= x + btn_size.0 && my >= y && my <= y + btn_size.1;
    let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

    let texture_idx = if is_pressed {
        pressed_idx
    } else if is_hovered {
        hover_idx
    } else {
        normal_idx
    };

    let tint = if enabled {
        WHITE
    } else {
        Color::from_rgba(180, 180, 180, 255)
    };

    if let Some(info) = library.get_texture(texture_idx) {
        if let Some(ref tex) = info.image {
            draw_texture(tex, x, y, tint);
        }
    } else {
        // 降级绘制
        let color = if is_pressed {
            Color::from_rgba(100, 100, 150, 255)
        } else if is_hovered {
            Color::from_rgba(80, 80, 100, 255)
        } else {
            Color::from_rgba(60, 60, 80, 255)
        };
        draw_rectangle(x, y, btn_size.0, btn_size.1, color);
        draw_rectangle_lines(x, y, btn_size.0, btn_size.1, 1.0, WHITE);
    }

    enabled && is_hovered && is_mouse_button_pressed(MouseButton::Left)
}

pub fn draw_multiline_text_cn(text: &str, x: f32, mut y: f32, font_size: f32, color: Color) {
    let line_h = font_size + 2.0;
    for line in text.lines() {
        draw_text_cn(line, x, y, font_size, color);
        y += line_h;
    }
}

pub fn draw_message_box(message_text: &str) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    let box_w = 300.0;
    let box_h = 150.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = (screen_h - box_h) / 2.0;

    // 背景
    draw_rectangle(
        box_x,
        box_y,
        box_w,
        box_h,
        Color::from_rgba(40, 40, 50, 240),
    );
    draw_rectangle_lines(
        box_x,
        box_y,
        box_w,
        box_h,
        2.0,
        Color::from_rgba(100, 100, 120, 255),
    );

    // 标题
    draw_text_cn(
        "提示",
        box_x + box_w / 2.0 - 15.0,
        box_y + 30.0,
        18.0,
        WHITE,
    );

    // 消息文本
    let text_width = measure_text_cn(message_text, 14.0).width;
    draw_text_cn(
        message_text,
        box_x + (box_w - text_width) / 2.0,
        box_y + 70.0,
        14.0,
        WHITE,
    );
}

/// 更紧凑的消息框（用于 SelectScene），尺寸与按钮位置由调用方传入。
///
/// - 不绘制标题
/// - 文本垂直位置与历史实现保持一致
pub fn draw_message_box_compact(message_text: &str, dialog_w: f32, dialog_h: f32) {
    let dialog = compact_message_box_rect(dialog_w, dialog_h);
    let dialog_x = dialog.x;
    let dialog_y = dialog.y;

    // 背景
    draw_rectangle(
        dialog_x,
        dialog_y,
        dialog_w,
        dialog_h,
        Color::from_rgba(40, 40, 50, 240),
    );
    draw_rectangle_lines(
        dialog_x,
        dialog_y,
        dialog_w,
        dialog_h,
        2.0,
        Color::from_rgba(100, 100, 120, 255),
    );

    // 消息
    let text_width = measure_text_cn(message_text, 14.0).width;
    draw_text_cn(
        message_text,
        dialog_x + (dialog_w - text_width) / 2.0,
        dialog_y + 50.0,
        14.0,
        WHITE,
    );

    // 确定按钮（位置/尺寸保持与历史实现一致）
    let btn = compact_message_box_ok_button_rect(dialog);
    let btn_x = btn.x;
    let btn_y = btn.y;
    let (mx, my) = mouse_position();
    let hovered = mx >= btn_x && mx <= btn_x + btn.w && my >= btn_y && my <= btn_y + btn.h;
    let color = if hovered {
        Color::from_rgba(80, 120, 180, 255)
    } else {
        Color::from_rgba(60, 80, 120, 255)
    };
    draw_rectangle(btn_x, btn_y, btn.w, btn.h, color);
    draw_rectangle_lines(
        btn_x,
        btn_y,
        btn.w,
        btn.h,
        1.0,
        Color::from_rgba(100, 120, 150, 255),
    );
    draw_text_cn("确定", btn_x + 22.0, btn_y + 22.0, 14.0, WHITE);
}

/// MirMessageBox(OK) 风格（对齐 C#）：
/// - 背景：Prguse[360]
/// - 文本区域：起点 (35,35)
/// - OK 按钮：Title[200/201/202] at (360,157)
///
/// 返回 true 表示 OK 被点击（调用方应关闭消息框）。
pub fn draw_mir_message_box_ok(message_text: &str) -> bool {
    let (dialog_w, dialog_h) = if let Some(info) = LibraryName::Prguse.get_texture(360) {
        (info.width as f32, info.height as f32)
    } else {
        (460.0, 210.0)
    };

    let dialog = begin_modal(dialog_w, dialog_h, 200);
    let dialog_x = dialog.x;
    let dialog_y = dialog.y;

    // 背景纹理 Prguse[360]（缺失时回退到通用面板）
    if let Some(info) = LibraryName::Prguse.get_texture(360) {
        if let Some(ref tex) = info.image {
            draw_texture(
                tex,
                // ClientRust message_box 使用 draw_sprite_at（不应用 offset）
                dialog_x, dialog_y, WHITE,
            );
        } else {
            draw_dialog_panel(
                dialog_x,
                dialog_y,
                dialog_w,
                dialog_h,
                Color::from_rgba(40, 40, 50, 240),
                Color::from_rgba(100, 100, 120, 255),
            );
        }
    } else {
        draw_dialog_panel(
            dialog_x,
            dialog_y,
            dialog_w,
            dialog_h,
            Color::from_rgba(40, 40, 50, 240),
            Color::from_rgba(100, 100, 120, 255),
        );
    }

    // 文本（按换行绘制；与 C# MirLabel 位置一致）
    let mut y = dialog_y + 60.0;
    for line in message_text.lines() {
        draw_text_cn(line, dialog_x + 35.0, y, 14.0, WHITE);
        y += 16.0;
    }

    // OK 按钮：Title 200/201/202 at (360,157)
    draw_button(
        LibraryName::Title,
        dialog_x + 360.0,
        dialog_y + 157.0,
        200,
        201,
        202,
        true,
    )
}

/// SelectScene 的紧凑消息框（300x120）使用的居中 Rect。
pub fn compact_message_box_rect(dialog_w: f32, dialog_h: f32) -> Rect {
    centered_rect(dialog_w, dialog_h)
}

/// SelectScene 的紧凑消息框 OK 按钮 Rect。
pub fn compact_message_box_ok_button_rect(dialog: Rect) -> Rect {
    // 与历史实现一致：按钮宽高 80x30；x=dialog_center-40；y=dialog_bottom-45
    Rect::new(
        dialog.x + dialog.w / 2.0 - 40.0,
        dialog.y + dialog.h - 45.0,
        80.0,
        30.0,
    )
}

pub fn draw_text_input_box(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: &str,
    is_focused: bool,
    cursor_visible: bool,
    font_size: f32,
    bg_color: Color,
    border_color: Color,
    text_color: Color,
    cursor_color: Color,
) {
    draw_rectangle(x, y, width, height, bg_color);
    draw_rectangle_lines(x, y, width, height, 1.0, border_color);

    // 这里维持与历史实现一致：padding=5，baseline=y+18
    draw_text_cn(text, x + 5.0, y + 18.0, font_size, text_color);

    if is_focused && cursor_visible {
        let text_width = measure_text_cn(text, font_size).width;
        let cursor_x = x + 5.0 + text_width;
        draw_line(
            cursor_x,
            y + 3.0,
            cursor_x,
            y + height - 3.0,
            1.0,
            cursor_color,
        );
    }
}

pub fn draw_rect_text_button(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    label: &str,
    font_size: f32,
    normal_color: Color,
    hover_color: Color,
    border_color: Color,
    text_color: Color,
    disabled_color: Color,
    disabled_text_color: Color,
    enabled: bool,
) -> bool {
    let (mx, my) = mouse_position();
    let hovered = enabled && mx >= x && mx <= x + width && my >= y && my <= y + height;

    let fill = if enabled {
        if hovered {
            hover_color
        } else {
            normal_color
        }
    } else {
        disabled_color
    };
    let tcolor = if enabled {
        text_color
    } else {
        disabled_text_color
    };

    draw_rectangle(x, y, width, height, fill);
    draw_rectangle_lines(x, y, width, height, 1.0, border_color);

    // 与原实现一致：由调用方给出具体 x 偏移（这里做居中绘制）
    let text_size = measure_text_cn(label, font_size);
    let tx = x + (width - text_size.width) / 2.0;
    let ty = y + height * 0.73;
    draw_text_cn(label, tx, ty, font_size, tcolor);

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

pub fn draw_dialog_panel(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    bg_color: Color,
    border_color: Color,
) {
    draw_rectangle(x, y, width, height, bg_color);
    draw_rectangle_lines(x, y, width, height, 2.0, border_color);
}

pub fn draw_dialog_title_centered(
    title: &str,
    dialog_x: f32,
    dialog_y: f32,
    dialog_w: f32,
    y_offset: f32,
    font_size: f32,
    color: Color,
) {
    let text_width = measure_text_cn(title, font_size).width;
    let x = dialog_x + (dialog_w - text_width) / 2.0;
    draw_text_cn(title, x, dialog_y + y_offset, font_size, color);
}

pub fn centered_rect(width: f32, height: f32) -> Rect {
    let x = (screen_width() - width) / 2.0;
    let y = (screen_height() - height) / 2.0;
    Rect::new(x, y, width, height)
}

pub fn draw_modal_overlay(alpha: u8) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, alpha),
    );
}

/// 开始绘制一个模态弹窗：先画遮罩，再返回居中的弹窗 Rect。
pub fn begin_modal(width: f32, height: f32, overlay_alpha: u8) -> Rect {
    draw_modal_overlay(overlay_alpha);
    centered_rect(width, height)
}
