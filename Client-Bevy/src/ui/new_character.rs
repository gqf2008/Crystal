// ============================================================================
// NewCharacterPlugin - 新建角色对话框（对齐原版 C# NewCharacterDialog）
// ============================================================================
// 原版布局（Client/MirScenes/Dialogs/NewCharacterDialog.cs）：
// - 背景 Prguse[73] 588x460，屏幕居中
// - 职业按钮 Prguse[2426..2437]（44x42）：战士(323,296) 法师(373,296) 道士(423,296) 刺客(473,296)
// - 男女按钮 Prguse[2420..2425]（44x42）：男(323,343) 女(373,343)
// - OK Title[360..362] (160,425) / Cancel Title[280..282] (425,425)
// - 名字输入框 (325,268) 240x20；描述 (279,70)；预览 ChrSel 16帧 (120,250) UseOffSet=true

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use mir2_shared::{MirClass, MirGender};

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_sprite, spawn_ui_text, ui_image, UiButton, UiEntity,
    UiImageCache,
};

pub struct NewCharacterPlugin;

impl Plugin for NewCharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewCharState>();
        app.add_systems(
            Update,
            new_char_ui_system.run_if(in_state(AppState::Select)),
        );
        app.add_systems(
            Update,
            new_char_anim_system.run_if(in_state(AppState::Select)),
        );
    }
}

/// 新建角色对话框状态
#[derive(Resource)]
pub struct NewCharState {
    pub visible: bool,
    pub class: MirClass,
    pub gender: MirGender,
    /// 上一次预览用职业/性别（检测变化后重载预览帧）
    pub last_class: MirClass,
    pub last_gender: MirGender,
    pub name: String,
    pub name_focused: bool,
    /// 创建失败提示（由 select_ui_system 从网络层桥接）
    pub error: Option<String>,
    pub cursor_visible: bool,
    pub cursor_timer: f32,
    pub preview_frame: usize,
    pub preview_timer: f32,
    pub preview_handles: Vec<Handle<Image>>,
    pub preview_offsets: Vec<(f32, f32)>,
}

impl Default for NewCharState {
    fn default() -> Self {
        Self {
            visible: false,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            last_class: MirClass::Warrior,
            last_gender: MirGender::Male,
            name: String::new(),
            name_focused: true,
            error: None,
            cursor_visible: true,
            cursor_timer: 0.0,
            preview_frame: 0,
            preview_timer: 0.0,
            preview_handles: Vec::new(),
            preview_offsets: Vec::new(),
        }
    }
}

#[derive(Component)]
struct NcDlg;

#[derive(Component)]
struct NcClassBtn {
    class: MirClass,
    rect: (f32, f32, f32, f32),
    /// [normal, selected, pressed]
    frames: [usize; 3],
}

#[derive(Component)]
struct NcGenderBtn {
    gender: MirGender,
    rect: (f32, f32, f32, f32),
    frames: [usize; 3],
}

#[derive(Component)]
struct NcOkBtn;

#[derive(Component)]
struct NcCancelBtn;

#[derive(Component)]
struct NcPreview;

#[derive(Component)]
struct NcDesc;

#[derive(Component)]
struct NcError;

#[derive(Component)]
struct NcNameBox;

/// 对话框常量（相对 1024x768 画布，背景居中）
const DLG_W: f32 = 588.0;
const DLG_H: f32 = 460.0;
const DLG_X: f32 = (1024.0 - DLG_W) / 2.0; // 218
const DLG_Y: f32 = (768.0 - DLG_H) / 2.0; // 154
const PREVIEW_X: f32 = 120.0;
const PREVIEW_Y: f32 = 250.0;

/// 预览起始帧（对齐原版 UpdateInterface）
pub(crate) fn new_char_preview_base(class: MirClass, gender: MirGender) -> usize {
    let g = if gender == MirGender::Female { 1usize } else { 0 };
    match class {
        MirClass::Archer => {
            if g == 0 {
                100
            } else {
                140
            }
        }
        _ => 20 + (class as usize * 20) + (g * 280),
    }
}

fn class_desc(class: MirClass) -> &'static str {
    match class {
        MirClass::Warrior => "近战王者，血厚攻高，冲锋陷阵。",
        MirClass::Wizard => "远程法术，群体伤害，血薄需保护。",
        MirClass::Taoist => "召唤神兽，辅助治疗，攻守兼备。",
        MirClass::Assassin => "身法迅捷，爆发力强，近身刺杀。",
        MirClass::Archer => "百步穿杨，远程牵制，来去自如。",
    }
}

/// 加载某职业/性别的 16 帧预览 + 每帧偏移（原版 UseOffSet=true）
fn load_preview_frames(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    state: &mut NewCharState,
) {
    state.preview_handles.clear();
    state.preview_offsets.clear();
    let base = new_char_preview_base(state.class, state.gender);
    for i in 0..16usize {
        let idx = base + i;
        let offsets = libs
            .0
            .get_image(LibraryName::ChrSel, idx)
            .map(|info| (info.offset_x as f32, info.offset_y as f32));
        if let Some(h) = ui_image(libs, images, cache, LibraryName::ChrSel, idx) {
            state.preview_handles.push(h);
            state.preview_offsets.push(offsets.unwrap_or((0.0, 0.0)));
        }
    }
}

fn preview_pos(state: &NewCharState, frame: usize) -> (f32, f32) {
    let (ox, oy) = state
        .preview_offsets
        .get(frame)
        .copied()
        .unwrap_or((0.0, 0.0));
    (DLG_X + PREVIEW_X + ox, DLG_Y + PREVIEW_Y + oy)
}

/// 生成新建角色对话框（由 SelectPlugin setup 调用；实体带 NcDlg 标记便于显隐）
pub fn spawn_new_character_dialog(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    font: &Handle<Font>,
    state: &mut NewCharState,
) {
    // 背景 Prguse[73]
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Prguse, 73) {
        let e = spawn_ui_sprite(commands, h, DLG_X, DLG_Y, 4.0, 1.0);
        commands.entity(e).insert((NcDlg, Visibility::Hidden));
    }
    // 标题 Title[20]
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Title, 20) {
        let e = spawn_ui_sprite(commands, h, DLG_X + 206.0, DLG_Y + 11.0, 5.0, 1.0);
        commands.entity(e).insert((NcDlg, Visibility::Hidden));
    }
    // 预览（初始战士男）
    load_preview_frames(libs, images, cache, state);
    if let Some(h) = state.preview_handles.first().cloned() {
        let (px, py) = preview_pos(state, 0);
        let e = spawn_ui_sprite(commands, h, px, py, 5.0, 1.0);
        commands.entity(e).insert((NcDlg, NcPreview, Visibility::Hidden));
    }
    // 描述
    let desc_e = spawn_ui_text(
        commands,
        font,
        class_desc(state.class),
        DLG_X + 279.0,
        DLG_Y + 70.0,
        13.0,
        Color::WHITE,
        5.0,
    );
    commands
        .entity(desc_e)
        .insert((NcDlg, NcDesc, Visibility::Hidden));
    // 创建失败提示
    let err_e = spawn_ui_text(
        commands,
        font,
        "",
        DLG_X + 279.0,
        DLG_Y + 92.0,
        13.0,
        Color::srgb(1.0, 0.4, 0.4),
        3.0,
    );
    commands
        .entity(err_e)
        .insert((NcDlg, NcError, Visibility::Hidden));
    // 名字输入框底色（原版 MirTextBox 位置）
    commands.spawn((
        UiEntity,
        NcDlg,
        Visibility::Hidden,
        Sprite {
            color: Color::srgba(0.05, 0.06, 0.08, 0.85),
            custom_size: Some(Vec2::new(240.0, 20.0)),
            ..default()
        },
        Transform::from_xyz(DLG_X + 325.0 + 120.0, -(DLG_Y + 268.0 + 10.0), 4.5),
    ));
    // 名字文本
    let name_e = spawn_ui_text(
        commands,
        font,
        "",
        DLG_X + 329.0,
        DLG_Y + 271.0,
        14.0,
        Color::WHITE,
        5.0,
    );
    commands
        .entity(name_e)
        .insert((NcDlg, NcNameBox, Visibility::Hidden));

    // 职业按钮：战士/法师/道士/刺客（原版坐标，帧组 2426/2429/2432/2435）
    let class_btns: [(MirClass, usize, f32); 4] = [
        (MirClass::Warrior, 2426, 323.0),
        (MirClass::Wizard, 2429, 373.0),
        (MirClass::Taoist, 2432, 423.0),
        (MirClass::Assassin, 2435, 473.0),
    ];
    for (class, base, x) in class_btns {
        if let Some(e) = spawn_ui_button(
            commands,
            libs,
            images,
            cache,
            LibraryName::Prguse,
            base,
            base + 1,
            base + 2,
            DLG_X + x,
            DLG_Y + 296.0,
            5.0,
            44.0,
            42.0,
        ) {
            commands.entity(e).insert((
                NcDlg,
                NcClassBtn {
                    class,
                    rect: (DLG_X + x, DLG_Y + 296.0, 44.0, 42.0),
                    frames: [base, base + 1, base + 2],
                },
                Visibility::Hidden,
            ));
        }
    }
    // 男女按钮
    let gender_btns: [(MirGender, usize, f32); 2] = [
        (MirGender::Male, 2420, 323.0),
        (MirGender::Female, 2423, 373.0),
    ];
    for (gender, base, x) in gender_btns {
        if let Some(e) = spawn_ui_button(
            commands,
            libs,
            images,
            cache,
            LibraryName::Prguse,
            base,
            base + 1,
            base + 2,
            DLG_X + x,
            DLG_Y + 343.0,
            5.0,
            44.0,
            42.0,
        ) {
            commands.entity(e).insert((
                NcDlg,
                NcGenderBtn {
                    gender,
                    rect: (DLG_X + x, DLG_Y + 343.0, 44.0, 42.0),
                    frames: [base, base + 1, base + 2],
                },
                Visibility::Hidden,
            ));
        }
    }
    // OK / Cancel
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        360,
        361,
        362,
        DLG_X + 160.0,
        DLG_Y + 425.0,
        5.0,
        60.0,
        25.0,
    ) {
        commands.entity(e).insert((NcDlg, NcOkBtn, Visibility::Hidden));
    }
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        280,
        281,
        282,
        DLG_X + 425.0,
        DLG_Y + 425.0,
        5.0,
        60.0,
        25.0,
    ) {
        commands.entity(e).insert((NcDlg, NcCancelBtn, Visibility::Hidden));
    }
}

#[allow(clippy::too_many_arguments)]
fn new_char_ui_system(
    mut keys: MessageReader<KeyboardInput>,
    net: ResMut<NetworkContext>,
    mut state: ResMut<NewCharState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut dlg: Query<&mut Visibility, With<NcDlg>>,
    mut class_btns: Query<(&NcClassBtn, &mut Sprite), (Without<NcPreview>, Without<NcGenderBtn>)>,
    mut gender_btns: Query<(&NcGenderBtn, &mut Sprite), (Without<NcPreview>, Without<NcClassBtn>)>,
    mut preview: Query<(&mut Sprite, &mut Transform), With<NcPreview>>,
    mut texts: ParamSet<(
        Query<&mut Text2d, (With<NcNameBox>, Without<NcDesc>, Without<NcError>)>,
        Query<&mut Text2d, (With<NcDesc>, Without<NcNameBox>, Without<NcError>)>,
        Query<&mut Text2d, (With<NcError>, Without<NcNameBox>, Without<NcDesc>)>,
    )>,
    ok_btns: Query<&UiButton, With<NcOkBtn>>,
    cancel_btns: Query<&UiButton, With<NcCancelBtn>>,
) {
    // 显隐
    let show = state.visible;
    for mut vis in dlg.iter_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if !show {
        return;
    }

    let (mx, my) = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    let lclick = mouse.just_pressed(MouseButton::Left);

    // 名字输入聚焦
    let name_rect = (DLG_X + 325.0, DLG_Y + 268.0, 240.0, 20.0);
    if lclick {
        let (x, y, w, h) = name_rect;
        state.name_focused = mx >= x && mx <= x + w && my >= y && my <= y + h;
    }

    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();
    if state.name_focused {
        for key in &key_list {
            if key.state != bevy::input::ButtonState::Pressed {
                continue;
            }
            match key.logical_key {
                Key::Backspace => {
                    state.name.pop();
                }
                _ => {
                    if let Some(text) = &key.text {
                        if !text.is_empty() && state.name.chars().count() < 12 {
                            state.name.push_str(text);
                        }
                    }
                }
            }
        }
    }

    // 职业按钮点击 + 帧更新
    for (btn, mut sprite) in class_btns.iter_mut() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over {
            state.class = btn.class;
        }
        let selected = state.class == btn.class;
        let frame = if selected { btn.frames[1] } else { btn.frames[0] };
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, frame) {
            if sprite.image != h {
                sprite.image = h;
            }
        }
    }
    // 男女按钮点击 + 帧更新
    for (btn, mut sprite) in gender_btns.iter_mut() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over {
            state.gender = btn.gender;
        }
        let selected = state.gender == btn.gender;
        let frame = if selected { btn.frames[1] } else { btn.frames[0] };
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, frame) {
            if sprite.image != h {
                sprite.image = h;
            }
        }
    }

    // 职业/性别变化 → 重载预览
    if state.class != state.last_class || state.gender != state.last_gender {
        state.last_class = state.class;
        state.last_gender = state.gender;
        state.preview_frame = 0;
        load_preview_frames(&mut libs, &mut images, &mut cache, &mut state);
        if let Ok((mut s, mut tf)) = preview.single_mut() {
            if let Some(h) = state.preview_handles.first() {
                s.image = h.clone();
                let (px, py) = preview_pos(&state, 0);
                tf.translation.x = px;
                tf.translation.y = -py;
            }
        }
        if let Ok(mut t) = texts.p1().single_mut() {
            t.0 = class_desc(state.class).to_string();
        }
    }

    // OK / Cancel（点击自算）
    for btn in ok_btns.iter() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over && !state.name.is_empty() {
            state.visible = false;
            state.error = None;
            net.send_packet(&mir2_shared::packets::client::NewCharacter {
                name: state.name.clone(),
                gender: state.gender,
                class: state.class,
            });
            state.name.clear();
        }
    }
    for btn in cancel_btns.iter() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over {
            state.visible = false;
            state.name.clear();
        }
    }
    // ESC 关闭
    for key in &key_list {
        if key.state == bevy::input::ButtonState::Pressed && key.logical_key == Key::Escape {
            state.visible = false;
            state.name.clear();
        }
    }

    // 创建失败提示显示
    if let Ok(mut t) = texts.p2().single_mut() {
        t.0 = state.error.clone().unwrap_or_default();
    }

    // 名字显示（带光标）
    let mut display = state.name.clone();
    if state.name_focused && state.cursor_visible {
        display.push('|');
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        t.0 = display;
    }
}

fn new_char_anim_system(
    mut state: ResMut<NewCharState>,
    time: Res<Time>,
    mut preview: Query<(&mut Sprite, &mut Transform), With<NcPreview>>,
) {
    if !state.visible {
        return;
    }
    state.preview_timer += time.delta_secs();
    if state.preview_timer >= 0.25 {
        state.preview_timer = 0.0;
        state.preview_frame = (state.preview_frame + 1) % state.preview_handles.len().max(1);
        if let Ok(mut s) = preview.single_mut() {
            if let Some(h) = state.preview_handles.get(state.preview_frame) {
                s.0.image = h.clone();
                let (px, py) = preview_pos(&state, state.preview_frame);
                s.1.translation.x = px;
                s.1.translation.y = -py;
            }
        }
    }
}

