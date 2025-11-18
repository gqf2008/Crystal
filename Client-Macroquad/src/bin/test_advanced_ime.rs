// ============================================================================
// 高级IME测试 - 验证macroquad的Unicode字符支持
// ============================================================================

use macroquad::prelude::*;
use egui_macroquad::egui;

fn window_conf() -> Conf {
    Conf {
        window_title: "macroquad 中文输入验证".to_owned(),
        window_width: 800,
        window_height: 600,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 macroquad 中文输入验证程序");
    println!("✨ 这个程序将验证 macroquad 的 get_char_pressed() 是否支持中文");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 加载 macroquad 中文字体
    let mq_font = match load_ttf_font("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf").await {
        Ok(f) => {
            println!("✅ macroquad 中文字体加载成功");
            Some(f)
        },
        Err(_) => {
            println!("⚠️  无法加载 macroquad 中文字体，使用默认字体");
            None
        }
    };

    // 配置 egui 中文字体
    egui_macroquad::cfg(|ctx| {
        let mut fonts = egui_macroquad::egui::FontDefinitions::default();
        
        // 尝试加载中文字体
        let font_data = std::fs::read("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf")
            .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\msyh.ttc"))
            .unwrap_or_else(|_| {
                println!("⚠️  使用默认字体，中文显示可能有问题");
                vec![]
            });

        if !font_data.is_empty() {
            fonts.font_data.insert(
                "chinese".to_owned(),
                std::sync::Arc::new(egui_macroquad::egui::FontData::from_owned(font_data)),
            );

            fonts.families.get_mut(&egui_macroquad::egui::FontFamily::Proportional)
                .unwrap().insert(0, "chinese".to_owned());
            
            println!("✅ 已加载中文字体");
        }

        ctx.set_fonts(fonts);
    });

    let mut macroquad_text = String::new();
    let mut egui_text = String::new();
    let mut char_log: Vec<String> = Vec::new();

    loop {
        // 1. 使用 macroquad 原生方法获取字符
        if let Some(ch) = get_char_pressed() {
            let info = format!("字符: '{}' (U+{:04X}) - {}", 
                ch, ch as u32, 
                if ch.is_ascii() { "ASCII" } else { "Unicode" }
            );
            println!("📝 {}", info);
            char_log.push(info);
            
            // 处理退格
            if ch == '\u{8}' { // 退格键
                macroquad_text.pop();
            } else if !ch.is_control() {
                macroquad_text.push(ch);
            }
            
            // 限制日志长度
            if char_log.len() > 15 {
                char_log.remove(0);
            }
        }

        // 2. 清除背景
        clear_background(Color::from_rgba(25, 25, 30, 255));

        // 3. 绘制macroquad原生输入测试区域
        let title_y = 50.0;
        draw_text_ex("🔤 macroquad 原生字符输入测试", 20.0, title_y, TextParams {
            font: mq_font.as_ref(),
            font_size: 24,
            color: WHITE,
            ..Default::default()
        });
        
        // 输入框背景
        let input_box_y = 80.0;
        let input_box_h = 50.0;
        draw_rectangle(20.0, input_box_y, screen_width() - 40.0, input_box_h, 
                      Color::from_rgba(40, 40, 50, 255));
        draw_rectangle_lines(20.0, input_box_y, screen_width() - 40.0, input_box_h, 
                           2.0, SKYBLUE);
        
        // 显示输入的文本
        let display_text = if macroquad_text.is_empty() {
            "请切换到中文输入法，输入中文测试...".to_string()
        } else {
            macroquad_text.clone()
        };
        draw_text_ex(&display_text, 30.0, input_box_y + 30.0, TextParams {
            font: mq_font.as_ref(),
            font_size: 20,
            color: WHITE,
            ..Default::default()
        });

        // 字符统计
        let stats = format!("字符数: {} | 字节数: {} | 最后字符: {}", 
            macroquad_text.chars().count(),
            macroquad_text.len(),
            macroquad_text.chars().last().map_or("无".to_string(), |c| format!("'{}'", c))
        );
        draw_text_ex(&stats, 30.0, input_box_y + input_box_h + 20.0, TextParams {
            font: mq_font.as_ref(),
            font_size: 16,
            color: GRAY,
            ..Default::default()
        });

        // 4. egui 测试区域
        egui_macroquad::ui(|ctx| {
            egui::Window::new("🎌 egui TextEdit 中文输入测试")
                .default_pos([20.0, 200.0])
                .default_size([screen_width() - 40.0, 200.0])
                .show(ctx, |ui| {
                    ui.label("egui TextEdit 应该天然支持中文输入:");
                    ui.text_edit_multiline(&mut egui_text);
                    
                    ui.separator();
                    ui.label(format!("egui输入内容: \"{}\"", egui_text));
                    ui.label(format!("字符数: {}", egui_text.chars().count()));
                });

            // 字符事件日志窗口
            egui::Window::new("📊 字符事件日志")
                .default_pos([20.0, 450.0])
                .default_size([screen_width() - 40.0, 120.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for log_entry in char_log.iter().rev() {
                                ui.label(log_entry);
                            }
                        });
                });
        });

        egui_macroquad::draw();

        // 5. 底部说明
        let bottom_y = screen_height() - 80.0;
        draw_text_ex("💡 测试说明:", 20.0, bottom_y, TextParams {
            font: mq_font.as_ref(),
            font_size: 18,
            color: YELLOW,
            ..Default::default()
        });
        draw_text_ex("• macroquad区域: 使用get_char_pressed()获取字符", 20.0, bottom_y + 25.0, TextParams {
            font: mq_font.as_ref(),
            font_size: 14,
            color: LIGHTGRAY,
            ..Default::default()
        });
        draw_text_ex("• egui区域: 使用TextEdit组件输入", 20.0, bottom_y + 45.0, TextParams {
            font: mq_font.as_ref(),
            font_size: 14,
            color: LIGHTGRAY,
            ..Default::default()
        });
        draw_text("• 按ESC退出，清空输入按Ctrl+A选中后删除", 20.0, bottom_y + 65.0, 14.0, LIGHTGRAY);

        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            println!("\n🎯 测试结果总结:");
            println!("━━━━━━━━━━━━━━━━━━");
            println!("macroquad原生输入: \"{}\"", macroquad_text);
            println!("egui TextEdit输入: \"{}\"", egui_text);
            println!("字符事件总数: {}", char_log.len());
            
            if macroquad_text.chars().any(|c| !c.is_ascii()) {
                println!("✅ macroquad 支持Unicode/中文输入!");
            } else {
                println!("⚠️  只检测到ASCII字符，可能需要进一步测试");
            }
            
            if egui_text.chars().any(|c| !c.is_ascii()) {
                println!("✅ egui TextEdit 支持中文输入!");
            }
            
            println!("👋 测试完成");
            break;
        }

        next_frame().await;
    }
}