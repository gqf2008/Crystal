// ============================================================================
// 英雄技能对话框（#218）
// 参考：C# HeroDialog SkillPage = CharacterDialog(MirGridType.HeroEquipment)
//   - 背景 Title[504]，角色页 Title[508] at (8,90)，7 行技能（MagIcon2 图标 + 名称/等级/经验）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::hero::HeroState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

const DIALOG_X: f32 = 1024.0 - 264.0;
const DIALOG_Y: f32 = 0.0;
const PAGE_X: f32 = 8.0;
const PAGE_Y: f32 = 90.0;
const ROWS: usize = 7;

#[derive(Component)]
pub struct HeroSkillWidget;

#[derive(Component)]
pub struct HeroSkillClose;

#[derive(Component)]
pub struct HeroSkillRow(pub usize);

#[derive(Component)]
pub struct HeroSkillIcon(pub usize);

#[derive(Component)]
pub struct HeroSkillText(pub usize);

pub struct HeroSkillPlugin;

impl Plugin for HeroSkillPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_hero_skills);
        app.add_systems(OnExit(AppState::Game), cleanup_hero_skills);
        app.add_systems(
            Update,
            hero_skill_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hero_skills(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_hero_skills(
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

    // 背景 Title[504]（264x380 @ (760,0)）+ 技能页 Title[508]
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 504) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, 264.0, 380.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::HeroSkill), HeroSkillWidget));

    commands.entity(panel).with_children(|p| {
        // 技能页 Title[508]（C# SkillPage at (8,90)，原生尺寸）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 508) {
            let (iw, ih) = match libs.0.get_image(LibraryName::Title, 508) {
                Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                None => (190.0, 259.0),
            };
            spawn_image(p, h, PAGE_X, PAGE_Y, iw, ih, 8);
        }
        // 关闭（C# CharacterDialog CloseButton at (241,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 241.0, 3.0, 20.0, 20.0, 10).insert(HeroSkillClose);
        }
        // 7 行技能：容器（行显隐随容器）+ 图标 + 文本
        for i in 0..ROWS {
            spawn_container(p, PAGE_X + 8.0, PAGE_Y + 8.0 + i as f32 * 33.0, 231.0, 33.0, 9)
                .insert((HeroSkillRow(i), Visibility::Hidden))
                .with_children(|c| {
                    let white = images.add(crate::map_renderer::make_image(
                        vec![255, 255, 255, 255],
                        1,
                        1,
                    ));
                    spawn_image(c, white, 36.0, 0.0, 36.0, 36.0, 10).insert(HeroSkillIcon(i));
                    spawn_label(c, &font, "", 78.0, 6.0, 12.0, Color::WHITE, 10)
                        .insert(HeroSkillText(i));
                });
        }
    });
}

/// 显隐 + 英雄魔法列表渲染
#[allow(clippy::too_many_arguments)]
fn hero_skill_ui_system(
    mut mgr: ResMut<DialogManager>,
    hero: Res<HeroState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    close: Query<(Entity, &Interaction), With<HeroSkillClose>>,
    mut widgets: Query<&mut Visibility, (With<HeroSkillWidget>, Without<HeroSkillRow>)>,
    mut rows: Query<(&mut Visibility, &HeroSkillRow)>,
    mut icons: Query<(&mut ImageNode, &HeroSkillIcon)>,
    mut texts: Query<(&mut Text, &HeroSkillText)>,
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
    let open = mgr.is_open(DialogKind::HeroSkill);
    for mut vis in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::HeroSkill);
        }
    }
    // 行显隐随容器（子节点自动跟随）
    for (mut vis, row) in &mut rows {
        *vis = if hero.magics.get(row.0).is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 图标换图 + 文本渲染
    for (mut node, icon) in &mut icons {
        if let Some(m) = hero.magics.get(icon.0) {
            if let Some(h) = load_lib_image(
                &mut libs,
                &mut images,
                LibraryName::MagIcon2,
                m.icon as usize * 2,
            ) {
                if node.image != h {
                    node.image = h;
                }
            }
        }
    }
    for (mut t, txt) in &mut texts {
        if let Some(m) = hero.magics.get(txt.0) {
            t.0 = format!("Lv.{}  {}（经验 {}/1000）", m.level, m.name, m.experience);
        } else {
            t.0 = String::new();
        }
    }
}
