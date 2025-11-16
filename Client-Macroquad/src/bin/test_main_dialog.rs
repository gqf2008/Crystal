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
    
    // 创建 MainDialog（内部自动创建所有子对话框）
    let mut main_dialog = MainDialog::new();
    
    println!("✅ MainDialog 及所有子对话框已创建");
    println!("💡 提示: ");
    println!("   - 点击 Size 按钮或按 Tab 键切换聊天窗口大小");
    println!("   - 按 ESC 退出");

    loop {
        clear_background(Color::from_rgba(60, 80, 100, 255));

        // 绘制背景提示
        let text = "游戏主场景 - 点击 Size 按钮或按 Tab 切换聊天窗口大小";
        let font_size = 32.0;
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
            // 绘制主对话框和所有子对话框
            main_dialog.show(ctx);
            main_dialog.show_dialogs(ctx);
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
