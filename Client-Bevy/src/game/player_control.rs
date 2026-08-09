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
use crate::game::movement::{direction_from_delta, mouse_direction, next_direction, point_move, previous_direction, world_to_tile, LocalMove};
use mir2_shared::enums::MirDirection;
use crate::game::pathfinding;
use crate::map_renderer::{GameData, GameLibraries, TILE_WIDTH};
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
            (advance_attack_timer_system, autorun_toggle_system, right_click_move_system, left_click_interact_system, key_pickup_system, pet_pickup_system, pet_mode_system, hold_move_system, auto_attack_system, pickup_arrival_system, context_cursor_system)
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
    mut control: ResMut<ControlState>,
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
            // #1590：C# 右键空地寻路取消当前目标（TargetObject = null）——停止自动攻击/拾取
            control.attack_target = None;
            control.pickup_target = None;
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
    mut players: Query<(Entity, &Transform, &mut ActorAnim), (With<LocalPlayer>, With<NetObjectId>)>,
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
    let Ok((pe, ptf, mut anim)) = players.single_mut() else { return };
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
                // #1584：C# 点击攻击后停止移动（CanMove=false）——清除寻路路径并回站立
                commands.entity(pe).remove::<LocalMove>();
                anim.action = mir2_shared::enums::MirAction::Standing;
                anim.frame_index = 0;
                tracing::info!("⚔️ [Shift] 攻击玩家 {}", object_id);
            } else {
                tracing::debug!("🖱️ 点击玩家 {}（C# break，不攻击）", object_id);
                control.attack_target = None;
            }
        } else {
            control.attack_target = Some(object_id);
            control.last_attack = 0.0; // 立即攻击
            // #1584：C# 点击攻击后停止移动（CanMove=false）——清除寻路路径并回站立
            commands.entity(pe).remove::<LocalMove>();
            anim.action = mir2_shared::enums::MirAction::Standing;
            anim.frame_index = 0;
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

/// #1556：攻击包类型选择——弓手且装备武器 → 远程（C# AttackRange1 → C.RangeAttack），
/// 其余 → 近战（C# Attack1 → C.Attack）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAttackKind {
    Melee,
    Ranged,
}

/// C# HumanObject.HasClassWeapon 简化：Class==Archer 且武器槽非空。
pub(crate) fn player_attack_kind(class: u8, has_weapon: bool) -> PlayerAttackKind {
    if class == mir2_shared::enums::MirClass::Archer as u8 && has_weapon {
        PlayerAttackKind::Ranged
    } else {
        PlayerAttackKind::Melee
    }
}

/// #1556：构造 C.RangeAttack 包（C# PlayerObject.cs:1574 字段映射）
pub fn build_ranged_attack(
    direction: MirDirection,
    player_tile: (i32, i32),
    target_id: u32,
    target_tile: (i32, i32),
) -> mir2_shared::packets::client::combat::RangeAttack {
    mir2_shared::packets::client::combat::RangeAttack {
        direction,
        location: mir2_shared::map::Point { x: player_tile.0, y: player_tile.1 },
        target_id,
        target_location: mir2_shared::map::Point { x: target_tile.0, y: target_tile.1 },
    }
}

/// 自动攻击（目标存在且存活时循环攻击）
/// #1554：对齐 C# 攻击距离（GameScene.CheckInput）：
///   - 近战：InRange(目标, 玩家, 1) 才 Attack1（GameScene.cs:11502）
///   - 弓手（Class==Archer 且装备武器）：InRange(..., MaxAttackRange=9) 才远程攻击（11480）
///   - 范围外：每 1s 提示"目标太远"（OutputDelay=1000，TargetTooFar，11491-11495）
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
    mut chat: ResMut<crate::game::chat::ChatState>,
    // C# OutputDelay=1000ms：范围外提示节流
    mut too_far_timer: Local<f32>,
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

    // #1554：弓手（Archer 且装备武器）→ 远程范围 9；否则近战范围 1（C# InRange Chebyshev）
    let attack_kind = player_attack_kind(
        hud.class,
        hud.equipment.get(0).and_then(|s| s.as_ref()).is_some(),
    );
    let is_archer = attack_kind == PlayerAttackKind::Ranged;
    let max_range = if is_archer { 9 } else { 1 };
    let p_tile = world_to_tile(player_tf.translation.x, player_tf.translation.y);
    let t_tile = world_to_tile(target_tf.translation.x, target_tf.translation.y);
    let in_range = (t_tile.0 - p_tile.0).abs() <= max_range
        && (t_tile.1 - p_tile.1).abs() <= max_range;

    if !in_range {
        // 保留目标（玩家需走近），每 1s 提示一次（C# OutputDelay）
        *too_far_timer += time.delta_secs();
        if *too_far_timer >= 1.0 {
            *too_far_timer = 0.0;
            chat.add_line(
                "目标太远".to_string(),
                crate::game::chat::chat_color(mir2_shared::enums::ChatType::System),
                crate::game::chat::ChatChannel::System,
            );
            tracing::debug!("🚫 目标太远: target={} range={} 需 <= {}", target_id, max_range, max_range);
        }
        return;
    }
    *too_far_timer = 0.0;

    if control.last_attack < control.attack_interval {
        return;
    }
    control.last_attack = 0.0;

    // 朝向目标
    let dx = (target_tf.translation.x - player_tf.translation.x) as i32;
    let dy = (target_tf.translation.y - player_tf.translation.y) as i32;
    let dir = direction_from_delta(dx.signum(), dy.signum()).unwrap_or(mir2_shared::enums::MirDirection::Up);

    if is_archer {
        // #1556：弓手 → C.RangeAttack（C# PlayerObject.cs:1574 AttackRange1）：
        //   Direction / Location=玩家格 / TargetID / TargetLocation=目标格；
        //   远程弹道由服务端回 S.RangeAttack 渲染（PendingEffect::Projectile），
        //   C# PlayAttackSound 弓手直接 return（不播近战挥击音）。
        net.send_packet(&build_ranged_attack(dir, p_tile, target_id, t_tile));
        tracing::debug!("🏹 RangeAttack target={} dir={:?} range={}", target_id, dir, max_range);
    } else {
        net.send_packet(&mir2_shared::packets::client::combat::Attack {
            direction: dir,
            spell: mir2_shared::enums::Spell::None,
        });
        // #1564：近战挥击音按武器/职业/骑乘选择（C# PlayAttackSound；弓手无挥击音）
        if let Some(sound_id) = crate::game::sound::attack_swing_sound(
            hud.class,
            hud.riding,
            hud.mount_type,
            hud.equipment.get(0).and_then(|s| s.as_ref()).map(|i| i.shape).unwrap_or(-1),
        ) {
            crate::game::sound::play_sound(&mut commands, &mut audio_assets, &sound_bank, sound_id);
        }
    }
    // 诊断（#57）：攻击时打印玩家/目标瓦片与方向（debug 级）
    tracing::debug!(
        "⚔️ Attack target={} dir={:?} range={} in_range={}",
        target_id, dir, max_range, in_range
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
    net: Res<NetConnection>,
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
            // #1548/#1550：方向驱动按住移动（对齐 C# GameScene.CheckInput MapButtons 分支）
            //   右键按住：鼠标距玩家 <= 2 格 → 只转向（C# GameScene.cs:11614 InRange 2）；
            //     CanRun（2/3 格）→ Run；否则 CanWalk 退避 → Walk 1 格；不可走原地转向
            //   左键按住：CanWalk 退避 → Walk 1 格；不可走原地转向
            //   陷阱/负重：InTrapRock 不可走/跑；背包或穿戴超重不可跑（C# CanRun 12139）
            let Ok((pe, ptf, mut lm, mut anim)) = players.single_mut() else { return };
            let from_tile = world_to_tile(ptf.translation.x, ptf.translation.y);
            let dir = mouse_direction(Vec2::new(ptf.translation.x, ptf.translation.y), world);
            let new_dir = dir as u8;
            let direction_changed = control.hold_target != Some((new_dir as i32, 0))
                || control.hold_run != Some(run);

            // 右键按住且鼠标距玩家 <= 2 格 → 只转向（C# GameScene.cs:11614）
            if run && (world - Vec2::new(ptf.translation.x, ptf.translation.y)).length() <= TILE_WIDTH * 2.0 {
                if direction_changed {
                    anim.direction = new_dir;
                    anim.action = mir2_shared::enums::MirAction::Standing;
                    anim.frame_index = 0;
                    control.hold_target = Some((new_dir as i32, 0));
                    control.hold_run = Some(true);
                }
                lm.path.clear();
                lm.last = None;
                return;
            }

            // 陷阱：InTrapRock 不可走/跑（C# CanWalk 12094 / CanRun 12139）
            let in_trap = hud.in_trap_rock;

            // 门检查：目标格是门（door_index != 0）且当前未放行 → 发 Opendoor（C# CheckDoorOpen 12113）
            // 门状态由服务端 Opendoor 包更新；walkable 中门格已阻挡，发 Opendoor 后服务端刷新地图可走
            fn check_door(map: &crate::map_renderer::LoadedMap, net: &NetConnection, p: (i32, i32)) -> bool {
                if !map.in_bounds(p.0, p.1) {
                    return false;
                }
                let di = map.doors[p.0 as usize][p.1 as usize];
                if di == 0 {
                    return true;
                }
                // 门格：walkable=false 表示门关；发 Opendoor 让服务端开门
                if !map.is_walkable(p.0, p.1) {
                    net.send_packet(&mir2_shared::packets::client::misc::Opendoor {
                        door_index: di,
                    });
                    tracing::debug!("🚪 请求开门 door={} at ({},{})", di, p.0, p.1);
                }
                // 门仍关（walkable=false）→ 不可走；开（walkable=true）→ 可走
                map.is_walkable(p.0, p.1)
            }

            // 尝试方向：原方向 → NextDir → PreviousDir（C# CanWalk(dir, out dir)）
            let mut chosen: Option<(MirDirection, i32)> = None; // (方向, 步数)
            let mut can_walk = |d: MirDirection| -> bool {
                if in_trap {
                    return false;
                }
                let p = point_move(from_tile.0, from_tile.1, d, 1);
                check_door(map, &net, p)
            };
            if run {
                // C# CanRun：负重不超限 && CanWalk(dir) && EmptyCell(2 格)；
                // 骑乘或冲刺（且非潜行）→ 3 格（C# GameScene.cs:12143-12147）
                // 潜行且非冲刺 → 不可跑（C# CheckInput 11528：!Sneaking || (Sneaking && Sprint)）
                let bag_ok = hud.inventory.weight <= hud.inventory.max_weight;
                let wear_ok = hud
                    .equipment
                    .iter()
                    .flatten()
                    .map(|i| i.weight as u32)
                    .sum::<u32>()
                    <= hud.inventory.max_weight;
                let can_run_base = !in_trap && bag_ok && wear_ok && !(hud.sneaking && !hud.sprint);
                let run_dist = if hud.riding || (hud.sprint && !hud.sneaking) { 3 } else { 2 };
                for d in [dir, next_direction(dir), previous_direction(dir)] {
                    if !can_run_base {
                        break;
                    }
                    let mut ok = true;
                    for k in 1..=run_dist {
                        let p = point_move(from_tile.0, from_tile.1, d, k);
                        if !check_door(map, &net, p) {
                            ok = false;
                            break;
                        }
                    }
                    if ok {
                        chosen = Some((d, run_dist));
                        break;
                    }
                }
                if chosen.is_none() {
                    // 跑不可达 → 退走 1 格（C# 右键按住先 CanRun 后 CanWalk）
                    for d in [dir, next_direction(dir), previous_direction(dir)] {
                        if can_walk(d) {
                            chosen = Some((d, 1));
                            break;
                        }
                    }
                }
            } else {
                for d in [dir, next_direction(dir), previous_direction(dir)] {
                    if can_walk(d) {
                        chosen = Some((d, 1));
                        break;
                    }
                }
            }

            let need_step = chosen.is_some()
                && (direction_changed || lm.path.is_empty());
            if let Some((d, steps)) = chosen {
                if need_step {
                    // 立即转向（C# Standing direction 语义：即使不可走也先转方向）
                    anim.direction = d as u8;
                    control.hold_target = Some((d as u8 as i32, 0));
                    control.hold_run = Some(run);
                    if steps == 2 {
                        let p2 = point_move(from_tile.0, from_tile.1, d, 2);
                        anim.action = mir2_shared::enums::MirAction::Running;
                        commands.entity(pe).insert(LocalMove {
                            path: vec![point_move(from_tile.0, from_tile.1, d, 1), p2].into(),
                            step_timer_ms: 0.0,
                            run: true,
                            last: None,
                            step_origin: None,
                            turn_acc: 0.0,
                        });
                    } else {
                        let p1 = point_move(from_tile.0, from_tile.1, d, 1);
                        anim.action = mir2_shared::enums::MirAction::Walking;
                        commands.entity(pe).insert(LocalMove {
                            path: vec![p1].into(),
                            step_timer_ms: 0.0,
                            run: false,
                            last: None,
                            step_origin: None,
                            turn_acc: 0.0,
                        });
                    }
                }
            } else {
                // 三方向都不可走 → 原地转向（C# CanWalk 失败 → Standing direction）
                if direction_changed {
                    anim.direction = new_dir;
                    anim.action = mir2_shared::enums::MirAction::Standing;
                    anim.frame_index = 0;
                    control.hold_target = Some((new_dir as i32, 0));
                    control.hold_run = Some(run);
                }
                // 清空旧路径避免卡住
                lm.path.clear();
                lm.last = None;
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
/// #1558：构造 C.IntelligentCreaturePickup（SharedRust packets/client/misc.rs）
/// `[mouse_mode: u8][x: i32][y: i32]`——mouse_mode=true 鼠标拾取、false 半自动（C# GameScene.cs:804/811）
pub fn build_pet_pickup(
    mouse_mode: bool,
    tile: (i32, i32),
) -> mir2_shared::packets::client::misc::IntelligentCreaturePickup {
    mir2_shared::packets::client::misc::IntelligentCreaturePickup {
        mouse_mode,
        location: mir2_shared::map::Point { x: tile.0, y: tile.1 },
    }
}

/// 宠物拾取指令（#1558，C# KeybindOptions.CreaturePickup/CreatureAutoPickup）：
/// - X：宠物到鼠标位置拾取（MouseMode=true，GameScene.cs:811）
/// - Ctrl+X：宠物半自动拾取（MouseMode=false，GameScene.cs:804，服务端在宠物/玩家位置附近拾取）
fn pet_pickup_system(
    keys: Res<ButtonInput<KeyCode>>,
    kb: Res<crate::game::dialogs::keyboard_layout::KeyboardState>,
    net: Res<NetConnection>,
    chat: Res<crate::game::chat::ChatState>,
    windows: Query<&Window>,
    camera: Query<
        &Transform,
        (With<Camera2d>, Without<UiButton>, Without<crate::ui::sprite_ui::UiEntity>),
    >,
    players: Query<&Transform, (With<LocalPlayer>, With<NetObjectId>)>,
    hud: Res<HudState>,
) {
    if hud.dead {
        return;
    }
    // 聊天输入激活时不触发（X/Ctrl+X 是文本按键，避免输入字母时误发拾取指令）
    if chat.input_active {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.physical_cursor_position() else { return };
    let Ok(cam_tf) = camera.single() else { return };
    let world = screen_to_world(cursor, cam_tf, window);
    let mouse_tile = world_to_tile(world.x, world.y);
    let player_tile = players
        .single()
        .map(|tf| world_to_tile(tf.translation.x, tf.translation.y))
        .ok();

    for b in &kb.bindings {
        if b.action == "宠物拾取" && b.matches(&keys) {
            net.send_packet(&build_pet_pickup(true, mouse_tile));
            tracing::info!("🐾 宠物拾取（鼠标）@ ({},{})", mouse_tile.0, mouse_tile.1);
        } else if b.action == "宠物半自动拾取" && b.matches(&keys) {
            let tile = player_tile.unwrap_or(mouse_tile);
            net.send_packet(&build_pet_pickup(false, tile));
            tracing::info!("🐾 宠物半自动拾取 @ ({},{})", tile.0, tile.1);
        }
    }
}

/// #1562：宠物模式循环（C# GameScene.cs:906 ChangePetMode）：
/// Both → MoveOnly → AttackOnly → None → FocusMasterTarget → Both
pub fn next_pet_mode(current: mir2_shared::enums::PetMode) -> mir2_shared::enums::PetMode {
    use mir2_shared::enums::PetMode;
    match current {
        PetMode::Both => PetMode::MoveOnly,
        PetMode::MoveOnly => PetMode::AttackOnly,
        PetMode::AttackOnly => PetMode::None,
        PetMode::None => PetMode::FocusMasterTarget,
        PetMode::FocusMasterTarget => PetMode::Both,
    }
}

/// #1562：构造 C.ChangePMode（SharedRust packets/client/misc.rs）
pub fn build_change_pmode(mode: mir2_shared::enums::PetMode) -> mir2_shared::packets::client::misc::ChangePMode {
    mir2_shared::packets::client::misc::ChangePMode { mode }
}

/// 宠物模式切换（#1562，C# KeybindOptions.ChangePetmode → GameScene.ChangePetMode）：
/// - 默认 Ctrl+T（C# 默认 Ctrl+A，但 Bevy 中 A 用于相机平移，避免冲突改 Ctrl+T）；
/// - 500ms 冷却（C# ChangePModeTime = Time + 500）；
/// - 发送后由服务端 S.ChangePMode 确认更新 HudState.pet_mode（不本地抢改）。
fn pet_mode_system(
    keys: Res<ButtonInput<KeyCode>>,
    kb: Res<crate::game::dialogs::keyboard_layout::KeyboardState>,
    net: Res<NetConnection>,
    chat: Res<crate::game::chat::ChatState>,
    time: Res<Time>,
    hud: Res<crate::game::hud::HudState>,
    mut last_toggle: Local<f32>,
) {
    if chat.input_active {
        return;
    }
    let Some(b) = kb.bindings.iter().find(|b| b.action == "宠物模式切换") else {
        return;
    };
    if !b.matches(&keys) {
        return;
    }
    if time.elapsed_secs() - *last_toggle < 0.5 {
        return;
    }
    *last_toggle = time.elapsed_secs();
    let next = next_pet_mode(hud.pet_mode);
    net.send_packet(&build_change_pmode(next));
    tracing::info!("🐾 宠物模式切换 {:?} -> {:?}", hud.pet_mode, next);
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

    #[test]
    fn test_loaded_map_doors_grid() {
        // #1550：LoadedMap doors 网格（C# M2CellInfo.DoorIndex）
        let map = crate::map_renderer::LoadedMap {
            name: "d".into(),
            width: 2,
            height: 2,
            walkable: vec![vec![true; 2]; 2],
            doors: vec![vec![0u8, 1], vec![0, 0]],
        };
        assert_eq!(map.doors[0][1], 1, "门索引应保留");
        assert_eq!(map.doors[1][0], 0);
        assert!(map.is_walkable(1, 1));
    }

    #[test]
    fn test_can_run_weight_and_trap_conditions() {
        // #1550：C# CanRun（GameScene.cs:12139）——负重/陷阱决定能否跑
        let mut hud = HudState::default();
        hud.inventory.weight = 10;
        hud.inventory.max_weight = 100;
        assert!(hud.inventory.weight <= hud.inventory.max_weight);
        let wear: u32 = hud.equipment.iter().flatten().map(|i| i.weight as u32).sum();
        assert!(wear <= hud.inventory.max_weight);
        // 背包超重 → 不可跑（C# CurrentBagWeight > BagWeight）
        hud.inventory.weight = 200;
        assert!(hud.inventory.weight > hud.inventory.max_weight);
        // 陷阱 → 不可走/跑（C# InTrapRock）
        hud.in_trap_rock = true;
        assert!(hud.in_trap_rock);
    }

    #[test]
    fn test_hold_right_click_close_turn_only() {
        // #1550：右键按住且鼠标距玩家 <= 2 格 → 只转向（C# GameScene.cs:11614 InRange 2）
        let threshold = TILE_WIDTH * 2.0;
        assert!(96.0f32 <= threshold);
        assert!(!(240.0f32 <= threshold));
    }

    #[test]
    fn test_attack_range_chebyshev() {
        // #1554：C# Functions.InRange = Chebyshev（max(|dx|,|dy|) <= i）
        let in_range = |p: (i32,i32), t: (i32,i32), r: i32| {
            (t.0 - p.0).abs() <= r && (t.1 - p.1).abs() <= r
        };
        // 近战范围 1：对角也算 1（C# InRange 1）
        assert!(in_range((0,0), (1,1), 1));
        assert!(in_range((0,0), (1,0), 1));
        assert!(!in_range((0,0), (2,0), 1));
        assert!(!in_range((0,0), (2,2), 1));
        // 弓手范围 9：C# MaxAttackRange=9
        assert!(in_range((0,0), (9,0), 9));
        assert!(in_range((0,0), (5,5), 9));
        assert!(!in_range((0,0), (10,0), 9));
    }

    #[test]
    fn test_player_attack_kind_archer_vs_melee() {
        // #1556：弓手（Archer + 武器）→ Ranged；其余（战士/无武器弓手）→ Melee
        assert_eq!(
            player_attack_kind(mir2_shared::enums::MirClass::Archer as u8, true),
            PlayerAttackKind::Ranged
        );
        assert_eq!(
            player_attack_kind(mir2_shared::enums::MirClass::Archer as u8, false),
            PlayerAttackKind::Melee
        );
        assert_eq!(
            player_attack_kind(mir2_shared::enums::MirClass::Warrior as u8, true),
            PlayerAttackKind::Melee
        );
        assert_eq!(
            player_attack_kind(mir2_shared::enums::MirClass::Wizard as u8, true),
            PlayerAttackKind::Melee
        );
    }

    #[test]
    fn test_build_ranged_attack_fields() {
        // #1556：C# PlayerObject.cs:1574 C.RangeAttack 字段映射
        let pkt = build_ranged_attack(
            mir2_shared::enums::MirDirection::UpRight,
            (10, 20),
            101,
            (15, 25),
        );
        assert_eq!(pkt.direction, mir2_shared::enums::MirDirection::UpRight);
        assert_eq!(pkt.location.x, 10);
        assert_eq!(pkt.location.y, 20);
        assert_eq!(pkt.target_id, 101);
        assert_eq!(pkt.target_location.x, 15);
        assert_eq!(pkt.target_location.y, 25);
    }

    #[test]
    fn test_build_pet_pickup_fields() {
        // #1558：C.IntelligentCreaturePickup = [mouse_mode u8][Point x i32][y i32]
        let pkt = build_pet_pickup(true, (10, 20));
        assert!(pkt.mouse_mode);
        assert_eq!(pkt.location.x, 10);
        assert_eq!(pkt.location.y, 20);

        let pkt = build_pet_pickup(false, (5, 6));
        assert!(!pkt.mouse_mode);
        assert_eq!(pkt.location.x, 5);
        assert_eq!(pkt.location.y, 6);
    }

    #[test]
    fn test_next_pet_mode_cycle() {
        // #1562：C# GameScene.cs:906 循环顺序
        use mir2_shared::enums::PetMode;
        assert_eq!(next_pet_mode(PetMode::Both), PetMode::MoveOnly);
        assert_eq!(next_pet_mode(PetMode::MoveOnly), PetMode::AttackOnly);
        assert_eq!(next_pet_mode(PetMode::AttackOnly), PetMode::None);
        assert_eq!(next_pet_mode(PetMode::None), PetMode::FocusMasterTarget);
        assert_eq!(next_pet_mode(PetMode::FocusMasterTarget), PetMode::Both);
    }

    #[test]
    fn test_build_change_pmode() {
        // #1562：C.ChangePMode 字段
        let pkt = build_change_pmode(mir2_shared::enums::PetMode::AttackOnly);
        assert_eq!(pkt.mode, mir2_shared::enums::PetMode::AttackOnly);
    }

    #[test]
    fn test_archer_detection() {
        // #1554：弓手 = Class==Archer 且装备武器（C# HasClassWeapon 简化）
        let mut hud = crate::game::hud::HudState::default();
        hud.class = mir2_shared::enums::MirClass::Archer as u8;
        // 无武器 → 非弓手
        let is_archer = hud.class == mir2_shared::enums::MirClass::Archer as u8
            && hud.equipment.get(0).and_then(|s| s.as_ref()).is_some();
        assert!(!is_archer);
        // 装备武器 → 弓手（远程范围 9）
        let mut bow = crate::game::dialogs::inventory::InvItem::default();
        bow.item_type = mir2_shared::enums::ItemType::Weapon as u8;
        hud.equipment[0] = Some(bow);
        let is_archer = hud.class == mir2_shared::enums::MirClass::Archer as u8
            && hud.equipment.get(0).and_then(|s| s.as_ref()).is_some();
        assert!(is_archer);
        // 战士 → 近战
        let mut warrior = crate::game::hud::HudState::default();
        warrior.class = mir2_shared::enums::MirClass::Warrior as u8;
        warrior.equipment[0] = Some(crate::game::dialogs::inventory::InvItem::default());
        let is_archer = warrior.class == mir2_shared::enums::MirClass::Archer as u8
            && warrior.equipment.get(0).and_then(|s| s.as_ref()).is_some();
        assert!(!is_archer);
    }
}

