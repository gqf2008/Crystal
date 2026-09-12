// ============================================================================
// 物品租赁（物主侧，批10 #2720 对齐 C#）
// 参考：C# `ItemRentDialog`（费用窗，ItemRentDialog.cs:16-170）+
//       `ItemRentingDialog`（物品窗，ItemRentingDialog.cs:20-219）
//   - 两窗同用 Prguse[238]（原生 204x109）：
//       费用窗 Location = (ScreenWidth - W - W/2, H + H/2)        = (718,163)
//       物品窗 Location = (ScreenWidth - W - W/2, H*2 + H/2 + 15)  = (718,287)
//   - 关闭 Prguse2[360..362] @(180,3)；名称标签 (30,8) 150x14；数值标签 (60,42) 150x14
//   - 费用窗：价格按钮 Prguse[28] @(18,46) 32x17（MirAmountBox(116, 金币) → C.ItemRentalFee）、
//             锁定费用 Prguse[250..252] @(22,76) 28x25（C.ItemRentalLockFee）
//   - 物品窗：物品格 @(16,35)（GridType.Renting slot 0）、
//             锁定物品 Prguse[250..252] @(18,76)（期限 1..30 有效 → C.ItemRentalLockItem）、
//             设置期限 Prguse3[7..9] @(46,76) 84x28（→ C.ItemRentalPeriod{Days}）、
//             确认 Prguse3[10..12] @(130,76) 58x28（C.ConfirmItemRental，can_confirm 才可用）
// 网络沿用既有 Rust wire；发起租赁改由浏览窗（`item_rental_browse`）RENT 按钮发
// `C.ItemRentalRequest`（C# `ItemRentalDialog.rentItemButton` 同源）。
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{InvClickState, InvItem};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Gold, Inventory};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_item_cell_ui, spawn_label, spawn_panel, UiItemCellData,
    UiItemCellIcon,
};

/// C# 两窗原生尺寸（Prguse[238]）
const RENT_W: f32 = 204.0;
const RENT_H: f32 = 109.0;
/// C# `ItemRentDialog.Location`（费用窗）：(ScreenWidth - W - W/2, H + H/2)
const FEE_POS: (f32, f32) = (718.0, 163.0);
/// C# `ItemRentingDialog.Location`（物品窗）：(ScreenWidth - W - W/2, H*2 + H/2 + 15)
const ITEM_POS: (f32, f32) = (718.0, 287.0);
const CLOSE_POS: (f32, f32) = (180.0, 3.0);
const NAME_POS: (f32, f32) = (30.0, 8.0);
const VALUE_POS: (f32, f32) = (60.0, 42.0);
/// 费用窗：价格按钮 + 锁定费用
const FEE_PRICE_POS: (f32, f32) = (18.0, 46.0);
const FEE_LOCK_POS: (f32, f32) = (22.0, 76.0);
/// 物品窗：物品格 + 锁定物品 + 设置期限 + 确认
const ITEM_CELL_POS: (f32, f32) = (16.0, 35.0);
const RENTAL_CELL_W: f32 = 34.0;
const RENTAL_CELL_H: f32 = 32.0;
const ITEM_LOCK_POS: (f32, f32) = (18.0, 76.0);
const ITEM_PERIOD_POS: (f32, f32) = (46.0, 76.0);
const ITEM_CONFIRM_POS: (f32, f32) = (130.0, 76.0);
/// C# `InputRentalPeroid`：期限 1..=30
pub const RENTAL_PERIOD_MIN: u32 = 1;
pub const RENTAL_PERIOD_MAX: u32 = 30;

/// C# 期限校验（`RentalPeriod < 1 || > 30` 直接 return）
pub fn rental_period_valid(period: u32) -> bool {
    (RENTAL_PERIOD_MIN..=RENTAL_PERIOD_MAX).contains(&period)
}

/// 租赁会话状态（服务端事件写入）
#[derive(Resource, Default)]
pub struct ItemRentalState {
    pub request_received: bool,
    /// 玩家名（C# `RefreshInterface()`：`_nameLabel.Text = GameScene.User.Name`）
    pub name: String,
    pub has_item: bool,
    pub fee: u32,
    pub period: i32,
    pub can_confirm: bool,
    pub message: String,
    pub confirmed: bool,
    /// 最近存入物品 uid
    pub deposit_uid: Option<u64>,
    /// 已存入物品（本地镜像，用于物品格渲染）
    pub deposit_item: Option<InvItem>,
    /// 费用/物品是否已锁定（C# `Lock()`：锁定键换 253 帧并禁用设限期按钮）
    pub fee_locked: bool,
    pub item_locked: bool,
}

#[derive(Component)]
pub struct ItemRentalWidget;

/// 关闭（两窗各一个）
#[derive(Component)]
pub struct ItemRentalClose;

/// 费用窗：价格按钮 / 锁定费用
#[derive(Component)]
pub struct ItemRentalPriceBtn;

#[derive(Component)]
pub struct ItemRentalLockFeeBtn;

/// 物品窗：物品格 / 锁定物品 / 设置期限 / 确认
#[derive(Component)]
pub struct RentalItemCell;

#[derive(Component)]
pub struct ItemRentalLockItemBtn;

#[derive(Component)]
pub struct ItemRentalPeriodBtn;

#[derive(Component)]
pub struct ItemRentalConfirmBtn;

/// 两窗共用的名称标签（C# `_nameLabel`）
#[derive(Component)]
pub struct ItemRentalNameText;

/// 费用窗数值标签（C# `_rentalPriceLabel`）
#[derive(Component)]
pub struct ItemRentalFeeText;

/// 物品窗数值标签（C# `_rentalPeriodLabel`）
#[derive(Component)]
pub struct ItemRentalPeriodText;

/// 数量输入框用途（费用 / 期限）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RentalInput {
    Fee,
    Period,
}

pub struct ItemRentalPlugin;

impl Plugin for ItemRentalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRentalState>();
        app.add_systems(
            Update,
            rental_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_item_rental);
        app.add_systems(OnExit(AppState::Game), cleanup_item_rental);
        app.add_systems(
            Update,
            (item_rental_ui_system, item_rental_action_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_item_rental(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_item_rental(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    let Some(bg_fee) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 238) else {
        return;
    };
    let Some(bg_item) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 238) else {
        return;
    };

    // ---- 费用窗（C# ItemRentDialog @ (718,163)）----
    let fee_panel = spawn_panel(&mut commands, bg_fee, FEE_POS.0, FEE_POS.1, RENT_W, RENT_H, 30);
    commands.entity(fee_panel).insert((
        DialogRoot(DialogKind::ItemRental),
        ItemRentalWidget,
        Visibility::Hidden,
    ));
    commands.entity(fee_panel).with_children(|p| {
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, CLOSE_POS.0, CLOSE_POS.1, 24.0, 21.0, 10)
                .insert(ItemRentalClose);
        }
        spawn_label(p, &cjk, "", NAME_POS.0, NAME_POS.1, 12.0, Color::WHITE, 9)
            .insert(ItemRentalNameText);
        spawn_label(p, &cjk, "", VALUE_POS.0, VALUE_POS.1, 12.0, Color::WHITE, 9)
            .insert(ItemRentalFeeText);
        // 价格按钮 Prguse[28]（C# 只有 Index，无 hover/pressed 帧）
        if let Some(price) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 28) {
            spawn_icon_button(
                p,
                price.clone(),
                price.clone(),
                price,
                FEE_PRICE_POS.0,
                FEE_PRICE_POS.1,
                32.0,
                17.0,
                10,
            )
            .insert(ItemRentalPriceBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 250),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 251),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 252),
        ) {
            spawn_icon_button(p, n, h, pr, FEE_LOCK_POS.0, FEE_LOCK_POS.1, 28.0, 25.0, 10)
                .insert(ItemRentalLockFeeBtn);
        }
    });

    // ---- 物品窗（C# ItemRentingDialog @ (718,287)）----
    let item_panel = spawn_panel(
        &mut commands,
        bg_item,
        ITEM_POS.0,
        ITEM_POS.1,
        RENT_W,
        RENT_H,
        30,
    );
    commands.entity(item_panel).insert((
        DialogRoot(DialogKind::ItemRental),
        ItemRentalWidget,
        Visibility::Hidden,
    ));
    commands.entity(item_panel).with_children(|p| {
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, CLOSE_POS.0, CLOSE_POS.1, 24.0, 21.0, 10)
                .insert(ItemRentalClose);
        }
        spawn_label(p, &cjk, "", NAME_POS.0, NAME_POS.1, 12.0, Color::WHITE, 9)
            .insert(ItemRentalNameText);
        spawn_label(p, &cjk, "", VALUE_POS.0, VALUE_POS.1, 12.0, Color::WHITE, 9)
            .insert(ItemRentalPeriodText);
        // 物品格（C# MirItemCell GridType.Renting slot 0 @(16,35)）
        spawn_item_cell_ui(
            p,
            &mut images,
            &font,
            ITEM_CELL_POS.0,
            ITEM_CELL_POS.1,
            RENTAL_CELL_W,
            RENTAL_CELL_H,
            9,
            0,
        )
        .insert((RentalItemCell, Button));
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 250),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 251),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 252),
        ) {
            spawn_icon_button(p, n, h, pr, ITEM_LOCK_POS.0, ITEM_LOCK_POS.1, 28.0, 25.0, 10)
                .insert(ItemRentalLockItemBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 7),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 8),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 9),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                ITEM_PERIOD_POS.0,
                ITEM_PERIOD_POS.1,
                84.0,
                28.0,
                10,
            )
            .insert(ItemRentalPeriodBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 10),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 11),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse3, 12),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                ITEM_CONFIRM_POS.0,
                ITEM_CONFIRM_POS.1,
                58.0,
                28.0,
                10,
            )
            .insert(ItemRentalConfirmBtn);
        }
    });
}

/// 显隐 + 标签 + 物品格渲染
#[allow(clippy::too_many_arguments)]
fn item_rental_ui_system(
    mut mgr: ResMut<DialogManager>,
    state: Res<ItemRentalState>,
    net: Res<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut image_cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mut inv_click: ResMut<InvClickState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    close: Query<(Entity, &Interaction), With<ItemRentalClose>>,
    cell: Query<(Entity, &Interaction), With<RentalItemCell>>,
    mut widgets: Query<&mut Visibility, With<ItemRentalWidget>>,
    mut names: Query<&mut Text, With<ItemRentalNameText>>,
    mut fee_texts: Query<
        &mut Text,
        (With<ItemRentalFeeText>, Without<ItemRentalNameText>, Without<ItemRentalPeriodText>),
    >,
    mut period_texts: Query<
        &mut Text,
        (With<ItemRentalPeriodText>, Without<ItemRentalNameText>, Without<ItemRentalFeeText>),
    >,
    mut cells: Query<(&RentalItemCell, &mut UiItemCellData), Without<UiItemCellIcon>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    let open = mgr.is_open(DialogKind::ItemRental);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // 关闭（两窗）：C# `cancelButton.Click → CancelItemRental()`
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::CancelItemRental);
            mgr.close(DialogKind::ItemRental);
        }
    }
    // 物品格：放入背包选中物（C# `ItemCell` GridType.Renting，`C.DepositRentalItem{From,To}`）
    for (e, inter) in &cell {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(sel) = inv_click.selected else {
            continue;
        };
        let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
        let Some(item) = items.get(sel).and_then(|s| s.as_ref()) else {
            continue;
        };
        net.send_packet(&mir2_shared::packets::client::item::DepositRentalItem {
            from: sel as i32,
            to: 0,
        });
        inv_click.selected = None;
    }
    for mut text in &mut names {
        // C# `RefreshInterface()`：`_nameLabel.Text = GameScene.User.Name`
        let new = if state.name.is_empty() {
            "租赁".to_string()
        } else {
            state.name.clone()
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    for mut text in &mut fee_texts {
        // C# `_rentalPriceLabel`：「费用 N 金币」
        let new = format!("费用: {} 金币", state.fee);
        if text.0 != new {
            text.0 = new;
        }
    }
    for mut text in &mut period_texts {
        // C# `_rentalPeriodLabel`：「租赁期限：N 天」
        let new = if state.period > 0 {
            format!("租赁期限: {} 天", state.period)
        } else {
            "租赁期限: 未设置".to_string()
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    for (_cell, mut data) in &mut cells {
        match state.deposit_item.as_ref() {
            Some(item) => {
                data.icon = if item.image > 0 {
                    crate::ui::sprite_ui::ui_image(
                        &mut libs,
                        &mut images,
                        &mut image_cache,
                        LibraryName::Items,
                        item.image as usize,
                    )
                } else {
                    None
                };
                data.count = (item.count > 1).then_some(item.count as u32);
            }
            None => {
                data.icon = None;
                data.count = None;
            }
        }
        data.dura_ratio = None;
    }
}

/// 交互：物品格存入 / 价格（费用）/ 期限 / 双向锁定 / 确认 / 取消
#[allow(clippy::too_many_arguments)]
fn item_rental_action_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ItemRentalState>,
    net: Res<NetConnection>,
    hud_q: Query<&crate::game::hud::HudData>,
    mut amount: ResMut<crate::game::dialogs::amount_box::AmountBoxState>,
    mut amount_result: MessageReader<crate::game::dialogs::amount_box::AmountBoxResult>,
    gold_q: Query<&Gold, With<LocalPlayer>>,
    price_btn: Query<(Entity, &Interaction), With<ItemRentalPriceBtn>>,
    lock_fee_btn: Query<(Entity, &Interaction), With<ItemRentalLockFeeBtn>>,
    period_btn: Query<(Entity, &Interaction), With<ItemRentalPeriodBtn>>,
    lock_item_btn: Query<(Entity, &Interaction), With<ItemRentalLockItemBtn>>,
    confirm_btn: Query<(Entity, &Interaction), With<ItemRentalConfirmBtn>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut pending: Local<Option<RentalInput>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    if !mgr.is_open(DialogKind::ItemRental) {
        return;
    }
    // C# `RefreshInterface()`：名称标签取玩家名（HUD 快照组件）
    if let Ok(hud) = hud_q.single() {
        if state.name != hud.name {
            state.name = hud.name.clone();
        }
    }
    // 数量输入框结果（费用 / 期限共用；C# 用 MirAmountBox / MirInputBox）
    for res in amount_result.read() {
        let Some(value) = res.0 else {
            *pending = None;
            continue;
        };
        match *pending {
            Some(RentalInput::Fee) => {
                if value > 0 {
                    net.send_packet(&mir2_shared::packets::client::item::ItemRentalFee { amount: value });
                    state.fee = value;
                    state.message = format!("设置租赁费用 {}", value);
                }
            }
            Some(RentalInput::Period) => {
                if rental_period_valid(value) {
                    net.send_packet(&mir2_shared::packets::client::item::ItemRentalPeriod { days: value });
                    state.period = value as i32;
                    state.message = format!("设置租赁期限 {} 天", value);
                } else {
                    state.message = format!("期限需在 {}-{} 之间", RENTAL_PERIOD_MIN, RENTAL_PERIOD_MAX);
                }
            }
            None => {}
        }
        *pending = None;
    }
    // 价格按钮 → 数量输入（C# `MirAmountBox(RentalFee, 116, GameScene.Gold)`）
    for (e, inter) in &price_btn {
        if edge(e, inter, &mut prev_inter) {
            let max = gold_q.single().map(|g| g.0).unwrap_or(0);
            if max == 0 {
                state.message = "金币不足".to_string();
                continue;
            }
            amount.ask("租赁费用", max);
            *pending = Some(RentalInput::Fee);
        }
    }
    // 锁定费用（C# `ItemRentalLockFee`）
    for (e, inter) in &lock_fee_btn {
        if edge(e, inter, &mut prev_inter) && !state.fee_locked {
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockFee);
            state.fee_locked = true;
            state.message = "已锁定费用".to_string();
        }
    }
    // 设置期限 → 数量输入（C# `InputRentalPeroid()` → MirInputBox）
    for (e, inter) in &period_btn {
        if edge(e, inter, &mut prev_inter) && !state.item_locked {
            amount.ask("租赁期限(1-30)", RENTAL_PERIOD_MAX);
            if state.period > 0 {
                amount.value = state.period.to_string();
                amount.fresh = false;
            }
            *pending = Some(RentalInput::Period);
        }
    }
    // 锁定物品（C# `_lockButton.Click`：期限 1..30 才发 `ItemRentalLockItem`）
    for (e, inter) in &lock_item_btn {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if !rental_period_valid(state.period.max(0) as u32) {
            state.message = format!("请先设置 {}-{} 天期限", RENTAL_PERIOD_MIN, RENTAL_PERIOD_MAX);
            continue;
        }
        net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockItem);
        state.item_locked = true;
        state.message = "已锁定物品".to_string();
    }
    // 确认（C# `_confirmButton`，双方锁定后可用）
    for (e, inter) in &confirm_btn {
        if edge(e, inter, &mut prev_inter) {
            if !state.can_confirm {
                state.message = "双方锁定后才能确认".to_string();
                continue;
            }
            net.send_packet(&mir2_shared::packets::client::item::ConfirmItemRental);
            state.message = "已发送确认".to_string();
        }
    }
}

/// 消费服务端租赁事件（网络层只广播 ServerEvent；文案在此构造）
fn rental_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut rental: ResMut<ItemRentalState>,
    mut mgr: ResMut<DialogManager>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::RentalRequestReceived => {
                rental.request_received = true;
                rental.message = "收到租赁请求（物主）".to_string();
                // C# `OpenItemRentalDialog()`：开背包 + 费用窗 + 物品窗
                if !mgr.is_open(DialogKind::Inventory) {
                    mgr.open.push(DialogKind::Inventory);
                }
                mgr.open(DialogKind::ItemRental);
            }
            ServerEvent::RentalItemUpdate { has_item, fee, period } => {
                rental.has_item = *has_item;
                rental.fee = *fee;
                rental.period = *period;
                rental.message = format!(
                    "租赁更新: 物品={} 费用={} 期限={}",
                    if rental.has_item { "有" } else { "无" },
                    rental.fee,
                    rental.period
                );
            }
            ServerEvent::RentalFee { fee } => {
                rental.fee = *fee;
                rental.message = format!("租赁费用更新: {}", rental.fee);
            }
            ServerEvent::RentalPeriod { period } => {
                rental.period = *period;
                rental.message = format!("租赁期限更新: {} 天", rental.period);
            }
            ServerEvent::RentalDeposit { uid, success } => {
                rental.deposit_uid = Some(*uid);
                rental.message = format!(
                    "存入租赁物品: {} ({})",
                    uid,
                    if *success { "成功" } else { "失败" }
                );
                if !*success {
                    rental.deposit_item = None;
                }
            }
            ServerEvent::RentalRetrieve { uid, success } => {
                rental.message = format!(
                    "取回租赁物品: {} ({})",
                    uid,
                    if *success { "成功" } else { "失败" }
                );
                rental.has_item = false;
                rental.deposit_item = None;
            }
            ServerEvent::RentalLocked => {
                rental.fee_locked = true;
                rental.message = "费用已锁定".to_string();
            }
            ServerEvent::RentalPartnerLocked => {
                rental.item_locked = true;
                rental.message = "对方已锁定".to_string();
            }
            ServerEvent::RentalCanConfirm { can_confirm } => {
                rental.can_confirm = *can_confirm;
                rental.message = if rental.can_confirm {
                    "双方已锁定，可确认成交".to_string()
                } else {
                    "等待双方锁定".to_string()
                };
            }
            ServerEvent::RentalConfirmed { success } => {
                rental.confirmed = *success;
                rental.message = if *success {
                    "租赁成交".to_string()
                } else {
                    "租赁确认失败".to_string()
                };
            }
            ServerEvent::RentalCancelled => {
                rental.confirmed = false;
                rental.can_confirm = false;
                rental.fee_locked = false;
                rental.item_locked = false;
                rental.deposit_item = None;
                rental.deposit_uid = None;
                rental.message = "租赁已取消".to_string();
                mgr.close(DialogKind::ItemRental);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2720：两窗面板/控件锚点对齐 C#（ItemRentDialog.cs:16-170 / ItemRentingDialog.cs:20-219）
    #[test]
    fn item_rental_layout_matches_csharp_anchors() {
        assert_eq!((RENT_W, RENT_H), (204.0, 109.0)); // Prguse[238]
        assert_eq!(FEE_POS, (718.0, 163.0)); // (1024-204-102, 109+54)
        assert_eq!(ITEM_POS, (718.0, 287.0)); // (1024-204-102, 218+54+15)
        assert_eq!(CLOSE_POS, (180.0, 3.0));
        assert_eq!(NAME_POS, (30.0, 8.0));
        assert_eq!(VALUE_POS, (60.0, 42.0));
        assert_eq!(FEE_PRICE_POS, (18.0, 46.0));
        assert_eq!(FEE_LOCK_POS, (22.0, 76.0));
        assert_eq!(ITEM_CELL_POS, (16.0, 35.0));
        assert_eq!(ITEM_LOCK_POS, (18.0, 76.0));
        assert_eq!(ITEM_PERIOD_POS, (46.0, 76.0));
        assert_eq!(ITEM_CONFIRM_POS, (130.0, 76.0));
        // 控件落在 204x109 面板内
        let inside = |(x, y): (f32, f32), w: f32, h: f32| {
            x >= 0.0 && y >= 0.0 && x + w <= RENT_W && y + h <= RENT_H
        };
        assert!(inside(CLOSE_POS, 24.0, 21.0));
        assert!(inside(FEE_PRICE_POS, 32.0, 17.0));
        assert!(inside(FEE_LOCK_POS, 28.0, 25.0));
        assert!(inside(ITEM_CELL_POS, RENTAL_CELL_W, RENTAL_CELL_H));
        assert!(inside(ITEM_LOCK_POS, 28.0, 25.0));
        assert!(inside(ITEM_PERIOD_POS, 84.0, 28.0));
        assert!(inside(ITEM_CONFIRM_POS, 58.0, 28.0));
    }

    /// #2720：C# 期限校验 1..=30（`InputRentalPeroid` / `_lockButton.Click`）
    #[test]
    fn rental_period_validation() {
        assert!(!rental_period_valid(0));
        assert!(rental_period_valid(1));
        assert!(rental_period_valid(30));
        assert!(!rental_period_valid(31));
    }
}
