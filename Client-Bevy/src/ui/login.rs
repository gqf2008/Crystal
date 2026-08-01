// ============================================================================
// LoginPlugin - 登录界面（对齐 macroquad LoginScene：光标 + 新建账号/改密码对话框）
// ============================================================================

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
        app.init_resource::<CursorBlink>();
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

const DIALOG_W: f32 = 328.0;
const DIALOG_H: f32 = 220.0;
const OK_FRAMES: [usize; 3] = [320, 321, 322];
const ACCOUNT_FRAMES: [usize; 3] = [323, 324, 325];
const PASS_FRAMES: [usize; 3] = [326, 327, 328];
const NA_OK: [usize; 3] = [200, 201, 202];
const NA_CANCEL: [usize; 3] = [203, 204, 205];
const CP_OK: [usize; 3] = [107, 108, 109];
const CP_CANCEL: [usize; 3] = [110, 111, 112];

#[derive(Resource, Default)]
pub struct LoginState {
    pub show_new_account: bool,
    pub show_change_password: bool,
}

#[derive(Resource, Default)]
pub struct LoginAnim {
    pub playing: bool,
    pub frame: usize,
    pub timer: f32,
    pub handles: Vec<Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct CursorBlink {
    pub timer: f32,
    pub visible: bool,
}

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
struct AccountButton;
#[derive(Component)]
struct PassButton;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct NaOkButton;
#[derive(Component)]
struct NaCancelButton;
#[derive(Component)]
struct CpOkButton;
#[derive(Component)]
struct CpCancelButton;
#[derive(Component)]
struct NaField(u8);
#[derive(Component)]
struct CpField(u8);
#[derive(Component)]
struct DialogRoot {
    kind: DialogKind,
}
#[derive(Clone, Copy, PartialEq)]
enum DialogKind {
    NewAccount,
    ChangePassword,
}

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

type Btn3 = (Handle<Image>, Handle<Image>, Handle<Image>);

fn setup_login_ui(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut anim: ResMut<LoginAnim>,
) {
    libs.0.ensure_initialized();
    let font = FontSource::Handle(load_cn_font(&mut fonts));

    anim.handles.clear();
    for i in 0..19usize {
        if let Some(h) =
            crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::ChrSel, i)
        {
            anim.handles.push(h);
        }
    }

    let bg_dialog = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1084);
    let logo = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 30);
    let label_acc = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 31);
    let label_pwd = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Title, 32);
    let ok_btn = load_button(&mut libs, &mut images, &OK_FRAMES);
    let acc_btn = load_button(&mut libs, &mut images, &ACCOUNT_FRAMES);
    let pwd_btn = load_button(&mut libs, &mut images, &PASS_FRAMES);
    let na_bg = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 63);
    let na_ok = load_button(&mut libs, &mut images, &NA_OK);
    let na_cancel = load_button(&mut libs, &mut images, &NA_CANCEL);
    let cp_bg = crate::ui::theme::load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 50);
    let cp_ok = load_button(&mut libs, &mut images, &CP_OK);
    let cp_cancel = load_button(&mut libs, &mut images, &CP_CANCEL);

    commands
        .spawn((
            LoginRoot,
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            if let Some(bg) = anim.handles.first().cloned() {
                root.spawn((
                    LoginBg,
                    ImageNode { image: bg, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0), top: Val::Px(0.0),
                        width: Val::Percent(100.0), height: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }

            // 登录对话框
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(476.0), top: Val::Px(290.0),
                    width: Val::Px(DIALOG_W), height: Val::Px(DIALOG_H),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|d| {
                if let Some(bg) = bg_dialog.clone() {
                    d.spawn((
                        ImageNode { image: bg, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0), top: Val::Px(0.0),
                            width: Val::Px(DIALOG_W), height: Val::Px(DIALOG_H),
                            ..default()
                        },
                    ));
                }
                if let Some(lg) = logo.clone() {
                    d.spawn((
                        ImageNode { image: lg, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(10.0), top: Val::Px(12.0),
                            ..default()
                        },
                    ));
                }
                if let Some(l) = label_acc.clone() {
                    d.spawn((
                        ImageNode { image: l, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(52.0), top: Val::Px(83.0),
                            ..default()
                        },
                    ));
                }
                if let Some(l) = label_pwd.clone() {
                    d.spawn((
                        ImageNode { image: l, ..default() },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(43.0), top: Val::Px(105.0),
                            ..default()
                        },
                    ));
                }
                spawn_input(d, &font, AccountInput, TextInputNode { focused: true, ..TextInputNode::new(false) }, 85.0, 85.0);
                spawn_input(d, &font, PasswordInput, TextInputNode::new(true), 85.0, 108.0);
                if let Some(b) = ok_btn.clone() { spawn_img_button(d, b, LoginOkButton, 227.0, 81.0); }
                if let Some(b) = acc_btn.clone() { spawn_img_button(d, b, AccountButton, 60.0, 163.0); }
                if let Some(b) = pwd_btn.clone() { spawn_img_button(d, b, PassButton, 166.0, 163.0); }
                d.spawn((
                    StatusText,
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: FontSize::Px(11.0), ..default() },
                    TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(20.0), top: Val::Px(190.0),
                        ..default()
                    },
                ));
            });

            // 新建账号对话框
            spawn_dialog(root, &font, DialogKind::NewAccount, na_bg, (600.0, 470.0), na_ok, na_cancel);
            // 修改密码对话框
            spawn_dialog(root, &font, DialogKind::ChangePassword, cp_bg, (400.0, 280.0), cp_ok, cp_cancel);
        });
}

fn spawn_dialog(
    root: &mut ChildSpawnerCommands,
    font: &FontSource,
    kind: DialogKind,
    bg: Option<Handle<Image>>,
    size: (f32, f32),
    ok: Option<Btn3>,
    cancel: Option<Btn3>,
) {
    let (w, h) = size;
    let dx = (1280.0 - w) / 2.0;
    let dy = (800.0 - h) / 2.0;
    root.spawn((
        DialogRoot { kind },
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(dx), top: Val::Px(dy),
            width: Val::Px(w), height: Val::Px(h),
            ..default()
        },
    ))
    .with_children(|d| {
        if let Some(bg) = bg {
            d.spawn((
                ImageNode { image: bg, ..default() },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0), top: Val::Px(0.0),
                    width: Val::Px(w), height: Val::Px(h),
                    ..default()
                },
            ));
        }
        match kind {
            DialogKind::NewAccount => {
                let ys = [103.0f32, 129.0, 155.0, 189.0, 215.0, 250.0, 276.0, 311.0];
                let pws = [false, true, true, false, false, false, false, false];
                for i in 0..8 {
                    spawn_input(d, font, NaField(i as u8), TextInputNode::new(pws[i]), 226.0, ys[i]);
                }
                if let Some(b) = ok { spawn_img_button(d, b, NaOkButton, 135.0, 425.0); }
                if let Some(b) = cancel { spawn_img_button(d, b, NaCancelButton, 409.0, 425.0); }
            }
            DialogKind::ChangePassword => {
                let ys = [75.0f32, 113.0, 151.0, 188.0];
                let pws = [false, true, true, true];
                for i in 0..4 {
                    spawn_input(d, font, CpField(i as u8), TextInputNode::new(pws[i]), 178.0, ys[i]);
                }
                if let Some(b) = ok { spawn_img_button(d, b, CpOkButton, 80.0, 236.0); }
                if let Some(b) = cancel { spawn_img_button(d, b, CpCancelButton, 222.0, 236.0); }
            }
        }
    });
}

fn spawn_img_button<M: Bundle>(
    parent: &mut ChildSpawnerCommands,
    b: Btn3,
    marker: M,
    x: f32,
    y: f32,
) {
    parent.spawn((
        marker,
        Button,
        ImageButton { normal: b.0.clone(), hover: b.1.clone(), pressed: b.2.clone() },
        ImageNode { image: b.0, ..default() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x), top: Val::Px(y),
            width: Val::Px(42.0), height: Val::Px(42.0),
            ..default()
        },
    ));
}

fn load_button(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    frames: &[usize; 3],
) -> Option<Btn3> {
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
                left: Val::Px(x), top: Val::Px(y),
                width: Val::Px(136.0), height: Val::Px(18.0),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(3.0)),
                ..default()
            },
            Interaction::default(),
        ))
        .with_children(|input_node| {
            input_node.spawn((
                Text::new(""),
                TextFont { font: font.clone(), font_size: FontSize::Px(13.0), ..default() },
                TextColor(colors::TEXT),
            ));
        });
}

fn cleanup_login_ui(mut commands: Commands, root: Query<Entity, With<LoginRoot>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

fn login_anim_system(
    mut anim: ResMut<LoginAnim>,
    mut net: ResMut<NetworkContext>,
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time>,
    mut bg: Query<&mut ImageNode, With<LoginBg>>,
) {
    if !anim.playing {
        if net.login_success {
            net.login_success = false;
            anim.playing = true;
            anim.frame = 0;
            anim.timer = 0.0;
        }
        return;
    }
    anim.timer += time.delta_secs();
    if anim.timer >= 0.15 {
        anim.timer = 0.0;
        anim.frame += 1;
        if anim.frame >= anim.handles.len() {
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
    mut net: ResMut<NetworkContext>,
    mut login: ResMut<LoginState>,
    mut cursor: ResMut<CursorBlink>,
    time: Res<Time>,
    mut inputs: Query<(
        Entity,
        &Interaction,
        &mut TextInputNode,
        &Children,
        Option<&AccountInput>,
        Option<&PasswordInput>,
        Option<&NaField>,
        Option<&CpField>,
    )>,
    mut texts: Query<&mut Text, Without<StatusText>>,
    ok_btn: Query<&Interaction, (With<LoginOkButton>, Without<TextInputNode>)>,
    acc_btn: Query<&Interaction, (With<AccountButton>, Without<TextInputNode>)>,
    pwd_btn: Query<&Interaction, (With<PassButton>, Without<TextInputNode>)>,
    na_ok: Query<&Interaction, (With<NaOkButton>, Without<TextInputNode>)>,
    na_cancel: Query<&Interaction, (With<NaCancelButton>, Without<TextInputNode>)>,
    cp_ok: Query<&Interaction, (With<CpOkButton>, Without<TextInputNode>)>,
    cp_cancel: Query<&Interaction, (With<CpCancelButton>, Without<TextInputNode>)>,
    mut dialogs: Query<(&DialogRoot, &mut Visibility)>,
    mut status: Query<&mut Text, (With<StatusText>, Without<TextInputNode>)>,
) {
    cursor.timer += time.delta_secs();
    if cursor.timer >= 0.5 {
        cursor.timer = 0.0;
        cursor.visible = !cursor.visible;
    }

    // 点击聚焦
    let clicked: Option<Entity> = inputs
        .iter_mut()
        .find(|(_, i, _, _, _, _, _, _)| **i == Interaction::Pressed)
        .map(|(e, ..)| e);
    for (e, _, mut input, _, _, _, _, _) in inputs.iter_mut() {
        input.focused = Some(e) == clicked;
    }

    // 键盘输入
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
    for (_, _, mut input, _, _, _, _, _) in inputs.iter_mut() {
        if !input.focused { continue; }
        for key in &key_list {
            if key.state != bevy::input::ButtonState::Pressed { continue; }
            if key.logical_key == Key::Backspace {
                input.value.pop();
            } else if let Some(text) = &key.text {
                if !text.is_empty() { input.value.push_str(text); }
            }
        }
    }

    // 显示文本（密码打码 + 聚焦光标）
    for (_, _, input, children, _, _, _, _) in inputs.iter_mut() {
        let mut display = if input.password {
            "*".repeat(input.value.len())
        } else {
            input.value.clone()
        };
        if input.focused && cursor.visible {
            display.push('|');
        }
        for child in children.iter() {
            if let Ok(mut t) = texts.get_mut(child) {
                t.0 = display.clone();
            }
        }
    }

    // 登录
    if net.state != crate::network::NetState::LoggingIn
        && ok_btn.iter().any(|i| *i == Interaction::Pressed)
    {
        let mut account = String::new();
        let mut password = String::new();
        for (_, _, input, _, a, p, _, _) in inputs.iter() {
            if a.is_some() {
                account = input.value.clone();
            }
            if p.is_some() {
                password = input.value.clone();
            }
        }
        net.state = crate::network::NetState::LoggingIn;
        net.send_packet(&mir2_shared::packets::client::account::Login {
            account_id: account,
            password,
        });
    }

    // 打开对话框
    if acc_btn.iter().any(|i| *i == Interaction::Pressed) {
        login.show_new_account = true;
    }
    if pwd_btn.iter().any(|i| *i == Interaction::Pressed) {
        login.show_change_password = true;
    }

    // 对话框 OK/Cancel
    if na_ok.iter().any(|i| *i == Interaction::Pressed) {
        let mut account = String::new();
        let mut p1 = String::new();
        for (_, _, input, _, _, _, na, _) in inputs.iter() {
            if let Some(f) = na {
                if f.0 == 0 {
                    account = input.value.clone();
                }
                if f.0 == 1 {
                    p1 = input.value.clone();
                }
            }
        }
        net.send_packet(&mir2_shared::packets::client::account::NewAccount {
            account_id: account,
            password: p1,
            ..Default::default()
        });
        login.show_new_account = false;
    }
    if na_cancel.iter().any(|i| *i == Interaction::Pressed) {
        login.show_new_account = false;
    }
    if cp_ok.iter().any(|i| *i == Interaction::Pressed) {
        let mut account = String::new();
        let mut cur = String::new();
        let mut new1 = String::new();
        for (_, _, input, _, _, _, _, cp) in inputs.iter() {
            if let Some(f) = cp {
                match f.0 {
                    0 => account = input.value.clone(),
                    1 => cur = input.value.clone(),
                    2 => new1 = input.value.clone(),
                    _ => {}
                }
            }
        }
        net.send_packet(&mir2_shared::packets::client::account::ChangePassword {
            account_id: account,
            current_password: cur,
            new_password: new1,
        });
        login.show_change_password = false;
    }
    if cp_cancel.iter().any(|i| *i == Interaction::Pressed) {
        login.show_change_password = false;
    }

    // 对话框显隐
    for (root, mut vis) in dialogs.iter_mut() {
        let show = match root.kind {
            DialogKind::NewAccount => login.show_new_account,
            DialogKind::ChangePassword => login.show_change_password,
        };
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // 状态文字
    if let Ok(mut t) = status.single_mut() {
        t.0 = match net.state {
            crate::network::NetState::LoggingIn => "连接中…".to_string(),
            _ => net.login_error.clone().unwrap_or_default(),
        };
    }
}
