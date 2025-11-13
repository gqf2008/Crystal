// LoginScene - 登录界面

use crate::compat::GameResult;
use crate::scenes::{SceneHandler, SceneTransition};
use macroquad::prelude::*;

pub struct LoginScene {
    account_input: String,
    password_input: String,
    focused_input: usize,
}

impl LoginScene {
    pub fn new() -> Self {
        Self {
            account_input: String::new(),
            password_input: String::new(),
            focused_input: 0,
        }
    }
    
    fn draw_input_box(&self, label: &str, text: &str, x: f32, y: f32, w: f32, h: f32, focused: bool) {
        let bg = if focused { 
            Color::from_rgba(60, 60, 80, 255) 
        } else { 
            Color::from_rgba(40, 40, 60, 255) 
        };
        draw_rectangle(x, y, w, h, bg);
        
        let border = if focused { 
            Color::from_rgba(100, 150, 255, 255) 
        } else { 
            Color::from_rgba(80, 80, 100, 255) 
        };
        draw_rectangle_lines(x, y, w, h, 2.0, border);
        
        draw_text_ex(label, x, y - 10.0, TextParams {
            font: None, font_size: 20, color: WHITE, ..Default::default()
        });
        
        draw_text_ex(text, x + 10.0, y + h / 2.0 + 8.0, TextParams {
            font: None, font_size: 24, color: WHITE, ..Default::default()
        });
    }
}

impl SceneHandler for LoginScene {
    fn name(&self) -> &str {
        "登录界面"
    }
    
    fn on_enter(&mut self) -> GameResult {
        self.account_input.clear();
        self.password_input.clear();
        self.focused_input = 0;
        println!("🎬 进入登录界面");
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开登录界面");
        Ok(())
    }
    
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        if is_key_pressed(KeyCode::Enter) 
            && !self.account_input.is_empty() 
            && !self.password_input.is_empty() 
        {
            println!("🔐 登录: {}", self.account_input);
            return Ok(SceneTransition::CharacterSelect);
        }
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        clear_background(Color::from_rgba(20, 20, 30, 255));
        
        let w = screen_width();
        let h = screen_height();
        
        draw_text_ex("传奇2 - 登录", w / 2.0 - 150.0, 100.0, TextParams {
            font: None, font_size: 64, color: Color::from_rgba(255, 200, 100, 255), ..Default::default()
        });
        
        let iw = 400.0;
        let ih = 50.0;
        let ix = w / 2.0 - iw / 2.0;
        
        self.draw_input_box("账号:", &self.account_input, ix, h / 2.0 - 100.0, iw, ih, self.focused_input == 0);
        self.draw_input_box("密码:", &"*".repeat(self.password_input.len()), ix, h / 2.0 - 20.0, iw, ih, self.focused_input == 1);
        
        draw_text_ex("Tab: 切换 | Enter: 登录 | ESC: 退出", w / 2.0 - 200.0, h - 50.0, TextParams {
            font: None, font_size: 16, color: GRAY, ..Default::default()
        });
        
        Ok(())
    }
    
    fn handle_input(&mut self) -> GameResult {
        if is_key_pressed(KeyCode::Tab) {
            self.focused_input = (self.focused_input + 1) % 2;
        }
        
        if let Some(ch) = get_char_pressed() {
            if ch.is_alphanumeric() || ch == '_' {
                if self.focused_input == 0 {
                    self.account_input.push(ch);
                } else {
                    self.password_input.push(ch);
                }
            }
        }
        
        if is_key_pressed(KeyCode::Backspace) {
            if self.focused_input == 0 {
                self.account_input.pop();
            } else {
                self.password_input.pop();
            }
        }
        
        Ok(())
    }
}
