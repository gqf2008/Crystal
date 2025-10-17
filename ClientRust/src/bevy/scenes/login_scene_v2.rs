// LoginScene for Bevy 0.17.2 - Complete Implementation
// Migrated from Client/MirScenes/LoginScene.cs

use bevy::prelude::*;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::picking::hover::Hovered;
use bevy::ui_widgets::Button;
use regex::Regex;

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
const INPUT_BORDER_VALID: Color = Color::srgba(0.0, 1.0, 0.0, 1.0);
const INPUT_BORDER_INVALID: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);
const TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

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
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
            version_checked: true, // 跳过版本检查用于测试
            version_valid: true,
            login_enabled: false,
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: false,
            account_id: String::new(),
            password: String::new(),
            account_id_valid: false,
            password_valid: false,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginButtonType {
    Login,
    NewAccount,
    PasswordChange,
    ViewKey,
    Close,
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
    info!("🎮 Setting up LoginScene v2");
    
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
    
    info!("🎉 LoginScene v2 设置完成!");
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
pub fn update_background_animation(
    time: Res<Time>,
    mut login_state: ResMut<LoginState>,
    mut query: Query<(&mut ImageNode, &mut LoginBackground)>,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if login_state.animation_paused {
        return;
    }
    
    login_state.animation_timer += time.delta_secs();
    
    if login_state.animation_timer >= ANIMATION_DELAY {
        login_state.animation_timer = 0.0;
        
        // Advance to next frame
        login_state.background_frame = (login_state.background_frame + 1) % ANIMATION_FRAME_COUNT;
        
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
    
    // Load button textures
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
                width: Val::Px(80.0),
                height: Val::Px(30.0),
                ..default()
            },
            ImageNode::from(ok_button_tex),
            Button,
            Hovered::default(),
            BackgroundColor(BUTTON_NORMAL_COLOR),
            Interaction::default(),
            ButtonType(LoginButtonType::Login),
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
            BackgroundColor(BUTTON_NORMAL_COLOR),
            Interaction::default(),
            ButtonType(LoginButtonType::NewAccount),
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
            BackgroundColor(BUTTON_NORMAL_COLOR),
            Interaction::default(),
            ButtonType(LoginButtonType::PasswordChange),
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
            BackgroundColor(BUTTON_NORMAL_COLOR),
            Interaction::default(),
            ButtonType(LoginButtonType::ViewKey),
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
            BackgroundColor(BUTTON_NORMAL_COLOR),
            Interaction::default(),
            ButtonType(LoginButtonType::Close),
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
        (&Hovered, &mut ImageNode),
        (Changed<Hovered>, With<ButtonType>),
    >,
) {
    for (hovered, mut image) in query.iter_mut() {
        if hovered.0 {
            // Brighten on hover
            image.color = Color::srgba(1.2, 1.2, 1.2, 1.0);
            info!("🖱️ Button hover: ON");
        } else {
            // Normal color
            image.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
            info!("🖱️ Button hover: OFF");
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
            }
        }
    }
}

// ============================================================================
// Message Handlers
// ============================================================================

/// Handle login button message
pub fn handle_login_message(
    events: Option<MessageReader<LoginButtonPressed>>,
    mut next_state: ResMut<NextState<crate::bevy::GameState>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("🚀 Processing login: {}", event.account_id);
        // TODO: Send network packet to server
        // For now, just transition to select scene
        next_state.set(crate::bevy::GameState::Select);
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
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("📝 Opening New Account dialog");
        // TODO: Show new account dialog
    }
}

/// Handle password change message
pub fn handle_password_change_message(
    events: Option<MessageReader<PasswordChangeButtonPressed>>,
) {
    let Some(mut events) = events else { return; };
    
    for _ in events.read() {
        info!("🔑 Opening Password Change dialog");
        // TODO: Show password change dialog
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
