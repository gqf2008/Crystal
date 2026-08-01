// ============================================================================
// LoginPlugin - 登录界面（bevy_ui 原生 UI）
// ============================================================================
// 账号/密码输入 + 登录按钮 → 发 Login 包（mock 服务器回应 LoginSuccess → Select）

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::MessageReader;
use bevy::prelude::*;

use crate::network::NetworkContext;
use crate::scenes::AppState;
use crate::ui::theme::{colors, spawn_text_button, CN_FONT};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginState>();
        app.add_systems(OnEnter(AppState::Login), setup_login_ui);
        app.add_systems(OnExit(AppState::Login), cleanup_login_ui);
        app.add_systems(
            Update,
            process_text_input.run_if(in_state(AppState::Login)),
        );
    }
}

#[derive(Resource, Default)]
pub struct LoginState {
    pub account: String,
    pub password: String,
}

// UI 组件标记
#[derive(Component)]
struct LoginRoot;
#[derive(Component)]
struct AccountInput;
#[derive(Component)]
struct PasswordInput;
#[derive(Component)]
struct LoginButton;
#[derive(Component)]
struct StatusText;

/// 文本输入框状态
#[derive(Component)]
pub struct TextInputNode {
    pub value: String,
    pub focused: bool,
    pub password: bool,
}

impl TextInputNode {
    pub fn new(password: bool) -> Self {
        Self {
            value: String::new(),
            focused: false,
            password,
        }
    }
}

fn setup_login_ui(mut commands: Commands, assets: Res<AssetServer>) {
    let font = FontSource::Handle(assets.load(CN_FONT));
    commands
        .spawn((
            LoginRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.06, 0.09)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("传 奇 2"),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(52.0),
                    ..default()
                },
                TextColor(colors::TITLE_GOLD),
            ));
            root.spawn((
                Text::new("Legend of Mir 2 · Bevy 移植版"),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(colors::GRAY),
            ));
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(colors::PANEL_BG),
            ))
            .with_children(|card| {
                spawn_label(card, &font, "账号");
                spawn_input(card, &font, AccountInput, TextInputNode::new(false));
                spawn_label(card, &font, "密码");
                spawn_input(card, &font, PasswordInput, TextInputNode::new(true));
                // 状态提示（连接中/错误）
                card.spawn((
                    StatusText,
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(colors::GRAY),
                ));
                spawn_text_button(card, &font, "登  录", 18.0, LoginButton);
            });
        });
}

fn spawn_label(parent: &mut ChildSpawnerCommands, font: &FontSource, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(colors::TEXT),
    ));
}

fn spawn_input(
    parent: &mut ChildSpawnerCommands,
    font: &FontSource,
    marker: impl Bundle,
    input: TextInputNode,
) {
    parent
        .spawn((
            marker,
            input,
            Node {
                width: Val::Px(240.0),
                height: Val::Px(34.0),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(colors::INPUT_BG),
            Interaction::default(),
        ))
        .with_children(|input_node| {
            input_node.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(colors::TEXT),
            ));
        });
}

fn cleanup_login_ui(mut commands: Commands, root: Query<Entity, With<LoginRoot>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

/// 处理文本输入焦点与按键，以及登录按钮
#[allow(clippy::type_complexity)]
fn process_text_input(
    mut keys: MessageReader<KeyboardInput>,
    mut login: ResMut<LoginState>,
    mut net: ResMut<NetworkContext>,
    mut inputs: Query<(
        Entity,
        &Interaction,
        &mut TextInputNode,
        &Children,
        Option<&AccountInput>,
        Option<&PasswordInput>,
    )>,
    mut texts: Query<&mut Text, Without<StatusText>>,
    buttons: Query<(&Interaction, &LoginButton)>,
    mut status: Query<&mut Text, (With<StatusText>, Without<TextInputNode>)>,
) {
    // 1) 点击聚焦
    let clicked: Option<Entity> = inputs
        .iter_mut()
        .find(|(_, i, _, _, _, _)| **i == Interaction::Pressed)
        .map(|(e, ..)| e);
    for (e, _, mut input, _, _, _) in inputs.iter_mut() {
        input.focused = Some(e) == clicked;
    }

    // 2) 按键输入到聚焦框
    let keys: Vec<KeyboardInput> = keys.read().cloned().collect();
    for (_, _, mut input, _, acc, pwd) in inputs.iter_mut() {
        if !input.focused {
            continue;
        }
        for key in &keys {
            if key.state != bevy::input::ButtonState::Pressed {
                continue;
            }
            if key.logical_key == Key::Backspace {
                input.value.pop();
            } else if let Some(text) = &key.text {
                if !text.is_empty() {
                    input.value.push_str(text);
                }
            }
        }
        if acc.is_some() {
            login.account = input.value.clone();
        }
        if pwd.is_some() {
            login.password = input.value.clone();
        }
    }

    // 3) 更新输入框显示文本（密码打码）
    for (_, _, input, children, _, _) in inputs.iter_mut() {
        let display = if input.password {
            "*".repeat(input.value.len())
        } else {
            input.value.clone()
        };
        for child in children.iter() {
            if let Ok(mut t) = texts.get_mut(child) {
                t.0 = display.clone();
            }
        }
    }

    // 4) 登录按钮（防重：连接中不重复发送）
    if net.state != crate::network::NetState::LoggingIn
        && buttons.iter().any(|(i, _)| *i == Interaction::Pressed)
    {
        net.state = crate::network::NetState::LoggingIn;
        net.send_packet(&mir2_shared::packets::client::account::Login {
            account_id: login.account.clone(),
            password: login.password.clone(),
        });
    }

    // 5) 状态显示
    if let Ok(mut t) = status.single_mut() {
        t.0 = match net.state {
            crate::network::NetState::LoggingIn => "连接中…".to_string(),
            _ => net
                .login_error
                .clone()
                .unwrap_or_else(|| "输入账号密码后点击登录".to_string()),
        };
    }
}
