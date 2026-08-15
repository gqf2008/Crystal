// ============================================================================
// 组队对话框（M21）
// 布局参考：C# GroupDialog.cs（背景 Prguse[120] 232x249 实测，Location = Center 居中；成员 2 列）
//   - 成员[0] (16,33)，其余 ((i+1)%2)*100+16, 55+((i-1)/2)*20
//   - 开关按钮 Prguse[114/115/116] (25,219)；关闭 Prguse2[360-362] (206,3)
//   - 标题 Title[5] (18,8)；添加 Title[133-135] (70,219)；移除 Title[136-138] (140,219)
//   - 邀请提示：MirMessageBox（Prguse[360]，Yes Title[206-208] / No Title[210-212]）
// 网络：GroupMembersMap（成员列表）→ 显示；GroupInvite（邀请）→ 提示
//       右键玩家 → C.AddMember{Name} 邀请；开关 → C.SwitchGroup；回复 → C.GroupInvite{accept}
// ============================================================================

use crate::actor::{LocalPlayer, PlayerName};
use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState, TextInputSubmit,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use bevy::prelude::*;

/// 组队成员（SharedRust 定义，与服务端 GroupMembersMap wire 一致）
pub use mir2_shared::packets::server::group::GroupMember;

/// 待处理邀请
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupInviteInfo {
    pub inviter_name: String,
    pub inviter_id: u64,
}

/// 组队状态（网络 GroupMembersMap / GroupInvite 写入）
#[derive(Resource, Default)]
pub struct GroupState {
    pub members: Vec<GroupMember>,
    pub invite: Option<GroupInviteInfo>,
    /// 是否允许组队（原版 C# GroupDialog.AllowGroup）
    pub allow_group: bool,
    /// 移除成员输入框是否打开（C# DelButton → MirInputBox）
    pub del_open: bool,
    /// #1349：按名邀请输入框是否打开（C# AddButton → MirInputBox → C.AddMember）
    pub add_open: bool,
}

/// 组队框背景索引（C# GroupDialog `Index = 120`，Prguse，AutoSize → 实测 232x249。
/// 旧代码误用 Prguse[964]——实测仅 16x18 的小图，导致按钮/成员悬浮在无背景上）
const GROUP_BG_INDEX: usize = 120;

/// 屏幕中心原点公式（C# MirControl.Center，MirControl.cs:645：
/// ((1024-W)/2, (768-H)/2) **整数除法**——用 floor 复刻截断）
fn center_origin(w: f32, h: f32) -> (f32, f32) {
    (((1024.0 - w) / 2.0).floor(), ((768.0 - h) / 2.0).floor())
}

/// 窗口原点 = 屏幕中心（C# GroupDialog.cs:27 `Location = Center`；W/H = 背景真实尺寸）
fn group_dialog_origin(libs: &mut GameLibraries) -> (f32, f32) {
    match libs.0.get_image(LibraryName::Prguse, GROUP_BG_INDEX) {
        Some(i) => center_origin(i.width.max(0) as f32, i.height.max(0) as f32),
        None => (396.0, 259.0), // 232x249 缺失时的兜底（probe 实测值代入 C# 公式）
    }
}

#[derive(Component)]
pub struct GroupWidget;

#[derive(Component)]
pub struct GroupClose;

#[derive(Component)]
pub struct GroupSwitch;

#[derive(Component)]
pub struct GroupMemberLine(usize);

/// #1349：按名邀请按钮（C# GroupDialog AddButton Title[133-135] (70,219)）
#[derive(Component)]
pub struct GroupAddBtn;

/// #1349：按名邀请输入框
#[derive(Component)]
pub struct GroupAddInput;

/// #1349：按名邀请确认按钮
#[derive(Component)]
pub struct GroupAddOk;

/// 移除成员按钮（C# GroupDialog DelButton Title[136-138] (140,219)）
#[derive(Component)]
pub struct GroupDelBtn;

/// 移除成员输入框（TextInput id 32）
#[derive(Component)]
pub struct GroupDelInput;

/// 移除确认按钮
#[derive(Component)]
pub struct GroupDelOk;

// 邀请提示组件
#[derive(Component)]
pub struct GroupInviteWidget;

#[derive(Component)]
pub struct GroupInviteText;

#[derive(Component)]
pub struct GroupInviteYes;

#[derive(Component)]
pub struct GroupInviteNo;

pub struct GroupPlugin;

impl Plugin for GroupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroupState>();
        app.add_systems(Update, group_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(OnEnter(AppState::Game), spawn_group);
        app.add_systems(OnExit(AppState::Game), cleanup_group);
        app.add_systems(
            Update,
            (
                group_ui_system,
                group_invite_system,
                group_switch_system,
                group_invite_player_system,
                group_del_system,
                group_add_system,
                ui_button_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_group(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_group(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    // 窗口原点（屏幕中心，C# Location = Center；随背景真实尺寸计算）
    let (dx, dy) = group_dialog_origin(&mut libs);

    // 背景 Prguse[120]（C# GroupDialog.Index=120；旧 964 实为 16x18 小图）
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        GROUP_BG_INDEX,
    ) {
        let e = spawn_ui_sprite(&mut commands, h, dx, dy, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Group),
            GroupWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[5]（C# GroupDialog.TitleLabel）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 5) {
        let e = spawn_ui_sprite(&mut commands, h, dx + 18.0, dy + 8.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Group),
            GroupWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        dx + 206.0,
        dy + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands
            .entity(e)
            .insert((GroupClose, DialogRoot(DialogKind::Group), GroupWidget));
    }
    // 允许组队开关（C# SwitchButton Prguse[114/115] (25,219)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse,
        114,
        115,
        116,
        dx + 25.0,
        dy + 219.0,
        7.0,
        60.0,
        23.0,
    ) {
        commands
            .entity(e)
            .insert((GroupSwitch, DialogRoot(DialogKind::Group), GroupWidget));
    }
    // #1349：按名邀请按钮（C# GroupDialog AddButton Title[133-135] (70,219)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        133,
        134,
        135,
        dx + 70.0,
        dy + 219.0,
        7.0,
        60.0,
        23.0,
    ) {
        commands
            .entity(e)
            .insert((GroupAddBtn, DialogRoot(DialogKind::Group), GroupWidget));
    }
    // 成员列表（C# GroupMembers 2 列布局）
    for i in 0..8usize {
        let (x, y) = if i == 0 {
            (16.0, 33.0)
        } else {
            (
                ((i + 1) % 2) as f32 * 100.0 + 16.0,
                55.0 + ((i - 1) / 2) as f32 * 20.0,
            )
        };
        let e = spawn_ui_text(
            &mut commands,
            &font,
            "",
            dx + x,
            dy + y,
            12.0,
            Color::WHITE,
            8.0,
        );
        commands.entity(e).insert((
            GroupMemberLine(i),
            DialogRoot(DialogKind::Group),
            GroupWidget,
        ));
    }

    // 移除成员（C# GroupDialog DelButton Title[136-138] (140,219) → MirInputBox → C.DellMember{Name}）
    let del_btn = spawn_ui_text(
        &mut commands,
        &font,
        "移除",
        dx + 140.0,
        dy + 219.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(del_btn).insert((
        GroupDelBtn,
        UiButton {
            rect: (dx + 140.0, dy + 219.0, 44.0, 22.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Group),
        GroupWidget,
    ));
    // 移除输入框（TextInput id 32）+ 确认按钮（C# MirInputBox 语义）
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let input_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Group),
            GroupWidget,
            GroupDelInput,
            TextInputField(32),
            TextInputRect(dx + 25.0, dy + 180.0, 120.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(120.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(dx + 25.0, -(dy + 180.0), 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(input_e).with_children(|p| {
        p.spawn((
            TextInputDisplay(32),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    let ok_btn = spawn_ui_text(
        &mut commands,
        &font,
        "确认",
        dx + 150.0,
        dy + 180.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(ok_btn).insert((
        GroupDelOk,
        UiButton {
            rect: (dx + 150.0, dy + 180.0, 40.0, 20.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Group),
        GroupWidget,
    ));
    // #1349：按名邀请输入框（TextInput id 33）+ 确认按钮
    let add_input = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Group),
            GroupWidget,
            GroupAddInput,
            TextInputField(33),
            TextInputRect(dx + 25.0, dy + 180.0, 120.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(120.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(dx + 25.0, -(dy + 180.0), 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(add_input).with_children(|p| {
        p.spawn((
            TextInputDisplay(33),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
    let add_ok = spawn_ui_text(
        &mut commands,
        &font,
        "确认",
        dx + 150.0,
        dy + 180.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(add_ok).insert((
        GroupAddOk,
        UiButton {
            rect: (dx + 150.0, dy + 180.0, 40.0, 20.0),
            clicked: false,
        },
        DialogRoot(DialogKind::Group),
        GroupWidget,
        Visibility::Hidden,
    ));
    // 邀请提示（MirMessageBox：Prguse[360] 居中，Yes Title[206-208] / No Title[210-212]）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands.entity(e).insert((
            GroupInviteWidget,
            DialogRoot(DialogKind::Group),
            Visibility::Hidden,
        ));
    }
    let t = spawn_ui_text(
        &mut commands,
        &font,
        "",
        bx + 35.0,
        by + 40.0,
        12.0,
        Color::WHITE,
        9.6,
    );
    commands.entity(t).insert((
        GroupInviteText,
        GroupInviteWidget,
        DialogRoot(DialogKind::Group),
    ));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        206,
        207,
        208,
        bx + 240.0,
        by + 150.0,
        9.7,
        76.0,
        25.0,
    ) {
        commands.entity(e).insert((
            GroupInviteYes,
            GroupInviteWidget,
            DialogRoot(DialogKind::Group),
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        210,
        211,
        212,
        bx + 340.0,
        by + 150.0,
        9.7,
        76.0,
        25.0,
    ) {
        commands.entity(e).insert((
            GroupInviteNo,
            GroupInviteWidget,
            DialogRoot(DialogKind::Group),
        ));
    }
}

/// 显示/隐藏 + 成员列表渲染 + 邀请提示显隐
#[allow(clippy::type_complexity)]
fn group_ui_system(
    mut mgr: ResMut<DialogManager>,
    group: Res<GroupState>,
    close: Query<&UiButton, With<GroupClose>>,
    mut widgets: Query<
        (&mut Visibility, Option<&GroupMemberLine>),
        (
            With<GroupWidget>,
            Without<GroupInviteWidget>,
            Without<GroupInviteText>,
        ),
    >,
    mut lines: Query<(&mut Text2d, &GroupMemberLine), Without<GroupInviteText>>,
    mut invite_widgets: Query<&mut Visibility, (With<GroupInviteWidget>, Without<GroupWidget>)>,
    mut invite_texts: Query<(&mut Text2d, &GroupInviteText), Without<GroupMemberLine>>,
) {
    let open = mgr.is_open(DialogKind::Group);
    for (mut vis, _line) in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        // 邀请框（GroupInviteWidget）只在 open 分支管理显隐，关闭时必须隐藏
        for mut vis in &mut invite_widgets {
            *vis = Visibility::Hidden;
        }
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Group);
        }
    }
    // 成员列表（队长/离线标记，原版 C# 语义）
    for (mut text, line) in &mut lines {
        text.0 = match group.members.get(line.0) {
            Some(m) if m.is_leader => format!("★{}", m.name),
            Some(m) if !m.online => format!("{}（离线）", m.name),
            Some(m) => m.name.clone(),
            None => String::new(),
        };
    }
    // 邀请提示
    let has_invite = group.invite.is_some();
    for mut vis in &mut invite_widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut invite_texts {
        text.0 = match group.invite.as_ref() {
            Some(inv) => format!("{} 邀请你加入组队", inv.inviter_name),
            None => String::new(),
        };
    }
}

/// 邀请提示：Yes/No → C.GroupInvite{accept}
fn group_invite_system(
    mut group: ResMut<GroupState>,
    net: Res<NetConnection>,
    yes: Query<&UiButton, With<GroupInviteYes>>,
    no: Query<&UiButton, With<GroupInviteNo>>,
) {
    if group.invite.is_none() {
        return;
    }
    let mut accept: Option<bool> = None;
    for btn in &yes {
        if btn.clicked {
            accept = Some(true);
        }
    }
    for btn in &no {
        if btn.clicked {
            accept = Some(false);
        }
    }
    if let Some(a) = accept {
        net.send_packet(&mir2_shared::packets::client::group::GroupInvite { accept_invite: a });
        tracing::info!(
            "👥 组队邀请回复: accept={} (来自 {})",
            a,
            group
                .invite
                .as_ref()
                .map(|i| i.inviter_name.as_str())
                .unwrap_or("?")
        );
        group.invite = None;
    }
}

/// 允许组队开关 → C.SwitchGroup{allow_group}
fn group_switch_system(
    mut group: ResMut<GroupState>,
    net: Res<NetConnection>,
    btns: Query<&UiButton, With<GroupSwitch>>,
) {
    for btn in &btns {
        if btn.clicked {
            group.allow_group = !group.allow_group;
            net.send_packet(&mir2_shared::packets::client::group::SwitchGroup {
                allow_group: group.allow_group,
            });
            tracing::info!("👥 允许组队: {}", group.allow_group);
        }
    }
}

/// 右键点击远端玩家 → 组队邀请（原版 C# MainDialogs 右键玩家 → 组队邀请 → C.AddMember{Name}）
#[allow(clippy::too_many_arguments)]
fn group_invite_player_system(
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<&Transform, With<Camera2d>>,
    remote_players: Query<
        (&crate::actor::PlayerName, &Transform),
        (
            Without<crate::actor::LocalPlayer>,
            With<crate::actor::NetObjectId>,
        ),
    >,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(cam) = camera.single() else { return };
    // 屏幕坐标 → 世界坐标（原版 C# 点击玩家判定）
    let world = crate::game::player_control::screen_to_world(cursor, cam, &window);
    let mut target: Option<String> = None;
    for (name, tf) in &remote_players {
        if (tf.translation.x - world.x).abs() < 24.0 && (tf.translation.y - world.y).abs() < 24.0 {
            target = Some(name.0.clone());
        }
    }
    if let Some(name) = target {
        net.send_packet(&mir2_shared::packets::client::group::AddMember { name: name.clone() });
        tracing::info!("👥 邀请组队: {}", name);
    }
}

/// 移除成员（C# DelButton → MirInputBox → C.DellMember{Name}；仅队长可见可用）
#[allow(clippy::too_many_arguments)]
fn group_del_system(
    mut mgr: ResMut<DialogManager>,
    mut group: ResMut<GroupState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut submit: MessageReader<TextInputSubmit>,
    del_btn: Query<&UiButton, With<GroupDelBtn>>,
    ok_btn: Query<&UiButton, With<GroupDelOk>>,
    local_player: Query<&PlayerName, With<LocalPlayer>>,
    // #1290：Bevy B0001——三个 &mut Visibility Query 需 Without 隔离（#1288 组队踢人合并后启动 panic）
    mut del_btn_vis: Query<
        &mut Visibility,
        (
            With<GroupDelBtn>,
            Without<GroupDelInput>,
            Without<GroupDelOk>,
        ),
    >,
    mut input_vis: Query<
        &mut Visibility,
        (
            With<GroupDelInput>,
            Without<GroupDelBtn>,
            Without<GroupDelOk>,
        ),
    >,
    mut ok_vis: Query<
        &mut Visibility,
        (
            With<GroupDelOk>,
            Without<GroupDelBtn>,
            Without<GroupDelInput>,
        ),
    >,
) {
    let open = mgr.is_open(DialogKind::Group);
    if !open {
        group.del_open = false;
        if input.texts.len() > 32 {
            input.texts[32].clear();
        }
        return;
    }
    let self_name = local_player
        .single()
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let is_leader = group
        .members
        .first()
        .map(|m| m.name == self_name)
        .unwrap_or(false);
    // 非队长隐藏移除按钮（C# GroupPanel_BeforeDraw：非队长 Add/Del 不可见）
    for mut vis in del_btn_vis.iter_mut() {
        *vis = if is_leader {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !is_leader {
        group.del_open = false;
    }
    for mut vis in input_vis.iter_mut() {
        *vis = if group.del_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in ok_vis.iter_mut() {
        *vis = if group.del_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 点击移除 → 打开输入框
    for btn in &del_btn {
        if btn.clicked && is_leader {
            group.del_open = true;
            input.active = Some(32);
        }
    }
    // 确认 / Enter → 发送 DellMember{Name}
    let mut confirmed = false;
    for btn in &ok_btn {
        if btn.clicked {
            confirmed = true;
        }
    }
    for s in submit.read() {
        if s.0 == 32 {
            confirmed = true;
        }
    }
    if confirmed && group.del_open {
        let name = input.texts.get(32).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if !name.is_empty() {
            net.send_packet(&mir2_shared::packets::client::group::DellMember {
                name: name.clone(),
            });
            tracing::info!("👥 移除成员: {}", name);
        }
        group.del_open = false;
        if input.texts.len() > 32 {
            input.texts[32].clear();
        }
    }
}

/// #1349：按名邀请（C# AddButton → MirInputBox → C.AddMember{Name}；仅队长可见可用）
#[allow(clippy::too_many_arguments)]
fn group_add_system(
    mut mgr: ResMut<DialogManager>,
    mut group: ResMut<GroupState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut submit: MessageReader<TextInputSubmit>,
    add_btn: Query<&UiButton, With<GroupAddBtn>>,
    ok_btn: Query<&UiButton, With<GroupAddOk>>,
    local_player: Query<&PlayerName, With<LocalPlayer>>,
    // #1290：Bevy B0001——三个 &mut Visibility Query 需 Without 隔离
    mut add_btn_vis: Query<
        &mut Visibility,
        (
            With<GroupAddBtn>,
            Without<GroupAddInput>,
            Without<GroupAddOk>,
        ),
    >,
    mut input_vis: Query<
        &mut Visibility,
        (
            With<GroupAddInput>,
            Without<GroupAddBtn>,
            Without<GroupAddOk>,
        ),
    >,
    mut ok_vis: Query<
        &mut Visibility,
        (
            With<GroupAddOk>,
            Without<GroupAddBtn>,
            Without<GroupAddInput>,
        ),
    >,
) {
    let open = mgr.is_open(DialogKind::Group);
    if !open {
        group.add_open = false;
        if input.texts.len() > 33 {
            input.texts[33].clear();
        }
        return;
    }
    let self_name = local_player
        .single()
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let is_leader = group
        .members
        .first()
        .map(|m| m.name == self_name)
        .unwrap_or(false);
    // 非队长隐藏邀请按钮（C# GroupPanel_BeforeDraw：非队长 Add/Del 不可见）
    for mut vis in add_btn_vis.iter_mut() {
        *vis = if is_leader {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !is_leader {
        group.add_open = false;
    }
    for mut vis in input_vis.iter_mut() {
        *vis = if group.add_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in ok_vis.iter_mut() {
        *vis = if group.add_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 点击邀请 → 打开输入框
    for btn in &add_btn {
        if btn.clicked && is_leader {
            group.add_open = true;
            input.active = Some(33);
        }
    }
    // 确认 / Enter → 发送 AddMember{Name}
    let mut confirmed = false;
    for btn in &ok_btn {
        if btn.clicked {
            confirmed = true;
        }
    }
    for s in submit.read() {
        if s.0 == 33 {
            confirmed = true;
        }
    }
    if confirmed && group.add_open {
        let name = input.texts.get(33).cloned().unwrap_or_default();
        let name = name.trim().to_string();
        if !name.is_empty() {
            net.send_packet(&mir2_shared::packets::client::group::AddMember { name: name.clone() });
            tracing::info!("👥 按名邀请组队: {}", name);
        }
        group.add_open = false;
        if input.texts.len() > 33 {
            input.texts[33].clear();
        }
    }
}

/// 消费服务端组队事件（网络层只广播 ServerEvent）
fn group_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut group: ResMut<GroupState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::GroupMembers { members } => {
                group.members = members.clone();
            }
            ServerEvent::GroupInvite {
                inviter_name,
                inviter_id,
            } => {
                group.invite = Some(GroupInviteInfo {
                    inviter_name: inviter_name.clone(),
                    inviter_id: *inviter_id,
                });
            }
            ServerEvent::GroupDeleted => {
                group.members.clear();
                group.invite = None;
            }
            ServerEvent::GroupMemberLeft { name } => {
                group.members.retain(|m| m.name != *name);
            }
            ServerEvent::GroupAllowChanged { allow_group } => {
                group.allow_group = *allow_group;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# GroupDialog `Location = Center`（GroupDialog.cs:27）+ MirControl.Center 整除
    /// （MirControl.cs:645）。背景 Prguse[120] 实测 232x249 → 原点 (396,259)；
    /// 奇数尺寸验证整数除法截断：(768-249)/2 = 259（非 259.5）。
    #[test]
    fn group_origin_is_screen_center() {
        assert_eq!(center_origin(232.0, 249.0), (396.0, 259.0));
        // 奇数宽：C# int 除法截断 → floor
        assert_eq!(center_origin(231.0, 249.0), (396.0, 259.0));
        assert_eq!(center_origin(1024.0, 768.0), (0.0, 0.0));
        assert_eq!(GROUP_BG_INDEX, 120, "背景索引 = C# GroupDialog.Index=120");
    }
}
