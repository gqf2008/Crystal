// ============================================================================
#![allow(clippy::type_complexity)]
// 玩家控制（M8）
// 交互参考：Client/MirScenes/GameScene.cs
//   - 右键点击空地 → 寻路移动（NewMove）
//   - 左键点击 NPC → CallNPC [@Main]；左键点击怪物 → 攻击
//   - 中键 → AutoRun 切换（跑步）
// ============================================================================

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::actor::{ActorAnim, ActorAppearance, GroundItem, LocalPlayer, NetObjectId};
use crate::game::hud::HudState;
use crate::game::movement::{direction_from_delta, world_to_tile, LocalMove};
use crate::game::pathfinding;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetConnection;
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
    /// 待拾取的地面物品 object_id（寻路到达后自动 PickUp）
    pub pickup_target: Option<u32>,
    /// 按住移动状态：目标格 + 模式（true=跑, false=走），用于持续追踪鼠标
    pub hold_target: Option<(i32, i32)>,
    pub hold_run: Option<bool>,
    /// 是否已进入“按住移动”模式（长按 0.2s 后才置位，区分单击寻路）
    pub hold_active: bool,
    /// 按下时刻（秒），用于区分单击/长按
    pub hold_pressed_at: Option<f32>,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            autorun: false,
            attack_target: None,
            last_attack: 0.0,
            attack_interval: 1.0,
            pickup_target: None,
            hold_target: None,
            hold_run: None,
            hold_active: false,
            hold_pressed_at: None,
        }
    }
}

pub struct PlayerControlPlugin;

impl Plugin for PlayerControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (player_input_system, hold_move_system, auto_attack_system, pickup_arrival_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 屏幕坐标 → 世界坐标（用物理像素，避免 DPI 缩放导致 cursor_position 偏差）
pub fn screen_to_world(screen: Vec2, cam_tf: &Transform, window: &Window) -> Vec2 {
    let half_w = window.physical_width() as f32 / 2.0;
    let half_h = window.physical_height() as f32 / 2.0;
    // 屏幕 y 向下、世界 y 向上：点击下方 → 世界 y 减小（必须取反，否则方向相反）
    Vec2::new(
        screen.x - half_w + cam_tf.translation.x,
        cam_tf.translation.y - (screen.y - half_h),
    )
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

/// 物品选中/弹窗打开时屏蔽世界左键点击（原版 C# SelectedCell/Modal）
/// 合并三个守卫参数，避免系统参数超过 Bevy 16 上限
#[derive(SystemParam)]
struct InteractionGuards<'w> {
    click: Res<'w, crate::game::dialogs::inventory::InvClickState>,
    amount: Res<'w, crate::game::dialogs::amount_box::AmountBoxState>,
    confirm: Res<'w, crate::game::dialogs::inventory::InvDropConfirm>,
}

fn player_input_system(
    mut commands: Commands,
    time: Res<Time>,
    mut control: ResMut<ControlState>,
    net: Res<NetConnection>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<&Transform, (With<Camera2d>, Without<UiButton>, Without<crate::ui::sprite_ui::UiEntity>)>,
    players: Query<
        (Entity, &Transform, &mut ActorAnim),
        (With<LocalPlayer>, With<NetObjectId>),
    >,
    actors: Query<(&NetObjectId, &Transform, &ActorAppearance)>,
    items: Query<(&NetObjectId, &Transform), (With<GroundItem>, Without<LocalPlayer>)>,
    buttons: Query<&UiButton>,
    guards: InteractionGuards,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else { return };
    let Some(cursor_logical) = window.cursor_position() else { return };
    let Ok(cam_tf) = camera.single() else { return };

    // UI 按钮点击时不处理地图交互（按钮 rect 是逻辑坐标，必须用逻辑光标比较；
    // 之前误用物理光标 → DPI 1.5 下坐标错位，右下大部分点击被当成主对话框忽略）
    let over_ui = buttons.iter().any(|b| {
        let (x, y, w, h) = b.rect;
        cursor_logical.x >= x && cursor_logical.x <= x + w && cursor_logical.y >= y && cursor_logical.y <= y + h
    });

    // DPI 环境下物理/逻辑坐标可能不一致，两个换算点都参与命中
    let world = screen_to_world(cursor, cam_tf, window);
    let world_logical = screen_to_world(cursor_logical, cam_tf, window);

    // 中键：AutoRun 切换（原版 GameScene.OnMouseClick Middle）
    if mouse.just_pressed(MouseButton::Middle) {
        control.autorun = !control.autorun;
        tracing::info!("🏃 AutoRun: {}", control.autorun);
    }

    // 右键：寻路移动（原版 NewMove + PathFinder.FindPath）
    if mouse.just_pressed(MouseButton::Right) && !over_ui && !over_main_dialog(cursor_logical) && !over_chat_panel(cursor_logical) {
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
                    last: None,
                    step_origin: None,
                    turn_acc: 0.0,
                });
                tracing::info!("🚶 寻路 {} -> {}（{} 格）", from_tile.0, from_tile.1, len);
            }
        } else {
            tracing::debug!("🚫 目标不可达: {:?}", target_tile);
        }
    }

    // 左键：点击 NPC → CallNPC；点击怪物 → 攻击目标
    // （选中物品/数量框/确认框打开时不处理世界点击——丢弃流程由背包系统接管）
    if mouse.just_pressed(MouseButton::Left)
        && guards.click.selected.is_none()
        && !guards.amount.visible
        && !guards.confirm.visible
        && !over_ui
        && !over_main_dialog(cursor_logical)
        && !over_chat_panel(cursor_logical)
    {
        tracing::debug!("🖱️ 左键点击 screen=({},{}) world=({:.0},{:.0})", cursor.x, cursor.y, world.x, world.y);
        // 命中测试：世界坐标下最近的对象（20px 内）
        let mut best: Option<(u32, f32)> = None;
        for (id, tf, app) in &actors {
            let d1 = Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length();
            let d2 = Vec2::new(tf.translation.x - world_logical.x, tf.translation.y - world_logical.y).length();
            let dist = d1.min(d2);
            if dist < 60.0 && best.map(|(_, d)| dist < d).unwrap_or(true) {
                best = Some((id.0, dist));
            }
            let _ = app;
        }
        tracing::info!("[HITDBG] best_actor={:?}", best);
        // 地面物品命中（原版 C# ItemObject：点击物品 → 邻近拾取 / 远距离走过去拾取）
        let mut best_item: Option<(u32, f32)> = None;
        for (id, tf) in &items {
            let d1 = Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length();
            let d2 = Vec2::new(tf.translation.x - world_logical.x, tf.translation.y - world_logical.y).length();
            let dist = d1.min(d2);
            if dist < 45.0 && best_item.map(|(_, d)| dist < d).unwrap_or(true) {
                best_item = Some((id.0, dist));
            }
        }
        if let Some((item_id, item_d)) = best_item {
            let actor_d = best.map(|(_, d)| d);
            if actor_d.map(|d| item_d < d).unwrap_or(true) {
                let Ok((pe, ptf, _)) = players.single() else { return };
                let from_tile = world_to_tile(ptf.translation.x, ptf.translation.y);
                let item_tile = items
                    .iter()
                    .find(|(id, _)| id.0 == item_id)
                    .map(|(_, tf)| world_to_tile(tf.translation.x, tf.translation.y));
                if let Some(item_tile) = item_tile {
                    let adjacent = (item_tile.0 - from_tile.0).abs() <= 1
                        && (item_tile.1 - from_tile.1).abs() <= 1;
                    if adjacent {
                        net.send_packet(&mir2_shared::packets::client::item::PickUp {});
                        control.attack_target = None;
                        tracing::info!("🎒 拾取地面物品 id={}", item_id);
                    } else if let Some(map) = &game_data.map {
                        if let Some(p) = pathfinding::find_path(map, from_tile, item_tile) {
                            if p.is_empty() {
                                tracing::debug!("🚫 物品不可达: {:?}", item_tile);
                            } else {
                                let len = p.len();
                                commands.entity(pe).insert(LocalMove {
                                    path: p.into(),
                                    step_timer_ms: 0.0,
                                    run: control.autorun,
                                    last: None,
                                    step_origin: None,
                                    turn_acc: 0.0,
                                });
                                control.attack_target = None;
                                control.pickup_target = Some(item_id);
                                tracing::info!("🚶 走向物品 id={}（{} 格）", item_id, len);
                            }
                        }
                    }
                }
                return;
            }
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

/// 拾取到达：寻路结束后自动 PickUp（原版 C# 点击物品 → 移动 → 拾取）
fn pickup_arrival_system(
    mut control: ResMut<ControlState>,
    net: Res<NetConnection>,
    items: Query<(&NetObjectId, &Transform), (With<GroundItem>, Without<LocalPlayer>)>,
    players: Query<(&Transform, Option<&LocalMove>), (With<LocalPlayer>, With<NetObjectId>)>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    let Some(target) = control.pickup_target else { return };
    // 物品已消失（被拾取/过期）→ 清除目标
    let Some((_, item_tf)) = items.iter().find(|(id, _)| id.0 == target) else {
        control.pickup_target = None;
        return;
    };
    let Ok((player_tf, lm)) = players.single() else { return };
    // 仍在移动中（路径未走完）
    if let Some(lm) = lm {
        if !lm.path.is_empty() {
            return;
        }
    } else {
        return;
    }
    let item_tile = world_to_tile(item_tf.translation.x, item_tf.translation.y);
    let player_tile = world_to_tile(player_tf.translation.x, player_tf.translation.y);
    if (item_tile.0 - player_tile.0).abs() <= 1 && (item_tile.1 - player_tile.1).abs() <= 1 {
        net.send_packet(&mir2_shared::packets::client::item::PickUp {});
        tracing::info!("🎒 到达后拾取物品 id={}", target);
    }
    control.pickup_target = None;
}

/// 自动攻击（目标存在且存活时循环攻击）
fn auto_attack_system(
    mut commands: Commands,
    time: Res<Time>,
    mut control: ResMut<ControlState>,
    net: Res<NetConnection>,
    sound_bank: Res<crate::game::sound::SoundBank>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
    players: Query<&Transform, (With<LocalPlayer>, With<NetObjectId>)>,
    actors: Query<(&NetObjectId, &Transform)>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
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
    crate::game::sound::play_sound(&mut commands, &mut audio_assets, &sound_bank, 10050);
    // 诊断（#57）：攻击时打印玩家/目标瓦片与方向（debug 级）
    tracing::debug!(
        "⚔️ Attack target={} dir={:?}",
        target_id, dir
    );
}


/// 按住鼠标持续移动（对齐原版 C# GameScene）：
/// - 右键按住 = 跑、左键按住 = 走，方向持续跟随鼠标
/// - 目标格变化或路径走完 → 自动重新寻路（避障，不停下）
/// - 左键按住且鼠标下有 NPC/怪物/物品时不做移动（交互交给点击处理）
fn hold_move_system(
    mut commands: Commands,
    mut control: ResMut<ControlState>,
    time: Res<Time>,
    game_data: Res<GameData>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<
        &Transform,
        (With<Camera2d>, Without<UiButton>, Without<crate::ui::sprite_ui::UiEntity>),
    >,
    mut players: Query<
        (Entity, &Transform, &mut LocalMove, &mut ActorAnim),
        (With<LocalPlayer>, With<NetObjectId>),
    >,
    actors: Query<(&Transform, &ActorAppearance), (Without<LocalPlayer>, Without<GroundItem>)>,
    items: Query<&Transform, (With<GroundItem>, Without<LocalPlayer>)>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    let Some(map) = &game_data.map else { return };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else { return };
    let Ok(cam_tf) = camera.single() else { return };
    let world = screen_to_world(cursor, cam_tf, window);
    let target_tile = world_to_tile(world.x, world.y);

    let run = if mouse.pressed(MouseButton::Right) {
        Some(true)
    } else if mouse.pressed(MouseButton::Left) {
        // 左键按住：鼠标下有可交互对象时交给点击交互，不做移动
        let near_actor = actors
            .iter()
            .any(|(tf, _)| Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length() < 45.0)
            || items
                .iter()
                .any(|tf| Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length() < 40.0);
        if near_actor {
            None
        } else {
            Some(false)
        }
    } else {
        None
    };

    // 长按 0.2s 才进入“按住移动”模式（单击只触发 NewMove 寻路）
    let t = time.elapsed_secs();
    if mouse.just_pressed(MouseButton::Right) || mouse.just_pressed(MouseButton::Left) {
        control.hold_pressed_at = Some(t);
    }
    let pressed_long = control
        .hold_pressed_at
        .map(|p| t - p >= 0.2)
        .unwrap_or(false);
    if run.is_some() && pressed_long {
        control.hold_active = true;
    }

    if control.hold_active {
        if let Some(run) = run {
            let Ok((pe, ptf, mut lm, mut anim)) = players.single_mut() else { return };
            let from_tile = world_to_tile(ptf.translation.x, ptf.translation.y);
            let need_repath = control.hold_target != Some(target_tile)
                || control.hold_run != Some(run)
                || lm.path.is_empty();
            if need_repath {
                control.hold_target = Some(target_tile);
                control.hold_run = Some(run);
                if from_tile == target_tile {
                    return;
                }
                if let Some(p) = pathfinding::find_path(map, from_tile, target_tile) {
                    if p.is_empty() {
                        tracing::debug!("[HOLD] 目标不可达 {:?}", target_tile);
                    }
                    if !p.is_empty() {
                        let first = p[0];
                        if let Some(d) =
                            direction_from_delta(first.0 - from_tile.0, first.1 - from_tile.1)
                        {
                            anim.direction = d as u8;
                        }
                        anim.action = if run {
                            mir2_shared::enums::MirAction::Running
                        } else {
                            mir2_shared::enums::MirAction::Walking
                        };
                        commands.entity(pe).insert(LocalMove {
                            path: p.into(),
                            step_timer_ms: 0.0,
                            run,
                            last: None,
                            step_origin: None,
                            turn_acc: 0.0,
                        });
                    }
                }
            }
        } else {
            // 按住移动中松开 → 立即停下
            control.hold_target = None;
            control.hold_run = None;
            control.hold_active = false;
            control.hold_pressed_at = None;
            if let Ok((_, _, mut lm, mut anim)) = players.single_mut() {
                lm.path.clear();
                lm.last = None;
                anim.action = mir2_shared::enums::MirAction::Standing;
                anim.frame_index = 0;
            }
        }
    } else if run.is_none() {
        // 未进入按住模式且鼠标未按住：清除按住状态（单击寻路路径保留）
        control.hold_target = None;
        control.hold_run = None;
        control.hold_active = false;
        control.hold_pressed_at = None;
    }
}
