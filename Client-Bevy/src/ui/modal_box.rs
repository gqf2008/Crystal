// ============================================================================
// ModalBoxPlugin - 模态框（对齐原版 C# MirMessageBox / MirInputBox）
// ============================================================================
// 原版流程（SelectScene.DeleteCharacter）：
//   1. 点“删除角色” → MirMessageBox YesNo（Prguse[360]）：确认删除角色「name」？
//   2. 点 Yes      → MirInputBox（Prguse[660]）：输入角色名确认删除
// 原版布局：
// - MirMessageBox:背景 Prguse[360] 456x190，文字(35,35) 390x110，
//                 Yes Title[206-208]@(260,157)，No Title[210-212]@(360,157)
// - MirInputBox:  背景 Prguse[660] 288x156，文字(25,25)，输入框(23,86) 240x19，
//                 OK Title[200-202]@(60,123)，Cancel Title[203-205]@(160,123)
//
// 注意：原版 SelectScene 的 CreditsButton.Click 为空 → Bevy 版不做任何 Credits 弹窗。

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use mir2_shared::SelectInfo;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_sprite, spawn_ui_text, ui_image, UiButton, UiEntity, UiImageCache,
};

pub struct ModalBoxPlugin;

impl Plugin for ModalBoxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModalState>();
        app.add_systems(Update, modal_ui_system.run_if(in_state(AppState::Select)));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    None,
    /// 第一步：MirMessageBox YesNo（确认删除角色？）
    DeleteAsk,
    /// 第二步：MirInputBox（输入角色名确认）
    DeleteConfirm,
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

// 删除确认输入框（原版 MirInputBox，Prguse[660] 288x156）
pub const DLG_X: f32 = (1024.0 - 288.0) / 2.0; // 368
pub const DLG_Y: f32 = (768.0 - 156.0) / 2.0; // 306

// 删除确认询问框（原版 MirMessageBox，Prguse[360] 456x190）
pub const MSG_X: f32 = (1024.0 - 456.0) / 2.0; // 284
pub const MSG_Y: f32 = (768.0 - 190.0) / 2.0; // 289

#[derive(Component)]
struct ModalDeleteDlg;

#[derive(Component)]
struct ModalDeleteAskDlg;

#[derive(Component)]
struct ModalOk;

#[derive(Component)]
struct ModalCancel;

#[derive(Component)]
struct ModalYes;

#[derive(Component)]
struct ModalNo;

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
        commands
            .entity(e)
            .insert((ModalDeleteDlg, Visibility::Hidden));
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

    // ===== 删除确认询问框（MirMessageBox YesNo 样式）=====
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(commands, h, MSG_X, MSG_Y, 6.0, 1.0);
        commands
            .entity(e)
            .insert((ModalDeleteAskDlg, Visibility::Hidden));
    }
    // 提示文字（原版 MirMessageBox Label (35,35) 390x110）
    let ask_text = spawn_ui_text(
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
        .entity(ask_text)
        .insert((UiEntity, ModalDeleteAskDlg, ModalText, Visibility::Hidden));
    // Yes Title[206-208]@(260,157) / No Title[210-212]@(360,157)
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        206,
        207,
        208,
        MSG_X + 260.0,
        MSG_Y + 157.0,
        7.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((ModalDeleteAskDlg, ModalYes, Visibility::Hidden));
    }
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        210,
        211,
        212,
        MSG_X + 360.0,
        MSG_Y + 157.0,
        7.0,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((ModalDeleteAskDlg, ModalNo, Visibility::Hidden));
    }
}

fn modal_ui_system(
    mut keys: MessageReader<KeyboardInput>,
    net: ResMut<NetworkContext>,
    mut state: ResMut<ModalState>,
    time: Res<Time>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut delete_dlg: Query<&mut Visibility, (With<ModalDeleteDlg>, Without<ModalDeleteAskDlg>)>,
    mut delete_ask_dlg: Query<&mut Visibility, (With<ModalDeleteAskDlg>, Without<ModalDeleteDlg>)>,
    // 4 个 Text2d 查询互斥（不同 With/Without），并入 ParamSet 以腾出参数位给内置 IME
    mut texts: ParamSet<(
        Query<&mut Text2d, (With<ModalDeleteAskDlg>, With<ModalText>, Without<ModalDeleteDlg>)>,
        Query<&mut Text2d, (With<ModalDeleteDlg>, With<ModalText>, Without<ModalDeleteAskDlg>)>,
        Query<&mut Text2d, (With<ModalError>, Without<ModalText>, Without<ModalInput>)>,
        Query<&mut Text2d, (With<ModalInput>, Without<ModalText>, Without<ModalError>)>,
    )>,
    ok_btns: Query<&UiButton, With<ModalOk>>,
    cancel_btns: Query<&UiButton, With<ModalCancel>>,
    yes_btns: Query<&UiButton, (With<ModalYes>, Without<ModalOk>, Without<ModalNo>)>,
    no_btns: Query<&UiButton, (With<ModalNo>, Without<ModalOk>, Without<ModalYes>)>,
    mut ime: ResMut<PinyinIme>,
    mut focus: ResMut<ImeFocus>,
) {
    state.cursor_timer += time.delta_secs();
    if state.cursor_timer >= 0.5 {
        state.cursor_timer = 0.0;
        state.cursor_visible = !state.cursor_visible;
    }

    // 回填内置 IME 聚焦框（只写 Some；None 由 clear_ime_focus 每帧统一重置，
    // 避免与 Select 态其他输入框如新建角色名互相覆盖）
    if state.kind == ModalKind::DeleteConfirm && state.input_focused {
        focus.rect = Some((DLG_X + 23.0, DLG_Y + 86.0, 240.0, 19.0));
    }

    let show_delete = state.kind == ModalKind::DeleteConfirm;
    let show_delete_ask = state.kind == ModalKind::DeleteAsk;
    for mut vis in delete_dlg.iter_mut() {
        *vis = if show_delete {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in delete_ask_dlg.iter_mut() {
        *vis = if show_delete_ask {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !show_delete && !show_delete_ask {
        return;
    }

    // 一帧键盘事件只读一次（MessageReader::read() 推进游标，二次 read 为空）
    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();

    // 当前选中角色（删除确认用）
    let selected: Option<SelectInfo> = net
        .selected_index
        .and_then(|i| net.characters.iter().find(|c| c.index == i))
        .cloned();

    // 主文本（询问框 / 输入框各自的 ModalText 分别更新）
    let ask_text = match &selected {
        Some(c) => format!("确定要删除角色「{}」吗？", c.name),
        None => "没有选中的角色。".to_string(),
    };
    for mut t in texts.p0().iter_mut() {
        t.0 = ask_text.clone();
    }
    for mut t in texts.p1().iter_mut() {
        t.0 = "请输入角色名确认删除：".to_string();
    }
    if let Ok(mut t) = texts.p2().single_mut() {
        t.0 = state.error.clone().unwrap_or_default();
    }
    for mut t in texts.p3().iter_mut() {
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

    let in_rect = |rect: (f32, f32, f32, f32)| {
        let (x, y, w, h) = rect;
        mx >= x && mx <= x + w && my >= y && my <= y + h
    };

    // 输入框聚焦 + 字符输入（第二步 MirInputBox）
    if state.kind == ModalKind::DeleteConfirm {
        let input_rect = (DLG_X + 23.0, DLG_Y + 86.0, 240.0, 19.0);
        // 只有点击落在对话框内才改变聚焦（点 OK/Cancel 按钮不清除聚焦）
        if lclick {
            let in_dlg = in_rect((DLG_X, DLG_Y, 288.0, 156.0));
            if in_dlg {
                state.input_focused = in_rect(input_rect);
            }
        }
        if state.input_focused {
            for key in &key_list {
                if key.state != bevy::input::ButtonState::Pressed {
                    continue;
                }
                // 内置 IME 接管该键（拼音/选候选/编辑）→ 跳过原始插入
                if ime.consumes_key(key) {
                    continue;
                }
                match key.logical_key {
                    Key::Backspace => {
                        state.name_input.pop();
                    }
                    _ => {
                        if let Some(text) = &key.text {
                            if !text.is_empty() && state.name_input.chars().count() < 50 {
                                state.name_input.push_str(text);
                            }
                        }
                    }
                }
            }
        }
    }

    // 内置拼音 IME 提交的汉字 → 追加到删除确认输入框（≤50 字）
    // 先记录本帧是否有 IME 提交（take_commit 会清空 commit_pending，Enter 守卫需要它）
    let ime_committed = ime.has_commit();
    if let Some(c) = ime.take_commit() {
        if state.kind == ModalKind::DeleteConfirm && state.input_focused {
            for ch in c.chars() {
                if state.name_input.chars().count() < 50 {
                    state.name_input.push(ch);
                }
            }
        }
    }

    // 提交删除（第二步 MirInputBox OK / 回车）
    let submit_delete =
        |net: &NetworkContext, state: &mut ModalState, selected: &Option<SelectInfo>| {
            if let Some(c) = selected {
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
        };
    // 取消（关闭对话框）
    let cancel = |state: &mut ModalState| {
        state.kind = ModalKind::None;
        state.name_input.clear();
        state.error = None;
    };

    // 键盘：回车=确认，ESC=取消（对齐原版 MirInputBox/MirMessageBox）
    let mut enter = false;
    let mut escape = false;
    for key in &key_list {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        // 内置 IME 接管该键（如组合中按 Enter 提交候选）→ 不触发对话框动作
        if ime.consumes_key(key) {
            continue;
        }
        // 本帧 IME 刚提交候选（Enter 被 IME 消费）→ 不触发对话框 Enter 动作
        if ime_committed && key.logical_key == Key::Enter {
            continue;
        }
        match key.logical_key {
            Key::Enter => enter = true,
            Key::Escape => escape = true,
            _ => {}
        }
    }
    if escape {
        cancel(&mut state);
        return;
    }
    if enter {
        match state.kind {
            ModalKind::DeleteConfirm => submit_delete(&net, &mut state, &selected),
            // 原版 MirMessageBox YesNo 回车 = Yes
            ModalKind::DeleteAsk => {
                state.kind = ModalKind::DeleteConfirm;
                state.name_input.clear();
                state.error = None;
                state.input_focused = true;
            }
            ModalKind::None => {}
        }
        return;
    }

    // 按钮点击
    match state.kind {
        ModalKind::DeleteConfirm => {
            if ok_btns.iter().any(|b| lclick && in_rect(b.rect)) {
                submit_delete(&net, &mut state, &selected);
            } else if cancel_btns.iter().any(|b| lclick && in_rect(b.rect)) {
                cancel(&mut state);
            }
        }
        ModalKind::DeleteAsk => {
            if yes_btns.iter().any(|b| lclick && in_rect(b.rect)) {
                // 点 Yes → 进入输入角色名确认框
                state.kind = ModalKind::DeleteConfirm;
                state.name_input.clear();
                state.error = None;
                state.input_focused = true;
            } else if no_btns.iter().any(|b| lclick && in_rect(b.rect)) {
                cancel(&mut state);
            }
        }
        ModalKind::None => {}
    }
}
