// 测试 ChatDialog 和 ChatControlBar

use client_macroquad::scenes::dialogs::game::{ChatControlBar, ChatDialog};
use client_macroquad::scenes::dialogs::Dialog;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - ChatDialog + ChatControlBar 测试".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - ChatDialog + ChatControlBar 测试");
    println!("📐 窗口尺寸: {}x{}", screen_width(), screen_height());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 配置 egui（只设置一次）
    egui_macroquad::cfg(|ctx| {
        let mut fonts = egui_macroquad::egui::FontDefinitions::default();
        
        // 加载中文字体
        let font_data = std::fs::read("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf")
            .or_else(|_| std::fs::read("assets/fonts/Chinese.ttc"))
            .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\msyh.ttc"))
            .unwrap_or_else(|_| {
                println!("⚠️  无法加载中文字体，使用默认字体");
                vec![]
            });

        if !font_data.is_empty() {
            fonts.font_data.insert(
                "chinese".to_owned(),
                std::sync::Arc::new(egui_macroquad::egui::FontData::from_owned(font_data)),
            );

            // 设置字体优先级
            fonts
                .families
                .get_mut(&egui_macroquad::egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "chinese".to_owned());

            fonts
                .families
                .get_mut(&egui_macroquad::egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "chinese".to_owned());
                
            println!("✅ 已加载中文字体");
        }

        ctx.set_fonts(fonts);

        // 设置 DPI 缩放
        let dpi_scale = screen_dpi_scale();
        ctx.set_pixels_per_point(dpi_scale);
    });

    let screen_h = screen_height() / screen_dpi_scale();
    let _screen_w = screen_width() / screen_dpi_scale();
    
    // 模拟 MainDialog 的 X 坐标
    let main_dialog_x = 0.0;
    
    // 创建 ChatControlBar（位于屏幕底部上方）
    // 位置：MainDialog.X + 230, ScreenHeight - 112
    let mut chat_control_bar = ChatControlBar::new(main_dialog_x, screen_h, 1); // 1024分辨率
    let mut control_bar_open = true;
    println!("✅ ChatControlBar 已创建");
    
    // 创建 ChatDialog
    // 位置：MainDialog.X + 230, ScreenHeight - 97
    let mut chat_dialog = ChatDialog::new(main_dialog_x, screen_h, 1); // 1024分辨率
    let mut chat_open = true;
    
    // 添加测试消息（需要超过4条才能测试滚动条）
    chat_dialog.add_message(
        "Welcome to the Legend of Mir 2 Server.",
        egui_macroquad::egui::Color32::from_rgb(255, 255, 0), // 黄色
    );
    chat_dialog.add_message(
        "[Mode: Peaceful]",
        egui_macroquad::egui::Color32::from_rgb(0, 255, 0), // 绿色
    );
    chat_dialog.add_message(
        "[Pet: Attack and Move]",
        egui_macroquad::egui::Color32::from_rgb(100, 200, 255), // 浅蓝色
    );
    chat_dialog.add_message(
        "System: ChatDialog 测试中...",
        egui_macroquad::egui::Color32::WHITE,
    );
    
    // 添加更多消息以测试滚动功能
    for i in 1..=15 {
        let msg = format!("测试消息 #{}: 这是一条用于测试滚动条功能的消息", i);
        let color = match i % 3 {
            0 => egui_macroquad::egui::Color32::from_rgb(255, 200, 100), // 橙色
            1 => egui_macroquad::egui::Color32::from_rgb(150, 150, 255), // 紫色
            _ => egui_macroquad::egui::Color32::WHITE,
        };
        chat_dialog.add_message(&msg, color);
    }
    
    println!("✅ ChatDialog 已创建");
    println!("📍 ChatControlBar 位置: x={}, y={}", main_dialog_x + 230.0, screen_h - 112.0);
    println!("📍 ChatDialog 位置: x={}, y={}", main_dialog_x + 230.0, screen_h - 97.0);
    println!("💡 提示:");
    println!("   - 按 Enter 键显示聊天输入框");
    println!("   - 在输入框中输入文字后按 Enter 发送");
    println!("   - 按 Tab 键切换聊天窗口大小（小→中→大）");
    println!("   - 按 ESC 退出");
    println!();
    println!("🔍 请检查:");
    println!("   - ChatControlBar 是否显示在 ChatDialog 上方？");
    println!("   - 聊天频道按钮是否可见？（All/Shout/Whisper等）");
    println!("   - Size 和 Settings 按钮是否在右侧？");
    println!("   - ChatDialog 背景纹理是否显示？");
    println!("   - 右侧滚动条是否可见？");

    loop {
        clear_background(Color::from_rgba(60, 80, 100, 255));

        // 处理键盘输入 - Tab 切换窗口大小
        if is_key_pressed(KeyCode::Tab) {
            chat_dialog.change_size(screen_h);
            let size_info = match chat_dialog.get_window_size() {
                0 => "小 (4行)",
                1 => "中 (7行)",
                2 => "大 (11行)",
                _ => "未知",
            };
            println!("🔄 切换聊天窗口大小: {}", size_info);
        }

        // 绘制 UI
        egui_macroquad::ui(|egui_ctx| {
            let mut open = true;
            
            // 显示 ChatControlBar 并获取按钮点击状态
            let (size_clicked, _settings_clicked) = chat_control_bar.show(egui_ctx, &mut open);
            
            // 如果 Size 按钮被点击，改变 ChatDialog 大小
            if size_clicked {
                chat_dialog.change_size(screen_h);
                
                // 同步更新 ChatControlBar 位置（保持在 ChatDialog 上方）
                let chat_pos = chat_dialog.get_position();
                let control_bar_y = chat_pos.y - 15.0; // ChatControlBar 高度约15像素
                chat_control_bar.set_position(egui_macroquad::egui::pos2(chat_pos.x, control_bar_y));
                
                let size_info = match chat_dialog.get_window_size() {
                    0 => "小 (4行)",
                    1 => "中 (7行)",
                    2 => "大 (11行)",
                    _ => "未知",
                };
                println!("🔄 切换聊天窗口大小: {}", size_info);
            }
            
            chat_dialog.show(egui_ctx, &mut open);
        });
        
        egui_macroquad::draw();

        // 绘制背景提示
        let text = "ChatDialog + ChatControlBar 测试 - 点击输入框输入聊天";
        let font_size = 32.0;
        let text_size = measure_text(&text, None, font_size as u16, 1.0);
        draw_text(
            text,
            screen_width() / 2.0 - text_size.width / 2.0,
            100.0,
            font_size,
            WHITE,
        );

        // 绘制提示信息
        let info_text = "检查聊天控制栏是否显示在聊天框上方";
        let info_size = measure_text(&info_text, None, 20, 1.0);
        draw_text(
            info_text,
            screen_width() / 2.0 - info_size.width / 2.0,
            150.0,
            20.0,
            YELLOW,
        );

        // egui UI
        egui_macroquad::ui(|ctx| {
            // 绘制 ChatControlBar（在 ChatDialog 上方）
            chat_control_bar.show(ctx, &mut control_bar_open);
            
            // 绘制 ChatDialog
            chat_dialog.show(ctx, &mut chat_open);
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
