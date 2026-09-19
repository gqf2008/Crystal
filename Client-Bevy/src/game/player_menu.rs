// ============================================================================
// 右键玩家菜单（#138，C# MainDialogs 右键玩家菜单对齐）
// 交易 / 组队 / 私聊 / 查看 / 添加好友
// ============================================================================

use bevy::prelude::*;

use crate::actor::{LocalPlayer, NetObjectId, PlayerName};
use crate::game::chat::ChatState;
use crate::game::dialogs::mail::MailState;
use crate::game::dialogs::text_input::TextInputState;
use crate::game::dialogs::{DialogKind, DialogManager};
use crate::network::NetConnection;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    shared_cjk_font, spawn_ui_text, ui_button_system, UiButton, UiCjkFont, UiEntity, UiFont,
};

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
pub struct PlayerMenuOption {
    action: PlayerMenuAction,
    /// #2771：行号。此前按 Query 迭代顺序自增（实体表顺序 ≠ 生成顺序）→ 菜单项顺序会乱
    /// （实机见过「交易/组队/加好友/观察/邮件/私聊/查看」与生成顺序不符）；改为按生成序固定。
    index: usize,
}

/// C# `KeybindOptions.Trade` 的绑定名（`KeyBindSettings.cs:340`，默认 T，文案「请求交易」）
pub const TRADE_KEYBIND_ACTION: &str = "请求交易";

/// 玩家菜单项 Hint 文案（C# `MainDialogs.cs` 玩家菜单）：
/// 组队=「邀请加入队伍」、加好友=「添加到好友列表」、邮件=「发送邮件」、
/// 交易=「交易 ({键})」（`Trade` 键位文本）、观察=「观战」；私聊/查看 C# 无 Hint。
pub fn player_menu_hint(action: PlayerMenuAction, trade_key: &str) -> Option<String> {
    let text = match action {
        PlayerMenuAction::Group => "邀请加入队伍".to_string(),
        PlayerMenuAction::AddFriend => "添加到好友列表".to_string(),
        PlayerMenuAction::Mail => "发送邮件".to_string(),
        PlayerMenuAction::Trade => format!("交易 ({trade_key})"),
        PlayerMenuAction::Observe => "观战".to_string(),
        PlayerMenuAction::Whisper | PlayerMenuAction::Inspect => return None,
    };
    Some(text)
}

pub struct PlayerMenuPlugin;

impl Plugin for PlayerMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerMenuState>();
        app.add_systems(OnEnter(AppState::Game), spawn_player_menu);
        app.add_systems(OnExit(AppState::Game), cleanup_player_menu);
        app.add_systems(
            Update,
            (
                player_menu_open_system,
                player_menu_ui_system,
                ui_button_system,
            )
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
    mut cjk_font: ResMut<crate::ui::sprite_ui::UiCjkFont>,
    kb: Res<crate::game::dialogs::keyboard_layout::KeyboardState>,
) {
    // #2771 实机暴露：菜单项文字用 `UiFont`(Arial) 时**渲染成豆腐**（与批17 提示框同源——
    // parley 的 Hani 回退只在首次排版生效，而本菜单文本先以 (-999,-999) 建好、打开时才移进视野
    // → 首次排版时机与可见性错开）。改用共享宋体主字体（与 NPC/公告等动态文本一致）。
    let font = crate::ui::sprite_ui::shared_cjk_font(&mut fonts, &mut cjk_font);
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
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
    // #2771：菜单项 Hint（C# `MainDialogs.cs` 玩家菜单按钮：2232 GroupButton=邀请加入队伍、
    // 2261 FriendButton=添加到好友列表、2277 MailButton=发送邮件、2290 TradeButton=交易 ({键})、
    // 2303 ObserveButton=观战）。私聊/查看两项 C# 无 Hint。
    let trade_key = kb
        .bindings
        .iter()
        .find(|b| b.action == TRADE_KEYBIND_ACTION)
        .map(|b| crate::game::dialogs::keyboard_layout::key_name(b.key))
        .unwrap_or_default();
    for (i, (label, action)) in items.iter().enumerate() {
        let t = spawn_ui_text(
            &mut commands,
            &font,
            label,
            -999.0,
            -999.0,
            12.0,
            Color::WHITE,
            20.2,
        );
        commands.entity(t).insert((
            PlayerMenuOption {
                action: *action,
                index: i,
            },
            UiButton {
                rect: (-999.0, -999.0, 90.0, 18.0),
                clicked: false,
            },
        ));
        if let Some(hint) = player_menu_hint(*action, &trade_key) {
            commands
                .entity(t)
                .insert(crate::ui::tooltip::TooltipHint(hint));
        }
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// #2771：玩家菜单 Hint 文案对齐 C# `MainDialogs.cs` 的五个按钮；私聊/查看 C# 无 Hint
    #[test]
    fn player_menu_hint_matches_csharp() {
        assert_eq!(
            player_menu_hint(PlayerMenuAction::Group, "T").as_deref(),
            Some("邀请加入队伍")
        );
        assert_eq!(
            player_menu_hint(PlayerMenuAction::AddFriend, "T").as_deref(),
            Some("添加到好友列表")
        );
        assert_eq!(
            player_menu_hint(PlayerMenuAction::Mail, "T").as_deref(),
            Some("发送邮件")
        );
        // C# `Trade` 文案带键位：`交易 ({GetKey(Trade)})`
        assert_eq!(
            player_menu_hint(PlayerMenuAction::Trade, "T").as_deref(),
            Some("交易 (T)")
        );
        assert_eq!(
            player_menu_hint(PlayerMenuAction::Observe, "T").as_deref(),
            Some("观战")
        );
        assert_eq!(player_menu_hint(PlayerMenuAction::Whisper, "T"), None);
        assert_eq!(player_menu_hint(PlayerMenuAction::Inspect, "T"), None);
    }
}

/// 菜单显隐 + 定位 + 选项点击
/// （pub(crate)：#2604 esc_close_dialogs_system 的 Esc 让路依赖
/// `.before(本系统)` 排序锚点——本系统同帧消费 Esc 置 visible=false，
/// 若先跑则 esc_close 读到 false 误入 Closeall 连坐）
pub(crate) fn player_menu_ui_system(
    mut state: ResMut<PlayerMenuState>,
    mut mgr: ResMut<DialogManager>,
    mut mail: ResMut<MailState>,
    mut input: ResMut<TextInputState>,
    net: Res<NetConnection>,
    mut chat: ResMut<ChatState>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    gate: Res<crate::game::input_gate::TextInputGate>,
    windows: Query<&Window>,
    mut options: Query<(
        &mut Transform,
        &mut UiButton,
        &mut Visibility,
        &PlayerMenuOption,
    )>,
    mut widgets: Query<
        (&mut Transform, &mut Visibility),
        (With<PlayerMenuWidget>, Without<PlayerMenuOption>),
    >,
) {
    // ESC 关闭（#146）。#2604：输入态（聊天/数量框/通用输入框）激活时不抢
    // Esc——那些模态自己消费（否则同帧 Esc 既关菜单又关输入框，层级穿透）
    if state.visible && !gate.0 && keys.just_pressed(KeyCode::Escape) {
        state.visible = false;
    }
    // 点击菜单外关闭
    if state.visible && mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                if cursor.x < state.x
                    || cursor.x > state.x + 90.0
                    || cursor.y < state.y
                    || cursor.y > state.y + 140.0
                {
                    state.visible = false;
                }
            }
        }
    }
    // 面板定位 + 显隐
    for (mut tf, mut vis) in &mut widgets {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if state.visible {
            tf.translation.x = state.x;
            tf.translation.y = -state.y;
        }
    }
    // 选项定位（跟随面板）；菜单未打开时必须隐藏并移出屏幕，
    // 否则 7 个菜单文字会一直显示在左上角 (8,6..126)（用户看到的“交易/组队/私聊...”）
    // 且按钮 rect 留在 (0,0) 附近可被误点击
    for (mut tf, mut btn, mut vis, option) in &mut options {
        if !state.visible {
            *vis = Visibility::Hidden;
            tf.translation.x = -999.0;
            tf.translation.y = -999.0;
            btn.rect = (-999.0, -999.0, 90.0, 18.0);
            continue;
        }
        *vis = Visibility::Visible;
        // #2771：行号取生成序（`option.index`），不再依赖 Query 迭代顺序
        let oy = state.y + 6.0 + option.index as f32 * 20.0;
        tf.translation.x = state.x + 8.0;
        tf.translation.y = -oy;
        btn.rect = (state.x, oy, 90.0, 18.0);
    }
    if !state.visible {
        return;
    }
    // 选项点击
    for (_, btn, _vis, option) in &options {
        if !btn.clicked {
            continue;
        }
        match option.action {
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
                    ranking: false,
                    name: String::new(),
                });
                tracing::info!("🔍 查看玩家 {}", state.name);
            }
            PlayerMenuAction::Observe => {
                net.send_packet(&crate::network::ObserveWire {
                    name: state.name.clone(),
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
