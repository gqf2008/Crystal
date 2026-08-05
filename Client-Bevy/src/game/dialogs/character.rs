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
use crate::ui::sprite_ui::{
    spawn_ui_button, spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton,
    UiEntity, UiFont, UiImageCache,
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
            equipment: vec![None; 14],
        }
    }
}

pub(crate) const DIALOG_X: f32 = 1024.0 - 264.0;
pub(crate) const DIALOG_Y: f32 = 0.0;
const PAGE_X: f32 = 8.0;
const PAGE_Y: f32 = 90.0;

/// 装备槽位置（C# EquipmentSlot 顺序）
pub(crate) const EQUIP_SLOTS: [(f32, f32); 14] = [
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
pub(crate) const SLOT_SIZE: f32 = 36.0;

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

pub struct CharacterDialogPlugin;

/// ServerRust equipment 槽位(0..10) → C# EQUIP_SLOTS 位置索引
/// 服务端: 0Weapon 1Armour 2Helmet 3Necklace 4BraceletL 5BraceletR 6RingL 7RingR 8Shoes 9Pendant 10Mount
pub(crate) const SERVER_SLOT_TO_POS: [usize; 12] = [0, 1, 2, 4, 5, 6, 7, 8, 11, 9, 13, 13];

impl Plugin for CharacterDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterState>();
        app.init_resource::<CharPage>();
        app.init_resource::<CharSkillStart>();
        app.add_systems(OnEnter(AppState::Game), spawn_character_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_character_dialog);
        app.add_systems(
            Update,
            (character_ui_system, char_equip_system, char_skill_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 背景 Title[504]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 504) {
        let e = commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Character),
                CharDialogWidget,
                Sprite::from_image(h),
                Anchor::TOP_LEFT,
                Transform::from_xyz(DIALOG_X, -DIALOG_Y, 6.0),
                Visibility::Hidden,
            ))
            .id();
        let _ = e;
    }

    // 页面背景（4 页；显示当前页，其余隐藏）
    let page_bgs: [(usize, LibraryName, usize); 4] = [
        (0, LibraryName::Prguse, 340),
        (1, LibraryName::Title, 506),
        (2, LibraryName::Title, 507),
        (3, LibraryName::Title, 508),
    ];
    for (idx, lib, tex) in page_bgs {
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, lib, tex) {
            let e = spawn_ui_sprite(&mut commands, h, DIALOG_X + PAGE_X, DIALOG_Y + PAGE_Y, 6.2, 1.0);
            commands.entity(e).insert((
                DialogRoot(DialogKind::Character),
                CharPageBg(idx),
                Visibility::Hidden,
            ));
        }
    }

    // 标签页 Title[500-503]（64x20，y=70）
    for (idx, x) in [(0usize, 8.0f32), (1, 70.0), (2, 132.0), (3, 194.0)] {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            LibraryName::Title, 500 + idx, 500 + idx, 500 + idx,
            DIALOG_X + x, DIALOG_Y + 70.0, 7.0, 64.0, 20.0,
        ) {
            commands.entity(e).insert((
                CharTab(idx),
                DialogRoot(DialogKind::Character),
                CharDialogWidget,
            ));
        }
    }

    // 关闭按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        DIALOG_X + 241.0, DIALOG_Y + 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            CharClose,
            DialogRoot(DialogKind::Character),
            CharDialogWidget,
        ));
    }

    // 名字/行会（C# NameLabel (0,12) 264x20；GuildLabel (0,33)）
    let name = spawn_ui_text(&mut commands, &font, "", DIALOG_X + 132.0 - 40.0, DIALOG_Y + 12.0, 14.0, Color::WHITE, 8.0);
    commands.entity(name).insert((
        CharNameText,
        DialogRoot(DialogKind::Character),
        CharDialogWidget,
    ));
    let guild = spawn_ui_text(&mut commands, &font, "", DIALOG_X + 132.0 - 30.0, DIALOG_Y + 33.0, 12.0, Color::srgb(1.0, 0.85, 0.3), 8.0);
    commands.entity(guild).insert((
        CharGuildText,
        DialogRoot(DialogKind::Character),
        CharDialogWidget,
    ));

    // 装备槽（14 个，深色底；服务端 12 槽按 SERVER_SLOT_TO_POS 映射）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for (pos, (ox, oy)) in EQUIP_SLOTS.iter().enumerate() {
        let slot_entity = commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Character),
                CharDialogWidget,
                CharEquipSlot(pos),
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                    custom_size: Some(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(DIALOG_X + ox, -(DIALOG_Y + oy), 6.3),
                Visibility::Hidden,
            ))
            .id();
        // 服务端槽位对应此位置（无对应则 -1）
        let server_idx = SERVER_SLOT_TO_POS.iter().position(|p| *p == pos);
        if let Some(si) = server_idx {
            commands.entity(slot_entity).with_children(|p| {
                p.spawn((
                    CharEquipIcon(si),
                    Sprite {
                        image: white.clone(),
                        custom_size: Some(Vec2::new(SLOT_SIZE - 4.0, SLOT_SIZE - 4.0)),
                        ..default()
                    },
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(2.0, -2.0, 6.4),
                    Visibility::Hidden,
                ));
            });
        }
    }

    // 状态数值标签（Status 页：HP/MP/AC/MAC/DC/MC/SC/CritR/CritD/AtkSpd/Acc/Agil/Luck）
    let stat_ys: [f32; 13] = [
        20.0, 38.0, 56.0, 74.0, 92.0, 110.0, 128.0, 146.0, 164.0, 182.0, 200.0, 218.0, 236.0,
    ];
    for (i, y) in stat_ys.iter().enumerate() {
        let e = spawn_ui_text(
            &mut commands, &font, "0",
            DIALOG_X + PAGE_X + 126.0, DIALOG_Y + PAGE_Y + y - 2.0,
            11.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            CharStatText(i),
            DialogRoot(DialogKind::Character),
            CharDialogWidget,
        ));
    }
    // 技能页（页 3）：7 行技能按钮（C# MagicButton (8, 8+i*33)，231x33）
    // 行内子控件坐标参考 C# MagicButton：图标 (36,0)、LevelImage Title[516] (73,7)、
    // ExpImage Title[517] (73,19)、KeyLabel (2,2)、LevelLabel (88,2)、NameLabel (109,2)、ExpLabel (109,15)
    let transparent = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for i in 0..SKILL_ROW_COUNT {
        let rx = DIALOG_X + PAGE_X + 8.0;
        let ry = DIALOG_Y + PAGE_Y + 8.0 + i as f32 * 33.0;
        let row = commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Character),
                CharSkillRow(i),
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
                Anchor::TOP_LEFT,
                Transform::from_xyz(rx, -ry, 6.5),
                Visibility::Hidden,
            ))
            .id();
        commands.entity(row).with_children(|p| {
            // 技能图标（MagIcon2[icon*2]，原版 SkillButton (36,0)）
            p.spawn((
                CharSkillIcon(i),
                CharSkillRowChild(i),
                Sprite {
                    image: transparent.clone(),
                    custom_size: Some(Vec2::new(36.0, 36.0)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(36.0, 0.0, 6.6),
                Visibility::Hidden,
            ));
            // 等级/经验背景条（Title[516] / Title[517]）
            for (tex, oy) in [(516usize, 7.0f32), (517, 19.0)] {
                if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, tex) {
                    p.spawn((
                        CharSkillRowChild(i),
                        Sprite::from_image(h),
                        Anchor::TOP_LEFT,
                        Transform::from_xyz(73.0, -oy, 6.5),
                        Visibility::Hidden,
                    ));
                }
            }
            // Key/Level/Name/Exp 文本
            for (kind, ox, oy, size) in [
                (SkillTextKind::Key, 2.0f32, 2.0f32, 10.0f32),
                (SkillTextKind::Level, 88.0, 2.0, 11.0),
                (SkillTextKind::Name, 109.0, 2.0, 11.0),
                (SkillTextKind::Exp, 109.0, 15.0, 11.0),
            ] {
                p.spawn((
                    CharSkillText { row: i, kind },
                    CharSkillRowChild(i),
                    Text2d::new(""),
                    Anchor::TOP_LEFT,
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(ox, -oy, 6.7),
                    Visibility::Hidden,
                ));
            }
        });
    }
    // Next/Back（Prguse[396/397] (140,250) / [398/399] (90,250)，页内）
    for (is_next, idx, x) in [
        (true, 396usize, 140.0f32),
        (false, 398usize, 90.0f32),
    ] {
        if let Some(e) = spawn_ui_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            LibraryName::Prguse, idx, idx, idx + 1,
            DIALOG_X + PAGE_X + x, DIALOG_Y + PAGE_Y + 250.0, 7.0, 40.0, 22.0,
        ) {
            let mut ec = commands.entity(e);
            ec.insert((
                DialogRoot(DialogKind::Character),
            ));
            if is_next {
                ec.insert(CharSkillNext);
            } else {
                ec.insert(CharSkillBack);
            }
        }
    }
}

/// 显示/隐藏 + 页切换 + 关闭 + 状态更新
fn character_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<CharacterState>,
    assign_key: Res<AssignKeyState>,
    mut page: ResMut<CharPage>,
    mut widgets: Query<&mut Visibility, (With<CharDialogWidget>, Without<CharPageBg>)>,
    tabs: Query<(&UiButton, &CharTab)>,
    close: Query<&UiButton, (With<CharClose>, Without<CharTab>)>,
    mut page_bgs: Query<(&mut Visibility, &CharPageBg), Without<CharDialogWidget>>,
    mut name_texts: Query<&mut Text2d, (With<CharNameText>, Without<CharGuildText>, Without<CharStatText>)>,
    mut guild_texts: Query<&mut Text2d, (With<CharGuildText>, Without<CharNameText>, Without<CharStatText>)>,
    mut stat_texts: Query<(&mut Text2d, &CharStatText), (Without<CharNameText>, Without<CharGuildText>)>,
) {
    let open = mgr.is_open(DialogKind::Character);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    // 页面背景：仅打开时显示当前页
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
        // 标签页切换
        for (btn, tab) in &tabs {
            if btn.clicked {
                page.0 = tab.0;
            }
        }
        // 关闭
        for btn in &close {
            if btn.clicked {
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
        t.0 = match idx.0 {
            0 => format!("{}/{}", state.hp, state.max_hp),
            1 => format!("{}/{}", state.mp, state.max_mp),
            2 => format!("{}-{}", state.stats[0][0], state.stats[0][1]),
            3 => format!("{}-{}", state.stats[1][0], state.stats[1][1]),
            4 => format!("{}-{}", state.stats[2][0], state.stats[2][1]),
            5 => format!("{}-{}", state.stats[3][0], state.stats[3][1]),
            6 => format!("{}-{}", state.stats[4][0], state.stats[4][1]),
            7 => format!("{}%", state.critical_rate),
            8 => format!("{}%", state.critical_damage),
            9 => format!("{}", state.attack_speed),
            10 => format!("+{}", state.accuracy),
            11 => format!("+{}", state.agility),
            12 => format!("{}", state.luck),
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
    mut cache: ResMut<UiImageCache>,
    mut icons: Query<
        (&mut Sprite, &mut Visibility, &CharEquipIcon),
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
                for (pos, (ox, oy)) in EQUIP_SLOTS.iter().enumerate() {
                    let sx = DIALOG_X + ox;
                    let sy = DIALOG_Y + oy;
                    if cursor.x >= sx
                        && cursor.x <= sx + SLOT_SIZE
                        && cursor.y >= sy
                        && cursor.y <= sy + SLOT_SIZE
                    {
                        // 位置 → 服务端槽位（SERVER_SLOT_TO_POS 反查）
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
    for (mut sprite, mut vis, icon) in &mut icons {
        let item = hud.equipment.get(icon.0).and_then(|s| s.as_ref());
        match item {
            Some(item) => {
                let handle = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Items,
                    item.image as usize,
                );
                match handle {
                    Some(h) if sprite.image != h => sprite.image = h,
                    None => *vis = Visibility::Hidden,
                    _ => {}
                }
                if sprite.image.is_strong() {
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
    mut start: ResMut<CharSkillStart>,
    mut assign_key: ResMut<AssignKeyState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut rows: Query<
        (
            &mut Visibility,
            &UiButton,
            Option<&CharSkillRow>,
            Option<&CharSkillNext>,
            Option<&CharSkillBack>,
        ),
    >,
    mut children: Query<(&mut Visibility, &CharSkillRowChild), Without<UiButton>>,
    mut icons: Query<(&mut Sprite, &CharSkillIcon), Without<CharSkillText>>,
    mut texts: Query<(&mut Text2d, &CharSkillText), Without<CharSkillIcon>>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if !mgr.is_open(crate::game::dialogs::DialogKind::Character) {
        return;
    }

    let open = mgr.is_open(DialogKind::Character) && page.0 == 3;
    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    let mouse_down = mouse.pressed(MouseButton::Left);

    for (mut vis, btn, row, next, back) in &mut rows {
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
            if btn.clicked {
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
                let over = cursor
                    .map(|c| {
                        let (x, y, w, h) = btn.rect;
                        c.x >= x && c.x <= x + w && c.y >= y && c.y <= y + h
                    })
                    .unwrap_or(false);
                let frame = m.icon as usize * 2 + if mouse_down && over { 1 } else { 0 };
                if let Some(h) =
                    ui_image(&mut libs, &mut images, &mut cache, LibraryName::MagIcon2, frame)
                {
                    for (mut sprite, ic) in &mut icons {
                        if ic.0 == row.0 && sprite.image != h {
                            sprite.image = h.clone();
                        }
                    }
                }
            }
        } else if btn.clicked {
            if next.is_some() && start.0 + SKILL_ROW_COUNT < magics.magics.len() {
                start.0 += SKILL_ROW_COUNT;
            } else if back.is_some() && start.0 >= SKILL_ROW_COUNT {
                start.0 -= SKILL_ROW_COUNT;
            }
        }
    }

    // 子控件可见性：行有技能时显示（行/翻页按钮的可见性在 rows 查询中处理）
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
fn key_label(key: u8) -> String {
    if key == 0 {
        return String::new();
    }
    let prefix = ["", "CTRL", "Shift"][((key - 1) / 8) as usize];
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
