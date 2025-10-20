// Shared components, resources, and messages for LoginScene
// Extracted from login_scene_v2.rs for better modularity

use bevy::prelude::*;

// Constants are now in constants.rs
// Resources (LoginState, DialogType) are now in resources.rs

// ============================================================================
// Component Markers
// ============================================================================

#[derive(Component)]
pub struct LoginSceneRoot;

#[derive(Component)]
pub struct LoginBackground {
    pub current_frame: usize,
}

#[derive(Component)]
pub struct LoginDialog;

#[derive(Component)]
pub struct AccountIdInput;

#[derive(Component)]
pub struct PasswordInput;

#[derive(Component)]
pub struct InputFocused;

/// Marker for input cursor - 带闪烁计时器和可见性标志
#[derive(Component)]
pub struct InputCursor {
    pub blink_timer: Timer,
    pub visible: bool,
}

#[derive(Component)]
pub struct ButtonType(pub LoginButtonType);

/// 按钮纹理索引 - 用于按钮hover效果
#[derive(Component, Clone)]
pub struct ButtonTextures {
    pub normal_index: i32,
    pub hover_index: i32,
    pub pressed_index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginButtonType {
    Login,
    NewAccount,
    PasswordChange,
    ViewKey,
    Close,
    DialogOK,
    DialogCancel,
}

/// Marker for new account dialog
#[derive(Component)]
pub struct NewAccountDialog;

/// Marker for change password dialog
#[derive(Component)]
pub struct ChangePasswordDialog;

/// Marker for any dialog
#[derive(Component)]
pub struct Dialog;

/// Marker for dialog input fields - 包含输入字段类型
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct DialogInputField {
    pub field_type: DialogFieldType,
}

/// Marker for input text node  
#[derive(Component)]
pub struct InputText;

/// Dialog input field types
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DialogFieldType {
    // New Account fields
    NewAccountId,
    NewPassword1,
    NewPassword2,
    NewEmail,
    NewUserName,
    NewBirthDate,
    NewQuestion,
    NewAnswer,
    
    // Change Password fields
    ChangeAccountId,
    ChangeCurrentPassword,
    ChangeNewPassword1,
    ChangeNewPassword2,
}

// ============================================================================
// Messages (Events)
// ============================================================================

#[derive(Message, Clone)]
pub struct LoginButtonPressed {
    pub account_id: String,
    pub password: String,
}

#[derive(Message, Clone, Default)]
pub struct NewAccountButtonPressed;

#[derive(Message, Clone, Default)]
pub struct PasswordChangeButtonPressed;

#[derive(Message, Clone, Default)]
pub struct ViewKeyButtonPressed;

#[derive(Message, Clone, Default)]
pub struct CloseButtonPressed;
