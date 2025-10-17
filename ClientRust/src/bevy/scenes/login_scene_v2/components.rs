// Shared components, resources, and messages for LoginScene
// Extracted from login_scene_v2.rs for better modularity

use bevy::prelude::*;

// ============================================================================
// Constants
// ============================================================================

/// Animation constants
pub const ANIMATION_FRAME_COUNT: usize = 19;
pub const ANIMATION_DELAY: f32 = 0.1; // 100ms per frame

/// Dialog dimensions
pub const DIALOG_WIDTH: f32 = 328.0;
pub const DIALOG_HEIGHT: f32 = 220.0;

/// Input validation constants
pub const MIN_ACCOUNT_ID_LENGTH: usize = 3;
pub const MAX_ACCOUNT_ID_LENGTH: usize = 15;
pub const MIN_PASSWORD_LENGTH: usize = 5;
pub const MAX_PASSWORD_LENGTH: usize = 15;

/// UI Colors
pub const INPUT_BORDER_NORMAL: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);
pub const INPUT_BORDER_FOCUSED: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);
pub const INPUT_BORDER_VALID: Color = Color::srgba(0.0, 1.0, 0.0, 1.0);
pub const INPUT_BORDER_INVALID: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);
pub const TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

// ============================================================================
// Resources
// ============================================================================

#[derive(Resource, Debug, Clone)]
pub struct LoginState {
    /// Network connection state
    pub connecting: bool,
    pub connect_attempts: u32,
    
    /// Version check state
    pub version_checked: bool,
    pub version_valid: bool,
    
    /// Login enabled state
    pub login_enabled: bool,
    pub require_password_change: bool,
    
    /// Background animation state
    pub background_frame: usize,
    pub animation_timer: f32,
    
    /// Input state
    pub account_id: String,
    pub password: String,
    pub account_id_valid: bool,
    pub password_valid: bool,
    
    /// Active dialog
    pub active_dialog: Option<DialogType>,
}

/// Dialog type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    NewAccount,
    ChangePassword,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
            version_checked: true,
            version_valid: true,
            login_enabled: false,
            require_password_change: false,
            background_frame: 0,
            animation_timer: 0.0,
            account_id: String::new(),
            password: String::new(),
            account_id_valid: false,
            password_valid: false,
            active_dialog: None,
        }
    }
}

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

/// Marker for input cursor
#[derive(Component)]
pub struct InputCursor {
    pub blink_timer: f32,
}

#[derive(Component)]
pub struct ButtonType(pub LoginButtonType);

#[derive(Component, Clone)]
pub struct ButtonTextures {
    pub normal_index: u32,
    pub hover_index: u32,
    pub pressed_index: u32,
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

/// Marker for dialog input fields
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct DialogInputField(pub DialogInputType);

/// Dialog input field type alias for compatibility
pub type DialogInputType = DialogFieldType;

/// Marker for input text node  
#[derive(Component)]
pub struct InputText;

/// Dialog input field types
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum DialogFieldType {
    // New Account fields
    NewAccountId,
    NewPassword,
    NewPasswordConfirm,
    NewEmail,
    NewUsername,
    NewBirthDate,
    NewQuestion,
    NewAnswer,
    
    // Change Password fields
    ChangeAccountId,
    ChangeCurrentPassword,
    ChangeNewPassword,
    ChangeNewPasswordConfirm,
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
