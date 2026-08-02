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

use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::character;
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use mir2_shared::enums::MirGridType;

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
        app.init_resource::<InvTooltip>();
        app.init_resource::<InvDropConfirm>();
        app.init_resource::<InvPendingAmount>();
        app.add_systems(OnEnter(AppState::Game), spawn_inventory_dialog);
        app.add_systems(OnEnter(AppState::Game), spawn_inv_confirm);
        app.add_systems(OnExit(AppState::Game), cleanup_dialogs);
        app.add_systems(OnEnter(AppState::Game), spawn_inv_tooltip_text);
        app.add_systems(
            Update,
            (
                inventory_ui_system,
                inv_selection_system,
                inv_tooltip_system,
                inv_tooltip_text_system,
                inv_item_action_system,
                inv_confirm_system,
                ui_button_system,
            )
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

/// 光标坐标 → 背包格（0..39）；供仓库对话框复用（原版 C# MirItemCell 命中语义）
pub fn inv_slot_at(cx: f32, cy: f32) -> Option<usize> {
    for i in 0..(GRID_COLS * GRID_ROWS) {
        let x = i % GRID_COLS;
        let y = i / GRID_COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
            return Some(i);
        }
    }
    None
}

/// 背包格子索引（0..39）
#[derive(Component, Clone, Copy)]
pub struct InvSlot(pub usize);
/// 物品图标（格子子实体）
#[derive(Component, Clone, Copy)]
pub struct InvIcon(pub usize);

/// 堆叠数量文本（格子子实体）
#[derive(Component, Clone, Copy)]
pub struct InvCount(pub usize);

/// 悬停提示（原版 C# MirItemCell.Hint）
#[derive(Resource, Default)]
pub struct InvTooltip {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

/// 双击检测（记录最近一次左键点击的格子与时间）
#[derive(Resource, Default)]
pub struct InvClickState {
    pub last: Option<(usize, f64)>,
    /// 当前选中格子（原版 C# GameScene.SelectedCell）
    pub selected: Option<usize>,
}

/// 丢弃确认框（原版 C# MirMessageBox YesNo：DropTip）
#[derive(Resource, Default)]
pub struct InvDropConfirm {
    pub visible: bool,
    pub text: String,
    pub unique_id: u64,
    pub count: u16,
}

/// 数量框待处理操作（拆分/丢弃，原版 C# MirAmountBox OK 回调）
#[derive(Resource, Default)]
pub struct InvPendingAmount {
    pub split_uid: Option<u64>,
    pub drop_uid: Option<u64>,
}

#[derive(Component)]
pub struct InvConfirmWidget;

#[derive(Component)]
pub struct InvConfirmYes;

#[derive(Component)]
pub struct InvConfirmNo;

/// 显示/隐藏 + 页切换 + 关闭 + 物品图标渲染 + 双击使用/装备
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inventory_ui_system(
    mut mgr: ResMut<DialogManager>,
    hud: Res<HudState>,
    mut page: ResMut<InvPage>,
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

/// 悬停提示系统：光标在物品格上时显示 名称 x数量
fn inv_tooltip_system(
    inv: Res<crate::game::hud::HudState>,
    mut tooltip: ResMut<InvTooltip>,
    windows: Query<&Window>,
    slots: Query<&InvSlot, Without<InvIcon>>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let mut text = String::new();
    for slot in &slots {
        let i = slot.0;
        let x = i % GRID_COLS;
        let y = i / GRID_COLS;
        let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
        let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
        if cursor.x >= sx && cursor.x <= sx + CELL_W && cursor.y >= sy && cursor.y <= sy + CELL_H {
            if let Some(item) = inv.inventory.items.get(i).and_then(|s| s.as_ref()) {
                if item.count > 1 {
                    text = format!("{} x{}", item.name, item.count);
                } else {
                    text = item.name.clone();
                }
            }
            break;
        }
    }
    if tooltip.text != text {
        tooltip.text = text;
    }
    tooltip.x = cursor.x + 14.0;
    tooltip.y = cursor.y + 14.0;
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

/// 悬停提示文本实体（常驻，跟随光标）
#[derive(Component)]
pub struct InvTooltipText;

fn spawn_inv_tooltip_text(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    commands.spawn((
        UiEntity,
        InvTooltipText,
        Text2d::new(String::new()),
        Anchor::TOP_LEFT,
        TextFont {
            font: FontSource::Handle(font),
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.85)),
        Transform::from_xyz(0.0, 0.0, 9.0),
        Visibility::Hidden,
    ));
}

/// 更新提示文本位置与内容
fn inv_tooltip_text_system(
    tooltip: Res<InvTooltip>,
    mut texts: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<InvTooltipText>>,
) {
    for (mut text, mut tf, mut vis) in &mut texts {
        if tooltip.text.is_empty() {
            *vis = Visibility::Hidden;
            continue;
        }
        text.0 = tooltip.text.clone();
        // 跟随光标（右上偏移，避免遮住物品）
        tf.translation.x = tooltip.x + 4.0;
        tf.translation.y = -(tooltip.y + 4.0);
        *vis = Visibility::Visible;
    }
}

/// 使用/装备物品（原版 C# MirItemCell.UseItem：右键/双击触发）
fn use_or_equip(item: &InvItem, net: &NetworkContext) {
    if item.is_equipment() {
        if let Some(to) = item.equip_slot() {
            net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                // 协议字段为 MirGridType；服务端按 unique_id 定位背包格
                grid: MirGridType::Inventory,
                unique_id: item.unique_id,
                to,
            });
            tracing::info!(
                "⚔️ 使用/装备 {} (uid={}) -> 槽 {}",
                item.name,
                item.unique_id,
                to
            );
        }
    } else if item.is_usable() {
        net.send_packet(&mir2_shared::packets::client::item::UseItem {
            unique_id: item.unique_id,
        });
        tracing::info!("💊 使用 {} (uid={})", item.name, item.unique_id);
    } else {
        tracing::debug!("背包物品 {} 不可用/不可装备", item.name);
    }
}

/// 生成丢弃确认框（原版 C# MirMessageBox：Prguse[360] 456x190，Yes/No Title[206-208]/[210-212]）
fn spawn_inv_confirm(
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
    // MirMessageBox 居中（原版 456x190 → (1024-456)/2=284, (768-190)/2=289）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands.entity(e).insert((InvConfirmWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 35.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((InvConfirmWidget, Visibility::Hidden));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 260.0, by + 157.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((InvConfirmYes, InvConfirmWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 360.0, by + 157.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((InvConfirmNo, InvConfirmWidget));
    }
}

/// 丢弃确认框：Yes → DropItem；No → 关闭（原版 C# MirMessageBox YesNo）
fn inv_confirm_system(
    mut confirm: ResMut<InvDropConfirm>,
    mut click: ResMut<InvClickState>,
    net: Res<NetworkContext>,
    mut widgets: Query<&mut Visibility, With<InvConfirmWidget>>,
    yes: Query<&UiButton, (With<InvConfirmYes>, Without<InvConfirmNo>)>,
    no: Query<&UiButton, (With<InvConfirmNo>, Without<InvConfirmYes>)>,
) {
    for mut vis in &mut widgets {
        *vis = if confirm.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !confirm.visible {
        return;
    }
    for btn in &yes {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::item::DropItem {
                unique_id: confirm.unique_id,
                count: confirm.count as u32,
                hero_inventory: false,
            });
            tracing::info!(
                "🗑️ 确认丢弃 uid={} count={}",
                confirm.unique_id,
                confirm.count
            );
            confirm.visible = false;
            click.selected = None;
        }
    }
    for btn in &no {
        if btn.clicked {
            confirm.visible = false;
        }
    }
}

/// 物品高级交互：
///   - 右键 → 使用/装备（原版 C# MouseButtons.Right → UseItem）
///   - Shift+左键 → 拆分堆叠（MirAmountBox → SplitItem）
///   - 选中物品 + 点场景地面 → 丢弃（单件 YesNo 确认 / 多件数量框 → DropItem）
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn inv_item_action_system(
    hud: Res<HudState>,
    mgr: Res<DialogManager>,
    mut click: ResMut<InvClickState>,
    net: Res<NetworkContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    time: Res<Time>,
    mut amount: ResMut<AmountBoxState>,
    mut confirm: ResMut<InvDropConfirm>,
    npc_goods: Res<crate::game::dialogs::npc_goods::NpcGoodsState>,
    mut pending: ResMut<InvPendingAmount>,
    mut result: MessageReader<AmountBoxResult>,
    all_buttons: Query<&UiButton>,
    // 弹窗模态门：上一帧有弹窗 → 本帧点击视为弹窗按钮，不处理格子（原版 C# Modal）
    mut last_modal: Local<bool>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // 数量框结果：拆分/丢弃
    for r in result.read() {
        let Some(n) = r.0 else {
            pending.split_uid = None;
            pending.drop_uid = None;
            continue;
        };
        if n == 0 {
            continue;
        }
        if let Some(uid) = pending.split_uid.take() {
            net.send_packet(&mir2_shared::packets::client::item::SplitItem {
                grid: MirGridType::Inventory,
                unique_id: uid,
                count: n,
            });
            tracing::info!("🔪 拆分物品 uid={} count={}", uid, n);
        } else if let Some(uid) = pending.drop_uid.take() {
            net.send_packet(&mir2_shared::packets::client::item::DropItem {
                unique_id: uid,
                count: n,
                hero_inventory: false,
            });
            tracing::info!("🗑️ 丢弃物品 uid={} count={}", uid, n);
        }
    }

    // 光标下的背包格
    let slot_at = |cx: f32, cy: f32| -> Option<usize> {
        for i in 0..(GRID_COLS * GRID_ROWS) {
            let x = i % GRID_COLS;
            let y = i / GRID_COLS;
            let sx = DIALOG_X + 9.0 + x as f32 * (CELL_W + 1.0);
            let sy = DIALOG_Y + 37.0 + y as f32 * (CELL_H + 1.0);
            if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
                return Some(i);
            }
        }
        None
    };

    // 弹窗模态门（原版 C# Modal：弹窗打开期间/刚关闭帧不响应格子点击）
    let modal_now = amount.visible || confirm.visible;
    let modal_was = *last_modal;
    *last_modal = modal_now;
    if modal_was || modal_now {
        return;
    }

    // 双击/单击检测（原版 C# MirItemCell.OnMouseDoubleClick / OnMouseClick）
    let now = time.elapsed_secs_f64();
    let mut dbl: Option<usize> = None;
    let mut single: Option<usize> = None;
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
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
                        grid: MirGridType::Inventory,
                        from: from as i32,
                        to: i as i32,
                    });
                    tracing::info!("📦 移动物品 {} -> {}", from, i);
                    click.selected = None;
                }
                None => {
                    // 只有物品格可选中（空格不选中）
                    if hud.inventory.items.get(i).and_then(|s| s.as_ref()).is_some() {
                        click.selected = Some(i);
                    }
                }
            }
        }
    }
    // 双击：使用/装备
    if let Some(i) = dbl {
        if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
            use_or_equip(item, &net);
        }
    }

    // 右键：使用/装备
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                use_or_equip(item, &net);
            }
        }
    }

    // Alt+左键：快速出售（原版 C# "Add support for ALT + click to sell quickly"）
    if mouse.just_pressed(MouseButton::Left) && keys.pressed(KeyCode::AltLeft) {
        if npc_goods.visible {
            if let Some(i) = slot_at(cursor.x, cursor.y) {
                if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&mir2_shared::packets::client::npc::SellItem {
                        unique_id: item.unique_id,
                        count: 1,
                    });
                    tracing::info!("💰 出售 {} (uid={})", item.name, item.unique_id);
                }
            }
        }
        return;
    }

    // Shift+左键：拆分堆叠
    if mouse.just_pressed(MouseButton::Left) && keys.pressed(KeyCode::ShiftLeft) {
        if let Some(i) = slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
                if item.count > 1 {
                    if !hud.inventory.items.iter().any(|s| s.is_none()) {
                        tracing::warn!("背包已满，无法拆分");
                        return;
                    }
                    amount.ask("拆分数量", (item.count - 1) as u32);
                    pending.split_uid = Some(item.unique_id);
                    tracing::info!(
                        "🔪 拆分 {} (uid={}) 最大 {}",
                        item.name,
                        item.unique_id,
                        item.count - 1
                    );
                    return;
                }
            }
        }
    }

    // 选中物品 + 左键点场景（非背包格/非装备格/非按钮/非背包面板）→ 丢弃流程
    if mouse.just_pressed(MouseButton::Left) {
        let Some(sel) = click.selected else { return };
        if slot_at(cursor.x, cursor.y).is_some() {
            return;
        }
        // 背包面板背景内不触发（原版：点对话框不丢物品）
        if cursor.x >= DIALOG_X
            && cursor.x <= DIALOG_X + 318.0
            && cursor.y >= DIALOG_Y
            && cursor.y <= DIALOG_Y + 256.0
        {
            return;
        }
        // 角色对话框装备格区域不触发
        if mgr.is_open(DialogKind::Character) {
            let in_eq = character::EQUIP_SLOTS.iter().any(|(ox, oy)| {
                let sx = character::DIALOG_X + ox;
                let sy = character::DIALOG_Y + oy;
                cursor.x >= sx
                    && cursor.x <= sx + character::SLOT_SIZE
                    && cursor.y >= sy
                    && cursor.y <= sy + character::SLOT_SIZE
            });
            if in_eq {
                return;
            }
        }
        // 任意 UI 按钮上不触发
        let over_btn = all_buttons.iter().any(|b| {
            let (x, y, w, h) = b.rect;
            cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h
        });
        if over_btn {
            return;
        }
        let Some(item) = hud.inventory.items.get(sel).and_then(|s| s.as_ref()) else {
            click.selected = None;
            return;
        };
        if item.count > 1 {
            amount.ask("丢弃数量", item.count as u32);
            pending.drop_uid = Some(item.unique_id);
        } else {
            confirm.text = format!("确定丢弃 {} 吗？", item.name);
            confirm.unique_id = item.unique_id;
            confirm.count = 1;
            confirm.visible = true;
        }
        tracing::info!("🗑️ 准备丢弃 {} (uid={})", item.name, item.unique_id);
        click.selected = None;
    }
}

