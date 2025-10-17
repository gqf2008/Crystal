// LoginScene for Bevy - Login screen implementation
// Mirrors Client/MirScenes/LoginScene.cs

use bevy::prelude::*;
use bevy::ecs::message::{MessageReader, MessageWriter};

/// LoginScene state resource
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
    pub animation_paused: bool,
    
    /// Button hover states
    pub ok_button_hovered: bool,
    pub account_button_hovered: bool,
    pub pass_button_hovered: bool,
    pub close_button_hovered: bool,
    pub view_key_button_hovered: bool,
    
    /// Status tracking
    pub last_status: Option<String>,
    pub message_log: Vec<String>,
    
    /// Login results
    pub last_login_result: Option<u8>,
    pub last_new_account_result: Option<u8>,
    pub last_change_password_result: Option<u8>,
    
    /// Ban information
    pub login_ban_info: Option<BanInfo>,
    pub password_change_ban_info: Option<BanInfo>,
}

#[derive(Debug, Clone)]
pub struct BanInfo {
    pub reason: String,
    pub expiry_date: i64,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
            version_checked: false,
            version_valid: false,
            login_enabled: false,
            require_password_change: false,
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: false,
            ok_button_hovered: false,
            account_button_hovered: false,
            pass_button_hovered: false,
            close_button_hovered: false,
            view_key_button_hovered: false,
            last_status: None,
            message_log: Vec::new(),
            last_login_result: None,
            last_new_account_result: None,
            last_change_password_result: None,
            login_ban_info: None,
            password_change_ban_info: None,
        }
    }
}

/// UI component markers
#[derive(Component)]
pub struct LoginSceneRoot;

#[derive(Component)]
pub struct LoginBackground;

#[derive(Component)]
pub struct LoginDialog;

#[derive(Component)]
pub struct AccountIdInput;

#[derive(Component)]
pub struct PasswordInput;

#[derive(Component)]
pub struct OkButton;

#[derive(Component)]
pub struct AccountButton;

#[derive(Component)]
pub struct PasswordChangeButton;

#[derive(Component)]
pub struct ViewKeyButton;

#[derive(Component)]
pub struct CloseButton;

#[derive(Component)]
pub struct VersionLabel;

#[derive(Component)]
pub struct StatusLabel;

/// Events
#[derive(Message)]
pub struct LoginButtonPressed {
    pub account_id: String,
    pub password: String,
}

#[derive(Message)]
pub struct NewAccountButtonPressed;

#[derive(Message)]
pub struct PasswordChangeButtonPressed;

#[derive(Message)]
pub struct ViewKeyButtonPressed;

#[derive(Message)]
pub struct CloseButtonPressed;

/// Animation constants
const ANIMATION_FRAME_COUNT: usize = 19;
const ANIMATION_DELAY: f32 = 0.1; // 100ms

/// Dialog dimensions (from C# original)
const DIALOG_WIDTH: f32 = 328.0;
const DIALOG_HEIGHT: f32 = 220.0;

/// Input validation constants
const MIN_ACCOUNT_ID_LENGTH: usize = 3;
const MAX_ACCOUNT_ID_LENGTH: usize = 15;
const MIN_PASSWORD_LENGTH: usize = 5;
const MAX_PASSWORD_LENGTH: usize = 15;

/// Setup LoginScene
pub fn setup_login_scene(
    mut commands: Commands,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    info!("🎮 Setting up LoginScene");
    
    // Insert LoginState resource with version already validated for testing
    let mut login_state = LoginState::default();
    login_state.version_checked = true;
    login_state.version_valid = true; // 暂时跳过版本检查,直接显示界面
    commands.insert_resource(login_state);
    
    info!("✅ LoginState 已创建");
    
    // Create root entity
    let root = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        LoginSceneRoot,
        Name::new("LoginSceneRoot"),
    )).id();
    
    info!("✅ Root 实体已创建");
    
    // Create background
    spawn_background(&mut commands, root, &mut mlibrary_assets, &mut images);
    info!("✅ 背景已创建");
    
    // Create login dialog
    spawn_login_dialog(&mut commands, root, &mut mlibrary_assets, &mut images);
    info!("✅ 登录对话框已创建");
    
    // Create version label
    spawn_version_label(&mut commands, root);
    info!("✅ 版本标签已创建");
    
    info!("🎉 LoginScene 设置完成!");
}

/// Spawn background animation
fn spawn_background(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
) {
    // Load background image from ChrSel library (index 0)
    info!("🔄 开始加载背景纹理 (ChrSel:0)...");
    
    let background_texture = match mlibrary_assets.get_texture("ChrSel", 0, images) {
        Some(texture) => {
            info!("✅ 背景纹理加载成功");
            texture.clone()
        }
        None => {
            error!("❌ 背景纹理加载失败!");
            return; // 如果加载失败,直接返回
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
        LoginBackground,
        Name::new("LoginBackground"),
    )).id();
    
    commands.entity(parent).add_child(background);
}

/// Spawn login dialog UI
fn spawn_login_dialog(
    commands: &mut Commands,
    parent: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
) {
    // Load dialog background from Prguse library (index 1084)
    info!("🔄 开始加载对话框纹理 (Prguse:1084)...");
    
    let dialog_texture = match mlibrary_assets.get_texture("Prguse", 1084, images) {
        Some(texture) => {
            info!("✅ 对话框纹理加载成功");
            texture.clone()
        }
        None => {
            error!("❌ 对话框纹理加载失败!");
            return;
        }
    };
    
    // Create dialog entity
    let dialog = commands.spawn((
        ImageNode {
            image: dialog_texture,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(DIALOG_WIDTH),
            height: Val::Px(DIALOG_HEIGHT),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        LoginDialog,
        Visibility::Visible, // 直接显示,不等待版本检查
        Name::new("LoginDialog"),
    )).id();
    
    // Set dialog as child of parent
    commands.entity(parent).add_child(dialog);
    
    // Create dialog contents
    spawn_dialog_contents(commands, dialog, mlibrary_assets, images);
}

/// Spawn dialog contents (inputs, buttons, labels)
fn spawn_dialog_contents(
    commands: &mut Commands,
    dialog: Entity,
    mlibrary_assets: &mut crate::bevy::MLibraryAssets,
    images: &mut Assets<Image>,
) {
    // Pre-load all textures to avoid borrow issues
    let title_texture = mlibrary_assets.get_texture("Title", 30, images)
        .expect("Failed to load title texture").clone();
    let account_label_texture = mlibrary_assets.get_texture("Title", 31, images)
        .expect("Failed to load account label").clone();
    let pass_label_texture = mlibrary_assets.get_texture("Title", 32, images)
        .expect("Failed to load password label").clone();
    let ok_button_texture = mlibrary_assets.get_texture("Title", 320, images)
        .expect("Failed to load OK button").clone();
    let account_button_texture = mlibrary_assets.get_texture("Title", 323, images)
        .expect("Failed to load account button").clone();
    let pass_button_texture = mlibrary_assets.get_texture("Title", 326, images)
        .expect("Failed to load password button").clone();
    let view_key_button_texture = mlibrary_assets.get_texture("Title", 332, images)
        .expect("Failed to load view key button").clone();
    let close_button_texture = mlibrary_assets.get_texture("Title", 329, images)
        .expect("Failed to load close button").clone();
    
    commands.entity(dialog).with_children(|parent| {
        // Title label
        parent.spawn((
            ImageNode {
                image: title_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                ..default()
            },
            Name::new("TitleLabel"),
        ));
        
        // Account ID label
        parent.spawn((
            ImageNode {
                image: account_label_texture,
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
                image: pass_label_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(43.0),
                top: Val::Px(105.0),
                ..default()
            },
            Name::new("PassLabel"),
        ));
        
        // Account ID input
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(85.0),
                top: Val::Px(85.0),
                width: Val::Px(136.0),
                height: Val::Px(15.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            AccountIdInput,
            Name::new("AccountIDInput"),
        ));
        
        // Password input
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(85.0),
                top: Val::Px(108.0),
                width: Val::Px(136.0),
                height: Val::Px(15.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            PasswordInput,
            Name::new("PasswordInput"),
        ));
        
        // OK button
        parent.spawn((
            Button,
            ImageNode {
                image: ok_button_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(227.0),
                top: Val::Px(81.0),
                width: Val::Px(42.0),
                height: Val::Px(42.0),
                ..default()
            },
            OkButton,
            Name::new("OKButton"),
        ));
        
        // Account button
        parent.spawn((
            Button,
            ImageNode {
                image: account_button_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(60.0),
                top: Val::Px(163.0),
                ..default()
            },
            AccountButton,
            Name::new("AccountButton"),
        ));
        
        // Password change button
        parent.spawn((
            Button,
            ImageNode {
                image: pass_button_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(166.0),
                top: Val::Px(163.0),
                ..default()
            },
            PasswordChangeButton,
            Name::new("PasswordChangeButton"),
        ));
        
        // View key button
        parent.spawn((
            Button,
            ImageNode {
                image: view_key_button_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(60.0),
                top: Val::Px(189.0),
                ..default()
            },
            ViewKeyButton,
            Name::new("ViewKeyButton"),
        ));
        
        // Close button
        parent.spawn((
            Button,
            ImageNode {
                image: close_button_texture,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(166.0),
                top: Val::Px(189.0),
                ..default()
            },
            CloseButton,
            Name::new("CloseButton"),
        ));
    });
}

/// Spawn version label
fn spawn_version_label(commands: &mut Commands, parent: Entity) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new(format!("Build: Bevy {}", env!("CARGO_PKG_VERSION"))),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(5.0),
                bottom: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
            VersionLabel,
            Name::new("VersionLabel"),
        ));
    });
}

/// Update background animation
pub fn update_background_animation(
    time: Res<Time>,
    mut login_state: ResMut<LoginState>,
    mut query: Query<&mut ImageNode, With<LoginBackground>>,
    mut mlibrary_assets: ResMut<crate::bevy::MLibraryAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if login_state.animation_paused {
        return;
    }
    
    login_state.animation_timer += time.delta_secs();
    
    if login_state.animation_timer >= ANIMATION_DELAY {
        login_state.animation_timer = 0.0;
        
        // Update frame
        if login_state.background_frame < ANIMATION_FRAME_COUNT - 1 {
            login_state.background_frame += 1;
            
            // Update background texture
            for mut image in query.iter_mut() {
                if let Some(texture) = mlibrary_assets.get_texture("ChrSel", login_state.background_frame as i32, &mut images) {
                    image.image = texture;
                }
            }
        } else {
            // Animation complete, pause
            login_state.animation_paused = true;
        }
    }
}

/// Handle button interactions
pub fn handle_button_interactions(
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor, Option<&OkButton>, Option<&AccountButton>, 
                                   Option<&PasswordChangeButton>, Option<&ViewKeyButton>, Option<&CloseButton>), 
                                   (Changed<Interaction>, With<Button>)>,
    login_events: Option<MessageWriter<LoginButtonPressed>>,
    account_events: Option<MessageWriter<NewAccountButtonPressed>>,
    password_events: Option<MessageWriter<PasswordChangeButtonPressed>>,
    view_key_events: Option<MessageWriter<ViewKeyButtonPressed>>,
    close_events: Option<MessageWriter<CloseButtonPressed>>,
) {
    let Some(mut login_events) = login_events else { return; };
    let Some(mut account_events) = account_events else { return; };
    let Some(mut password_events) = password_events else { return; };
    let Some(mut view_key_events) = view_key_events else { return; };
    let Some(mut close_events) = close_events else { return; };
    
    for (interaction, mut color, ok_btn, account_btn, pass_btn, view_key_btn, close_btn) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgba(0.8, 0.8, 0.8, 1.0));
                
                // Trigger appropriate event
                if ok_btn.is_some() {
                    login_events.write(LoginButtonPressed {
                        account_id: String::new(), // TODO: Get from input
                        password: String::new(),
                    });
                } else if account_btn.is_some() {
                    account_events.write(NewAccountButtonPressed);
                } else if pass_btn.is_some() {
                    password_events.write(PasswordChangeButtonPressed);
                } else if view_key_btn.is_some() {
                    view_key_events.write(ViewKeyButtonPressed);
                } else if close_btn.is_some() {
                    close_events.write(CloseButtonPressed);
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgba(0.9, 0.9, 0.9, 1.0));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 1.0));
            }
        }
    }
}

/// Handle login button press
pub fn handle_login_button(
    events: Option<MessageReader<LoginButtonPressed>>,
    mut login_state: ResMut<LoginState>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("Login button pressed: account={}, password={}", event.account_id, event.password);
        
        // Validate input
        if !validate_account_id(&event.account_id) {
            login_state.last_status = Some("Invalid Account ID".to_string());
            continue;
        }
        
        if !validate_password(&event.password) {
            login_state.last_status = Some("Invalid Password".to_string());
            continue;
        }
        
        // TODO: Send login request to server
        login_state.connecting = true;
        login_state.last_status = Some("Connecting...".to_string());
    }
}

/// Handle close button
pub fn handle_close_button(
    events: Option<MessageReader<CloseButtonPressed>>,
    mut app_exit: Option<MessageWriter<bevy::app::AppExit>>,
) {
    let Some(mut events) = events else { return; };
    let Some(mut app_exit) = app_exit else { return; };
    
    for _ in events.read() {
        info!("Close button pressed, exiting application");
        app_exit.write(bevy::app::AppExit::Success);
    }
}

/// Validate account ID
fn validate_account_id(account_id: &str) -> bool {
    if account_id.len() < MIN_ACCOUNT_ID_LENGTH || account_id.len() > MAX_ACCOUNT_ID_LENGTH {
        return false;
    }
    
    // Only alphanumeric characters
    account_id.chars().all(|c| c.is_alphanumeric())
}

/// Validate password
fn validate_password(password: &str) -> bool {
    if password.len() < MIN_PASSWORD_LENGTH || password.len() > MAX_PASSWORD_LENGTH {
        return false;
    }
    
    true
}

/// Show login dialog after version check
pub fn show_login_dialog_system(
    login_state: Res<LoginState>,
    mut query: Query<&mut Visibility, With<LoginDialog>>,
) {
    // 如果版本有效且登录未启用,显示对话框
    if login_state.version_valid && !login_state.login_enabled {
        for mut visibility in query.iter_mut() {
            if *visibility != Visibility::Visible {
                *visibility = Visibility::Visible;
                info!("✅ 登录对话框已显示");
            }
        }
    }
}

/// Cleanup LoginScene
pub fn cleanup_login_scene(
    mut commands: Commands,
    query: Query<Entity, With<LoginSceneRoot>>,
) {
    for entity in query.iter() {
        // Despawn entity and all its children
        commands.entity(entity).despawn();
    }
    
    commands.remove_resource::<LoginState>();
    
    info!("LoginScene cleaned up");
}
