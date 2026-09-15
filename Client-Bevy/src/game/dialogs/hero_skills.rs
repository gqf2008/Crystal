// ============================================================================
// 英雄技能对话框（#218）
// 参考：C# HeroDialog SkillPage = CharacterDialog(MirGridType.HeroEquipment)
//   - 背景 Title[504]，角色页 Title[508] at (8,90)，7 行技能（MagIcon2 图标 + 名称/等级/经验）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::assign_key::AssignKeyState;
use crate::game::dialogs::hero::HeroState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label, spawn_panel,
};

/// C# `CharacterDialog(HeroEquipment, hero)`：`Index = 504` @ `(ScreenWidth-264, 0)`（`CharacterDialog.cs:32-34`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 504);
pub const PANEL_SIZE: (f32, f32) = (264.0, 380.0);
pub const DIALOG_X: f32 = 1024.0 - 264.0;
pub const DIALOG_Y: f32 = 0.0;
/// 技能页 `SkillPage` = `Title[508]` @(8,90)（`CharacterDialog.cs:136-143`）
pub const PAGE: (LibraryName, usize) = (LibraryName::Title, 508);
pub const PAGE_X: f32 = 8.0;
pub const PAGE_Y: f32 = 90.0;
/// 关闭键 `Prguse2[360..362]` @(241,3)（`CharacterDialog.cs:190-199`，无 `Size` → art 24x21）
pub const CLOSE_REL: (f32, f32) = (241.0, 3.0);
/// 技能行数（C# `Magics` 显示区 7 行）
pub const ROWS: usize = 7;
/// 行容器：页内 @(8, 8 + i*33) 231x33
pub const ROW_X: f32 = 8.0;
pub const ROW_Y: f32 = 8.0;
pub const ROW_H: f32 = 33.0;
pub const ROW_W: f32 = 231.0;

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
    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        DIALOG_X,
        DIALOG_Y,
        PANEL_SIZE.0,
        PANEL_SIZE.1,
        30,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::HeroSkill), HeroSkillWidget));

    commands.entity(panel).with_children(|p| {
        // 技能页容器（#2892 批57：页根显隐；子节点 = 技能页图 Title[508] + 7 行）
        crate::ui::theme::spawn_container(p, 0.0, 0.0, PANEL_SIZE.0, PANEL_SIZE.1, 8)
            .insert(crate::game::dialogs::hero_pages::HeroPageRoot(
                crate::game::dialogs::hero_pages::HeroPage::Skill,
            ))
            .with_children(|c| {
                // 技能页 Title[508]（C# SkillPage at (8,90)，原生尺寸）
                if let Some(h) = load_lib_image(&mut libs, &mut images, PAGE.0, PAGE.1) {
                    let (iw, ih) = match libs.0.get_image(PAGE.0, PAGE.1) {
                        Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                        None => (190.0, 259.0),
                    };
                    spawn_image(c, h, PAGE_X, PAGE_Y, iw, ih, 8);
                }
                for i in 0..ROWS {
                    spawn_container(
                        c,
                        PAGE_X + ROW_X,
                        PAGE_Y + ROW_Y + i as f32 * ROW_H,
                        ROW_W,
                        ROW_H,
                        9,
                    )
                    .insert((Button, HeroSkillRow(i), Visibility::Hidden))
                    .with_children(|cc| {
                        let white = images.add(crate::map_renderer::make_image(
                            vec![255, 255, 255, 255],
                            1,
                            1,
                        ));
                        spawn_image(cc, white, 36.0, 0.0, 36.0, 36.0, 10).insert(HeroSkillIcon(i));
                        spawn_label(cc, &font, "", 78.0, 6.0, 12.0, Color::WHITE, 10)
                            .insert(HeroSkillText(i));
                    });
                }
            });
        // #2892 批57：四页签（点装备/状态页会切到 HeroEquipment 窗的对应页）
        crate::game::dialogs::hero_pages::spawn_hero_tabs(p, &mut libs, &mut images);
        // 关闭（C# CharacterDialog CloseButton at (241,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            // C# 无 `Size` → art 24x21（此前 20x20 是自造尺寸）
            spawn_icon_button(p, n, h, pr, CLOSE_REL.0, CLOSE_REL.1, 24.0, 21.0, 10)
                .insert(HeroSkillClose);
        }
    });
}

/// 显隐 + 英雄魔法列表渲染
#[allow(clippy::too_many_arguments)]
fn hero_skill_ui_system(
    mut mgr: ResMut<DialogManager>,
    hero: Res<HeroState>,
    mut assign_key: ResMut<AssignKeyState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    close: Query<(Entity, &Interaction), With<HeroSkillClose>>,
    mut widgets: Query<&mut Visibility, (With<HeroSkillWidget>, Without<HeroSkillRow>)>,
    mut rows: Query<(Entity, &mut Visibility, &HeroSkillRow, &Interaction)>,
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
    // 行显隐随容器（子节点自动跟随）；点击技能行进入 Shift+F1..F8 分配。
    for (e, mut vis, row, inter) in &mut rows {
        let magic = hero.magics.get(row.0);
        *vis = if magic.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !assign_key.visible {
            if edge(e, inter, &mut prev_inter) {
                if let Some(m) = magic {
                    assign_key.open_hero(m.spell, m.key);
                    tracing::info!("🔑 打开英雄技能快捷键面板: {} key={}", m.name, m.key);
                }
            }
        }
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
