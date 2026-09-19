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
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::pinyin_ime::{ImeFocus, PinyinIme};
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_sprite, spawn_ui_text, ui_image, UiButton, UiEntity, UiImageCache,
};

pub struct NewCharacterPlugin;

impl Plugin for NewCharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewCharState>();
        // #2892 批C：同一个对话框在 Select（玩家建角）与 Game（英雄创建，
        // `S.HeroCreateRequest` 触发，`hero_mode = true`）两个状态下都用
        let active = in_state(AppState::Select).or(in_state(AppState::Game));
        app.add_systems(
            Update,
            (new_char_ui_system, new_char_ime_system)
                .chain()
                .run_if(active.clone()),
        );
        app.add_systems(Update, new_char_anim_system.run_if(active.clone()));
        app.add_systems(Update, new_char_name_border_system.run_if(active));
        // #2892 批C：游戏内预生成（隐藏）英雄创建对话框
        app.add_systems(OnEnter(AppState::Game), spawn_hero_create_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_hero_create_dialog);
    }
}

/// #2892 批C：进游戏时预生成同一个「新建角色」对话框（隐藏），
/// 由 `S.HeroCreateRequest` 置 `hero_mode` + `visible` 后显示
/// （C# `NewHeroDialog` 就是 `NewCharacterDialog` 的一个实例，`GameScene.cs:320`）。
fn spawn_hero_create_dialog(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut state: ResMut<NewCharState>,
) {
    libs.0.ensure_initialized();
    let font = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    state.hero_mode = false;
    state.visible = false;
    spawn_new_character_dialog(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        &font,
        &mut state,
    );
}

/// 退出 Game 时清理英雄创建对话框实体（Select 场景的由该场景自行清理）
fn cleanup_hero_create_dialog(mut commands: Commands, q: Query<Entity, With<NcDlg>>) {
    for e in q.iter() {
        commands.entity(e).despawn();
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
    /// 法师 blend 叠加层 16 帧（ChrSel[frame+560]；对齐 C# DrawBlend）
    pub blend_handles: Vec<Handle<Image>>,
    pub blend_offsets: Vec<(f32, f32)>,
    /// #2892 批C：英雄创建模式（C# `NewHeroDialog = new NewCharacterDialog{...}`，
    /// `GameScene.cs:320-333`）——标题换 `Title[847]@(246,11)`、OK 发 `C.NewHero`、
    /// 职业钮按 `can_create_class` 显隐
    pub hero_mode: bool,
    /// `[Warrior, Wizard, Taoist, Assassin, Archer]`（C# `S.HeroCreateRequest.CanCreateClass`）
    pub can_create_class: [bool; 5],
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
            blend_handles: Vec::new(),
            blend_offsets: Vec::new(),
            hero_mode: false,
            can_create_class: [true; 5],
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

/// 法师 blend 叠加层精灵（对齐 C# CharacterDisplay.AfterDraw: Class==Wizard 时
/// ChrSel.DrawBlend(Index+560, DisplayLocationWithoutOffSet, White, offSet=true)）
#[derive(Component)]
struct NcBlend;

#[derive(Component)]
struct NcDesc;

#[derive(Component)]
struct NcError;

#[derive(Component)]
struct NcNameBox;

/// 名字输入框校验边框（4 条细线；颜色随校验结果变化：空=透明 / 不合法=红 / 合法=绿）
#[derive(Component)]
struct NcNameBorder;

/// 玩家创建标题 `Title[20]`（英雄模式下隐藏）
#[derive(Component)]
struct NcTitlePlayer;

/// 英雄创建标题 `Title[847]`（C# `GameScene.cs:321`；非英雄模式隐藏）
#[derive(Component)]
struct NcTitleHero;

/// #2892 批C：提交创建——英雄模式发 `C.NewHero`，否则发 `C.NewCharacter`
/// （C# 是同类的两个实例：`NewHeroDialog` 的 `OnCreateCharacter` 回调发 `C.NewHero`）
fn submit_new_char(net: &NetConnection, state: &NewCharState) {
    if state.hero_mode {
        net.send_packet(&mir2_shared::packets::client::hero::NewHero {
            name: state.name.clone(),
            gender: state.gender,
            class: state.class,
        });
    } else {
        net.send_packet(&mir2_shared::packets::client::NewCharacter {
            name: state.name.clone(),
            gender: state.gender,
            class: state.class,
        });
    }
}

/// 名字合法性（对齐原版 NewCharacterDialog 正则意图：仅允许字母数字与中文，长度 1..=15）。
/// 原版正则 `^[A-Za-z0-9]|[一-龥]{3,15}$` 因 `|` 拼接存在缺陷，此处取其语义。
fn name_valid(name: &str) -> bool {
    let count = name.chars().count();
    if count == 0 || count > 15 {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fa5}').contains(&c))
}

/// 对话框常量（相对 1024x768 画布，背景居中）
const DLG_W: f32 = 588.0;
const DLG_H: f32 = 460.0;
pub const DLG_X: f32 = (1024.0 - DLG_W) / 2.0; // 218
pub const DLG_Y: f32 = (768.0 - DLG_H) / 2.0; // 154
const PREVIEW_X: f32 = 120.0;
const PREVIEW_Y: f32 = 250.0;

/// 预览起始帧（对齐原版 UpdateInterface）
pub(crate) fn new_char_preview_base(class: MirClass, gender: MirGender) -> usize {
    let g = if gender == MirGender::Female {
        1usize
    } else {
        0
    };
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
    // 法师 blend 叠加层（ChrSel[frame+560]）。非法师职业该段为空占位（4x1），仅 Wizard 有内容。
    state.blend_handles.clear();
    state.blend_offsets.clear();
    for i in 0..16usize {
        let idx = base + i + 560;
        let boff = libs
            .0
            .get_image(LibraryName::ChrSel, idx)
            .map(|info| (info.offset_x as f32, info.offset_y as f32));
        if let Some(h) = ui_image(libs, images, cache, LibraryName::ChrSel, idx) {
            state.blend_handles.push(h);
            state.blend_offsets.push(boff.unwrap_or((0.0, 0.0)));
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

/// blend 叠加层屏幕坐标（= Location + blend 精灵自身 offset，与主帧同 Location、各自 offset）
fn blend_pos(state: &NewCharState, frame: usize) -> (f32, f32) {
    let (ox, oy) = state
        .blend_offsets
        .get(frame)
        .copied()
        .unwrap_or((0.0, 0.0));
    (DLG_X + PREVIEW_X + ox, DLG_Y + PREVIEW_Y + oy)
}

/// 生成对话框作用域的空心边框（4 条细线，带 NcDlg 随对话框显隐），返回 4 个实体。
/// 用于对齐原版 MirLabel/MirTextBox 的 Border=true。
fn spawn_dlg_border(
    commands: &mut Commands,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    z: f32,
) -> Vec<Entity> {
    let t = 1.0_f32; // 线宽
    let mut ids: Vec<Entity> = Vec::with_capacity(4);
    let mut line = |cx: f32, cy: f32, sw: f32, sh: f32| -> Entity {
        commands
            .spawn((
                UiEntity,
                NcDlg,
                Visibility::Hidden,
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(sw, sh)),
                    ..default()
                },
                Transform::from_xyz(cx, -cy, z),
            ))
            .id()
    };
    ids.push(line(x + w / 2.0, y - t / 2.0, w + t, t)); // 上
    ids.push(line(x + w / 2.0, y + h + t / 2.0, w + t, t)); // 下
    ids.push(line(x - t / 2.0, y + h / 2.0, t, h + t)); // 左
    ids.push(line(x + w + t / 2.0, y + h / 2.0, t, h + t)); // 右
    ids
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
        commands
            .entity(e)
            .insert((NcDlg, NcTitlePlayer, Visibility::Hidden));
    }
    // #2892 批C：英雄创建标题 `Title[847]` @(246,11)（C# `GameScene.cs:321-322` 覆盖 TitleLabel）
    if let Some(h) = ui_image(libs, images, cache, LibraryName::Title, 847) {
        let e = spawn_ui_sprite(commands, h, DLG_X + 246.0, DLG_Y + 11.0, 5.0, 1.0);
        commands
            .entity(e)
            .insert((NcDlg, NcTitleHero, Visibility::Hidden));
    }
    // 预览（初始战士男）
    load_preview_frames(libs, images, cache, state);
    if let Some(h) = state.preview_handles.first().cloned() {
        let (px, py) = preview_pos(state, 0);
        let e = spawn_ui_sprite(commands, h, px, py, 5.0, 1.0);
        commands
            .entity(e)
            .insert((NcDlg, NcPreview, Visibility::Hidden));
    }
    // 法师 blend 叠加层（z=5.1 略高于预览；显隐由 new_char_ui_system 按 class==Wizard 控制）
    if let Some(h) = state.blend_handles.first().cloned() {
        let (bx, by) = blend_pos(state, 0);
        let e = spawn_ui_sprite(commands, h, bx, by, 5.1, 1.0);
        commands
            .entity(e)
            .insert((NcDlg, NcBlend, Visibility::Hidden));
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
    // 描述边框（原版 Description MirLabel Border=true，278x170 @(279,70)）
    spawn_dlg_border(
        commands,
        DLG_X + 279.0,
        DLG_Y + 70.0,
        278.0,
        170.0,
        Color::srgb(0.4, 0.4, 0.4),
        4.8,
    );
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
    // 名字校验边框（240x20 @(325,268)；颜色由 new_char_name_border_system 按合法性更新）
    for e in spawn_dlg_border(
        commands,
        DLG_X + 325.0,
        DLG_Y + 268.0,
        240.0,
        20.0,
        Color::srgba(0.0, 0.0, 0.0, 0.0),
        4.6,
    ) {
        commands.entity(e).insert(NcNameBorder);
    }
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

    // 职业按钮：战士/法师/道士/刺客/弓手（原版坐标 y=296，帧组 2426/2429/2432/2435/2438）
    let class_btns: [(MirClass, usize, f32); 5] = [
        (MirClass::Warrior, 2426, 323.0),
        (MirClass::Wizard, 2429, 373.0),
        (MirClass::Taoist, 2432, 423.0),
        (MirClass::Assassin, 2435, 473.0),
        (MirClass::Archer, 2438, 523.0),
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
        commands
            .entity(e)
            .insert((NcDlg, NcOkBtn, Visibility::Hidden));
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
        commands
            .entity(e)
            .insert((NcDlg, NcCancelBtn, Visibility::Hidden));
    }
}

#[allow(clippy::too_many_arguments)]
fn new_char_ui_system(
    mut keys: MessageReader<KeyboardInput>,
    net: ResMut<NetConnection>,
    mut state: ResMut<NewCharState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut dlg: Query<
        (
            &mut Visibility,
            Option<&NcTitlePlayer>,
            Option<&NcTitleHero>,
            Option<&NcClassBtn>,
        ),
        (With<NcDlg>, Without<NcBlend>),
    >,
    mut class_btns: Query<
        (&NcClassBtn, &mut Sprite),
        (Without<NcPreview>, Without<NcGenderBtn>, Without<NcBlend>),
    >,
    mut gender_btns: Query<
        (&NcGenderBtn, &mut Sprite),
        (Without<NcPreview>, Without<NcClassBtn>, Without<NcBlend>),
    >,
    // p0=主预览帧；p1=法师 blend 叠加层（含 Visibility 以按 class==Wizard 控制显隐）
    mut preview: ParamSet<(
        Query<(&mut Sprite, &mut Transform), With<NcPreview>>,
        Query<(&mut Sprite, &mut Transform, &mut Visibility), With<NcBlend>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text2d, (With<NcNameBox>, Without<NcDesc>, Without<NcError>)>,
        Query<&mut Text2d, (With<NcDesc>, Without<NcNameBox>, Without<NcError>)>,
        Query<&mut Text2d, (With<NcError>, Without<NcNameBox>, Without<NcDesc>)>,
    )>,
    ok_btns: Query<&UiButton, With<NcOkBtn>>,
    cancel_btns: Query<&UiButton, With<NcCancelBtn>>,
    mut ime: ResMut<PinyinIme>,
) {
    // 显隐
    let show = state.visible;
    // #2892 批C：标题按模式二选一（玩家 `Title[20]` / 英雄 `Title[847]`），
    // 职业钮按 `S.HeroCreateRequest.CanCreateClass` 显隐（C# `GameScene.cs:6044-6052`）
    for (mut vis, title_player, title_hero, class_btn) in dlg.iter_mut() {
        let want = if !show {
            Visibility::Hidden
        } else if title_player.is_some() {
            if state.hero_mode {
                Visibility::Hidden
            } else {
                Visibility::Visible
            }
        } else if title_hero.is_some() {
            if state.hero_mode {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else if let Some(btn) = class_btn {
            let allowed = state
                .can_create_class
                .get(btn.class as usize)
                .copied()
                .unwrap_or(true);
            if allowed {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        } else {
            Visibility::Visible
        };
        if *vis != want {
            *vis = want;
        }
    }
    // 法师 blend 叠加层：仅对话框可见且职业为法师时显示（对齐 C# AfterDraw: Class==Wizard）
    let blend_show = show && state.class == MirClass::Wizard;
    for (_, _, mut vis) in preview.p1().iter_mut() {
        *vis = if blend_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
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
            // 内置 IME 接管该键（拼音/选候选/编辑）→ 跳过原始插入
            if ime.consumes_key(key) {
                continue;
            }
            match key.logical_key {
                Key::Backspace => {
                    // #2596-10：IME 本帧消费的退格按次数精确跳过，其余放行
                    if ime.consume_backspace() {
                        continue;
                    }
                    state.name.pop();
                }
                _ => {
                    if let Some(text) = &key.text {
                        if !text.is_empty() && state.name.chars().count() < 15 {
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
        let frame = if selected {
            btn.frames[1]
        } else {
            btn.frames[0]
        };
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            frame,
        ) {
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
        let frame = if selected {
            btn.frames[1]
        } else {
            btn.frames[0]
        };
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Prguse,
            frame,
        ) {
            if sprite.image != h {
                sprite.image = h;
            }
        }
    }

    // 职业/性别变化 → 重载预览（主帧 + 法师 blend 叠加层）
    if state.class != state.last_class || state.gender != state.last_gender {
        state.last_class = state.class;
        state.last_gender = state.gender;
        state.preview_frame = 0;
        load_preview_frames(&mut libs, &mut images, &mut cache, &mut state);
        if let Ok((mut s, mut tf)) = preview.p0().single_mut() {
            if let Some(h) = state.preview_handles.first() {
                s.image = h.clone();
                let (px, py) = preview_pos(&state, 0);
                tf.translation.x = px;
                tf.translation.y = -py;
            }
        }
        // blend 叠加层复位到第 0 帧（仅法师有内容；非法师为 4x1 空占位）
        if let Ok((mut s, mut tf, _)) = preview.p1().single_mut() {
            if let Some(h) = state.blend_handles.first() {
                s.image = h.clone();
                let (bx, by) = blend_pos(&state, 0);
                tf.translation.x = bx;
                tf.translation.y = -by;
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
        if lclick && over && name_valid(&state.name) {
            state.visible = false;
            state.error = None;
            submit_new_char(&net, &state);
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
    // ESC 关闭 / Enter 提交（对齐 C# NewCharacterDialog：Esc 隐藏，TextBox_KeyPress 中
    // Enter 且 OKButton.Enabled → CreateCharacter）
    for key in &key_list {
        if key.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        // 内置 IME 接管该键（如组合中按 Enter 提交候选）→ 不触发提交
        if ime.consumes_key(key) {
            continue;
        }
        match key.logical_key {
            Key::Escape => {
                state.visible = false;
                state.name.clear();
            }
            Key::Enter if name_valid(&state.name) => {
                state.visible = false;
                state.error = None;
                submit_new_char(&net, &state);
                state.name.clear();
            }
            _ => {}
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

/// 内置拼音 IME：回填名字框聚焦矩形 + 注入已选汉字（单独系统避免参数超限）
fn new_char_ime_system(
    mut state: ResMut<NewCharState>,
    mut ime: ResMut<PinyinIme>,
    mut focus: ResMut<ImeFocus>,
) {
    // 只写 Some（None 由 clear_ime_focus 每帧统一重置，避免与 Select 态其他输入框互相覆盖）
    if state.visible && state.name_focused {
        // 名字输入框屏幕矩形（候选条定位 + 判定字母是否进 IME）
        focus.rect = Some((DLG_X + 325.0, DLG_Y + 268.0, 240.0, 20.0));
    }

    // 内置拼音 IME 提交的汉字 → 追加到名字（≤15 字）
    if let Some(c) = ime.take_commit() {
        if state.visible && state.name_focused {
            for ch in c.chars() {
                if state.name.chars().count() < 15 {
                    state.name.push(ch);
                }
            }
        }
    }
}

/// 名字校验边框颜色（独立系统，避免 new_char_ui_system 参数超限）：
/// 空=透明、不合法=红、合法=绿（对齐原版 NameTextBox.TextChanged 的 BorderColour）
fn new_char_name_border_system(
    state: Res<NewCharState>,
    mut borders: Query<&mut Sprite, With<NcNameBorder>>,
) {
    let color = if state.name.is_empty() {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
    } else if name_valid(&state.name) {
        Color::srgb(0.0, 1.0, 0.0)
    } else {
        Color::srgb(1.0, 0.0, 0.0)
    };
    for mut s in borders.iter_mut() {
        s.color = color;
    }
}

fn new_char_anim_system(
    mut state: ResMut<NewCharState>,
    time: Res<Time>,
    mut preview: ParamSet<(
        Query<(&mut Sprite, &mut Transform), With<NcPreview>>,
        Query<(&mut Sprite, &mut Transform), With<NcBlend>>,
    )>,
) {
    if !state.visible {
        return;
    }
    state.preview_timer += time.delta_secs();
    if state.preview_timer >= 0.25 {
        state.preview_timer = 0.0;
        state.preview_frame = (state.preview_frame + 1) % state.preview_handles.len().max(1);
        if let Ok(mut s) = preview.p0().single_mut() {
            if let Some(h) = state.preview_handles.get(state.preview_frame) {
                s.0.image = h.clone();
                let (px, py) = preview_pos(&state, state.preview_frame);
                s.1.translation.x = px;
                s.1.translation.y = -py;
            }
        }
        // 法师 blend 叠加层同步推进一帧（与主帧同帧号）
        if let Ok(mut s) = preview.p1().single_mut() {
            if let Some(h) = state.blend_handles.get(state.preview_frame) {
                s.0.image = h.clone();
                let (bx, by) = blend_pos(&state, state.preview_frame);
                s.1.translation.x = bx;
                s.1.translation.y = -by;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2970 同类隐患防线：`new_char_ui_system` 是全仓**剩下唯一**的「同函数双 ParamSet」
    /// （`preview` + `texts`）。Bevy 的 B0001 豁免只在**单个** ParamSet 内部生效，两个
    /// ParamSet 的并集一旦在某个组件上都持写访问，就会在**系统参数初始化期** panic——
    /// 商城系统正是这样「一进游戏即崩」（#2969 / #2970）。
    ///
    /// 本测试跑真实的系统初始化（不构造业务数据）：将来任何字段重叠、或有人再加一个
    /// ParamSet，都会在这里变红，而不是等上线后崩在玩家机器上。
    #[test]
    fn new_char_ui_system_initializes_without_query_conflict() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(NetConnection::default());
        world.insert_resource(NewCharState::default());
        world.insert_resource(GameLibraries::default());
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(UiImageCache::default());
        world.init_resource::<ButtonInput<MouseButton>>();
        world.insert_resource(PinyinIme::new());
        world.init_resource::<Messages<KeyboardInput>>();
        world.run_system_once(new_char_ui_system).unwrap();
    }

    /// #2892 批C：同一个对话框两种模式——英雄模式 OK 发 `C.NewHero`、
    /// 玩家模式发 `C.NewCharacter`（C# 两个实例各自的回调，`GameScene.cs:323-331`）
    #[test]
    fn submit_routes_to_hero_or_player_packet() {
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        let mut net = NetConnection::default();
        net.to_server = Some(tx);
        let mut state = NewCharState::default();
        state.name = "测试名".to_string();
        state.gender = MirGender::Female;
        state.class = MirClass::Taoist;

        state.hero_mode = true;
        submit_new_char(&net, &state);
        let raw = rx.try_recv().expect("英雄模式应发包");
        let hero: mir2_shared::packets::client::hero::NewHero =
            mir2_shared::packets::base::deserialize_packet(&mut std::io::Cursor::new(raw))
                .expect("应为 C.NewHero");
        assert_eq!(
            (hero.name.as_str(), hero.gender, hero.class),
            ("测试名", MirGender::Female, MirClass::Taoist)
        );

        state.hero_mode = false;
        submit_new_char(&net, &state);
        let raw = rx.try_recv().expect("玩家模式应发包");
        let chr: mir2_shared::packets::client::NewCharacter =
            mir2_shared::packets::base::deserialize_packet(&mut std::io::Cursor::new(raw))
                .expect("应为 C.NewCharacter");
        assert_eq!(
            (chr.name.as_str(), chr.gender, chr.class),
            ("测试名", MirGender::Female, MirClass::Taoist)
        );
    }
}
