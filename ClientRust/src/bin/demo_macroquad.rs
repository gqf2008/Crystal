// ============================================================================
// Macroquad 最小演示 - 完全独立
// ============================================================================
//
// 这是一个独立的 macroquad 演示程序，展示：
// - macroquad 的基本使用
// - 跨平台渲染能力
// - 中文字体支持
//
// 用法：
//   cargo run --bin demo_macroquad --no-default-features --features backend-macroquad
//
// ============================================================================

use macroquad::prelude::*;

/// macroquad 窗口配置
fn window_conf() -> Conf {
    Conf {
        window_title: "Macroquad 演示 - Crystal".to_owned(),
        window_width: 1280,
        window_height: 960,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🚀 Macroquad 演示启动");

    // 简单的动画状态
    let mut rotation = 0.0f32;
    let mut zoom = 1.0f32;
    let mut show_grid = false;

    // 加载字体（如果存在）
    let font = load_ttf_font("resources/font/AlibabaPuHuiTi-3-55-Regular.ttf")
        .await
        .ok();

    // FPS 计数
    let mut frame_count = 0u32;
    let mut fps_timer = 0.0f64;
    let mut current_fps = 0u32;

    println!("✅ 初始化完成，进入主循环");

    loop {
        let dt = get_frame_time();

        // FPS 计算
        frame_count += 1;
        fps_timer += dt as f64;
        if fps_timer >= 1.0 {
            current_fps = frame_count;
            frame_count = 0;
            fps_timer -= 1.0;
        }

        // 键盘输入
        if is_key_pressed(KeyCode::Escape) {
            println!("👋 用户按下 ESC，退出程序");
            break;
        }

        if is_key_pressed(KeyCode::G) {
            show_grid = !show_grid;
            println!("🔲 网格显示: {}", show_grid);
        }

        // 鼠标滚轮缩放
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            zoom *= 1.0 + wheel * 0.1;
            zoom = zoom.clamp(0.5, 3.0);
        }

        // 旋转动画
        rotation += dt;

        // ========== 渲染 ==========
        clear_background(Color::from_rgba(30, 30, 40, 255));

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // 绘制旋转的矩形
        draw_rectangle(
            center_x - 50.0 * zoom,
            center_y - 50.0 * zoom,
            100.0 * zoom,
            100.0 * zoom,
            Color::from_rgba(100, 149, 237, 255), // 矢车菊蓝
        );

        // 绘制旋转的圆形（演示动画）
        let circle_x = center_x + rotation.cos() * 150.0;
        let circle_y = center_y + rotation.sin() * 150.0;
        draw_circle(circle_x, circle_y, 30.0 * zoom, RED);

        // 绘制网格
        if show_grid {
            let grid_size = 50.0;
            let grid_color = Color::from_rgba(255, 255, 255, 50);

            // 垂直线
            let mut x = 0.0;
            while x < screen_width() {
                draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
                x += grid_size;
            }

            // 水平线
            let mut y = 0.0;
            while y < screen_height() {
                draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
                y += grid_size;
            }
        }

        // 绘制 UI 文本
        let text_color = WHITE;
        let title = "🎮 Macroquad 演示 - Crystal";
        let info = format!(
            "FPS: {} | 缩放: {:.1}x | 网格: {} | 屏幕: {:.0}x{:.0}",
            current_fps,
            zoom,
            if show_grid { "开" } else { "关" },
            screen_width(),
            screen_height()
        );
        let controls = "控制: 滚轮缩放 | G 切换网格 | ESC 退出";

        if let Some(font) = font {
            // 使用自定义字体
            draw_text_ex(
                title,
                10.0,
                30.0,
                TextParams {
                    font: Some(&font),
                    font_size: 24,
                    color: text_color,
                    ..Default::default()
                },
            );

            draw_text_ex(
                &info,
                10.0,
                60.0,
                TextParams {
                    font: Some(&font),
                    font_size: 16,
                    color: text_color,
                    ..Default::default()
                },
            );

            draw_text_ex(
                controls,
                10.0,
                90.0,
                TextParams {
                    font: Some(&font),
                    font_size: 14,
                    color: Color::from_rgba(200, 200, 200, 255),
                    ..Default::default()
                },
            );
        } else {
            // 使用默认字体
            draw_text(title, 10.0, 30.0, 24.0, text_color);
            draw_text(&info, 10.0, 60.0, 16.0, text_color);
            draw_text(controls, 10.0, 90.0, 14.0, text_color);
        }

        // 底部状态栏
        let status_y = screen_height() - 30.0;
        draw_rectangle(
            0.0,
            status_y,
            screen_width(),
            30.0,
            Color::from_rgba(0, 0, 0, 180),
        );

        let status_text = format!(
            "✅ Macroquad 运行正常 | 帧时间: {:.2}ms | 鼠标: ({:.0}, {:.0})",
            dt * 1000.0,
            mouse_position().0,
            mouse_position().1
        );

        if let Some(font) = font {
            draw_text_ex(
                &status_text,
                10.0,
                status_y + 20.0,
                TextParams {
                    font: Some(&font),
                    font_size: 14,
                    color: Color::from_rgba(100, 255, 100, 255),
                    ..Default::default()
                },
            );
        } else {
            draw_text(
                &status_text,
                10.0,
                status_y + 20.0,
                14.0,
                Color::from_rgba(100, 255, 100, 255),
            );
        }

        next_frame().await
    }

    println!("👋 程序正常退出");
}
