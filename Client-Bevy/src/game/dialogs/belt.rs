// ============================================================================
// 快捷栏（M9 第 1 批）
// 参考：C# MainDialogs.cs SkillBarDialog / macroquad belt_dialog.rs
//   - 横向：背景 Prguse[1932]，8 格 32x32，间距 35，偏移 12
//   - 旋转按钮 Prguse[1926-1928]（横）/ [1938-1940]（纵）
//   - 关闭按钮 Prguse[1923-1925]（横）/ [1935-1937]（纵）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::skills::MagicsState;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, ui_button_system, ui_image, UiButton, UiEntity, UiFont, UiImageCache,
};

/// 快捷栏数据（技能/物品图标索引；后续由网络/技能系统写入）
#[derive(Resource, Default)]
pub struct BeltState {
    pub slots: [Option<u32>; 8],
}

#[derive(Component)]
pub struct BeltWidget;

#[derive(Component)]
pub struct BeltBg;

#[derive(Component)]
pub struct BeltRotate;

#[derive(Component)]
pub struct BeltClose;

#[derive(Component)]
pub struct BeltSlot;

/// 快捷格技能图标（子实体）
#[derive(Component, Clone, Copy)]
pub struct BeltIcon(pub usize);

#[derive(Resource, Default)]
pub struct BeltVisible(pub bool);

#[derive(Resource, Default)]
pub struct BeltVertical(pub bool);

const CELL_SIZE: f32 = 32.0;
const CELL_SPACING: f32 = 35.0;
const CELL_OFFSET: f32 = 12.0;

pub struct BeltPlugin;

impl Plugin for BeltPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BeltState>();
        // 原版 C# SkillBarDialog 默认显示（F1-F8 快捷技能栏）
        app.insert_resource(BeltVisible(true));
        app.init_resource::<BeltVertical>();
        app.add_systems(OnEnter(AppState::Game), spawn_belt);
        app.add_systems(OnExit(AppState::Game), cleanup_belt);
        app.add_systems(
            Update,
            (belt_ui_system, belt_icon_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_belt(mut commands: Commands, roots: Query<Entity, With<BeltWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_belt(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    _fonts: ResMut<Assets<Font>>,
    _ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();

    // 横向背景 Prguse[1932]（位置：主对话框上方，快捷栏默认 (400,600) 附近）
    let pos = (1024.0 - 330.0) / 2.0; // 底部居中偏上
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1932) {
        let e = spawn_ui_sprite(&mut commands, h, pos, 600.0, 5.5, 1.0);
        commands.entity(e).insert((BeltWidget, BeltBg, Visibility::Visible));
    }

    // 8 个格子
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for i in 0..8usize {
        let x = pos + CELL_OFFSET + i as f32 * CELL_SPACING;
        let slot = commands
            .spawn((
                UiEntity,
                BeltWidget,
                BeltSlot,
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.2),
                    custom_size: Some(Vec2::new(CELL_SIZE, CELL_SIZE)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(x, -603.0, 5.6),
                Visibility::Visible,
            ))
            .id();
        // 技能图标（MagIcon[icon*2]，原版 C# Cells[i].Index = magic.Icon * 2）
        commands.entity(slot).with_children(|p| {
            p.spawn((
                BeltIcon(i),
                Sprite {
                    image: white.clone(),
                    custom_size: Some(Vec2::new(CELL_SIZE - 4.0, CELL_SIZE - 4.0)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(2.0, -2.0, 5.7),
                Visibility::Hidden,
            ));
        });
    }

    // 旋转按钮（横 Prguse[1926-1928]）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 1926, 1927, 1928,
        pos + 3.0, 600.0 + 3.0, 5.6, 20.0, 20.0,
    ) {
        commands.entity(e).insert((BeltWidget, BeltRotate));
    }
    // 关闭按钮（横 Prguse[1923-1925]，右端）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 1923, 1924, 1925,
        pos + CELL_OFFSET + 8.0 * CELL_SPACING - 20.0, 600.0 + 3.0, 5.6, 20.0, 20.0,
    ) {
        commands.entity(e).insert((BeltWidget, BeltClose));
    }
}

/// 显示/隐藏 + 旋转/关闭
fn belt_ui_system(
    mut visible: ResMut<BeltVisible>,
    mut vertical: ResMut<BeltVertical>,
    mut widgets: Query<&mut Visibility, With<BeltWidget>>,
    rotate: Query<&UiButton, (With<BeltRotate>, Without<BeltClose>)>,
    close: Query<&UiButton, (With<BeltClose>, Without<BeltRotate>)>,
) {
    for mut vis in widgets.iter_mut() {
        *vis = if visible.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible.0 {
        return;
    }
    for btn in &rotate {
        if btn.clicked {
            vertical.0 = !vertical.0;
            tracing::info!("🔁 快捷栏旋转: {}", if vertical.0 { "纵向" } else { "横向" });
        }
    }
    for btn in &close {
        if btn.clicked {
            visible.0 = false;
        }
    }
}

/// 快捷栏技能图标：按 MagicsState 中 key 绑定（F1=1..F8=8）显示 MagIcon 图标
fn belt_icon_system(
    magics: Res<MagicsState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    cd: Res<crate::game::skills::MagicCooldowns>,
    mut icons: Query<(&mut Sprite, &mut Visibility, &BeltIcon), Without<BeltWidget>>,
) {
    for (mut sprite, mut vis, icon) in &mut icons {
        let slot = icon.0;
        let magic = magics.by_key(slot as u8 + 1);
        match magic {
            Some(m) => {
                let handle = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::MagIcon,
                    (m.icon as usize) * 2,
                );
                match handle {
                    Some(h) if sprite.image != h => sprite.image = h,
                    None => *vis = Visibility::Hidden,
                    _ => {}
                }
                if sprite.image.is_strong() {
                    *vis = Visibility::Visible;
                }
                // #1376：冷却暗化（对齐 C# CoolDowns 覆盖层；按剩余比例变暗）
                let frac = cd.fraction(m.spell);
                if frac > 0.0 {
                    let k = 1.0 - 0.7 * frac;
                    sprite.color = Color::srgba(k, k, k, 1.0);
                } else {
                    sprite.color = Color::WHITE;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}
