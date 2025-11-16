// 测试 MainDialog

use client_macroquad::scenes::dialogs::{MainDialog, BeltDialog, ChatDialog, Dialog};
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

    let screen_h = screen_height() / screen_dpi_scale();
    let screen_w = screen_width() / screen_dpi_scale();
    
    // 计算 main_dialog_x
    let main_dialog_x = (screen_w - 1024.0) / 2.0;
    
    // 创建对话框
    let mut main_dialog = MainDialog::new();
    let mut belt_dialog = BeltDialog::new(main_dialog_x, screen_h);
    let mut chat_dialog = ChatDialog::new(main_dialog_x, screen_h, 1); // 1024分辨率
    let mut belt_open = true;
    let mut chat_open = true;
    
    // 添加欢迎消息
    chat_dialog.add_message(
        "Welcome to the Legend of Legend Mir 2 Server.",
        egui_macroquad::egui::Color32::YELLOW,
    );
    chat_dialog.add_message(
        "[Mode: Peaceful]",
        egui_macroquad::egui::Color32::GREEN,
    );
    chat_dialog.add_message(
        "[Pet: Attack and Move]",
        egui_macroquad::egui::Color32::LIGHT_BLUE,
    );
    
    println!("✅ MainDialog、BeltDialog 和 ChatDialog 已创建");
    println!("💡 提示: ");
    println!("   - 按 Enter 键显示聊天输入框");
    println!("   - 按 B 键切换快捷栏布局");
    println!("   - 按 ESC 退出");

    loop {
        clear_background(Color::from_rgba(60, 80, 100, 255));

        // Enter 键显示聊天输入框
        if is_key_pressed(KeyCode::Enter) {
            chat_dialog.show_input();
            println!("💬 显示聊天输入框");
        }

        // B 键切换快捷栏布局
        if is_key_pressed(KeyCode::B) {
            belt_dialog.flip_layout();
            println!("🔄 切换快捷栏布局");
        }

        // 绘制背景提示
        let text = "游戏主场景 - Enter 打开聊天, B 切换快捷栏";
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
            // 绘制顺序：BeltDialog -> ChatDialog -> MainDialog
            use client_macroquad::scenes::dialogs::Dialog;
            belt_dialog.show(ctx, &mut belt_open);
            chat_dialog.show(ctx, &mut chat_open);
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
