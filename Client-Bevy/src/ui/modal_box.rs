// ============================================================================
// ModalBoxPlugin - 模态框（删除角色确认 = 原版 MirInputBox / Credits = 原版 MirMessageBox）
// ============================================================================
// 原版布局：
// - MirInputBox:  背景 Prguse[660] 288x156，文字(25,25)，输入框(23,86) 240x19，
//                 OK Title[200-202]@(60,123)，Cancel Title[203-205]@(160,123)
// - MirMessageBox:背景 Prguse[360] 456x190，文字(35,35) 390x110，OK Title[200-202]@(360,157)

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::window::Ime;
use mir2_shared::SelectInfo;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_sprite, spawn_ui_text, ui_image, UiButton, UiEntity, UiImageCache,
};

pub struct ModalBoxPlugin;

impl Plugin for ModalBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModalState>();
        app.add_systems(
            Update,
            modal_ui_system.run_if(in_state(AppState::Select)),
        );
        app.add_systems(
            Update,
            modal_ime_system.run_if(in_state(AppState::Select)),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    None,
    DeleteConfirm,
    Credits,
}

#[derive(Resource)]
pub struct ModalState {
    pub kind: ModalKind,
    pub name_input: String,
    pub input_focused: bool,
    pub error: Option<String>,
    pub cursor_timer: f32,
    pub cursor_visible: bool,
}

impl Default for ModalState {
    fn default() -> Self {
        Self {
            kind: ModalKind::None,
            name_input: String::new(),
            input_focused: true,
            error: None,
            cursor_timer: 0.0,
            cursor_visible: true,
        }
    }
}

// 删除确认框（原版 MirInputBox，Prguse[660] 288x156）
const DLG_X: f32 = (1024.0 - 288.0) / 2.0; // 368
const DLG_Y: f32 = (768.0 - 156.0) / 2.0; // 306
// Credits 框（原版 MirMessageBox，Prguse[360] 456x190）
const MSG_X: f32 = (1024.0 - 456.0) / 2.0; // 284
const MSG_Y: f32 = (768.0 - 190.0) / 2.0; // 289

#[derive(Component)]
struct ModalDeleteDlg;

#[derive(Component)]
struct ModalCreditsDlg;

#[derive(Component)]
struct ModalOk;

#[derive(Component)]
struct ModalCancel;

#[derive(Component)]
struct ModalInput;

#[derive(Component)]
struct ModalText;

#[derive(Component)]
struct ModalError;

/// 生成模态框（隐藏，由 ModalKind 控制显隐）
pub fn spawn_modal_box(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    font: &Handle<Font>,
) {
    // ===== 删除确认框（MirInputBox 样式）=====
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 660) {
        let e = spawn_ui_sprite(commands, h, DLG_X, DLG_Y, 6.0, 1.0);
        commands.entity(e).insert((ModalDeleteDlg, Visibility::Hidden));
    }
    // 提示文字
    let text_e = spawn_ui_text(
        commands,
        font,
        "",
        DLG_X + 25.0,
        DLG_Y + 25.0,
        13.0,
        Color::WHITE,
        7.0,
    );
    commands
        .entity(text_e)
        .insert((UiEntity, ModalDeleteDlg, ModalText, Visibility::Hidden));
    // 输入框底（原版 MirTextBox (23,86) 240x19，绿边框）
    commands.spawn((
        UiEntity,
        ModalDeleteDlg,
        Visibility::Hidden,
        Sprite {
            color: Color::srgba(0.02, 0.03, 0.04, 0.9),
            custom_size: Some(Vec2::new(240.0, 19.0)),
            ..default()
        },
        Transform::from_xyz(DLG_X + 23.0 + 120.0, -(DLG_Y + 86.0 + 9.5), 6.5),
    ));
    commands.spawn((
        UiEntity,
        ModalDeleteDlg,
        Visibility::Hidden,
        Sprite {
            color: Color::srgb(0.0, 1.0, 0.0),
            custom_size: Some(Vec2::new(240.0, 1.0)),
            ..default()
        },
        Transform::from_xyz(DLG_X + 23.0 + 120.0, -(DLG_Y + 86.0), 6.6),
    ));
    commands.spawn((
        UiEntity,
        ModalDeleteDlg,
        Visibility::Hidden,
        Sprite {
            color: Color::srgb(0.0, 1.0, 0.0),
            custom_size: Some(Vec2::new(240.0, 1.0)),
            ..default()
        },
        Transform::from_xyz(DLG_X + 23.0 + 120.0, -(DLG_Y + 86.0 + 19.0), 6.6),
    ));
    commands.spawn((
        UiEntity,
        ModalDeleteDlg,
        Visibility::Hidden,
        Sprite {
            color: Color::srgb(0.0, 1.0, 0.0),
            custom_size: Some(Vec2::new(1.0, 19.0)),
            ..default()
        },
        Transform::from_xyz(DLG_X + 23.0, -(DLG_Y + 86.0 + 9.5), 6.6),
    ));
    commands.spawn((
        UiEntity,
        ModalDeleteDlg,
        Visibility::Hidden,
        Sprite {
            color: Color::srgb(0.0, 1.0, 0.0),
            custom_size: Some(Vec2::new(1.0, 19.0)),
            ..default()
        },
        Transform::from_xyz(DLG_X + 23.0 + 240.0, -(DLG_Y + 86.0 + 9.5), 6.6),
    ));
    // 输入文字
    let input_e = spawn_ui_text(
        commands,
        font,
        "",
        DLG_X + 27.0,
        DLG_Y + 89.0,
        13.0,
        Color::WHITE,
        7.0,
    );
    commands
        .entity(input_e)
        .insert((UiEntity, ModalDeleteDlg, ModalInput, Visibility::Hidden));
    // 错误提示
    let err_e = spawn_ui_text(
        commands,
        font,
        "",
        DLG_X + 25.0,
        DLG_Y + 112.0,
        12.0,
        Color::srgb(1.0, 0.4, 0.4),
        7.0,
    );
    commands
        .entity(err_e)
        .insert((UiEntity, ModalDeleteDlg, ModalError, Visibility::Hidden));
    // OK / Cancel
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        200,
        201,
        202,
        DLG_X + 60.0,
        DLG_Y + 123.0,
        7.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((ModalDeleteDlg, ModalOk, Visibility::Hidden));
    }
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        203,
        204,
        205,
        DLG_X + 160.0,
        DLG_Y + 123.0,
        7.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((ModalDeleteDlg, ModalCancel, Visibility::Hidden));
    }

    // ===== Credits 框（MirMessageBox 样式）=====
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(commands, h, MSG_X, MSG_Y, 6.0, 1.0);
        commands.entity(e).insert((ModalCreditsDlg, Visibility::Hidden));
    }
    let c_text = spawn_ui_text(
        commands,
        font,
        "",
        MSG_X + 35.0,
        MSG_Y + 35.0,
        14.0,
        Color::WHITE,
        7.0,
    );
    commands
        .entity(c_text)
        .insert((UiEntity, ModalCreditsDlg, ModalText, Visibility::Hidden));
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        200,
        201,
        202,
        MSG_X + 360.0,
        MSG_Y + 157.0,
        7.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((ModalCreditsDlg, ModalOk, Visibility::Hidden));
    }
}

fn modal_ui_system(
    mut keys: MessageReader<KeyboardInput>,
    net: ResMut<NetworkContext>,
    mut state: ResMut<ModalState>,
    time: Res<Time>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut delete_dlg: Query<&mut Visibility, (With<ModalDeleteDlg>, Without<ModalCreditsDlg>)>,
    mut credits_dlg: Query<&mut Visibility, (With<ModalCreditsDlg>, Without<ModalDeleteDlg>)>,
    mut texts: Query<&mut Text2d, (With<ModalText>, Without<ModalError>, Without<ModalInput>)>,
    mut errors: Query<&mut Text2d, (With<ModalError>, Without<ModalText>, Without<ModalInput>)>,
    mut inputs: Query<&mut Text2d, (With<ModalInput>, Without<ModalText>, Without<ModalError>)>,
    ok_btns: Query<&UiButton, With<ModalOk>>,
    cancel_btns: Query<&UiButton, With<ModalCancel>>,
) {
    state.cursor_timer += time.delta_secs();
    if state.cursor_timer >= 0.5 {
        state.cursor_timer = 0.0;
        state.cursor_visible = !state.cursor_visible;
    }

    let show_delete = state.kind == ModalKind::DeleteConfirm;
    let show_credits = state.kind == ModalKind::Credits;
    for mut vis in delete_dlg.iter_mut() {
        *vis = if show_delete {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in credits_dlg.iter_mut() {
        *vis = if show_credits {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !show_delete && !show_credits {
        return;
    }

    // 当前选中角色（删除确认用）
    let selected: Option<SelectInfo> = net
        .selected_index
        .and_then(|i| net.characters.iter().find(|c| c.index == i))
        .cloned();

    // 主文本
    let main_text = match state.kind {
        ModalKind::DeleteConfirm => match &selected {
            Some(c) => format!("删除角色「{}」？\n请输入角色名确认：", c.name),
            None => "没有选中的角色。".to_string(),
        },
        ModalKind::Credits => {
            "传 奇 2 (Legend of Mir 2)\nBevy 客户端移植版 v0.1.0\n\n原版资源 + Rust/Bevy 重制".to_string()
        }
        ModalKind::None => String::new(),
    };
    if let Ok(mut t) = texts.single_mut() {
        t.0 = main_text;
    }
    if let Ok(mut t) = errors.single_mut() {
        t.0 = state.error.clone().unwrap_or_default();
    }
    for mut t in inputs.iter_mut() {
        let mut display = state.name_input.clone();
        if state.input_focused && state.cursor_visible {
            display.push('|');
        }
        t.0 = display;
    }

    let (mx, my) = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    let lclick = mouse.just_pressed(MouseButton::Left);

    // 输入框聚焦（删除确认）
    if state.kind == ModalKind::DeleteConfirm {
        let input_rect = (DLG_X + 23.0, DLG_Y + 86.0, 240.0, 19.0);
        if lclick {
            let (x, y, w, h) = input_rect;
            state.input_focused = mx >= x && mx <= x + w && my >= y && my <= y + h;
        }
        let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
        if state.input_focused {
            for key in &key_list {
                if key.state != bevy::input::ButtonState::Pressed {
                    continue;
                }
                match key.logical_key {
                    Key::Backspace => {
                        state.name_input.pop();
                    }
                    _ => {
                        if let Some(text) = &key.text {
                            if !text.is_empty() && state.name_input.chars().count() < 24 {
                                state.name_input.push_str(text);
                            }
                        }
                    }
                }
            }
        }
    }

    // OK（点击自算）
    for btn in ok_btns.iter() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over {
            match state.kind {
                ModalKind::DeleteConfirm => {
                    if let Some(c) = &selected {
                        if state.name_input.trim() == c.name {
                            net.send_packet(&mir2_shared::packets::client::DeleteCharacter {
                                character_index: c.index,
                            });
                            state.kind = ModalKind::None;
                            state.name_input.clear();
                            state.error = None;
                        } else {
                            state.error = Some("输入的角色名不匹配！".to_string());
                        }
                    } else {
                        state.error = Some("没有选中的角色。".to_string());
                    }
                }
                ModalKind::Credits => {
                    state.kind = ModalKind::None;
                }
                ModalKind::None => {}
            }
        }
    }
    for btn in cancel_btns.iter() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over {
            state.kind = ModalKind::None;
            state.name_input.clear();
            state.error = None;
        }
    }
    // ESC 关闭
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
    for key in &key_list {
        if key.state == bevy::input::ButtonState::Pressed && key.logical_key == Key::Escape {
            state.kind = ModalKind::None;
            state.name_input.clear();
            state.error = None;
        }
    }
}

/// 中文输入法：删除确认输入框 IME 组合文本
fn modal_ime_system(
    mut state: ResMut<ModalState>,
    mut ime: MessageReader<Ime>,
    mut windows: Query<&mut Window>,
) {
    if state.kind != ModalKind::DeleteConfirm || !state.input_focused {
        return;
    }
    if let Ok(mut w) = windows.single_mut() {
        w.ime_position = Vec2::new(DLG_X + 23.0, DLG_Y + 86.0);
    }
    for ev in ime.read() {
        if let Ime::Commit { value, .. } = ev {
            for ch in value.chars() {
                if state.name_input.chars().count() < 24 {
                    state.name_input.push(ch);
                }
            }
        }
    }
}
