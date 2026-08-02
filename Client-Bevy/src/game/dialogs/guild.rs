// ============================================================================
// 行会对话框（M27）
// 布局参考：C# GuildDialog.cs / macroquad guild_dialog.rs
//   - 背景 Prguse[956]，标题 Title[15]，位置 (280,80)
//   - 行会名/会长/金币、成员列表（职务+在线）、公告、创建输入框
// 网络：GuildStatus（1 字节 in_guild / 完整信息，同 opcode 双格式）、GuildNoticeChange、GuildMemberChange
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

/// 行会成员
#[derive(Debug, Clone, Default)]
pub struct GuildMember {
    pub name: String,
    pub rank: u8,
    pub online: bool,
}

/// 行会状态
#[derive(Resource, Default)]
pub struct GuildState {
    pub in_guild: bool,
    pub name: String,
    pub leader: String,
    pub notice: Vec<String>,
    pub members: Vec<GuildMember>,
    pub gold: u32,
    /// 待处理行会邀请（行会名）
    pub invite: Option<String>,
    /// 选中的成员行（踢出用）
    pub selected_member: Option<usize>,
}

#[derive(Component)]
pub struct GuildWidget;

#[derive(Component)]
pub struct GuildClose;

/// 创建行会输入框（TextInputState id 0）
#[derive(Component)]
pub struct GuildNameField;

#[derive(Component)]
pub struct GuildCreateBtn;

/// 邀请玩家输入框（TextInput id 1）
#[derive(Component)]
pub struct GuildInviteField;

#[derive(Component)]
pub struct GuildInviteBtn;

/// 踢出选中成员
#[derive(Component)]
pub struct GuildKickBtn;

// 邀请提示
#[derive(Component)]
pub struct GuildInviteWidget;

#[derive(Component)]
pub struct GuildInviteText;

#[derive(Component)]
pub struct GuildInviteYes;

#[derive(Component)]
pub struct GuildInviteNo;

#[derive(Component)]
pub struct GuildLine(usize);

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildState>();
        app.add_systems(OnEnter(AppState::Game), spawn_guild);
        app.add_systems(OnExit(AppState::Game), cleanup_guild);
        app.add_systems(
            Update,
            (guild_ui_system, guild_invite_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_guild(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_guild(
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

    // 背景 Prguse[956]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 956) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[15]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 15) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 88.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 340.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            GuildClose,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 行会名/会长文本（GuildLine 0 占位显示头部）
    let head = spawn_ui_text(
        &mut commands, &font, "",
        298.0, 120.0, 12.0, Color::srgb(1.0, 0.9, 0.5), 8.0,
    );
    commands.entity(head).insert((
        GuildLine(0),
        DialogRoot(DialogKind::Guild),
        GuildWidget,
    ));
    // 成员列表（10 行，1..=10）
    for i in 1..=10usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 140.0 + (i - 1) as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            GuildLine(i),
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 创建行会：输入框 + 按钮（原版 C# GuildDialog 创建流程）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let name_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildNameField,
            crate::game::dialogs::text_input::TextInputField(0),
            crate::game::dialogs::text_input::TextInputRect(340.0, 330.0, 200.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(200.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(340.0, -330.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(name_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(0),
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
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 360.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildCreateBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    // 邀请玩家：输入框（TextInput id 1）+ 邀请按钮
    let inv_box = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GuildInviteField,
            crate::game::dialogs::text_input::TextInputField(1),
            crate::game::dialogs::text_input::TextInputRect(340.0, 390.0, 200.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(200.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(340.0, -390.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(inv_box).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(1),
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
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 420.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildInviteBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 420.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            GuildKickBtn,
            DialogRoot(DialogKind::Guild),
            GuildWidget,
        ));
    }

    // 邀请提示（MirMessageBox）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands
            .entity(e)
            .insert((GuildInviteWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 40.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((GuildInviteText, GuildInviteWidget));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 240.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((GuildInviteYes, GuildInviteWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 340.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((GuildInviteNo, GuildInviteWidget));
    }
}

/// 显隐 + 渲染 + 打开时请求行会信息 + 创建按钮
#[allow(clippy::too_many_arguments)]
fn guild_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetworkContext>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    create_btn: Query<&UiButton, With<GuildCreateBtn>>,
    invite_btn: Query<&UiButton, With<GuildInviteBtn>>,
    kick_btn: Query<&UiButton, With<GuildKickBtn>>,
    close: Query<&UiButton, With<GuildClose>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<
        (&mut Visibility, Option<&GuildLine>, Option<&GuildNameField>),
        (With<GuildWidget>, Without<GuildCreateBtn>),
    >,
    mut lines: Query<(&mut Text2d, &GuildLine)>,
    mut requested: Local<bool>,
) {
    let open = mgr.is_open(DialogKind::Guild);
    for (mut vis, _line, _field) in &mut widgets {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求行会信息（原版 C# GuildDialog.Show → RequestGuildInfo）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::guild::RequestGuildInfo {
            info_type: 0,
        });
        tracing::info!("🏰 请求行会信息");
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Guild);
        }
    }
    // 渲染
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => {
                if guild.in_guild {
                    format!(
                        "{}（{}）金币:{}",
                        guild.name,
                        guild.leader,
                        guild.gold
                    )
                } else {
                    "未加入行会".to_string()
                }
            }
            i => match guild.members.get(i - 1) {
                Some(m) => {
                    let rank = match m.rank {
                        0 => "会长",
                        1 => "副会长",
                        _ => "成员",
                    };
                    format!(
                        "{}{} ({})",
                        m.name,
                        if m.online { "" } else { "（离线）" },
                        rank
                    )
                }
                None => String::new(),
            },
        };
    }
    // 创建按钮 → GuildNameReturn（原版 C#：输入行会名 → 创建）
    for btn in &create_btn {
        if btn.clicked {
            let name = input.texts.get(0).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: name.clone(),
                });
                tracing::info!("🏰 创建行会: {}", name);
                input.texts[0].clear();
                input.active = None;
            }
        }
    }
    // 邀请按钮 → EditGuildMember{0=add member}（C# GuildDialog 邀请）
    for btn in &invite_btn {
        if btn.clicked {
            let name = input.texts.get(1).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() && guild.in_guild {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 0,
                    rank_index: 0,
                    name: name.clone(),
                    rank_name: String::new(),
                });
                tracing::info!("🏰 邀请玩家加入行会: {}", name);
                input.texts[1].clear();
                input.active = None;
            }
        }
    }
    // 踢出按钮 → EditGuildMember{1=delete member}（对选中的成员）
    for btn in &kick_btn {
        if btn.clicked {
            if let Some(idx) = guild.selected_member {
                if let Some(m) = guild.members.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                        change_type: 1,
                        rank_index: 0,
                        name: m.name.clone(),
                        rank_name: String::new(),
                    });
                    tracing::info!("🏰 踢出行会成员: {}", m.name);
                    guild.selected_member = None;
                }
            }
        }
    }
    // 点击成员行选中（踢出目标）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 1..=10usize {
                    let y = 140.0 + (i - 1) as f32 * 20.0;
                    if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                        if guild.members.get(i - 1).is_some() {
                            guild.selected_member = Some(i - 1);
                            tracing::info!(
                                "🏰 选中行会成员: {}",
                                guild.members[i - 1].name
                            );
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// 行会邀请提示：Yes/No → C.GuildInvite{accept}
fn guild_invite_system(
    mut guild: ResMut<GuildState>,
    net: Res<NetworkContext>,
    yes: Query<&UiButton, With<GuildInviteYes>>,
    no: Query<&UiButton, With<GuildInviteNo>>,
    mut widgets: Query<
        &mut Visibility,
        (With<GuildInviteWidget>, Without<GuildWidget>),
    >,
    mut texts: Query<(&mut Text2d, &GuildInviteText)>,
) {
    let has_invite = guild.invite.is_some();
    for mut vis in &mut widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut texts {
        text.0 = match guild.invite.as_ref() {
            Some(name) => format!("{} 邀请你加入行会", name),
            None => String::new(),
        };
    }
    if guild.invite.is_none() {
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
        net.send_packet(&mir2_shared::packets::client::guild::GuildInvite {
            accept_invite: a,
        });
        tracing::info!("🏰 行会邀请回复: accept={}", a);
        guild.invite = None;
    }
}