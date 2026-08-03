// ============================================================================
// SelectPlugin - 角色选择（Sprite 精确坐标版，对齐 macroquad SelectScene）
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::modal_box::{spawn_modal_box, ModalKind, ModalState};
use crate::ui::new_character::{spawn_new_character_dialog, NewCharState};
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_camera, spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image,
    UiButton, UiEntity, UiFont, UiImageCache,
};

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectAnim>();
        app.init_resource::<UiImageCache>();
        app.init_resource::<UiFont>();
        app.add_systems(OnEnter(AppState::Select), setup_select_ui);
        app.add_systems(OnExit(AppState::Select), cleanup_select_ui);
        app.add_systems(
            Update,
            (
                select_ui_system,
                select_anim_system,
                ui_button_system,
                select_reload_system,
                auto_create_system,
            )
                .run_if(in_state(AppState::Select)),
        );
    }
}

#[derive(Resource, Default)]
pub struct SelectAnim {
    pub preview_frame: usize,
    pub preview_timer: f32,
    pub preview_handles: Vec<Handle<Image>>,
    /// 每帧 ChrSel 的 offset_x/offset_y（对齐 C# UseOffSet=true）
    pub preview_offsets: Vec<(f32, f32)>,
    /// 法师 blend 叠加层 16 帧（ChrSel[frame+560]；对齐 C# SelectScene AfterDraw DrawBlend）
    pub blend_handles: Vec<Handle<Image>>,
    pub blend_offsets: Vec<(f32, f32)>,
}

#[derive(Component)]
struct PreviewImg;

/// 法师 blend 叠加层精灵（对齐 C# CharacterDisplay.AfterDraw:
/// ChrSel.DrawBlend(Index+560, DisplayLocationWithoutOffSet, White, offSet=true)）
#[derive(Component)]
struct PreviewBlend;

#[derive(Component)]
struct CharButton {
    index: i32,
    /// 职业槽位（0..4，对应 Title[660+slot] / 选中 +5）
    slot: usize,
    rect: (f32, f32, f32, f32),
}

/// 最近登录时间文本（随选中角色更新）
#[derive(Component)]
struct LastAccessText;

#[derive(Clone, Copy, PartialEq)]
enum BottomBtn {
    Start,
    NewChar,
    Delete,
    Credits,
    Exit,
}

#[derive(Component)]
struct BottomButton(BottomBtn);

/// 角色预览（对齐 C# CharacterDisplay: Location=(260,420), UseOffSet=true, 16帧, 250ms）
/// 注意：C# MirAnimatedControl 无缩放（1F），预览不要放大
const PREVIEW_X: f32 = 260.0;
const PREVIEW_Y: f32 = 420.0;
const PREVIEW_SCALE: f32 = 1.0;
const PREVIEW_FRAMES: usize = 16;

/// 加载某职业/性别的 16 帧预览 + 每帧偏移（C# MirAnimatedControl 每帧按 offset 绘制）
fn load_preview(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    anim: &mut SelectAnim,
    class: mir2_shared::MirClass,
    gender: mir2_shared::MirGender,
) {
    anim.preview_handles.clear();
    anim.preview_offsets.clear();
    let base = preview_base_index(class, gender);
    for i in 0..PREVIEW_FRAMES {
        let idx = base + i;
        let offsets = libs
            .0
            .get_image(LibraryName::ChrSel, idx)
            .map(|info| (info.offset_x as f32, info.offset_y as f32));
        if let Some(h) = ui_image(libs, images, cache, LibraryName::ChrSel, idx) {
            anim.preview_handles.push(h);
            anim.preview_offsets.push(offsets.unwrap_or((0.0, 0.0)));
        }
    }
    // 法师 blend 叠加层（ChrSel[frame+560]）。非法师职业该段为空占位（4x1），仅 Wizard 有内容。
    anim.blend_handles.clear();
    anim.blend_offsets.clear();
    for i in 0..PREVIEW_FRAMES {
        let idx = base + i + 560;
        let boff = libs
            .0
            .get_image(LibraryName::ChrSel, idx)
            .map(|info| (info.offset_x as f32, info.offset_y as f32));
        if let Some(h) = ui_image(libs, images, cache, LibraryName::ChrSel, idx) {
            anim.blend_handles.push(h);
            anim.blend_offsets.push(boff.unwrap_or((0.0, 0.0)));
        }
    }
}

/// 计算第 frame 帧的屏幕坐标（Location + offset * scale）
fn preview_pos(anim: &SelectAnim, frame: usize) -> (f32, f32) {
    let (ox, oy) = anim
        .preview_offsets
        .get(frame)
        .copied()
        .unwrap_or((0.0, 0.0));
    (
        PREVIEW_X + ox * PREVIEW_SCALE,
        PREVIEW_Y + oy * PREVIEW_SCALE,
    )
}

/// blend 叠加层屏幕坐标（= Location + blend 精灵自身 offset，与主帧同 Location、各自 offset）
fn blend_pos(anim: &SelectAnim, frame: usize) -> (f32, f32) {
    let (ox, oy) = anim
        .blend_offsets
        .get(frame)
        .copied()
        .unwrap_or((0.0, 0.0));
    (
        PREVIEW_X + ox * PREVIEW_SCALE,
        PREVIEW_Y + oy * PREVIEW_SCALE,
    )
}

fn preview_base_index(class: mir2_shared::MirClass, gender: mir2_shared::MirGender) -> usize {
    let g = match gender {
        mir2_shared::MirGender::Female => 1usize,
        _ => 0,
    };
    match class {
        mir2_shared::MirClass::Archer => {
            if g == 0 {
                100
            } else {
                140
            }
        }
        _ => 20 + (class as usize * 20) + (g * 280),
    }
}

fn class_slot(c: &mir2_shared::SelectInfo) -> usize {
    match c.class {
        mir2_shared::MirClass::Warrior => 0usize,
        mir2_shared::MirClass::Wizard => 1,
        mir2_shared::MirClass::Taoist => 2,
        mir2_shared::MirClass::Assassin => 3,
        mir2_shared::MirClass::Archer => 4,
    }
}

fn setup_select_ui(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut anim: ResMut<SelectAnim>,
    mut cache: ResMut<UiImageCache>,
    mut ui_font: ResMut<UiFont>,
    mut new_char: ResMut<NewCharState>,
    mut modal: ResMut<ModalState>,
    mut net: ResMut<NetworkContext>,
) {
    new_char.visible = false;
    new_char.name.clear();
    // 调试：BEVY_OPEN_NEWCHAR=1 启动即打开新建角色对话框（live 截屏验证用）
    if std::env::var("BEVY_OPEN_NEWCHAR").as_deref() == Ok("1") {
        new_char.visible = true;
        new_char.name_focused = true;
    }
    // 调试：BEVY_SELECT_INDEX=n 预选第 n 个角色（live 截屏验证预览用）
    if let Ok(v) = std::env::var("BEVY_SELECT_INDEX") {
        if let Ok(i) = v.parse::<i32>() {
            net.selected_index = Some(i);
        }
    }
    // 调试：BEVY_OPEN_MODAL=delete/delete_confirm 打开对应模态框（live 截屏验证用）
    match std::env::var("BEVY_OPEN_MODAL").as_deref() {
        Ok("delete") => modal.kind = ModalKind::DeleteAsk,
        Ok("delete_confirm") => modal.kind = ModalKind::DeleteConfirm,
        _ => {}
    }
    build_select_ui(
        &mut commands,
        &mut libs,
        &mut images,
        &mut fonts,
        &mut anim,
        &mut cache,
        &mut ui_font,
        &net,
        &mut new_char,
        &mut modal,
    );
}

#[allow(clippy::too_many_arguments)]
fn build_select_ui(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    fonts: &mut Assets<Font>,
    anim: &mut SelectAnim,
    cache: &mut UiImageCache,
    ui_font: &mut UiFont,
    net: &NetworkContext,
    new_char: &mut NewCharState,
    _modal: &mut ModalState,
) {
    libs.0.ensure_initialized();
    ui_font.0 = crate::ui::sprite_ui::load_ui_font(fonts);
    let font = ui_font.0.clone();
    spawn_ui_camera(commands.reborrow());

    // 背景 Prguse[65]（1024x768）
    if let Some(h) = ui_image(
        &mut *libs,
        &mut *images,
        &mut *cache,
        LibraryName::Prguse,
        65,
    ) {
        spawn_ui_sprite(&mut *commands, h, 0.0, 0.0, 0.0, 1.0);
    }
    // 标题 Title[40]
    if let Some(h) = ui_image(
        &mut *libs,
        &mut *images,
        &mut *cache,
        LibraryName::Title,
        40,
    ) {
        spawn_ui_sprite(&mut *commands, h, 468.0, 20.0, 1.0, 1.0);
    }
    // 服务器名（原版 ServerLabel (432,60) 155x17 居中；Parent=Background 在 (0,0)）
    spawn_ui_text(
        &mut *commands,
        &font,
        "Legend of Mir 2",
        432.0,
        60.0,
        17.0,
        Color::WHITE,
        2.0,
    );

    // 角色预览（初始）
    // 角色预览（初始，带帧偏移，对齐 C# UseOffSet=true；优先选中的角色）
    let preview_char = net
        .selected_index
        .and_then(|i| net.characters.iter().find(|c| c.index == i))
        .or_else(|| net.characters.first());
    if let Some(c) = preview_char {
        load_preview(
            &mut *libs,
            &mut *images,
            &mut *cache,
            &mut *anim,
            c.class,
            c.gender,
        );
    }
    if let Some(pv) = anim.preview_handles.first().cloned() {
        let (px, py) = preview_pos(&anim, 0);
        let e = spawn_ui_sprite(&mut *commands, pv, px, py, 3.0, PREVIEW_SCALE);
        commands.entity(e).insert(PreviewImg);
    }
    // 法师 blend 叠加层（z=3.1 略高于预览；仅 Wizard 有内容，其余为 4x1 空占位）
    if let Some(bv) = anim.blend_handles.first().cloned() {
        let (bx, by) = blend_pos(&anim, 0);
        let e = spawn_ui_sprite(&mut *commands, bv, bx, by, 3.1, PREVIEW_SCALE);
        commands.entity(e).insert(PreviewBlend);
    }
    // 角色信息（对齐原版：LastAccessLabel 值 (265,609) 180x21，
    // LastAccessLabelLabel 标题 "最后登录:" 是其子控件 (-65,0) → 绝对 (200,609) 100x21）
    spawn_ui_text(
        &mut *commands,
        &font,
        "最后登录:",
        200.0,
        609.0,
        12.0,
        Color::WHITE,
        2.0,
    );
    if let Some(c) = net.characters.first() {
        let e = spawn_ui_text(
            &mut *commands,
            &font,
            &c.last_access.format("%Y/%m/%d %H:%M:%S").to_string(),
            265.0,
            609.0,
            12.0,
            Color::WHITE,
            2.0,
        );
        commands.entity(e).insert(LastAccessText);
    }

    // 角色按钮（4 槽位，对齐 C# CharacterButton：选中帧 660+class+5）
    let positions = [
        (637.0f32, 194.0f32),
        (637.0, 298.0),
        (637.0, 402.0),
        (637.0, 506.0),
    ];
    for (i, (x, y)) in positions.iter().enumerate() {
        if let Some(c) = net.characters.get(i) {
            let slot = class_slot(c);
            let selected = net.selected_index == Some(i as i32);
            let frame = if selected { slot + 5 } else { slot };
            if let Some(h) = ui_image(
                &mut *libs,
                &mut *images,
                &mut *cache,
                LibraryName::Title,
                660 + frame,
            ) {
                let e = spawn_ui_sprite(&mut *commands, h, *x, *y, 2.0, 1.0);
                commands.entity(e).insert(CharButton {
                    index: i as i32,
                    slot,
                    // 原版 CharacterButton: MirImageControl AutoSize → 命中框 = 精灵尺寸 Title[660]=288x56
                    rect: (*x, *y, 288.0, 56.0),
                });
            }
            // 名字/Lv/职业
            let class_name = match c.class {
                mir2_shared::MirClass::Warrior => "战士",
                mir2_shared::MirClass::Wizard => "法师",
                mir2_shared::MirClass::Taoist => "道士",
                mir2_shared::MirClass::Assassin => "刺客",
                mir2_shared::MirClass::Archer => "弓手",
            };
            spawn_ui_text(
                &mut *commands,
                &font,
                &c.name,
                x + 107.0,
                y + 9.0,
                12.0,
                Color::WHITE,
                3.0,
            );
            spawn_ui_text(
                &mut *commands,
                &font,
                &c.level.to_string(),
                x + 107.0,
                y + 28.0,
                11.0,
                Color::WHITE,
                3.0,
            );
            spawn_ui_text(
                &mut *commands,
                &font,
                class_name,
                x + 178.0,
                y + 28.0,
                11.0,
                Color::WHITE,
                3.0,
            );
        } else {
            // 空槽位 Prguse[44]
            if let Some(h) = ui_image(
                &mut *libs,
                &mut *images,
                &mut *cache,
                LibraryName::Prguse,
                44,
            ) {
                spawn_ui_sprite(&mut *commands, h, *x, *y, 2.0, 1.0);
            }
        }
    }

    // 底部按钮（三态帧，对齐原版 SelectScene）
    // 原版用整数：xPoint=(1024-200)/5=164，btnX(N)=100+164*N-82-50 = 164*N-32
    // → 132 / 296 / 460 / 624 / 788，y=ScreenHeight-32=736
    let bottom_xs: [f32; 5] = [132.0, 296.0, 460.0, 624.0, 788.0];
    let y = 768.0 - 32.0;
    spawn_bottom_btn(
        &mut *commands,
        &mut *libs,
        &mut *images,
        &mut *cache,
        340,
        bottom_xs[0],
        y,
        BottomBtn::Start,
    );
    spawn_bottom_btn(
        &mut *commands,
        &mut *libs,
        &mut *images,
        &mut *cache,
        343,
        bottom_xs[1],
        y,
        BottomBtn::NewChar,
    );
    spawn_bottom_btn(
        &mut *commands,
        &mut *libs,
        &mut *images,
        &mut *cache,
        346,
        bottom_xs[2],
        y,
        BottomBtn::Delete,
    );
    spawn_bottom_btn(
        &mut *commands,
        &mut *libs,
        &mut *images,
        &mut *cache,
        349,
        bottom_xs[3],
        y,
        BottomBtn::Credits,
    );
    spawn_bottom_btn(
        &mut *commands,
        &mut *libs,
        &mut *images,
        &mut *cache,
        352,
        bottom_xs[4],
        y,
        BottomBtn::Exit,
    );

    // 新建角色对话框（隐藏，点“新建角色”打开）
    spawn_new_character_dialog(commands, libs, images, cache, &font, new_char);
    // 通用模态框（删除确认 / Credits）
    spawn_modal_box(commands, libs, images, cache, &font);
}
fn spawn_bottom_btn(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    index: usize,
    x: f32,
    y: f32,
    kind: BottomBtn,
) {
    if let Some(e) = spawn_ui_button(
        commands,
        libs,
        images,
        cache,
        LibraryName::Title,
        index,
        index + 1,
        index + 2,
        x,
        y,
        2.0,
        100.0,
        25.0,
    ) {
        commands.entity(e).insert(BottomButton(kind));
    }
}

fn cleanup_select_ui(mut commands: Commands, root: Query<Entity, With<UiEntity>>) {
    for e in root.iter() {
        commands.entity(e).despawn();
    }
}

/// 新建角色成功后重建整个选角 UI（角色槽位增加）
#[allow(clippy::too_many_arguments)]
fn select_reload_system(
    mut commands: Commands,
    mut net: ResMut<NetworkContext>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut anim: ResMut<SelectAnim>,
    mut cache: ResMut<UiImageCache>,
    mut ui_font: ResMut<UiFont>,
    mut new_char: ResMut<NewCharState>,
    mut modal: ResMut<ModalState>,
    ui_entities: Query<Entity, With<UiEntity>>,
) {
    if !net.select_reload {
        return;
    }
    net.select_reload = false;
    tracing::info!("[SELECT] 重建选角 UI（角色数={}）", net.characters.len());
    for e in ui_entities.iter() {
        commands.entity(e).despawn();
    }
    new_char.visible = false;
    new_char.name.clear();
    // 调试：BEVY_OPEN_NEWCHAR=1 启动即打开新建角色对话框（live 截屏验证用）
    if std::env::var("BEVY_OPEN_NEWCHAR").as_deref() == Ok("1") {
        new_char.visible = true;
        new_char.name_focused = true;
    }
    // 调试：BEVY_SELECT_INDEX=n 预选第 n 个角色（live 截屏验证预览用）
    if let Ok(v) = std::env::var("BEVY_SELECT_INDEX") {
        if let Ok(i) = v.parse::<i32>() {
            net.selected_index = Some(i);
        }
    }
    // 调试：BEVY_OPEN_MODAL=delete/delete_confirm 打开对应模态框（live 截屏验证用）
    match std::env::var("BEVY_OPEN_MODAL").as_deref() {
        Ok("delete") => modal.kind = ModalKind::DeleteAsk,
        Ok("delete_confirm") => modal.kind = ModalKind::DeleteConfirm,
        _ => {}
    }
    build_select_ui(
        &mut commands,
        &mut libs,
        &mut images,
        &mut fonts,
        &mut anim,
        &mut cache,
        &mut ui_font,
        &net,
        &mut new_char,
        &mut modal,
    );
}

fn select_ui_system(
    mut net: ResMut<NetworkContext>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut anim: ResMut<SelectAnim>,
    mut cache: ResMut<UiImageCache>,
    mut new_char: ResMut<NewCharState>,
    mut modal: ResMut<ModalState>,
    mut char_btns: Query<(&CharButton, &mut Sprite), (Without<PreviewImg>, Without<PreviewBlend>)>,
    mut last_access_texts: Query<&mut Text2d, With<LastAccessText>>,
    bottom: Query<(&UiButton, &BottomButton)>,
    // p0=主预览帧；p1=法师 blend 叠加层（均在 CharButton 之外）
    mut preview: ParamSet<(
        Query<(&mut Sprite, &mut Transform), (With<PreviewImg>, Without<CharButton>)>,
        Query<(&mut Sprite, &mut Transform), (With<PreviewBlend>, Without<CharButton>)>,
    )>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    // 鼠标状态（自算，不依赖 ui_button_system 的执行顺序）
    let (mx, my) = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    let lclick = mouse.just_pressed(MouseButton::Left);

    // 桥接网络层的新建角色错误提示
    if let Some(err) = net.character_error.take() {
        new_char.error = Some(err);
        new_char.visible = true;
        new_char.name_focused = true;
    }

    // 角色按钮：点击选择 + 悬停/选中帧 + 高亮边框
    let mut selection_changed = false;
    for (cb, mut sprite) in char_btns.iter_mut() {
        let (x, y, w, h) = cb.rect;
        let hovered = mx >= x && mx <= x + w && my >= y && my <= y + h;
        let selected = net.selected_index == Some(cb.index);
        if lclick && hovered && !selected {
            net.selected_index = Some(cb.index);
            selection_changed = true;
        }
        // 选中或悬停都显示高亮帧（原版选中帧 660+slot+5）
        let frame = if selected || hovered {
            cb.slot + 5
        } else {
            cb.slot
        };
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Title,
            660 + frame,
        ) {
            if sprite.image != h {
                sprite.image = h;
            }
        }
    }
    // 边框：原版仅靠选中/悬停的精灵帧（Title[660+slot+5]）表示高亮，无自绘边框
    if selection_changed {
        if let Some(c) = net
            .selected_index
            .and_then(|i| net.characters.iter().find(|c| c.index == i))
        {
            load_preview(
                &mut libs,
                &mut images,
                &mut cache,
                &mut anim,
                c.class,
                c.gender,
            );
            anim.preview_frame = 0;
            if let Ok(mut s) = preview.p0().single_mut() {
                if let Some(h) = anim.preview_handles.first() {
                    s.0.image = h.clone();
                    let (px, py) = preview_pos(&anim, 0);
                    s.1.translation.x = px;
                    s.1.translation.y = -py;
                }
            }
            // 法师 blend 叠加层复位到第 0 帧（仅 Wizard 有内容；其余为 4x1 空占位）
            if let Ok(mut s) = preview.p1().single_mut() {
                if let Some(h) = anim.blend_handles.first() {
                    s.0.image = h.clone();
                    let (bx, by) = blend_pos(&anim, 0);
                    s.1.translation.x = bx;
                    s.1.translation.y = -by;
                }
            }
            if let Ok(mut t) = last_access_texts.single_mut() {
                t.0 = c.last_access.format("%Y/%m/%d %H:%M:%S").to_string();
            }
        }
    }
    for (btn, bb) in bottom.iter() {
        let (x, y, w, h) = btn.rect;
        let over = mx >= x && mx <= x + w && my >= y && my <= y + h;
        if lclick && over {
            match bb.0 {
                BottomBtn::Start => {
                    if let Some(idx) = net.selected_index {
                        net.send_packet(&mir2_shared::packets::client::account::StartGame {
                            character_index: idx,
                        });
                    }
                }
                BottomBtn::Exit => std::process::exit(0),
                BottomBtn::NewChar => {
                    modal.kind = ModalKind::None;
                    new_char.visible = true;
                    new_char.name.clear();
                    new_char.name_focused = true;
                }
                BottomBtn::Delete => {
                    new_char.visible = false;
                    modal.kind = ModalKind::DeleteAsk;
                    modal.name_input.clear();
                    modal.error = None;
                }
                BottomBtn::Credits => {
                    // 原版 SelectScene 的 CreditsButton.Click 为空 → 不做任何弹窗
                    new_char.visible = false;
                    modal.kind = ModalKind::None;
                    modal.error = None;
                }
            }
        }
    }
}

/// 调试：BEVY_AUTO_CREATE=1 进入选角后自动创建角色（验证 新建→列表刷新 链路）
fn auto_create_system(net: ResMut<NetworkContext>, mut done: Local<bool>) {
    if *done {
        return;
    }
    if std::env::var("BEVY_AUTO_CREATE").as_deref() != Ok("1") {
        return;
    }
    if net.characters.is_empty() {
        return;
    }
    *done = true;
    tracing::info!("[AUTO_CREATE] 自动创建角色");
    net.send_packet(&mir2_shared::packets::client::NewCharacter {
        name: "自动创建".to_string(),
        gender: mir2_shared::MirGender::Male,
        class: mir2_shared::MirClass::Taoist,
    });
}

fn select_anim_system(
    mut anim: ResMut<SelectAnim>,
    time: Res<Time>,
    mut preview: ParamSet<(
        Query<(&mut Sprite, &mut Transform), With<PreviewImg>>,
        Query<(&mut Sprite, &mut Transform), With<PreviewBlend>>,
    )>,
) {
    anim.preview_timer += time.delta_secs();
    if anim.preview_timer >= 0.25 {
        anim.preview_timer = 0.0;
        anim.preview_frame = (anim.preview_frame + 1) % anim.preview_handles.len().max(1);
        if let Ok(mut s) = preview.p0().single_mut() {
            if let Some(h) = anim.preview_handles.get(anim.preview_frame) {
                s.0.image = h.clone();
                // 对齐 C#：每帧按自身 offset 绘制（世界坐标 y 取反）
                let (px, py) = preview_pos(&anim, anim.preview_frame);
                s.1.translation.x = px;
                s.1.translation.y = -py;
            }
        }
        // 法师 blend 叠加层同步推进一帧（与主帧同帧号）
        if let Ok(mut s) = preview.p1().single_mut() {
            if let Some(h) = anim.blend_handles.get(anim.preview_frame) {
                s.0.image = h.clone();
                let (bx, by) = blend_pos(&anim, anim.preview_frame);
                s.1.translation.x = bx;
                s.1.translation.y = -by;
            }
        }
    }
}
