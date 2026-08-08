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
use bevy::window::{CursorIcon, SystemCursorIcon};

use crate::actor::{ActorAnim, GroundItem, LocalPlayer, Monster, NetObjectId, Npc, Player};
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
    /// NPC 对话冷却（C# GameScene.NPCTime/NPCID：同 NPC 5 秒内忽略重复 CallNPC）
    pub npc_id: Option<u32>,
    pub last_npc_call: f32,
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
            npc_id: None,
            last_npc_call: 0.0,
        }
    }
}

pub struct PlayerControlPlugin;

impl Plugin for PlayerControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (advance_attack_timer_system, autorun_toggle_system, right_click_move_system, left_click_interact_system, key_pickup_system, hold_move_system, auto_attack_system, pickup_arrival_system, context_cursor_system)
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

/// 上下文光标（#1321：对齐 C# SetMouseCursor——NPC→手型、怪物→准星、其他→默认）
fn context_cursor_system(
    windows: Query<(Entity, &Window)>,
    camera: Query<&Transform, With<Camera2d>>,
    actors: Query<(&Transform, Has<Npc>, Has<Monster>), Without<LocalPlayer>>,
    mut commands: Commands,
    mut last: Local<SystemCursorIcon>,
) {
    let Ok((w_entity, window)) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(cam) = camera.single() else { return };
    let world = screen_to_world(cursor, cam, window);
    let mut icon = SystemCursorIcon::Default;
    for (tf, is_npc, is_monster) in &actors {
        if (tf.translation.x - world.x).abs() < 24.0 && (tf.translation.y - world.y).abs() < 24.0 {
            if is_npc {
                icon = SystemCursorIcon::Pointer;
                break;
            }
            if is_monster && icon == SystemCursorIcon::Default {
                icon = SystemCursorIcon::Crosshair;
            }
        }
    }
    if icon != *last {
        *last = icon;
        commands.entity(w_entity).insert(CursorIcon::System(icon));
    }
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

/// 打包 UI 锁定资源（数量框/确认框/选中物品），避免系统参数超 Bevy 16 上限
#[derive(SystemParam)]
struct UiLockState<'w> {
    click: Res<'w, crate::game::dialogs::inventory::InvClickState>,
    amount: Res<'w, crate::game::dialogs::amount_box::AmountBoxState>,
    confirm: Res<'w, crate::game::dialogs::inventory::InvDropConfirm>,
}

impl UiLockState<'_> {
    fn locked(&self) -> bool {
        self.click.selected.is_some() || self.amount.visible || self.confirm.visible
    }
}

/// 修饰键检测（C# CMain.Alt：采集）
fn is_alt_down(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)
}

/// 修饰键检测（C# CMain.Shift：强制攻击/远程攻击）
fn is_shift_down(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
}



/// 推进攻击计时（原 player_input_system 末尾逻辑独立成系统；保持既有行为）
fn advance_attack_timer_system(
    time: Res<Time>,
    mut control: ResMut<ControlState>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    control.last_attack += time.delta_secs();
}

/// 中键：AutoRun 切换（原版 GameScene.OnMouseClick Middle）
fn autorun_toggle_system(
    mut control: ResMut<ControlState>,
    mouse: Res<ButtonInput<MouseButton>>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    if mouse.just_pressed(MouseButton::Middle) {
        control.autorun = !control.autorun;
        tracing::info!("🏃 AutoRun: {}", control.autorun);
    }
}

/// 右键：寻路移动（原版 NewMove + PathFinder.FindPath）
fn right_click_move_system(
    mut commands: Commands,
    control: Res<ControlState>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<&Transform, (With<Camera2d>, Without<UiButton>, Without<crate::ui::sprite_ui::UiEntity>)>,
    players: Query<(Entity, &Transform, &mut ActorAnim), (With<LocalPlayer>, With<NetObjectId>)>,
    objects: Query<
        (&NetObjectId, &Transform),
        (
            Or<(With<Npc>, With<Monster>, With<crate::actor::PlayerName>)>,
            Without<LocalPlayer>,
        ),
    >,
    buttons: Query<&UiButton>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else { return };
    let Some(cursor_logical) = window.cursor_position() else { return };
    let Ok(cam_tf) = camera.single() else { return };
    let over_ui = buttons.iter().any(|b| {
        let (x, y, w, h) = b.rect;
        cursor_logical.x >= x && cursor_logical.x <= x + w && cursor_logical.y >= y && cursor_logical.y <= y + h
    });
    if !mouse.just_pressed(MouseButton::Right) || over_ui || over_main_dialog(cursor_logical) || over_chat_panel(cursor_logical) {
        return;
    }
    let Some(map) = &game_data.map else { return };
    let world = screen_to_world(cursor, cam_tf, window);
    // C# OnMouseClick Right：仅当 MouseObject == null（空地）才寻路移动；
    // 点到怪物/NPC/玩家不移动（玩家右键由 player_menu 弹菜单）
    let Some(cursor_logical) = window.cursor_position() else { return };
    let world_logical = screen_to_world(cursor_logical, cam_tf, window);
    let hit_object = objects.iter().any(|(_, tf)| {
        let d1 = Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length();
        let d2 = Vec2::new(tf.translation.x - world_logical.x, tf.translation.y - world_logical.y).length();
        d1.min(d2) < 60.0
    });
    if hit_object {
        tracing::debug!("🖱️ 右键对象 → 不移动（C# 仅空地右键寻路）");
        return;
    }
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

/// 左键：点击 NPC → CallNPC；点击怪物 → 攻击目标；点击物品 → 拾取/走过去拾取
fn left_click_interact_system(
    mut commands: Commands,
    mut control: ResMut<ControlState>,
    net: Res<NetConnection>,
    game_data: Res<GameData>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window>,
    camera: Query<&Transform, (With<Camera2d>, Without<UiButton>, Without<crate::ui::sprite_ui::UiEntity>)>,
    players: Query<(Entity, &Transform, &mut ActorAnim), (With<LocalPlayer>, With<NetObjectId>)>,
    actors: Query<(&NetObjectId, &Transform, Has<Npc>), Without<LocalPlayer>>,
    remote_players: Query<&NetObjectId, (With<crate::actor::Player>, Without<LocalPlayer>)>,
    items: Query<(&NetObjectId, &Transform), (With<GroundItem>, Without<LocalPlayer>)>,
    buttons: Query<&UiButton>,
    ui: UiLockState,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else { return };
    let Some(cursor_logical) = window.cursor_position() else { return };
    let Ok(cam_tf) = camera.single() else { return };
    let over_ui = buttons.iter().any(|b| {
        let (x, y, w, h) = b.rect;
        cursor_logical.x >= x && cursor_logical.x <= x + w && cursor_logical.y >= y && cursor_logical.y <= y + h
    });
    let world = screen_to_world(cursor, cam_tf, window);
    let world_logical = screen_to_world(cursor_logical, cam_tf, window);
    // （选中物品/数量框/确认框打开时不处理世界点击——丢弃流程由背包系统接管）
    if !mouse.just_pressed(MouseButton::Left)
        || ui.locked()
        || over_ui
        || over_main_dialog(cursor_logical)
        || over_chat_panel(cursor_logical)
    {
        return;
    }
    tracing::debug!("🖱️ 左键点击 screen=({},{}) world=({:.0},{:.0})", cursor.x, cursor.y, world.x, world.y);
    let Ok((pe, ptf, anim)) = players.single() else { return };
    let from_tile = world_to_tile(ptf.translation.x, ptf.translation.y);
    // C# OnMouseDown：Alt+左键 → Harvest（采集/挖矿，方向 = 玩家→鼠标方向）
    if is_alt_down(&keys) {
        let target_tile = world_to_tile(world.x, world.y);
        let dx = (target_tile.0 - from_tile.0).signum();
        let dy = (target_tile.1 - from_tile.1).signum();
        let direction = direction_from_delta(dx, dy).unwrap_or(
            mir2_shared::enums::MirDirection::try_from(anim.direction)
                .unwrap_or(mir2_shared::enums::MirDirection::Up),
        );
        net.send_packet(&mir2_shared::packets::client::combat::Harvest { direction });
        tracing::info!("⛏️ Alt+左键采集 dir={:?} target=({},{})", direction, target_tile.0, target_tile.1);
        return;
    }
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
        let is_npc = actors
            .iter()
            .find(|(id, _, _)| id.0 == object_id)
            .map(|(_, _, is_npc)| is_npc)
            .unwrap_or(false);
        let is_player = remote_players.iter().any(|id| id.0 == object_id);
        if is_npc {
            // C# OnMouseClick Left：CallNPC，同 NPC 5 秒冷却（GameScene.NPCTime/NPCID）
            let now = time.elapsed_secs();
            if !npc_call_allowed(control.npc_id, control.last_npc_call, now, object_id) {
                tracing::debug!("🧙 CallNPC {} 冷却中", object_id);
                return;
            }
            control.npc_id = Some(object_id);
            control.last_npc_call = now;
            net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                object_id,
                key: "[@Main]".to_string(),
            });
            tracing::info!("🧙 CallNPC {}", object_id);
        } else if is_player {
            // C# OnMouseDown Left：点击玩家 break（不攻击）；Shift+左键才攻击（PvP）
            if is_shift_down(&keys) {
                control.attack_target = Some(object_id);
                control.last_attack = 0.0;
                tracing::info!("⚔️ [Shift] 攻击玩家 {}", object_id);
            } else {
                tracing::debug!("🖱️ 点击玩家 {}（C# break，不攻击）", object_id);
                control.attack_target = None;
            }
        } else {
            control.attack_target = Some(object_id);
            control.last_attack = 0.0; // 立即攻击
            tracing::info!("⚔️ 攻击目标 {}", object_id);
        }
    } else {
        control.attack_target = None;
    }
}

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
    actors: Query<
        &Transform,
        (
            Without<LocalPlayer>,
            Without<GroundItem>,
            Or<(With<Player>, With<Monster>, With<Npc>)>,
        ),
    >,
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
            .any(|tf| Vec2::new(tf.translation.x - world.x, tf.translation.y - world.y).length() < 45.0)
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


/// 按键拾取最近物品（#158 C# KeybindOptions.Pickup：默认 Tab；保留 Space 为次键）
fn key_pickup_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    kb: Res<crate::game::dialogs::keyboard_layout::KeyboardState>,
    net: Res<NetConnection>,
    game_data: Res<GameData>,
    mut control: ResMut<ControlState>,
    items: Query<(&crate::actor::NetObjectId, &Transform), With<crate::actor::GroundItem>>,
    players: Query<(Entity, &Transform), (With<LocalPlayer>, Without<crate::actor::GroundItem>)>,
) {
    let Some(b) = kb
        .bindings
        .iter()
        .find(|b| b.action == "拾取" || b.action == "拾取2")
    else {
        return;
    };
    if !keys.just_pressed(b.key) {
        return;
    }
    let Ok((pe, ptf)) = players.single() else { return };
    // 找最近地面物品
    let mut best: Option<(u32, f32)> = None;
    for (id, tf) in &items {
        let d = Vec2::new(tf.translation.x - ptf.translation.x, tf.translation.y - ptf.translation.y).length();
        if d < 800.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((id.0, d));
        }
    }
    let Some((item_id, _)) = best else { return };
    let item_tile = items
        .iter()
        .find(|(id, _)| id.0 == item_id)
        .map(|(_, tf)| world_to_tile(tf.translation.x, tf.translation.y));
    let Some(item_tile) = item_tile else { return };
    let from_tile = world_to_tile(ptf.translation.x, ptf.translation.y);
    let adjacent = (item_tile.0 - from_tile.0).abs() <= 1 && (item_tile.1 - from_tile.1).abs() <= 1;
    if adjacent {
        net.send_packet(&mir2_shared::packets::client::item::PickUp {});
        control.attack_target = None;
        tracing::info!("🎒 [KEY] 拾取地面物品 id={}", item_id);
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
                tracing::info!("🚶 [KEY] 走向物品 id={}（{} 格）", item_id, len);
            }
        }
    }
}
/// C# GameScene.NPCTime/NPCID：同 NPC 5 秒内忽略重复 CallNPC
fn npc_call_allowed(prev_id: Option<u32>, last_call: f32, now: f32, object_id: u32) -> bool {
    !(prev_id == Some(object_id) && now - last_call < 5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_call_cooldown() {
        assert!(npc_call_allowed(None, 0.0, 10.0, 1));
        assert!(npc_call_allowed(Some(1), 10.0, 15.0, 1)); // 正好 5 秒 → 允许
        assert!(!npc_call_allowed(Some(1), 10.0, 14.9, 1)); // 5 秒内 → 忽略
        assert!(npc_call_allowed(Some(1), 10.0, 11.0, 2)); // 不同 NPC → 允许
    }

    #[test]
    fn test_modifier_helpers() {
        let mut keys = ButtonInput::<KeyCode>::default();
        assert!(!is_alt_down(&keys));
        assert!(!is_shift_down(&keys));
        keys.press(KeyCode::AltLeft);
        assert!(is_alt_down(&keys));
        assert!(!is_shift_down(&keys));
        keys.press(KeyCode::ShiftRight);
        assert!(is_shift_down(&keys));
    }
}



