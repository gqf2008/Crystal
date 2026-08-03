// ============================================================================
// LoginPlugin - 登录界面（Sprite 精确坐标版，对齐 macroquad LoginScene）
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::MessageReader;
use bevy::prelude::*;
use bevy::window::Ime;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_camera, spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image,
    UiButton, UiEntity, UiFont, UiImageCache,
};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginState>();
        app.init_resource::<LoginAnim>();
        app.init_resource::<CursorBlink>();
        app.init_resource::<UiImageCache>();
        app.init_resource::<UiFont>();
        app.add_systems(OnEnter(AppState::Login), setup_login_ui);
        app.add_systems(OnExit(AppState::Login), cleanup_login_ui);
        app.add_systems(
            Update,
            (login_ui_system, login_status_system, login_anim_system, ui_button_system)
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

const SW: f32 = 1024.0;
const SH: f32 = 768.0;
const DX: f32 = 348.0;
const DY: f32 = 274.0;

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
    spawn_ui_camera(commands.reborrow());

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

    // 输入框
    spawn_input(
        &mut commands,
        &font,
        InputKind::LoginAccount,
        DX + 85.0,
        DY + 85.0,
        false,
        true,
    );
    spawn_input(
        &mut commands,
        &font,
        InputKind::LoginPassword,
        DX + 85.0,
        DY + 108.0,
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
        commands.entity(e).insert(UiButtonKind(ButtonKind::LoginOk));
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

fn spawn_input(
    commands: &mut Commands,
    font: &Handle<Font>,
    kind: InputKind,
    x: f32,
    y: f32,
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
            rect: (x, y, 136.0, 18.0),
            text_entity,
            kind,
        },
    ));
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
    let dx = (SW - 588.0) / 2.0;
    let dy = (SH - 460.0) / 2.0;
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 63) {
        let e = spawn_ui_sprite(commands, h, dx, dy, 4.0, 1.0);
        commands
            .entity(e)
            .insert((InDialog(DialogKind::NewAccount), Visibility::Hidden));
    }
    let ys = [103.0f32, 129.0, 155.0, 189.0, 215.0, 250.0, 276.0, 311.0];
    let pws = [false, true, true, false, false, false, false, false];
    for i in 0..8 {
        spawn_input(
            commands,
            font,
            InputKind::Na(i as u8),
            dx + 226.0,
            dy + ys[i],
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
}

fn spawn_change_password_dialog(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    font: &Handle<Font>,
) {
    let dx = (SW - 348.0) / 2.0;
    let dy = (SH - 268.0) / 2.0;
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
        (dx + 222.0, dy + 236.0, 90.0, 25.0),
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
    mut net: ResMut<NetworkContext>,
    mut login: ResMut<LoginState>,
    mut cursor: ResMut<CursorBlink>,
    time: Res<Time>,
    mut inputs: Query<&mut UiInput>,
    mut texts: Query<&mut Text2d>,
    buttons: Query<(&UiButton, &UiButtonKind)>,
    mut dlg_sprites: Query<(&InDialog, &mut Visibility)>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut ime: MessageReader<Ime>,
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

    // 聚焦
    for mut input in inputs.iter_mut() {
        let (x, y, w, h) = input.rect;
        if lclick && mx >= x && mx <= x + w && my >= y && my <= y + h {
            input.focused = true;
        } else if lclick {
            input.focused = false;
        }
    }

    // 中文输入法：IME 组合完成文本追加到聚焦输入框
    for ev in ime.read() {
        if let Ime::Commit { value, .. } = ev {
            for mut input in inputs.iter_mut() {
                if input.focused && !input.password {
                    input.value.push_str(value);
                }
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
                    if account.is_empty() {
                        login.status_msg = "请输入账号".to_string();
                        login.status_error = true;
                    } else if password.is_empty() {
                        login.status_msg = "请输入密码".to_string();
                        login.status_error = true;
                    } else {
                        net.state = crate::network::NetState::LoggingIn;
                        net.login_error = None;
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
                if v[0].is_empty() {
                    login.status_msg = "请输入账号".to_string();
                    login.status_error = true;
                } else if v[1].is_empty() {
                    login.status_msg = "请输入密码".to_string();
                    login.status_error = true;
                } else if v[1] != v[2] {
                    login.status_msg = "两次输入的密码不一致".to_string();
                    login.status_error = true;
                } else {
                    login.show_new_account = false;
                    login.status_msg = "注册请求已发送…".to_string();
                    login.status_error = false;
                    net.new_account_error = None;
                    net.new_account_success = false;
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
                if v[0].is_empty() {
                    login.status_msg = "请输入账号".to_string();
                    login.status_error = true;
                } else if v[1].is_empty() {
                    login.status_msg = "请输入当前密码".to_string();
                    login.status_error = true;
                } else if v[2].is_empty() {
                    login.status_msg = "请输入新密码".to_string();
                    login.status_error = true;
                } else if v[2] != v[3] {
                    login.status_msg = "两次输入的新密码不一致".to_string();
                    login.status_error = true;
                } else {
                    login.show_change_password = false;
                    login.status_msg = "修改密码请求已发送…".to_string();
                    login.status_error = false;
                    net.change_password_error = None;
                    net.change_password_success = false;
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
    mut net: ResMut<NetworkContext>,
    login: ResMut<LoginState>,
    mut texts: Query<(&mut Text2d, &mut TextColor), With<LoginStatusText>>,
) {
    // 优先级：断线 > 注册/改密结果 > 登录错误 > 本地校验消息
    let (msg, is_error) = if let Some(d) = &net.disconnected {
        (format!("与服务器断开连接：{}", d), true)
    } else if let Some(e) = &net.new_account_error {
        (e.clone(), true)
    } else if net.new_account_success {
        net.new_account_success = false;
        ("注册成功，请登录".to_string(), false)
    } else if let Some(e) = &net.change_password_error {
        (e.clone(), true)
    } else if net.change_password_success {
        net.change_password_success = false;
        ("密码修改成功".to_string(), false)
    } else if let Some(e) = &net.login_error {
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
    mut net: ResMut<NetworkContext>,
    mut next: ResMut<NextState<AppState>>,
    time: Res<Time>,
    mut bg: Query<&mut Sprite, With<LoginBg>>,
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
        if let Ok(mut sprite) = bg.single_mut() {
            if let Some(h) = anim.handles.get(anim.frame) {
                sprite.image = h.clone();
            }
        }
    }
}
