//! LoginScene ECS 组件定义
//! 
//! 将UI元素抽象为可复用的组件，每个实体由多个组件组合而成

use crate::graphics::LibraryName;

// ============== 标记组件 ==============

/// 标记：LoginScene的实体
#[derive(Debug, Clone, Copy)]
pub struct LoginSceneEntity;

/// 标记：背景实体
#[derive(Debug, Clone, Copy)]
pub struct BackgroundEntity;

/// 标记：对话框实体
#[derive(Debug, Clone, Copy)]
pub struct DialogEntity;

/// 标记：按钮实体
#[derive(Debug, Clone, Copy)]
pub struct ButtonEntity;

/// 标记：输入框实体
#[derive(Debug, Clone, Copy)]
pub struct TextInputEntity;

// ============== 空间组件 ==============

/// 位置组件
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// 尺寸组件
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// 边界框组件（用于点击检测）
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }
}

// ============== 渲染组件 ==============

/// 静态图片精灵
#[derive(Debug, Clone)]
pub struct Sprite {
    pub library: LibraryName,
    pub index: i32,
    pub visible: bool,
}

/// 动画精灵
#[derive(Debug, Clone)]
pub struct AnimatedSprite {
    pub library: LibraryName,
    pub start_index: i32,
    pub frame_count: usize,
    pub current_frame: usize,
    pub frame_duration: f32,  // 秒
    pub timer: f32,
    pub paused: bool,
    pub loop_animation: bool,
}

impl AnimatedSprite {
    pub fn new(library: LibraryName, start_index: i32, frame_count: usize, frame_duration: f32) -> Self {
        Self {
            library,
            start_index,
            frame_count,
            current_frame: 0,
            frame_duration,
            timer: 0.0,
            paused: true,
            loop_animation: false,
        }
    }

    pub fn current_index(&self) -> i32 {
        self.start_index + self.current_frame as i32
    }
}

// ============== 交互组件 ==============

/// 按钮组件
#[derive(Debug, Clone)]
pub struct Button {
    pub normal_index: i32,
    pub hover_index: i32,
    pub pressed_index: i32,
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub action: ButtonAction,
}

impl Button {
    pub fn current_index(&self) -> i32 {
        if !self.enabled {
            self.normal_index
        } else if self.pressed {
            self.pressed_index
        } else if self.hovered {
            self.hover_index
        } else {
            self.normal_index
        }
    }
}

/// 按钮动作枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    // LoginDialog
    Login,
    NewAccount,
    ChangePassword,
    
    // NewAccountDialog
    NewAccountOk,
    NewAccountCancel,
    
    // ChangePasswordDialog
    ChangePasswordOk,
    ChangePasswordCancel,
    
    // ConnectingBox
    CancelConnect,
}

/// 悬停状态组件
#[derive(Debug, Clone, Copy)]
pub struct HoverState {
    pub hovered: bool,
}

/// 可点击组件
#[derive(Debug, Clone, Copy)]
pub struct Clickable {
    pub enabled: bool,
}

// ============== 文本组件 ==============

/// 文本输入框组件
#[derive(Debug, Clone)]
pub struct TextInput {
    pub text: String,
    pub focused: bool,
    pub password: bool,
    pub max_length: usize,
    pub validation: InputValidation,
    pub valid: bool,
}

impl TextInput {
    pub fn new(max_length: usize, password: bool) -> Self {
        Self {
            text: String::new(),
            focused: false,
            password,
            max_length,
            validation: InputValidation::None,
            valid: true,
        }
    }

    pub fn validate(&mut self) {
        self.valid = match &self.validation {
            InputValidation::None => true,
            InputValidation::Regex(pattern) => {
                // 简单的验证逻辑
                !self.text.is_empty() && self.text.len() <= self.max_length
            }
            InputValidation::MinLength(min_len) => {
                self.text.len() >= *min_len
            }
            InputValidation::EmailFormat => {
                self.text.contains('@') && self.text.contains('.')
            }
            InputValidation::PasswordMatch(other_text) => {
                self.text == *other_text
            }
        };
    }
}

/// 输入验证规则
#[derive(Debug, Clone)]
pub enum InputValidation {
    None,
    Regex(String),
    MinLength(usize),
    EmailFormat,
    PasswordMatch(String),
}

/// 标签文本组件
#[derive(Debug, Clone)]
pub struct Label {
    pub text: String,
    pub font: String,
    pub size: f32,
    pub color: [u8; 4],  // RGBA
}

// ============== 状态组件 ==============

/// 可见性组件
#[derive(Debug, Clone, Copy)]
pub struct Visible(pub bool);

/// 启用状态组件
#[derive(Debug, Clone, Copy)]
pub struct Enabled(pub bool);

/// 聚焦状态组件
#[derive(Debug, Clone, Copy)]
pub struct Focused(pub bool);

// ============== 输入字段标识 ==============

/// 标识具体的输入框（用于自动聚焦）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFieldType {
    // LoginDialog
    LoginAccount,
    LoginPassword,
    
    // NewAccountDialog
    NewAccountId,
    NewAccountPassword,
    NewAccountConfirmPassword,
    NewAccountEmail,
    NewAccountName,
    NewAccountQuestion,
    NewAccountAnswer,
    NewAccountBirthday,
    
    // ChangePasswordDialog
    ChangePasswordAccount,
    ChangePasswordCurrent,
    ChangePasswordNew,
    ChangePasswordConfirm,
}

/// 输入字段组件（标识输入框类型）
#[derive(Debug, Clone, Copy)]
pub struct InputField {
    pub field_type: InputFieldType,
}
