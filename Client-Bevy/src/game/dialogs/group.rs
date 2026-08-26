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
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
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

/// bevy_ui 成员行文本子节点
#[derive(Component)]
pub struct GroupMemberText(usize);

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
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let (dx, dy) = group_dialog_origin(&mut libs);

    // 面板 Prguse[120]（232x249 @ 居中）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, GROUP_BG_INDEX) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, dx, dy, 232.0, 249.0, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Group), GroupWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[5] @(18,8)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 5) {
            spawn_image(p, h, 18.0, 8.0, 57.0, 15.0, 9);
        }
        // 关闭 Prguse2[360/361/362] @(206,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 206.0, 3.0, 20.0, 20.0, 10).insert(GroupClose);
        }
        // 允许组队开关 Prguse[114/115/116] @(25,219)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 114),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 115),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 116),
        ) {
            spawn_icon_button(p, n, h, pr, 25.0, 219.0, 60.0, 23.0, 10).insert(GroupSwitch);
        }
        // 按名邀请 Title[133/134/135] @(70,219)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 133),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 134),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 135),
        ) {
            spawn_icon_button(p, n, h, pr, 70.0, 219.0, 60.0, 23.0, 10).insert(GroupAddBtn);
        }
        // 移除成员按钮 Title[136/137/138] @(140,219)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 136),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 137),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 138),
        ) {
            spawn_icon_button(p, n, h, pr, 140.0, 219.0, 44.0, 22.0, 10).insert(GroupDelBtn);
        }
        // 成员列表（C# GroupMembers 2 列布局，TextInput 行可点击 Button + 文本）
        for i in 0..8usize {
            let (x, y) = if i == 0 {
                (16.0, 33.0)
            } else {
                (
                    ((i + 1) % 2) as f32 * 100.0 + 16.0,
                    55.0 + ((i - 1) / 2) as f32 * 20.0,
                )
            };
            spawn_container(p, x, y, 100.0, 18.0, 9)
                .insert((GroupMemberLine(i),))
                .with_children(|rc| {
                    rc.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        Text::new(String::new()),
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ZIndex(10),
                        GroupMemberText(i),
                    ));
                });
        }
        // 移除输入框（TextInput id 32）+ 确认
        spawn_container(p, 25.0, 180.0, 120.0, 20.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GroupDelInput,
                crate::game::dialogs::text_input::TextInputField(32),
                crate::game::dialogs::text_input::TextInputRect(dx + 25.0, dy + 180.0, 120.0, 20.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    crate::game::dialogs::text_input::TextInputDisplay(32),
                ));
            });
        spawn_label(p, &font, "确认", 150.0, 180.0, 12.0, Color::WHITE, 10)
            .insert((Button, GroupDelOk));
        // 邀请输入框（TextInput id 33）+ 确认
        spawn_container(p, 25.0, 180.0, 120.0, 20.0, 10)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GroupAddInput,
                crate::game::dialogs::text_input::TextInputField(33),
                crate::game::dialogs::text_input::TextInputRect(dx + 25.0, dy + 180.0, 120.0, 20.0),
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(4.0),
                        top: Val::Px(2.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(11),
                    crate::game::dialogs::text_input::TextInputDisplay(33),
                ));
            });
        spawn_label(p, &font, "确认", 150.0, 180.0, 12.0, Color::WHITE, 10)
            .insert((Button, GroupAddOk));
    });

    // 邀请提示（MirMessageBox：Prguse[360] @(284,289) 独立根节点）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let inv = spawn_panel(&mut commands, h, bx, by, 260.0, 120.0, 45);
        commands.entity(inv).insert((DialogRoot(DialogKind::Group), GroupInviteWidget));
        commands.entity(inv).with_children(|ip| {
            spawn_label(ip, &font, "", 35.0, 40.0, 12.0, Color::WHITE, 9)
                .insert(GroupInviteText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(ip, n, h, pr, 240.0, 150.0, 76.0, 25.0, 10)
                    .insert(GroupInviteYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(ip, n, h, pr, 340.0, 150.0, 76.0, 25.0, 10)
                    .insert(GroupInviteNo);
            }
        });
    }
}

/// 显示/隐藏 + 成员列表渲染 + 邀请提示显隐
#[allow(clippy::type_complexity)]
fn group_ui_system(
    mut mgr: ResMut<DialogManager>,
    group: Res<GroupState>,
    close: Query<(Entity, &Interaction), With<GroupClose>>,
    mut widgets: Query<
        &mut Visibility,
        (With<GroupWidget>, Without<GroupInviteWidget>, Without<GroupMemberText>),
    >,
    mut lines: Query<(&mut Text, &GroupMemberText), Without<GroupInviteText>>,
    mut invite_widgets: Query<&mut Visibility, (With<GroupInviteWidget>, Without<GroupWidget>)>,
    mut invite_texts: Query<(&mut Text, &GroupInviteText), Without<GroupMemberText>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Group);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        for mut vis in &mut invite_widgets {
            *vis = Visibility::Hidden;
        }
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Group);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match group.members.get(line.0) {
            Some(m) if m.is_leader => format!("★{}", m.name),
            Some(m) if !m.online => format!("{}（离线）", m.name),
            Some(m) => m.name.clone(),
            None => String::new(),
        };
    }
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
    yes: Query<(Entity, &Interaction), With<GroupInviteYes>>,
    no: Query<(Entity, &Interaction), With<GroupInviteNo>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    if group.invite.is_none() {
        return;
    }
    let mut accept: Option<bool> = None;
    for (e, inter) in &yes {
        if edge(e, inter, &mut prev_inter) {
            accept = Some(true);
        }
    }
    for (e, inter) in &no {
        if edge(e, inter, &mut prev_inter) {
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
    btns: Query<(Entity, &Interaction), With<GroupSwitch>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    for (e, inter) in &btns {
        if edge(e, inter, &mut prev_inter) {
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
    del_btn: Query<(Entity, &Interaction), With<GroupDelBtn>>,
    ok_btn: Query<(Entity, &Interaction), With<GroupDelOk>>,
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
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
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
    for (e, inter) in &del_btn {
        if edge(e, inter, &mut prev_inter) && is_leader {
            group.del_open = true;
            input.active = Some(32);
        }
    }
    // 确认 / Enter → 发送 DellMember{Name}
    let mut confirmed = false;
    for (e, inter) in &ok_btn {
        if edge(e, inter, &mut prev_inter) {
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
    add_btn: Query<(Entity, &Interaction), With<GroupAddBtn>>,
    ok_btn: Query<(Entity, &Interaction), With<GroupAddOk>>,
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
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
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
    for (e, inter) in &add_btn {
        if edge(e, inter, &mut prev_inter) && is_leader {
            group.add_open = true;
            input.active = Some(33);
        }
    }
    // 确认 / Enter → 发送 AddMember{Name}
    let mut confirmed = false;
    for (e, inter) in &ok_btn {
        if edge(e, inter, &mut prev_inter) {
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
