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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
            (hero_skill_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Title[504] + 技能页 Title[508]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 504) {
        let e = spawn_ui_sprite(&mut commands, h, DIALOG_X, DIALOG_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::HeroSkill),
            HeroSkillWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 508) {
        let e = spawn_ui_sprite(
            &mut commands,
            h,
            DIALOG_X + PAGE_X,
            DIALOG_Y + PAGE_Y,
            6.1,
            1.0,
        );
        commands.entity(e).insert((
            DialogRoot(DialogKind::HeroSkill),
            HeroSkillWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭（C# CharacterDialog CloseButton at (241,3)）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        DIALOG_X + 241.0,
        DIALOG_Y + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands.entity(e).insert((
            HeroSkillClose,
            DialogRoot(DialogKind::HeroSkill),
            HeroSkillWidget,
        ));
    }
    // 7 行技能：图标 + 文本
    let transparent = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    for i in 0..ROWS {
        let rx = DIALOG_X + PAGE_X + 8.0;
        let ry = DIALOG_Y + PAGE_Y + 8.0 + i as f32 * 33.0;
        let row = commands
            .spawn((
                crate::ui::sprite_ui::UiEntity,
                DialogRoot(DialogKind::HeroSkill),
                HeroSkillRow(i),
                UiButton {
                    rect: (rx, ry, 231.0, 33.0),
                    clicked: false,
                },
                Sprite {
                    image: transparent.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                    custom_size: Some(Vec2::new(231.0, 33.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(rx, -ry, 6.5),
                Visibility::Hidden,
            ))
            .id();
        commands.entity(row).with_children(|p| {
            p.spawn((
                HeroSkillIcon(i),
                Sprite {
                    image: transparent.clone(),
                    custom_size: Some(Vec2::new(36.0, 36.0)),
                    ..default()
                },
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(36.0, 0.0, 6.6),
                Visibility::Hidden,
            ));
            p.spawn((
                HeroSkillText(i),
                Text2d::new(String::new()),
                bevy::sprite::Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_xyz(78.0, 6.0, 6.6),
                Visibility::Hidden,
            ));
        });
    }
}

/// 显隐 + 英雄魔法列表渲染
#[allow(clippy::too_many_arguments)]
fn hero_skill_ui_system(
    mut mgr: ResMut<DialogManager>,
    hero: Res<HeroState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    close: Query<&UiButton, With<HeroSkillClose>>,
    mut widgets: Query<
        &mut Visibility,
        (
            With<HeroSkillWidget>,
            Without<HeroSkillRow>,
            Without<HeroSkillIcon>,
            Without<HeroSkillText>,
        ),
    >,
    mut rows: Query<
        (&mut Visibility, &HeroSkillRow),
        (Without<HeroSkillIcon>, Without<HeroSkillText>),
    >,
    mut icons: Query<
        (&mut Sprite, &mut Visibility, &HeroSkillIcon),
        (Without<HeroSkillRow>, Without<HeroSkillText>),
    >,
    mut texts: Query<
        (&mut Text2d, &mut Visibility, &HeroSkillText),
        (Without<HeroSkillRow>, Without<HeroSkillIcon>),
    >,
) {
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
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::HeroSkill);
        }
    }
    for (mut vis, row) in &mut rows {
        let show = hero.magics.get(row.0).is_some();
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut sprite, mut vis, icon) in &mut icons {
        if let Some(m) = hero.magics.get(icon.0) {
            if let Some(h) = ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::MagIcon2,
                m.icon as usize * 2,
            ) {
                if sprite.image != h {
                    sprite.image = h.clone();
                }
            }
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
    for (mut t, mut vis, txt) in &mut texts {
        if let Some(m) = hero.magics.get(txt.0) {
            t.0 = format!("Lv.{}  {}（经验 {}/1000）", m.level, m.name, m.experience);
            *vis = Visibility::Visible;
        } else {
            t.0 = String::new();
            *vis = Visibility::Hidden;
        }
    }
}
