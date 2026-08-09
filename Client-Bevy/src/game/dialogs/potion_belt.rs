// ============================================================================
// 药水快捷腰带（#1362）
// 参考：C# BeltDialog（InventoryDialog.cs）：6 格 32x32，GridType=Inventory
//   - 选中背包物品 → 点腰带格 指派（存 unique_id）
//   - 点腰带格 → C.UseItem（使用药水）；右键 → 清除
// 与技能快捷栏（belt.rs 8 格 F1-F8）并存
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::inventory::{try_use_belt_item, InvClickState, InvItem, ItemUseFeedback};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_image, UiEntity, UiFont, UiImageCache,
};

/// 药水腰带格数（C# BeltDialog 6 格）
const BELT_SLOTS: usize = 6;
const CELL_SIZE: f32 = 32.0;
const CELL_SPACING: f32 = 35.0;
const CELL_OFFSET: f32 = 12.0;
/// 位置（主对话框左下，避开底部居中的技能快捷栏）
const BELT_X: f32 = 60.0;
const BELT_Y: f32 = 618.0;

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

#[derive(Component)]
pub struct PotionBeltWidget;

#[derive(Component)]
pub struct PotionBeltSlot(usize);

#[derive(Component)]
pub struct PotionBeltIcon(usize);

#[derive(Component)]
pub struct PotionBeltCount(usize);

pub struct PotionBeltPlugin;

impl Plugin for PotionBeltPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PotionBeltState>();
        app.init_resource::<PotionBeltVisible>();
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

    // 背景（简单深色条；C# Prguse[1932] 已被技能栏占用）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    commands.spawn((
        UiEntity,
        PotionBeltWidget,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.1, 0.1, 0.15, 0.85),
            custom_size: Some(Vec2::new(
                BELT_SLOTS as f32 * CELL_SPACING + 8.0,
                CELL_SIZE + 8.0,
            )),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(BELT_X - 4.0, -(BELT_Y - 4.0), 5.4),
        Visibility::Visible,
    ));

    for i in 0..BELT_SLOTS {
        let x = BELT_X + CELL_OFFSET + i as f32 * CELL_SPACING;
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
                Transform::from_xyz(x, -BELT_Y, 5.5),
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
        // 数字标签
        let k = spawn_ui_text(
            &mut commands,
            &font,
            &(i + 1).to_string(),
            x + 2.0,
            BELT_Y + CELL_SIZE - 12.0,
            10.0,
            Color::WHITE,
            5.7,
        );
        commands.entity(k).insert((PotionBeltWidget,));
    }
}

/// 交互：选中物品指派 / 点击使用 / 右键清除
#[allow(clippy::too_many_arguments)]
fn potion_belt_ui_system(
    mut belt: ResMut<PotionBeltState>,
    visible: Res<PotionBeltVisible>,
    hud: Res<HudState>,
    net: Res<NetConnection>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut click: ResMut<InvClickState>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<PotionBeltWidget>>,
    slots: Query<&PotionBeltSlot>,
) {
    // #1370：腰带显隐（Z 快捷键）
    for mut vis in &mut widgets {
        *vis = if visible.0 { Visibility::Visible } else { Visibility::Hidden };
    }
    if !visible.0 {
        return;
    }
    let now = time.elapsed_secs_f64();
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // 定位点中的腰带格
    let mut hit: Option<usize> = None;
    for s in &slots {
        let x = BELT_X + CELL_OFFSET + s.0 as f32 * CELL_SPACING;
        if cursor.x >= x
            && cursor.x <= x + CELL_SIZE
            && cursor.y >= BELT_Y
            && cursor.y <= BELT_Y + CELL_SIZE
        {
            hit = Some(s.0);
            break;
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
        // 有物品 → 使用（C# BeltGrid 点击使用药水）
        // #1544：腰带使用走节流/钓鱼守卫（C# BeltDialog.Grid[i].UseItem）
        if try_use_belt_item(uid, &net, &hud, now, &mut feedback) {
            tracing::info!("🧪 使用腰带物品 uid={}", uid);
        }
    } else if let Some(sel) = click.selected {
        // 无物品 + 背包有选中 → 指派（C# 拖入腰带格）
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

