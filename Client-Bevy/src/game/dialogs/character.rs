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

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
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
            equipment: vec![None; 14],
        }
    }
}

const DIALOG_X: f32 = 1024.0 - 264.0;
const DIALOG_Y: f32 = 0.0;
const PAGE_X: f32 = 8.0;
const PAGE_Y: f32 = 90.0;

/// 装备槽位置（C# EquipmentSlot 顺序）
const EQUIP_SLOTS: [(f32, f32); 14] = [
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
const SLOT_SIZE: f32 = 36.0;

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

#[derive(Component)]
pub struct CharNameText;

#[derive(Component)]
pub struct CharGuildText;

#[derive(Component)]
pub struct CharStatText(pub usize); // 状态数值标签序号

pub struct CharacterDialogPlugin;

impl Plugin for CharacterDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterState>();
        app.init_resource::<CharPage>();
        app.add_systems(OnEnter(AppState::Game), spawn_character_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_character_dialog);
        app.add_systems(
            Update,
            (character_ui_system, ui_button_system)
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

    // 装备槽（14 个，深色底）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for (ox, oy) in EQUIP_SLOTS {
        commands.spawn((
            UiEntity,
            DialogRoot(DialogKind::Character),
            CharDialogWidget,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                custom_size: Some(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(DIALOG_X + ox, -(DIALOG_Y + oy), 6.3),
            Visibility::Hidden,
        ));
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
}

/// 显示/隐藏 + 页切换 + 关闭 + 状态更新
fn character_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<CharacterState>,
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
            _ => String::new(),
        };
    }
}
