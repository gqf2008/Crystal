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
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};

/// 背包物品条目（网络 UserInformation 写入）
#[derive(Debug, Clone, Default)]
pub struct InvItem {
    pub unique_id: u64,
    pub item_index: i32,
    pub name: String,
    pub image: u16,
    pub count: u16,
    /// ItemType 枚举值（SharedRust），用于区分使用/装备
    pub item_type: u8,
    /// ItemInfo.shape（部分物品按形状区分用途）
    pub shape: i16,
}

impl InvItem {
    /// 是否可装备（Weapon/Armour/Helmet/Torch/Necklace/Bracelet/Ring/Amulet/Belt/Boots/Stone/Mount）
    pub fn is_equipment(&self) -> bool {
        use mir2_shared::enums::ItemType;
        matches!(
            ItemType::try_from(self.item_type),
            Ok(ItemType::Weapon)
                | Ok(ItemType::Armour)
                | Ok(ItemType::Helmet)
                | Ok(ItemType::Torch)
                | Ok(ItemType::Necklace)
                | Ok(ItemType::Bracelet)
                | Ok(ItemType::Ring)
                | Ok(ItemType::Amulet)
                | Ok(ItemType::Belt)
                | Ok(ItemType::Boots)
                | Ok(ItemType::Stone)
                | Ok(ItemType::Mount)
        )
    }

    /// 计算装备目标槽位（ServerRust EquipmentSlot 值 0..10，见 actors/inventory.rs）
    pub fn equip_slot(&self) -> Option<i32> {
        use mir2_shared::enums::ItemType;
        let t = ItemType::try_from(self.item_type).ok()?;
        let s: i32 = match t {
            ItemType::Weapon => 0,   // Weapon
            ItemType::Armour => 1,   // Armour
            ItemType::Helmet => 2,   // Helmet
            ItemType::Necklace => 3, // Necklace
            ItemType::Bracelet => 5, // BraceletR
            ItemType::Ring => 7,     // RingR
            ItemType::Amulet => 9,   // Pendant
            ItemType::Boots => 8,    // Shoes
            ItemType::Mount => 10,   // Mount
            _ => return None,
        };
        Some(s)
    }

    /// 是否可双击使用（药水/卷轴/书/食物/脚本/宠物/变身/装饰等）
    pub fn is_usable(&self) -> bool {
        use mir2_shared::enums::ItemType;
        matches!(
            ItemType::try_from(self.item_type),
            Ok(ItemType::Potion)
                | Ok(ItemType::Scroll)
                | Ok(ItemType::Book)
                | Ok(ItemType::Food)
                | Ok(ItemType::Script)
                | Ok(ItemType::Pets)
                | Ok(ItemType::Transform)
                | Ok(ItemType::Deco)
                | Ok(ItemType::MonsterSpawn)
                | Ok(ItemType::SealedHero)
                | Ok(ItemType::Ore)
                | Ok(ItemType::Meat)
                | Ok(ItemType::CraftingMaterial)
                | Ok(ItemType::Gem)
                | Ok(ItemType::Fish)
        )
    }
}

/// 背包数据（网络 UserInformation.inventory 写入）
#[derive(Resource, Default)]
pub struct InventoryState {
    /// 40 格背包
    pub items: Vec<Option<InvItem>>,
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
        app.init_resource::<InvClickState>();
        app.add_systems(OnEnter(AppState::Game), spawn_inventory_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_dialogs);
        app.add_systems(
            Update,
            (inventory_ui_system, inv_selection_system, ui_button_system)
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
    if let Some(h) = ui_image(
        &mut libs,
        &mut images,
        &mut cache,
        crate::resources::libraries::LibraryName::Title,
        196,
    ) {
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
            &mut commands,
            &mut libs,
            &mut images,
            &mut cache,
            crate::resources::libraries::LibraryName::Title,
            normal,
            hover,
            hover,
            DIALOG_X + x,
            DIALOG_Y + 7.0,
            7.0,
            72.0,
            23.0,
        ) {
            commands.entity(e).insert((
                InvTab(idx),
                DialogRoot(DialogKind::Inventory),
                DialogWidget,
            ));
        }
    }

    // 关闭按钮（Prguse2 360/361/362）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        crate::resources::libraries::LibraryName::Prguse2,
        360,
        361,
        362,
        DIALOG_X + 289.0,
        DIALOG_Y + 3.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands
            .entity(e)
            .insert((InvCloseBtn, DialogRoot(DialogKind::Inventory), DialogWidget));
    }

    // 金币/负重文本
    let gold = spawn_ui_text(
        &mut commands,
        &font,
        "0",
        DIALOG_X + 40.0,
        DIALOG_Y + 210.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands
        .entity(gold)
        .insert((InvGoldText, DialogRoot(DialogKind::Inventory), DialogWidget));
    let weight = spawn_ui_text(
        &mut commands,
        &font,
        "0/0",
        DIALOG_X + 268.0,
        DIALOG_Y + 210.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(weight).insert((
        InvWeightText,
        DialogRoot(DialogKind::Inventory),
        DialogWidget,
    ));

    // 格子背景（40 格，8x5），每格带物品图标 + 堆叠数量
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let x = i % GRID_COLS;
        let y = i / GRID_COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        let slot = commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Inventory),
                DialogWidget,
                InvSlot(i),
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.18),
                    custom_size: Some(Vec2::new(CELL_W, CELL_H)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(sx, -sy, 6.5),
                Visibility::Hidden,
            ))
            .id();
        commands.entity(slot).with_children(|p| {
            // 物品图标（数据驱动：按 InvItem.image 从 Items 库取帧）
            p.spawn((
                InvIcon(i),
                Sprite {
                    image: white.clone(),
                    custom_size: Some(Vec2::new(CELL_W - 4.0, CELL_H - 4.0)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(2.0, -2.0, 6.6),
                Visibility::Hidden,
            ));
            // 堆叠数量（右下角小字）
            p.spawn((
                InvCount(i),
                Text2d::new(String::new()),
                Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.6)),
                Transform::from_xyz(CELL_W - 16.0, -(CELL_H - 13.0), 6.7),
                Visibility::Hidden,
            ));
        });
    }
}

#[derive(Component)]
struct InvCloseBtn;

/// 背包格子索引（0..39）
#[derive(Component, Clone, Copy)]
pub struct InvSlot(pub usize);
/// 物品图标（格子子实体）
#[derive(Component, Clone, Copy)]
pub struct InvIcon(pub usize);

/// 堆叠数量文本（格子子实体）
#[derive(Component, Clone, Copy)]
pub struct InvCount(pub usize);

/// 双击检测（记录最近一次左键点击的格子与时间）
#[derive(Resource, Default)]
pub struct InvClickState {
    pub last: Option<(usize, f64)>,
    /// 当前选中格子（原版 C# GameScene.SelectedCell）
    pub selected: Option<usize>,
}

/// 显示/隐藏 + 页切换 + 关闭 + 物品图标渲染 + 双击使用/装备
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inventory_ui_system(
    mut mgr: ResMut<DialogManager>,
    hud: Res<HudState>,
    mut page: ResMut<InvPage>,
    mut click: ResMut<InvClickState>,
    net: Res<NetworkContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // 背景/标签/关闭/格子 统一显隐（格子带 InvSlot）
    mut all_vis: Query<
        (&mut Visibility, Option<&InvSlot>),
        (
            With<DialogWidget>,
            Without<InvGoldText>,
            Without<InvWeightText>,
        ),
    >,
    mut icons: Query<
        (&mut Sprite, &mut Visibility, &InvIcon),
        (Without<DialogWidget>, Without<InvSlot>, Without<InvCount>),
    >,
    mut counts: Query<
        (&mut Text2d, &mut Visibility, &InvCount),
        (
            Without<DialogWidget>,
            Without<InvSlot>,
            Without<InvIcon>,
            Without<InvGoldText>,
            Without<InvWeightText>,
        ),
    >,
    buttons: Query<
        (&UiButton, Option<&InvTab>),
        (
            With<DialogWidget>,
            Without<InvSlot>,
            Without<InvGoldText>,
            Without<InvWeightText>,
        ),
    >,
    mut money: Query<
        (
            &mut Text2d,
            &mut Visibility,
            Option<&InvGoldText>,
            Option<&InvWeightText>,
        ),
        (
            With<DialogWidget>,
            Or<(With<InvGoldText>, With<InvWeightText>)>,
            Without<InvCount>,
            Without<InvSlot>,
        ),
    >,
) {
    let inv = &hud.inventory;
    let open = mgr.is_open(DialogKind::Inventory);
    for (mut vis, _slot) in &mut all_vis {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }

    // 物品图标（Items 库 item.image 帧）+ 堆叠数量
    for (mut sprite, mut vis, icon) in &mut icons {
        let item = inv.items.get(icon.0).and_then(|s| s.as_ref());
        match item {
            Some(item) => {
                let handle = ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    crate::resources::libraries::LibraryName::Items,
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
    for (mut text, mut vis, count) in &mut counts {
        let item = inv.items.get(count.0).and_then(|s| s.as_ref());
        match item {
            Some(item) if item.count > 1 => {
                text.0 = format!("{}", item.count);
                *vis = Visibility::Visible;
            }
            _ => {
                text.0 = String::new();
                *vis = Visibility::Hidden;
            }
        }
    }

    // 双击格子 → 使用/装备（原版 C# MirItemCell.OnMouseDoubleClick → UseItem）
    let now = time.elapsed_secs_f64();
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let mut dbl: Option<usize> = None;
    // 本次左键点击的格子（原版 C#：选中格子后点另一格 = MoveItem）
    let mut single: Option<usize> = None;
    if mouse.just_pressed(MouseButton::Left) {
        for (_vis, slot) in &all_vis {
            let Some(slot) = slot else { continue };
            let i = slot.0;
            let x = i % GRID_COLS;
            let y = i / GRID_COLS;
            let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
            let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
            if cursor.x >= sx
                && cursor.x <= sx + CELL_W
                && cursor.y >= sy
                && cursor.y <= sy + CELL_H
            {
                if let Some((last_i, last_t)) = click.last {
                    if last_i == i && now - last_t < 0.4 {
                        dbl = Some(i);
                        click.last = None;
                    } else {
                        click.last = Some((i, now));
                        single = Some(i);
                    }
                } else {
                    click.last = Some((i, now));
                    single = Some(i);
                }
                break;
            }
        }
    }
    // 单击：选中 → 移动（MoveItem，原版 C# MirItemCell.MoveItem）
    if dbl.is_none() {
        if let Some(i) = single {
            match click.selected {
                Some(from) if from == i => click.selected = None,
                Some(from) => {
                    // 目标格子可空可满（服务端处理交换/合并）
                    net.send_packet(&mir2_shared::packets::client::item::MoveItem {
                        grid: mir2_shared::enums::MirGridType::Inventory,
                        from: from as i32,
                        to: i as i32,
                    });
                    tracing::info!("📦 移动物品 {} -> {}", from, i);
                    click.selected = None;
                }
                None => {
                    // 只有物品格可选中（空格不选中）
                    if inv.items.get(i).and_then(|s| s.as_ref()).is_some() {
                        click.selected = Some(i);
                        tracing::debug!("🎒 选中格子 {}", i);
                    }
                }
            }
        }
    }
    if let Some(i) = dbl {
        if let Some(item) = inv.items.get(i).and_then(|s| s.as_ref()) {
            if item.is_equipment() {
                if let Some(to) = item.equip_slot() {
                    net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                        // 协议字段为 MirGridType；服务端按 unique_id 定位背包格
                        grid: mir2_shared::enums::MirGridType::Inventory,
                        unique_id: item.unique_id,
                        to,
                    });
                    tracing::info!(
                        "⚔️ 双击装备 {} (uid={}) -> 槽 {}",
                        item.name,
                        item.unique_id,
                        to
                    );
                }
            } else if item.is_usable() {
                net.send_packet(&mir2_shared::packets::client::item::UseItem {
                    unique_id: item.unique_id,
                });
                tracing::info!("💊 双击使用 {} (uid={})", item.name, item.unique_id);
            } else {
                tracing::debug!("背包物品 {} 不可用/不可装备", item.name);
            }
        }
    }

    // 标签页切换 / 关闭按钮
    for (btn, tab) in &buttons {
        if btn.clicked {
            match tab {
                Some(t) => {
                    page.0 = t.0;
                    tracing::debug!("背包页 -> {}", t.0);
                }
                None => mgr.close(DialogKind::Inventory),
            }
        }
    }
    for (mut t, _vis, gold, weight) in &mut money {
        if gold.is_some() {
            t.0 = format!("{}", inv.gold);
        } else if weight.is_some() {
            t.0 = format!("{}/{}", inv.weight, inv.max_weight);
        }
    }
}

/// 选中格子高亮（原版 C# SelectedCell 黄色边框语义：用黄色半透明覆盖表示）
fn inv_selection_system(
    click: Res<InvClickState>,
    mut slots: Query<(&mut Sprite, &InvSlot), Without<InvIcon>>,
) {
    for (mut sprite, slot) in &mut slots {
        let selected = click.selected == Some(slot.0);
        let target = if selected {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if sprite.color != target {
            sprite.color = target;
        }
    }
}
