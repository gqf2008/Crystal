// SelectScene Components - 角色选择场景的组件和资源定义

use bevy::prelude::*;
use std::collections::HashMap;

/// 角色选择场景的全局状态资源
#[derive(Resource, Debug)]
pub struct SelectSceneState {
    /// 可选的角色列表
    pub characters: Vec<CharacterInfo>,
    
    /// 当前选中的角色索引
    pub selected_index: Option<usize>,
    
    /// 是否显示新角色创建对话框
    pub show_create_dialog: bool,
    
    /// 是否显示删除确认对话框
    pub show_delete_dialog: bool,
    
    /// 删除确认的角色索引
    pub delete_confirm_index: Option<usize>,
    
    /// 新角色创建时的输入
    pub new_character_name: String,
    pub new_character_class: u8,
    pub new_character_gender: u8,
    
    /// 动画状态
    pub animation_timer: f32,
    pub is_animating: bool,
}

impl Default for SelectSceneState {
    fn default() -> Self {
        Self {
            characters: Vec::new(),
            selected_index: None,
            show_create_dialog: false,
            show_delete_dialog: false,
            delete_confirm_index: None,
            new_character_name: String::new(),
            new_character_class: 0,
            new_character_gender: 0,
            animation_timer: 0.0,
            is_animating: false,
        }
    }
}

/// 角色信息 - 从服务器接收的角色数据
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    pub experience: i64,
    pub hair: u8,
    pub deleted: bool,
}

// ============================================================================
// Messages for SelectScene
// ============================================================================

/// 选择角色消息
#[derive(Message, Clone, Default)]
pub struct SelectCharacterMessage {
    pub index: usize,
}

/// 删除角色消息
#[derive(Message, Clone, Default)]
pub struct DeleteCharacterMessage {
    pub index: usize,
}

/// 创建新角色消息
#[derive(Message, Clone, Default)]
pub struct CreateCharacterMessage {
    pub name: String,
    pub class: u8,
    pub gender: u8,
}

/// 开始游戏消息
#[derive(Message, Clone, Default)]
pub struct StartGameMessage {
    pub character_index: i32,
}

/// 返回登录消息
#[derive(Message, Clone, Default)]
pub struct BackToLoginMessage;

// ============================================================================
// UI Components
// ============================================================================

/// 选择场景根节点
#[derive(Component)]
pub struct SelectSceneRoot;

/// 背景
#[derive(Component)]
pub struct SelectBackground;

/// 角色列表容器
#[derive(Component)]
pub struct CharacterListContainer;

/// 单个角色项目
#[derive(Component)]
pub struct CharacterItem {
    pub index: usize,
}

/// 角色选择按钮
#[derive(Component)]
pub struct SelectButton {
    pub character_index: usize,
}

/// 删除按钮
#[derive(Component)]
pub struct DeleteButton {
    pub character_index: usize,
}

/// 创建新角色按钮
#[derive(Component, Default)]
pub struct CreateCharacterButton;

/// 开始游戏按钮
#[derive(Component, Default)]
pub struct StartGameButton;

/// 返回登录按钮
#[derive(Component, Default)]
pub struct BackToLoginButton;

/// 新角色创建对话框
#[derive(Component)]
pub struct CreateDialog;

/// 删除确认对话框
#[derive(Component)]
pub struct DeleteConfirmDialog;

/// 角色名称输入框
#[derive(Component)]
pub struct CharacterNameInput;

/// 职业选择按钮
#[derive(Component)]
pub struct ClassSelectButton {
    pub class: u8,
}

/// 性别选择按钮
#[derive(Component)]
pub struct GenderSelectButton {
    pub gender: u8,
}

// ============================================================================
// Constants
// ============================================================================

pub const BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 1.0);
pub const BUTTON_COLOR: Color = Color::srgba(0.2, 0.2, 0.3, 1.0);
pub const BUTTON_HOVER_COLOR: Color = Color::srgba(0.3, 0.3, 0.4, 1.0);
pub const BUTTON_PRESSED_COLOR: Color = Color::srgba(0.15, 0.15, 0.25, 1.0);
pub const TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
pub const SELECTED_COLOR: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);
pub const ERROR_COLOR: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);

/// 最大可创建的角色数
pub const MAX_CHARACTERS: usize = 3;

/// 职业列表
pub const CLASSES: &[(&str, u8)] = &[
    ("战士 (Warrior)", 0),
    ("道士 (Taoist)", 1),
    ("法师 (Wizard)", 2),
];

/// 性别列表
pub const GENDERS: &[(&str, u8)] = &[
    ("男 (Male)", 0),
    ("女 (Female)", 1),
];
