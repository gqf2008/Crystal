// ============================================================================
#![allow(clippy::type_complexity)]
// 玩家控制（M8）
// 交互参考：Client/MirScenes/GameScene.cs
//   - 右键点击空地 → 寻路移动（NewMove）
//   - 左键点击 NPC → CallNPC [@Main]；左键点击怪物 → 攻击
//   - 中键 → AutoRun 切换（跑步）
// ============================================================================

use bevy::prelude::*;

use crate::actor::{ActorAnim, ActorAppearance, LocalPlayer, NetObjectId};
use crate::game::movement::{direction_from_delta, world_to_tile, LocalMove};
use crate::game::pathfinding;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetworkContext;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiButton;

#[derive(Resource)]
pub struct ControlState {
    /// 自动跑步（中键切换）
    pub autorun: bool,
    /// 当前攻击目标 object_id
    pub attack_target: Option<u32>,
    /// 上次攻击时间（秒）
    pub last_attack: f32,
    /// 攻击间隔（原版 AttackTime 约 1 秒）
    pub attack_interval: f32,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            autorun: false,
            attack_target: None,
            last_attack: 0.0,
            attack_interval: 1.0,
        }
    }
}

pub struct PlayerControlPlugin;

impl Plugin for PlayerControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (player_input_system, auto_attack_system).run_if(in_state(AppState::Game)),
        );
    }
}

/// 屏幕坐标 → 世界坐标
fn screen_to_world(screen: Vec2, cam_tf: &Transform, window: &Window) -> Vec2 {
    let half_w = window.width() / 2.0;
    let half_h = window.height() / 2.0;
    Vec2::new(screen.x - half_w + cam_tf.translation.x, screen.y - half_h + cam_tf.translation.y)
}

/// 主对话框底部区域（点击不响应移动）
fn over_main_dialog(screen: Vec2) -> bool {
    // 主对话框：底部居中，高约 150
    screen.y >= 768.0 - 150.0
}

/// 聊天面板区域（左上）
fn over_chat_panel(screen: Vec2) -> bool {
    screen.x <= 380.0 && screen.y >= 768.0 - 150.0 - 190.0
}

fn player_input_system(
    mut commands: Commands,
    time: Res<Time>,
    mut control: ResMut<ControlState>,
    net: Res<NetworkContext>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<&Transform, (With<Camera2d>, Without<UiButton>)>,
    players: Query<
        (Entity, &Transform, &mut ActorAnim),
        (With<LocalPlayer>, Without<NetObjectId>),
    >,
    actors: Query<(&NetObjectId, &Transform, &ActorAppearance)>,
    buttons: Query<&UiButton>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(cam_tf) = camera.single() else { return };

    // UI 按钮点击时不处理地图交互
    let over_ui = buttons.iter().any(|b| {
        let (x, y, w, h) = b.rect;
        cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
    });

    let world = screen_to_world(cursor, cam_tf, window);

    // 中键：AutoRun 切换（原版 GameScene.OnMouseClick Middle）
    if mouse.just_pressed(MouseButton::Middle) {
        control.autorun = !control.autorun;
        tracing::info!("🏃 AutoRun: {}", control.autorun);
    }

    // 右键：寻路移动（原版 NewMove + PathFinder.FindPath）
    if mouse.just_pressed(MouseButton::Right) && !over_ui && !over_main_dialog(cursor) && !over_chat_panel(cursor) {
        let Some(map) = &game_data.map else { return };
        let target_tile = world_to_tile(world.x, world.y);
        let Ok((pe, ptf, _)) = players.single() else { return };
        let from_tile = world_to_tile(ptf.translation.x, ptf.translation.y);
        libs.0.ensure_initialized();
        if let Some(p) = pathfinding::find_path(map, from_tile, target_tile) {
            if p.is_empty() {
                tracing::debug!("🚫 目标不可达: {:?}", target_tile);
            } else {
                let len = p.len();
                commands.entity(pe).insert(LocalMove {
                    path: p.into(),
                    step_timer_ms: 0.0,
                    run: control.autorun,
                });
                tracing::info!("🚶 寻路 {} -> {}（{} 格）", from_tile.0, from_tile.1, len);
            }
        } else {
            tracing::debug!("🚫 目标不可达: {:?}", target_tile);
        }
    }

    // 左键：点击 NPC → CallNPC；点击怪物 → 攻击目标
    if mouse.just_pressed(MouseButton::Left) && !over_ui && !over_main_dialog(cursor) && !over_chat_panel(cursor) {
        // 命中测试：世界坐标下最近的对象（20px 内）
        let mut best: Option<(u32, f32)> = None;
        for (id, tf, app) in &actors {
            let dist = Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length();
            if dist < 24.0 && best.map(|(_, d)| dist < d).unwrap_or(true) {
                best = Some((id.0, dist));
            }
            let _ = app;
        }
        if let Some((object_id, _)) = best {
            // 区分 NPC 与怪物/玩家
            let is_npc = actors
                .iter()
                .find(|(id, _, _)| id.0 == object_id)
                .map(|(_, _, app)| matches!(app, ActorAppearance::Npc { .. }))
                .unwrap_or(false);
            if is_npc {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("🧙 CallNPC {}", object_id);
            } else {
                control.attack_target = Some(object_id);
                control.last_attack = 0.0; // 立即攻击
                tracing::info!("⚔️ 攻击目标 {}", object_id);
            }
        } else {
            // 点击空地：取消当前攻击目标
            control.attack_target = None;
        }
    }

    // 自动攻击：每 attack_interval 秒对目标发 Attack
    control.last_attack += time.delta_secs();
    let _ = &mut libs;
}

/// 自动攻击（目标存在且存活时循环攻击）
fn auto_attack_system(
    time: Res<Time>,
    mut control: ResMut<ControlState>,
    net: Res<NetworkContext>,
    players: Query<&Transform, (With<LocalPlayer>, Without<NetObjectId>)>,
    actors: Query<(&NetObjectId, &Transform)>,
) {
    control.last_attack += time.delta_secs();
    let Some(target_id) = control.attack_target else { return };

    // 目标已消失 → 停止攻击
    let Some((_, target_tf)) = actors.iter().find(|(id, _)| id.0 == target_id) else {
        control.attack_target = None;
        return;
    };
    let Ok(player_tf) = players.single() else { return };

    if control.last_attack < control.attack_interval {
        return;
    }
    control.last_attack = 0.0;

    // 朝向目标
    let dx = (target_tf.translation.x - player_tf.translation.x) as i32;
    let dy = (target_tf.translation.y - player_tf.translation.y) as i32;
    let dir = direction_from_delta(dx.signum(), dy.signum()).unwrap_or(mir2_shared::enums::MirDirection::Up);

    net.send_packet(&mir2_shared::packets::client::combat::Attack {
        direction: dir,
        spell: mir2_shared::enums::Spell::None,
    });
    tracing::debug!("⚔️ Attack {}", target_id);
}
