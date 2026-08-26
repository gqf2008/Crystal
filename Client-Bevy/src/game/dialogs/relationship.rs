// ============================================================================
// 关系/婚姻对话框（M49）
// 参考：C# RelationshipDialog + ServerRust social.rs 婚姻系统
// 网络：
//   C: MarriageRequest[target dotnet] / MarriageReply[bool] / ChangeMarriage(空)
//      DivorceRequest[partner dotnet] / DivorceReply[bool]
//   S: MarriageRequest[lover dotnet] / LoverUpdate[Name dotnet][Date i64][MapName dotnet][MarriedDays i16] / DivorceRequest[lover dotnet]
// bevy_ui 迁移（批 14）：面板 Prguse[170] @(280,80) 244x207，全节点化；
//   邀请弹窗 = C# MirMessageBox（Prguse[360] 原生 456x190 居中 @(284,289)，
//   Label(35,35)、Yes Title[206/207/208] (260,157)、No Title[210/211/212] (360,157)）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
};

/// 婚姻状态
#[derive(Resource, Default)]
pub struct RelationshipState {
    pub married: bool,
    /// 配偶名（#1329：LoverUpdate 全量）
    pub lover_name: String,
    /// 结婚日期（unix 秒）
    pub date: i64,
    /// 配偶当前地图标题
    pub map_name: String,
    /// 结婚天数
    pub married_days: i16,
    /// 收到结婚邀请（对方名字）
    pub invite: Option<String>,
    pub message: String,
}

#[derive(Component)]
pub struct RelationshipWidget;

#[derive(Component)]
pub struct RelationshipClose;

#[derive(Component)]
pub struct RelationshipPropose;

#[derive(Component)]
pub struct RelationshipDivorce;

#[derive(Component)]
pub struct RelationshipLine(usize);

/// 邀请弹窗
#[derive(Component)]
pub struct MarriageInviteWidget;

#[derive(Component)]
pub struct MarriageInviteText;

#[derive(Component)]
pub struct MarriageInviteYes;

#[derive(Component)]
pub struct MarriageInviteNo;

/// 目标名输入框（TextInput 13）
#[derive(Component)]
pub struct RelationshipTargetField;

pub struct RelationshipPlugin;

impl Plugin for RelationshipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RelationshipState>();
                app.add_systems(
            Update,
            relationship_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_relationship);
        app.add_systems(OnExit(AppState::Game), cleanup_relationship);
        app.add_systems(
            Update,
            (
                relationship_ui_system,
                marriage_invite_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_relationship(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_relationship(
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

    // 面板 Prguse[170]（244x207 @ 280,80）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 244.0, 207.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::Relationship), RelationshipWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(RelationshipClose);
        }
        // 信息行 4 @(18,40+22i)
        for i in 0..4usize {
            spawn_label(p, &font, "", 18.0, 40.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(RelationshipLine(i));
        }
        // 目标名输入框（TextInput id 13）@(18,135)，命中矩形 = 屏幕坐标 (298,215,160,20)
        spawn_container(p, 18.0, 135.0, 160.0, 20.0, 10)
            .insert((
                RelationshipTargetField,
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                crate::game::dialogs::text_input::TextInputField(13),
                crate::game::dialogs::text_input::TextInputRect(298.0, 215.0, 160.0, 20.0),
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
                    crate::game::dialogs::text_input::TextInputDisplay(13),
                ));
            });
        // 求婚 Title[206/207/208] @(20,170)、离婚 Title[210/211/212] @(110,170)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 170.0, 76.0, 25.0, 10)
                .insert(RelationshipPropose);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 110.0, 170.0, 76.0, 25.0, 10)
                .insert(RelationshipDivorce);
        }
    });

    // 婚姻邀请弹窗（C# MirMessageBox：Prguse[360] 原生 456x190 居中 @(284,289)）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let inv = spawn_panel(&mut commands, h, bx, by, 456.0, 190.0, 45);
        commands.entity(inv).insert(MarriageInviteWidget);
        commands.entity(inv).with_children(|ip| {
            // Label（C# (35,35)，390x110）
            spawn_label(ip, &font, "", 35.0, 35.0, 12.0, Color::WHITE, 9)
                .insert(MarriageInviteText);
            // Yes Title[206/207/208]（C# (260,157)）
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(ip, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(MarriageInviteYes);
            }
            // No Title[210/211/212]（C# (360,157)）
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(ip, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(MarriageInviteNo);
            }
        });
    }
}

/// 显隐 + 渲染 + 求婚/离婚
#[allow(clippy::too_many_arguments)]
fn relationship_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<RelationshipState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<(Entity, &Interaction), With<RelationshipClose>>,
    propose_btn: Query<(Entity, &Interaction), With<RelationshipPropose>>,
    divorce_btn: Query<(Entity, &Interaction), With<RelationshipDivorce>>,
    mut widgets: Query<&mut Visibility, With<RelationshipWidget>>,
    mut lines: Query<(&mut Text, &RelationshipLine)>,
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
    let open = mgr.is_open(DialogKind::Relationship);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Relationship);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "关系（婚姻）".to_string(),
            1 => {
                if state.married {
                    format!("婚姻状态: 已婚（{}，{} 天）", state.lover_name, state.married_days)
                } else {
                    "婚姻状态: 未婚".to_string()
                }
            }
            2 => state.message.clone(),
            3 => {
                if state.married {
                    format!("配偶位置: {}", if state.map_name.is_empty() { "未知" } else { state.map_name.as_str() })
                } else {
                    "输入目标名 → 求婚；已婚可离婚".to_string()
                }
            }
            _ => String::new(),
        };
    }
    for (e, inter) in &propose_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(13).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() && !state.married {
                net.send_packet(&crate::network::MarriageRequestWire {
                    target_name: name.clone(),
                });
                state.message = format!("已向 {} 求婚", name);
                tracing::info!("💍 求婚 → {}", name);
                input.texts[13].clear();
                input.active = None;
            }
        }
    }
    for (e, inter) in &divorce_btn {
        if edge(e, inter, &mut prev_inter) && state.married {
            // 服务端离婚流程：发起离婚请求 → 对方确认
            net.send_packet(&crate::network::DivorceRequestWire {
                partner_name: String::new(),
            });
            state.message = "已发起离婚请求".to_string();
            tracing::info!("💔 发起离婚");
        }
    }
}

/// 婚姻邀请弹窗：Yes/No → MarriageReply
fn marriage_invite_system(
    mut state: ResMut<RelationshipState>,
    net: Res<NetConnection>,
    yes: Query<(Entity, &Interaction), With<MarriageInviteYes>>,
    no: Query<(Entity, &Interaction), With<MarriageInviteNo>>,
    mut widgets: Query<&mut Visibility, With<MarriageInviteWidget>>,
    mut texts: Query<(&mut Text, &MarriageInviteText)>,
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
            Some(name) => format!("{} 向你求婚！", name),
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
        net.send_packet(&mir2_shared::packets::client::misc::MarriageReply {
            accept_invite: a,
        });
        tracing::info!("💍 婚姻邀请回复: accept={}", a);
        state.invite = None;
    }
}


/// 消费服务端婚姻/关系事件（网络层只广播 ServerEvent）
fn relationship_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut relationship: ResMut<RelationshipState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::MarriageInvite { name } => {
                relationship.invite = Some(name.clone());
                relationship.message = format!("收到 {} 的求婚", name);
            }
            ServerEvent::LoverUpdate { lover_name, date, map_name, married_days } => {
                relationship.lover_name = lover_name.clone();
                relationship.date = *date;
                relationship.map_name = map_name.clone();
                relationship.married_days = *married_days;
                relationship.married = !lover_name.is_empty();
                relationship.message = if relationship.married {
                    format!("婚姻关系已建立：{}（结婚 {} 天）", lover_name, married_days)
                } else {
                    "婚姻关系已解除".to_string()
                };
            }
            ServerEvent::DivorceRequest => {
                relationship.message = "收到离婚请求".to_string();
            }
            _ => {}
        }
    }
}
