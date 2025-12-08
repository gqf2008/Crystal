//! 详细 IME 测试 - 带调试输出和中文字体
//! 
//! 运行: cargo run --example test_ime_debug

use macroquad::prelude::*;
use miniquad::window::show_keyboard;

fn conf() -> Conf {
    Conf {
        window_title: "IME 详细调试测试".to_string(),
        window_width: 800,
        window_height: 600,
        ..Default::default()
    }
}

/// 输入框状态
struct InputBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    focused: bool,
    cursor_visible: bool,
    cursor_blink_time: f64,
}

impl InputBox {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            focused: false,
            cursor_visible: true,
            cursor_blink_time: 0.0,
        }
    }
    
    /// 检查点击是否在输入框内
    fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width &&
        py >= self.y && py <= self.y + self.height
    }
    
    /// 处理点击事件
    fn handle_click(&mut self, px: f32, py: f32) -> bool {
        let was_focused = self.focused;
        let clicked_inside = self.contains(px, py);
        
        if clicked_inside {
            if !self.focused {
                self.focused = true;
                self.cursor_visible = true;
                self.cursor_blink_time = get_time();
                // 启用输入法
                show_keyboard(true);
                eprintln!("[InputBox] 获得焦点，启用输入法");
            }
        } else {
            if self.focused {
                self.focused = false;
                // 禁用输入法 - 这是关键！
                show_keyboard(false);
                eprintln!("[InputBox] 失去焦点，禁用输入法");
            }
        }
        
        was_focused != self.focused
    }
    
    /// 更新光标闪烁
    fn update_cursor(&mut self) {
        if self.focused {
            let now = get_time();
            // 每 0.5 秒切换一次光标可见性
            if now - self.cursor_blink_time >= 0.5 {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_blink_time = now;
            }
        }
    }
    
    /// 当有输入时重置光标为可见
    fn on_input(&mut self) {
        self.cursor_visible = true;
        self.cursor_blink_time = get_time();
    }
    
    /// 绘制输入框
    fn draw(&self, text: &str, font: &Font) {
        // 背景
        let bg_color = if self.focused {
            Color::from_rgba(60, 60, 80, 255)
        } else {
            Color::from_rgba(50, 50, 60, 255)
        };
        draw_rectangle(self.x, self.y, self.width, self.height, bg_color);
        
        // 边框
        let border_color = if self.focused {
            Color::from_rgba(100, 150, 255, 255)
        } else {
            WHITE
        };
        draw_rectangle_lines(self.x, self.y, self.width, self.height, 2.0, border_color);
        
        // 文本位置
        let text_x = self.x + 10.0;
        let text_y = self.y + 35.0;
        
        // 绘制文本或占位符
        if text.is_empty() && !self.focused {
            draw_text_ex(
                "点击此处输入中文...",
                text_x, text_y,
                TextParams {
                    font: Some(font),
                    font_size: 24,
                    color: GRAY,
                    ..Default::default()
                },
            );
        } else {
            draw_text_ex(
                text,
                text_x, text_y,
                TextParams {
                    font: Some(font),
                    font_size: 24,
                    color: WHITE,
                    ..Default::default()
                },
            );
            
            // 绘制闪烁光标
            if self.focused && self.cursor_visible {
                // 计算光标位置（在文本末尾）
                let text_dims = measure_text(text, Some(font), 24, 1.0);
                let cursor_x = text_x + text_dims.width;
                let cursor_y1 = self.y + 10.0;
                let cursor_y2 = self.y + self.height - 10.0;
                
                draw_line(cursor_x, cursor_y1, cursor_x, cursor_y2, 2.0, WHITE);
            }
        }
        
        // 如果没有文本但有焦点，也显示光标
        if text.is_empty() && self.focused && self.cursor_visible {
            let cursor_x = text_x;
            let cursor_y1 = self.y + 10.0;
            let cursor_y2 = self.y + self.height - 10.0;
            draw_line(cursor_x, cursor_y1, cursor_x, cursor_y2, 2.0, WHITE);
        }
    }
}

#[macroquad::main(conf)]
async fn main() {
    // 加载中文字体
    let font = match load_ttf_font("C:/Windows/Fonts/msyh.ttc").await {
        Ok(f) => {
            println!("✅ 加载微软雅黑字体成功");
            f
        }
        Err(_) => {
            println!("⚠️ 微软雅黑不存在，尝试宋体...");
            load_ttf_font("C:/Windows/Fonts/simsun.ttc").await
                .expect("无法加载任何中文字体")
        }
    };
    
    let mut input_text = String::new();
    let mut char_history: Vec<(char, u32, f64)> = Vec::new();
    
    // 创建输入框
    let mut input_box = InputBox::new(20.0, 100.0, 760.0, 50.0);
    
    println!("=== IME 详细调试测试 ===");
    println!("点击输入框启用输入法");
    println!("请切换不同输入法测试（如手心输入法）");
    println!();
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        let now = get_time();
        
        // 处理鼠标点击
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            input_box.handle_click(mx, my);
        }
        
        // 更新光标闪烁
        input_box.update_cursor();
        
        // 只有在输入框有焦点时才处理输入
        if input_box.focused {
            // 获取输入的字符
            while let Some(chr) = get_char_pressed() {
                let code = chr as u32;
                
                // 过滤控制字符用于显示
                let display = if code >= 0x20 && code != 0x7F {
                    format!("'{}'", chr)
                } else {
                    format!("<0x{:02X}>", code)
                };
                
                println!("[{:.2}s] 字符: {} U+{:04X}", now, display, code);
                
                char_history.push((chr, code, now));
                if char_history.len() > 30 {
                    char_history.remove(0);
                }
                
                // 只添加可打印字符到文本
                if code >= 0x20 && code != 0x7F {
                    input_text.push(chr);
                    input_box.on_input(); // 重置光标
                }
            }
            
            // 退格键处理
            if is_key_pressed(KeyCode::Backspace) && !input_text.is_empty() {
                input_text.pop();
                input_box.on_input();
                println!("[{:.2}s] 退格", now);
            }
            
            // 回车键清空
            if is_key_pressed(KeyCode::Enter) {
                println!("[{:.2}s] 清空输入: {}", now, input_text);
                input_text.clear();
                input_box.on_input();
            }
        } else {
            // 没有焦点时清空字符队列，避免积累
            while get_char_pressed().is_some() {}
        }
        
        // ESC退出
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 绘制标题 (使用中文字体)
        draw_text_ex(
            "IME 详细调试测试 - 点击输入框启用输入法",
            20.0, 40.0,
            TextParams {
                font: Some(&font),
                font_size: 28,
                color: WHITE,
                ..Default::default()
            },
        );
        
        draw_text_ex(
            &format!("输入框焦点: {} | 查看终端输出了解详细信息", 
                if input_box.focused { "✓ 已获得" } else { "✗ 未获得" }),
            20.0, 70.0,
            TextParams {
                font: Some(&font),
                font_size: 18,
                color: if input_box.focused { GREEN } else { YELLOW },
                ..Default::default()
            },
        );
        
        // 绘制输入框
        input_box.draw(&input_text, &font);
        
        // 绘制字符历史
        draw_text_ex(
            "字符历史 (最近30个):",
            20.0, 180.0,
            TextParams {
                font: Some(&font),
                font_size: 16,
                color: YELLOW,
                ..Default::default()
            },
        );
        
        let start_time = char_history.first().map(|x| x.2).unwrap_or(0.0);
        
        for (i, (chr, code, time)) in char_history.iter().enumerate() {
            let display_char = if *code >= 0x20 && *code != 0x7F {
                format!("'{}'", chr)
            } else {
                format!("<{:02X}>", code)
            };
            
            let relative_time = time - start_time;
            let text = format!("{:5.2}s {} {:04X}", relative_time, display_char, code);
            
            let col = i % 5;
            let row = i / 5;
            
            draw_text_ex(
                &text,
                20.0 + col as f32 * 150.0,
                200.0 + row as f32 * 22.0,
                TextParams {
                    font: Some(&font),
                    font_size: 14,
                    color: Color::from_rgba(180, 180, 200, 255),
                    ..Default::default()
                },
            );
        }
        
        // 统计信息
        let chinese_count = char_history.iter().filter(|(_, code, _)| *code > 0x4E00).count();
        let ascii_count = char_history.iter().filter(|(_, code, _)| *code < 0x80 && *code >= 0x20).count();
        
        draw_text_ex(
            &format!("统计: 中文字符 {} | ASCII {} | 总计 {}", chinese_count, ascii_count, char_history.len()),
            20.0, 350.0,
            TextParams {
                font: Some(&font),
                font_size: 16,
                color: Color::from_rgba(100, 200, 100, 255),
                ..Default::default()
            },
        );
        
        // 说明
        let instructions = [
            "操作说明:",
            "1. 点击输入框获得焦点并启用输入法",
            "2. 点击输入框外部失去焦点",
            "3. 输入框获得焦点后会显示闪烁光标",
            "",
            "按 ESC 退出 | Enter 清空 | Backspace 删除",
        ];
        
        for (i, line) in instructions.iter().enumerate() {
            draw_text_ex(
                line,
                20.0, 400.0 + i as f32 * 24.0,
                TextParams {
                    font: Some(&font),
                    font_size: 16,
                    color: Color::from_rgba(150, 150, 200, 255),
                    ..Default::default()
                },
            );
        }
        
        // 显示输入长度
        draw_text_ex(
            &format!("当前输入: {} 字符 | 字节: {}", 
                input_text.chars().count(),
                input_text.len()),
            20.0, 560.0,
            TextParams {
                font: Some(&font),
                font_size: 14,
                color: GRAY,
                ..Default::default()
            },
        );
        
        next_frame().await;
    }
}
