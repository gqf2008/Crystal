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
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{load_lib_image, spawn_icon_button, spawn_image, spawn_label, spawn_panel, ImageButton};

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
            assign_key_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_assign_key_panel(mut commands: Commands, roots: Query<Entity, With<AssignKeyWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// Assign 面板按钮标签（#2584）：C# AssignKeyPanel 按钮文本带
/// Environment.NewLine（MainDialogs.cs:3302-3309 Shift、:3325-3332 Ctrl）——
/// "Ctrl\nF1" 双行；1..8 无前缀不受换行影响。技能列表行内后缀
/// （skills.rs " [Ctrl F1]"）仍是单行空格格式，二者是不同 C# 站点。
pub fn assign_key_label(key: u8) -> String {
    crate::game::skills::skill_key_name(key).replace(' ', "\n")
}

fn spawn_assign_key_panel(
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

    // 背景 Prguse[710]（380x144），屏幕居中（原版 Location = Center）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 710) else {
        return;
    };
    let (w, h) = (380.0, 144.0);
    let (px, py) = ((1024.0 - w) / 2.0, (768.0 - h) / 2.0);
    let panel = spawn_panel(&mut commands, bg, px, py, w, h, 60);
    commands.entity(panel).insert(AssignKeyWidget);

    commands.entity(panel).with_children(|p| {
        // 魔法图标 MagIcon2[icon*2] (16,16)，尺寸 36x34
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::MagIcon2, 0) {
            spawn_image(p, h, 16.0, 16.0, 36.0, 34.0, 9).insert(AssignKeyIcon);
        }
        // 标题（C# TitleLabel (49,17)）
        spawn_label(p, &font, "", 49.0, 17.0, 12.0, Color::WHITE, 9).insert(AssignKeyTitle);
        // None 按钮 Title[287-289] (284,64)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 287),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 288),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 289),
        ) {
            spawn_icon_button(p, n, h, pr, 284.0, 64.0, 76.0, 25.0, 10).insert(AssignKeyNone);
        }
        // Save 按钮 Title[156-158] (284,101)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 156),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 157),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 158),
        ) {
            spawn_icon_button(p, n, h, pr, 284.0, 101.0, 60.0, 25.0, 10).insert(AssignKeySave);
        }
        // 16 个 F 键按钮（Prguse 1656/1657/1658，32x32）
        if let (Some(n), Some(hov), Some(pre)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1656),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1657),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1658),
        ) {
            for i in 0..16usize {
                let rx = 17.0 + 32.0 * (i % 8) as f32 + 5.0 * ((i % 8) / 4) as f32;
                let ry = 58.0 + 37.0 * (i / 8) as f32;
                // F 键用自定义 AssignKeyFrames（选中态固定 pressed 帧），
                // 不挂 ImageButton（避免与 image_button_system 抢帧）
                p.spawn((
                    Button,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(rx),
                        top: Val::Px(ry),
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        ..default()
                    },
                    ImageNode::new(n.clone()),
                    AssignKeyFKey(i),
                    AssignKeyFrames {
                        normal: n.clone(),
                        hover: hov.clone(),
                        pressed: pre.clone(),
                    },
                    ZIndex(10),
                ))
                    .with_children(|c| {
                        // 键名文本：F1..F8 / "Ctrl\nF1" 双行（#2584）
                        c.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(4.0),
                                top: Val::Px(11.0),
                                ..default()
                            },
                            Text::new(assign_key_label(i as u8 + 1)),
                            TextFont {
                                font: FontSource::Handle(font.clone()),
                                font_size: FontSize::Px(9.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            ZIndex(11),
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
    mut all_vis: Query<&mut Visibility, With<AssignKeyWidget>>,
    mut icon: Query<
        (&mut ImageNode, &AssignKeyIcon),
        (Without<AssignKeyTitle>, Without<AssignKeyFKey>),
    >,
    mut title: Query<(&mut Text, &AssignKeyTitle), Without<AssignKeyIcon>>,
    mut actions: Query<
        (Entity, &Interaction, Option<&AssignKeyNone>, Option<&AssignKeySave>),
        Without<AssignKeyFKey>,
    >,
    mut fkeys: Query<(Entity, &Interaction, &AssignKeyFKey, &mut ImageNode, &AssignKeyFrames)>,
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
        if let Some(h) = load_lib_image(
            &mut libs,
            &mut images,
            LibraryName::MagIcon2,
            m.icon as usize * 2,
        ) {
            for (mut node, _) in &mut icon {
                if node.image != h {
                    node.image = h.clone();
                }
            }
        }
        for (mut t, _) in &mut title {
            t.0 = format!("选择 {} 的快捷键", m.name);
        }
    }

    // None / Save（C# NoneButton.Click / SaveButton.Click）
    for (e, inter, none, save) in &mut actions {
        if !edge(e, inter, &mut prev_inter) {
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
    for (e, inter, fkey, mut node, frames) in &mut fkeys {
        if edge(e, inter, &mut prev_inter) {
            state.key = (fkey.0 + 1) as u8;
        }
        let selected = state.key as usize == fkey.0 + 1;
        let frame = if selected || *inter == Interaction::Pressed {
            &frames.pressed
        } else if *inter == Interaction::Hovered {
            &frames.hover
        } else {
            &frames.normal
        };
        if node.image != *frame {
            node.image = frame.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# AssignKeyPanel 按钮标签双行（MainDialogs.cs:3302-3333，#2584）
    #[test]
    fn assign_key_labels_two_line_like_csharp() {
        assert_eq!(assign_key_label(0), "");
        assert_eq!(assign_key_label(1), "F1");
        assert_eq!(assign_key_label(8), "F8");
        assert_eq!(assign_key_label(9), "Ctrl\nF1");
        assert_eq!(assign_key_label(16), "Ctrl\nF8");
        assert_eq!(assign_key_label(17), "Shift\nF1");
        assert_eq!(assign_key_label(24), "Shift\nF8");
        assert_eq!(assign_key_label(25), "");
    }
}
