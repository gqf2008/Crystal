// ============================================================================
// LoginPlugin - 登录界面（对齐 macroquad LoginScene）
// ============================================================================
// 布局：全屏 ChrSel[0] 背景 + 居中 328x220 对话框（Prguse[1084]），
// 元素坐标与原版一致（Title[30] logo、Title[31/32] 标签、输入框、
// Title[320-328] 三帧图按钮）。登录成功后播 ChrSel 0-18 帧动画 → 选角。

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::MessageReader;
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{colors, load_cn_font, ImageButton};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginState>();
        app.init_resource::<LoginAnim>();
        app.add_systems(OnEnter(AppState::Login), setup_login_ui);
        app.add_systems(OnExit(AppState::Login), cleanup_login_ui);
        app.add_systems(
            Update,
            (
                process_text_input,
                login_anim_system,
                crate::ui::theme::image_button_system,
            )
                .run_if(in_state(AppState::Login)),
        );
    }
}

/// 对话框尺寸（原版）
const DIALOG_W: f32 = 328.0;
const DIALOG_H: f32 = 220.0;
/// 原版按钮帧
const OK_FRAMES: [usize; 3] = [320, 321, 322];
const ACCOUNT_FRAMES: [usize; 3] = [323, 324, 325];
const PASS_FRAMES: [usize; 3] = [326, 327, 328];

#[derive(Resource, Default)]
pub struct LoginState {
    pub account: String,
    pub password: String,
}

/// ChrSel 背景动画状态
#[derive(Resource, Default)]
pub struct LoginAnim {
    pub playing: bool,
    pub frame: usize,
    pub timer: f32,
    pub handles: Vec<Handle<Image>>,
}

// UI 组件
#[derive(Component)]
struct LoginRoot;
#[derive(Component)]
struct LoginBg;
#[derive(Component)]
struct AccountInput;
#[derive(Component)]
struct PasswordInput;
#[derive(Component)]
struct LoginOkButton;
#[derive(Component)]
struct StatusText;

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

fn setup_login_ui(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut anim: ResMut<LoginAnim>,
) {
    libs.0.ensure_initialized();
    let font = FontSource::Handle(load_cn_font(&mut fonts));

    // 预加载 ChrSel 背景动画帧（0-18）
    anim.handles.clear();
    for i in 0..19usize {
        if let Some(h) = crate::ui::theme::load_lib_image(
            &mut libs,
            &mut images,
            LibraryName::ChrSel,
            i,
        ) {
            anim.handles.push(h);
        }
    }

    // 对话框图（背景 + 标题 + 标签）
    let bg_dialog = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1084);
    let logo = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 30);
    let label_acc = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 31);
    let label_pwd = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 32);

    let ok_btn = load_button(&mut libs, &mut images, &OK_FRAMES);
    let acc_btn = load_button(&mut libs, &mut images, &ACCOUNT_FRAMES);
    let pwd_btn = load_button(&mut libs, &mut images, &PASS_FRAMES);

    let dialog_x = 476.0; // (1280-328)/2
    let dialog_y = 290.0; // (800-220)/2

    commands
        .spawn((
            LoginRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            // 全屏 ChrSel[0] 背景
            if let Some(bg) = anim.handles.first().cloned() {
                root.spawn((
                    LoginBg,
                    ImageNode { image: bg, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }

            // 对话框
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(dialog_x),
                    top: Val::Px(dialog_y),
                    width: Val::Px(DIALOG_W),
                    height: Val::Px(DIALOG_H),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|d| {
                // 对话框底图 Prguse[1084]
                if let Some(bg) = bg_dialog.clone() {
                    d.spawn((
                        ImageNode { image: bg, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(DIALOG_W),
                            height: Val::Px(DIALOG_H),
                            ..default()
                        },
                    ));
                }
                // 标题 logo Title[30]
                if let Some(lg) = logo.clone() {
                    // 尺寸按图宽，水平居中、顶部 12px
                    d.spawn((
                        ImageNode { image: lg, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(10.0),
                            top: Val::Px(12.0),
                            ..default()
                        },
                    ));
                }
                // 账号标签 Title[31] @ (52,83)
                if let Some(l) = label_acc.clone() {
                    d.spawn((
                        ImageNode { image: l, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(52.0),
                            top: Val::Px(83.0),
                            ..default()
                        },
                    ));
                }
                // 密码标签 Title[32] @ (43,105)
                if let Some(l) = label_pwd.clone() {
                    d.spawn((
                        ImageNode { image: l, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(43.0),
                            top: Val::Px(105.0),
                            ..default()
                        },
                    ));
                }
                // 账号输入框 @ (85,85) 136x15
                spawn_input(d, &font, AccountInput, TextInputNode::new(false), 85.0, 85.0);
                // 密码输入框 @ (85,108)
                spawn_input(d, &font, PasswordInput, TextInputNode::new(true), 85.0, 108.0);
                // OK 按钮 @ (227,81) 42x42
                if let Some(b) = ok_btn.clone() {
                    d.spawn((
                        LoginOkButton,
                        Button,
                        ImageButton {
                            normal: b.0.clone(),
                            hover: b.1.clone(),
                            pressed: b.2.clone(),
                        },
                        ImageNode { image: b.0, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(227.0),
                            top: Val::Px(81.0),
                            width: Val::Px(42.0),
                            height: Val::Px(42.0),
                            ..default()
                        },
                    ));
                }
                // 新建账号按钮 @ (60,163)
                if let Some(b) = acc_btn.clone() {
                    d.spawn((
                        ImageButton {
                            normal: b.0.clone(),
                            hover: b.1.clone(),
                            pressed: b.2.clone(),
                        },
                        ImageNode { image: b.0, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(60.0),
                            top: Val::Px(163.0),
                            width: Val::Px(84.0),
                            height: Val::Px(30.0),
                            ..default()
                        },
                    ));
                }
                // 修改密码按钮 @ (166,163)
                if let Some(b) = pwd_btn.clone() {
                    d.spawn((
                        ImageButton {
                            normal: b.0.clone(),
                            hover: b.1.clone(),
                            pressed: b.2.clone(),
                        },
                        ImageNode { image: b.0, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(166.0),
                            top: Val::Px(163.0),
                            width: Val::Px(84.0),
                            height: Val::Px(30.0),
                            ..default()
                        },
                    ));
                }
                // 状态文字（连接中/错误）
                d.spawn((
                    StatusText,
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(20.0),
                        top: Val::Px(190.0),
                        ..default()
                    },
                ));
            });
        });
}

/// 加载三帧按钮图
fn load_button(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    frames: &[usize; 3],
) -> Option<(Handle<Image>, Handle<Image>, Handle<Image>)> {
    let n = crate::ui::theme::load_lib_image(libs, images, LibraryName::Title, frames[0])?;
    let h = crate::ui::theme::load_lib_image(libs, images, LibraryName::Title, frames[1])?;
    let p = crate::ui::theme::load_lib_image(libs, images, LibraryName::Title, frames[2])?;
    Some((n, h, p))
}

fn spawn_input(
    parent: &mut ChildSpawnerCommands,
    font: &FontSource,
    marker: impl Bundle,
    input: TextInputNode,
    x: f32,
    y: f32,
) {
    parent
        .spawn((
            marker,
            input,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                width: Val::Px(136.0),
                height: Val::Px(15.0),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(3.0)),
                ..default()
            },
            Interaction::default(),
        ))
        .with_children(|input_node| {
            input_node.spawn((
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: FontSize::Px(13.0),
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

/// ChrSel 背景动画：登录成功后播放 0-18 帧，播完进选角
fn login_anim_system(
    mut anim: ResMut<LoginAnim>,
    mut net: ResMut<NetworkContext>,
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time>,
    mut bg: Query<&mut ImageNode, With<LoginBg>>,
) {
    if !anim.playing {
        // 登录成功 → 开始播放
        if net.login_success {
            net.login_success = false;
            anim.playing = true;
            anim.frame = 0;
            anim.timer = 0.0;
        }
        return;
    }
    anim.timer += time.delta_secs();
    let frame_delay = 0.15;
    if anim.timer >= frame_delay {
        anim.timer = 0.0;
        anim.frame += 1;
        if anim.frame >= anim.handles.len() {
            // 播放完成 → 进入选角
            anim.playing = false;
            next.set(AppState::Select);
            return;
        }
        if let Ok(mut node) = bg.single_mut() {
            if let Some(h) = anim.handles.get(anim.frame) {
                node.image = h.clone();
            }
        }
    }
}

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
    ok_btn: Query<&Interaction, (With<LoginOkButton>, Without<TextInputNode>)>,
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
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
    for (_, _, mut input, _, acc, pwd) in inputs.iter_mut() {
        if !input.focused {
            continue;
        }
        for key in &key_list {
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

    // 4) OK 按钮 → 发送登录
    if net.state != crate::network::NetState::LoggingIn
        && ok_btn.iter().any(|i| *i == Interaction::Pressed)
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
                .unwrap_or_default(),
        };
    }
}
