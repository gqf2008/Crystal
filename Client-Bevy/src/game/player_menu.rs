// ============================================================================
// 右键玩家菜单（#138，C# MainDialogs 右键玩家菜单对齐）
// 交易 / 组队 / 私聊 / 查看 / 添加好友
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, NetObjectId, PlayerName};
use crate::game::chat::ChatState;
use crate::game::dialogs::{DialogKind, DialogManager};
use crate::game::dialogs::mail::MailState;
use crate::game::dialogs::text_input::TextInputState;
use crate::network::NetConnection;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_text, ui_button_system, UiButton, UiEntity, UiFont};

/// 菜单选项
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerMenuAction {
    Trade,
    Group,
    Whisper,
    Inspect,
    AddFriend,
    Observe,
    Mail,
}

/// 右键菜单状态
#[derive(Resource, Default)]
pub struct PlayerMenuState {
    pub visible: bool,
    pub name: String,
    pub object_id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct PlayerMenuWidget;

#[derive(Component)]
pub struct PlayerMenuOption(PlayerMenuAction);

pub struct PlayerMenuPlugin;

impl Plugin for PlayerMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerMenuState>();
        app.add_systems(OnEnter(AppState::Game), spawn_player_menu);
        app.add_systems(OnExit(AppState::Game), cleanup_player_menu);
        app.add_systems(
            Update,
            (player_menu_open_system, player_menu_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_player_menu(mut commands: Commands, roots: Query<Entity, With<PlayerMenuWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_player_menu(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        PlayerMenuWidget,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.1, 0.1, 0.14, 0.96),
            custom_size: Some(Vec2::new(90.0, 140.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(-999.0, -999.0, 20.0),
        Visibility::Hidden,
    ));
    let items: [(&str, PlayerMenuAction); 7] = [
        ("交易", PlayerMenuAction::Trade),
        ("组队", PlayerMenuAction::Group),
        ("私聊", PlayerMenuAction::Whisper),
        ("查看", PlayerMenuAction::Inspect),
        ("加好友", PlayerMenuAction::AddFriend),
        ("观察", PlayerMenuAction::Observe),
        ("邮件", PlayerMenuAction::Mail),
    ];
    for (i, (label, action)) in items.iter().enumerate() {
        let t = spawn_ui_text(
            &mut commands, &font, label,
            -999.0, -999.0,
            12.0, Color::WHITE, 20.2,
        );
        commands.entity(t).insert((
            PlayerMenuOption(*action),
            UiButton {
                rect: (-999.0, -999.0, 90.0, 18.0),
                clicked: false,
            },
        ));
    }
}

/// 右键点击远端玩家 → 打开菜单；右键空地/他人 → 关闭
fn player_menu_open_system(
    mut state: ResMut<PlayerMenuState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<&Transform, With<Camera2d>>,
    remote_players: Query<
        (&PlayerName, &NetObjectId, &Transform),
        (Without<LocalPlayer>, Without<PlayerMenuWidget>),
    >,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(cam) = camera.single() else { return };
    let world = crate::game::player_control::screen_to_world(cursor, cam, &window);
    let mut target: Option<(String, u32)> = None;
    for (name, id, tf) in &remote_players {
        if (tf.translation.x - world.x).abs() < 24.0 && (tf.translation.y - world.y).abs() < 24.0 {
            target = Some((name.0.clone(), id.0));
        }
    }
    if let Some((name, object_id)) = target {
        state.visible = true;
        state.name = name;
        state.object_id = object_id;
        state.x = cursor.x;
        state.y = cursor.y;
        tracing::info!("🖱️ 右键玩家 {} → 打开菜单", state.name);
    } else {
        state.visible = false;
    }
}

/// 菜单显隐 + 定位 + 选项点击
fn player_menu_ui_system(
    mut state: ResMut<PlayerMenuState>,
    mut mgr: ResMut<DialogManager>,
    mut mail: ResMut<MailState>,
    mut input: ResMut<TextInputState>,
    net: Res<NetConnection>,
    mut chat: ResMut<ChatState>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut options: Query<(&mut Transform, &mut UiButton, &PlayerMenuOption)>,
    mut widgets: Query<(&mut Transform, &mut Visibility), (With<PlayerMenuWidget>, Without<PlayerMenuOption>)>,
) {
    // ESC 关闭（#146）
    if state.visible && keys.just_pressed(KeyCode::Escape) {
        state.visible = false;
    }
    // 点击菜单外关闭
    if state.visible && mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                if cursor.x < state.x || cursor.x > state.x + 90.0 || cursor.y < state.y || cursor.y > state.y + 140.0 {
                    state.visible = false;
                }
            }
        }
    }
    // 面板定位 + 显隐
    for (mut tf, mut vis) in &mut widgets {
        *vis = if state.visible { Visibility::Visible } else { Visibility::Hidden };
        if state.visible {
            tf.translation.x = state.x;
            tf.translation.y = -state.y;
        }
    }
    // 选项定位（跟随面板）
    let mut idx = 0usize;
    for (mut tf, mut btn, _) in &mut options {
        let oy = state.y + 6.0 + idx as f32 * 20.0;
        tf.translation.x = state.x + 8.0;
        tf.translation.y = -oy;
        btn.rect = (state.x, oy, 90.0, 18.0);
        idx += 1;
    }
    if !state.visible {
        return;
    }
    // 选项点击
    for (_, btn, action) in &options {
        if !btn.clicked {
            continue;
        }
        match action.0 {
            PlayerMenuAction::Trade => {
                net.send_packet(&mir2_shared::packets::client::trade::TradeRequest);
                tracing::info!("🤝 请求交易: {}", state.name);
            }
            PlayerMenuAction::Group => {
                net.send_packet(&mir2_shared::packets::client::group::AddMember {
                    name: state.name.clone(),
                });
                tracing::info!("👥 邀请组队: {}", state.name);
            }
            PlayerMenuAction::Whisper => {
                chat.input_active = true;
                chat.input_text = format!("/w {} ", state.name);
                tracing::info!("💬 私聊 {}", state.name);
            }
            PlayerMenuAction::Inspect => {
                net.send_packet(&mir2_shared::packets::client::chat::Inspect {
                    object_id: state.object_id,
                });
                tracing::info!("🔍 查看玩家 {}", state.name);
            }
            PlayerMenuAction::Observe => {
                net.send_packet(&crate::network::ObserveWire {
                    target_id: state.object_id,
                });
                tracing::info!("👁️ 观察玩家 {}", state.name);
            }
            PlayerMenuAction::Mail => {
                // C# PlayerDialog.MailButton → MailComposeLetterDialog.ComposeMail(Name)
                mgr.open.push(DialogKind::Mail);
                mail.compose = true;
                mail.detail = None;
                mail.attach = vec![None; 5];
                mail.compose_gold = 0;
                if input.texts.len() < 4 {
                    input.texts.resize(4, String::new());
                }
                input.texts[0] = state.name.clone();
                input.active = None;
                tracing::info!("✉️ 给 {} 写邮件", state.name);
            }
            PlayerMenuAction::AddFriend => {
                net.send_packet(&mir2_shared::packets::client::friend::AddFriend {
                    name: state.name.clone(),
                    blocked: false,
                });
                tracing::info!("👥 添加好友 {}", state.name);
            }
        }
        state.visible = false;
    }
}


