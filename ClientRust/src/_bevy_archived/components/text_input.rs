// Text Input Component - 简单的文本输入框组件
use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;

/// 文本输入框组件
#[derive(Component, Debug)]
pub struct TextInput {
    /// 当前文本内容
    pub text: String,
    /// 最大长度
    pub max_length: usize,
    /// 是否为密码框 (显示星号)
    pub is_password: bool,
    /// 是否获得焦点
    pub focused: bool,
    /// 光标位置
    pub cursor_position: usize,
    /// 允许的字符类型
    pub allowed_chars: CharFilter,
}

/// 字符过滤器
#[derive(Debug, Clone)]
pub enum CharFilter {
    /// 所有字符
    All,
    /// 仅字母和数字
    AlphaNumeric,
    /// 自定义过滤函数
    Custom(fn(char) -> bool),
}

impl TextInput {
    /// 创建新的文本输入框
    pub fn new(max_length: usize) -> Self {
        Self {
            text: String::new(),
            max_length,
            is_password: false,
            focused: false,
            cursor_position: 0,
            allowed_chars: CharFilter::All,
        }
    }
    
    /// 设置为密码框
    pub fn password(mut self) -> Self {
        self.is_password = true;
        self
    }
    
    /// 设置字符过滤器
    pub fn with_filter(mut self, filter: CharFilter) -> Self {
        self.allowed_chars = filter;
        self
    }
    
    /// 设置初始文本
    pub fn with_text(mut self, text: String) -> Self {
        self.text = text.chars().take(self.max_length).collect();
        self.cursor_position = self.text.len();
        self
    }
    
    /// 插入字符
    pub fn insert_char(&mut self, c: char) {
        if self.text.len() >= self.max_length {
            return;
        }
        
        // 检查字符是否允许
        if !self.is_char_allowed(c) {
            return;
        }
        
        self.text.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }
    
    /// 删除光标前的字符 (Backspace)
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.text.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }
    
    /// 删除光标后的字符 (Delete)
    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.text.len() {
            self.text.remove(self.cursor_position);
        }
    }
    
    /// 移动光标到左侧
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }
    
    /// 移动光标到右侧
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.len() {
            self.cursor_position += 1;
        }
    }
    
    /// 移动光标到开头
    pub fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }
    
    /// 移动光标到结尾
    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.text.len();
    }
    
    /// 清空文本
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_position = 0;
    }
    
    /// 获取显示文本 (密码框显示星号)
    pub fn display_text(&self) -> String {
        if self.is_password {
            "*".repeat(self.text.len())
        } else {
            self.text.clone()
        }
    }
    
    /// 检查字符是否允许
    fn is_char_allowed(&self, c: char) -> bool {
        match &self.allowed_chars {
            CharFilter::All => true,
            CharFilter::AlphaNumeric => c.is_alphanumeric(),
            CharFilter::Custom(f) => f(c),
        }
    }
}

/// 文本输入焦点标记
#[derive(Component)]
pub struct TextInputFocused;

/// 文本输入系统 - 处理键盘输入
pub fn text_input_system(
    mut keyboard_events: MessageReader<KeyboardInput>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut TextInput>,
) {
    // 找到获得焦点的输入框
    let mut focused_input = None;
    for (i, input) in query.iter().enumerate() {
        if input.focused {
            focused_input = Some(i);
            break;
        }
    }
    
    let Some(focused_index) = focused_input else {
        return;
    };
    
    // 获取可变引用
    let mut inputs: Vec<_> = query.iter_mut().collect();
    if focused_index >= inputs.len() {
        return;
    }
    
    let input = &mut inputs[focused_index];
    
    // 处理键盘输入 - 在Bevy 0.17中使用KeyCode处理
    // 注意: 这里简化处理,仅支持基本字符和控制键
    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        
        // key_code 已经是 KeyCode 类型,不是 Option
        let key_code = event.key_code;
        
        // 处理字符输入 (简化版本 - 仅支持字母和数字)
        match key_code {
            KeyCode::KeyA => input.insert_char('a'),
            KeyCode::KeyB => input.insert_char('b'),
            KeyCode::KeyC => input.insert_char('c'),
            KeyCode::KeyD => input.insert_char('d'),
            KeyCode::KeyE => input.insert_char('e'),
            KeyCode::KeyF => input.insert_char('f'),
            KeyCode::KeyG => input.insert_char('g'),
            KeyCode::KeyH => input.insert_char('h'),
            KeyCode::KeyI => input.insert_char('i'),
            KeyCode::KeyJ => input.insert_char('j'),
            KeyCode::KeyK => input.insert_char('k'),
            KeyCode::KeyL => input.insert_char('l'),
            KeyCode::KeyM => input.insert_char('m'),
            KeyCode::KeyN => input.insert_char('n'),
            KeyCode::KeyO => input.insert_char('o'),
            KeyCode::KeyP => input.insert_char('p'),
            KeyCode::KeyQ => input.insert_char('q'),
            KeyCode::KeyR => input.insert_char('r'),
            KeyCode::KeyS => input.insert_char('s'),
            KeyCode::KeyT => input.insert_char('t'),
            KeyCode::KeyU => input.insert_char('u'),
            KeyCode::KeyV => input.insert_char('v'),
            KeyCode::KeyW => input.insert_char('w'),
            KeyCode::KeyX => input.insert_char('x'),
            KeyCode::KeyY => input.insert_char('y'),
            KeyCode::KeyZ => input.insert_char('z'),
            KeyCode::Digit0 => input.insert_char('0'),
            KeyCode::Digit1 => input.insert_char('1'),
            KeyCode::Digit2 => input.insert_char('2'),
            KeyCode::Digit3 => input.insert_char('3'),
            KeyCode::Digit4 => input.insert_char('4'),
            KeyCode::Digit5 => input.insert_char('5'),
            KeyCode::Digit6 => input.insert_char('6'),
            KeyCode::Digit7 => input.insert_char('7'),
            KeyCode::Digit8 => input.insert_char('8'),
            KeyCode::Digit9 => input.insert_char('9'),
            _ => {}
        }
    }
    
    // 处理特殊按键
    if keyboard_input.just_pressed(KeyCode::Backspace) {
        input.delete_char();
    }
    
    if keyboard_input.just_pressed(KeyCode::Delete) {
        input.delete_char_forward();
    }
    
    if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        input.move_cursor_left();
    }
    
    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        input.move_cursor_right();
    }
    
    if keyboard_input.just_pressed(KeyCode::Home) {
        input.move_cursor_home();
    }
    
    if keyboard_input.just_pressed(KeyCode::End) {
        input.move_cursor_end();
    }
}

/// 文本输入焦点系统 - 处理点击焦点切换
pub fn text_input_focus_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut input_query: Query<(&mut TextInput, &Node, &GlobalTransform, Entity)>,
) {
    // 检查是否有鼠标点击
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }
    
    // 获取鼠标位置
    let Some(window) = windows.iter().next() else {
        return;
    };
    
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    
    // 检查是否点击了某个输入框
    // TODO: 在 Bevy 0.17 中实现正确的边界框检测
    // Node组件的API已经改变,暂时简化处理
    
    for (mut input, _node, _transform, _entity) in input_query.iter_mut() {
        // 简单地让第一个输入框获得焦点
        input.focused = true;
        break;
    }
}

/// 文本输入渲染系统 - 更新显示文本
pub fn text_input_render_system(
    mut query: Query<(&TextInput, &mut Text), Changed<TextInput>>,
) {
    for (input, mut text) in query.iter_mut() {
        let display = input.display_text();
        
        // 添加光标显示
        if input.focused {
            let before = &display[..input.cursor_position.min(display.len())];
            let after = &display[input.cursor_position.min(display.len())..];
            text.0 = format!("{}|{}", before, after);
        } else {
            text.0 = display;
        }
    }
}
