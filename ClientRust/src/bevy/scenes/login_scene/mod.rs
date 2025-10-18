// LoginScene for Bevy 0.17.2 - Modularized Implementation
// Migrated from Client/MirScenes/LoginScene.cs

use bevy::prelude::*;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::picking::hover::Hovered;
use bevy::ui_widgets::Button;
use regex::Regex;

// Sub-modules
pub mod components;
pub mod button_systems;
pub mod input_systems;
pub mod ui_helpers;

// ============================================================================
// Constants
// ============================================================================

/// Animation constants
const ANIMATION_FRAME_COUNT: usize = 19;
const ANIMATION_DELAY: f32 = 0.1; // 100ms per frame

/// Dialog dimensions
const DIALOG_WIDTH: f32 = 328.0;
const DIALOG_HEIGHT: f32 = 220.0;

/// Input validation constants
const MIN_ACCOUNT_ID_LENGTH: usize = 3;
const MAX_ACCOUNT_ID_LENGTH: usize = 15;
const MIN_PASSWORD_LENGTH: usize = 5;
const MAX_PASSWORD_LENGTH: usize = 15;

/// UI Colors
const BUTTON_NORMAL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
const BUTTON_HOVER_COLOR: Color = Color::srgba(0.9, 0.9, 0.9, 1.0);
const BUTTON_PRESSED_COLOR: Color = Color::srgba(0.8, 0.8, 0.8, 1.0);
const INPUT_BORDER_NORMAL: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);
const INPUT_BORDER_FOCUSED: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);
const INPUT_BORDER_VALID: Color = Color::srgba(0.0, 1.0, 0.0, 1.0);
const INPUT_BORDER_INVALID: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);
const TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

// ============================================================================
// Resources
// ============================================================================

#[derive(Resource, Debug)]
pub struct LoginState {
    /// Network connection state
    pub connecting: bool,
    pub connect_attempts: u32,
    
    /// Version check state
    pub version_checked: bool,
    pub version_valid: bool,
    
    /// Login enabled state
    pub login_enabled: bool,
    pub login_success: bool,
    pub frames_after_login: usize,  // 登录后经过的帧数
    
    /// Background animation state
    pub background_frame: usize,
    pub animation_timer: f32,
    pub animation_paused: bool,
    
    /// Input values
    pub account_id: String,
    pub password: String,
    
    /// Input validation
    pub account_id_valid: bool,
    pub password_valid: bool,
    
    /// Dialog state
    pub dialog_visible: DialogType,
    
    /// Dialog input values
    pub dialog_inputs: std::collections::HashMap<DialogFieldType, String>,
    
    /// Network command sender - for sending login requests to network thread
    pub command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
}

/// Dialog type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    None,
    NewAccount,
    ChangePassword,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
            version_checked: true, // 跳过版本检查用于测试
            version_valid: true,
            login_enabled: false,
            login_success: false,
            frames_after_login: 0,
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: true,  // 启动时暂停动画,登录成功后才开始播放
            account_id: String::new(),
            password: String::new(),
            account_id_valid: false,
            password_valid: false,
            dialog_visible: DialogType::None,
            dialog_inputs: std::collections::HashMap::new(),
            command_tx: None,
        }
    }
}

impl LoginState {
    /// Set the network command sender for sending login requests
    pub fn set_command_sender(&mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>) {
        self.command_tx = Some(tx);
    }
}

// ============================================================================
// Components
// ============================================================================

#[derive(Component)]
pub struct LoginSceneRoot;

#[derive(Component)]
pub struct LoginBackground {
    pub frame: usize,
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
    pub blink_timer: Timer,
    pub visible: bool,
}

#[derive(Component)]
pub struct ButtonType(pub LoginButtonType);

#[derive(Component)]
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
    // Dialog buttons
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
pub struct DialogInputField {
    pub field_type: DialogFieldType,
}

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
// Messages
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

// ============================================================================
// Setup System
// ============================================================================

pub fn setup_login_scene(
    mut commands: Commands,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    info!("🎮 Setting up LoginScene");
    
    // Insert LoginState resource
    commands.insert_resource(LoginState::default());
    info!("✅ LoginState 已创建");
    
    // Create root entity (full screen container)
    let root = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        LoginSceneRoot,
        Name::new("LoginSceneRoot"),
    )).id();
    
    info!("✅ Root 实体已创建");
    
    // Spawn background with animation
    spawn_animated_background(&mut commands, root, &mut mlibrary_assets, &mut images);
    
    // Spawn login dialog
    spawn_login_dialog_v2(&mut commands, root, &mut mlibrary_assets, &mut images, &asset_server);
    
    // Spawn version label
    spawn_version_label(&mut commands, root, &asset_server);
    
    info!("🎉 LoginScene 设置完成!");
}

/// Initialize network command channel for LoginScene
/// This sets up the communication channel between Bevy and the network thread
pub fn init_network_channel(
    mut login_state: Option<ResMut<LoginState>>,
) {
    // Only process if LoginState exists (only during Login state)
    if login_state.is_none() {
        return;
    }
    
    if let Some(mut _login_state) = login_state {
        // Create a channel for sending network commands from UI to network thread
        // Note: The receiver will be handled by the network manager thread
        let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<crate::network::NetworkCommand>();
        
        // For now, we don't have a real network manager running yet
        // In a full implementation, you would receive the tx from an existing network manager
        // and store it in login_state.command_tx
        
        // TODO: Integrate with real NetworkManager when available
        info!("📡 Network channel initialization (waiting for NetworkManager integration)");
    }
}

// ============================================================================
// Background Animation
// ============================================================================

fn spawn_animated_background(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
) {
    info!("🔄 加载背景动画 (ChrSel:0-18)...");
    
    // Load first frame as default
    let background_texture = match mlibrary_assets.get_texture("ChrSel", 0, images) {
        Some(texture) => {
            info!("✅ 背景纹理加载成功");
            texture.clone()
        }
        None => {
            error!("❌ 背景纹理加载失败!");
            return;
        }
    };
    
    let background = commands.spawn((
        ImageNode {
            image: background_texture,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        LoginBackground { frame: 0 },
        Name::new("LoginBackground"),
    )).id();
    
    commands.entity(parent).add_child(background);
}

/// Update background animation system
/// Update background animation and handle scene transition after login
pub fn update_background_animation(
    time: Res<Time>,
    mut login_state: ResMut<LoginState>,
    mut query: Query<(&mut ImageNode, &mut LoginBackground)>,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
) {
    if login_state.animation_paused {
        return;
    }
    
    login_state.animation_timer += time.delta_secs();
    
    if login_state.animation_timer >= ANIMATION_DELAY {
        login_state.animation_timer = 0.0;
        
        // Advance to next frame
        let old_frame = login_state.background_frame;
        login_state.background_frame = (login_state.background_frame + 1) % ANIMATION_FRAME_COUNT;
        
        // 如果登录成功,计数帧数并检查是否完成一轮完整动画
        if login_state.login_success {
            login_state.frames_after_login += 1;
            if login_state.frames_after_login >= ANIMATION_FRAME_COUNT {
                info!("🎬 Animation cycle completed ({} frames), transitioning to Select scene", 
                    login_state.frames_after_login);
                next_state.set(crate::bevy::GameState::Select);
                return;
            }
        }
        
        // Update background texture
        for (mut image_node, mut background) in query.iter_mut() {
            if let Some(texture) = mlibrary_assets.get_texture("ChrSel", login_state.background_frame as i32, &mut images) {
                image_node.image = texture.clone();
                background.frame = login_state.background_frame;
            }
        }
    }
}

// ============================================================================
// Login Dialog
// ============================================================================

fn spawn_login_dialog_v2(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
    asset_server: &Res<AssetServer>,
) {
    info!("🔄 加载登录对话框...");
    
    // Load dialog background texture
    let dialog_bg = match mlibrary_assets.get_texture("Prguse", 1084, images) {
        Some(texture) => texture.clone(),
        None => {
            error!("❌ 对话框背景加载失败!");
            return;
        }
    };
    
    // Create dialog container
    let dialog = commands.spawn((
        ImageNode {
            image: dialog_bg,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(DIALOG_WIDTH),
            height: Val::Px(DIALOG_HEIGHT),
            left: Val::Px((1024.0 - DIALOG_WIDTH) / 2.0), // Center horizontally
            top: Val::Px((768.0 - DIALOG_HEIGHT) / 2.0), // Center vertically
            ..default()
        },
        LoginDialog,
        Name::new("LoginDialog"),
    )).id();
    
    commands.entity(parent).add_child(dialog);
    
    // Add dialog contents
    spawn_dialog_contents_v2(commands, dialog, mlibrary_assets, images, asset_server);
}

fn spawn_dialog_contents_v2(
    commands: &mut Commands,
    dialog: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
    asset_server: &Res<AssetServer>,
) {
    // Load textures
    let title_tex = mlibrary_assets.get_texture("Title", 30, images).unwrap().clone();
    let account_label_tex = mlibrary_assets.get_texture("Title", 31, images).unwrap().clone();
    let pass_label_tex = mlibrary_assets.get_texture("Title", 32, images).unwrap().clone();
    
    // Load button textures (only normal state initially, hover/pressed will be loaded on demand)
    let ok_button_tex = mlibrary_assets.get_texture("Title", 320, images).unwrap().clone();
    let account_button_tex = mlibrary_assets.get_texture("Title", 323, images).unwrap().clone();
    let pass_button_tex = mlibrary_assets.get_texture("Title", 326, images).unwrap().clone();
    let view_key_button_tex = mlibrary_assets.get_texture("Title", 332, images).unwrap().clone();
    let close_button_tex = mlibrary_assets.get_texture("Title", 329, images).unwrap().clone();
    
    // Load font (使用支持中文的字体)
    let font = asset_server.load("fonts/NotoSansSC-Regular.ttf");
    
    commands.entity(dialog).with_children(|parent| {
        // Title label image
        parent.spawn((
            ImageNode {
                image: title_tex,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((DIALOG_WIDTH - 160.0) / 2.0),
                top: Val::Px(12.0),
                ..default()
            },
            Name::new("TitleLabel"),
        ));
        
        // Account ID label
        parent.spawn((
            ImageNode {
                image: account_label_tex,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(52.0),
                top: Val::Px(83.0),
                ..default()
            },
            Name::new("AccountIDLabel"),
        ));
        
        // Password label
        parent.spawn((
            ImageNode {
                image: pass_label_tex,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(43.0),
                top: Val::Px(105.0),
                ..default()
            },
            Name::new("PasswordLabel"),
        ));
        
        // Account ID Input
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(85.0),
                top: Val::Px(85.0),
                width: Val::Px(136.0),
                height: Val::Px(20.0),
                padding: UiRect::all(Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
            BorderColor::all(INPUT_BORDER_NORMAL),
            AccountIdInput,
            Button,
            Interaction::default(),
            Name::new("AccountInput"),
        )).with_children(|input_parent| {
            // Add text child
            input_parent.spawn((
                Text::new("账号"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
            ));
            
            // Add cursor child
            input_parent.spawn((
                Text::new("|"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                InputCursor {
                    blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                    visible: false,
                },
                Visibility::Hidden,
            ));
        });
        
        // Password Input
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(85.0),
                top: Val::Px(108.0),
                width: Val::Px(136.0),
                height: Val::Px(20.0),
                padding: UiRect::all(Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
            BorderColor::all(INPUT_BORDER_NORMAL),
            PasswordInput,
            Button,
            Interaction::default(),
            Name::new("PasswordInput"),
        )).with_children(|input_parent| {
            // Add text child
            input_parent.spawn((
                Text::new("密码"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 1.0)),
            ));
            
            // Add cursor child
            input_parent.spawn((
                Text::new("|"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                InputCursor {
                    blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                    visible: false,
                },
                Visibility::Hidden,
            ));
        });
        
        // OK Button (Login)
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(227.0),
                top: Val::Px(81.0),
                width: Val::Px(42.0),
                height: Val::Px(42.0),
                ..default()
            },
            ImageNode::from(ok_button_tex),
            Button,
            Hovered::default(),
            Interaction::default(),
            ButtonType(LoginButtonType::Login),
            ButtonTextures {
                normal_index: 320,
                hover_index: 321,
                pressed_index: 322,
            },
            Name::new("OKButton"),
        ));
        
        // New Account Button
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(60.0),
                top: Val::Px(163.0),
                width: Val::Px(80.0),
                height: Val::Px(20.0),
                ..default()
            },
            ImageNode::from(account_button_tex),
            Button,
            Hovered::default(),
            Interaction::default(),
            ButtonType(LoginButtonType::NewAccount),
            ButtonTextures {
                normal_index: 323,
                hover_index: 324,
                pressed_index: 325,
            },
            Name::new("AccountButton"),
        ));
        
        // Password Change Button
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(166.0),
                top: Val::Px(163.0),
                width: Val::Px(80.0),
                height: Val::Px(20.0),
                ..default()
            },
            ImageNode::from(pass_button_tex),
            Button,
            Hovered::default(),
            Interaction::default(),
            ButtonType(LoginButtonType::PasswordChange),
            ButtonTextures {
                normal_index: 326,
                hover_index: 327,
                pressed_index: 328,
            },
            Name::new("PassButton"),
        ));
        
        // View Key Button
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(60.0),
                top: Val::Px(189.0),
                width: Val::Px(80.0),
                height: Val::Px(20.0),
                ..default()
            },
            ImageNode::from(view_key_button_tex),
            Button,
            Hovered::default(),
            Interaction::default(),
            ButtonType(LoginButtonType::ViewKey),
            ButtonTextures {
                normal_index: 332,
                hover_index: 333,
                pressed_index: 334,
            },
            Name::new("ViewKeyButton"),
        ));
        
        // Close Button
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(166.0),
                top: Val::Px(189.0),
                width: Val::Px(80.0),
                height: Val::Px(20.0),
                ..default()
            },
            ImageNode::from(close_button_tex),
            Button,
            Hovered::default(),
            Interaction::default(),
            ButtonType(LoginButtonType::Close),
            ButtonTextures {
                normal_index: 329,
                hover_index: 330,
                pressed_index: 331,
            },
            Name::new("CloseButton"),
        ));
    });
}

// ============================================================================
// UI Helper Functions
// ============================================================================

fn spawn_version_label(
    commands: &mut Commands,
    parent: Entity,
    asset_server: &Res<AssetServer>,
) {
    let font = asset_server.load("fonts/NotoSansSC-Regular.ttf");
    
    let version_text = format!("Build: Crystal v0.1.0 (Bevy {})", env!("CARGO_PKG_VERSION"));
    
    let version_label = commands.spawn((
        Text::new(version_text),
        TextFont {
            font,
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(5.0),
            bottom: Val::Px(5.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        Name::new("VersionLabel"),
    )).id();
    
    commands.entity(parent).add_child(version_label);
}

// ============================================================================
// Input Handling Systems
// ============================================================================

/// Handle text input for focused text boxes
pub fn handle_text_input(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut login_state: ResMut<LoginState>,
    account_query: Query<Entity, (With<AccountIdInput>, With<InputFocused>)>,
    password_query: Query<Entity, (With<PasswordInput>, With<InputFocused>)>,
) {
    let has_account_focus = !account_query.is_empty();
    let has_password_focus = !password_query.is_empty();
    
    if !has_account_focus && !has_password_focus {
        return; // No input has focus
    }
    
    for event in keyboard_events.read() {
        info!("⌨️ Keyboard event: {:?}", event);
        
        // Only process when key is first pressed
        if !event.state.is_pressed() {
            continue;
        }
        
        match &event.logical_key {
            // Handle Tab key to switch focus
            Key::Tab => {
                if has_account_focus {
                    // Switch to password input
                    info!("⇥ Tab pressed: switching to password");
                    // The focus switching will be handled by a separate system
                } else if has_password_focus {
                    // Switch to account input
                    info!("⇥ Tab pressed: switching to account");
                }
                // Don't add Tab character to input
                continue;
            }
            // Handle backspace
            Key::Backspace => {
                if has_account_focus && !login_state.account_id.is_empty() {
                    login_state.account_id.pop();
                    validate_account_id(&mut login_state);
                } else if has_password_focus && !login_state.password.is_empty() {
                    login_state.password.pop();
                    validate_password(&mut login_state);
                }
            }
            // Handle character input
            Key::Character(text) => {
                // Filter to only printable ASCII characters
                for ch in text.chars() {
                    if !is_printable_char(ch) {
                        continue;
                    }
                    
                    if has_account_focus {
                        if login_state.account_id.len() < MAX_ACCOUNT_ID_LENGTH {
                            login_state.account_id.push(ch);
                            validate_account_id(&mut login_state);
                        }
                    } else if has_password_focus {
                        if login_state.password.len() < MAX_PASSWORD_LENGTH {
                            login_state.password.push(ch);
                            validate_password(&mut login_state);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// Helper function to check if character is printable
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}

/// Update text display for inputs
pub fn update_input_display(
    login_state: Res<LoginState>,
    mut account_query: Query<&Children, With<AccountIdInput>>,
    mut password_query: Query<&Children, (With<PasswordInput>, Without<AccountIdInput>)>,
    mut text_query: Query<(&mut Text, &mut TextColor), Without<InputCursor>>,
) {
    // Update account ID display
    for children in account_query.iter_mut() {
        // First child is the text node (not the cursor)
        if let Some(child) = children.first() {
            if let Ok((mut text, mut color)) = text_query.get_mut(*child) {
                if login_state.account_id.is_empty() {
                    text.0 = "账号".to_string();
                    color.0 = Color::srgba(0.5, 0.5, 0.5, 1.0); // 灰色占位符
                } else {
                    text.0 = login_state.account_id.clone();
                    color.0 = Color::srgba(1.0, 1.0, 1.0, 1.0); // 白色文本
                }
                info!("📝 Account text updated: '{}'", text.0);
            }
        }
    }
    
    // Update password display (masked)
    for children in password_query.iter_mut() {
        if let Some(child) = children.first() {
            if let Ok((mut text, mut color)) = text_query.get_mut(*child) {
                if login_state.password.is_empty() {
                    text.0 = "密码".to_string();
                    color.0 = Color::srgba(0.5, 0.5, 0.5, 1.0); // 灰色占位符
                } else {
                    text.0 = "*".repeat(login_state.password.len());
                    color.0 = Color::srgba(1.0, 1.0, 1.0, 1.0); // 白色文本
                }
                info!("📝 Password text updated: '{}'", text.0);
            }
        }
    }
}

/// Handle input focus on click
pub fn handle_input_focus(
    mut commands: Commands,
    mut interaction_query: Query<
        (Entity, &Interaction, &Children, Option<&AccountIdInput>, Option<&PasswordInput>),
        (Changed<Interaction>, With<Button>),
    >,
    focused_query: Query<(Entity, &Children), With<InputFocused>>,
    mut cursor_query: Query<&mut Visibility, With<InputCursor>>,
) {
    for (entity, interaction, children, is_account, is_password) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed && (is_account.is_some() || is_password.is_some()) {
            // Remove focus from all inputs and hide their cursors
            for (focused_entity, focused_children) in focused_query.iter() {
                commands.entity(focused_entity).remove::<InputFocused>();
                
                // Hide cursor of unfocused input
                for child in focused_children.iter() {
                    if let Ok(mut visibility) = cursor_query.get_mut(child) {
                        *visibility = Visibility::Hidden;
                    }
                }
            }
            
            // Add focus to clicked input
            commands.entity(entity).insert(InputFocused);
            
            // Show cursor for focused input
            for child in children.iter() {
                if let Ok(mut visibility) = cursor_query.get_mut(child) {
                    *visibility = Visibility::Inherited;
                }
            }
            
            let input_type = if is_account.is_some() { "账号" } else { "密码" };
            info!("✅ Input focus changed to {}", input_type);
        }
    }
}

/// Handle Tab key to switch focus between inputs
pub fn handle_tab_focus(
    mut commands: Commands,
    mut keyboard_events: EventReader<KeyboardInput>,
    account_query: Query<(Entity, &Children), (With<AccountIdInput>, With<InputFocused>)>,
    password_query: Query<(Entity, &Children), (With<PasswordInput>, With<InputFocused>)>,
    account_entity_query: Query<(Entity, &Children), (With<AccountIdInput>, Without<InputFocused>)>,
    password_entity_query: Query<(Entity, &Children), (With<PasswordInput>, Without<InputFocused>)>,
    mut cursor_query: Query<&mut Visibility, With<InputCursor>>,
) {
    for event in keyboard_events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        
        if let Key::Tab = event.logical_key {
            // Check which input has focus
            if let Ok((account_entity, account_children)) = account_query.single() {
                // Account has focus, switch to password
                commands.entity(account_entity).remove::<InputFocused>();
                
                // Hide account cursor
                for child in account_children.iter() {
                    if let Ok(mut visibility) = cursor_query.get_mut(child) {
                        *visibility = Visibility::Hidden;
                    }
                }
                
                // Focus password
                if let Ok((password_entity, password_children)) = password_entity_query.single() {
                    commands.entity(password_entity).insert(InputFocused);
                    
                    // Show password cursor
                    for child in password_children.iter() {
                        if let Ok(mut visibility) = cursor_query.get_mut(child) {
                            *visibility = Visibility::Inherited;
                        }
                    }
                    
                    info!("⇥ Tab: Account → Password");
                }
            } else if let Ok((password_entity, password_children)) = password_query.single() {
                // Password has focus, switch to account
                commands.entity(password_entity).remove::<InputFocused>();
                
                // Hide password cursor
                for child in password_children.iter() {
                    if let Ok(mut visibility) = cursor_query.get_mut(child) {
                        *visibility = Visibility::Hidden;
                    }
                }
                
                // Focus account
                if let Ok((account_entity, account_children)) = account_entity_query.single() {
                    commands.entity(account_entity).insert(InputFocused);
                    
                    // Show account cursor
                    for child in account_children.iter() {
                        if let Ok(mut visibility) = cursor_query.get_mut(child) {
                            *visibility = Visibility::Inherited;
                        }
                    }
                    
                    info!("⇥ Tab: Password → Account");
                }
            }
        }
    }
}

/// Update cursor blink animation
pub fn update_cursor_blink(
    time: Res<Time>,
    input_query: Query<&Children, With<InputFocused>>,
    mut cursor_query: Query<(&mut InputCursor, &mut Visibility)>,
) {
    for children in input_query.iter() {
        for child in children.iter() {
            if let Ok((mut cursor, mut visibility)) = cursor_query.get_mut(child) {
                cursor.blink_timer.tick(time.delta());
                
                if cursor.blink_timer.just_finished() {
                    cursor.visible = !cursor.visible;
                    *visibility = if cursor.visible {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
    }
}

/// Update input border colors based on validation
pub fn update_input_borders(
    login_state: Res<LoginState>,
    mut account_query: Query<&mut BorderColor, With<AccountIdInput>>,
    mut password_query: Query<&mut BorderColor, (With<PasswordInput>, Without<AccountIdInput>)>,
) {
    // Update account input border
    for mut border in account_query.iter_mut() {
        let new_color = if login_state.account_id.is_empty() {
            INPUT_BORDER_NORMAL
        } else if login_state.account_id_valid {
            INPUT_BORDER_VALID
        } else {
            INPUT_BORDER_INVALID
        };
        *border = BorderColor::all(new_color);
    }
    
    // Update password input border
    for mut border in password_query.iter_mut() {
        let new_color = if login_state.password.is_empty() {
            INPUT_BORDER_NORMAL
        } else if login_state.password_valid {
            INPUT_BORDER_VALID
        } else {
            INPUT_BORDER_INVALID
        };
        *border = BorderColor::all(new_color);
    }
}

// ============================================================================
// Button Interaction Systems
// ============================================================================

/// Handle button hover effects
pub fn handle_button_hover(
    mut query: Query<
        (&Hovered, &ButtonTextures, &mut ImageNode),
        (Changed<Hovered>, With<ButtonType>),
    >,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    for (hovered, textures, mut image) in query.iter_mut() {
        if hovered.0 {
            // Load and apply hover texture
            if let Some(hover_tex) = mlibrary_assets.get_texture("Title", textures.hover_index, &mut images) {
                image.image = hover_tex.clone();
                info!("🖱️ Button hover: ON (index {})", textures.hover_index);
            }
        } else {
            // Load and apply normal texture
            if let Some(normal_tex) = mlibrary_assets.get_texture("Title", textures.normal_index, &mut images) {
                image.image = normal_tex.clone();
                info!("🖱️ Button hover: OFF (index {})", textures.normal_index);
            }
        }
    }
}

/// Handle button pressed state by changing textures
pub fn handle_button_press(
    mut query: Query<
        (&Interaction, &ButtonTextures, &Hovered, &mut ImageNode),
        (Changed<Interaction>, With<ButtonType>),
    >,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    for (interaction, textures, hovered, mut image) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                // Load and apply pressed texture
                if let Some(pressed_tex) = mlibrary_assets.get_texture("Title", textures.pressed_index, &mut images) {
                    image.image = pressed_tex.clone();
                    info!("🖱️ Button pressed (index {})", textures.pressed_index);
                }
            }
            Interaction::Hovered => {
                // Load and apply hover texture
                if let Some(hover_tex) = mlibrary_assets.get_texture("Title", textures.hover_index, &mut images) {
                    image.image = hover_tex.clone();
                }
            }
            Interaction::None => {
                // Restore to hover or normal based on hover state
                let index = if hovered.0 { textures.hover_index } else { textures.normal_index };
                if let Some(tex) = mlibrary_assets.get_texture("Title", index, &mut images) {
                    image.image = tex.clone();
                }
            }
        }
    }
}

/// Handle button clicks and send messages
pub fn handle_button_clicks(
    mut query: Query<(&Interaction, &ButtonType), Changed<Interaction>>,
    login_state: Res<LoginState>,
    login_events: Option<MessageWriter<LoginButtonPressed>>,
    account_events: Option<MessageWriter<NewAccountButtonPressed>>,
    password_events: Option<MessageWriter<PasswordChangeButtonPressed>>,
    view_key_events: Option<MessageWriter<ViewKeyButtonPressed>>,
    close_events: Option<MessageWriter<CloseButtonPressed>>,
) {
    // Early return if any message writer is not available
    let Some(mut login_events) = login_events else { return; };
    let Some(mut account_events) = account_events else { return; };
    let Some(mut password_events) = password_events else { return; };
    let Some(mut view_key_events) = view_key_events else { return; };
    let Some(mut close_events) = close_events else { return; };
    
    for (interaction, button_type) in query.iter_mut() {
        if *interaction == Interaction::Pressed {
            match button_type.0 {
                LoginButtonType::Login => {
                    if login_state.account_id_valid && login_state.password_valid {
                        info!("🔐 Login button clicked: {}", login_state.account_id);
                        login_events.write(LoginButtonPressed {
                            account_id: login_state.account_id.clone(),
                            password: login_state.password.clone(),
                        });
                    } else {
                        warn!("❌ Login validation failed");
                    }
                }
                LoginButtonType::NewAccount => {
                    info!("📝 New Account button clicked");
                    account_events.write_default();
                }
                LoginButtonType::PasswordChange => {
                    info!("🔑 Password Change button clicked");
                    password_events.write_default();
                }
                LoginButtonType::ViewKey => {
                    info!("👁️ View Key button clicked");
                    view_key_events.write_default();
                }
                LoginButtonType::Close => {
                    info!("❌ Close button clicked");
                    close_events.write_default();
                }
                // Dialog buttons handled in handle_dialog_buttons
                LoginButtonType::DialogOK | LoginButtonType::DialogCancel => {}
            }
        }
    }
}

// ============================================================================
// Message Handlers
// ============================================================================

/// Handle login button message - Send login request to server
pub fn handle_login_message(
    events: Option<MessageReader<LoginButtonPressed>>,
    mut login_state: ResMut<LoginState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("🚀 Processing login request: account={}", event.account_id);
        
        // Mark as connecting
        login_state.connecting = true;
        login_state.connect_attempts += 1;
        
        // Send login request through network command channel
        if let Some(tx) = &login_state.command_tx {
            info!("📤 Sending login request to server: {}", event.account_id);
            info!("🔐 Account: {} (password length: {})", event.account_id, event.password.len());
            
            // Create and send login command
            let command = crate::network::NetworkCommand::Login {
                username: event.account_id.clone(),
                password: event.password.clone(),
            };
            
            match tx.send(command) {
                Ok(_) => {
                    info!("✅ Login command sent to network thread successfully");
                }
                Err(e) => {
                    error!("❌ Failed to send login command: {}", e);
                    login_state.connecting = false;
                    login_state.connect_attempts -= 1;
                }
            }
        } else {
            warn!("⚠️  Network command channel not initialized");
            // Fallback: auto-approve for testing when network is not set up
            info!("📤 [TESTING] Auto-approving login without network: {}", event.account_id);
            login_state.login_success = true;
            login_state.frames_after_login = 0;
            login_state.animation_paused = false;
            login_state.connecting = false;
            info!("✅ [TESTING] Login success! Starting animation, will transition after {} frames", ANIMATION_FRAME_COUNT);
        }
    }
}

/// Handle close button message
pub fn handle_close_message(
    events: Option<MessageReader<CloseButtonPressed>>,
    mut app_exit: Option<MessageWriter<bevy::app::AppExit>>,
) {
    let Some(mut events) = events else { return; };
    let Some(mut app_exit) = app_exit else { return; };
    
    for _ in events.read() {
        info!("👋 Exiting application");
        app_exit.write(bevy::app::AppExit::Success);
    }
}

/// Handle new account button message
pub fn handle_new_account_message(
    events: Option<MessageReader<NewAccountButtonPressed>>,
    mut login_state: ResMut<LoginState>,
    mut commands: Commands,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    root_query: Query<Entity, With<LoginSceneRoot>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("📝 Opening New Account dialog");
        login_state.dialog_visible = DialogType::NewAccount;
        
        // Spawn new account dialog
        if let Ok(root) = root_query.single() {
            spawn_new_account_dialog(&mut commands, &mut mlibrary_assets, &mut images, &asset_server, root);
        }
    }
}

/// Handle password change message
pub fn handle_password_change_message(
    events: Option<MessageReader<PasswordChangeButtonPressed>>,
    mut login_state: ResMut<LoginState>,
    mut commands: Commands,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    root_query: Query<Entity, With<LoginSceneRoot>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🔑 Opening Password Change dialog");
        login_state.dialog_visible = DialogType::ChangePassword;
        
        // Spawn change password dialog
        if let Ok(root) = root_query.single() {
            spawn_change_password_dialog(&mut commands, &mut mlibrary_assets, &mut images, &asset_server, root);
        }
    }
}

/// Handle view key message
pub fn handle_view_key_message(
    events: Option<MessageReader<ViewKeyButtonPressed>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("👁️ Opening View Key dialog");
        // TODO: Show view key dialog
    }
}

// ============================================================================
// Cleanup
// ============================================================================

/// Cleanup LoginScene
pub fn cleanup_login_scene(
    mut commands: Commands,
    query: Query<Entity, With<LoginSceneRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    
    commands.remove_resource::<LoginState>();
    
    info!("🧹 LoginScene cleaned up");
}

// ============================================================================
// Validation Helper Functions
// ============================================================================

fn validate_account_id(login_state: &mut LoginState) {
    let re = Regex::new(&format!(
        r"^[A-Za-z0-9]{{{},{}}}$",
        MIN_ACCOUNT_ID_LENGTH,
        MAX_ACCOUNT_ID_LENGTH
    )).unwrap();
    
    login_state.account_id_valid = re.is_match(&login_state.account_id);
    update_login_enabled(login_state);
}

fn validate_password(login_state: &mut LoginState) {
    let re = Regex::new(&format!(
        r"^[A-Za-z0-9]{{{},{}}}$",
        MIN_PASSWORD_LENGTH,
        MAX_PASSWORD_LENGTH
    )).unwrap();
    
    login_state.password_valid = re.is_match(&login_state.password);
    update_login_enabled(login_state);
}

fn update_login_enabled(login_state: &mut LoginState) {
    login_state.login_enabled = login_state.account_id_valid && login_state.password_valid;
}

// ============================================================================
// Dialog Creation Functions
// ============================================================================

/// Spawn new account dialog
fn spawn_new_account_dialog(
    commands: &mut Commands,
    mlibrary_assets: &mut ResMut<crate::bevy::MLibraryAssets>,
    images: &mut ResMut<Assets<Image>>,
    _asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    info!("📝 Creating New Account Dialog");
    
    // Load dialog background texture (index 63 from Prguse)
    let dialog_bg = mlibrary_assets.get_texture("Prguse", 63, images);
    
    if dialog_bg.is_none() {
        warn!("❌ Failed to load new account dialog background texture");
        return;
    }
    
    let dialog_bg = dialog_bg.unwrap();
    
    // Load button textures from Title library
    // New Account Dialog: OK: 200/201/202, Cancel: 203/204/205 (from C# original)
    let ok_button_tex = mlibrary_assets.get_texture("Title", 200, images)
        .expect("Failed to load OK button texture");
    let cancel_button_tex = mlibrary_assets.get_texture("Title", 203, images)
        .expect("Failed to load Cancel button texture");
    
    // Get texture size from image
    let bg_image = images.get(&dialog_bg).unwrap();
    let dialog_width = bg_image.width() as f32;
    let dialog_height = bg_image.height() as f32;
    
    info!("📐 Dialog size: {}x{}", dialog_width, dialog_height);
    
    // Calculate dialog position (centered on screen - 1024x768 like C# original)
    let dialog_x = (1024.0 - dialog_width) / 2.0;
    let dialog_y = (768.0 - dialog_height) / 2.0;
    
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(dialog_x),
                top: Val::Px(dialog_y),
                width: Val::Px(dialog_width),
                height: Val::Px(dialog_height),
                ..default()
            },
            ZIndex(100), // On top of everything
            ImageNode::from(dialog_bg.clone()),
            NewAccountDialog,
            Dialog,
            Name::new("NewAccountDialog"),
        )).with_children(|dialog| {
            // OK Button - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(135.0),
                    top: Val::Px(425.0),
                    width: Val::Px(80.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::from(ok_button_tex.clone()),
                Button,
                Hovered::default(),
                Interaction::default(),
                ButtonType(LoginButtonType::DialogOK),
                ButtonTextures {
                    normal_index: 200,
                    hover_index: 201,
                    pressed_index: 202,
                },
                Name::new("DialogOKButton"),
            ));
            
            // Cancel Button - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(409.0),
                    top: Val::Px(425.0),
                    width: Val::Px(80.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::from(cancel_button_tex.clone()),
                Button,
                Hovered::default(),
                Interaction::default(),
                ButtonType(LoginButtonType::DialogCancel),
                ButtonTextures {
                    normal_index: 203,
                    hover_index: 204,
                    pressed_index: 205,
                },
                Name::new("DialogCancelButton"),
            ));
            
            // Add input fields for account creation
            // AccountID Input (226, 103, 136, 18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(103.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_FOCUSED),
                DialogInputField {
                    field_type: DialogFieldType::NewAccountId,
                },
                InputFocused, // Set initial focus here
                Button,
                Interaction::default(),
                Name::new("NewAccountIdInput"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                ));
            });
            
            // Password1 Input (226, 129, 136, 18)
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(129.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewPassword1,
                },
                Button,
                Interaction::default(),
                Name::new("NewPassword1Input"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Password2 Input (226, 155, 136, 18)
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(155.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewPassword2,
                },
                Button,
                Interaction::default(),
                Name::new("NewPassword2Input"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Email Input (226, 311, 136, 18) - optional field
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(311.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewEmail,
                },
                Button,
                Interaction::default(),
                Name::new("NewEmailInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // UserName Input (226, 189, 136, 18) - optional field
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(189.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewUserName,
                },
                Button,
                Interaction::default(),
                Name::new("NewUserNameInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // BirthDate Input (226, 215, 136, 18) - optional field
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(215.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewBirthDate,
                },
                Button,
                Interaction::default(),
                Name::new("NewBirthDateInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Question Input (226, 250, 190, 18) - optional field, wider
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(250.0),
                    width: Val::Px(190.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewQuestion,
                },
                Button,
                Interaction::default(),
                Name::new("NewQuestionInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
            
            // Answer Input (226, 276, 190, 18) - optional field, wider
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(226.0),
                    top: Val::Px(276.0),
                    width: Val::Px(190.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::NewAnswer,
                },
                Button,
                Interaction::default(),
                Name::new("NewAnswerInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: false,
                    },
                    Visibility::Hidden,
                ));
            });
        });
    });
    
    info!("✅ New Account Dialog created with 8 input fields");
}

/// Spawn change password dialog
fn spawn_change_password_dialog(
    commands: &mut Commands,
    mlibrary_assets: &mut ResMut<crate::bevy::MLibraryAssets>,
    images: &mut ResMut<Assets<Image>>,
    _asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    info!("🔑 Creating Change Password Dialog");
    
    // Load dialog background texture (index 50 from Prguse - C# original)
    let dialog_bg = mlibrary_assets.get_texture("Prguse", 50, images);
    
    if dialog_bg.is_none() {
        warn!("❌ Failed to load change password dialog background texture");
        return;
    }
    
    let dialog_bg = dialog_bg.unwrap();
    
    // Get texture size from image
    let bg_image = images.get(&dialog_bg).unwrap();
    let dialog_width = bg_image.width() as f32;
    let dialog_height = bg_image.height() as f32;
    
    info!("📐 Dialog size: {}x{}", dialog_width, dialog_height);
    
    // Load button textures from Title library (C# original)
    // OK: 107/108/109, Cancel: 110/111/112
    let ok_button_tex = mlibrary_assets.get_texture("Title", 107, images)
        .expect("Failed to load OK button texture");
    let cancel_button_tex = mlibrary_assets.get_texture("Title", 110, images)
        .expect("Failed to load Cancel button texture");
    
    // Calculate dialog position (centered)
    let dialog_x = (1024.0 - dialog_width) / 2.0;
    let dialog_y = (768.0 - dialog_height) / 2.0;
    
    // Load font for input fields
    let font = _asset_server.load("fonts/NotoSansSC-Regular.ttf");
    
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(dialog_x),
                top: Val::Px(dialog_y),
                width: Val::Px(dialog_width),
                height: Val::Px(dialog_height),
                ..default()
            },
            ZIndex(100), // On top of everything
            ImageNode::from(dialog_bg.clone()),
            ChangePasswordDialog,
            Dialog,
            Name::new("ChangePasswordDialog"),
        )).with_children(|dialog| {
            // OK Button (80, 236) - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(80.0),
                    top: Val::Px(236.0),
                    width: Val::Px(80.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::from(ok_button_tex.clone()),
                Button,
                Hovered::default(),
                Interaction::default(),
                ButtonType(LoginButtonType::DialogOK),
                ButtonTextures {
                    normal_index: 107,
                    hover_index: 108,
                    pressed_index: 109,
                },
                Name::new("DialogOKButton"),
            ));
            
            // Cancel Button (222, 236) - with button texture and hover effect
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(222.0),
                    top: Val::Px(236.0),
                    width: Val::Px(80.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                ImageNode::from(cancel_button_tex.clone()),
                Button,
                Hovered::default(),
                Interaction::default(),
                ButtonType(LoginButtonType::DialogCancel),
                ButtonTextures {
                    normal_index: 110,
                    hover_index: 111,
                    pressed_index: 112,
                },
                Name::new("DialogCancelButton"),
            ));
            
            // AccountID Input (178, 75, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(75.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_FOCUSED),
                DialogInputField {
                    field_type: DialogFieldType::ChangeAccountId,
                },
                InputFocused, // Set initial focus
                Button,
                Interaction::default(),
                Name::new("ChangeAccountIdInput"),
            )).with_children(|input_parent| {
                // Add text child
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                // Add cursor child
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Inherited,
                    Name::new("InputCursor"),
                ));
            });
            
            // CurrentPassword Input (178, 113, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(113.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::ChangeCurrentPassword,
                },
                Button,
                Interaction::default(),
                Name::new("ChangeCurrentPasswordInput"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Hidden,
                    Name::new("InputCursor"),
                ));
            });
            
            // NewPassword1 Input (178, 151, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(151.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::ChangeNewPassword1,
                },
                Button,
                Interaction::default(),
                Name::new("ChangeNewPassword1Input"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Hidden,
                    Name::new("InputCursor"),
                ));
            });
            
            // NewPassword2 Input (178, 188, 136x18) from C# original
            dialog.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(178.0),
                    top: Val::Px(188.0),
                    width: Val::Px(136.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                BorderColor::all(INPUT_BORDER_NORMAL),
                DialogInputField {
                    field_type: DialogFieldType::ChangeNewPassword2,
                },
                Button,
                Interaction::default(),
                Name::new("ChangeNewPassword2Input"),
            )).with_children(|input_parent| {
                input_parent.spawn((
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Name::new("InputText"),
                ));
                
                input_parent.spawn((
                    Text::new("|"),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    InputCursor {
                        blink_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
                        visible: true,
                    },
                    Visibility::Hidden,
                    Name::new("InputCursor"),
                ));
            });
        });
    });
    
    info!("✅ Change Password Dialog created with 4 input fields");
}

/// Handle dialog button clicks
pub fn handle_dialog_buttons(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &ButtonType),
        (Changed<Interaction>, With<Button>),
    >,
    dialog_query: Query<Entity, With<Dialog>>,
    mut login_state: ResMut<LoginState>,
) {
    for (interaction, button_type) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            match button_type.0 {
                LoginButtonType::DialogOK => {
                    info!("✅ Dialog OK clicked");
                    // TODO: Validate and submit form
                    // For now, just close dialog
                    close_all_dialogs(&mut commands, &dialog_query, &mut login_state);
                }
                LoginButtonType::DialogCancel => {
                    info!("❌ Dialog Cancel clicked");
                    close_all_dialogs(&mut commands, &dialog_query, &mut login_state);
                }
                _ => {}
            }
        }
    }
}

/// Close all dialogs
fn close_all_dialogs(
    commands: &mut Commands,
    dialog_query: &Query<Entity, With<Dialog>>,
    login_state: &mut ResMut<LoginState>,
) {
    for entity in dialog_query.iter() {
        // Despawn the dialog and its children
        commands.entity(entity).despawn();
    }
    login_state.dialog_visible = DialogType::None;
    login_state.dialog_inputs.clear();
    info!("🚪 All dialogs closed");
}

// ============================================================================
// Dialog Input Handling
// ============================================================================

/// Handle text input for dialog fields
pub fn handle_dialog_text_input(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut login_state: ResMut<LoginState>,
    focused_query: Query<&DialogInputField, With<InputFocused>>,
) {
    // Get focused dialog field
    let Some(focused_field) = focused_query.iter().next() else {
        return; // No dialog field has focus
    };
    
    for event in keyboard_events.read() {
        // Only process when key is first pressed
        if !event.state.is_pressed() {
            continue;
        }
        
        match &event.logical_key {
            // Handle Tab key (will be handled by focus switching system)
            Key::Tab => {
                info!("⇥ Tab pressed in dialog");
                continue;
            }
            // Handle backspace
            Key::Backspace => {
                if let Some(text) = login_state.dialog_inputs.get_mut(&focused_field.field_type) {
                    if !text.is_empty() {
                        text.pop();
                        info!("⌫ Backspace in dialog field {:?}: '{}'", focused_field.field_type, text);
                    }
                }
            }
            // Handle character input
            Key::Character(input_text) => {
                for ch in input_text.chars() {
                    if !is_printable_char(ch) {
                        continue;
                    }
                    
                    // Get or create text for this field
                    let text = login_state.dialog_inputs
                        .entry(focused_field.field_type)
                        .or_insert_with(String::new);
                    
                    // Check max length based on field type
                    let max_len = match focused_field.field_type {
                        DialogFieldType::NewAccountId | DialogFieldType::ChangeAccountId => MAX_ACCOUNT_ID_LENGTH,
                        DialogFieldType::NewPassword1 | DialogFieldType::NewPassword2 |
                        DialogFieldType::ChangeCurrentPassword | DialogFieldType::ChangeNewPassword1 | 
                        DialogFieldType::ChangeNewPassword2 => MAX_PASSWORD_LENGTH,
                        DialogFieldType::NewEmail => 50,
                        DialogFieldType::NewUserName => 20,
                        DialogFieldType::NewBirthDate => 10,
                        DialogFieldType::NewQuestion | DialogFieldType::NewAnswer => 30,
                    };
                    
                    if text.len() < max_len {
                        text.push(ch);
                        info!("📝 Dialog input {:?}: '{}'", focused_field.field_type, text);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Update dialog input field display
pub fn update_dialog_input_display(
    login_state: Res<LoginState>,
    field_query: Query<(&DialogInputField, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    for (field, children) in field_query.iter() {
        if let Some(input) = login_state.dialog_inputs.get(&field.field_type) {
            // Show asterisks for password fields
            let display_text = match field.field_type {
                DialogFieldType::NewPassword1 | DialogFieldType::NewPassword2 |
                DialogFieldType::ChangeCurrentPassword | DialogFieldType::ChangeNewPassword1 |
                DialogFieldType::ChangeNewPassword2 => "*".repeat(input.len()),
                _ => input.clone(),
            };
            
            // Update the first text child
            if let Some(&child) = children.first() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    **text = display_text;
                }
            }
        } else {
            // Clear text if no input
            if let Some(&child) = children.first() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    **text = String::new();
                }
            }
        }
    }
}

/// Handle Tab key to switch focus between dialog fields
pub fn handle_dialog_tab_focus(
    mut commands: Commands,
    mut keyboard_events: EventReader<KeyboardInput>,
    dialog_query: Query<Entity, With<Dialog>>,
    focused_query: Query<(Entity, &DialogInputField), With<InputFocused>>,
    all_fields_query: Query<(Entity, &DialogInputField), Without<InputFocused>>,
) {
    if dialog_query.is_empty() {
        return; // No dialog open
    }
    
    // Check if Tab was pressed
    let tab_pressed = keyboard_events.read()
        .any(|e| e.state.is_pressed() && matches!(e.logical_key, Key::Tab));
    
    if !tab_pressed {
        return;
    }
    
    // Get currently focused field
    let Some((focused_entity, focused_field)) = focused_query.iter().next() else {
        return;
    };
    
    // Determine next field in sequence
    let next_field_type = match focused_field.field_type {
        // New Account sequence
        DialogFieldType::NewAccountId => DialogFieldType::NewPassword1,
        DialogFieldType::NewPassword1 => DialogFieldType::NewPassword2,
        DialogFieldType::NewPassword2 => DialogFieldType::NewEmail,
        DialogFieldType::NewEmail => DialogFieldType::NewUserName,
        DialogFieldType::NewUserName => DialogFieldType::NewBirthDate,
        DialogFieldType::NewBirthDate => DialogFieldType::NewQuestion,
        DialogFieldType::NewQuestion => DialogFieldType::NewAnswer,
        DialogFieldType::NewAnswer => DialogFieldType::NewAccountId, // Loop back
        
        // Change Password sequence
        DialogFieldType::ChangeAccountId => DialogFieldType::ChangeCurrentPassword,
        DialogFieldType::ChangeCurrentPassword => DialogFieldType::ChangeNewPassword1,
        DialogFieldType::ChangeNewPassword1 => DialogFieldType::ChangeNewPassword2,
        DialogFieldType::ChangeNewPassword2 => DialogFieldType::ChangeAccountId, // Loop back
    };
    
    // Find next field entity
    if let Some((next_entity, _)) = all_fields_query.iter()
        .find(|(_, field)| field.field_type == next_field_type) {
        
        // Remove focus from current
        commands.entity(focused_entity).remove::<InputFocused>();
        
        // Add focus to next
        commands.entity(next_entity).insert(InputFocused);
        
        info!("⇥ Dialog focus switched: {:?} -> {:?}", focused_field.field_type, next_field_type);
    }
}

/// Handle clicking on dialog input fields to focus them
pub fn handle_dialog_input_click(
    mut commands: Commands,
    dialog_input_query: Query<(Entity, &Interaction, &DialogInputField), Changed<Interaction>>,
    focused_query: Query<Entity, With<InputFocused>>,
) {
    for (entity, interaction, field) in dialog_input_query.iter() {
        if *interaction == Interaction::Pressed {
            // Remove focus from all previously focused inputs
            for focused_entity in focused_query.iter() {
                commands.entity(focused_entity).remove::<InputFocused>();
            }
            
            // Add focus to clicked input
            commands.entity(entity).insert(InputFocused);
            
            info!("🖱️ Dialog input clicked and focused: {:?}", field.field_type);
        }
    }
}

/// Update border colors for dialog inputs based on focus state
pub fn update_dialog_input_borders(
    mut dialog_input_query: Query<(&mut BorderColor, Has<InputFocused>), With<DialogInputField>>,
) {
    for (mut border, is_focused) in dialog_input_query.iter_mut() {
        if is_focused {
            *border = BorderColor::all(INPUT_BORDER_FOCUSED);
        } else {
            *border = BorderColor::all(INPUT_BORDER_NORMAL);
        }
    }
}

/// Update cursor visibility for dialog inputs based on focus state
/// Only runs when focus changes
pub fn update_dialog_cursor_visibility(
    dialog_input_query: Query<(&Children, Has<InputFocused>), (With<DialogInputField>, Changed<InputFocused>)>,
    mut cursor_query: Query<(&mut InputCursor, &mut Visibility)>,
) {
    for (children, is_focused) in dialog_input_query.iter() {
        for child in children.iter() {
            if let Ok((mut cursor, mut visibility)) = cursor_query.get_mut(child) {
                if is_focused {
                    // Reset cursor to visible and restart blink timer when focused
                    cursor.visible = true;
                    cursor.blink_timer.reset();
                    *visibility = Visibility::Inherited;
                } else {
                    // Hide cursor when unfocused
                    cursor.visible = false;
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}
