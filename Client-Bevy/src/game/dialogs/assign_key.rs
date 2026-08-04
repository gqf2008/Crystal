// ============================================================================
#![allow(clippy::type_complexity)]
// 技能快捷键分配面板（AssignKeyPanel，C# MainDialogs.cs ~3786）
// 布局参考：C# AssignKeyPanel
//   - 背景 Prguse[710] 居中（Location = Center）；魔法图标 MagIcon2[icon*2] (16,16)
//   - 标题 (49,17)；None 按钮 Title[287-289] (284,64)；Save Title[156-158] (284,101)
//   - 16 个 F 键按钮 Prguse[1656-1658]：(17+32*(i%8)+5*(i%8/4), 58+37*(i/8))
//   - Save：清除同键冲突 → C.MagicKey{Spell,Key,OldKey} → 本地更新 → 关闭
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;
use mir2_shared::enums::Spell;

use crate::game::skills::MagicsState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    ui_button_system, ui_image, ButtonFrames, UiButton, UiEntity, UiFont, UiImageCache,
};

/// 面板状态（C# AssignKeyPanel：Magic/Key；Save 时发包并本地更新）
#[derive(Resource, Default)]
pub struct AssignKeyState {
    pub visible: bool,
    pub spell: Option<Spell>,
    /// 当前选择的键（0 = 无；1..8 = F1..F8；9..16 = Ctrl+F1..F8）
    pub key: u8,
    /// 打开时的旧键（C.MagicKey.OldKey）
    pub old_key: u8,
}

impl AssignKeyState {
    pub fn open(&mut self, spell: Spell, key: u8) {
        self.visible = true;
        self.spell = Some(spell);
        self.key = key;
        self.old_key = key;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.spell = None;
        self.key = 0;
        self.old_key = 0;
    }
}

#[derive(Component)]
pub struct AssignKeyWidget;

#[derive(Component)]
pub struct AssignKeyIcon;

#[derive(Component)]
pub struct AssignKeyTitle;

#[derive(Component)]
pub struct AssignKeyNone;

#[derive(Component)]
pub struct AssignKeySave;

/// F 键按钮（0..15 = F1..F8 / Ctrl+F1..F8）
#[derive(Component)]
pub struct AssignKeyFKey(pub usize);

/// F 键三态帧（Prguse 1656/1657/1658；选中态固定用 pressed 帧）
#[derive(Component)]
pub struct AssignKeyFrames {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
}

pub struct AssignKeyPlugin;

impl Plugin for AssignKeyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssignKeyState>();
        app.add_systems(OnEnter(AppState::Game), spawn_assign_key_panel);
        app.add_systems(OnExit(AppState::Game), cleanup_assign_key_panel);
        app.add_systems(
            Update,
            (assign_key_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_assign_key_panel(mut commands: Commands, roots: Query<Entity, With<AssignKeyWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_assign_key_panel(
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

    // 背景 Prguse[710]，屏幕居中（原版 Location = Center）
    let Some(bg) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 710) else {
        return;
    };
    let size = images.get(&bg).map(|i| i.size()).unwrap_or(UVec2::new(350, 150));
    let (w, h) = (size.x as f32, size.y as f32);
    let (px, py) = ((1024.0 - w) / 2.0, (768.0 - h) / 2.0);
    let panel = commands
        .spawn((
            UiEntity,
            AssignKeyWidget,
            Sprite::from_image(bg),
            Anchor::TOP_LEFT,
            Transform::from_xyz(px, -py, 7.5),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(panel).with_children(|p| {
        // 魔法图标 MagIcon2[icon*2] (16,16)
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::MagIcon2, 0) {
            p.spawn((
                AssignKeyWidget,
                AssignKeyIcon,
                Sprite::from_image(h),
                Anchor::TOP_LEFT,
                Transform::from_xyz(16.0, -16.0, 7.6),
                Visibility::Hidden,
            ));
        }
        // 标题（C# TitleLabel (49,17)）
        p.spawn((
            AssignKeyWidget,
            AssignKeyTitle,
            Text2d::new(""),
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(49.0, -17.0, 7.6),
            Visibility::Hidden,
        ));
        // None 按钮 Title[287-289] (284,64)
        if let (Some(n), Some(hov), Some(pre)) = (
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 287),
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 288),
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 289),
        ) {
            let sz = images.get(&n).map(|i| i.size()).unwrap_or(UVec2::new(40, 20));
            p.spawn((
                AssignKeyWidget,
                AssignKeyNone,
                UiButton {
                    rect: (px + 284.0, py + 64.0, sz.x as f32, sz.y as f32),
                    clicked: false,
                },
                ButtonFrames {
                    normal: n.clone(),
                    hover: hov,
                    pressed: pre,
                },
                Sprite::from_image(n.clone()),
                Anchor::TOP_LEFT,
                Transform::from_xyz(284.0, -64.0, 7.6),
                Visibility::Hidden,
            ));
        }
        // Save 按钮 Title[156-158] (284,101)
        if let (Some(n), Some(hov), Some(pre)) = (
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 156),
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 157),
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 158),
        ) {
            let sz = images.get(&n).map(|i| i.size()).unwrap_or(UVec2::new(40, 20));
            p.spawn((
                AssignKeyWidget,
                AssignKeySave,
                UiButton {
                    rect: (px + 284.0, py + 101.0, sz.x as f32, sz.y as f32),
                    clicked: false,
                },
                ButtonFrames {
                    normal: n.clone(),
                    hover: hov,
                    pressed: pre,
                },
                Sprite::from_image(n.clone()),
                Anchor::TOP_LEFT,
                Transform::from_xyz(284.0, -101.0, 7.6),
                Visibility::Hidden,
            ));
        }
        // 16 个 F 键按钮（Prguse 1656/1657/1658）
        if let (Some(n), Some(hov), Some(pre)) = (
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1656),
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1657),
            ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 1658),
        ) {
            let sz = images.get(&n).map(|i| i.size()).unwrap_or(UVec2::new(32, 32));
            for i in 0..16usize {
                let rx = 17.0 + 32.0 * (i % 8) as f32 + 5.0 * ((i % 8) / 4) as f32;
                let ry = 58.0 + 37.0 * (i / 8) as f32;
                p.spawn((
                    AssignKeyWidget,
                    AssignKeyFKey(i),
                    AssignKeyFrames {
                        normal: n.clone(),
                        hover: hov.clone(),
                        pressed: pre.clone(),
                    },
                    UiButton {
                        rect: (px + rx, py + ry, sz.x as f32, sz.y as f32),
                        clicked: false,
                    },
                    Sprite::from_image(n.clone()),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(rx, -ry, 7.6),
                    Visibility::Hidden,
                ))
                .with_children(|c| {
                    // 键名文本（F1..F8 / Ctrl F1..F8）
                    let label = if i < 8 {
                        format!("F{}", i + 1)
                    } else {
                        format!("Ctrl F{}", i - 7)
                    };
                    c.spawn((
                        AssignKeyWidget,
                        Text2d::new(label),
                        Anchor::TOP_LEFT,
                        TextFont {
                            font: FontSource::Handle(font.clone()),
                            font_size: FontSize::Px(9.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Transform::from_xyz(4.0, -10.0, 7.7),
                        Visibility::Hidden,
                    ));
                });
            }
        }
    });
}

/// 显示/隐藏 + 图标/标题 + None/Save/F 键交互（C# AssignKeyPanel 逻辑）
fn assign_key_system(
    mut state: ResMut<AssignKeyState>,
    mut magics: ResMut<MagicsState>,
    net: Res<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut all_vis: Query<&mut Visibility, With<AssignKeyWidget>>,
    mut icon: Query<
        (&mut Sprite, &AssignKeyIcon),
        (Without<AssignKeyTitle>, Without<AssignKeyFKey>),
    >,
    mut title: Query<(&mut Text2d, &AssignKeyTitle), Without<AssignKeyIcon>>,
    mut actions: Query<
        (&UiButton, Option<&AssignKeyNone>, Option<&AssignKeySave>),
        Without<AssignKeyFKey>,
    >,
    mut fkeys: Query<(&UiButton, &AssignKeyFKey, &mut Sprite, &AssignKeyFrames)>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    for mut vis in &mut all_vis {
        *vis = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.visible {
        return;
    }

    // 图标 + 标题（C# MagicImage / TitleLabel）
    if let Some(m) = state.spell.and_then(|s| magics.by_spell(s)) {
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::MagIcon2,
            m.icon as usize * 2,
        ) {
            for (mut sprite, _) in &mut icon {
                if sprite.image != h {
                    sprite.image = h.clone();
                }
            }
        }
        for (mut t, _) in &mut title {
            t.0 = format!("选择 {} 的快捷键", m.name);
        }
    }

    // None / Save（C# NoneButton.Click / SaveButton.Click）
    for (btn, none, save) in &mut actions {
        if !btn.clicked {
            continue;
        }
        if none.is_some() {
            state.key = 0;
            tracing::info!("🔑 已选择：无快捷键");
        } else if save.is_some() {
            if let Some(spell) = state.spell {
                let old_key = magics.assign_key(spell, state.key).unwrap_or(state.old_key);
                net.send_packet(&mir2_shared::packets::client::combat::MagicKey {
                    spell,
                    key: state.key,
                    old_key,
                });
                tracing::info!(
                    "🔑 保存快捷键: {:?} → key={} (old={})",
                    spell,
                    state.key,
                    old_key
                );
            }
            state.close();
        }
    }

    // F 键：点击选择；选中态固定 pressed 帧（C# AssignKeyPanel_BeforeDraw）
    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    let mouse_down = mouse.pressed(MouseButton::Left);
    for (btn, fkey, mut sprite, frames) in &mut fkeys {
        if btn.clicked {
            state.key = (fkey.0 + 1) as u8;
        }
        let selected = state.key as usize == fkey.0 + 1;
        let over = cursor
            .map(|c| {
                let (x, y, w, h) = btn.rect;
                c.x >= x && c.x <= x + w && c.y >= y && c.y <= y + h
            })
            .unwrap_or(false);
        let frame = if selected || (mouse_down && over) {
            &frames.pressed
        } else if over {
            &frames.hover
        } else {
            &frames.normal
        };
        if sprite.image != *frame {
            sprite.image = frame.clone();
        }
    }
}
