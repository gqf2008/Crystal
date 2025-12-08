// ============================================================================
// LoginScene - 登录界面 (纯 Native 版本 - macroquad 原生渲染)
// ============================================================================
// 对应原版: C# Client/MirScenes/LoginScene.cs
//
// 【渲染架构】纯 macroquad 原生渲染
// - 所有 UI 元素使用 macroquad 原生绘制
// - 无 egui 依赖
//
// ============================================================================

use crate::game::GameResult;
use crate::resources::LibraryName;
use crate::scenes::{Scene, SceneTransition};
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use macroquad::prelude::*;

/// 登录场景 - 纯 Native 版本
pub struct LoginScene {
    // 登录信息
    account: String,
    password: String,
    password_focus: bool,
    
    // 背景动画
    background_frame: usize,
    animation_playing: bool,
    frame_timer: f32,
    frame_delay: f32,
    
    // UI 状态
    cursor_visible: bool,
    cursor_timer: f32,
    input_focus: InputFocus,
    
    // 消息框
    show_message: bool,
    message_text: String,
}

#[derive(PartialEq, Clone, Copy)]
enum InputFocus {
    Account,
    Password,
    None,
}

impl LoginScene {
    pub fn new() -> Self {
        Self {
            account: String::new(),
            password: String::new(),
            password_focus: false,
            
            background_frame: 0,
            animation_playing: false,
            frame_timer: 0.0,
            frame_delay: 0.1,
            
            cursor_visible: true,
            cursor_timer: 0.0,
            input_focus: InputFocus::Account,
            
            show_message: false,
            message_text: String::new(),
        }
    }

    /// 绘制登录对话框背景
    fn draw_login_background(&self) -> (f32, f32, f32, f32) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // 获取背景纹理并计算居中位置
        let (dialog_w, dialog_h, dialog_x, dialog_y) =
            if let Some(info) = LibraryName::Prguse.get_texture(1084) {
                if let Some(ref bg_tex) = info.image {
                    let w = bg_tex.width();
                    let h = bg_tex.height();
                    let x = (screen_w - w) / 2.0;
                    let y = (screen_h - h) / 2.0;

                    draw_texture(bg_tex, x, y, WHITE);
                    (w, h, x, y)
                } else {
                    let w = 328.0;
                    let h = 220.0;
                    (w, h, (screen_w - w) / 2.0, (screen_h - h) / 2.0)
                }
            } else {
                let w = 328.0;
                let h = 220.0;
                (w, h, (screen_w - w) / 2.0, (screen_h - h) / 2.0)
            };

        // 绘制标题 (Title 30)
        if let Some(info) = LibraryName::Title.get_texture(30) {
            if let Some(ref tex) = info.image {
                let w = tex.width();
                let x = dialog_x + (dialog_w - w) / 2.0;
                let y = dialog_y + 12.0;
                draw_texture(tex, x, y, WHITE);
            }
        }

        // 绘制账号标签 (Title 31)
        if let Some(info) = LibraryName::Title.get_texture(31) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + 52.0, dialog_y + 83.0, WHITE);
            }
        }

        // 绘制密码标签 (Title 32)
        if let Some(info) = LibraryName::Title.get_texture(32) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + 43.0, dialog_y + 105.0, WHITE);
            }
        }

        (dialog_w, dialog_h, dialog_x, dialog_y)
    }

    /// 绘制输入框
    fn draw_input_box(&self, x: f32, y: f32, width: f32, height: f32, text: &str, is_password: bool, is_focused: bool) {
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
        draw_text_cn(&display_text, x + 5.0, text_y, 14.0, WHITE);
        
        // 绘制光标
        if is_focused && self.cursor_visible {
            let text_width = measure_text_cn(&display_text, 14.0).width;
            let cursor_x = x + 5.0 + text_width;
            draw_line(cursor_x, y + 3.0, cursor_x, y + height - 3.0, 1.0, WHITE);
        }
    }

    /// 绘制按钮
    fn draw_button(&self, x: f32, y: f32, normal_idx: usize, hover_idx: usize, pressed_idx: usize) -> bool {
        let (mx, my) = mouse_position();
        
        let btn_size = if let Some(info) = LibraryName::Prguse.get_texture(normal_idx) {
            (info.width as f32, info.height as f32)
        } else {
            (80.0, 25.0)
        };
        
        let is_hovered = mx >= x && mx <= x + btn_size.0 && my >= y && my <= y + btn_size.1;
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);
        
        let texture_idx = if is_pressed {
            pressed_idx
        } else if is_hovered {
            hover_idx
        } else {
            normal_idx
        };
        
        if let Some(info) = LibraryName::Prguse.get_texture(texture_idx) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, x, y, WHITE);
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
        
        is_hovered && is_mouse_button_pressed(MouseButton::Left)
    }

    /// 绘制消息框
    fn draw_message_box(&self) {
        let screen_w = screen_width();
        let screen_h = screen_height();
        
        let box_w = 300.0;
        let box_h = 150.0;
        let box_x = (screen_w - box_w) / 2.0;
        let box_y = (screen_h - box_h) / 2.0;
        
        // 背景
        draw_rectangle(box_x, box_y, box_w, box_h, Color::from_rgba(40, 40, 50, 240));
        draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, Color::from_rgba(100, 100, 120, 255));
        
        // 标题
        draw_text_cn("提示", box_x + box_w / 2.0 - 15.0, box_y + 30.0, 18.0, WHITE);
        
        // 消息文本
        let text_width = measure_text_cn(&self.message_text, 14.0).width;
        draw_text_cn(&self.message_text, box_x + (box_w - text_width) / 2.0, box_y + 70.0, 14.0, WHITE);
    }

    /// 登录按钮点击
    fn on_login_clicked(&mut self) {
        if self.account.is_empty() || self.password.is_empty() {
            self.message_text = "账号或密码不能为空!".to_string();
            self.show_message = true;
            return;
        }

        println!("🔐 Login: account={}", self.account);

        // 保存配置
        self.save_config();

        // 开始播放登录成功动画
        self.animation_playing = true;
        self.background_frame = 0;
    }

    /// 保存配置到本地文件
    fn save_config(&self) {
        use std::fs;
        use std::io::Write;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let config = format!(
            "[Login]\nAccount={}\nSavePassword=false\nLastLogin={}\nVersion={}\n",
            self.account,
            timestamp,
            env!("CARGO_PKG_VERSION")
        );

        if let Ok(mut file) = fs::File::create("config.ini") {
            let _ = file.write_all(config.as_bytes());
            println!("✅ 配置已保存");
        }
    }

    /// 加载配置
    fn load_config(&mut self) {
        use std::fs;

        if let Ok(content) = fs::read_to_string("config.ini") {
            for line in content.lines() {
                if let Some(account) = line.strip_prefix("Account=") {
                    self.account = account.to_string();
                    println!("✅ 已加载账号: {}", account);
                }
            }
        }
    }

    /// 处理键盘输入
    fn handle_text_input(&mut self) {
        // 处理字符输入
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii() && !ch.is_control() {
                match self.input_focus {
                    InputFocus::Account => {
                        if self.account.len() < 20 {
                            self.account.push(ch);
                        }
                    }
                    InputFocus::Password => {
                        if self.password.len() < 20 {
                            self.password.push(ch);
                        }
                    }
                    InputFocus::None => {}
                }
            }
        }

        // 处理退格键
        if is_key_pressed(KeyCode::Backspace) {
            match self.input_focus {
                InputFocus::Account => {
                    self.account.pop();
                }
                InputFocus::Password => {
                    self.password.pop();
                }
                InputFocus::None => {}
            }
        }

        // Tab 切换焦点
        if is_key_pressed(KeyCode::Tab) {
            self.input_focus = match self.input_focus {
                InputFocus::Account => InputFocus::Password,
                InputFocus::Password => InputFocus::Account,
                InputFocus::None => InputFocus::Account,
            };
        }

        // Enter 登录
        if is_key_pressed(KeyCode::Enter) {
            self.on_login_clicked();
        }
    }
}

impl Scene for LoginScene {
    fn name(&self) -> &str {
        "登录界面"
    }

    fn on_enter(&mut self) -> GameResult {
        self.account.clear();
        self.password.clear();
        self.load_config();
        println!("🎬 进入登录界面");
        Ok(())
    }

    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开登录界面");
        Ok(())
    }

    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
        // 更新光标闪烁
        self.cursor_timer += dt;
        if self.cursor_timer >= 0.5 {
            self.cursor_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
        }

        // 更新背景动画
        if self.animation_playing {
            self.frame_timer += dt;
            if self.frame_timer >= self.frame_delay {
                self.frame_timer = 0.0;
                self.background_frame += 1;

                if self.background_frame >= 19 {
                    println!("✓ Login animation finished, switching to character select...");
                    return Ok(SceneTransition::CharacterSelect);
                }
            }
        }

        self.handle_input()?;

        Ok(SceneTransition::None)
    }

    fn render(&mut self) -> GameResult {
        clear_background(BLACK);

        // 绘制背景动画 (ChrSel 库)
        let frame_index = if self.animation_playing {
            self.background_frame
        } else {
            0
        };

        if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, 0.0, 0.0, WHITE);
            }
        }

        // 如果没有播放动画，绘制登录对话框
        if !self.animation_playing {
            let (dialog_w, _dialog_h, dialog_x, dialog_y) = self.draw_login_background();
            
            // 绘制输入框
            let input_x = dialog_x + 86.0;
            let input_w = 136.0;
            let input_h = 18.0;
            
            // 账号输入框
            let account_y = dialog_y + 71.0;
            self.draw_input_box(input_x, account_y, input_w, input_h, &self.account, false, self.input_focus == InputFocus::Account);
            
            // 密码输入框
            let password_y = dialog_y + 93.0;
            self.draw_input_box(input_x, password_y, input_w, input_h, &self.password, true, self.input_focus == InputFocus::Password);
            
            // 绘制按钮
            let btn_y = dialog_y + 130.0;
            let btn_spacing = 80.0;
            let btn_start_x = dialog_x + (dialog_w - 4.0 * btn_spacing) / 2.0;
            
            // 登录按钮 (Prguse 192-194)
            if self.draw_button(btn_start_x, btn_y, 192, 193, 194) {
                self.on_login_clicked();
            }
            
            // 新建账号按钮 (Prguse 195-197)
            if self.draw_button(btn_start_x + btn_spacing, btn_y, 195, 196, 197) {
                println!("🆕 新建账号");
            }
            
            // 修改密码按钮 (Prguse 198-200)
            if self.draw_button(btn_start_x + btn_spacing * 2.0, btn_y, 198, 199, 200) {
                println!("🔑 修改密码");
            }
            
            // 退出按钮 (Prguse 201-203)
            if self.draw_button(btn_start_x + btn_spacing * 3.0, btn_y, 201, 202, 203) {
                std::process::exit(0);
            }
            
            // 处理点击输入框切换焦点
            let (mx, my) = mouse_position();
            if is_mouse_button_pressed(MouseButton::Left) {
                if mx >= input_x && mx <= input_x + input_w {
                    if my >= account_y && my <= account_y + input_h {
                        self.input_focus = InputFocus::Account;
                    } else if my >= password_y && my <= password_y + input_h {
                        self.input_focus = InputFocus::Password;
                    }
                }
            }
        }

        // 绘制消息框
        if self.show_message {
            self.draw_message_box();
            
            // 点击任意位置关闭消息框
            if is_mouse_button_pressed(MouseButton::Left) {
                self.show_message = false;
            }
        }

        Ok(())
    }

    fn handle_input(&mut self) -> GameResult {
        if is_key_pressed(KeyCode::Escape) {
            if self.show_message {
                self.show_message = false;
            } else {
                std::process::exit(0);
            }
        }

        // 处理文本输入
        if !self.animation_playing && !self.show_message {
            self.handle_text_input();
        }

        Ok(())
    }
}
