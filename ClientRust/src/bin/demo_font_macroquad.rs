// ============================================================================
// Macroquad 字体系统演示
// ============================================================================
//
// 展示字体加载、渲染、对齐、效果等功能
//
// 用法：
//   cargo run --bin demo_font_macroquad --no-default-features --features backend-macroquad
//
// ============================================================================

use macroquad::prelude::*;
use mir2_client::backends::macroquad::{FontManager, TextAlign, TextBuilder};

/// macroquad 窗口配置
fn window_conf() -> Conf {
    Conf {
        window_title: "Macroquad 字体系统演示 - Crystal".to_owned(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🚀 Macroquad 字体系统演示启动");

    // 创建字体管理器
    let mut font_mgr = FontManager::new();

    // 尝试加载中文字体
    let font_path = "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf";
    match font_mgr.load_font("chinese", font_path).await {
        Ok(_) => println!("✅ 成功加载中文字体: {}", font_path),
        Err(e) => {
            println!("⚠️  无法加载字体: {}", e);
            println!("   将使用 macroquad 内置字体");
        }
    }

    let mut demo_index = 0;
    let demo_count = 7;

    println!("✅ 初始化完成，进入主循环");
    println!("\n📖 操作说明:");
    println!("   ← →     : 切换演示");
    println!("   ESC     : 退出\n");

    loop {
        // ========== 输入处理 ==========
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        if is_key_pressed(KeyCode::Right) {
            demo_index = (demo_index + 1) % demo_count;
        }

        if is_key_pressed(KeyCode::Left) {
            demo_index = if demo_index == 0 {
                demo_count - 1
            } else {
                demo_index - 1
            };
        }

        // ========== 渲染 ==========
        clear_background(Color::from_rgba(30, 34, 42, 255));

        let screen_w = screen_width();
        let screen_h = screen_height();

        // 绘制标题
        TextBuilder::new(
            &font_mgr,
            &format!("演示 {}/{}", demo_index + 1, demo_count),
        )
        .position(screen_w / 2.0, 40.0)
        .font_size(32)
        .color(Color::from_rgba(255, 200, 100, 255))
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

        // 根据索引绘制不同的演示
        match demo_index {
            0 => demo_basic_text(&font_mgr, screen_w, screen_h),
            1 => demo_text_alignment(&font_mgr, screen_w, screen_h),
            2 => demo_font_sizes(&font_mgr, screen_w, screen_h),
            3 => demo_text_colors(&font_mgr, screen_w, screen_h),
            4 => demo_text_shadow(&font_mgr, screen_w, screen_h),
            5 => demo_text_outline(&font_mgr, screen_w, screen_h),
            6 => demo_multiline_text(&font_mgr, screen_w, screen_h),
            _ => {}
        }

        // 底部提示
        let help_text = "← → : 切换演示 | ESC : 退出";
        draw_rectangle(
            0.0,
            screen_h - 30.0,
            screen_w,
            30.0,
            Color::from_rgba(0, 0, 0, 200),
        );
        TextBuilder::new(&font_mgr, help_text)
            .position(screen_w / 2.0, screen_h - 10.0)
            .font_size(16)
            .color(Color::from_rgba(200, 200, 200, 255))
            .align(TextAlign::Center)
            .draw();

        next_frame().await;
    }

    println!("👋 演示结束");
}

/// 演示 1: 基础文本
fn demo_basic_text(font_mgr: &FontManager, _w: f32, h: f32) {
    let y_start = h * 0.25;

    TextBuilder::new(font_mgr, "基础文本渲染")
        .position(50.0, y_start)
        .font_size(28)
        .color(WHITE)
        .font("chinese")
        .draw();

    TextBuilder::new(font_mgr, "Hello, World!")
        .position(50.0, y_start + 50.0)
        .font_size(24)
        .color(SKYBLUE)
        .draw();

    TextBuilder::new(font_mgr, "你好，世界！🌍")
        .position(50.0, y_start + 100.0)
        .font_size(24)
        .color(YELLOW)
        .font("chinese")
        .draw();

    TextBuilder::new(font_mgr, "支持 Emoji: 😀 🎮 ⚔️ 🛡️")
        .position(50.0, y_start + 150.0)
        .font_size(24)
        .color(PINK)
        .font("chinese")
        .draw();
}

/// 演示 2: 文本对齐
fn demo_text_alignment(font_mgr: &FontManager, w: f32, h: f32) {
    let y_start = h * 0.25;
    let center_x = w / 2.0;

    TextBuilder::new(font_mgr, "文本对齐方式")
        .position(center_x, y_start)
        .font_size(28)
        .color(WHITE)
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

    // 绘制中线
    draw_line(
        center_x,
        y_start + 40.0,
        center_x,
        h - 100.0,
        2.0,
        Color::from_rgba(100, 100, 100, 255),
    );

    // 左对齐
    TextBuilder::new(font_mgr, "← 左对齐 (Left)")
        .position(center_x, y_start + 80.0)
        .font_size(20)
        .color(LIGHTGRAY)
        .align(TextAlign::Left)
        .font("chinese")
        .draw();

    // 居中对齐
    TextBuilder::new(font_mgr, "⬤ 居中对齐 (Center)")
        .position(center_x, y_start + 130.0)
        .font_size(20)
        .color(YELLOW)
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

    // 右对齐
    TextBuilder::new(font_mgr, "右对齐 (Right) →")
        .position(center_x, y_start + 180.0)
        .font_size(20)
        .color(LIGHTGRAY)
        .align(TextAlign::Right)
        .font("chinese")
        .draw();
}

/// 演示 3: 字体大小
fn demo_font_sizes(font_mgr: &FontManager, _w: f32, h: f32) {
    let y_start = h * 0.2;

    TextBuilder::new(font_mgr, "不同字体大小")
        .position(50.0, y_start)
        .font_size(28)
        .color(WHITE)
        .font("chinese")
        .draw();

    let sizes = [12, 16, 20, 24, 32, 40, 48];
    let mut y = y_start + 60.0;

    for size in sizes {
        TextBuilder::new(font_mgr, &format!("字号 {} - The Quick Brown Fox", size))
            .position(50.0, y)
            .font_size(size)
            .color(Color::from_rgba(200, 200, 255, 255))
            .font("chinese")
            .draw();
        y += size as f32 + 10.0;
    }
}

/// 演示 4: 文本颜色
fn demo_text_colors(font_mgr: &FontManager, w: f32, h: f32) {
    let y_start = h * 0.25;

    TextBuilder::new(font_mgr, "丰富的文本颜色")
        .position(w / 2.0, y_start)
        .font_size(28)
        .color(WHITE)
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

    let colors = [
        (RED, "红色 (Red)"),
        (ORANGE, "橙色 (Orange)"),
        (YELLOW, "黄色 (Yellow)"),
        (GREEN, "绿色 (Green)"),
        (SKYBLUE, "天蓝 (Sky Blue)"),
        (BLUE, "蓝色 (Blue)"),
        (PURPLE, "紫色 (Purple)"),
        (PINK, "粉色 (Pink)"),
    ];

    let mut y = y_start + 60.0;
    for (color, name) in colors {
        TextBuilder::new(font_mgr, name)
            .position(w / 2.0, y)
            .font_size(24)
            .color(color)
            .align(TextAlign::Center)
            .font("chinese")
            .draw();
        y += 40.0;
    }
}

/// 演示 5: 文本阴影
fn demo_text_shadow(font_mgr: &FontManager, w: f32, h: f32) {
    let y_start = h * 0.25;

    TextBuilder::new(font_mgr, "文本阴影效果")
        .position(w / 2.0, y_start)
        .font_size(28)
        .color(WHITE)
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

    let shadow_offsets = [1.0, 2.0, 3.0, 4.0, 5.0];
    let mut y = y_start + 80.0;

    for offset in shadow_offsets {
        TextBuilder::new(font_mgr, &format!("阴影偏移 {:.0} 像素", offset))
            .position(w / 2.0, y)
            .font_size(24)
            .color(YELLOW)
            .align(TextAlign::Center)
            .shadow(offset, Color::from_rgba(0, 0, 0, 200))
            .font("chinese")
            .draw();
        y += 50.0;
    }
}

/// 演示 6: 文本边框
fn demo_text_outline(font_mgr: &FontManager, w: f32, h: f32) {
    let y_start = h * 0.25;

    TextBuilder::new(font_mgr, "文本边框效果")
        .position(w / 2.0, y_start)
        .font_size(28)
        .color(WHITE)
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

    let texts = [
        ("白色文本 + 黑色边框", WHITE, BLACK),
        ("黄色文本 + 红色边框", YELLOW, RED),
        ("青色文本 + 蓝色边框", SKYBLUE, BLUE),
        ("粉色文本 + 紫色边框", PINK, PURPLE),
    ];

    let mut y = y_start + 80.0;

    for (text, color, outline_color) in texts {
        TextBuilder::new(font_mgr, text)
            .position(w / 2.0, y)
            .font_size(26)
            .color(color)
            .align(TextAlign::Center)
            .outline(outline_color)
            .font("chinese")
            .draw();
        y += 60.0;
    }
}

/// 演示 7: 多行文本
fn demo_multiline_text(font_mgr: &FontManager, w: f32, h: f32) {
    let y_start = h * 0.2;

    TextBuilder::new(font_mgr, "多行文本渲染")
        .position(w / 2.0, y_start)
        .font_size(28)
        .color(WHITE)
        .align(TextAlign::Center)
        .font("chinese")
        .draw();

    let multiline_text = "传奇世界 - Legend of Mir 2\n\
                          经典 MMORPG 游戏\n\
                          Rust 重制版\n\
                          \n\
                          使用 macroquad 渲染引擎\n\
                          支持跨平台部署：\n\
                          • Windows / macOS / Linux\n\
                          • Web (WASM)\n\
                          • iOS / Android";

    font_mgr.draw_text_multiline(
        multiline_text,
        50.0,
        y_start + 60.0,
        20,
        30.0,
        Color::from_rgba(200, 220, 255, 255),
        Some("chinese"),
    );
}
