// ============================================================================
// 背包对话框（M9 第一批）
// 布局参考：Client/MirScenes/Dialogs/InventoryDialog.cs
//   - 窗口位置 (182, 217)，背景 Title[196]
//   - 标签页：道具(6,7) / 道具2(76,7) / 任务(146,7)，72x23
//   - 关闭按钮 (289,3) Prguse2[360/361/362]
//   - 金币 (40,212) 111x14；负重 (268,212)
//   - 格子：8 列 x 5 行，cell 36x32，起点 (9,37)，x 间隔 1
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

/// 背包数据（网络写入；当前服务器 UserInformation has_inventory=false，先为空）
#[derive(Resource, Default)]
pub struct InventoryState {
    /// 40 格背包（页 1/2 各 40）
    pub items: Vec<Option<u32>>,
    pub gold: u32,
    pub weight: u32,
    pub max_weight: u32,
}

const DIALOG_X: f32 = 182.0;
const DIALOG_Y: f32 = 217.0;
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 5;
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;

#[derive(Component)]
pub struct InventoryPanel;

/// 背包对话框内所有 UI 元素（统一显隐）
#[derive(Component)]
pub struct DialogWidget;

#[derive(Component)]
pub struct InvTab(pub usize); // 0=道具 1=道具2 2=任务

#[derive(Component)]
pub struct InvGoldText;

#[derive(Component)]
pub struct InvWeightText;

/// 页切换（当前显示页）
#[derive(Resource, Default)]
pub struct InvPage(pub usize);

pub struct InventoryDialogPlugin;

impl Plugin for InventoryDialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InvPage>();
        app.add_systems(OnEnter(AppState::Game), spawn_inventory_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_dialogs);
        app.add_systems(
            Update,
            (inventory_ui_system, ui_button_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );

    }
}

fn cleanup_dialogs(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 生成背包对话框实体（初始隐藏，由 HUD 按钮/管理器显示）
fn spawn_inventory_dialog(
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

    // 背景 Title[196]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, crate::resources::libraries::LibraryName::Title, 196) {
        let e = spawn_ui_sprite(&mut commands, h, DIALOG_X, DIALOG_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Inventory),
            InventoryPanel,
            DialogWidget,
            Visibility::Hidden,
        ));
    }

    // 标签页按钮（Title 737/197 道具，738/168 道具2，739/198 任务）
    let tabs: [(usize, usize, usize, f32); 3] = [
        (0, 737, 197, 6.0),
        (1, 738, 168, 76.0),
        (2, 739, 198, 146.0),
    ];
    for (idx, normal, hover, x) in tabs {
        if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
            &mut commands, &mut libs, &mut images, &mut cache,
            crate::resources::libraries::LibraryName::Title, normal, hover, hover,
            DIALOG_X + x, DIALOG_Y + 7.0, 7.0, 72.0, 23.0,
        ) {
            commands.entity(e).insert((InvTab(idx), DialogRoot(DialogKind::Inventory), DialogWidget));
        }
    }

    // 关闭按钮（Prguse2 360/361/362）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        crate::resources::libraries::LibraryName::Prguse2, 360, 361, 362,
        DIALOG_X + 289.0, DIALOG_Y + 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((InvCloseBtn, DialogRoot(DialogKind::Inventory), DialogWidget));
    }

    // 金币/负重文本
    let gold = spawn_ui_text(&mut commands, &font, "0", DIALOG_X + 40.0, DIALOG_Y + 210.0, 12.0, Color::WHITE, 8.0);
    commands.entity(gold).insert((InvGoldText, DialogRoot(DialogKind::Inventory), DialogWidget));
    let weight = spawn_ui_text(&mut commands, &font, "0/0", DIALOG_X + 268.0, DIALOG_Y + 210.0, 12.0, Color::WHITE, 8.0);
    commands.entity(weight).insert((InvWeightText, DialogRoot(DialogKind::Inventory), DialogWidget));

    // 格子背景（40 格，8x5）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let x = i % GRID_COLS;
        let y = i / GRID_COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Inventory),
                DialogWidget,
                InvSlot,
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.18),
                    custom_size: Some(Vec2::new(CELL_W, CELL_H)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(sx, -sy, 6.5),
                Visibility::Hidden,
            ));
    }
}

#[derive(Component)]
struct InvCloseBtn;

#[derive(Component)]
struct InvSlot;

/// 显示/隐藏 + 页切换 + 关闭按钮（DialogWidget 统一显隐）
fn inventory_ui_system(
    mut mgr: ResMut<DialogManager>,
    inv: Res<InventoryState>,
    mut page: ResMut<InvPage>,
    mut widgets: Query<&mut Visibility, (With<DialogWidget>, Without<InvSlot>)>,
    tabs: Query<(&UiButton, &InvTab)>,
    close: Query<&UiButton, (With<InvCloseBtn>, Without<InvTab>)>,
    mut gold_texts: Query<&mut Text2d, (With<InvGoldText>, Without<InvWeightText>)>,
    mut weight_texts: Query<&mut Text2d, (With<InvWeightText>, Without<InvGoldText>)>,

) {
    let open = mgr.is_open(DialogKind::Inventory);
    tracing::debug!("[INV] open={} widgets={}", open, widgets.iter().count());
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }

    // 标签页切换
    for (btn, tab) in &tabs {
        if btn.clicked {
            page.0 = tab.0;
            tracing::debug!("背包页 -> {}", tab.0);
        }
    }
    // 关闭按钮
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::Inventory);
        }
    }
    if let Ok(mut t) = gold_texts.single_mut() {
        t.0 = format!("{}", inv.gold);
    }
    if let Ok(mut t) = weight_texts.single_mut() {
        t.0 = format!("{}/{}", inv.weight, inv.max_weight);
    }
}

