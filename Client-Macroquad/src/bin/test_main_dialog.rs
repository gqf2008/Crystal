// 测试 MainDialog

use client_macroquad::scenes::dialogs::MainDialog;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - MainDialog 测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - MainDialog 测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 创建 MainDialog
    let mut main_dialog = MainDialog::new();
    
    println!("✅ MainDialog 已创建");
    println!("💡 提示: ");
    println!("   - 按 Enter 键显示聊天输入框");
    println!("   - 在输入框中输入文字后按 Enter 发送");
    println!("   - 按 ESC 取消输入或退出");

    loop {
        clear_background(Color::from_rgba(60, 80, 100, 255));

        // 检测 Enter 键显示聊天输入框
        if is_key_pressed(KeyCode::Enter) {
            main_dialog.show_chat_input();
            println!("💬 显示聊天输入框");
        }

        // 绘制背景提示
        let text = "游戏主场景 - 按 Enter 打开聊天";
        let font_size = 36.0;
        let text_size = measure_text(&text, None, font_size as u16, 1.0);
        draw_text(
            text,
            screen_width() / 2.0 - text_size.width / 2.0,
            screen_height() / 2.0 - 100.0,
            font_size,
            WHITE,
        );

        // egui UI
        egui_macroquad::ui(|ctx| {
            main_dialog.show(ctx);
        });

        // 绘制 egui
        egui_macroquad::draw();

        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            println!("👋 退出测试");
            break;
        }

        next_frame().await;
    }
}
