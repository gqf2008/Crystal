// ============================================================================
// 英雄技能**页**内容（#218 → #2892 批58 合并回 C# 的单窗结构）
// 参考：C# `HeroDialog.SkillPage` = 同一个 `CharacterDialog(MirGridType.HeroEquipment, hero)`
//   - 页图 Title[508] at (8,90)，7 行技能（MagIcon2 图标 + 名称/等级/经验）
//   - 页签/页容器与窗口由 `hero_equipment.rs`（英雄对话框）持有；本模块只提供
//     「技能页容器 + 7 行」的生成与行内容刷新
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::assign_key::AssignKeyState;
use crate::game::dialogs::hero::HeroState;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::theme::{load_lib_image, spawn_container, spawn_image, spawn_label};

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

/// 技能行容器（页图 + 7 行由 [`spawn_hero_skill_page`] 生成）
#[derive(Component)]
pub struct HeroSkillRow(pub usize);

#[derive(Component)]
pub struct HeroSkillIcon(pub usize);

#[derive(Component)]
pub struct HeroSkillText(pub usize);

pub struct HeroSkillPlugin;

impl Plugin for HeroSkillPlugin {
    fn build(&self, app: &mut App) {
        // 窗口（含页签/关闭键）由 `HeroEquipmentPlugin`（英雄对话框）持有；
        // 本插件只负责技能页**行内容**刷新与行点击（Shift+F1..F8 分配）
        app.add_systems(
            Update,
            hero_skill_rows_system.run_if(in_state(AppState::Game)),
        );
    }
}

/// 生成「技能页」（`Title[508]` @(8,90) + 7 行）到英雄对话框窗口内。
/// 由 `hero_equipment.rs`（英雄对话框）调用；页显隐由 `hero_pages_system` 统一控制。
pub fn spawn_hero_skill_page(
    parent: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
) {
    crate::ui::theme::spawn_container(parent, 0.0, 0.0, PANEL_SIZE.0, PANEL_SIZE.1, 8)
        .insert(crate::game::dialogs::hero_pages::HeroPageRoot(
            crate::game::dialogs::hero_pages::HeroPage::Skill,
        ))
        .with_children(|c| {
            // 技能页 Title[508]（C# SkillPage at (8,90)，原生尺寸）
            if let Some(h) = load_lib_image(libs, images, PAGE.0, PAGE.1) {
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
                    spawn_label(cc, font, "", 78.0, 6.0, 12.0, Color::WHITE, 10)
                        .insert(HeroSkillText(i));
                });
            }
        });
}

/// 技能页**行内容**刷新 + 行点击（Shift+F1..F8 分配）
/// 显隐 + 英雄魔法列表渲染
#[allow(clippy::too_many_arguments)]
fn hero_skill_rows_system(
    hero: Res<HeroState>,
    mut assign_key: ResMut<AssignKeyState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
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
    // 行的整页显隐由 `hero_pages_system` 管（页容器）；这里只管「该行有没有技能」
    // 与行点击进入 Shift+F1..F8 分配
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
