// ============================================================================
// 师徒对话框（M33）
// 布局参考：原版 C# MentorDialog.cs（背景 Prguse[170]、标题 Title[51]）
// 交互逻辑对齐 C#：
//   - 打开对话框显示师父/徒弟两行（名字/等级/在线），等级高者即师父
//   - AllowButton → C.AllowMentor{allow}（C# 按钮开关）
//   - AddButton → 输入名字 → C.AddMentor{Name}
//   - RemoveButton → C.CancelMentor（C# 有 YesNo 确认，这里直接解除）
// 网络：S.MentorRequest{Name,Level}（邀请弹窗 → C.MentorReply{accept}）
//      S.MentorUpdate{Name,Level,Online,MenteeEXP}（C# GetMentor 语义）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

/// 师徒状态（MentorUpdate 写入；mentor_* 字段语义同 C# MentorDialog：对方信息）
#[derive(Resource, Default)]
pub struct MentorState {
    /// 对方名字（无师徒关系时为空）
    pub mentor_name: String,
    /// 对方等级（0 = 无关系）
    pub mentor_level: u32,
    /// 对方是否在线
    pub mentor_online: bool,
    /// 拜师经验（师父视角）
    pub mentee_exp: i64,
    /// 是否允许拜师（本地开关）
    pub allow_mentor: bool,
    /// 待处理拜师邀请（名字, 等级）
    pub invite: Option<(String, u16)>,
}

#[derive(Component)]
pub struct MentorWidget;

#[derive(Component)]
pub struct MentorClose;

#[derive(Component)]
pub struct MentorAllow;

#[derive(Component)]
pub struct MentorAdd;

#[derive(Component)]
pub struct MentorRemove;

/// 邀请输入框（TextInput id 4）
#[derive(Component)]
pub struct MentorNameField;

#[derive(Component)]
pub struct MentorLine(usize);

// 邀请提示
#[derive(Component)]
pub struct MentorInviteWidget;

#[derive(Component)]
pub struct MentorInviteText;

#[derive(Component)]
pub struct MentorInviteYes;

#[derive(Component)]
pub struct MentorInviteNo;

pub struct MentorPlugin;

impl Plugin for MentorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MentorState>();
                app.add_systems(
            Update,
            mentor_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_mentor);
        app.add_systems(OnExit(AppState::Game), cleanup_mentor);
        app.add_systems(
            Update,
            (
                mentor_ui_system,
                mentor_invite_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_mentor(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_mentor(
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

    // 面板 Prguse[170]（C# MentorDialog.Index=170，244x207 @ 280,80）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 244.0, 207.0, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Mentor), MentorWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[51] @(18,8)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 51) {
            spawn_image(p, h, 18.0, 8.0, 103.0, 17.0, 9);
        }
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(MentorClose);
        }
        // 信息行（0 标题 / 1 师父 / 2 徒弟 / 3 拜师经验 / 4 允许拜师状态）@(10,45+26i)
        for i in 0..5usize {
            spawn_label(p, &font, "", 10.0, 45.0 + i as f32 * 26.0, 12.0, Color::WHITE, 9)
                .insert(MentorLine(i));
        }
        // 允许拜师（Prguse[114/115/116] @(20,195)）、添加（Title[213/214/215] @(90,195)）、
        // 解除（Title[216/217/218] @(180,195)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 114),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 115),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 116),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 195.0, 60.0, 25.0, 10).insert(MentorAllow);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 213),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 214),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 215),
        ) {
            spawn_icon_button(p, n, h, pr, 90.0, 195.0, 76.0, 25.0, 10).insert(MentorAdd);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 216),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 217),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 218),
        ) {
            spawn_icon_button(p, n, h, pr, 180.0, 195.0, 76.0, 25.0, 10).insert(MentorRemove);
        }
        // 师父名字输入框（TextInput id 4）@(20,235)
        spawn_container(p, 20.0, 235.0, 180.0, 20.0, 10)
            .insert((
                MentorNameField,
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                crate::game::dialogs::text_input::TextInputField(4),
                crate::game::dialogs::text_input::TextInputRect(300.0, 315.0, 180.0, 20.0),
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
                    crate::game::dialogs::text_input::TextInputDisplay(4),
                ));
            });
    });

    // 邀请提示（C# MirMessageBox：Prguse[360] 原生 456x190 居中 @(284,289)，
    // Label(35,35)、Yes Title[206/207/208] (260,157)、No Title[210/211/212] (360,157)。
    // 批 14 修正：旧实现面板 260x120 + 按钮 (240/340,150) 被 Overflow::clip 裁掉不可见）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let inv = spawn_panel(&mut commands, h, bx, by, 456.0, 190.0, 45);
        commands.entity(inv).insert(MentorInviteWidget);
        commands.entity(inv).with_children(|ip| {
            spawn_label(ip, &font, "", 35.0, 35.0, 12.0, Color::WHITE, 9)
                .insert(MentorInviteText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(ip, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(MentorInviteYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(ip, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(MentorInviteNo);
            }
        });
    }
}

/// 显隐 + 渲染 + 允许/添加/解除按钮
#[allow(clippy::too_many_arguments)]
fn mentor_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<MentorState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    // #2633 批次4 步7：name→`PlayerName`、level→`Progression`（HudState 已于步9 删除）；
    // 实体缺失按 HudState 默认（level=1/name=""）
    player_q: Query<
        (
            &crate::game::player_state::Progression,
            &crate::actor::PlayerName,
        ),
        With<crate::actor::LocalPlayer>,
    >,
    close: Query<(Entity, &Interaction), With<MentorClose>>,
    allow_btn: Query<(Entity, &Interaction), With<MentorAllow>>,
    add_btn: Query<(Entity, &Interaction), With<MentorAdd>>,
    remove_btn: Query<(Entity, &Interaction), With<MentorRemove>>,
    mut widgets: Query<&mut Visibility, With<MentorWidget>>,
    mut lines: Query<(&mut Text, &MentorLine)>,
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
    let open = mgr.is_open(DialogKind::Mentor);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Mentor);
        }
    }
    let has = !state.mentor_name.is_empty() && state.mentor_level != 0;
    let (me_level, me_name) = player_q
        .single()
        .map(|(p, n)| (p.level, n.0.clone()))
        .unwrap_or((1, String::new()));
    let self_is_mentor = has && (me_level as u32) > state.mentor_level;
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "师徒".to_string(),
            1 => {
                if !has {
                    "师父: 无".to_string()
                } else if self_is_mentor {
                    format!("师父: {} Lv.{}", me_name, me_level)
                } else {
                    format!(
                        "师父: {} Lv.{}{}",
                        state.mentor_name,
                        state.mentor_level,
                        if state.mentor_online { "（在线）" } else { "（离线）" }
                    )
                }
            }
            2 => {
                if !has {
                    "徒弟: 无".to_string()
                } else if self_is_mentor {
                    format!(
                        "徒弟: {} Lv.{}{}",
                        state.mentor_name,
                        state.mentor_level,
                        if state.mentor_online { "（在线）" } else { "（离线）" }
                    )
                } else {
                    format!("徒弟: {} Lv.{}", me_name, me_level)
                }
            }
            3 => {
                if self_is_mentor {
                    format!("拜师经验: {}", state.mentee_exp)
                } else {
                    String::new()
                }
            }
            4 => format!(
                "允许拜师: {}（点开关按钮切换）",
                if state.allow_mentor { "开" } else { "关" }
            ),
            _ => String::new(),
        };
    }
    for (e, inter) in &allow_btn {
        if edge(e, inter, &mut prev_inter) {
            state.allow_mentor = !state.allow_mentor;
            net.send_packet(&crate::network::AllowMentorWire {
                allow: state.allow_mentor,
            });
            tracing::info!("🧑‍🏫 允许拜师: {}", state.allow_mentor);
        }
    }
    for (e, inter) in &add_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(4).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() && !has {
                net.send_packet(&mir2_shared::packets::client::misc::AddMentor {
                    name: name.clone(),
                });
                tracing::info!("🧑‍🏫 请求拜师: {}", name);
                input.texts[4].clear();
                input.active = None;
            } else if has {
                tracing::warn!("🧑‍🏫 已有师徒关系，无法再拜师");
            }
        }
    }
    for (e, inter) in &remove_btn {
        if edge(e, inter, &mut prev_inter) && has {
            net.send_packet(&mir2_shared::packets::client::misc::CancelMentor);
            tracing::info!("🧑‍🏫 解除师徒关系");
        }
    }
}

/// 拜师邀请提示：Yes/No → C.MentorReply{accept}
fn mentor_invite_system(
    mut state: ResMut<MentorState>,
    net: Res<NetConnection>,
    yes: Query<(Entity, &Interaction), With<MentorInviteYes>>,
    no: Query<(Entity, &Interaction), With<MentorInviteNo>>,
    mut widgets: Query<&mut Visibility, With<MentorInviteWidget>>,
    mut texts: Query<(&mut Text, &MentorInviteText)>,
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
    let has_invite = state.invite.is_some();
    for mut vis in widgets.iter_mut() {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut texts {
        text.0 = match state.invite.as_ref() {
            Some((name, level)) => format!("{}（Lv.{}）请求拜你为师", name, level),
            None => String::new(),
        };
    }
    if state.invite.is_none() {
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
        net.send_packet(&mir2_shared::packets::client::misc::MentorReply {
            accept_invite: a,
        });
        tracing::info!("🧑‍🏫 拜师邀请回复: accept={}", a);
        state.invite = None;
    }
}

/// 消费服务端师徒事件（网络层只广播 ServerEvent）
fn mentor_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mentor: ResMut<MentorState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::MentorInvite { name, level } => {
                mentor.invite = Some((name.clone(), *level));
            }
            ServerEvent::MentorUpdate { name, level, online, mentee_exp } => {
                mentor.mentor_name = name.clone();
                mentor.mentor_level = *level;
                mentor.mentor_online = *online;
                mentor.mentee_exp = *mentee_exp;
            }
            _ => {}
        }
    }
}
