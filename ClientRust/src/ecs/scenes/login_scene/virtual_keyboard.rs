//! 虚拟键盘 - 安全输入对话框
//! 功能: 防止键盘记录器,提供屏幕点击输入

use ggez::{Context, graphics::Canvas};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::Button;
use rand::seq::SliceRandom;

const KEYBOARD_WIDTH: f32 = 204.0;  // Prguse:1080 实际宽度
const KEYBOARD_HEIGHT: f32 = 276.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedInput {
    Account,
    Password,
}

/// 虚拟键盘对话框
pub struct VirtualKeyboard {
    pub visible: bool,
    pub x: f32,
    pub y: f32,
    pub focused_input: FocusedInput,  // 公开焦点状态
    
    // 按钮
    esc_button: Button,      // Esc - 关闭键盘
    delete_button: Button,   // Delete - 删除字符
    random_button: Button,   // Random - 随机打乱
    enter_button: Button,    // Enter - 确认
    
    // 字符按钮
    number_buttons: Vec<(char, Button)>,  // 0-9
    letter_buttons: Vec<(char, Button)>,  // A-Z
    
    // 字符数据
    numbers: Vec<char>,
    letters: Vec<char>,
}

impl VirtualKeyboard {
    pub fn new(base_w: f32, base_h: f32) -> Self {
        // 键盘居中偏右,参考 C# Location = (ScreenWidth/2 - Width/2 + 285, ScreenHeight/2 - Height/2 + 150)
        let x = (base_w - KEYBOARD_WIDTH) / 2.0 + 285.0;
        let y = (base_h - KEYBOARD_HEIGHT) / 2.0 + 150.0;
        
        // C#: Esc 按钮在 (12, 12), Title库 300-302
        let mut esc_button = Button::new_with_states(
            x + 12.0, y + 12.0,
            LibraryName::Title, 300, 301, 302
        );
        esc_button.text = "Esc".to_string();
        esc_button.center_text = true;
        
        // C#: Delete 按钮在 (140, 76), Title库 303-305
        let mut delete_button = Button::new_with_states(
            x + 140.0, y + 76.0,
            LibraryName::Title, 303, 304, 305
        );
        delete_button.text = "Delete".to_string();
        delete_button.center_text = true;
        
        // C#: Random 按钮在 (76, 236), Title库 309-311
        let mut random_button = Button::new_with_states(
            x + 76.0, y + 236.0,
            LibraryName::Title, 309, 310, 311
        );
        random_button.text = "Random".to_string();
        random_button.center_text = true;
        
        // C#: Enter 按钮在 (140, 236), Title库 306-308
        let mut enter_button = Button::new_with_states(
            x + 140.0, y + 236.0,
            LibraryName::Title, 306, 307, 308
        );
        enter_button.text = "Enter".to_string();
        enter_button.center_text = true;
        
        let mut keyboard = Self {
            visible: false,
            x, y,
            esc_button,
            delete_button,
            random_button,
            enter_button,
            number_buttons: Vec::new(),
            letter_buttons: Vec::new(),
            numbers: "0123456789".chars().collect(),
            letters: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect(),
            focused_input: FocusedInput::Password,
        };
        
        keyboard.rebuild_keys();
        keyboard
    }
    
    /// 显示虚拟键盘
    pub fn show(&mut self, focused: FocusedInput) {
        self.visible = true;
        self.focused_input = focused;
    }
    
    /// 隐藏虚拟键盘
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// 随机打乱键盘布局
    pub fn randomize(&mut self) {
        let mut rng = rand::rng();
        self.numbers.shuffle(&mut rng);
        self.letters.shuffle(&mut rng);
        self.rebuild_keys();
    }
    
    /// 重建所有键盘按钮
    fn rebuild_keys(&mut self) {
        self.number_buttons.clear();
        self.letter_buttons.clear();
        
        // C#: 数字键 0-9,每行6个,位置从 (12, 44) 开始,每个按钮 32×30
        // Location = (12 + (i % 6 * 32), 44 + (i / 6 * 32))
        for (i, &ch) in self.numbers.iter().enumerate() {
            let btn_x = self.x + 12.0 + ((i % 6) as f32 * 32.0);
            let btn_y = self.y + 44.0 + ((i / 6) as f32 * 32.0);
            
            let mut button = Button::new_with_states(
                btn_x, btn_y,
                LibraryName::Prguse, 1081, 1082, 1083
            );
            button.width = 32.0;
            button.height = 30.0;
            button.text = ch.to_string();
            button.center_text = true;
            
            self.number_buttons.push((ch, button));
        }
        
        // C#: 字母键 A-Z,每行6个,位置从 (12, 108) 开始
        // Location = (12 + (i % 6 * 32), 108 + (i / 6 * 32))
        for (i, &ch) in self.letters.iter().enumerate() {
            let btn_x = self.x + 12.0 + ((i % 6) as f32 * 32.0);
            let btn_y = self.y + 108.0 + ((i / 6) as f32 * 32.0);
            
            let mut button = Button::new_with_states(
                btn_x, btn_y,
                LibraryName::Prguse, 1081, 1082, 1083
            );
            button.width = 32.0;
            button.height = 30.0;
            button.text = ch.to_string();
            button.center_text = true;
            
            self.letter_buttons.push((ch, button));
        }
    }
    
    /// 更新鼠标悬停
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        if !self.visible { return; }
        
        self.esc_button.update_hover(x, y);
        self.delete_button.update_hover(x, y);
        self.random_button.update_hover(x, y);
        self.enter_button.update_hover(x, y);
        
        for (_, btn) in &mut self.number_buttons {
            btn.update_hover(x, y);
        }
        for (_, btn) in &mut self.letter_buttons {
            btn.update_hover(x, y);
        }
    }
    
    /// 处理点击,返回需要执行的动作
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> VirtualKeyboardAction {
        if !self.visible {
            return VirtualKeyboardAction::None;
        }
        
        // Esc - 关闭键盘
        if self.esc_button.contains(x, y) {
            return VirtualKeyboardAction::Close;
        }
        
        // Delete - 删除字符
        if self.delete_button.contains(x, y) {
            return VirtualKeyboardAction::Delete;
        }
        
        // Random - 随机打乱
        if self.random_button.contains(x, y) {
            self.randomize();
            return VirtualKeyboardAction::None;
        }
        
        // Enter - 确认(不需要特殊处理,关闭即可)
        if self.enter_button.contains(x, y) {
            return VirtualKeyboardAction::Close;
        }
        
        // 数字按钮
        for (ch, btn) in &self.number_buttons {
            if btn.contains(x, y) {
                return VirtualKeyboardAction::Input(*ch);
            }
        }
        
        // 字母按钮
        for (ch, btn) in &self.letter_buttons {
            if btn.contains(x, y) {
                return VirtualKeyboardAction::Input(*ch);
            }
        }
        
        VirtualKeyboardAction::None
    }
    
    /// 绘制虚拟键盘
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }
        
        // 绘制背景 - Prguse库 Index=1080
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 1080, self.x, self.y)?;
        
        // 绘制控制按钮
        self.esc_button.draw(ctx, canvas)?;
        self.delete_button.draw(ctx, canvas)?;
        self.random_button.draw(ctx, canvas)?;
        self.enter_button.draw(ctx, canvas)?;
        
        // 绘制数字按钮
        for (_, btn) in &self.number_buttons {
            btn.draw(ctx, canvas)?;
        }
        
        // 绘制字母按钮
        for (_, btn) in &self.letter_buttons {
            btn.draw(ctx, canvas)?;
        }
        
        // TODO: 绘制按钮文字 (需要文本渲染系统支持)
        
        Ok(())
    }
}

/// 虚拟键盘操作
#[derive(Debug, Clone, PartialEq)]
pub enum VirtualKeyboardAction {
    None,
    Close,           // 关闭键盘
    Delete,          // 删除字符
    Input(char),     // 输入字符
}
