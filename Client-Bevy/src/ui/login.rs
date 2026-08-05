// ============================================================================
// LoginPlugin - 登录界面（Sprite 精确坐标版，对齐 macroquad LoginScene）
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::MessageReader;
use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_camera, spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image,
    UiButton, UiEntity, UiFont, UiImageCache,
};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginState>();
        app.init_resource::<AuthFeedback>();
        app.init_resource::<LoginAnim>();
        app.init_resource::<CursorBlink>();
        app.init_resource::<UiImageCache>();
        app.init_resource::<UiFont>();
        app.init_resource::<LoginValidation>();
        app.add_systems(OnEnter(AppState::Login), setup_login_ui);
        app.add_systems(OnEnter(AppState::Login), spawn_ui_camera);
        app.add_systems(OnExit(AppState::Login), cleanup_login_ui);
        app.add_systems(
            Update,
            (
                login_input_validation_system,
                login_ui_system,
                login_status_system,
                login_anim_system,
                ui_button_system,
            )
                .chain()
                .run_if(in_state(AppState::Login)),
        );
    }
}

#[derive(Resource, Default)]
pub struct LoginState {
    pub show_new_account: bool,
    pub show_change_password: bool,
    /// 屏幕提示（登录错误/断线/注册结果），is_error=true 时红色显示
    pub status_msg: String,
    pub status_error: bool,
}

/// 认证反馈（#66：登录/注册/改密的 UI 提示字段，从 NetConnection 移回登录界面模块）
#[derive(Resource, Default)]
pub struct AuthFeedback {
    /// 登录错误信息（登录失败 / 连接失败 / 断线）
    pub login_error: Option<String>,
    /// 登录成功标志（LoginScene 播放 ChrSel 动画后进选角）
    pub login_success: bool,
    /// 注册新账号错误信息
    pub new_account_error: Option<String>,
    /// 注册新账号成功（UI 关闭对话框并提示）
    pub new_account_success: bool,
    /// 修改密码错误信息
    pub change_password_error: Option<String>,
    /// 修改密码成功
    pub change_password_success: bool,
}

/// 登录界面底部状态文本标记
#[derive(Component)]
struct LoginStatusText;

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

pub const DX: f32 = 348.0;
pub const DY: f32 = 274.0;
/// 新建账号对话框原点（Prguse[63] 588×460 居中）
pub const NA_X: f32 = (1024.0 - 588.0) / 2.0;
pub const NA_Y: f32 = (768.0 - 460.0) / 2.0;
/// 修改密码对话框原点（Prguse[50] 348×268 居中）
pub const CP_X: f32 = (1024.0 - 348.0) / 2.0;
pub const CP_Y: f32 = (768.0 - 268.0) / 2.0;

/// 账号/密码长度限制（对齐 Shared/Globals.cs：MinAccountID=3/Max=15，MinPassword=5/Max=15）
const MIN_ACC_ID: usize = 3;
const MAX_ACC_ID: usize = 15;
const MIN_PW: usize = 5;
const MAX_PW: usize = 15;
const GREEN: Color = Color::srgb(0.0, 1.0, 0.0);
const RED: Color = Color::srgb(1.0, 0.0, 0.0);
const GRAY: Color = Color::srgb(0.5, 0.5, 0.5);

#[derive(Component)]
pub struct UiInput {
    pub value: String,
    pub focused: bool,
    pub password: bool,
    pub rect: (f32, f32, f32, f32),
    pub text_entity: Entity,
    pub kind: InputKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputKind {
    LoginAccount,
    LoginPassword,
    Na(u8),
    Cp(u8),
}

/// 输入框校验边框（4 条细线之一），关联到 InputKind
#[derive(Component)]
struct InputBorderTag(InputKind);

/// 新建账号对话框"字段说明"文字（随聚焦字段变化，对齐 C# Description (15,340,300x70)）
#[derive(Component)]
struct NaDesc;

/// 各对话框整体校验结果（validation 系统写、ui 系统读以 gate OK/Enter）
#[derive(Resource, Default)]
struct LoginValidation {
    login_ok: bool,
    na_ok: bool,
    cp_ok: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum DialogKind {
    NewAccount,
    ChangePassword,
}

#[derive(Component)]
struct InDialog(DialogKind);

#[derive(Component)]
struct LoginBg;

#[derive(Clone, Copy, PartialEq)]
enum ButtonKind {
    LoginOk,
    NewAccount,
    ChangePassword,
    NaOk,
    NaCancel,
    CpOk,
    CpCancel,
    ViewKey,
    Close,
}

#[derive(Component)]
struct UiButtonKind(ButtonKind);

fn setup_login_ui(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut anim: ResMut<LoginAnim>,
    mut cache: ResMut<UiImageCache>,
    mut ui_font: ResMut<UiFont>,
    mut login: ResMut<LoginState>,
) {
    libs.0.ensure_initialized();
    ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    let font = ui_font.0.clone();
    
    // ChrSel 动画帧
    anim.handles.clear();
    for i in 0..19usize {
        if let Some(info) = libs.0.get_image(LibraryName::ChrSel, i) {
            if let Some(rgba) = info.rgba.clone() {
                let w = info.width.max(0) as u32;
                let h = info.height.max(0) as u32;
                if w > 0 && h > 0 {
                    anim.handles
                        .push(images.add(crate::map_renderer::make_image(rgba, w, h)));
                }
            }
        }
    }

    // 背景
    if let Some(bg) = anim.handles.first().cloned() {
        let e = spawn_ui_sprite(&mut commands, bg, 0.0, 0.0, 0.0, 1.0);
        commands.entity(e).insert(LoginBg);
    }

    // 登录对话框
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        1084,
    ) {
        spawn_ui_sprite(&mut commands, h, DX, DY, 1.0, 1.0);
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 30) {
        spawn_ui_sprite(
            &mut commands,
            h,
            DX + (328.0 - 102.0) / 2.0,
            DY + 12.0,
            1.0,
            1.0,
        );
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 31) {
        spawn_ui_sprite(&mut commands, h, DX + 52.0, DY + 83.0, 1.0, 1.0);
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 32) {
        spawn_ui_sprite(&mut commands, h, DX + 43.0, DY + 105.0, 1.0, 1.0);
    }

    // 输入框（原版 LoginDialog：账号/密码均 136x15）
    spawn_input(
        &mut commands,
        &font,
        InputKind::LoginAccount,
        DX + 85.0,
        DY + 85.0,
        136.0,
        15.0,
        false,
        true,
    );
    spawn_input(
        &mut commands,
        &font,
        InputKind::LoginPassword,
        DX + 85.0,
        DY + 108.0,
        136.0,
        15.0,
        true,
        false,
    );

    // 按钮（三态帧：正常/hover/按下，对齐原版 LoginDialog）
    if let Some(e) = spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        320,
        321,
        322,
        DX + 227.0,
        DY + 81.0,
        2.0,
        48.0,
        48.0,
    ) {
        commands.entity(e).insert((
            UiButtonKind(ButtonKind::LoginOk),
            // #91 悬停音效（C# ButtonA 语义：主要按钮悬停/点击都有反馈）
            crate::ui::sprite_ui::ButtonHoverSound(10104),
        ));
    }
    if let Some(e) = spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        323,
        324,
        325,
        DX + 60.0,
        DY + 163.0,
        2.0,
        100.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert(UiButtonKind(ButtonKind::NewAccount));
    }
    if let Some(e) = spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        326,
        327,
        328,
        DX + 166.0,
        DY + 163.0,
        2.0,
        100.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert(UiButtonKind(ButtonKind::ChangePassword));
    }
    // ViewKeyButton Title[332-334] at (60,189)；CloseButton Title[329-331] at (166,189)
    if let Some(e) = spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        332,
        333,
        334,
        DX + 60.0,
        DY + 189.0,
        2.0,
        100.0,
        25.0,
    ) {
        commands.entity(e).insert(UiButtonKind(ButtonKind::ViewKey));
    }
    if let Some(e) = spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        329,
        330,
        331,
        DX + 166.0,
        DY + 189.0,
        2.0,
        100.0,
        25.0,
    ) {
        commands.entity(e).insert(UiButtonKind(ButtonKind::Close));
    }

    // 左下角服务器/模式状态提示（对齐原版）
    let status = "服务器: mock  模式: Mock";
    spawn_ui_text(
        &mut commands,
        &font,
        status,
        DX + 10.0,
        DY + 210.0,
        12.0,
        Color::srgb(0.75, 0.75, 0.75),
        3.0,
    );

    // 左下角版本号（对齐原版 LoginScene.Version：AutoSize、半透明深色底、黑边、(5,748)）
    let version_text = format!(
        "Build: Crystal.Debug.{}",
        env!("CARGO_PKG_VERSION")
    );
    commands.spawn((
        UiEntity,
        Sprite {
            color: Color::srgba(0.196, 0.196, 0.196, 0.78), // Color.FromArgb(200,50,50,50)
            custom_size: Some(Vec2::new(170.0, 16.0)),
            ..default()
        },
        Transform::from_xyz(5.0 + 85.0, -(748.0 + 8.0), 2.0),
    ));
    spawn_ui_text(
        &mut commands,
        &font,
        &version_text,
        8.0,
        749.0,
        10.0,
        Color::WHITE,
        3.0,
    );

    // 对话框
    spawn_status_text(&mut commands, &font);
    spawn_new_account_dialog(&mut commands, &mut libs, &mut images, &mut cache, &font);
    spawn_change_password_dialog(&mut commands, &mut libs, &mut images, &mut cache, &font);

    // 调试：BEVY_OPEN_DIALOG=na / cp 启动时打开对应弹窗（live 截屏验证用）
    match std::env::var("BEVY_OPEN_DIALOG").as_deref() {
        Ok("na") => login.show_new_account = true,
        Ok("cp") => login.show_change_password = true,
        _ => {}
    }
}

fn ascii_alnum(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric())
}
fn valid_account(s: &str) -> bool {
    (MIN_ACC_ID..=MAX_ACC_ID).contains(&s.len()) && ascii_alnum(s)
}
fn valid_password(s: &str) -> bool {
    (MIN_PW..=MAX_PW).contains(&s.len()) && ascii_alnum(s)
}
/// 简易邮箱（近似 C# regex）：含 @ 且 @ 后含 .
fn valid_email(s: &str) -> bool {
    s.len() <= 50 && s.contains('@') && s.split('@').last().map(|d| d.contains('.')).unwrap_or(false)
}
/// 简易日期：长度<=10 且仅含数字/分隔符（近似 C# DateTime.TryParse）
fn valid_date(s: &str) -> bool {
    s.len() <= 10 && !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '/' || c == '-')
}

/// 新建账号字段合法性 → (valid, required)，字段顺序：账号/密码/确认/用户名/生日/问题/答案/邮箱
fn na_field_valid(k: u8, v: &str, na: &[String; 8]) -> (bool, bool) {
    match k {
        0 => (valid_account(v), true),
        1 => (valid_password(v), true),
        2 => (valid_password(v) && v == na[1].as_str(), true),
        3 => (v.len() <= 20, false),
        4 => (v.is_empty() || valid_date(v), false),
        5 => (v.len() <= 30, false),
        6 => (v.len() <= 30, false),
        7 => (v.is_empty() || valid_email(v), false),
        _ => (true, false),
    }
}
/// 修改密码字段合法性 → (valid, required)，顺序：账号/当前密码/新密码/确认
fn cp_field_valid(k: u8, v: &str, cp: &[String; 4]) -> (bool, bool) {
    match k {
        0 => (valid_account(v), true),
        1 => (valid_password(v), true),
        2 => (valid_password(v), true),
        3 => (valid_password(v) && v == cp[2].as_str(), true),
        _ => (true, false),
    }
}

/// 登录输入框边框色：空=透明（C# Border=!empty），非空非法=红，合法=绿
fn login_border_color(valid: bool, empty: bool) -> Color {
    if empty { Color::NONE } else if valid { GREEN } else { RED }
}
/// 必填/可选字段边框色：必填空=红；可选空=灰；否则绿/红
fn field_border_color(valid: bool, required: bool, empty: bool) -> Color {
    if empty { if required { RED } else { GRAY } } else if valid { GREEN } else { RED }
}

fn na_desc(k: u8) -> &'static str {
    match k {
        0 => "账号：3-15 位字母或数字",
        1 | 2 => "密码：5-15 位字母或数字，两次需一致",
        3 => "用户名：可选，最长 20 字",
        4 => "生日：可选，格式 yyyy/MM/dd",
        5 => "密保问题：可选，最长 30 字",
        6 => "密保答案：可选，最长 30 字",
        7 => "邮箱：可选，最长 50 字",
        _ => "",
    }
}

/// 输入框校验边框（4 条细线），关联 InputKind。z 高于对话框背景与文字。
fn spawn_input_border(commands: &mut Commands, kind: InputKind, x: f32, y: f32, w: f32, h: f32) {
    let z = match kind {
        InputKind::LoginAccount | InputKind::LoginPassword => 3.5,
        _ => 4.6,
    };
    let t = 1.0;
    let line = |c: &mut Commands, cx: f32, cy: f32, sw: f32, sh: f32| {
        c.spawn((
            UiEntity,
            InputBorderTag(kind),
            Sprite {
                color: Color::NONE,
                custom_size: Some(Vec2::new(sw, sh)),
                ..default()
            },
            Transform::from_xyz(cx, -cy, z),
        ));
    };
    line(commands, x + w / 2.0, y - t / 2.0, w + t, t);
    line(commands, x + w / 2.0, y + h + t / 2.0, w + t, t);
    line(commands, x - t / 2.0, y + h / 2.0, t, h + t);
    line(commands, x + w + t / 2.0, y + h / 2.0, t, h + t);
}

/// 实时校验：重算各对话框整体合法性 → 写 LoginValidation；按字段重绘红/绿/灰边框；
/// 更新新建账号字段说明文字。对齐 C# 各 *_TextChanged + GotFocus Description。
fn login_input_validation_system(
    login: Res<LoginState>,
    inputs: Query<&UiInput>,
    mut borders: Query<(&InputBorderTag, &mut Sprite)>,
    mut val: ResMut<LoginValidation>,
    mut desc: Query<&mut Text2d, With<NaDesc>>,
) {
    let mut acc = String::new();
    let mut pw = String::new();
    let mut na: [String; 8] = Default::default();
    let mut cp: [String; 4] = Default::default();
    for i in &inputs {
        match i.kind {
            InputKind::LoginAccount => acc = i.value.clone(),
            InputKind::LoginPassword => pw = i.value.clone(),
            InputKind::Na(k) if (k as usize) < 8 => na[k as usize] = i.value.clone(),
            InputKind::Cp(k) if (k as usize) < 4 => cp[k as usize] = i.value.clone(),
            _ => {}
        }
    }
    val.login_ok = valid_account(&acc) && valid_password(&pw);
    val.na_ok = (0u8..8).all(|k| na_field_valid(k, &na[k as usize], &na).0);
    val.cp_ok = (0u8..4).all(|k| cp_field_valid(k, &cp[k as usize], &cp).0);

    for (tag, mut sprite) in &mut borders {
        let (color, visible) = match tag.0 {
            InputKind::LoginAccount => (login_border_color(valid_account(&acc), acc.is_empty()), true),
            InputKind::LoginPassword => (login_border_color(valid_password(&pw), pw.is_empty()), true),
            InputKind::Na(k) => match na.get(k as usize) {
                Some(v) => {
                    let (valid, req) = na_field_valid(k, v, &na);
                    (field_border_color(valid, req, v.is_empty()), login.show_new_account)
                }
                None => (Color::NONE, false),
            },
            InputKind::Cp(k) => match cp.get(k as usize) {
                Some(v) => {
                    let (valid, req) = cp_field_valid(k, v, &cp);
                    (field_border_color(valid, req, v.is_empty()), login.show_change_password)
                }
                None => (Color::NONE, false),
            },
        };
        sprite.color = if visible { color } else { Color::NONE };
    }

    // 新建账号字段说明（聚焦字段的帮助文字）
    let focused_na = inputs.iter().find_map(|i| {
        if i.focused {
            if let InputKind::Na(k) = i.kind { Some(k) } else { None }
        } else {
            None
        }
    });
    let text = if login.show_new_account { focused_na.map(na_desc).unwrap_or("") } else { "" };
    if let Ok(mut t) = desc.single_mut() {
        t.0 = text.to_string();
    }
}

fn spawn_input(
    commands: &mut Commands,
    font: &Handle<Font>,
    kind: InputKind,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    password: bool,
    focused: bool,
) {
    let text_entity = spawn_ui_text(
        commands,
        font,
        "",
        x + 3.0,
        y + 2.0,
        14.0,
        Color::WHITE,
        3.0,
    );
    commands.spawn((
        UiEntity,
        UiInput {
            value: String::new(),
            focused,
            password,
            rect: (x, y, w, h),
            text_entity,
            kind,
        },
    ));
    spawn_input_border(commands, kind, x, y, w, h);
}

fn spawn_btn(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    index: usize,
    rect: (f32, f32, f32, f32),
    kind: ButtonKind,
    dlg: Option<DialogKind>,
) {
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        name,
        index,
        index + 1,
        index + 2,
        rect.0,
        rect.1,
        2.0,
        rect.2,
        rect.3,
    ) {
        commands.entity(e).insert(UiButtonKind(kind));
        if let Some(d) = dlg {
            commands.entity(e).insert((InDialog(d), Visibility::Hidden));
        }
    }
}

/// 登录状态文本（错误/断线/注册结果）
fn spawn_status_text(
    commands: &mut Commands,
    font: &Handle<Font>,
) {
    let e = spawn_ui_text(
        commands,
        font,
        "",
        360.0,
        508.0,
        16.0,
        Color::srgb(1.0, 0.35, 0.35),
        5.0,
    );
    commands.entity(e).insert(LoginStatusText);
}

fn spawn_new_account_dialog(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    font: &Handle<Font>,
) {
    let dx = NA_X;
    let dy = NA_Y;
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 63) {
        let e = spawn_ui_sprite(commands, h, dx, dy, 4.0, 1.0);
        commands
            .entity(e)
            .insert((InDialog(DialogKind::NewAccount), Visibility::Hidden));
    }
    let ys = [103.0f32, 129.0, 155.0, 189.0, 215.0, 250.0, 276.0, 311.0];
    let pws = [false, true, true, false, false, false, false, false];
    // 原版 NewAccountDialog：密码问题(i=5)/答案(i=6) 为 190 宽，其余 136 宽，统一 18 高
    let widths = [136.0f32, 136.0, 136.0, 136.0, 136.0, 190.0, 190.0, 136.0];
    for i in 0..8 {
        spawn_input(
            commands,
            font,
            InputKind::Na(i as u8),
            dx + 226.0,
            dy + ys[i],
            widths[i],
            18.0,
            pws[i],
            false,
        );
    }
    spawn_btn(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        200,
        (dx + 135.0, dy + 425.0, 76.0, 25.0),
        ButtonKind::NaOk,
        Some(DialogKind::NewAccount),
    );
    spawn_btn(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        203,
        (dx + 409.0, dy + 425.0, 76.0, 25.0),
        ButtonKind::NaCancel,
        Some(DialogKind::NewAccount),
    );
    // 字段说明文字（C# Description (15,340,300x70)，随聚焦字段变化）
    let desc_e = spawn_ui_text(
        commands,
        font,
        "",
        dx + 15.0,
        dy + 340.0,
        12.0,
        Color::srgb(0.85, 0.85, 0.85),
        4.7,
    );
    commands.entity(desc_e).insert((UiEntity, NaDesc));
}

fn spawn_change_password_dialog(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    font: &Handle<Font>,
) {
    let dx = CP_X;
    let dy = CP_Y;
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 50) {
        let e = spawn_ui_sprite(commands, h, dx, dy, 4.0, 1.0);
        commands
            .entity(e)
            .insert((InDialog(DialogKind::ChangePassword), Visibility::Hidden));
    }
    let ys = [75.0f32, 113.0, 151.0, 188.0];
    let pws = [false, true, true, true];
    for i in 0..4 {
        spawn_input(
            commands,
            font,
            InputKind::Cp(i as u8),
            dx + 178.0,
            dy + ys[i],
            136.0,
            18.0,
            pws[i],
            false,
        );
    }
    spawn_btn(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        107,
        (dx + 80.0, dy + 236.0, 90.0, 25.0),
        ButtonKind::CpOk,
        Some(DialogKind::ChangePassword),
    );
    spawn_btn(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        110,
        (dx + 222.0, dy + 236.0, 68.0, 25.0),
        ButtonKind::CpCancel,
        Some(DialogKind::ChangePassword),
    );
}

fn cleanup_login_ui(mut commands: Commands, root: Query<Entity, With<UiEntity>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

fn login_ui_system(
    mut keys: MessageReader<KeyboardInput>,
    mut net: ResMut<NetConnection>,
    mut auth: ResMut<AuthFeedback>,
    mut login: ResMut<LoginState>,
    mut cursor: ResMut<CursorBlink>,
    time: Res<Time>,
    val: Res<LoginValidation>,
    mut inputs: Query<&mut UiInput>,
    mut texts: Query<&mut Text2d>,
    buttons: Query<(&UiButton, &UiButtonKind)>,
    mut dlg_sprites: Query<(&InDialog, &mut Visibility)>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut ime: ResMut<PinyinIme>,
    mut focus: ResMut<ImeFocus>,
) {
    cursor.timer += time.delta_secs();
    if cursor.timer >= 0.5 {
        cursor.timer = 0.0;
        cursor.visible = !cursor.visible;
    }

    let (mx, my) = match windows.single() {
        Ok(w) => w
            .cursor_position()
            .map(|p| (p.x, p.y))
            .unwrap_or((0.0, 0.0)),
        Err(_) => (0.0, 0.0),
    };
    let lclick = mouse.just_pressed(MouseButton::Left);

    // 聚焦 + 回填内置 IME 聚焦框（仅非密码框：候选条定位 + 决定字母是否进 IME）
    // 只写 Some；None 由 clear_ime_focus 每帧统一重置
    for mut input in inputs.iter_mut() {
        let (x, y, w, h) = input.rect;
        if lclick && mx >= x && mx <= x + w && my >= y && my <= y + h {
            input.focused = true;
        } else if lclick {
            input.focused = false;
        }
        if input.focused && !input.password {
            focus.rect = Some(input.rect);
        }
    }

    // 内置拼音 IME 提交的汉字 → 注入聚焦的非密码输入框
    // 先记录本帧是否有 IME 提交（take_commit 会清空 commit_pending，Enter 守卫需要它）
    let ime_committed = ime.has_commit();
    if let Some(c) = ime.take_commit() {
        for mut input in inputs.iter_mut() {
            if input.focused && !input.password {
                input.value.push_str(&c);
            }
        }
    }

    // 键盘
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
    for mut input in inputs.iter_mut() {
        if !input.focused {
            continue;
        }
        for key in &key_list {
            if key.state != bevy::input::ButtonState::Pressed {
                continue;
            }
            // 内置 IME 接管该键（拼音/选候选/编辑）→ 跳过原始插入
            if ime.consumes_key(key) {
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
    }

    // 显示文本 + 对话框输入显隐
    for input in inputs.iter_mut() {
        let shown = match input.kind {
            InputKind::Na(_) => login.show_new_account,
            InputKind::Cp(_) => login.show_change_password,
            _ => true,
        };
        let mut display = if !shown {
            String::new()
        } else if input.password {
            "*".repeat(input.value.len())
        } else {
            input.value.clone()
        };
        if input.focused && cursor.visible {
            display.push('|');
        }
        if let Ok(mut t) = texts.get_mut(input.text_entity) {
            t.0 = display;
        }
    }

    // 调试热键：F9 打开新建账号，F10 打开改密码（live 验证用）
    for key in &key_list {
        if key.state == bevy::input::ButtonState::Pressed {
            match key.logical_key {
                Key::F9 => login.show_new_account = true,
                Key::F10 => login.show_change_password = true,
                Key::Escape => {
                    login.show_new_account = false;
                    login.show_change_password = false;
                }
                _ => {}
            }
        }
    }

    // 对话框精灵显隐
    for (dlg, mut vis) in dlg_sprites.iter_mut() {
        let show = match dlg.0 {
            DialogKind::NewAccount => login.show_new_account,
            DialogKind::ChangePassword => login.show_change_password,
        };
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 按钮点击
    let dialog_open = login.show_new_account || login.show_change_password;
    let mut clicked: Option<ButtonKind> = None;
    for (btn, kind) in buttons.iter() {
        // 对话框按钮：仅在其对话框显示时响应
        let hidden = match kind.0 {
            ButtonKind::NaOk | ButtonKind::NaCancel => !login.show_new_account,
            ButtonKind::CpOk | ButtonKind::CpCancel => !login.show_change_password,
            _ => false,
        };
        if btn.clicked && !hidden {
            let is_dialog_btn = matches!(
                kind.0,
                ButtonKind::NaOk | ButtonKind::NaCancel | ButtonKind::CpOk | ButtonKind::CpCancel
            );
            // 模态：子对话框打开时，主登录界面按钮不响应点击
            if !is_dialog_btn && dialog_open {
                continue;
            }
            clicked = Some(kind.0);
        }
    }
    // Enter 提交（对齐 C# TextBox_KeyPress：登录/新建账号/改密）
    for key in &key_list {
        if key.state == bevy::input::ButtonState::Pressed
            && key.logical_key == Key::Enter
            && !ime.consumes_key(key)
            // 本帧 IME 刚提交候选（Enter 被 IME 消费）→ 不触发登录/注册/改密提交
            && !ime_committed
        {
            if login.show_new_account && val.na_ok {
                clicked = Some(ButtonKind::NaOk);
            } else if login.show_change_password && val.cp_ok {
                clicked = Some(ButtonKind::CpOk);
            } else if val.login_ok {
                clicked = Some(ButtonKind::LoginOk);
            }
        }
    }

    if let Some(kind) = clicked {
        match kind {
            ButtonKind::LoginOk => {
                if net.state != crate::network::NetState::LoggingIn {
                    let mut account = String::new();
                    let mut password = String::new();
                    for input in inputs.iter() {
                        match input.kind {
                            InputKind::LoginAccount => account = input.value.clone(),
                            InputKind::LoginPassword => password = input.value.clone(),
                            _ => {}
                        }
                    }
                    if !val.login_ok {
                        login.status_msg = "账号 3-15 位、密码 5-15 位（字母数字）".to_string();
                        login.status_error = true;
                    } else {
                        net.state = crate::network::NetState::LoggingIn;
                        auth.login_error = None;
                        login.status_msg = String::new();
                        net.send_packet(&mir2_shared::packets::client::account::Login {
                            account_id: account,
                            password,
                        });
                    }
                }
            }
            ButtonKind::NewAccount => login.show_new_account = true,
            ButtonKind::ChangePassword => login.show_change_password = true,
            ButtonKind::NaOk => {
                // 收集新建账号对话框输入（原版顺序：账号/密码/确认/用户名/生日/问题/答案/邮箱）
                let mut v: Vec<String> = vec![String::new(); 8];
                for input in inputs.iter() {
                    if let InputKind::Na(i) = input.kind {
                        if let Some(slot) = v.get_mut(i as usize) {
                            *slot = input.value.clone();
                        }
                    }
                }
                if !val.na_ok {
                    login.status_msg = "请检查标红的字段".to_string();
                    login.status_error = true;
                } else {
                    login.show_new_account = false;
                    login.status_msg = "注册请求已发送…".to_string();
                    login.status_error = false;
                    auth.new_account_error = None;
                    auth.new_account_success = false;
                    net.send_packet(&mir2_shared::packets::client::account::NewAccount {
                        account_id: v[0].clone(),
                        password: v[1].clone(),
                        birth_date_binary: 0,
                        user_name: v[3].clone(),
                        secret_question: v[5].clone(),
                        secret_answer: v[6].clone(),
                        email_address: v[7].clone(),
                    });
                }
            }
            ButtonKind::NaCancel => {
                login.show_new_account = false;
                login.status_msg = String::new();
            }
            ButtonKind::CpOk => {
                // 收集修改密码对话框输入：账号/当前密码/新密码/确认
                let mut v: Vec<String> = vec![String::new(); 4];
                for input in inputs.iter() {
                    if let InputKind::Cp(i) = input.kind {
                        if let Some(slot) = v.get_mut(i as usize) {
                            *slot = input.value.clone();
                        }
                    }
                }
                if !val.cp_ok {
                    login.status_msg = "请检查标红的字段".to_string();
                    login.status_error = true;
                } else {
                    login.show_change_password = false;
                    login.status_msg = "修改密码请求已发送…".to_string();
                    login.status_error = false;
                    auth.change_password_error = None;
                    auth.change_password_success = false;
                    net.send_packet(&mir2_shared::packets::client::account::ChangePassword {
                        account_id: v[0].clone(),
                        current_password: v[1].clone(),
                        new_password: v[2].clone(),
                    });
                }
            }
            ButtonKind::CpCancel => {
                login.show_change_password = false;
                login.status_msg = String::new();
            }
            ButtonKind::ViewKey => {}
            ButtonKind::Close => std::process::exit(0),
        }
    }
}

/// 网络状态提示（登录错误/断线/注册结果）显示到底部状态文本
fn login_status_system(
    mut net: ResMut<NetConnection>,
    mut auth: ResMut<AuthFeedback>,
    login: ResMut<LoginState>,
    mut texts: Query<(&mut Text2d, &mut TextColor), With<LoginStatusText>>,
) {
    // 优先级：断线 > 注册/改密结果 > 登录错误 > 本地校验消息
    let (msg, is_error) = if let Some(d) = &net.disconnected {
        (format!("与服务器断开连接：{}", d), true)
    } else if let Some(e) = &auth.new_account_error {
        (e.clone(), true)
    } else if auth.new_account_success {
        auth.new_account_success = false;
        ("注册成功，请登录".to_string(), false)
    } else if let Some(e) = &auth.change_password_error {
        (e.clone(), true)
    } else if auth.change_password_success {
        auth.change_password_success = false;
        ("密码修改成功".to_string(), false)
    } else if let Some(e) = &auth.login_error {
        (e.clone(), true)
    } else {
        (login.status_msg.clone(), login.status_error)
    };

    if msg.is_empty() {
        return;
    }
    for (mut text, mut color) in texts.iter_mut() {
        text.0 = msg.clone();
        color.0 = if is_error {
            Color::srgb(1.0, 0.35, 0.35)
        } else {
            Color::srgb(0.4, 1.0, 0.4)
        };
    }
}

fn login_anim_system(
    mut anim: ResMut<LoginAnim>,
    mut auth: ResMut<AuthFeedback>,
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time>,
    mut bg: Query<&mut Sprite, With<LoginBg>>,
) {
    if !anim.playing {
        if auth.login_success {
            auth.login_success = false;
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
        if let Ok(mut sprite) = bg.single_mut() {
            if let Some(h) = anim.handles.get(anim.frame) {
                sprite.image = h.clone();
            }
        }
    }
}
