// ============================================================================
#![allow(clippy::type_complexity)]
// 角色属性对话框（M9 第 1 批）
// 布局参考：Client/MirScenes/Dialogs/CharacterDialog.cs
//   - 窗口 (ScreenWidth-264, 0)，背景 Title[504]（264 宽）
//   - 标签页：Character/Status/State/Skill（Title[500-503]，64x20，y=70）
//   - 关闭 (241,3) Prguse2[360-362]
//   - 页面背景：Prguse[340] / Title[506] / Title[507] / Title[508]（页区 (8,90)）
//   - 14 个装备槽（36x36）；状态页数值标签（相对页区，x=126）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::assign_key::AssignKeyState;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::game::skills::MagicsState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_image, spawn_label,
    spawn_label_center, spawn_panel,
};

/// 角色状态（网络写入；当前服务器未下发属性，先默认值）
#[derive(Resource)]
pub struct CharacterState {
    pub name: String,
    pub guild: String,
    pub level: u16,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    /// [min, max] AC/MAC/DC/MC/SC
    pub stats: [[i32; 2]; 5],
    /// #208：暴击率/暴击伤害/攻速/命中/敏捷/幸运
    pub critical_rate: i32,
    pub critical_damage: i32,
    pub attack_speed: i32,
    pub accuracy: i32,
    pub agility: i32,
    pub luck: i32,
    /// #210：State 页数据
    pub exp: i64,
    pub max_exp: i64,
    pub bag_weight: i32,
    pub wear_weight: i32,
    pub hand_weight: i32,
    pub magic_resist: i32,
    pub poison_resist: i32,
    pub health_recovery: i32,
    pub spell_recovery: i32,
    pub poison_recovery: i32,
    pub holy: i32,
    pub freezing: i32,
    pub poison_atk: i32,
    /// 14 装备槽
    pub equipment: Vec<Option<u32>>,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            name: String::new(),
            guild: String::new(),
            level: 1,
            hp: 1,
            max_hp: 1000,
            mp: 1,
            max_mp: 600,
            stats: [[0; 2]; 5],
            critical_rate: 0,
            critical_damage: 0,
            attack_speed: 0,
            accuracy: 0,
            agility: 0,
            luck: 0,
            exp: 0,
            max_exp: 1,
            bag_weight: 0,
            wear_weight: 0,
            hand_weight: 0,
            magic_resist: 0,
            poison_resist: 0,
            health_recovery: 0,
            spell_recovery: 0,
            poison_recovery: 0,
            holy: 0,
            freezing: 0,
            poison_atk: 0,
            equipment: vec![None; 14],
        }
    }
}

pub const DIALOG_X: f32 = 1024.0 - 264.0;
pub const DIALOG_Y: f32 = 0.0;
/// C# CharacterPage @ (8,90)（CharacterDialog.cs:45）：装备格父容器的页偏移
pub const PAGE_X: f32 = 8.0;
pub const PAGE_Y: f32 = 90.0;

/// 装备槽位置（C# EquipmentSlot 页内坐标，CharacterDialog.cs:229-340；屏坐标 = DIALOG + PAGE + 此偏移）
pub const EQUIP_SLOTS: [(f32, f32); 14] = [
    (123.0, 7.0),   // Weapon
    (163.0, 7.0),   // Armor
    (203.0, 7.0),   // Helmet
    (203.0, 134.0), // Torch
    (203.0, 98.0),  // Necklace
    (8.0, 170.0),   // BraceletL
    (203.0, 170.0), // BraceletR
    (8.0, 206.0),   // RingL
    (203.0, 206.0), // RingR
    (8.0, 242.0),   // Amulet
    (88.0, 242.0),  // Belt
    (48.0, 242.0),  // Boots
    (128.0, 242.0), // Stone
    (203.0, 62.0),  // Mount
];
/// C# MirItemCell 装备格 Size(36,32)（MirItemCell.cs:186；纵向步进 36=32+4 间隙，如 BraceletL(8,170)→RingL(8,206)）
pub const SLOT_W: f32 = 36.0;
pub const SLOT_H: f32 = 32.0;
/// C# NameLabel (0,12) 264x20 / GuildLabel (0,33) 264x30，HCenter|VCenter 框心（对话框相对）：
/// x=264/2=132；name y=12+20/2=22；guild y=33+30/2=48（CharacterDialog.cs:202-217）
pub const NAME_CX: f32 = 132.0;
pub const NAME_CY: f32 = 22.0;
pub const GUILD_CY: f32 = 48.0;
/// C# ClassImage @ (15,33)（CharacterDialog.cs:222，对话框相对、常显不随页）
pub const CLASS_IMG_X: f32 = 15.0;
pub const CLASS_IMG_Y: f32 = 33.0;

#[derive(Component)]
pub struct CharDialogWidget;

/// 当前页（0=角色 1=状态 2=状态2 3=技能）
#[derive(Resource, Default)]
pub struct CharPage(pub usize);

#[derive(Component)]
pub struct CharTab(pub usize);

#[derive(Component)]
pub struct CharPageBg(pub usize);

#[derive(Component)]
pub struct CharClose;

/// 装备槽（服务端 12 槽序号）
#[derive(Component, Clone, Copy)]
pub struct CharEquipSlot(pub usize);

/// 装备图标（子实体）
#[derive(Component, Clone, Copy)]
pub struct CharEquipIcon(pub usize);

#[derive(Component)]
pub struct CharNameText;

#[derive(Component)]
pub struct CharGuildText;

#[derive(Component)]
pub struct CharStatText(pub usize); // 状态数值标签序号

/// State 页数值标签（#210；挂 CharPageBg(2) 随页显隐）
#[derive(Component)]
pub struct CharState2Text(pub usize);
// ---- 技能页（C# CharacterDialog.MagicButton 7 行 + Next/Back） ----
const SKILL_ROW_COUNT: usize = 7;
/// 翻页按钮在 children 可见性查询中的哨兵行号
/// 技能页当前起始行（C# CharacterDialog.StartIndex）
#[derive(Resource, Default)]
pub struct CharSkillStart(pub usize);
/// 技能行（整行 231x33 可点击，打开快捷键面板）
#[derive(Component)]
pub struct CharSkillRow(pub usize);
/// 技能图标（MagIcon2[icon*2]，行内 (36,0)）
#[derive(Component)]
pub struct CharSkillIcon(pub usize);
/// 技能行子控件（图标/背景条/文本，可见性跟随行）
#[derive(Component)]
pub struct CharSkillRowChild(pub usize);
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SkillTextKind {
    Key,
    Level,
    Name,
    Exp,
}
#[derive(Component)]
pub struct CharSkillText {
    pub row: usize,
    pub kind: SkillTextKind,
}
#[derive(Component)]
pub struct CharSkillNext;
#[derive(Component)]
pub struct CharSkillBack;

/// 装备格屏坐标原点（对话框(760,0) + CharacterPage(8,90) + 页内偏移）。
/// 生成/右键/tooltip 三处统一用此换算——#2503 修的偏移 bug 即生成与右键漏加页偏移所致。
fn slot_screen_origin(pos: usize) -> Option<(f32, f32)> {
    EQUIP_SLOTS
        .get(pos)
        .map(|(ox, oy)| (DIALOG_X + PAGE_X + ox, DIALOG_Y + PAGE_Y + oy))
}

/// 装备格屏幕矩形（server_slot 0..13 → 绝对屏幕坐标 + C# MirItemCell 36x32 尺寸）
fn equip_slot_screen_rect(server_slot: usize) -> Option<(f32, f32, f32, f32)> {
    let pos_idx = *SERVER_SLOT_TO_POS.get(server_slot)?;
    let (x, y) = slot_screen_origin(pos_idx)?;
    Some((x, y, SLOT_W, SLOT_H))
}

/// 状态页数值标签文本（逐项对齐 C# StatusPage.BeforeDraw，CharacterDialog.cs:96-108）。
/// 关键差异：CritD（index 8）C# 是 `{0}` 不带 %（:104），仅 CritR（index 7）带 %（:103）。
fn stat_label_text(idx: usize, state: &CharacterState) -> String {
    match idx {
        0 => format!("{}/{}", state.hp, state.max_hp), // HP {0}/{1}
        1 => format!("{}/{}", state.mp, state.max_mp), // MP {0}/{1}
        2 => format!("{}-{}", state.stats[0][0], state.stats[0][1]), // AC {0}-{1}
        3 => format!("{}-{}", state.stats[1][0], state.stats[1][1]), // MAC
        4 => format!("{}-{}", state.stats[2][0], state.stats[2][1]), // DC
        5 => format!("{}-{}", state.stats[3][0], state.stats[3][1]), // MC
        6 => format!("{}-{}", state.stats[4][0], state.stats[4][1]), // SC
        7 => format!("{}%", state.critical_rate),      // CritR {0}%
        8 => format!("{}", state.critical_damage),     // CritD {0}（C# 无 %）
        9 => format!("{}", state.attack_speed),        // AtkSpd {0}
        10 => format!("+{}", state.accuracy),          // Acc +{0}
        11 => format!("+{}", state.agility),           // Agil +{0}
        12 => format!("{}", state.luck),               // Luck {0}
        _ => String::new(),
    }
}

/// 已装备格子悬停 tooltip（C# CharacterDialog MirItemCell；复用 #1244 item_tooltip_lines）
fn char_equip_tooltip_system(
    mgr: Res<DialogManager>,
    page: Res<CharPage>,
    inv: Res<HudState>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    windows: Query<&Window>,
) {
    if !mgr.is_open(DialogKind::Character) || page.0 != 0 {
        tooltip.update(5, false, String::new(), Vec::new(), 0.0, 0.0);
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let mut hit: Option<crate::game::dialogs::inventory::InvItem> = None;
    for server_slot in 0..14usize {
        if let Some((sx, sy, w, h)) = equip_slot_screen_rect(server_slot) {
            if cursor.x >= sx && cursor.x <= sx + w && cursor.y >= sy && cursor.y <= sy + h {
                hit = inv.equipment.get(server_slot).and_then(|s| s.as_ref()).cloned();
                break;
            }
        }
    }
    let Some(item) = hit else {
        tooltip.update(5, false, String::new(), Vec::new(), cursor.x, cursor.y);
        return;
    };
    let lines = crate::game::dialogs::inventory::item_tooltip_lines(&item);
    tooltip.update(5, true, item.name.clone(), lines, cursor.x, cursor.y);
}

pub struct CharacterDialogPlugin;

/// ServerRust equipment 槽位(0..13) → C# EQUIP_SLOTS 位置索引
/// 服务端: 0Weapon 1Armour 2Helmet 3Necklace 4BraceletL 5BraceletR 6RingL 7RingR 8Shoes 9Pendant 10Mount 11Torch 12Belt 13Stone
/// #1136：补 Torch(pos3)/Belt(pos10)/Stone(pos12)
/// ⚠ #2602：线序是 ServerRust `actors::inventory::EquipmentSlot` 旧序（非
/// SharedRust `enums::EquipmentSlot` 的 C# 序——两枚举同名异序，见各自
/// 互指注释）。服务端切到 SharedRust 序时本表必须同步改。
pub(crate) const SERVER_SLOT_TO_POS: [usize; 14] = [0, 1, 2, 4, 5, 6, 7, 8, 11, 9, 13, 3, 10, 12];

impl Plugin for CharacterDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterState>();
        app.init_resource::<CharPage>();
        app.init_resource::<CharSkillStart>();
        app.add_systems(OnEnter(AppState::Game), spawn_character_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_character_dialog);
        app.add_systems(
            Update,
            (character_ui_system, char_equip_system, char_skill_system, char_equip_tooltip_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_character_dialog(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_character_dialog(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    hud: Res<HudState>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));

    // bevy_ui 面板 Title[504]（264x380 @ 760,0）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 504) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, DIALOG_X, DIALOG_Y, 264.0, 380.0, 30);
    commands.entity(panel).insert((DialogRoot(DialogKind::Character), CharDialogWidget));

    commands.entity(panel).with_children(|p| {
        // 标签页 Title[500-503]（64x20，y=70）
        for (idx, x) in [(0usize, 8.0f32), (1, 70.0), (2, 132.0), (3, 194.0)] {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 500 + idx) {
                spawn_icon_button(p, h.clone(), h.clone(), h, x, 70.0, 64.0, 20.0, 10)
                    .insert(CharTab(idx));
            }
        }
        // 关闭（Prguse2 360/361/362 @(241,3)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 241.0, 3.0, 20.0, 20.0, 10).insert(CharClose);
        }
        // 职业图（Prguse[100+职业] @(15,33)）
        let class_idx = 100 + (hud.class as usize).min(4);
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, class_idx) {
            spawn_image(p, h, CLASS_IMG_X, CLASS_IMG_Y, 30.0, 30.0, 9);
        }
        // 名字/行会（框内居中）
        spawn_label_center(p, &font, "", NAME_CX, 2.0, 200.0, 14.0, Color::WHITE, 9)
            .insert(CharNameText);
        spawn_label_center(p, &font, "", NAME_CX, 28.0, 200.0, 12.0, Color::srgb(1.0, 0.85, 0.3), 9)
            .insert(CharGuildText);

        // 4 页容器（页区 (8,90)，页背景 248x284）
        let page_bgs: [(usize, LibraryName, usize); 4] = [
            (0, LibraryName::Prguse, 340),
            (1, LibraryName::Title, 506),
            (2, LibraryName::Title, 507),
            (3, LibraryName::Title, 508),
        ];
        for (idx, lib, tex) in page_bgs {
            if let Some(h) = load_lib_image(&mut libs, &mut images, lib, tex) {
                spawn_image(p, h, PAGE_X, PAGE_Y, 248.0, 284.0, 8)
                    .insert(CharPageBg(idx))
                    .with_children(|pg| {
                        match idx {
                            0 => {
                                // 14 装备槽
                                for pos in 0..EQUIP_SLOTS.len() {
                                    let (ox, oy) = EQUIP_SLOTS[pos];
                                    spawn_container(pg, ox, oy, SLOT_W, SLOT_H, 9)
                                        .insert((
                                            Button,
                                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.25)),
                                            CharEquipSlot(pos),
                                        ))
                                        .with_children(|sc| {
                                            if let Some(si) =
                                                SERVER_SLOT_TO_POS.iter().position(|pp| *pp == pos)
                                            {
                                                spawn_container(sc, 2.0, 2.0, SLOT_W - 4.0, SLOT_H - 4.0, 10)
                                                    .insert((
                                                        ImageNode::new(white.clone()),
                                                        CharEquipIcon(si),
                                                        Visibility::Hidden,
                                                    ));
                                            }
                                        });
                                }
                            }
                            1 => {
                                // 状态页数值（x=126，C# StatusPage）
                                let stat_ys: [f32; 13] = [
                                    20.0, 38.0, 56.0, 74.0, 92.0, 110.0, 128.0, 146.0, 164.0,
                                    182.0, 200.0, 218.0, 236.0,
                                ];
                                for (i, y) in stat_ys.iter().enumerate() {
                                    spawn_label(pg, &font, "0", 126.0, y - 2.0, 11.0, Color::WHITE, 9)
                                        .insert(CharStatText(i));
                                }
                            }
                            2 => {
                                // State 页数值
                                let stat_ys: [f32; 13] = [
                                    20.0, 38.0, 56.0, 74.0, 92.0, 110.0, 128.0, 146.0, 164.0,
                                    182.0, 200.0, 218.0, 236.0,
                                ];
                                for (i, y) in stat_ys.iter().take(12).enumerate() {
                                    spawn_label(pg, &font, "0", 126.0, y - 2.0, 11.0, Color::WHITE, 9)
                                        .insert(CharState2Text(i));
                                }
                            }
                            3 => {
                                // 技能页：7 行技能按钮（C# MagicButton (8,8+i*33) 231x33）
                                for i in 0..SKILL_ROW_COUNT {
                                    spawn_container(pg, 8.0, 8.0 + i as f32 * 33.0, 231.0, 33.0, 9)
                                        .insert((Button, CharSkillRow(i)))
                                        .with_children(|rc| {
                                            spawn_container(rc, 36.0, 0.0, 36.0, 36.0, 10)
                                                .insert((
                                                    ImageNode::new(white.clone()),
                                                    CharSkillIcon(i),
                                                    CharSkillRowChild(i),
                                                    Visibility::Hidden,
                                                ));
                                            for (tex, oy) in [(516usize, 7.0f32), (517, 19.0)] {
                                                if let Some(h) = load_lib_image(
                                                    &mut libs,
                                                    &mut images,
                                                    LibraryName::Title,
                                                    tex,
                                                ) {
                                                    spawn_container(rc, 73.0, oy, 24.0, 9.0, 9)
                                                        .insert((
                                                            ImageNode::new(h),
                                                            CharSkillRowChild(i),
                                                            Visibility::Hidden,
                                                        ));
                                                }
                                            }
                                            for (kind, ox, oy, size) in [
                                                (
                                                    SkillTextKind::Key,
                                                    2.0f32,
                                                    2.0f32,
                                                    10.0f32,
                                                ),
                                                (SkillTextKind::Level, 88.0, 2.0, 11.0),
                                                (SkillTextKind::Name, 109.0, 2.0, 11.0),
                                                (SkillTextKind::Exp, 109.0, 15.0, 11.0),
                                            ] {
                                                spawn_label(rc, &font, "", ox, oy, size, Color::WHITE, 11)
                                                    .insert((
                                                        CharSkillText { row: i, kind },
                                                        CharSkillRowChild(i),
                                                        Visibility::Hidden,
                                                    ));
                                            }
                                        });
                                }
                                // Next/Back（Prguse[396/397] @(140,250) / [398/399] @(90,250)）
                                for (is_next, idx, x) in [
                                    (true, 396usize, 140.0f32),
                                    (false, 398usize, 90.0f32),
                                ] {
                                    if let (Some(n), Some(h), Some(pr)) = (
                                        load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx),
                                        load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx),
                                        load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx + 1),
                                    ) {
                                        let mut b = spawn_icon_button(pg, n, h, pr, x, 250.0, 40.0, 22.0, 10);
                                        if is_next {
                                            b.insert(CharSkillNext);
                                        } else {
                                            b.insert(CharSkillBack);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    });
            }
        }
    });
}

fn character_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<CharacterState>,
    assign_key: Res<AssignKeyState>,
    mut page: ResMut<CharPage>,
    mut widgets: Query<&mut Visibility, (With<CharDialogWidget>, Without<CharPageBg>)>,
    tabs: Query<(Entity, &Interaction, &CharTab)>,
    close: Query<(Entity, &Interaction), (With<CharClose>, Without<CharTab>)>,
    mut page_bgs: Query<(&mut Visibility, &CharPageBg), Without<CharDialogWidget>>,
    mut name_texts: Query<
        &mut Text,
        (
            With<CharNameText>,
            Without<CharGuildText>,
            Without<CharStatText>,
            Without<CharState2Text>,
        ),
    >,
    mut guild_texts: Query<
        &mut Text,
        (
            With<CharGuildText>,
            Without<CharNameText>,
            Without<CharStatText>,
            Without<CharState2Text>,
        ),
    >,
    mut stat_texts: Query<
        (&mut Text, &CharStatText),
        (
            Without<CharNameText>,
            Without<CharGuildText>,
            Without<CharState2Text>,
        ),
    >,
    mut state2_texts: Query<
        (&mut Text, &CharState2Text),
        (
            Without<CharDialogWidget>,
            Without<CharNameText>,
            Without<CharGuildText>,
            Without<CharStatText>,
        ),
    >,
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

    let open = mgr.is_open(DialogKind::Character);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    for (mut vis, bg) in &mut page_bgs {
        *vis = if open && bg.0 == page.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    if !assign_key.visible {
        for (e, inter, tab) in &tabs {
            if edge(e, inter, &mut prev_inter) {
                page.0 = tab.0;
            }
        }
        for (e, inter) in &close {
            if edge(e, inter, &mut prev_inter) {
                mgr.close(DialogKind::Character);
            }
        }
    }
    // 文本
    if let Ok(mut t) = name_texts.single_mut() {
        t.0 = state.name.clone();
    }
    if let Ok(mut t) = guild_texts.single_mut() {
        t.0 = state.guild.clone();
    }
    for (mut t, idx) in &mut stat_texts {
        t.0 = stat_label_text(idx.0, &state);
    }
    for (mut t, idx) in &mut state2_texts {
        t.0 = match idx.0 {
            0 => format!(
                "{:.2}%",
                if state.max_exp > 0 {
                    state.exp as f64 * 100.0 / state.max_exp as f64
                } else {
                    0.0
                }
            ),
            1 => format!("{}", state.bag_weight),
            2 => format!("{}", state.wear_weight),
            3 => format!("{}", state.hand_weight),
            4 => format!("+{}", state.magic_resist),
            5 => format!("+{}", state.poison_resist),
            6 => format!("+{}", state.health_recovery),
            7 => format!("+{}", state.spell_recovery),
            8 => format!("+{}", state.poison_recovery),
            9 => format!("+{}", state.holy),
            10 => format!("+{}", state.freezing),
            11 => format!("+{}", state.poison_atk),
            _ => String::new(),
        };
    }
}

/// 装备图标（从 HudState.equipment 渲染 Items 库图标）+ 右键卸下装备
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn char_equip_system(
    hud: Res<HudState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut icons: Query<
        (&mut ImageNode, &mut Visibility, &CharEquipIcon),
        (Without<CharDialogWidget>, Without<CharEquipSlot>),
    >,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    net: Res<NetConnection>,
    mgr: Res<DialogManager>,
    page: Res<CharPage>,
) {
    // 右键卸下装备（原版 C# MirItemCell 右键 → UseItem → Equipment → RemoveItem）
    if mouse.just_pressed(MouseButton::Right) && mgr.is_open(DialogKind::Character) && page.0 == 0
    {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for pos in 0..EQUIP_SLOTS.len() {
                    let Some((sx, sy)) = slot_screen_origin(pos) else {
                        continue;
                    };
                    if cursor.x >= sx
                        && cursor.x <= sx + SLOT_W
                        && cursor.y >= sy
                        && cursor.y <= sy + SLOT_H
                    {
                        if let Some(server_idx) = SERVER_SLOT_TO_POS.iter().position(|p| *p == pos) {
                            if let Some(item) = hud
                                .equipment
                                .get(server_idx)
                                .and_then(|s| s.as_ref())
                            {
                                net.send_packet(&mir2_shared::packets::client::item::RemoveItem {
                                    grid: mir2_shared::enums::MirGridType::Inventory,
                                    unique_id: item.unique_id,
                                    to: 0,
                                });
                                tracing::info!(
                                    "🛡️ 右键卸下装备 {} (uid={})",
                                    item.name,
                                    item.unique_id
                                );
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    // 装备图标（ImageNode）
    for (mut node, mut vis, icon) in &mut icons {
        let item = hud.equipment.get(icon.0).and_then(|s| s.as_ref());
        match item {
            Some(item) => {
                let handle = load_lib_image(
                    &mut libs,
                    &mut images,
                    LibraryName::Items,
                    item.image as usize,
                );
                match handle {
                    Some(h) if node.image != h => node.image = h,
                    None => *vis = Visibility::Hidden,
                    _ => {}
                }
                if node.image.is_strong() {
                    *vis = Visibility::Visible;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// 技能页：行可见性/内容/翻页 + 点击打开快捷键面板
/// （C# CharacterDialog.RefreshInterface + MagicButton.Click → AssignKeyPanel）
fn char_skill_system(
    mgr: Res<DialogManager>,
    page: Res<CharPage>,
    magics: Res<MagicsState>,
    cd: Res<crate::game::skills::MagicCooldowns>,
    mut start: ResMut<CharSkillStart>,
    mut assign_key: ResMut<AssignKeyState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut rows: Query<
        (
            Entity,
            &mut Visibility,
            &Interaction,
            Option<&CharSkillRow>,
            Option<&CharSkillNext>,
            Option<&CharSkillBack>,
        ),
        (Without<CharSkillRowChild>, Without<CharSkillText>, Without<CharSkillIcon>),
    >,
    mut children: Query<(&mut Visibility, &CharSkillRowChild), Without<Interaction>>,
    mut icons: Query<(&mut ImageNode, &CharSkillIcon), Without<CharSkillText>>,
    mut texts: Query<(&mut Text, &CharSkillText), Without<CharSkillIcon>>,
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

    if !mgr.is_open(crate::game::dialogs::DialogKind::Character) {
        return;
    }
    let open = mgr.is_open(DialogKind::Character) && page.0 == 3;

    for (e, mut vis, inter, row, next, back) in &mut rows {
        let (show, magic) = if let Some(row) = row {
            let magic = magics.magics.get(start.0 + row.0);
            (open && magic.is_some(), magic)
        } else {
            (open, None) // Next/Back 仅在技能页显示
        };
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
        if !show || assign_key.visible {
            continue;
        }
        if let Some(row) = row {
            if edge(e, inter, &mut prev_inter) {
                if let Some(m) = magic {
                    assign_key.open(m.spell, m.key);
                    tracing::info!(
                        "🔑 打开技能快捷键面板: {} ({:?}) key={}",
                        m.name, m.spell, m.key
                    );
                }
            }
            // 图标帧：按下 = icon*2+1，否则 icon*2（原版 MagIcon2 Index/PressedIndex）
            if let Some(m) = magic {
                let frame = m.icon as usize * 2 + if *inter == Interaction::Pressed { 1 } else { 0 };
                if let Some(h) =
                    load_lib_image(&mut libs, &mut images, LibraryName::MagIcon2, frame)
                {
                    let frac = cd.fraction(m.spell);
                    for (mut node, ic) in &mut icons {
                        if ic.0 == row.0 {
                            if node.image != h {
                                node.image = h.clone();
                            }
                            if frac > 0.0 {
                                let k = 1.0 - 0.7 * frac;
                                node.color = Color::srgba(k, k, k, 1.0);
                            } else {
                                node.color = Color::WHITE;
                            }
                        }
                    }
                }
            }
        } else if edge(e, inter, &mut prev_inter) {
            if next.is_some() && start.0 + SKILL_ROW_COUNT < magics.magics.len() {
                start.0 += SKILL_ROW_COUNT;
            } else if back.is_some() && start.0 >= SKILL_ROW_COUNT {
                start.0 -= SKILL_ROW_COUNT;
            }
        }
    }

    // 子控件可见性：行有技能时显示
    for (mut vis, child) in &mut children {
        let show = open && magics.magics.get(start.0 + child.0).is_some();
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // 文本内容
    for (mut t, txt) in &mut texts {
        if let Some(m) = magics.magics.get(start.0 + txt.row) {
            t.0 = match txt.kind {
                SkillTextKind::Key => key_label(m.key),
                SkillTextKind::Level => m.level.to_string(),
                SkillTextKind::Name => m.name.clone(),
                SkillTextKind::Exp => exp_label(m),
            };
        }
    }
}

/// C# KeyLabel：Key=0 空；1..8 "F1..F8"；9..16 "CTRL\nF1.."；17..24 "Shift\nF1.."
/// （MainDialogs.cs:3422-3425：`Key > 8 ? Environment.NewLine : ""`）
/// 越界防御：key≥25 时 C# 会 IndexOutOfRange——Bevy 返回空串（skill_key_name
/// 的 `_ => ""` 同款防御），不让整客户端 panic（#2584）
fn key_label(key: u8) -> String {
    if key == 0 {
        return String::new();
    }
    let idx = ((key - 1) / 8) as usize;
    if idx >= 3 {
        return String::new();
    }
    let prefix = ["", "CTRL", "Shift"][idx];
    let f = (key - 1) % 8 + 1;
    if key > 8 {
        format!("{}\nF{}", prefix, f)
    } else {
        format!("F{}", f)
    }
}

/// C# ExpLabel：0/1/2 级 "exp/need"；3 级 "-"
fn exp_label(m: &mir2_shared::data::client_data::ClientMagic) -> String {
    match m.level {
        0 => format!("{}/{}", m.experience, m.need1),
        1 => format!("{}/{}", m.experience, m.need2),
        2 => format!("{}/{}", m.experience, m.need3),
        _ => "-".to_string(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equip_slot_screen_rect_weapon() {
        // 武器 server_slot=0 → EQUIP_SLOTS[0]=(123,7) + PAGE(8,90) + DIALOG(760,0)；格尺寸 C# 36x32
        let r = equip_slot_screen_rect(0).unwrap();
        assert_eq!(
            r,
            (1024.0 - 264.0 + 8.0 + 123.0, 0.0 + 90.0 + 7.0, 36.0, 32.0)
        );
        assert_eq!(equip_slot_screen_rect(14), None);
    }

    #[test]
    fn server_slot_mapping_covers_all() {
        assert_eq!(SERVER_SLOT_TO_POS.len(), 14);
        assert_eq!(EQUIP_SLOTS.len(), 14);
    }

    /// 键名标签双行格式 + 越界不 panic（#2584；C# MainDialogs.cs:3422-3425）
    #[test]
    fn key_label_two_line_and_out_of_range_safe() {
        assert_eq!(key_label(0), "");
        assert_eq!(key_label(1), "F1");
        assert_eq!(key_label(8), "F8");
        assert_eq!(key_label(9), "CTRL\nF1");
        assert_eq!(key_label(16), "CTRL\nF8");
        assert_eq!(key_label(17), "Shift\nF1");
        assert_eq!(key_label(24), "Shift\nF8");
        // key≥25：C# IndexOutOfRange——Bevy 防御返回空串
        assert_eq!(key_label(25), "");
        assert_eq!(key_label(255), "");
    }

    /// 页门控护栏（#2505）：跑真实 spawn，断言状态/State 数值标签挂在正确页背景组件上。
    /// C# 证据：StatusPage=Title[506]=页1（:86-93）、StatePage=Title[507]=页2（:111-118），
    /// 13 个状态标签 Parent=StatusPage（:351-452）、12 个 State 标签 Parent=StatePage（:454-548）。
    #[test]
    fn stat_labels_ride_correct_page_bg() {
        use crate::resources::libraries::Libraries;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(GameLibraries(Libraries::new("Data")));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiFont::default());
        world.insert_resource(HudState::default());
        world
            .run_system_once(spawn_character_dialog)
            .expect("spawn_character_dialog 应成功");

        // bevy_ui 结构：数值标签是页容器（挂 CharPageBg(idx)）的子节点。
        // 状态标签 → 父容器 CharPageBg(1)=StatusPage；State 标签 → 父容器 CharPageBg(2)=StatePage。
        fn parent_page_bg(world: &World, e: Entity) -> Option<usize> {
            let parent = world.get::<ChildOf>(e)?.0;
            world.get::<CharPageBg>(parent).map(|b| b.0)
        }

        let mut stat_q = world.query::<(Entity, &CharStatText)>();
        let mut stat_n = 0;
        for (e, _s) in stat_q.iter(&world) {
            stat_n += 1;
            assert_eq!(
                parent_page_bg(&world, e),
                Some(1),
                "状态标签父容器应挂 CharPageBg(1)=StatusPage"
            );
        }
        assert_eq!(stat_n, 13, "C# StatusPage 13 个状态数值标签");

        let mut state_q = world.query::<(Entity, &CharState2Text)>();
        let mut state_n = 0;
        for (e, _s) in state_q.iter(&world) {
            state_n += 1;
            assert_eq!(
                parent_page_bg(&world, e),
                Some(2),
                "State 标签父容器应挂 CharPageBg(2)=StatePage"
            );
        }
        assert_eq!(state_n, 12, "C# StatePage 12 个 State 数值标签");
    }

    /// 状态页数值格式护栏：逐项对照 C# StatusPage.BeforeDraw 字面值（CharacterDialog.cs:96-108）。
    /// 关键：CritD(index 8) C# 无 %（:104），与 CritR(index 7) 的 {0}%（:103）区分。
    #[test]
    fn stat_label_text_matches_csharp() {
        let mut s = CharacterState::default();
        s.hp = 120;
        s.max_hp = 130;
        s.mp = 40;
        s.max_mp = 50;
        s.stats = [[1, 9], [2, 8], [3, 7], [4, 6], [5, 10]]; // AC/MAC/DC/MC/SC [min,max]
        s.critical_rate = 15;
        s.critical_damage = 150;
        s.attack_speed = 3;
        s.accuracy = 7;
        s.agility = 9;
        s.luck = 2;
        assert_eq!(stat_label_text(0, &s), "120/130"); // HP {0}/{1}
        assert_eq!(stat_label_text(1, &s), "40/50"); // MP {0}/{1}
        assert_eq!(stat_label_text(2, &s), "1-9"); // AC {0}-{1}
        assert_eq!(stat_label_text(3, &s), "2-8"); // MAC
        assert_eq!(stat_label_text(4, &s), "3-7"); // DC
        assert_eq!(stat_label_text(5, &s), "4-6"); // MC
        assert_eq!(stat_label_text(6, &s), "5-10"); // SC
        assert_eq!(stat_label_text(7, &s), "15%"); // CritR {0}%
        assert_eq!(stat_label_text(8, &s), "150"); // CritD {0}（C# 无 %）
        assert_eq!(stat_label_text(9, &s), "3"); // AtkSpd {0}
        assert_eq!(stat_label_text(10, &s), "+7"); // Acc +{0}
        assert_eq!(stat_label_text(11, &s), "+9"); // Agil +{0}
        assert_eq!(stat_label_text(12, &s), "2"); // Luck {0}
    }
}
