// ============================================================================
#![allow(clippy::type_complexity)]
// 主对话框 HUD（M8）
// 布局参考：Client/MirScenes/Dialogs/MainDialogs.cs + Client-Macroquad
//   src/scenes/dialogs/game/main_dialog.rs（draw_health_mana_orbs / draw_exp_bar / draw_buttons）
// 纹理：Prguse[分辨率] 背景、Prguse[4] 血蓝球、Prguse[7/8] 经验条、
//       按钮（1900..1914 角色/背包/技能/任务/设置，1960.. 菜单，826.. 商城）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::{DialogKind, DialogManager};
use crate::ui::sprite_ui::UiButton;
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiEntity, UiFont, UiImageCache,
};

/// HUD 状态（网络 handler 写入，HUD 系统读取）
#[derive(Resource)]
pub struct HudState {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub max_exp: i64,
    pub level: u16,
    pub gold: u32,
    pub name: String,
    /// 本地玩家 object_id（UserInformation 提供）
    pub player_object_id: Option<u32>,
    /// 角色职业（显示用）
    pub class: u8,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            hp: 1,
            max_hp: 1000,
            mp: 1,
            max_mp: 600,
            exp: 0,
            max_exp: 100,
            level: 1,
            gold: 0,
            name: String::new(),
            player_object_id: None,
            class: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HudButtonKind {
    Character,
    Inventory,
    Skills,
    QuestLog,
    Option,
    Menu,
    GameShop,
}

#[derive(Component)]
pub struct HudButton(pub HudButtonKind);

/// HUD 按钮 → 对话框开关（M9：接入 DialogManager）
fn hud_button_system(
    mut mgr: ResMut<DialogManager>,
    buttons: Query<(&UiButton, &HudButton)>,
) {
    for (btn, kind) in &buttons {
        if btn.clicked {
            tracing::info!("🎛️ HUD 按钮点击: {:?}", kind.0);
            match kind.0 {
                HudButtonKind::Inventory => mgr.toggle(DialogKind::Inventory),
                HudButtonKind::Character => mgr.toggle(DialogKind::Character),
                HudButtonKind::Skills => mgr.toggle(DialogKind::Character),
                HudButtonKind::QuestLog => mgr.toggle(DialogKind::QuestLog),
                HudButtonKind::Option => mgr.toggle(DialogKind::Option),
                HudButtonKind::Menu => mgr.toggle(DialogKind::Menu),
                HudButtonKind::GameShop => mgr.toggle(DialogKind::GameShop),
            }
        }
    }
}

/// 动态部件标记（每帧按 HudState 更新）
#[derive(Component)] struct HpHpFill;
#[derive(Component)] struct MpMpFill;
#[derive(Component)] struct ExpFill;
#[derive(Component)] struct HpHpText;
#[derive(Component)] struct MpMpText;
#[derive(Component)] struct ExpText;
#[derive(Component)] struct LevelText;
#[derive(Component)] struct GoldText;
#[derive(Component)] struct NameText;

const ORB_HEIGHT: f32 = 80.0;
const ORB_TOP: f32 = 30.0;
const EXP_TOP: f32 = 143.0;
const BUTTON_TOP: f32 = 76.0;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), spawn_hud);
        app.add_systems(OnExit(AppState::Game), cleanup_hud);
        app.add_systems(
            Update,
            (ui_button_system, hud_button_system, hud_update_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_hud(mut commands: Commands, roots: Query<Entity, With<UiEntity>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_hud(
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

    // 分辨率索引：窗口 1024 宽 → 1（与 macroquad 一致：800→0，1024→1，其他→2）
    let resolution_index = 1usize;
    let bg_info = libs
        .0
        .get_image(LibraryName::Prguse, resolution_index)
        .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
        .unwrap_or((1024.0, 150.0));
    let (bg_w, bg_h) = bg_info;
    let main_x = (1024.0 - bg_w) / 2.0;
    let main_y = 768.0 - bg_h;

    // 背景
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, resolution_index) {
        spawn_ui_sprite(&mut commands, h, main_x, main_y, 1.0, 1.0);
    }

    // 血/蓝球填充（Prguse[4]：左半红 HP、右半蓝 MP）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 4) {
        let orb_x = main_x;
        let orb_y = main_y + ORB_TOP;
        // HP 球（左半）
        commands.spawn((
            UiEntity,
            HpHpFill,
            Sprite {
                image: h.clone(),
                rect: Some(Rect::new(0.0, 0.0, 50.0, ORB_HEIGHT)),
                custom_size: Some(Vec2::new(50.0, ORB_HEIGHT)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(orb_x, -orb_y, 2.0),
            Visibility::default(),
        ));
        // MP 球（右半）
        commands.spawn((
            UiEntity,
            MpMpFill,
            Sprite {
                image: h,
                rect: Some(Rect::new(51.0, 0.0, 101.0, ORB_HEIGHT)),
                custom_size: Some(Vec2::new(50.0, ORB_HEIGHT)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(orb_x + 51.0, -orb_y, 2.0),
            Visibility::default(),
        ));
    }

    // 经验条（Prguse[8]；800 宽用 7）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 8) {
        let bar_x = main_x + 9.0;
        let bar_y = main_y + EXP_TOP;
        let (tw, th) = libs
            .0
            .get_image(LibraryName::Prguse, 8)
            .map(|i| (i.width.max(0) as f32, i.height.max(0) as f32))
            .unwrap_or((100.0, 5.0));
        commands.spawn((
            UiEntity,
            ExpFill,
            Sprite {
                image: h,
                rect: Some(Rect::new(0.0, 0.0, tw, th)),
                custom_size: Some(Vec2::new(tw, th)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(bar_x, -bar_y, 2.0),
            Visibility::default(),
        ));
    }

    // 文本
    let orb_x = main_x;
    let orb_y = main_y + ORB_TOP;
    spawn_text(&mut commands, &font, &mut images, &mut cache, HpHpText, orb_x + 9.0, orb_y + 18.0, "");
    spawn_text(&mut commands, &font, &mut images, &mut cache, MpMpText, orb_x + 60.0, orb_y + 18.0, "");
    spawn_text(&mut commands, &font, &mut images, &mut cache, ExpText, main_x + 9.0 + 50.0, main_y + EXP_TOP - 2.0, "");
    spawn_text(&mut commands, &font, &mut images, &mut cache, LevelText, main_x + 9.0, main_y + 2.0, "");
    spawn_text(&mut commands, &font, &mut images, &mut cache, GoldText, main_x + bg_w - 90.0, main_y + 2.0, "");
    spawn_text(&mut commands, &font, &mut images, &mut cache, NameText, main_x + 9.0, main_y + 14.0, "");

    // 主对话框按钮（C# 位置：Size.Width - 119/-96/-73/-50/-27，y=+76）
    let button_y = main_y + BUTTON_TOP;
    let buttons: [(HudButtonKind, usize, usize, usize, f32); 5] = [
        (HudButtonKind::Character, 1900, 1901, 1902, bg_w - 119.0),
        (HudButtonKind::Inventory, 1903, 1904, 1905, bg_w - 96.0),
        (HudButtonKind::Skills, 1906, 1907, 1908, bg_w - 73.0),
        (HudButtonKind::QuestLog, 1909, 1910, 1911, bg_w - 50.0),
        (HudButtonKind::Option, 1912, 1913, 1914, bg_w - 27.0),
    ];
    for (kind, n, h, p, xoff) in buttons {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            LibraryName::Prguse, n, h, p,
            main_x + xoff, button_y, 3.0, 23.0, 23.0,
        ) {
            commands.entity(e).insert(HudButton(kind));
        }
    }
    // 菜单按钮（C#：Width-55, 35）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 1960, 1961, 1962,
        main_x + bg_w - 55.0, main_y + 35.0, 3.0, 23.0, 23.0,
    ) {
        commands.entity(e).insert(HudButton(HudButtonKind::Menu));
    }
    // 商城按钮（C#：Width-105, 35）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse, 826, 827, 828,
        main_x + bg_w - 105.0, main_y + 35.0, 3.0, 23.0, 23.0,
    ) {
        commands.entity(e).insert(HudButton(HudButtonKind::GameShop));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_text(
    commands: &mut Commands,
    font: &Handle<Font>,
    _images: &mut Assets<Image>,
    _cache: &mut UiImageCache,
    _marker: impl Component,
    x: f32,
    y: f32,
    text: &str,
) {
    let e = spawn_ui_text(commands, font, text, x, y, 12.0, Color::WHITE, 4.0);
    commands.entity(e).insert(_marker);
}

/// 每帧按 HudState 更新血/蓝/经验条与文本（单查询避免 Bevy B0001 冲突）
fn hud_update_system(
    hud: Res<HudState>,
    mut fills: Query<(
        &mut Sprite,
        &mut Transform,
        Option<&HpHpFill>,
        Option<&MpMpFill>,
        Option<&ExpFill>,
    )>,
    mut texts: Query<(
        &mut Text2d,
        Option<&HpHpText>,
        Option<&MpMpText>,
        Option<&ExpText>,
        Option<&LevelText>,
        Option<&GoldText>,
        Option<&NameText>,
    )>,
) {
    let hp_pct = (hud.hp as f32 / hud.max_hp.max(1) as f32).clamp(0.0, 1.0);
    let mp_pct = (hud.mp as f32 / hud.max_mp.max(1) as f32).clamp(0.0, 1.0);
    let exp_pct = (hud.exp as f32 / hud.max_exp.max(1) as f32).clamp(0.0, 1.0);

    for (mut sprite, mut tf, hp, mp, exp) in &mut fills {
        if hp.is_some() {
            let h = ORB_HEIGHT * hp_pct;
            sprite.rect = Some(Rect::new(0.0, ORB_HEIGHT - h, 50.0, ORB_HEIGHT));
            sprite.custom_size = Some(Vec2::new(50.0, h));
            tf.translation.y = -(ORB_TOP + (ORB_HEIGHT - h));
        } else if mp.is_some() {
            let h = ORB_HEIGHT * mp_pct;
            sprite.rect = Some(Rect::new(51.0, ORB_HEIGHT - h, 101.0, ORB_HEIGHT));
            sprite.custom_size = Some(Vec2::new(50.0, h));
            tf.translation.y = -(ORB_TOP + (ORB_HEIGHT - h));
        } else if exp.is_some() {
            let (tw, th) = match sprite.rect {
                Some(r) => (r.max.x - r.min.x, r.max.y - r.min.y),
                None => (100.0, 5.0),
            };
            let w = tw * exp_pct;
            sprite.rect = Some(Rect::new(0.0, 0.0, w, th));
            sprite.custom_size = Some(Vec2::new(w, th));
        }
    }

    for (mut t, hp, mp, exp, lv, gold, name) in &mut texts {
        if hp.is_some() {
            t.0 = format!("{}", hud.hp);
        } else if mp.is_some() {
            t.0 = format!("{}", hud.mp);
        } else if exp.is_some() {
            t.0 = format!("{:.1}%", exp_pct * 100.0);
        } else if lv.is_some() {
            t.0 = format!("Lv.{}", hud.level);
        } else if gold.is_some() {
            t.0 = format!("{}", hud.gold);
        } else if name.is_some() {
            t.0 = hud.name.clone();
        }
    }
}
