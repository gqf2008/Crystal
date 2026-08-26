// ============================================================================
// 药水快捷腰带（#1362）
// 参考：C# BeltDialog（InventoryDialog.cs）：6 格 32x32，GridType=Inventory
//   - 选中背包物品 → 点腰带格 指派（存 unique_id）
//   - 点腰带格 → C.UseItem（使用药水）；右键 → 清除
//   - 背景 Prguse[1932] + BeforeDraw 叠 Prguse[1933]（0.5 alpha）
//   - 位置 (MainDialog.X+230, ScreenHeight-150) = (230,618)
//   - 旋转按钮 Prguse[1926-1928] @(222,3)、关闭按钮 Prguse[1923-1925] @(222,19)
//   - 6 格 @(12+i*35, 3)，数字 1-6 @(8+i*35, 2)
// 与技能快捷栏（skills.rs 左上角 F1-F8）并存；belt.rs 旧底部 8 格条已废弃
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{InvClickState, InvItem, ItemUseFeedback, try_use_belt_item};
use crate::game::player_state::{Inventory, StatusFlags};
use crate::game::sets::GameSet;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
    ImageButton,
};

/// 药水腰带格数（C# BeltDialog 6 格）
const BELT_SLOTS: usize = 6;
const CELL_SIZE: f32 = 32.0;
const CELL_SPACING: f32 = 35.0;
/// 横向（C# Location (230,618)）
const BELT_X: f32 = 230.0;
const BELT_Y: f32 = 618.0;
/// 纵向（C# Flip Location (0,200)）
const BELT_VERT_X: f32 = 0.0;
const BELT_VERT_Y: f32 = 200.0;

/// 药水腰带状态（每格存背包物品 unique_id）
#[derive(Resource, Default)]
pub struct PotionBeltState {
    pub slots: [Option<u64>; BELT_SLOTS],
}

/// #1370：药水腰带显隐（C# BeltDialog 默认可见；Z 快捷键切换）
#[derive(Resource)]
pub struct PotionBeltVisible(pub bool);

impl Default for PotionBeltVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// 横/纵布局（C# BeltDialog.Flip）
#[derive(Resource, Default)]
pub struct PotionBeltVertical(pub bool);

#[derive(Component)]
pub struct PotionBeltWidget;

#[derive(Component)]
pub struct PotionBeltSlot(usize);

#[derive(Component)]
pub struct PotionBeltIcon(usize);

#[derive(Component)]
pub struct PotionBeltCount(usize);

/// 数字标签 1-6（C# Key[i]）
#[derive(Component)]
pub struct PotionBeltNumber(usize);

/// 背景（Prguse[1932]/[1944] 随横纵切换）
#[derive(Component)]
pub struct PotionBeltBg;

/// 半透明叠层（Prguse[1933]/[1945]，C# BeltPanel_BeforeDraw 0.5 alpha）
#[derive(Component)]
pub struct PotionBeltBgOverlay;

/// 旋转按钮（C# RotateButton）
#[derive(Component)]
pub struct PotionBeltRotate;

/// 关闭按钮（C# CloseButton）
#[derive(Component)]
pub struct PotionBeltClose;

pub struct PotionBeltPlugin;

impl Plugin for PotionBeltPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PotionBeltState>();
        app.init_resource::<PotionBeltVisible>();
        app.init_resource::<PotionBeltVertical>();
        app.add_systems(OnEnter(AppState::Game), spawn_potion_belt);
        app.add_systems(OnExit(AppState::Game), cleanup_potion_belt);
        // #2633 批次4：腰带补货 ServerEvent 写系统（玩家状态集）。须在 inventory_events
        // 扣减之前运行——used_item_index 取扣减前的物品 item_index（§12 R6）。
        app.add_systems(
            Update,
            belt_restock_events
                .before(crate::game::dialogs::inventory::inventory_events)
                .in_set(GameSet::PlayerState)
                .run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (potion_belt_ui_system, potion_belt_icon_system).run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_potion_belt(mut commands: Commands, roots: Query<Entity, With<PotionBeltWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 腰带自动补货写系统（#2633 批次4 步2：拆 hud_server_events 的 ItemUsed 补货段，设计 §10）。
///
/// #2633 批次4 步9：物品直读 `Inventory` 组件（HudState 已删）。
/// 须在 `inventory_events` 扣减**之前**运行——`used_item_index` 取扣减前的物品 item_index：
/// 被消耗物品 count==1 时扣减后即从背包移除，后置会读不到（§12 R6）。补货查找按
/// `unique_id != 已消耗` 排除自身，故与扣减的相对先后不影响查找结果（只影响 used_item_index
/// 的读取），因此只需保证本系统先读。背包扣减本身归 inventory_events（两系统读同一事件）。
pub(crate) fn belt_restock_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut belt: ResMut<PotionBeltState>,
) {
    use crate::network::server_event::ServerEvent;
    // R1：实体未生成则无背包可补货（原 HudState 默认空背包同样无补货，等价）
    let Ok(inv) = inv_q.single() else {
        return;
    };
    for ev in events.read() {
        if let ServerEvent::ItemUsed { unique_id } = ev {
            // #1544：扣减前记录被消耗物品的 item_index（腰带补货据此找同物品补上）
            let used_item_index = inv
                .items
                .iter()
                .find(|s| s.as_ref().map(|it| it.unique_id) == Some(*unique_id))
                .and_then(|s| s.as_ref())
                .map(|it| it.item_index);
            // #1544：腰带自动补货（C# MirItemCell.UseItem count==1 && ItemSlot < BeltIdx →
            // 背包找同物品 MoveItem 到腰带）；Bevy 腰带为 unique_id 虚拟槽：消耗后找同 item_index 补上
            if let Some(used_index) = used_item_index {
                for slot in belt.slots.iter_mut() {
                    if *slot == Some(*unique_id) {
                        let next = inv
                            .items
                            .iter()
                            .flatten()
                            .find(|it| it.unique_id != *unique_id && it.item_index == used_index)
                            .map(|it| it.unique_id);
                        if let Some(uid) = next {
                            *slot = Some(uid);
                            tracing::info!("🧪 腰带补货 uid={} -> {}", unique_id, uid);
                        } else {
                            *slot = None;
                        }
                    }
                }
            }
        }
    }
}

/// 横向：格 @(12+i*35,3)、数字 @(8+i*35,2)、旋转 @(222,3)、关闭 @(222,19)
fn h_slot(i: usize) -> (f32, f32) {
    (BELT_X + 12.0 + i as f32 * CELL_SPACING, BELT_Y + 3.0)
}
fn h_num(i: usize) -> (f32, f32) {
    (BELT_X + 8.0 + i as f32 * CELL_SPACING, BELT_Y + 2.0)
}
/// 纵向：格 @(3, x*35+12)、数字 @(3, x*35+10)、旋转 @(19,222)、关闭 @(3,222)
fn v_slot(i: usize) -> (f32, f32) {
    (
        BELT_VERT_X + 3.0,
        BELT_VERT_Y + 12.0 + i as f32 * CELL_SPACING,
    )
}
fn v_num(i: usize) -> (f32, f32) {
    (
        BELT_VERT_X + 3.0,
        BELT_VERT_Y + 10.0 + i as f32 * CELL_SPACING,
    )
}

fn spawn_potion_belt(
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

    // 背景 Prguse[1932]（横 240x38 @ (230,618)）/ 纵向 Prguse[1944]（40x241 @ (0,200)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1932) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, BELT_X, BELT_Y, 240.0, 38.0, 15);
    commands
        .entity(panel)
        .insert((PotionBeltWidget, PotionBeltBg, Visibility::Visible));

    commands.entity(panel).with_children(|p| {
        // 半透明叠层 Prguse[1933]（C# BeltPanel_BeforeDraw 0.5 alpha；ImageNode.color 调透明度）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1933) {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(240.0),
                    height: Val::Px(38.0),
                    ..default()
                },
                ImageNode::new(h).with_color(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                PotionBeltWidget,
                PotionBeltBgOverlay,
                ZIndex(1),
            ));
        }

        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        for i in 0..BELT_SLOTS {
            // 格（相对面板：横 (12+35i,3) / 纵 (3,12+35i)，由 ui_system 按布局更新）
            spawn_container(p, 12.0 + i as f32 * CELL_SPACING, 3.0, CELL_SIZE, CELL_SIZE, 2)
                .insert((
                    PotionBeltWidget,
                    PotionBeltSlot(i),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                ))
                .with_children(|c| {
                    spawn_image(c, white.clone(), 2.0, 2.0, CELL_SIZE - 4.0, CELL_SIZE - 4.0, 3)
                        .insert((PotionBeltWidget, PotionBeltIcon(i), Visibility::Hidden));
                    spawn_label(c, &font, "", 16.0, 20.0, 10.0, Color::WHITE, 3)
                        .insert((PotionBeltWidget, PotionBeltCount(i), Visibility::Hidden));
                });
        }
        // 数字标签 1-6（C# Key[i]；相对面板：横 (8+35i,2) / 纵 (3,10+35i)）
        for i in 0..BELT_SLOTS {
            spawn_label(p, &font, &(i + 1).to_string(), 8.0 + i as f32 * CELL_SPACING, 2.0, 10.0, Color::WHITE, 3)
                .insert((PotionBeltWidget, PotionBeltNumber(i)));
        }
        // 旋转按钮（C# RotateButton Prguse[1926-1928] @(222,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1926),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1927),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1928),
        ) {
            spawn_icon_button(p, n, h, pr, 222.0, 3.0, 16.0, 15.0, 3)
                .insert((PotionBeltWidget, PotionBeltRotate));
        }
        // 关闭按钮（C# CloseButton Prguse[1923-1925] @(222,19)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1923),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1924),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1925),
        ) {
            spawn_icon_button(p, n, h, pr, 222.0, 19.0, 16.0, 15.0, 3)
                .insert((PotionBeltWidget, PotionBeltClose));
        }
    });
}

/// 交互：显隐 + 横纵布局 + 旋转/关闭 + 选中指派/点击使用/右键清除
/// 单查询（Option 组件区分角色）避免 Bevy 16 参数上限（#1374 同款 B0001 预防）。
#[allow(clippy::type_complexity)]
fn potion_belt_ui_system(
    mut belt: ResMut<PotionBeltState>,
    mut visible: ResMut<PotionBeltVisible>,
    mut vertical: ResMut<PotionBeltVertical>,
    player_q: Query<(&Inventory, &StatusFlags), With<LocalPlayer>>,
    net: Res<NetConnection>,
    mut feedback: ResMut<ItemUseFeedback>,
    // #2631：选中态归 inventory 所有，本系统只读（指派腰带给当前背包选中格）
    click: Res<InvClickState>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut items: Query<
        (
            Entity,
            &mut Node,
            &mut Visibility,
            Option<&mut ImageNode>,
            Option<&Interaction>,
            Option<&PotionBeltBg>,
            Option<&PotionBeltBgOverlay>,
            Option<&PotionBeltSlot>,
            Option<&PotionBeltNumber>,
            Option<&PotionBeltRotate>,
            Option<&PotionBeltClose>,
        ),
        With<PotionBeltWidget>,
    >,
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
    // 显隐（Z 快捷键 / 关闭按钮）
    for (_, _, mut vis, _, _, _, _, _, _, _, _) in &mut items {
        *vis = if visible.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible.0 {
        return;
    }

    let vert = vertical.0;
    // 面板：横 (230,618) 240x38 / 纵 (0,200) 40x241
    let (px, py, pw, ph) = if vert {
        (BELT_VERT_X, BELT_VERT_Y, 40.0, 241.0)
    } else {
        (BELT_X, BELT_Y, 240.0, 38.0)
    };

    for (e, mut node, _, mut img, inter, bg, overlay, slot, num, rot, cls) in &mut items {
        if bg.is_some() {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, if vert { 1944 } else { 1932 }) {
                if let Some(img) = img.as_mut() {
                    if img.image != h {
                        img.image = h;
                    }
                }
            }
            node.left = Val::Px(px);
            node.top = Val::Px(py);
            node.width = Val::Px(pw);
            node.height = Val::Px(ph);
        } else if overlay.is_some() {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, if vert { 1945 } else { 1933 }) {
                if let Some(img) = img.as_mut() {
                    if img.image != h {
                        img.image = h;
                    }
                    // C# BeltPanel_BeforeDraw：Prguse[1933/1945] 用 0.5F 透明度叠加；
                    // 纹理本身 99% 是不透明黑块，不降 alpha 会整块黑盖住腰带
                    img.color = Color::srgba(1.0, 1.0, 1.0, 0.5);
                }
            }
            node.left = Val::Px(0.0);
            node.top = Val::Px(0.0);
            node.width = Val::Px(pw);
            node.height = Val::Px(ph);
        } else if let Some(s) = slot {
            let (x, y) = if vert { v_slot(s.0) } else { h_slot(s.0) };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
        } else if let Some(n) = num {
            let (x, y) = if vert { v_num(n.0) } else { h_num(n.0) };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
        } else if rot.is_some() {
            let (x, y) = if vert {
                (BELT_VERT_X + 19.0, BELT_VERT_Y + 222.0)
            } else {
                (BELT_X + 222.0, BELT_Y + 3.0)
            };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
            if let Some(inter) = inter {
                if edge(e, inter, &mut prev_inter) {
                    vertical.0 = !vertical.0;
                    tracing::info!("🔁 药水腰带旋转: {}", if !vert { "纵向" } else { "横向" });
                }
            }
        } else if cls.is_some() {
            let (x, y) = if vert {
                (BELT_VERT_X + 3.0, BELT_VERT_Y + 222.0)
            } else {
                (BELT_X + 222.0, BELT_Y + 19.0)
            };
            node.left = Val::Px(x - px);
            node.top = Val::Px(y - py);
            if let Some(inter) = inter {
                if edge(e, inter, &mut prev_inter) {
                    visible.0 = false;
                    tracing::info!("🧪 关闭药水腰带");
                }
            }
        }
    }

    let now = time.elapsed_secs_f64();
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // 定位点中的腰带格（按当前横纵布局）
    let mut hit: Option<usize> = None;
    for (_, _, _, _, _, _, _, slot, _, _, _) in &items {
        if let Some(s) = slot {
            let (x, y) = if vert { v_slot(s.0) } else { h_slot(s.0) };
            if cursor.x >= x && cursor.x <= x + CELL_SIZE && cursor.y >= y && cursor.y <= y + CELL_SIZE {
                hit = Some(s.0);
                break;
            }
        }
    }
    let Some(i) = hit else { return };

    // 右键清除（C# 拖动移出语义的简化）
    if mouse.just_pressed(MouseButton::Right) {
        belt.slots[i] = None;
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let player = player_q.single().ok();
    if let Some(uid) = belt.slots[i] {
        if try_use_belt_item(
            uid,
            &net,
            player.map(|(_, f)| f.fishing).unwrap_or(false),
            now,
            &mut feedback,
        ) {
            tracing::info!("🧪 使用腰带物品 uid={}", uid);
        }
    } else if let Some(sel) = click.selected() {
        if let Some(item) = player.and_then(|(inv, _)| inv.items.get(sel).and_then(|s| s.as_ref()))
        {
            belt.slots[i] = Some(item.unique_id);
            tracing::info!(
                "🧪 指派腰带 {}: {} (uid={})",
                i + 1,
                item.name,
                item.unique_id
            );
        }
    }
}

/// 渲染：图标/数量（从背包按 unique_id 找物品；#2633 批次4 步9 改读 `Inventory` 组件）
#[allow(clippy::too_many_arguments)]
fn potion_belt_icon_system(
    belt: Res<PotionBeltState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // #1374：B0001 修复（#1362 药水腰带合入后启动 panic）——icons 与 counts 的 &mut Visibility 需 Without 隔离
    mut icons: Query<(&mut ImageNode, &mut Visibility, &PotionBeltIcon), Without<PotionBeltCount>>,
    mut counts: Query<(&mut Text, &mut Visibility, &PotionBeltCount), Without<PotionBeltIcon>>,
) {
    let inv = inv_q.single().ok();
    let find = |i: usize| -> Option<&InvItem> {
        let uid = belt.slots.get(i).and_then(|u| u.as_ref())?;
        inv?.items
            .iter()
            .flatten()
            .find(|it| it.unique_id == *uid)
    };
    for (mut node, mut vis, icon) in &mut icons {
        if let Some(item) = find(icon.0) {
            if let Some(h) = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::Items,
                item.image as usize,
            ) {
                node.image = h;
            }
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
    for (mut text, mut vis, count) in &mut counts {
        if let Some(item) = find(count.0) {
            text.0 = if item.count > 1 {
                item.count.to_string()
            } else {
                String::new()
            };
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}
