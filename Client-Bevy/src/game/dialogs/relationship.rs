// ============================================================================
// 关系/婚姻对话框（M49）
// 参考：C# RelationshipDialog + ServerRust social.rs 婚姻系统
// 网络：
//   C: MarriageRequest[target dotnet] / MarriageReply[bool] / ChangeMarriage(空)
//      DivorceRequest[partner dotnet] / DivorceReply[bool]
//   S: MarriageRequest[lover dotnet] / LoverUpdate[Name dotnet][Date i64][MapName dotnet][MarriedDays i16] / DivorceRequest[lover dotnet]
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
                ui_button_system,
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Relationship),
            RelationshipWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            RelationshipClose,
            DialogRoot(DialogKind::Relationship),
            RelationshipWidget,
        ));
    }
    for i in 0..4usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            RelationshipLine(i),
            DialogRoot(DialogKind::Relationship),
            RelationshipWidget,
        ));
    }
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let box_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::Relationship),
            RelationshipWidget,
            RelationshipTargetField,
            crate::game::dialogs::text_input::TextInputField(13),
            crate::game::dialogs::text_input::TextInputRect(298.0, 215.0, 160.0, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(160.0, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(298.0, -215.0, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(13),
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
    // 求婚 / 离婚
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 250.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            RelationshipPropose,
            DialogRoot(DialogKind::Relationship),
            RelationshipWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 250.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            RelationshipDivorce,
            DialogRoot(DialogKind::Relationship),
            RelationshipWidget,
        ));
    }
    // 邀请弹窗
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands
            .entity(e)
            .insert((MarriageInviteWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 40.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((MarriageInviteText, MarriageInviteWidget));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 240.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarriageInviteYes, MarriageInviteWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 340.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((MarriageInviteNo, MarriageInviteWidget));
    }
}

/// 显隐 + 渲染 + 求婚/离婚
#[allow(clippy::too_many_arguments)]
fn relationship_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<RelationshipState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<&UiButton, With<RelationshipClose>>,
    propose_btn: Query<&UiButton, With<RelationshipPropose>>,
    divorce_btn: Query<&UiButton, With<RelationshipDivorce>>,
    mut widgets: Query<&mut Visibility, With<RelationshipWidget>>,
    mut lines: Query<(&mut Text2d, &RelationshipLine)>,
) {
    let open = mgr.is_open(DialogKind::Relationship);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
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
    for btn in &propose_btn {
        if btn.clicked {
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
    for btn in &divorce_btn {
        if btn.clicked && state.married {
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
    yes: Query<&UiButton, With<MarriageInviteYes>>,
    no: Query<&UiButton, With<MarriageInviteNo>>,
    mut widgets: Query<&mut Visibility, With<MarriageInviteWidget>>,
    mut texts: Query<(&mut Text2d, &MarriageInviteText)>,
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
            Some(name) => format!("{} 向你求婚！", name),
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
