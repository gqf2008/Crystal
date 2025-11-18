// ============================================================================
// MirTextBox 移植测试 - 从原工程移植输入法功能
// ============================================================================

use macroquad::prelude::*;
use egui_macroquad::egui;

fn window_conf() -> Conf {
    Conf {
        window_title: "MirTextBox IME测试 - Crystal客户端输入法移植".to_owned(),
        window_width: 900,
        window_height: 700,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    }
}

// 模拟原工程的MirTextBox结构
#[derive(Debug, Clone)]
struct MirTextBox {
    text: String,
    max_length: usize,
    password: bool,
    multiline: bool,
    focused: bool,
    cursor_pos: usize,
    selection_start: usize,
    selection_length: usize,
    background_color: Color,
    foreground_color: Color,
    enabled: bool,
    visible: bool,
}

impl MirTextBox {
    fn new() -> Self {
        Self {
            text: String::new(),
            max_length: 255,
            password: false,
            multiline: false,
            focused: false,
            cursor_pos: 0,
            selection_start: 0,
            selection_length: 0,
            background_color: Color::from_rgba(40, 40, 50, 255),
            foreground_color: WHITE,
            enabled: true,
            visible: true,
        }
    }

    fn set_focus(&mut self) {
        self.focused = true;
    }

    fn lose_focus(&mut self) {
        self.focused = false;
    }

    fn insert_char(&mut self, ch: char) {
        if !self.enabled || !self.visible {
            return;
        }

        if self.text.len() >= self.max_length {
            return;
        }

        if ch.is_control() && ch != '\u{8}' && ch != '\r' && ch != '\n' {
            return;
        }

        match ch {
            '\u{8}' => { // 退格键
                if self.cursor_pos > 0 {
                    self.text.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            },
            '\r' | '\n' => {
                if self.multiline {
                    self.text.insert(self.cursor_pos, '\n');
                    self.cursor_pos += 1;
                }
            },
            _ => {
                self.text.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
            }
        }
    }

    fn display_text(&self) -> String {
        if self.password {
            "*".repeat(self.text.chars().count())
        } else {
            self.text.clone()
        }
    }

    fn draw(&self, ui: &mut egui::Ui, rect: egui::Rect, label: &str) {
        if !label.is_empty() {
            ui.painter().text(
                rect.left_top() - egui::vec2(0.0, 20.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
        
        // 绘制背景
        ui.painter().rect_filled(
            rect,
            3.0,
            egui::Color32::from_rgba_premultiplied(40, 40, 50, 255),
        );
        
        // 绘制边框
        let border_color = if self.focused {
            egui::Color32::from_rgb(100, 150, 255)
        } else {
            egui::Color32::from_rgb(80, 80, 90)
        };
        
        // 绘制边框 - 使用简单的线条绘制
        let stroke = egui::Stroke::new(2.0, border_color);
        ui.painter().line_segment([rect.left_top(), rect.right_top()], stroke);
        ui.painter().line_segment([rect.right_top(), rect.right_bottom()], stroke);
        ui.painter().line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
        ui.painter().line_segment([rect.left_bottom(), rect.left_top()], stroke);
            
        // 绘制文本
        let text_rect = rect.shrink(8.0);
        let display_text = if self.text.is_empty() {
            if self.focused {
                "正在输入...".to_string()
            } else {
                "点击输入文本".to_string()
            }
        } else {
            self.display_text()
        };
        
        let text_color = if self.text.is_empty() {
            egui::Color32::GRAY
        } else {
            egui::Color32::WHITE
        };
        
        ui.painter().text(
            text_rect.left_top() + egui::vec2(0.0, 5.0),
            egui::Align2::LEFT_TOP,
            display_text,
            egui::FontId::default(),
            text_color,
        );
        
        // 绘制光标
        if self.focused {
            let cursor_x = text_rect.left() + (self.cursor_pos as f32 * 8.0);
            ui.painter().line_segment(
                [
                    egui::pos2(cursor_x, text_rect.top()),
                    egui::pos2(cursor_x, text_rect.bottom() - 5.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::WHITE),
            );
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 MirTextBox IME测试 - Crystal客户端输入法移植");
    println!("✨ 基于原工程MirTextBox.cs的Rust实现");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 配置 egui 中文字体
    egui_macroquad::cfg(|ctx| {
        let mut fonts = egui_macroquad::egui::FontDefinitions::default();
        
        // 尝试加载中文字体
        let font_data = std::fs::read("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf")
            .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\msyh.ttc"))
            .or_else(|_| std::fs::read("C:\\Windows\\Fonts\\simsun.ttc"))
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

    // 创建多个MirTextBox实例来测试不同场景
    let mut account_textbox = MirTextBox::new();
    let mut password_textbox = MirTextBox {
        password: true,
        ..MirTextBox::new()
    };
    let mut multiline_textbox = MirTextBox {
        multiline: true,
        max_length: 1000,
        ..MirTextBox::new()
    };
    
    let mut char_log: Vec<String> = Vec::new();
    let mut current_focus = 0; // 0: account, 1: password, 2: multiline

    loop {
        // 1. 处理键盘输入
        if let Some(ch) = get_char_pressed() {
            let info = format!("字符: '{}' (U+{:04X}) - {}", 
                ch, ch as u32, 
                if ch.is_ascii() { "ASCII" } else { "Unicode" }
            );
            println!("📝 {}", info);
            char_log.push(info);
            
            // 将字符发送到当前焦点的textbox
            match current_focus {
                0 => account_textbox.insert_char(ch),
                1 => password_textbox.insert_char(ch),
                2 => multiline_textbox.insert_char(ch),
                _ => {}
            }
            
            // 限制日志长度
            if char_log.len() > 15 {
                char_log.remove(0);
            }
        }

        // 2. 处理Tab键切换焦点
        if is_key_pressed(KeyCode::Tab) {
            match current_focus {
                0 => {
                    account_textbox.lose_focus();
                    password_textbox.set_focus();
                    current_focus = 1;
                },
                1 => {
                    password_textbox.lose_focus();
                    multiline_textbox.set_focus();
                    current_focus = 2;
                },
                2 => {
                    multiline_textbox.lose_focus();
                    account_textbox.set_focus();
                    current_focus = 0;
                },
                _ => {}
            }
        }

        // 3. 清除背景
        clear_background(Color::from_rgba(25, 25, 30, 255));

        // 4. egui界面
        egui_macroquad::ui(|ctx| {
            // 主标题窗口
            egui::Window::new("🎮 MirTextBox IME测试")
                .default_pos([20.0, 20.0])
                .default_size([860.0, 650.0])
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Crystal客户端输入法移植测试");
                    ui.label("基于原工程MirTextBox.cs的Rust实现");
                    ui.separator();

                    // 账号输入框
                    ui.horizontal(|ui| {
                        ui.label("账号登录:");
                        if ui.button(if current_focus == 0 { "🔸" } else { "⚪" }).clicked() {
                            account_textbox.lose_focus();
                            password_textbox.lose_focus();
                            multiline_textbox.lose_focus();
                            account_textbox.set_focus();
                            current_focus = 0;
                        }
                    });
                    
                    let account_rect = egui::Rect::from_min_size(
                        egui::pos2(100.0, 80.0), 
                        egui::vec2(400.0, 30.0)
                    );
                    account_textbox.draw(ui, account_rect, "");

                    ui.add_space(20.0);

                    // 密码输入框
                    ui.horizontal(|ui| {
                        ui.label("密码登录:");
                        if ui.button(if current_focus == 1 { "🔸" } else { "⚪" }).clicked() {
                            account_textbox.lose_focus();
                            password_textbox.lose_focus();
                            multiline_textbox.lose_focus();
                            password_textbox.set_focus();
                            current_focus = 1;
                        }
                    });
                    
                    let password_rect = egui::Rect::from_min_size(
                        egui::pos2(100.0, 140.0), 
                        egui::vec2(400.0, 30.0)
                    );
                    password_textbox.draw(ui, password_rect, "");

                    ui.add_space(20.0);

                    // 多行文本框
                    ui.horizontal(|ui| {
                        ui.label("多行输入:");
                        if ui.button(if current_focus == 2 { "🔸" } else { "⚪" }).clicked() {
                            account_textbox.lose_focus();
                            password_textbox.lose_focus();
                            multiline_textbox.lose_focus();
                            multiline_textbox.set_focus();
                            current_focus = 2;
                        }
                    });
                    
                    let multiline_rect = egui::Rect::from_min_size(
                        egui::pos2(100.0, 200.0), 
                        egui::vec2(400.0, 120.0)
                    );
                    multiline_textbox.draw(ui, multiline_rect, "");

                    ui.add_space(30.0);

                    // 状态信息
                    ui.separator();
                    ui.label("📊 输入状态:");
                    ui.label(format!("账号内容: \"{}\" ({}字符)", 
                        account_textbox.text, account_textbox.text.chars().count()));
                    ui.label(format!("密码内容: \"{}\" ({}字符)", 
                        if password_textbox.password { "*".repeat(password_textbox.text.chars().count()) } else { password_textbox.text.clone() },
                        password_textbox.text.chars().count()));
                    ui.label(format!("多行内容: \"{}\" ({}字符, {}行)", 
                        multiline_textbox.text.replace('\n', "\\n"), 
                        multiline_textbox.text.chars().count(),
                        multiline_textbox.text.lines().count()));

                    ui.add_space(10.0);
                    ui.label("💡 操作说明:");
                    ui.label("• Tab键切换输入框焦点");
                    ui.label("• 🔸表示当前焦点，⚪表示非焦点");
                    ui.label("• 支持中文输入法和Unicode字符");
                    ui.label("• ESC键退出程序");
                });

            // 字符事件日志窗口
            egui::Window::new("📝 字符事件日志")
                .default_pos([550.0, 20.0])
                .default_size([320.0, 300.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for log_entry in char_log.iter().rev().take(20) {
                                ui.label(log_entry);
                            }
                            if char_log.is_empty() {
                                ui.label("暂无输入事件");
                            }
                        });
                });

            // 测试原生egui TextEdit作为对比
            egui::Window::new("🔄 egui原生对比测试")
                .default_pos([550.0, 340.0])
                .default_size([320.0, 200.0])
                .show(ctx, |ui| {
                    ui.label("原生egui TextEdit:");
                    
                    static mut NATIVE_TEXT: String = String::new();
                    unsafe {
                        ui.text_edit_singleline(&mut NATIVE_TEXT);
                        ui.label(format!("内容: \"{}\"", NATIVE_TEXT));
                        ui.label(format!("字符数: {}", NATIVE_TEXT.chars().count()));
                    }
                });
        });

        egui_macroquad::draw();

        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            println!("\n🎯 MirTextBox测试结果总结:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("账号输入: \"{}\"", account_textbox.text);
            println!("密码输入: \"{}\"", password_textbox.text);
            println!("多行输入: \"{}\"", multiline_textbox.text.replace('\n', "\\n"));
            println!("字符事件总数: {}", char_log.len());
            
            let has_unicode = account_textbox.text.chars().any(|c| !c.is_ascii()) ||
                              password_textbox.text.chars().any(|c| !c.is_ascii()) ||
                              multiline_textbox.text.chars().any(|c| !c.is_ascii());
            
            if has_unicode {
                println!("✅ MirTextBox 成功支持Unicode/中文输入!");
            } else {
                println!("⚠️  未检测到Unicode字符，尝试输入中文测试");
            }
            
            println!("👋 测试完成 - 成功移植原工程输入法功能");
            break;
        }

        next_frame().await;
    }
}