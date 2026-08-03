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
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
        app.add_systems(OnEnter(AppState::Game), spawn_mentor);
        app.add_systems(OnExit(AppState::Game), cleanup_mentor);
        app.add_systems(
            Update,
            (
                mentor_ui_system,
                mentor_invite_system,
                ui_button_system,
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Prguse[170]（C# MentorDialog.Index=170）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[51]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 51) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 88.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭 Prguse2 360/361/362
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            MentorClose,
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
        ));
    }
    // 信息行（0 标题 / 1 师父 / 2 徒弟 / 3 拜师经验 / 4 允许拜师状态）
    for i in 0..5usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            290.0, 125.0 + i as f32 * 26.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            MentorLine(i),
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
        ));
    }
    // 允许拜师按钮（C# Prguse 114/115/116）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 114, 115, 116,
        300.0, 275.0, 8.3, 60.0, 25.0,
    ) {
        commands.entity(e).insert((
            MentorAllow,
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
        ));
    }
    // 添加师父按钮（C# Title 213/214/215）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 213, 214, 215,
        370.0, 275.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            MentorAdd,
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
        ));
    }
    // 解除师徒按钮（C# Title 216/217/218）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 216, 217, 218,
        460.0, 275.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            MentorRemove,
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
        ));
    }
    // 师父名字输入框（TextInput id 4）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let name_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Mentor),
            MentorWidget,
            MentorNameField,
            crate::game::dialogs::text_input::TextInputField(4),
            crate::game::dialogs::text_input::TextInputRect(300.0, 315.0, 180.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(180.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(300.0, -315.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(name_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(4),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });

    // 邀请提示（MirMessageBox，同行会邀请）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands
            .entity(e)
            .insert((MentorInviteWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 40.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((MentorInviteText, MentorInviteWidget));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 240.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MentorInviteYes, MentorInviteWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 340.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MentorInviteNo, MentorInviteWidget));
    }
}

/// 显隐 + 渲染 + 允许/添加/解除按钮
#[allow(clippy::too_many_arguments)]
fn mentor_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<MentorState>,
    net: Res<NetworkContext>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    hud: Res<crate::game::hud::HudState>,
    close: Query<&UiButton, With<MentorClose>>,
    allow_btn: Query<&UiButton, With<MentorAllow>>,
    add_btn: Query<&UiButton, With<MentorAdd>>,
    remove_btn: Query<&UiButton, With<MentorRemove>>,
    mut widgets: Query<&mut Visibility, With<MentorWidget>>,
    mut lines: Query<(&mut Text2d, &MentorLine)>,
) {
    let open = mgr.is_open(DialogKind::Mentor);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Mentor);
        }
    }
    // 渲染（C# UpdateInterface：等级高者即师父；无关系时隐藏信息）
    let has = !state.mentor_name.is_empty() && state.mentor_level != 0;
    let self_is_mentor = has && (hud.level as u32) > state.mentor_level;
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "师徒".to_string(),
            1 => {
                if !has {
                    "师父: 无".to_string()
                } else if self_is_mentor {
                    format!("师父: {} Lv.{}", hud.name, hud.level)
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
                    format!("徒弟: {} Lv.{}", hud.name, hud.level)
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
    // 允许拜师开关（C# AllowButton 点击 → C.AllowMentor）
    for btn in &allow_btn {
        if btn.clicked {
            state.allow_mentor = !state.allow_mentor;
            net.send_packet(&crate::network::AllowMentorWire {
                allow: state.allow_mentor,
            });
            tracing::info!("🧑‍🏫 允许拜师: {}", state.allow_mentor);
        }
    }
    // 添加师父（C# AddButton → 输入名字 → C.AddMentor）
    for btn in &add_btn {
        if btn.clicked {
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
    // 解除师徒（C# RemoveButton → C.CancelMentor）
    for btn in &remove_btn {
        if btn.clicked && has {
            net.send_packet(&mir2_shared::packets::client::misc::CancelMentor);
            tracing::info!("🧑‍🏫 解除师徒关系");
        }
    }
}

/// 拜师邀请提示：Yes/No → C.MentorReply{accept}
fn mentor_invite_system(
    mut state: ResMut<MentorState>,
    net: Res<NetworkContext>,
    yes: Query<&UiButton, With<MentorInviteYes>>,
    no: Query<&UiButton, With<MentorInviteNo>>,
    mut widgets: Query<&mut Visibility, With<MentorInviteWidget>>,
    mut texts: Query<(&mut Text2d, &MentorInviteText)>,
) {
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
        net.send_packet(&mir2_shared::packets::client::misc::MentorReply {
            accept_invite: a,
        });
        tracing::info!("🧑‍🏫 拜师邀请回复: accept={}", a);
        state.invite = None;
    }
}
