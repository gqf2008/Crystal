// ============================================================================
// 物品租赁（批10 #2720：物主/租客双侧 + C# 对方镜像窗）
//
// 参考 C#：
//   - 租客自有：`ItemRentDialog`（费用窗，ItemRentDialog.cs:16-170）
//   - 物主自有：`ItemRentingDialog`（物品窗，ItemRentingDialog.cs:20-219）
//   - 物主对方窗：`GuestItemRentDialog`（对方费用窗，ItemRentDialog.cs:190-270）
//   - 租客对方窗：`GuestItemRentingDialog`（对方物品窗，ItemRentingDialog.cs:221-330）
//
// C# 角色由 `S.ItemRentalRequest.Renting` 决定（GameScene.cs:10038-10052）：
//   `renting = false` → 本端**物主**（点 RENT 发起的一方）：
//        自有 物品窗 (718,287) + 对方 费用窗 (718,163)
//        物主动作：存入物品（C.DepositRentalItem）/取回、设期限、锁物品、确认
//   `renting = true`  → 本端**租客**（被请求的一方）：
//        自有 费用窗 (718,163) + 对方 物品窗 (718,287)
//        租客动作：设费用（C.ItemRentalFee）、锁费用
//   两端各只有 1 个自有窗 + 1 个对方窗 → 坐标互补（163 / 287），C# 原版亦然（不重叠）。
//
// 控件锚点（四窗同源 `Prguse[238]` 原生 204x109）：
//   关闭 `Prguse2[360..362]` @(180,3)：**仅自有窗有**（C# 对方窗无关闭键）
//   名称标签 (30,8) 150x14；数值标签 (60,42) 150x14
//   费用窗：价格按钮 `Prguse[28]` @(18,46) 32x17 → MirAmountBox → `C.ItemRentalFee`；
//           锁定费用 `Prguse[250..252]` @(22,76) 28x25 → `C.ItemRentalLockFee`
//   物品窗：物品格 @(16,35) 34x32（GridType.Renting / GuestRenting）、
//           锁定物品 `Prguse[250..252]` @(18,76) 28x25（期限 1..30 校验）、
//           设期限 `Prguse3[7..9]` @(46,76) 84x28、确认 `Prguse3[10..12]` @(130,76) 58x28
//   锁定后换 `Prguse[253]` 帧（C# `Lock()`）；对方窗控件全部 `Enabled = false`（纯展示）。
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
use crate::ui::sprite_ui::{shared_cjk_font, ui_image, UiCjkFont, UiFont, UiImageCache};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_item_cell_ui, spawn_label, spawn_panel, CloseButton,
    ImageButton, UiItemCellData, UiItemCellIcon,
};

/// Prguse[238] 面板帧（C# 四窗同源）
const RENT_BG_FRAME: usize = 238;
/// Prguse[28] 价格按钮帧（C# 只设 Index）
const RENTAL_PRICE_FRAME: usize = 28;
/// C# `Lock()` 后的锁定帧
const LOCKED_FRAME: usize = 253;
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

/// 本端在租赁会话中的角色（C# `S.ItemRentalRequest.Renting`）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RentalRole {
    /// `renting = false`：点 RENT 发起的一方，存物/设期/锁物/确认
    #[default]
    Owner,
    /// `renting = true`：被请求的一方，设费/锁费
    Renter,
}

/// 四个租赁窗（C#：每端只显示 1 个自有窗 + 1 个对方窗）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RentalWindow {
    /// 自有费用窗（C# `ItemRentDialog`，租客侧）
    OwnFee,
    /// 自有物品窗（C# `ItemRentingDialog`，物主侧）
    OwnItem,
    /// 对方费用窗（C# `GuestItemRentDialog`，物主侧显示租客费用）
    GuestFee,
    /// 对方物品窗（C# `GuestItemRentingDialog`，租客侧显示物主物品）
    GuestItem,
}

impl RentalWindow {
    /// C# `GameScene.ItemRentalRequest` 的显隐分流
    pub fn shown_for(self, role: RentalRole) -> bool {
        match (self, role) {
            (RentalWindow::OwnFee | RentalWindow::GuestItem, RentalRole::Renter) => true,
            (RentalWindow::OwnItem | RentalWindow::GuestFee, RentalRole::Owner) => true,
            _ => false,
        }
    }

    /// 窗口坐标（自有/对方同源，按类型落在 163 或 287 行）
    pub fn pos(self) -> (f32, f32) {
        match self {
            RentalWindow::OwnFee | RentalWindow::GuestFee => FEE_POS,
            RentalWindow::OwnItem | RentalWindow::GuestItem => ITEM_POS,
        }
    }
}

/// 文本控件归属（四窗共 6 个标签：自有名/自有费用/自有限期 + 对方名/对方费用/对方期限）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RentalTextKind {
    OwnName,
    OwnFee,
    OwnPeriod,
    GuestName,
    GuestFee,
    GuestPeriod,
}

/// 锁定图标归属（C# `_lockButton.Index` 250 ↔ 253）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RentalLockIcon {
    OwnFee,
    OwnItem,
    GuestFee,
    GuestItem,
}

/// 租赁会话状态（服务端事件写入）
#[derive(Resource, Default)]
pub struct ItemRentalState {
    /// 收到过 `S.ItemRentalRequest`（C# 会话建立；驱动脚本用作阶段门）
    pub request_received: bool,
    /// 本端角色（C# `Renting`）
    pub role: RentalRole,
    /// 自有侧玩家名（C# `_nameLabel.Text = GameScene.User.Name`）
    pub name: String,
    /// 对方玩家名（C# `SetGuestName`）
    pub partner_name: String,
    pub has_item: bool,
    pub fee: u32,
    pub period: i32,
    pub can_confirm: bool,
    pub message: String,
    pub confirmed: bool,
    /// 最近存入物品 uid
    pub deposit_uid: Option<u64>,
    /// 自有侧存入物品（本地镜像，用于自有物品窗物品格）
    pub deposit_item: Option<InvItem>,
    /// 自有侧锁定（C# `ItemRentalLock{GoldLocked/ItemLocked}`）
    pub fee_locked: bool,
    pub item_locked: bool,
    /// 对方侧锁定（C# `ItemRentalPartnerLock{GoldLocked/ItemLocked}`）
    pub partner_fee_locked: bool,
    pub partner_item_locked: bool,
    /// 对方侧费用（租客设置，物主屏幕的对方费用窗）
    pub partner_fee: u32,
    /// 对方侧期限（物主设置，租客屏幕的对方物品窗）
    pub partner_period: i32,
    /// 对方侧存入物品（租客屏幕的对方物品窗物品格）
    pub partner_item: Option<InvItem>,
}

impl ItemRentalState {
    /// 新会话开局清空（C# `OpenItemRentalDialog`/`OpenItemRentDialog` 前状态）
    fn begin(&mut self, role: RentalRole, partner_name: String) {
        let name = self.name.clone();
        *self = ItemRentalState {
            request_received: true,
            role,
            name,
            partner_name,
            message: if role == RentalRole::Owner {
                "收到租赁请求（物主）".to_string()
            } else {
                "对方已接受租赁请求（租客）".to_string()
            },
            ..Default::default()
        };
    }
}

/// 关闭（仅自有窗：C# `cancelButton.Click → CancelItemRental()`）
#[derive(Component)]
pub struct ItemRentalClose;

/// 自有费用窗：价格按钮 / 锁定费用
#[derive(Component)]
pub struct ItemRentalPriceBtn;

#[derive(Component)]
pub struct ItemRentalLockFeeBtn;

/// 自有物品窗：物品格（存入/取回）/ 锁定物品 / 设置期限 / 确认
#[derive(Component)]
pub struct RentalItemCell;

#[derive(Component)]
pub struct ItemRentalLockItemBtn;

#[derive(Component)]
pub struct ItemRentalPeriodBtn;

#[derive(Component)]
pub struct ItemRentalConfirmBtn;

/// 对方物品窗：只读物品格（C# `MirGridType.GuestRenting`）
#[derive(Component)]
pub struct RentalGuestItemCell;

/// 对方窗锁定键（纯展示；C# `Enabled = false`，锁定后同样换 253 帧）
#[derive(Component)]
pub struct RentalGuestLockBtn;

/// 数量输入框用途（费用 / 期限）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RentalInput {
    Fee,
    Period,
}

pub struct ItemRentalPlugin;

impl Plugin for ItemRentalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCjkFont>();
        app.init_resource::<ItemRentalState>();
        app.add_systems(
            Update,
            rental_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_item_rental);
        app.add_systems(OnExit(AppState::Game), cleanup_item_rental);
        app.add_systems(
            Update,
            (
                item_rental_ui_system,
                item_rental_lock_icon_system,
                item_rental_action_system,
            )
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

    // 四个窗口的背景都是 Prguse[238]（各自一份句柄，避免共享 ID 造成的清理耦合）
    let mut bg = |libs: &mut GameLibraries, images: &mut Assets<Image>| {
        load_lib_image(libs, images, LibraryName::Prguse, RENT_BG_FRAME)
    };
    let (Some(bg_own_fee), Some(bg_own_item), Some(bg_guest_fee), Some(bg_guest_item)) = (
        bg(&mut libs, &mut images),
        bg(&mut libs, &mut images),
        bg(&mut libs, &mut images),
        bg(&mut libs, &mut images),
    ) else {
        return;
    };

    // ---- 自有费用窗（C# `ItemRentDialog` @(718,163)，租客侧）----
    let own_fee = spawn_panel(
        &mut commands,
        bg_own_fee,
        FEE_POS.0,
        FEE_POS.1,
        RENT_W,
        RENT_H,
        30,
    );
    commands.entity(own_fee).insert((
        DialogRoot(DialogKind::ItemRental),
        RentalWindow::OwnFee,
        Visibility::Hidden,
    ));
    commands.entity(own_fee).with_children(|p| {
        spawn_close(p, &mut libs, &mut images);
        spawn_label(p, &cjk, "", NAME_POS.0, NAME_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::OwnName);
        spawn_label(p, &cjk, "", VALUE_POS.0, VALUE_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::OwnFee);
        // 价格按钮 Prguse[28]（C# 只有 Index，无 hover/pressed 帧）
        spawn_disabled_button(
            p,
            &mut libs,
            &mut images,
            LibraryName::Prguse,
            RENTAL_PRICE_FRAME,
            FEE_PRICE_POS.0,
            FEE_PRICE_POS.1,
            32.0,
            17.0,
        )
        .insert(ItemRentalPriceBtn);
        spawn_lock_button(
            p,
            &mut libs,
            &mut images,
            FEE_LOCK_POS.0,
            FEE_LOCK_POS.1,
            RentalLockIcon::OwnFee,
            ItemRentalLockFeeBtn,
        );
    });

    // ---- 自有物品窗（C# `ItemRentingDialog` @(718,287)，物主侧）----
    let own_item = spawn_panel(
        &mut commands,
        bg_own_item,
        ITEM_POS.0,
        ITEM_POS.1,
        RENT_W,
        RENT_H,
        30,
    );
    commands.entity(own_item).insert((
        DialogRoot(DialogKind::ItemRental),
        RentalWindow::OwnItem,
        Visibility::Hidden,
    ));
    commands.entity(own_item).with_children(|p| {
        spawn_close(p, &mut libs, &mut images);
        spawn_label(p, &cjk, "", NAME_POS.0, NAME_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::OwnName);
        spawn_label(p, &cjk, "", VALUE_POS.0, VALUE_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::OwnPeriod);
        // 物品格（C# MirItemCell GridType.Renting slot 0 @(16,35)）：存放/取回
        spawn_item_cell_ui(
            p,
            &mut images,
            &cjk,
            ITEM_CELL_POS.0,
            ITEM_CELL_POS.1,
            RENTAL_CELL_W,
            RENTAL_CELL_H,
            9,
            0,
        )
        .insert((RentalItemCell, Button));
        spawn_lock_button(
            p,
            &mut libs,
            &mut images,
            ITEM_LOCK_POS.0,
            ITEM_LOCK_POS.1,
            RentalLockIcon::OwnItem,
            ItemRentalLockItemBtn,
        );
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

    // ---- 对方费用窗（C# `GuestItemRentDialog` @(718,163)，物主侧显示租客费用）----
    let guest_fee = spawn_panel(
        &mut commands,
        bg_guest_fee,
        FEE_POS.0,
        FEE_POS.1,
        RENT_W,
        RENT_H,
        30,
    );
    commands.entity(guest_fee).insert((
        DialogRoot(DialogKind::ItemRental),
        RentalWindow::GuestFee,
        Visibility::Hidden,
    ));
    commands.entity(guest_fee).with_children(|p| {
        // C# 无关闭键
        spawn_label(p, &cjk, "", NAME_POS.0, NAME_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::GuestName);
        spawn_label(p, &cjk, "", VALUE_POS.0, VALUE_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::GuestFee);
        // C# `_rentalPriceButton` / `_lockButton` 均 `Enabled = false`：单帧展示
        spawn_disabled_button(
            p,
            &mut libs,
            &mut images,
            LibraryName::Prguse,
            RENTAL_PRICE_FRAME,
            FEE_PRICE_POS.0,
            FEE_PRICE_POS.1,
            32.0,
            17.0,
        );
        spawn_lock_button(
            p,
            &mut libs,
            &mut images,
            FEE_LOCK_POS.0,
            FEE_LOCK_POS.1,
            RentalLockIcon::GuestFee,
            RentalGuestLockBtn,
        );
    });

    // ---- 对方物品窗（C# `GuestItemRentingDialog` @(718,287)，租客侧显示物主物品）----
    let guest_item = spawn_panel(
        &mut commands,
        bg_guest_item,
        ITEM_POS.0,
        ITEM_POS.1,
        RENT_W,
        RENT_H,
        30,
    );
    commands.entity(guest_item).insert((
        DialogRoot(DialogKind::ItemRental),
        RentalWindow::GuestItem,
        Visibility::Hidden,
    ));
    commands.entity(guest_item).with_children(|p| {
        // C# 无关闭键
        spawn_label(p, &cjk, "", NAME_POS.0, NAME_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::GuestName);
        spawn_label(p, &cjk, "", VALUE_POS.0, VALUE_POS.1, 12.0, Color::WHITE, 9)
            .insert(RentalTextKind::GuestPeriod);
        // 对方物品格 GridType.GuestRenting：只读（无 Button）
        spawn_item_cell_ui(
            p,
            &mut images,
            &cjk,
            ITEM_CELL_POS.0,
            ITEM_CELL_POS.1,
            RENTAL_CELL_W,
            RENTAL_CELL_H,
            9,
            0,
        )
        .insert(RentalGuestItemCell);
        spawn_lock_button(
            p,
            &mut libs,
            &mut images,
            ITEM_LOCK_POS.0,
            ITEM_LOCK_POS.1,
            RentalLockIcon::GuestItem,
            RentalGuestLockBtn,
        );
        // C# `_setRentalPeriodButton` / `_confirmButton` 均 `Enabled = false`：单帧展示
        spawn_disabled_button(
            p,
            &mut libs,
            &mut images,
            LibraryName::Prguse3,
            7,
            ITEM_PERIOD_POS.0,
            ITEM_PERIOD_POS.1,
            84.0,
            28.0,
        );
        spawn_disabled_button(
            p,
            &mut libs,
            &mut images,
            LibraryName::Prguse3,
            10,
            ITEM_CONFIRM_POS.0,
            ITEM_CONFIRM_POS.1,
            58.0,
            28.0,
        );
    });
}

/// 自有窗关闭键（`Prguse2[360..362]` @(180,3)）
fn spawn_close(p: &mut ChildSpawnerCommands, libs: &mut GameLibraries, images: &mut Assets<Image>) {
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Prguse2, 360),
        load_lib_image(libs, images, LibraryName::Prguse2, 361),
        load_lib_image(libs, images, LibraryName::Prguse2, 362),
    ) {
        spawn_icon_button(p, n, h, pr, CLOSE_POS.0, CLOSE_POS.1, 24.0, 21.0, 10)
            .insert((ItemRentalClose, CloseButton));
    }
}

/// 锁定键（`Prguse[250..252]`；`Lock()` 后换 253 帧）
fn spawn_lock_button<M: Component>(
    p: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    x: f32,
    y: f32,
    icon: RentalLockIcon,
    marker: M,
) {
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Prguse, 250),
        load_lib_image(libs, images, LibraryName::Prguse, 251),
        load_lib_image(libs, images, LibraryName::Prguse, 252),
    ) {
        spawn_icon_button(p, n, h, pr, x, y, 28.0, 25.0, 10).insert((icon, marker));
    }
}

/// 单帧按钮（C# `Enabled = false` 或无 hover/pressed 帧的控件）
fn spawn_disabled_button<'a>(
    p: &'a mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    lib: LibraryName,
    index: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> EntityCommands<'a> {
    match load_lib_image(libs, images, lib, index) {
        Some(img) => spawn_icon_button(p, img.clone(), img.clone(), img, x, y, w, h, 10),
        None => p.spawn_empty(),
    }
}

/// 显隐（按角色）+ 标签 + 物品格渲染
#[allow(clippy::too_many_arguments)]
fn item_rental_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ItemRentalState>,
    net: Res<NetConnection>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut image_cache: ResMut<UiImageCache>,
    mut inv_click: ResMut<InvClickState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    close: Query<(Entity, &Interaction), With<ItemRentalClose>>,
    cell: Query<(Entity, &Interaction), With<RentalItemCell>>,
    mut widgets: Query<(&RentalWindow, &mut Visibility)>,
    mut texts: Query<(&RentalTextKind, &mut Text)>,
    mut cells: Query<(&RentalItemCell, &mut UiItemCellData), Without<UiItemCellIcon>>,
    mut guest_cells: Query<
        (&RentalGuestItemCell, &mut UiItemCellData),
        (Without<UiItemCellIcon>, Without<RentalItemCell>),
    >,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    let open = mgr.is_open(DialogKind::ItemRental);
    // C# `GameScene.ItemRentalRequest`：每端只显示自有窗 + 对方窗各一个
    for (window, mut vis) in &mut widgets {
        *vis = if open && window.shown_for(state.role) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
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
    // 关闭（仅自有窗）：C# `cancelButton.Click → CancelItemRental()`
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::CancelItemRental);
            mgr.close(DialogKind::ItemRental);
        }
    }
    // 自有物品格（仅物主）：格空 → 存入背包选中物；已有物品 → 取回背包空格
    // （C# `MirItemCell` GridType.Renting：`C.DepositRentalItem` / `C.RetrieveRentalItem`）
    for (e, inter) in &cell {
        if !edge(e, inter, &mut prev_inter) || state.role != RentalRole::Owner {
            continue;
        }
        let items = inv_q
            .single()
            .map(|inv| inv.items.as_slice())
            .unwrap_or(&[]);
        if state.deposit_item.is_some() {
            match items.iter().position(|s| s.is_none()) {
                Some(to) => {
                    net.send_packet(&mir2_shared::packets::client::item::RetrieveRentalItem {
                        from: 0,
                        to: to as i32,
                    });
                    state.deposit_item = None;
                    state.deposit_uid = None;
                    state.message = "已取回租赁物品".to_string();
                }
                None => state.message = "背包已满，无法取回".to_string(),
            }
            continue;
        }
        let Some(sel) = inv_click.selected else {
            continue;
        };
        let Some(item) = items.get(sel).and_then(|s| s.as_ref()) else {
            continue;
        };
        net.send_packet(&mir2_shared::packets::client::item::DepositRentalItem {
            from: sel as i32,
            to: 0,
        });
        state.deposit_uid = Some(item.unique_id);
        state.deposit_item = Some(item.clone());
        state.has_item = true;
        state.message = "已存入租赁物品".to_string();
        inv_click.selected = None;
    }
    // C# 各标签文案（`RefreshInterface()`）
    for (kind, mut text) in &mut texts {
        let new = match kind {
            RentalTextKind::OwnName => {
                if state.name.is_empty() {
                    "租赁".to_string()
                } else {
                    state.name.clone()
                }
            }
            RentalTextKind::OwnFee => format!("费用: {} 金币", state.fee),
            RentalTextKind::OwnPeriod => format!("租赁期限: {} 天", state.period.max(0)),
            RentalTextKind::GuestName => {
                if state.partner_name.is_empty() {
                    "对方".to_string()
                } else {
                    state.partner_name.clone()
                }
            }
            RentalTextKind::GuestFee => format!("费用: {} 金币", state.partner_fee),
            RentalTextKind::GuestPeriod => format!("租赁期限: {} 天", state.partner_period.max(0)),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    for (_cell, mut data) in &mut cells {
        apply_cell(
            &mut data,
            state.deposit_item.as_ref(),
            &mut libs,
            &mut images,
            &mut image_cache,
        );
    }
    for (_cell, mut data) in &mut guest_cells {
        apply_cell(
            &mut data,
            state.partner_item.as_ref(),
            &mut libs,
            &mut images,
            &mut image_cache,
        );
    }
}

/// 物品格渲染（图标 + 数量；租赁格不画耐久条）
fn apply_cell(
    data: &mut UiItemCellData,
    item: Option<&InvItem>,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
) {
    match item {
        Some(item) => {
            data.icon = if item.image > 0 {
                ui_image(libs, images, cache, LibraryName::Items, item.image as usize)
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

/// 锁定帧切换（C# `Lock()`：`_lockButton.Index = 253`）
fn item_rental_lock_icon_system(
    state: Res<ItemRentalState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut q: Query<(Entity, &RentalLockIcon, &mut ImageButton)>,
    mut prev: Local<std::collections::HashMap<Entity, bool>>,
) {
    for (e, icon, mut btn) in &mut q {
        let locked = match icon {
            RentalLockIcon::OwnFee => state.fee_locked,
            RentalLockIcon::OwnItem => state.item_locked,
            RentalLockIcon::GuestFee => state.partner_fee_locked,
            RentalLockIcon::GuestItem => state.partner_item_locked,
        };
        if prev.insert(e, locked) == Some(locked) {
            continue;
        }
        let (normal, hover, pressed) = if locked {
            let Some(frame) = ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Prguse,
                LOCKED_FRAME,
            ) else {
                continue;
            };
            (frame.clone(), frame.clone(), frame)
        } else {
            // 对方窗控件在 C# 是 `Enabled = false`：锁形只有单帧，不做 hover/pressed
            let single = matches!(icon, RentalLockIcon::GuestFee | RentalLockIcon::GuestItem);
            let n = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 250);
            let h = if single {
                n.clone()
            } else {
                ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 251)
            };
            let p = if single {
                n.clone()
            } else {
                ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 252)
            };
            let (Some(n), Some(h), Some(p)) = (n, h, p) else {
                continue;
            };
            (n, h, p)
        };
        btn.normal = normal;
        btn.hover = hover;
        btn.pressed = pressed;
    }
}

/// 交互：价格（费用）/ 期限 / 双向锁定 / 确认（按角色分流）
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
                // C# `ItemRentDialog._rentalPriceButton`（租客侧）
                if value > 0 && state.role == RentalRole::Renter {
                    net.send_packet(&mir2_shared::packets::client::item::ItemRentalFee {
                        amount: value,
                    });
                    state.fee = value;
                    state.message = format!("设置租赁费用 {}", value);
                }
            }
            Some(RentalInput::Period) => {
                // C# `ItemRentingDialog.InputRentalPeroid()`（物主侧）
                if state.role != RentalRole::Owner {
                    // 对方物品窗没有设限期按钮（C# `Enabled = false`）
                } else if rental_period_valid(value) {
                    net.send_packet(&mir2_shared::packets::client::item::ItemRentalPeriod {
                        days: value,
                    });
                    state.period = value as i32;
                    state.message = format!("设置租赁期限 {} 天", value);
                } else {
                    state.message =
                        format!("期限需在 {}-{} 之间", RENTAL_PERIOD_MIN, RENTAL_PERIOD_MAX);
                }
            }
            None => {}
        }
        *pending = None;
    }
    // 价格按钮 → 数量输入（C# `MirAmountBox(RentalFee, 116, GameScene.Gold)`，租客侧）
    for (e, inter) in &price_btn {
        if !edge(e, inter, &mut prev_inter) || state.role != RentalRole::Renter {
            continue;
        }
        let max = gold_q.single().map(|g| g.0).unwrap_or(0);
        if max == 0 {
            state.message = "金币不足".to_string();
            continue;
        }
        amount.ask("租赁费用", max);
        *pending = Some(RentalInput::Fee);
    }
    // 锁定费用（C# `ItemRentalLockFee`，租客侧；锁定后本端禁用）
    for (e, inter) in &lock_fee_btn {
        if !edge(e, inter, &mut prev_inter) || state.role != RentalRole::Renter || state.fee_locked
        {
            continue;
        }
        net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockFee);
        state.message = "已锁定费用".to_string();
    }
    // 设置期限 → 数量输入（C# `InputRentalPeroid()` → MirInputBox，物主侧）
    for (e, inter) in &period_btn {
        if !edge(e, inter, &mut prev_inter) || state.role != RentalRole::Owner || state.item_locked
        {
            continue;
        }
        amount.ask("租赁期限(1-30)", RENTAL_PERIOD_MAX);
        if state.period > 0 {
            amount.value = state.period.to_string();
            amount.fresh = false;
        }
        *pending = Some(RentalInput::Period);
    }
    // 锁定物品（C# `_lockButton.Click`：期限 1..30 才发 `ItemRentalLockItem`，物主侧）
    for (e, inter) in &lock_item_btn {
        if !edge(e, inter, &mut prev_inter) || state.role != RentalRole::Owner {
            continue;
        }
        if !rental_period_valid(state.period.max(0) as u32) {
            state.message = format!(
                "请先设置 {}-{} 天期限",
                RENTAL_PERIOD_MIN, RENTAL_PERIOD_MAX
            );
            continue;
        }
        net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockItem);
        state.message = "已锁定物品".to_string();
    }
    // 确认（C# `_confirmButton`：仅物主侧；双方锁定后服务端下发 `can_confirm`）
    for (e, inter) in &confirm_btn {
        if !edge(e, inter, &mut prev_inter) || state.role != RentalRole::Owner {
            continue;
        }
        if !state.can_confirm {
            state.message = "双方锁定后才能确认".to_string();
            continue;
        }
        net.send_packet(&mir2_shared::packets::client::item::ConfirmItemRental);
        state.message = "已发送确认".to_string();
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
            ServerEvent::RentalRequest { renting, name } => {
                // C# `GameScene.ItemRentalRequest`：Renting=false → 本端物主，true → 本端租客
                let role = if *renting {
                    RentalRole::Renter
                } else {
                    RentalRole::Owner
                };
                rental.begin(role, name.clone());
                // C# `OpenItemRentalDialog()/OpenItemRentDialog()`：开背包 + 双窗；浏览窗关闭
                if !mgr.is_open(DialogKind::Inventory) {
                    mgr.open(DialogKind::Inventory);
                }
                mgr.close(DialogKind::ItemRentalBrowse);
                mgr.open(DialogKind::ItemRental);
            }
            ServerEvent::RentalItemUpdate { item, fee, period } => {
                rental.has_item = item.is_some();
                rental.partner_item = item.clone();
                // Rust 扩展字段（C# `S.UpdateRentalItem` 只带物品）：费用归租客、期限归物主
                if *fee > 0 {
                    rental.fee = *fee;
                }
                if *period > 0 {
                    rental.period = *period;
                }
                rental.message = format!(
                    "租赁更新: 物品={}",
                    if rental.has_item { "有" } else { "无" }
                );
            }
            ServerEvent::RentalFee { fee } => {
                // C# `GuestItemRentDialog.SetGuestFee`（物主屏幕的对方费用窗）
                rental.partner_fee = *fee;
                rental.message = format!("对方设置费用: {}", rental.partner_fee);
            }
            ServerEvent::RentalPeriod { period } => {
                // C# `GuestItemRentingDialog.GuestRentalPeriod`（租客屏幕的对方物品窗）
                rental.partner_period = *period;
                rental.message = format!("对方设置期限: {} 天", rental.partner_period);
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
                if *success {
                    rental.has_item = false;
                    rental.deposit_item = None;
                }
            }
            ServerEvent::RentalLocked {
                gold_locked,
                item_locked,
            } => {
                // C# `ItemRentalLock`：分别点亮自有费用窗 / 自有物品窗的锁定帧
                if *gold_locked {
                    rental.fee_locked = true;
                    rental.message = "费用已锁定".to_string();
                }
                if *item_locked {
                    rental.item_locked = true;
                    rental.message = "物品已锁定".to_string();
                }
            }
            ServerEvent::RentalPartnerLocked {
                gold_locked,
                item_locked,
            } => {
                // C# `ItemRentalPartnerLock`：对方费用窗 / 对方物品窗的锁定帧
                if *gold_locked {
                    rental.partner_fee_locked = true;
                    rental.message = "对方已锁定费用".to_string();
                }
                if *item_locked {
                    rental.partner_item_locked = true;
                    rental.message = "对方已锁定物品".to_string();
                }
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
                // C# `ConfirmItemRental` → `ItemRentingDialog.Reset()` + `ItemRentDialog.Reset()`
                mgr.close(DialogKind::ItemRental);
            }
            ServerEvent::RentalCancelled => {
                let name = rental.name.clone();
                *rental = ItemRentalState {
                    name,
                    message: "租赁已取消".to_string(),
                    ..Default::default()
                };
                mgr.close(DialogKind::ItemRental);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2720：四窗面板/控件锚点对齐 C#
    /// （ItemRentDialog.cs:16-270 / ItemRentingDialog.cs:20-330）
    #[test]
    fn item_rental_layout_matches_csharp_anchors() {
        assert_eq!((RENT_W, RENT_H), (204.0, 109.0)); // Prguse[238]
        assert_eq!(RENT_BG_FRAME, 238);
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
        assert_eq!(LOCKED_FRAME, 253); // C# `Lock()` 换帧
        assert_eq!(RENTAL_PRICE_FRAME, 28); // C# 价格按钮 Prguse[28]
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
        // 自有窗/对方窗同列（C# Guest 窗坐标与本体完全相同）
        assert_eq!(RentalWindow::OwnFee.pos(), RentalWindow::GuestFee.pos());
        assert_eq!(RentalWindow::OwnItem.pos(), RentalWindow::GuestItem.pos());
        assert_ne!(
            RentalWindow::OwnFee.pos().1,
            RentalWindow::OwnItem.pos().1,
            "费用窗(163)/物品窗(287) 两行互补，同屏不重叠"
        );
    }

    /// #2720：C# 每端「1 自有 + 1 对方」显隐分流（`GameScene.ItemRentalRequest`）
    #[test]
    fn rental_window_role_visibility() {
        const ALL: [RentalWindow; 4] = [
            RentalWindow::OwnFee,
            RentalWindow::OwnItem,
            RentalWindow::GuestFee,
            RentalWindow::GuestItem,
        ];
        let owner: Vec<RentalWindow> = ALL
            .into_iter()
            .filter(|w| w.shown_for(RentalRole::Owner))
            .collect();
        assert_eq!(owner, vec![RentalWindow::OwnItem, RentalWindow::GuestFee]);

        let renter: Vec<RentalWindow> = ALL
            .into_iter()
            .filter(|w| w.shown_for(RentalRole::Renter))
            .collect();
        assert_eq!(renter, vec![RentalWindow::OwnFee, RentalWindow::GuestItem]);

        // 同一角色两个窗口纵向错开（C# 163 / 287）
        for role in [RentalRole::Owner, RentalRole::Renter] {
            let shown: Vec<(f32, f32)> = ALL
                .into_iter()
                .filter(|w| w.shown_for(role))
                .map(|w| w.pos())
                .collect();
            assert_eq!(shown.len(), 2);
            assert_ne!(shown[0].1, shown[1].1);
        }
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
