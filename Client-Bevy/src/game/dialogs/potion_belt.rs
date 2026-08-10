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

use crate::game::dialogs::inventory::{try_use_belt_item, InvClickState, InvItem, ItemUseFeedback};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont, UiImageCache,
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

/// 横向：格 @(12+i*35,3)、数字 @(8+i*35,2)、旋转 @(222,3)、关闭 @(222,19)
fn h_slot(i: usize) -> (f32, f32) {
    (BELT_X + 12.0 + i as f32 * CELL_SPACING, BELT_Y + 3.0)
}
fn h_num(i: usize) -> (f32, f32) {
    (BELT_X + 8.0 + i as f32 * CELL_SPACING, BELT_Y + 2.0)
}
/// 纵向：格 @(3, x*35+12)、数字 @(3, x*35+10)、旋转 @(19,222)、关闭 @(3,222)
fn v_slot(i: usize) -> (f32, f32) {
    (BELT_VERT_X + 3.0, BELT_VERT_Y + 12.0 + i as f32 * CELL_SPACING)
}
fn v_num(i: usize) -> (f32, f32) {
    (BELT_VERT_X + 3.0, BELT_VERT_Y + 10.0 + i as f32 * CELL_SPACING)
}

fn spawn_potion_belt(
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

    // 背景 Prguse[1932] + 半透明叠层 Prguse[1933]（C# BeltPanel_BeforeDraw 0.5 alpha）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1932) {
        let e = spawn_ui_sprite(&mut commands, h, BELT_X, BELT_Y, 5.4, 1.0);
        commands.entity(e).insert((PotionBeltWidget, PotionBeltBg, Visibility::Visible));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1933) {
        let e = spawn_ui_sprite(&mut commands, h, BELT_X, BELT_Y, 5.41, 1.0);
        commands
            .entity(e)
            .insert((PotionBeltWidget, PotionBeltBgOverlay, Visibility::Visible));
    }

    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for i in 0..BELT_SLOTS {
        let (x, y) = h_slot(i);
        let slot = commands
            .spawn((
                UiEntity,
                PotionBeltWidget,
                PotionBeltSlot(i),
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.35),
                    custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(x, -y, 5.5),
                Visibility::Visible,
            ))
            .id();
        commands.entity(slot).with_children(|p| {
            p.spawn((
                PotionBeltIcon(i),
                Sprite {
                    image: white.clone(),
                    custom_size: Some(Vec2::new(CELL_SIZE - 4.0, CELL_SIZE - 4.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(2.0, -2.0, 5.6),
                Visibility::Hidden,
            ));
            p.spawn((
                PotionBeltCount(i),
                Text2d::new(String::new()),
                bevy::sprite::Anchor::BOTTOM_RIGHT,
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_xyz(30.0, -30.0, 5.7),
                Visibility::Hidden,
            ));
        });
        // 数字标签 1-6（C# Key[i]）
        let (nx, ny) = h_num(i);
        let k = spawn_ui_text(
            &mut commands,
            &font,
            &(i + 1).to_string(),
            nx,
            ny,
            10.0,
            Color::WHITE,
            5.7,
        );
        commands.entity(k).insert((PotionBeltWidget, PotionBeltNumber(i)));
    }

    // 旋转按钮（C# RotateButton Prguse[1926-1928] @(222,3)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 1926, 1927, 1928,
        BELT_X + 222.0, BELT_Y + 3.0, 5.5, 16.0, 15.0,
    ) {
        commands.entity(e).insert((PotionBeltWidget, PotionBeltRotate, Visibility::Visible));
    }
    // 关闭按钮（C# CloseButton Prguse[1923-1925] @(222,19)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 1923, 1924, 1925,
        BELT_X + 222.0, BELT_Y + 19.0, 5.5, 16.0, 15.0,
    ) {
        commands.entity(e).insert((PotionBeltWidget, PotionBeltClose, Visibility::Visible));
    }
}

/// 交互：显隐 + 横纵布局 + 旋转/关闭 + 选中指派/点击使用/右键清除
#[allow(clippy::type_complexity)]
fn potion_belt_ui_system(
    mut belt: ResMut<PotionBeltState>,
    mut visible: ResMut<PotionBeltVisible>,
    mut vertical: ResMut<PotionBeltVertical>,
    hud: Res<HudState>,
    net: Res<NetConnection>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut click: ResMut<InvClickState>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // 关键：必须 With<PotionBeltWidget>，否则会匹配全屏所有 Sprite 实体，
    // 把其它 UI/地图/角色的可见性全部改掉（大量窗口闪烁/白块）
    mut items: Query<(
        &mut Visibility,
        &mut Transform,
        Option<&mut Sprite>,
        Option<&UiButton>,
        Option<&PotionBeltBg>,
        Option<&PotionBeltBgOverlay>,
        Option<&PotionBeltSlot>,
        Option<&PotionBeltNumber>,
        Option<&PotionBeltRotate>,
        Option<&PotionBeltClose>,
    ), (With<PotionBeltWidget>, Without<PotionBeltIcon>, Without<PotionBeltCount>)>,
) {
    // 显隐（Z 快捷键 / 关闭按钮）
    for (mut vis, _, _, _, _, _, _, _, _, _) in &mut items {
        *vis = if visible.0 { Visibility::Visible } else { Visibility::Hidden };
    }
    if !visible.0 {
        return;
    }

    let vert = vertical.0;

    // 横纵布局：背景图/叠层/格/数字/按钮位置 + 旋转/关闭点击
    for (_, mut tf, mut sp, btn, bg, overlay, slot, num, rot, cls) in &mut items {
        if bg.is_some() {
            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, if vert { 1944 } else { 1932 }) {
                if let Some(sp) = sp.as_mut() {
                    sp.image = h;
                }
            }
            let (x, y) = if vert { (BELT_VERT_X, BELT_VERT_Y) } else { (BELT_X, BELT_Y) };
            tf.translation.x = x;
            tf.translation.y = -y;
        } else if overlay.is_some() {
            if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, if vert { 1945 } else { 1933 }) {
                if let Some(sp) = sp.as_mut() {
                    sp.image = h;
                }
            }
            let (x, y) = if vert { (BELT_VERT_X, BELT_VERT_Y) } else { (BELT_X, BELT_Y) };
            tf.translation.x = x;
            tf.translation.y = -y;
        } else if let Some(s) = slot {
            let (x, y) = if vert { v_slot(s.0) } else { h_slot(s.0) };
            tf.translation.x = x;
            tf.translation.y = -y;
        } else if let Some(n) = num {
            let (x, y) = if vert { v_num(n.0) } else { h_num(n.0) };
            tf.translation.x = x;
            tf.translation.y = -y;
        } else if rot.is_some() {
            let (x, y) = if vert { (BELT_VERT_X + 19.0, BELT_VERT_Y + 222.0) } else { (BELT_X + 222.0, BELT_Y + 3.0) };
            tf.translation.x = x;
            tf.translation.y = -y;
            if let Some(b) = btn {
                if b.clicked {
                    vertical.0 = !vertical.0;
                    tracing::info!("🔁 药水腰带旋转: {}", if !vert { "纵向" } else { "横向" });
                }
            }
        } else if cls.is_some() {
            let (x, y) = if vert { (BELT_VERT_X + 3.0, BELT_VERT_Y + 222.0) } else { (BELT_X + 222.0, BELT_Y + 19.0) };
            tf.translation.x = x;
            tf.translation.y = -y;
            if let Some(b) = btn {
                if b.clicked {
                    visible.0 = false;
                    tracing::info!("🧪 关闭药水腰带");
                }
            }
        }
    }

    let now = time.elapsed_secs_f64();
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // 定位点中的腰带格（按当前横纵布局）
    let mut hit: Option<usize> = None;
    for (_, _, _, _, _, _, slot, _, _, _) in &items {
        if let Some(s) = slot {
            let (x, y) = if vert { v_slot(s.0) } else { h_slot(s.0) };
            if cursor.x >= x
                && cursor.x <= x + CELL_SIZE
                && cursor.y >= y
                && cursor.y <= y + CELL_SIZE
            {
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
    if let Some(uid) = belt.slots[i] {
        if try_use_belt_item(uid, &net, &hud, now, &mut feedback) {
            tracing::info!("🧪 使用腰带物品 uid={}", uid);
        }
    } else if let Some(sel) = click.selected {
        if let Some(item) = hud.inventory.items.get(sel).and_then(|s| s.as_ref()) {
            belt.slots[i] = Some(item.unique_id);
            tracing::info!("🧪 指派腰带 {}: {} (uid={})", i + 1, item.name, item.unique_id);
        }
    }
}

/// 渲染：图标/数量（从背包按 unique_id 找物品）
#[allow(clippy::too_many_arguments)]
fn potion_belt_icon_system(
    belt: Res<PotionBeltState>,
    hud: Res<HudState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // #1374：B0001 修复（#1362 药水腰带合入后启动 panic）——icons 与 counts 的 &mut Visibility 需 Without 隔离
    mut icons: Query<(&mut Sprite, &mut Visibility, &PotionBeltIcon), Without<PotionBeltCount>>,
    mut counts: Query<(&mut Text2d, &mut Visibility, &PotionBeltCount), Without<PotionBeltIcon>>,
) {
    let find = |i: usize| -> Option<&InvItem> {
        let uid = belt.slots.get(i).and_then(|u| u.as_ref())?;
        hud.inventory.items.iter().flatten().find(|it| it.unique_id == *uid)
    };
    for (mut sprite, mut vis, icon) in &mut icons {
        if let Some(item) = find(icon.0) {
            if let Some(h) = ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                item.image as usize,
            ) {
                sprite.image = h;
                sprite.custom_size = None;
            }
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
    for (mut text, mut vis, count) in &mut counts {
        if let Some(item) = find(count.0) {
            text.0 = if item.count > 1 { item.count.to_string() } else { String::new() };
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

