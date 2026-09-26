// ============================================================================
// 仓库对话框（M18）
// 布局参考：C# NPCDialogs.cs StorageDialog（10 列 x 8 行，cell 36x32 间隔 1）
//   - 背景 Prguse[586]（实测 388x346），原点 (0,0)（C# StorageDialog
//     Location = new Point(0, 0)，NPCDialogs.cs:2807；格子/按钮偏移同 C#）
//   - 关闭按钮 Prguse2[360-362]
//   - 交互（原版 C# MirItemCell 拖放语义，选中+点击）：
//       选中背包物品 → 点仓库格 → C.StoreItem{From=背包格, To=仓库格}
//       选中仓库物品 → 点背包格 → C.TakeBackItem{From=仓库格, To=背包格}
//       点已选中格取消选中；点空格清空选择
// 网络：UserStorage（服务端仓库内容）→ 显示；操作后服务端发完整 UserStorage + UserInformation 刷新
// ============================================================================

use bevy::prelude::*;

use crate::actor::LocalPlayer;
use crate::game::dialogs::inventory::{
    inv_slot_at, item_use_sound_id, use_item_core, InvClickState, InvDropConfirm, InvItem,
    InvLockReason, InvLockedSlots, InvUiState, ItemUseFeedback, LockGrid, UseItemCtx, UseOutcome,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot, NotDraggable};
use crate::game::player_state::{Inventory, Loadout, StatusFlags};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_item_cell_ui_root,
    spawn_label, spawn_panel, CloseButton, ImageButton, UiItemCell, UiItemCellData, UiItemCellIcon,
};

/// #2892 批B：面板精灵与 C# 原生尺寸（C# `StorageDialog.Index = 586; Library = Libraries.Prguse`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 586);
pub const PANEL_SIZE: (f32, f32) = (388.0, 346.0);

// ---- C# `StorageDialog` 子控件（`NPCDialogs.cs:2815-2935`）----
/// 标题 `TitleLabel` = `Title[0]` @(18,8)
pub const TITLE_SPRITE: (LibraryName, usize) = (LibraryName::Title, 0);
pub const TITLE_POS: (f32, f32) = (18.0, 8.0);
/// 页码钮 `Storage1Button`/`Storage2Button`：`[正常帧, 另一页激活时的帧]`
/// （C# `RefreshStorage1` 置 743/746，`RefreshStorage2` 置 744/745）
pub const PAGE1_TAB: [(LibraryName, usize); 2] =
    [(LibraryName::Title, 743), (LibraryName::Title, 744)];
pub const PAGE2_TAB: [(LibraryName, usize); 2] =
    [(LibraryName::Title, 746), (LibraryName::Title, 745)];
pub const PAGE1_TAB_POS: (f32, f32) = (8.0, 36.0);
pub const PAGE2_TAB_POS: (f32, f32) = (80.0, 36.0);
/// 租用扩容钮 `RentButton` = `Title[483/484/485]` @(283,33)（仅第 2 页可见）
pub const RENT_SPRITES: [(LibraryName, usize); 3] = [
    (LibraryName::Title, 483),
    (LibraryName::Title, 484),
    (LibraryName::Title, 485),
];
pub const RENT_BTN_POS: (f32, f32) = (283.0, 33.0);
/// 密码钮 `ProtectButton` = `Title[113/114/115]` @(328,33)
pub const PROTECT_SPRITES: [(LibraryName, usize); 3] = [
    (LibraryName::Title, 113),
    (LibraryName::Title, 114),
    (LibraryName::Title, 115),
];
pub const PROTECT_BTN_POS: (f32, f32) = (328.0, 33.0);
/// 关闭钮 `CloseButton` = `Prguse2[360/361/362]` @(363,3)
pub const CLOSE_SPRITES: [(LibraryName, usize); 3] = [
    (LibraryName::Prguse2, 360),
    (LibraryName::Prguse2, 361),
    (LibraryName::Prguse2, 362),
];
pub const CLOSE_POS: (f32, f32) = (363.0, 3.0);
/// 未扩容遮罩 `LockedPage` = `Prguse[2443]` @(8,59)（仅第 2 页且未扩容时可见）
pub const LOCKED_PAGE_SPRITE: (LibraryName, usize) = (LibraryName::Prguse, 2443);
pub const LOCKED_PAGE_POS: (f32, f32) = (8.0, 59.0);
/// 两行提示（C# `RentalLabel` / `StoragePasswordLabel`，`AutoSize` 故只给左上角）
pub const RENTAL_LABEL_POS: (f32, f32) = (40.0, 322.0);
pub const PASSWORD_LABEL_POS: (f32, f32) = (40.0, 304.0);

/// 第 1 页格数（C# `Globals.StorageGridSize` 基础 80 = 10×8）
pub const PAGE_CELLS: usize = COLS * ROWS;
/// 扩容后总格数（C# `Grid = new MirItemCell[10 * 16]`）
pub const MAX_CELLS: usize = PAGE_CELLS * 2;

/// 仓库页码（C# `Storage1Button`/`Storage2Button` 切页）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum StoragePage {
    #[default]
    One,
    Two,
}

/// 仓库数据（网络 UserStorage 写入）
#[derive(Resource, Default)]
pub struct StorageState {
    /// 80 格仓库（服务端 STORAGE_SIZE）
    pub items: Vec<Option<InvItem>>,
    pub visible: bool,
    /// 当前选中仓库格（原版 C# GameScene.SelectedCell）
    pub selected: Option<usize>,
    /// C# `UserInformation.RequireStoragePassword`（服务端 `Settings.RequireStoragePassword`，
    /// C# 默认 true；本端 Rust 服务端登录时置 true，见 `session.rs:798`）
    pub require_password: bool,
    /// C# `UserInformation.HasStoragePassword`
    pub has_password: bool,
    /// C# `UserInformation.StoragePasswordLastSet`（`DateTime` 秒；0 = 未设过）
    pub password_last_set: i64,
    /// C# `StorageDialog._storageUnlocked`（`Hide()` 里复位，见 `NPCDialogs.cs:2997-3001`）
    pub unlocked: bool,
    /// 「改密确认框」是否打开（C# `ManageStoragePassword` 里的
    /// `MirMessageBox(prompt, MirMessageBoxButtons.OKCancel)`，`NPCDialogs.cs:3104`）
    pub change_confirm: bool,
    /// 解锁提示是否正开着（`auto/inventory.rs` 的 `--storage-unlock-test` 读它判「解锁框出现」）
    pub unlock_prompt_open: bool,
    /// 最近一次解锁/密码操作的错误/结果文案（同上，供自动化判定「错误密码已提示」）
    pub pwd_last_error: String,
    /// 流程要求关闭仓库窗（C# `Hide()`；由 `storage_password_cancel` 置位、驱动系统落地，
    /// 因为取消回调发生在 `input_box` 系统里、拿不到 `DialogManager`）
    pub close_requested: bool,
    /// C# `StorageDialog._forcingPasswordSetup`（本端目前只在「取消置 false」与
    /// 「不一致时是否重来」两处读它；强制设密码闸门本身是下一轮的事）
    pub forcing_setup: bool,
    /// 当前页（C# `RefreshStorage1`/`RefreshStorage2`）
    pub page: StoragePage,
    /// 是否处于扩容状态（C# `UserInformation.HasExpandedStorage`；第 2 页放行条件）
    pub has_expanded_storage: bool,
    /// 扩容到期时间（C# `ExpandedStorageExpiryTime`，仅用于提示文本）
    pub expiry_time: i64,
    /// 租用扩容确认框是否打开（C# `RentButton.Click` → `MirMessageBox`）
    pub rent_confirm: bool,
    /// P3-3（#782）：本地物品名表——线包里的 `UserItem` **不带** `ItemInfo`
    /// （见 `crate::game::item_names` 的原版依据），仓库格名字靠这张表按索引解析；
    /// `UserInformation`（背包/装备名）与 `NewItemInfo`（按需请求的回包）都往这里写。
    pub item_names: std::collections::HashMap<i32, String>,
    /// P3-3：已发过 `RequestItemInfo` 的索引（按索引去重，对齐原版
    /// `GameScene.RequestedItemInfo` 这个 `HashSet`）
    pub requested_item_info: std::collections::HashSet<i32>,
}

impl StorageState {
    /// 按服务端 ResizeStorage 调整格数（C# `Array.Resize`：截断/补空；上限 = 扩容后 160 格）
    pub fn resize(&mut self, size: usize) {
        let size = size.min(MAX_CELLS);
        if size < self.items.len() {
            self.items.truncate(size);
        } else {
            self.items.resize(size, None);
        }
    }

    /// 当前页的格区间 `[start, end)`（C# `RefreshStorage1`/`RefreshStorage2` 的 `Visible` 规则）
    /// - 第 1 页：`0..min(len, 80)`（恒显示）
    /// - 第 2 页：`80..len`，但**未扩容时全隐藏**（C# `grid.ItemSlot < StorageGridSize || !HasExpandedStorage`）
    pub fn page_range(&self) -> (usize, usize) {
        match self.page {
            StoragePage::One => (0, self.items.len().min(PAGE_CELLS)),
            StoragePage::Two => {
                if self.has_expanded_storage {
                    (
                        PAGE_CELLS.min(self.items.len()),
                        self.items.len().min(MAX_CELLS),
                    )
                } else {
                    (PAGE_CELLS, PAGE_CELLS)
                }
            }
        }
    }

    /// 第 2 页是否可用（未扩容 → 显示 `LockedPage` 遮罩 + 租用钮）
    pub fn page2_available(&self) -> bool {
        self.has_expanded_storage && self.items.len() > PAGE_CELLS
    }

    /// 与 C# `GameScene.SelectedCell` 一致的「选中格」是否落在当前页
    pub fn selected_on_page(&self) -> bool {
        let (start, end) = self.page_range();
        self.selected.is_some_and(|s| s >= start && s < end)
    }

    /// P3-3（#782）：仓库格的显示名——`线包自带名 → 本地物品名表 → 兜底 #id`。
    ///
    /// 与商城格走 `crate::game::item_names::resolve_item_name` **同一个降级链**
    /// （#782 的验收判据）。
    pub fn display_name(&self, item: &InvItem) -> String {
        crate::game::item_names::resolve_item_name(&item.name, &self.item_names, item.item_index).0
    }

    /// P3-3：逐格把线包名解析成显示名（占位自愈——表里一旦到货就被纠正），
    /// 并把「表里也没有」的索引并入去重集合
    /// `requested_item_info`；返回**本次新增**的请求索引（调用方据此发
    /// `RequestItemInfo`——原版 `GameScene.RequestItemInfo` 的 `index <= 0 ||
    /// HasItemInfo || !RequestedItemInfo.Add(index)` 三条守卫等价于此）。
    ///
    /// 逻辑与网络/UI 解耦，门禁与阳性对照直接钉在它上面（无需 `NetConnection`）。
    pub fn resolve_wire_item_names(&mut self) -> Vec<i32> {
        // 表先借出，避免 `self.items` 可变借与 `self.item_names` 不可变借冲突
        let table = std::mem::take(&mut self.item_names);
        let requested = &mut self.requested_item_info;
        let mut wanted = Vec::new();
        for slot in self.items.iter_mut() {
            let Some(item) = slot.as_mut() else { continue };
            let (name, need) =
                crate::game::item_names::resolve_item_name(&item.name, &table, item.item_index);
            item.name = name;
            // `index <= 0` 不发请求（原版同名守卫）
            if need && item.item_index > 0 && requested.insert(item.item_index) {
                wanted.push(item.item_index);
            }
        }
        self.item_names = table;
        wanted
    }
}

/// 窗口原点 (0,0)：C# StorageDialog 显式 `Location = new Point(0, 0)`（NPCDialogs.cs:2812）。
/// 旧值 (600,60) 是移植期自定右置，与 C# 左上角原点不符。
pub const DIALOG_X: f32 = 0.0;
pub const DIALOG_Y: f32 = 0.0;
/// 仓库宽（Prguse[586] 实测 388x346）。C# Show 时背包推到 (仓宽+5, 仓Y)=(393,0) 并排
/// （NPCDialogs.cs:2967/2990）——避免仓库完全罩住背包
const STORAGE_W: f32 = 388.0;
const COLS: usize = 10;
const ROWS: usize = 8;
const CELL_W: f32 = 36.0;
const CELL_H: f32 = 32.0;
/// 面板/格子 GlobalZIndex：格子是根节点（非面板子实体），bevy 0.19 根节点按
/// GlobalZIndex 升序绘制——格子 z 必须高于面板 z，否则被面板背景盖住
/// （批38-40 评审 P0；密码/解锁覆盖层 45/46 之上不可盖）
pub const STORAGE_PANEL_Z: i32 = 30;
pub const STORAGE_CELL_Z: i32 = 31;

#[derive(Component)]
pub struct StorageWidget;

#[derive(Component)]
pub struct StorageClose;

/// 仓库密码按钮（C# `StorageDialog.ProtectButton`）
#[derive(Component)]
pub struct StoragePwdBtn;

/// 第 1 / 第 2 页页码钮（C# `Storage1Button` / `Storage2Button`）
#[derive(Component)]
pub struct StoragePage1Tab;
#[derive(Component)]
pub struct StoragePage2Tab;

/// 租用扩容钮（C# `StorageDialog.RentButton`）
#[derive(Component)]
pub struct StorageRentBtn;

/// 未扩容遮罩（C# `StorageDialog.LockedPage`）
#[derive(Component)]
pub struct StorageLockedPage;

/// 两行提示（C# `RentalLabel` / `StoragePasswordLabel`）
#[derive(Component)]
pub struct StorageRentalLabel;
#[derive(Component)]
pub struct StoragePasswordLabel;

/// 租用扩容确认框（C# `RentButton.Click` 里的 `MirMessageBox`）
#[derive(Component)]
pub struct StorageRentConfirm;
#[derive(Component)]
pub struct StorageRentConfirmText;
#[derive(Component)]
pub struct StorageRentConfirmOk;
#[derive(Component)]
pub struct StorageRentConfirmCancel;

/// 改密确认框（C# `ManageStoragePassword` 的 `MirMessageBox(prompt, OKCancel)`，含「上次设置」行）
#[derive(Component)]
pub struct StoragePwdChangeConfirm;
#[derive(Component)]
pub struct StoragePwdChangeConfirmText;
#[derive(Component)]
pub struct StoragePwdChangeOk;
#[derive(Component)]
pub struct StoragePwdChangeCancel;

/// 仓库格子索引（0..79）
#[derive(Component, Clone, Copy)]
pub struct StorageSlot(pub usize);

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StorageState>();
        app.init_resource::<StoragePwdFlow>();
        app.add_systems(OnEnter(AppState::Game), spawn_storage_dialog);
        app.add_systems(OnExit(AppState::Game), cleanup_storage);
        app.add_systems(
            Update,
            storage_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(
            Update,
            (
                storage_grid_sync_system,
                storage_ui_system,
                storage_page_system,
                storage_locked_icon_system,
                storage_action_system,
                storage_tooltip_system,
                storage_pwd_flow_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_storage(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_storage_dialog(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 背景 Prguse[586]（C# StorageDialog.Index=586，实测 388x346 @ (0,0)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 586) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        DIALOG_X,
        DIALOG_Y,
        STORAGE_W,
        PANEL_SIZE.1,
        STORAGE_PANEL_Z,
    );
    commands
        .entity(panel)
        // #2825 单元①：C# `StorageDialog` 未设 `Movable`（`NPCDialogs.cs:2798`）→
        // 默认 false（`MirControl.cs:372`）→ 本端不可拖动（三个根面板 + 格子都要排除）
        .insert((DialogRoot(DialogKind::Storage), StorageWidget, NotDraggable));

    commands.entity(panel).with_children(|p| {
        // 标题 `Title[0]` @(18,8)（C# `TitleLabel`，此前是自造文字「仓库」）
        if let Some(h) = load_lib_image(&mut libs, &mut images, TITLE_SPRITE.0, TITLE_SPRITE.1) {
            spawn_image(p, h, TITLE_POS.0, TITLE_POS.1, 71.0, 15.0, 9);
        }
        // 页码钮 Storage1Button/Storage2Button（Title[743/744] @(8,36)、Title[746/745] @(80,36)）
        // 帧由 `storage_ui_system` 按当前页切换（C# `RefreshStorage1/2` 改 `Index`/`HoverIndex`）
        if let Some(n) = load_lib_image(&mut libs, &mut images, PAGE1_TAB[0].0, PAGE1_TAB[0].1) {
            spawn_icon_button(
                p,
                n.clone(),
                n.clone(),
                n,
                PAGE1_TAB_POS.0,
                PAGE1_TAB_POS.1,
                72.0,
                23.0,
                10,
            )
            .insert(StoragePage1Tab);
        }
        if let Some(n) = load_lib_image(&mut libs, &mut images, PAGE2_TAB[0].0, PAGE2_TAB[0].1) {
            spawn_icon_button(
                p,
                n.clone(),
                n.clone(),
                n,
                PAGE2_TAB_POS.0,
                PAGE2_TAB_POS.1,
                72.0,
                23.0,
                10,
            )
            .insert(StoragePage2Tab);
        }
        // 未扩容遮罩 `Prguse[2443]` @(8,59)（默认隐藏，第 2 页未扩容时显示）
        if let Some(h) = load_lib_image(
            &mut libs,
            &mut images,
            LOCKED_PAGE_SPRITE.0,
            LOCKED_PAGE_SPRITE.1,
        ) {
            spawn_image(p, h, LOCKED_PAGE_POS.0, LOCKED_PAGE_POS.1, 372.0, 265.0, 9)
                .insert((StorageLockedPage, Visibility::Hidden));
        }
        // 租用扩容钮 `Title[483/484/485]` @(283,33)（仅第 2 页可见）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, RENT_SPRITES[0].0, RENT_SPRITES[0].1),
            load_lib_image(&mut libs, &mut images, RENT_SPRITES[1].0, RENT_SPRITES[1].1),
            load_lib_image(&mut libs, &mut images, RENT_SPRITES[2].0, RENT_SPRITES[2].1),
        ) {
            spawn_icon_button(p, n, h, pr, RENT_BTN_POS.0, RENT_BTN_POS.1, 48.0, 25.0, 10)
                .insert((StorageRentBtn, Visibility::Hidden));
        }
        // 密码钮 `Title[113/114/115]` @(328,33)（C# `ProtectButton`；此前本端自造「仓库密码」钮 @(18,330)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(
                &mut libs,
                &mut images,
                PROTECT_SPRITES[0].0,
                PROTECT_SPRITES[0].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                PROTECT_SPRITES[1].0,
                PROTECT_SPRITES[1].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                PROTECT_SPRITES[2].0,
                PROTECT_SPRITES[2].1,
            ),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                PROTECT_BTN_POS.0,
                PROTECT_BTN_POS.1,
                48.0,
                25.0,
                10,
            )
            .insert(StoragePwdBtn);
        }
        // 关闭钮 `Prguse2[360/361/362]` @(363,3) 24x21（此前按 20x20 自造尺寸）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(
                &mut libs,
                &mut images,
                CLOSE_SPRITES[0].0,
                CLOSE_SPRITES[0].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                CLOSE_SPRITES[1].0,
                CLOSE_SPRITES[1].1,
            ),
            load_lib_image(
                &mut libs,
                &mut images,
                CLOSE_SPRITES[2].0,
                CLOSE_SPRITES[2].1,
            ),
        ) {
            // StorageClose 判定查询带 `With<StorageWidget>`——必须同时挂，
            // 否则 ui_system 的关闭分支永不命中（交互 sweep closed=NO 根因）
            spawn_icon_button(p, n, h, pr, CLOSE_POS.0, CLOSE_POS.1, 24.0, 21.0, 10).insert((
                StorageWidget,
                StorageClose,
                CloseButton,
            ));
        }
        // 两行提示（C# `RentalLabel` @(40,322) / `StoragePasswordLabel` @(40,304)）
        spawn_label(
            p,
            &cjk,
            "",
            RENTAL_LABEL_POS.0,
            RENTAL_LABEL_POS.1,
            12.0,
            Color::WHITE,
            11,
        )
        .insert((StorageRentalLabel, Visibility::Hidden));
        spawn_label(
            p,
            &cjk,
            "",
            PASSWORD_LABEL_POS.0,
            PASSWORD_LABEL_POS.1,
            12.0,
            Color::WHITE,
            11,
        )
        .insert((StoragePasswordLabel, Visibility::Hidden));
    });

    // #3258：密码/解锁不再是本端自造的「三钮面板」——按 C# 金标准改成
    //   ① `MirInputBox` 提示序列（`input_box.rs`，OK `Title[200..202]` / Cancel `Title[203..205]`）
    //   ② 改密前的一次 `MirMessageBox(prompt, OKCancel)` 确认（下面这个框）
    // 依据：`NPCDialogs.cs:3085-3239`（ManageStoragePassword / BeginSetStoragePassword /
    // BeginChangeStoragePassword / PromptStorageUnlock / PromptStoragePassword）。
    // 原来的三钮面板用 `Title[206]`(YES)/`Title[210]`(NO) 当通用按钮，又在上面叠中文标签
    // ⇒ 艺术图烘的英文词与本端画的中文叠字（owner 反馈项）。整块删除。
    if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let confirm = spawn_panel(&mut commands, bg, 284.0, 289.0, 456.0, 190.0, 48);
        commands.entity(confirm).insert((
            StoragePwdChangeConfirm,
            DialogRoot(DialogKind::Storage),
            crate::game::dialogs::AlwaysVisible,
            NotDraggable,
        ));
        commands.entity(confirm).with_children(|p| {
            spawn_label(p, &cjk, "", 35.0, 35.0, 12.0, Color::WHITE, 9)
                .insert(StoragePwdChangeConfirmText);
            // `MirMessageBox(OKCancel)`：OK `Title[200..202]` @(260,157)、Cancel `Title[203..205]` @(360,157)
            // （`MirMessageBox.cs:54-74`）
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 200),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 201),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 202),
            ) {
                spawn_icon_button(p, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(StoragePwdChangeOk);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 203),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 204),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 205),
            ) {
                spawn_icon_button(p, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(StoragePwdChangeCancel);
            }
        });
    }

    // 格子底板不在此预生成：#281 由 storage_grid_sync_system 动态生成

    // 租用扩容确认框（C# `RentButton.Click` → `MirMessageBox(ExtraStorage | ExtendYourRentalPeriod,
    // OKCancel)`；`MirMessageBox` 面板 = `Prguse[360]` 456x190 居中 @(284,289)，
    // Yes `Title[206..208]` @(260,157) / No `Title[210..212]` @(360,157)）
    if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let confirm = spawn_panel(&mut commands, bg, 284.0, 289.0, 456.0, 190.0, 47);
        commands.entity(confirm).insert((
            StorageRentConfirm,
            DialogRoot(DialogKind::Storage),
            crate::game::dialogs::AlwaysVisible,
            NotDraggable,
        ));
        commands.entity(confirm).with_children(|p| {
            spawn_label(p, &cjk, "", 35.0, 35.0, 12.0, Color::WHITE, 9)
                .insert(StorageRentConfirmText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10)
                    .insert(StorageRentConfirmOk);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10)
                    .insert(StorageRentConfirmCancel);
            }
        });
    }
}

/// 光标坐标 → 仓库格（按实际格数，#281）。
/// ox/oy = 面板当前原点（拖动/推位后跟随，避免命中失准）
fn storage_slot_at(
    cx: f32,
    cy: f32,
    page_start: usize,
    page_end: usize,
    ox: f32,
    oy: f32,
) -> Option<usize> {
    for slot in page_start..page_end {
        let (rx, ry) = cell_pos(slot, page_start);
        let sx = ox + rx;
        let sy = oy + ry;
        if cx >= sx && cx <= sx + CELL_W && cy >= sy && cy <= sy + CELL_H {
            return Some(slot);
        }
    }
    None
}

/// 页内格位置（C# `Grid[idx].Location = (x*36+9+x, y%8*32+60+y%8)` → 步进 37/33）
pub fn cell_pos(slot: usize, page_start: usize) -> (f32, f32) {
    let d = slot - page_start; // 页内下标（第 2 页 80..159 → 0..79）
    let x = d % COLS;
    let y = d / COLS;
    (
        9.0 + x as f32 * (CELL_W + 1.0),
        60.0 + y as f32 * (CELL_H + 1.0),
    )
}

/// C# `ClientTextKeys` 文案（`Client/Localization/Chinese.json` 逐字）
pub const TEXT_EXPANDED_EXPIRES_ON: &str = "扩展仓库到期时间";
pub const TEXT_EXPANDED_LOCKED: &str = "扩展仓库已锁定";
pub const TEXT_RENT_EXTRA: &str = "是否租用额外仓库 10 天，费用为 1,000,000 金币？";
pub const TEXT_RENT_EXTEND: &str = "是否延长租期 10 天，费用为 1,000,000 金币？";

/// 页码/子控件层（C# `RefreshStorage1` / `RefreshStorage2` / `RentButton.Click`）：
/// 页签切页与帧、租用钮与未扩容遮罩显隐、`RentalLabel` 文案与颜色、租用扩容确认框。
#[allow(clippy::too_many_arguments)]
fn storage_page_system(
    mut state: ResMut<StorageState>,
    mgr: Res<DialogManager>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    net: Res<NetConnection>,
    mut tab1: Query<
        (Entity, &Interaction, &mut ImageButton),
        (With<StoragePage1Tab>, Without<StoragePage2Tab>),
    >,
    mut tab2: Query<
        (Entity, &Interaction, &mut ImageButton),
        (With<StoragePage2Tab>, Without<StoragePage1Tab>),
    >,
    rent_btn: Query<(Entity, &Interaction), With<StorageRentBtn>>,
    confirm_ok: Query<
        (Entity, &Interaction),
        (
            With<StorageRentConfirmOk>,
            Without<StorageRentConfirmCancel>,
        ),
    >,
    confirm_cancel: Query<
        (Entity, &Interaction),
        (
            With<StorageRentConfirmCancel>,
            Without<StorageRentConfirmOk>,
        ),
    >,
    mut locked_page: Query<&mut Visibility, (With<StorageLockedPage>, Without<StorageRentConfirm>)>,
    mut rent_vis: Query<
        &mut Visibility,
        (
            With<StorageRentBtn>,
            Without<StorageLockedPage>,
            Without<StorageRentConfirm>,
        ),
    >,
    mut confirm_vis: Query<&mut Visibility, (With<StorageRentConfirm>, Without<StorageLockedPage>)>,
    mut rental_label: Query<(&mut Text, &mut TextColor), With<StorageRentalLabel>>,
    mut confirm_text: Query<&mut Text, (With<StorageRentConfirmText>, Without<StorageRentalLabel>)>,
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

    // 页签点击 → 切页（C# `Storage1Button.Click` / `Storage2Button.Click`）
    for (e, inter, _) in &mut tab1 {
        if edge(e, inter, &mut prev_inter) {
            state.page = StoragePage::One;
            state.selected = None;
        }
    }
    for (e, inter, _) in &mut tab2 {
        if edge(e, inter, &mut prev_inter) {
            state.page = StoragePage::Two;
            state.selected = None;
        }
    }

    // 页签帧（C# `RefreshStorage1` → 743/746、`RefreshStorage2` → 744/745）
    let on_page1 = state.page == StoragePage::One;
    let t1 = if on_page1 { PAGE1_TAB[0] } else { PAGE1_TAB[1] };
    if let Some(h) = load_lib_image(&mut libs, &mut images, t1.0, t1.1) {
        for (_, _, mut btn) in &mut tab1 {
            if btn.normal != h {
                btn.normal = h.clone();
                btn.hover = h.clone();
                btn.pressed = h.clone();
            }
        }
    }
    let t2 = if on_page1 { PAGE2_TAB[0] } else { PAGE2_TAB[1] };
    if let Some(h) = load_lib_image(&mut libs, &mut images, t2.0, t2.1) {
        for (_, _, mut btn) in &mut tab2 {
            if btn.normal != h {
                btn.normal = h.clone();
                btn.hover = h.clone();
                btn.pressed = h.clone();
            }
        }
    }

    // 第 2 页：`RentButton` 显示；未扩容 → `LockedPage` 遮罩 + 红字「扩展仓库已锁定」，
    // 已扩容 → 白字「扩展仓库到期时间<binary 时间>」（C# `RefreshStorage2`）
    let page2 = !on_page1;
    let expanded = state.has_expanded_storage;
    for mut vis in &mut rent_vis {
        let want = if page2 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for mut vis in &mut locked_page {
        let want = if page2 && !expanded {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (mut text, mut color) in &mut rental_label {
        let (want_text, want_color) = if !page2 {
            (String::new(), Color::WHITE)
        } else if expanded {
            (
                format!("{}{}", TEXT_EXPANDED_EXPIRES_ON, state.expiry_time),
                Color::WHITE,
            )
        } else {
            // C# `RentalLabel.ForeColour = Color.Red`
            (TEXT_EXPANDED_LOCKED.to_string(), Color::srgb(1.0, 0.0, 0.0))
        };
        if text.0 != want_text {
            text.0 = want_text;
        }
        if color.0 != want_color {
            color.0 = want_color;
        }
    }

    // 租用钮点击 → 弹确认框（C# `RentButton.Click` → `MirMessageBox(..., OKCancel)`）
    for (e, inter) in &rent_btn {
        if edge(e, inter, &mut prev_inter) {
            state.rent_confirm = true;
        }
    }
    let storage_open = state.visible && mgr.is_open(DialogKind::Storage);
    let confirm_open = storage_open && state.rent_confirm;
    for mut vis in &mut confirm_vis {
        let want = if confirm_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    if confirm_open {
        let want = if expanded {
            TEXT_RENT_EXTEND
        } else {
            TEXT_RENT_EXTRA
        };
        for mut text in &mut confirm_text {
            if text.0 != want {
                text.0 = want.to_string();
            }
        }
    }
    // Yes → `C.Chat{Message="@ADDSTORAGE"}`（C# `messageBox.OKButton.Click`）
    for (e, inter) in &confirm_ok {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::chat::Chat {
                message: "@ADDSTORAGE".to_string(),
                linked_items: Vec::new(),
            });
            tracing::info!("📦 请求租用扩容仓库（@ADDSTORAGE）");
            state.rent_confirm = false;
        }
    }
    for (e, inter) in &confirm_cancel {
        if edge(e, inter, &mut prev_inter) {
            state.rent_confirm = false;
        }
    }
}

/// 显示/隐藏 + 物品图标渲染 + 选中高亮 + 关闭
#[allow(clippy::type_complexity)]
fn storage_ui_system(
    mut state: ResMut<StorageState>,
    mut mgr: ResMut<DialogManager>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    // 评审 P1：走 UiImageCache 缓存句柄——原 load_lib_image 每帧每格新建 Image
    // 资产（#112 的“无变化不写”因 Handle 恒不等而失效 → GPU 纹理每帧重传）
    mut image_cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mut all_vis: Query<(&mut Visibility, Option<&StorageSlot>), With<StorageWidget>>,
    mut cells: Query<(&mut UiItemCellData, &UiItemCell), With<StorageSlot>>,
    buttons: Query<(Entity, &Interaction, Option<&StorageClose>), With<StorageWidget>>,
    mut slots: Query<(&StorageSlot, &mut BackgroundColor), Without<UiItemCellIcon>>,
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
    let open = state.visible && mgr.is_open(DialogKind::Storage);
    for (mut vis, _slot) in &mut all_vis {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 物品图标 + 数量（#90 通用 UiItemCell：只写数据，渲染由 item_cell_ui_system 处理）
    for (mut data, cell) in &mut cells {
        let item = state.items.get(cell.slot).and_then(|s| s.as_ref());
        let icon = item.and_then(|it| {
            crate::ui::sprite_ui::ui_image(
                &mut libs,
                &mut images,
                &mut image_cache,
                LibraryName::Items,
                it.image as usize,
            )
        });
        let count = item.map(|it| it.count.max(1) as u32);
        // 性能（#112）：无变化不写，避免每帧标记 Changed
        if data.icon.as_ref() != icon.as_ref() {
            data.icon = icon;
        }
        if data.count != count {
            data.count = count;
        }
    }

    // 选中高亮（原版 C# SelectedCell 黄色语义）
    for (slot, mut bg) in &mut slots {
        let selected = state.selected == Some(slot.0);
        let target = if selected {
            Color::srgba(1.0, 0.9, 0.2, 0.35)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.18)
        };
        if bg.0 != target {
            bg.0 = target;
        }
    }

    // 关闭按钮
    for (e, inter, close) in &buttons {
        if edge(e, inter, &mut prev_inter) && close.is_some() {
            // #2956：双闸门同步清——只 mgr.close 会留 (visible=true, mgr=closed)
            // 失配态，此后 RPC `dialog storage toggle` 永远无法再开窗
            state.visible = false;
            // #3258：同上，C# `Hide()` 复位解锁态
            state.unlocked = false;
            mgr.close(DialogKind::Storage);
        }
    }
}

/// 仓库交互：选中+点击 存入/取出（原版 C# MirItemCell 拖放语义）
///
/// C# `MirItemCell` 的存入/取出目标选择：点击格为空 → 用它；否则取该网格**首个空格**
/// （`MirItemCell.cs:1360-1379` 存入、:1069-1090 取出）。返回 `None` = 目标网格已满。
pub fn store_target_slot<T>(items: &[Option<T>], clicked: usize) -> Option<usize> {
    if items.get(clicked).map(|s| s.is_none()).unwrap_or(false) {
        Some(clicked)
    } else {
        items.iter().position(|s| s.is_none())
    }
}

#[allow(clippy::too_many_arguments)]
fn storage_action_system(
    mut state: ResMut<StorageState>,
    mut inv_click: ResMut<InvClickState>,
    // #2633 批次4 步7：gender/class/level/riding 改读组件（HudState 已于步9 删除）
    player_q: Query<
        (
            &Inventory,
            &StatusFlags,
            &Loadout,
            &crate::actor::ActorAppearance,
            &crate::game::player_state::Progression,
            Option<&crate::actor::MountState>,
        ),
        With<LocalPlayer>,
    >,
    inv_ui: Res<InvUiState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    time: Res<Time>,
    inv_origin: Res<crate::game::dialogs::inventory::InventoryOrigin>,
    mut feedback: ResMut<ItemUseFeedback>,
    mut confirm: ResMut<InvDropConfirm>,
    mut locked: ResMut<crate::game::dialogs::inventory::InvLockedSlots>,
    mut last_storage_click: Local<Option<(usize, f64)>>,
    panel_origin: Query<&Node, With<StorageWidget>>,
) {
    if !state.visible || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let (ox, oy) = panel_origin
        .single()
        .map(|n| {
            (
                match n.left {
                    Val::Px(v) => v,
                    _ => DIALOG_X,
                },
                match n.top {
                    Val::Px(v) => v,
                    _ => DIALOG_Y,
                },
            )
        })
        .unwrap_or((DIALOG_X, DIALOG_Y));
    let player = player_q.single().ok();
    let (page_start, page_end) = state.page_range();
    let storage_slot = storage_slot_at(cursor.x, cursor.y, page_start, page_end, ox, oy)
        .filter(|i| !locked.is_locked_in(LockGrid::Storage, *i));
    let inv_slot = inv_slot_at(
        cursor.x,
        cursor.y,
        inv_ui.page,
        player
            .map(|(inv, _, _, _, _, _)| inv.items.len())
            .unwrap_or(0),
        (inv_origin.0, inv_origin.1),
    )
    .filter(|i| !locked.is_locked_in(LockGrid::Inventory, *i));

    // #1546：仓库格双击 → 装备（C# MirItemCell.OnMouseDoubleClick → UseItem；消耗品要求 Grid==Inventory/HeroInventory 故仓库拦截）
    let now = time.elapsed_secs_f64();
    let mut dbl_storage = false;
    if let Some(i) = storage_slot {
        if let Some((last_i, last_t)) = *last_storage_click {
            if last_i == i && now - last_t < 0.4 {
                dbl_storage = true;
                *last_storage_click = None;
            } else {
                *last_storage_click = Some((i, now));
            }
        } else {
            *last_storage_click = Some((i, now));
        }
    }
    if dbl_storage {
        if let Some(item) = state
            .items
            .get(storage_slot.unwrap())
            .and_then(|s| s.as_ref())
        {
            let ctx = UseItemCtx {
                grid: mir2_shared::enums::MirGridType::Storage,
                equipment: player
                    .map(|(_, _, l, _, _, _)| l.slots.as_slice())
                    .unwrap_or(&[]),
                gender: player.map(|(_, _, _, a, _, _)| a.gender as u8).unwrap_or(0),
                class: player.map(|(_, _, _, a, _, _)| a.class as u8).unwrap_or(0),
                level: player.map(|(_, _, _, _, p, _)| p.level).unwrap_or(1),
                check_fishing: true,
                allow_consumable: false,
            };
            if use_item_core(
                item,
                &net,
                // 实体缺失视同未骑乘（原 hud.riding=false 默认）
                player
                    .map(|(_, _, _, _, _, m)| m.is_some())
                    .unwrap_or(false),
                player.map(|(_, f, _, _, _, _)| f.fishing).unwrap_or(false),
                player
                    .map(|(_, _, l, _, _, _)| l.slots.as_slice())
                    .unwrap_or(&[]),
                ctx,
                now,
                &mut feedback,
                &mut confirm,
                // 仓库格不属于玩家背包锁范围（C# 锁的是仓库格）
                &mut None,
            ) == UseOutcome::Sent
            {
                if let Some(sid) = item_use_sound_id(item) {
                    feedback.sounds.push(sid);
                }
            }
        }
        state.selected = None;
        return;
    }

    // 1) 选中了背包物品 → 点仓库格：存入（原版 C# SelectedCell Inventory → Storage 拖放）
    // #2631：选中态归 inventory 所有，经 selected() 读、clear_selected() 清（存入后不再保留）
    if let Some(from) = inv_click.selected() {
        if let Some(to) = storage_slot {
            // C# `MirItemCell.cs:1360-1379`：目标格空则用它，否则取仓库首个空格；
            // 发包后 `StorageDialog.Grid[to].Locked = true` + `SelectedCell.Locked = true`，
            // `S.StoreItem`（GameScene.cs:2737-2752）回包把两格都解锁。
            let Some(to) = store_target_slot(&state.items, to) else {
                tracing::warn!("📦 仓库已满，无法存入");
                return;
            };
            inv_click.clear_selected();
            net.send_packet(&mir2_shared::packets::client::item::StoreItem {
                from: from as i32,
                to: to as i32,
            });
            locked.lock_in(InvLockReason::Storage, LockGrid::Storage, to);
            locked.lock_in(InvLockReason::Storage, LockGrid::Inventory, from);
            tracing::info!("📦 存入仓库 {} -> {}", from, to);
            state.selected = None;
            return;
        }
    }

    // 2) 选中了仓库物品 → 点背包格：取出（原版 C# SelectedCell Storage → Inventory 拖放）
    if let Some(from) = state.selected {
        if let Some(to) = inv_slot {
            // C# `MirItemCell.cs:1069-1090`：目标格空则用它，否则取背包首个空格；
            // 发包后 `temp.Locked = true`（目标背包格）+ `SelectedCell.Locked = true`（仓库来源格），
            // `S.TakeBackItem`（GameScene.cs:2720-2735）回包解锁两格。
            let Some(to) = player.and_then(|(inv, ..)| store_target_slot(&inv.items, to)) else {
                tracing::warn!("📦 背包已满，无法取出");
                return;
            };
            net.send_packet(&mir2_shared::packets::client::item::TakeBackItem {
                from: from as i32,
                to: to as i32,
            });
            locked.lock_in(InvLockReason::Storage, LockGrid::Inventory, to);
            locked.lock_in(InvLockReason::Storage, LockGrid::Storage, from);
            tracing::info!("📦 取出仓库 {} -> {}", from, to);
            state.selected = None;
            inv_click.clear_selected(); // #2631：经接口清（互斥）
            return;
        }
    }

    // 3) 点仓库格：选中/取消选中（只有物品格可选中）
    if let Some(i) = storage_slot {
        match state.selected {
            Some(sel) if sel == i => state.selected = None,
            _ => {
                if state.items.get(i).and_then(|s| s.as_ref()).is_some() {
                    state.selected = Some(i);
                    inv_click.clear_selected(); // #2631：经接口清（与背包选中互斥）
                }
            }
        }
    }

    // 4) 点背包物品格：交给背包系统（选中）；这里仅清掉仓库选择
    if inv_slot.is_some() {
        state.selected = None;
    }
}

/// 锁定仓库格灰化（C# `MirItemCell.DrawControl`：`Locked` → `Color.DimGray` × 0.8；
/// 与背包 `inv_locked_icon_system` 同一着色规则，`LOCKED_ITEM_COLOR` 单一来源）。
/// 通过 `ChildOf → StorageSlot` 映射，只作用于仓库格（其它对话框的 UiItemCellIcon 跳过）。
fn storage_locked_icon_system(
    locked: Res<InvLockedSlots>,
    slots: Query<&StorageSlot>,
    mut icons: Query<(&ChildOf, &mut ImageNode), With<UiItemCellIcon>>,
) {
    for (child_of, mut node) in &mut icons {
        let Ok(slot) = slots.get(child_of.parent()) else {
            continue;
        };
        let want = locked.color_at(LockGrid::Storage, slot.0);
        if node.color != want {
            node.color = want;
        }
    }
}

/// 消费服务端仓库事件（网络层只广播 ServerEvent；仓库/背包打开逻辑归本模块）
/// #2633 批次4 步9：ItemStored/ItemTakenBack 移动背包格直接写 `Inventory` 组件（HudState 已删）。
fn storage_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut storage: ResMut<StorageState>,
    net: Res<NetConnection>,
    mut mgr: ResMut<DialogManager>,
    mut inv_origin: ResMut<crate::game::dialogs::inventory::InventoryOrigin>,
    // 只推背包面板根（同 inventory_shift_right_system：子实体随根平移，
    // 根+格双重 +dx 会把背包推出屏幕——评审 P0）
    mut inv_entities: Query<
        (&mut Node, &DialogRoot),
        With<crate::game::dialogs::inventory::InventoryPanel>,
    >,
    mut inv_q: Query<&mut Inventory, With<LocalPlayer>>,
    mut locked: ResMut<InvLockedSlots>,
    // #3258：密码流程的落点（弹输入框 / 系统提示）都在这个系统里
    mut pwd_flow: ResMut<StoragePwdFlow>,
    mut input_box_state: ResMut<crate::game::dialogs::input_box::InputBoxState>,
    mut text_input_state: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut chat: ResMut<crate::game::chat::ChatState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::StorageOpened { items, visible } = ev {
            storage.items = items.clone();
            storage.visible = *visible;
            // C# `StorageDialog.Show()` 末尾调 `RefreshStorage1()` → 打开即第 1 页
            storage.page = StoragePage::One;
            storage.rent_confirm = false;
            storage.selected = None;
            if *visible {
                // 原版 C#：仓库打开时同时显示背包，且背包推到 (仓宽+5, 仓Y)=(393,0)
                // 并排（NPCDialogs.cs:2967/2990 `InventoryDialog.Location = new Point(Size.Width+5, Location.Y)`）
                // —— 否则 388x346 的仓库完全罩住 316x236 的背包。
                let mut min_x = f32::MAX;
                for (node, root) in inv_entities.iter() {
                    if root.0 == DialogKind::Inventory {
                        if let Val::Px(v) = node.left {
                            min_x = min_x.min(v);
                        }
                    }
                }
                if min_x < f32::MAX {
                    let dx = STORAGE_W + 5.0 - min_x;
                    for (mut node, root) in &mut inv_entities {
                        if root.0 == DialogKind::Inventory {
                            let cur = match node.left {
                                Val::Px(v) => v,
                                _ => 0.0,
                            };
                            node.left = Val::Px(cur + dx);
                        }
                    }
                    *inv_origin =
                        crate::game::dialogs::inventory::InventoryOrigin(STORAGE_W + 5.0, 0.0);
                }
                if !mgr.is_open(DialogKind::Storage) {
                    mgr.open.push(DialogKind::Storage);
                }
                if !mgr.is_open(DialogKind::Inventory) {
                    mgr.open.push(DialogKind::Inventory);
                }
            } else {
                // #2960：双闸门配对——visible=false 时 mgr 栈必须同步不含 Storage，
                // 否则 (visible=false, mgr=open) 失配，RPC `dialog storage toggle`
                // 在 (false,open)↔(true,closed) 间振荡、永远到不了 (true,true)
                mgr.close(DialogKind::Storage);
                // #3258：C# `Hide()`（`NPCDialogs.cs:2997-3001`）里 `_storageUnlocked = false`
                storage.unlocked = false;
            }
        }
        if let ServerEvent::StorageOpened { .. } = ev {
            // P3-3（#782）：仓库线包不带 `ItemInfo`（原版 `GameScene.cs:4955` 拿到
            // `Storage` 后逐格 `Bind`），显示名要按本地物品名表解析；表里也没有的
            // 索引按索引去重后发一次 `RequestItemInfo`（原版 `GameScene.RequestItemInfo`
            // 的三条守卫：`index <= 0 || HasItemInfo(index) || !RequestedItemInfo.Add(index)`）。
            for idx in storage.resolve_wire_item_names() {
                net.send_packet(&mir2_shared::packets::client::info::RequestItemInfo {
                    item_index: idx,
                });
                tracing::info!("🏬 仓库缺物品名，请求 ItemInfo: idx={}", idx);
            }
        }
        // P3-3：按需请求的回包——写进表（原版 `MirScene.cs:233` NewItemInfo → ItemInfoList），
        // 并把仍在占位的仓库格立刻刷成真名。
        if let ServerEvent::ItemInfoReceived { index, name, .. } = ev {
            if crate::game::item_names::remember_item_name(&mut storage.item_names, *index, name) {
                storage.requested_item_info.remove(index);
                for slot in storage.items.iter_mut() {
                    if let Some(item) = slot.as_mut() {
                        if item.item_index == *index {
                            item.name = name.clone();
                        }
                    }
                }
            }
        }
        // P3-3：背包/装备名（`UserInformation`）也进同一张表——原版那张 `ItemInfoList`
        // 是全局的，仓库格名字能从里面直接取到，省一次往返。
        if let ServerEvent::UserInformation {
            item_names,
            has_storage_password,
            require_storage_password,
            storage_password_last_set,
            ..
        } = ev
        {
            for (idx, name) in item_names {
                crate::game::item_names::remember_item_name(&mut storage.item_names, *idx, name);
            }
            // C# `GameScene.User.HasStoragePassword / RequireStoragePassword / StoragePasswordLastSet`
            // （`UserInformation` 逐字段赋值）—— 密码流程的三个判据源
            storage.has_password = *has_storage_password;
            storage.require_password = *require_storage_password;
            storage.password_last_set = *storage_password_last_set;
        }
        if let ServerEvent::StoragePasswordResult { result } = ev {
            // C# `HandleStoragePasswordResult`（`NPCDialogs.cs:3027-3078`）：
            // 4=成功 2=当前密码错误 5=未设置密码 1/3=格式不可接受 0=不可用
            let had_password = storage.has_password;
            storage.pwd_last_error.clear();
            let msg = match *result {
                4 => {
                    storage.has_password = true;
                    if had_password {
                        TEXT_PWD_CHANGE_SUCCESS
                    } else {
                        TEXT_PWD_SET_SUCCESS
                    }
                }
                1 | 3 => {
                    storage.pwd_last_error = TEXT_PWD_NOT_ACCEPTABLE.to_string();
                    TEXT_PWD_NOT_ACCEPTABLE
                }
                2 => {
                    storage.pwd_last_error = TEXT_PWD_WRONG.to_string();
                    TEXT_PWD_WRONG
                }
                5 => {
                    storage.pwd_last_error = TEXT_PWD_NO_PASSWORD.to_string();
                    TEXT_PWD_NO_PASSWORD
                }
                _ => {
                    storage.pwd_last_error = TEXT_PWD_UNAVAILABLE.to_string();
                    TEXT_PWD_UNAVAILABLE
                }
            };
            // C# `SendStorageSystemMessage`：走聊天窗的系统频道
            chat.add_line(
                msg,
                crate::game::chat::chat_color(mir2_shared::enums::ChatType::System),
                crate::game::chat::ChatChannel::System,
            );
            tracing::info!("🔒 仓库密码结果 result={}：{msg}", result);
        }
        if let ServerEvent::StoragePrompt = ev {
            // C# `S.NPCStorage` → `StorageDialog.Show()` → 有密码且未解锁 → `PromptStorageUnlock()`
            // （`NPCDialogs.cs:2982-2988`）。服务端只在**有密码**时发这个包（`npc.rs:781`）。
            start_unlock_prompt(&mut input_box_state, &mut text_input_state, &mut storage, &mut pwd_flow);
        }
        if let ServerEvent::StorageResized {
            size,
            has_expanded_storage,
            expiry_time,
        } = ev
        {
            // #281/#2892 批B：仓库扩容（C# S.ResizeStorage → Array.Resize + RefreshStorage2）
            storage.has_expanded_storage = *has_expanded_storage;
            storage.expiry_time = *expiry_time;
            storage.resize(*size);
            tracing::info!(
                "📦 仓库扩容 -> {} 格（扩容状态={}）",
                storage.items.len(),
                storage.has_expanded_storage
            );
        }
        if let ServerEvent::StorageUnlockResult {
            result,
            has_password,
        } = ev
        {
            // C# `HandleStorageUnlockResult`（`NPCDialogs.cs:3003-3025`）：
            // 0/4 = 成功 → `_storageUnlocked = true` + `Show()`；1/2/3 → 系统提示（解锁框留着）
            storage.has_password = *has_password;
            storage.pwd_last_error.clear();
            match *result {
                0 | 4 => {
                    storage.unlocked = true;
                    storage.unlock_prompt_open = false;
                }
                1 => {
                    storage.pwd_last_error = TEXT_PWD_NOT_ACCEPTABLE.to_string();
                }
                2 => {
                    storage.pwd_last_error = TEXT_PWD_WRONG.to_string();
                }
                3 => {
                    storage.pwd_last_error = TEXT_PWD_UNAVAILABLE.to_string();
                }
                _ => {
                    storage.pwd_last_error = TEXT_PWD_UNAVAILABLE.to_string();
                }
            }
            if !storage.pwd_last_error.is_empty() {
                chat.add_line(
                    storage.pwd_last_error.clone(),
                    crate::game::chat::chat_color(mir2_shared::enums::ChatType::System),
                    crate::game::chat::ChatChannel::System,
                );
            }
        }
        if let ServerEvent::ItemStored { from, to, success } = ev {
            // #512：C# S.StoreItem —— 背包 -> 仓库（success 时移动物品）
            // #2747+：C# `GameScene.StoreItem`（:2737-2752）回包同时解锁 `fromCell`/`toCell`
            locked.unlock_all(InvLockReason::Storage);
            if *success {
                let (fi, ti) = (*from as usize, *to as usize);
                if let Ok(mut inv) = inv_q.single_mut() {
                    if fi < inv.items.len() && ti < storage.items.len() {
                        if let Some(item) = inv.items[fi].take() {
                            if storage.items[ti].is_none() {
                                storage.items[ti] = Some(item);
                                tracing::info!("📦 存入仓库 {} -> {}（{}）", from, to, "成功");
                            } else {
                                inv.items[fi] = Some(item);
                                tracing::warn!("📦 存入仓库 {} -> {} 失败：目标格已占用", from, to);
                            }
                        }
                    }
                }
            }
        }
        if let ServerEvent::ItemTakenBack { from, to, success } = ev {
            // #512：C# S.TakeBackItem —— 仓库 -> 背包（success 时移动物品）
            // C# `GameScene.TakeBackItem`（:2720-2735）回包解锁 `fromCell`/`toCell`
            locked.unlock_all(InvLockReason::Storage);
            if *success {
                let (fi, ti) = (*from as usize, *to as usize);
                if let Ok(mut inv) = inv_q.single_mut() {
                    if fi < storage.items.len() && ti < inv.items.len() {
                        if let Some(item) = storage.items[fi].take() {
                            if inv.items[ti].is_none() {
                                inv.items[ti] = Some(item);
                                tracing::info!("📦 取出仓库 {} -> {}（{}）", from, to, "成功");
                            } else {
                                storage.items[fi] = Some(item);
                                tracing::warn!("📦 取出仓库 {} -> {} 失败：目标格已占用", from, to);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 悬停提示（#93 通用 Tooltip）：光标在仓库物品格上显示 名称 x数量
fn storage_tooltip_system(
    state: Res<StorageState>,
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    windows: Query<&Window>,
    panel_origin: Query<&Node, With<StorageWidget>>,
) {
    if !state.visible {
        tooltip.update(3, false, String::new(), Vec::new(), 0.0, 0.0);
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (ox, oy) = panel_origin
        .single()
        .map(|n| {
            (
                match n.left {
                    Val::Px(v) => v,
                    _ => DIALOG_X,
                },
                match n.top {
                    Val::Px(v) => v,
                    _ => DIALOG_Y,
                },
            )
        })
        .unwrap_or((DIALOG_X, DIALOG_Y));
    let mut hit: Option<crate::game::dialogs::inventory::InvItem> = None;
    let (page_start, page_end) = state.page_range();
    if let Some(i) = storage_slot_at(cursor.x, cursor.y, page_start, page_end, ox, oy) {
        hit = state.items.get(i).and_then(|s| s.as_ref()).cloned();
    }
    let Some(item) = hit else {
        tooltip.update(3, false, String::new(), Vec::new(), cursor.x, cursor.y);
        return;
    };
    // 与背包一致：完整属性行（#1244 item_tooltip_lines）
    let lines = crate::game::dialogs::inventory::item_tooltip_lines(&item);
    // P3-3（#782）：格子里存的是线包名（常常只是 `#id`），显示前按本地表解析
    let title = state.display_name(&item);
    tooltip.update(3, true, title, lines, cursor.x, cursor.y);
}

// ============================================================================
// #3258 仓库密码流程：按 C# 金标准（`NPCDialogs.cs:3085-3239`）重做
//
// C# 用**嵌套闭包回调**表达这个状态机：
//   ManageStoragePassword()（:3085）
//     ├─ !HasStoragePassword → BeginSetStoragePassword()（:3123）
//     │      Prompt(new) → Prompt(confirm) → `C.SetStoragePassword{"", new}`；
//     │      不一致 → 系统提示 + （force 时重来 / 否则原地重输）
//     └─ HasStoragePassword  → MirMessageBox(changePrompt, OKCancel)（:3104）
//                              OK → BeginChangeStoragePassword()（:3159）
//                                   Prompt(current) → Prompt(new) → Prompt(confirm)
//                                   → `C.SetStoragePassword{current, new}`
//   PromptStorageUnlock()（:3192）：Prompt(password) → `C.UnlockStorage{password}`；Cancel → Hide()
// 每一步都走 `PromptStoragePassword`（:3206）：`MirInputBox(prompt + "\n" + rules)`、
// `InputTextBox.Password = true`、空输入时 OK 无效。
//
// 原实现是一个自造的「设置 / 移除 / 关闭」三钮面板，用的还是 `Title[206]`(YES)/`Title[210]`(NO)
// 这套「艺术图里烘了英文」的按钮，又在上面叠中文标签 ⇒ 叠字（owner 反馈项）。整块删除。
// ============================================================================

/// `Globals.MinPasswordLength` / `MaxPasswordLength`（`Shared/Globals.cs:10`；服务端同值）
pub const MIN_PASSWORD_LEN: usize = 5;
pub const MAX_PASSWORD_LEN: usize = 15;

/// C# `ClientTextKeys.*`（英文默认值见 `Shared/Language.cs:2693-2709`；中文词条不在本机可见的
/// `Client/Localization/Chinese.json` 里——原始安装也没有 Localization 目录，C# 运行时会回落到
/// 那份英文默认。本端按同一模板给出中文措辞，语义逐条对应。）
pub const TEXT_PWD_PROMPT: &str = "请输入仓库密码。";
pub const TEXT_PWD_NEW: &str = "请输入新的仓库密码。";
pub const TEXT_PWD_CONFIRM: &str = "请再次输入新的仓库密码。";
pub const TEXT_PWD_CURRENT: &str = "请输入当前仓库密码。";
pub const TEXT_PWD_CHANGE_PROMPT: &str = "是否修改仓库密码？";
pub const TEXT_PWD_LAST_SET_PREFIX: &str = "上次设置：";
pub const TEXT_PWD_MISMATCH: &str = "两次输入的密码不一致。";
pub const TEXT_PWD_SET_SUCCESS: &str = "仓库密码已设置。";
pub const TEXT_PWD_CHANGE_SUCCESS: &str = "仓库密码已修改。";
pub const TEXT_PWD_WRONG: &str = "仓库密码错误。";
pub const TEXT_PWD_NOT_ACCEPTABLE: &str = "密码不符合要求。";
pub const TEXT_PWD_UNAVAILABLE: &str = "仓库不可用。";
pub const TEXT_PWD_NO_PASSWORD: &str = "未设置仓库密码。";

/// `StoragePasswordRules`（`Shared/Language.cs:2707`）：提示的第二行
pub fn storage_password_rules() -> String {
    format!(
        "密码长度必须为 {}-{} 个字符，只能用字母和数字。",
        MIN_PASSWORD_LEN, MAX_PASSWORD_LEN
    )
}

/// 流程当前步（对应 C# 里嵌套到哪一层闭包）
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub enum StoragePwdStep {
    #[default]
    None,
    /// `PromptStoragePassword(StoragePasswordNewPrompt)`（`:3125`）
    SetNew,
    /// 已收到新密码，等确认（`:3138`）
    SetConfirm { new: String },
    /// `PromptStoragePassword(StoragePasswordCurrentPrompt)`（`:3161`）
    ChangeCurrent,
    /// 已收到当前密码，等新密码（`:3168`）
    ChangeNew { current: String },
    /// 已收到新密码，等确认（`:3175`）
    ChangeConfirm { current: String, new: String },
    /// `PromptStoragePassword(StoragePasswordPrompt)`（`:3194`）
    Unlock,
}

/// 密码流程资源（C# 里那些闭包的现场）
#[derive(Resource, Default)]
pub struct StoragePwdFlow {
    pub step: StoragePwdStep,
    /// C# `PromptStorageUnlock(..., () => Hide())` 与 `BeginSetStoragePassword(onCancel=…)`：
    /// 取消时是否**连仓库窗一起关**；改密路径不传 onCancel ⇒ 只关输入框
    pub cancel_hides_storage: bool,
}

/// 要发的包（原版只有这两个：`SetStoragePassword` / `UnlockStorage`）
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PwdPacket {
    Set { current: String, new: String },
    Unlock { password: String },
}

/// 一步输入的判定结果（纯数据，便于单测；副作用由 `input_box.rs` 执行）
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct PwdStepResult {
    /// 输入框是否保持打开（空输入 = C# `return false`；不一致时 C# 也 `return false`）
    pub keep_open: bool,
    pub packet: Option<PwdPacket>,
    /// 要回显到系统频道的文案（C# `SendStorageSystemMessage`）
    pub message: Option<String>,
    /// 前进到下一问（调用方用 `open_pwd_prompt` 打开）
    pub next: Option<(StoragePwdStep, &'static str)>,
    /// C# `:3145-3147`：强制设密码且两次不一致 → 从第一步**重来**
    pub restart_set: bool,
}

/// 打开一步「仓库密码」输入框（C# `PromptStoragePassword`，`:3206-3239`）：
/// 标题 = `prompt + 换行 + rules`，输入框走密码遮罩（`TextInputState.masked`）。
pub fn open_pwd_prompt(
    ib: &mut crate::game::dialogs::input_box::InputBoxState,
    input: &mut crate::game::dialogs::text_input::TextInputState,
    flow: &mut StoragePwdFlow,
    step: StoragePwdStep,
    prompt: &str,
) {
    let title = format!("{prompt}\n{}", storage_password_rules());
    crate::game::dialogs::input_box::open_input_box(
        ib,
        input,
        crate::game::dialogs::input_box::InputPurpose::StoragePassword,
        &title,
    );
    input.set_masked(crate::game::dialogs::input_box::INPUT_FIELD_ID, true);
    flow.step = step;
}

/// 开始「设置密码」流程（C# `BeginSetStoragePassword`，`:3123`）
pub fn start_set_password_flow(
    ib: &mut crate::game::dialogs::input_box::InputBoxState,
    input: &mut crate::game::dialogs::text_input::TextInputState,
    flow: &mut StoragePwdFlow,
) {
    flow.cancel_hides_storage = true; // C# `BeginSetStoragePassword(onCancel=…)`
    open_pwd_prompt(ib, input, flow, StoragePwdStep::SetNew, TEXT_PWD_NEW);
    tracing::info!("🔒 仓库密码：开始设置流程");
}

/// 开始「修改密码」流程（C# `BeginChangeStoragePassword`，`:3159`）
pub fn start_change_password_flow(
    ib: &mut crate::game::dialogs::input_box::InputBoxState,
    input: &mut crate::game::dialogs::text_input::TextInputState,
    flow: &mut StoragePwdFlow,
) {
    flow.cancel_hides_storage = false; // C# 这条路径不传 onCancel
    open_pwd_prompt(
        ib,
        input,
        flow,
        StoragePwdStep::ChangeCurrent,
        TEXT_PWD_CURRENT,
    );
    tracing::info!("🔒 仓库密码：开始修改流程");
}

/// 解锁提示（C# `PromptStorageUnlock`，`:3192`）
pub fn start_unlock_prompt(
    ib: &mut crate::game::dialogs::input_box::InputBoxState,
    input: &mut crate::game::dialogs::text_input::TextInputState,
    st: &mut StorageState,
    flow: &mut StoragePwdFlow,
) {
    flow.cancel_hides_storage = true; // C# `() => Hide()`
    open_pwd_prompt(ib, input, flow, StoragePwdStep::Unlock, TEXT_PWD_PROMPT);
    st.unlock_prompt_open = true;
    tracing::info!("🔓 仓库密码：弹解锁输入框");
}

/// C# `StorageDialog.ManageStoragePassword()`（`:3085-3107`）—— ProtectButton 点击入口
pub fn manage_storage_password(
    ib: &mut crate::game::dialogs::input_box::InputBoxState,
    input: &mut crate::game::dialogs::text_input::TextInputState,
    st: &mut StorageState,
    flow: &mut StoragePwdFlow,
) {
    if !st.require_password {
        return; // C# `:3088`
    }
    if !st.has_password {
        start_set_password_flow(ib, input, flow); // C# `:3090-3093`
        return;
    }
    st.change_confirm = true; // C# `:3104` MirMessageBox(changePrompt, OKCancel)
}

/// C# `PromptStoragePassword` 的 `onSubmit`（`:3206-3239`）一步：推进状态机。
pub fn storage_password_step(
    text: &str,
    st: &mut StorageState,
    flow: &mut StoragePwdFlow,
) -> PwdStepResult {
    let mut out = PwdStepResult::default();
    match flow.step.clone() {
        StoragePwdStep::None => {}
        StoragePwdStep::Unlock => {
            if text.is_empty() {
                out.keep_open = true;
                return out;
            }
            out.packet = Some(PwdPacket::Unlock {
                password: text.to_string(),
            });
            flow.step = StoragePwdStep::None;
            st.unlock_prompt_open = false;
        }
        StoragePwdStep::SetNew => {
            if text.is_empty() {
                out.keep_open = true;
                return out;
            }
            out.next = Some((
                StoragePwdStep::SetConfirm {
                    new: text.to_string(),
                },
                TEXT_PWD_CONFIRM,
            ));
            out.keep_open = true;
        }
        StoragePwdStep::SetConfirm { new } => {
            if text.is_empty() {
                out.keep_open = true;
                return out;
            }
            if new != text {
                out.message = Some(TEXT_PWD_MISMATCH.to_string());
                // C# `:3143-3150`：force 时重来，否则 `return false`（原地重输）
                if st.forcing_setup {
                    out.restart_set = true;
                }
                out.keep_open = true;
                return out;
            }
            out.packet = Some(PwdPacket::Set {
                current: String::new(),
                new,
            });
            flow.step = StoragePwdStep::None;
        }
        StoragePwdStep::ChangeCurrent => {
            if text.is_empty() {
                out.keep_open = true;
                return out;
            }
            out.next = Some((
                StoragePwdStep::ChangeNew {
                    current: text.to_string(),
                },
                TEXT_PWD_NEW,
            ));
            out.keep_open = true;
        }
        StoragePwdStep::ChangeNew { current } => {
            if text.is_empty() {
                out.keep_open = true;
                return out;
            }
            out.next = Some((
                StoragePwdStep::ChangeConfirm {
                    current,
                    new: text.to_string(),
                },
                TEXT_PWD_CONFIRM,
            ));
            out.keep_open = true;
        }
        StoragePwdStep::ChangeConfirm { current, new } => {
            if text.is_empty() {
                out.keep_open = true;
                return out;
            }
            if new != text {
                out.message = Some(TEXT_PWD_MISMATCH.to_string());
                out.keep_open = true; // C# `:3177-3181`：提示后 `return false`
                return out;
            }
            out.packet = Some(PwdPacket::Set { current, new });
            flow.step = StoragePwdStep::None;
        }
    }
    out
}

/// 发密码流程的包（C# `Network.Enqueue(new C.SetStoragePassword{…} / new C.UnlockStorage{…})`）
pub fn send_pwd_packet(net: &NetConnection, pkt: &PwdPacket) {
    match pkt {
        PwdPacket::Set { current, new } => {
            net.send_packet(&mir2_shared::packets::client::storage::SetStoragePassword {
                current_password: current.clone(),
                new_password: new.clone(),
            });
            tracing::info!(
                "🔒 发送 C.SetStoragePassword（current_len={} new_len={}）",
                current.chars().count(),
                new.chars().count()
            );
        }
        PwdPacket::Unlock { password } => {
            net.send_packet(&mir2_shared::packets::client::storage::UnlockStorage {
                password: password.clone(),
            });
            tracing::info!("🔓 发送 C.UnlockStorage（len={}）", password.chars().count());
        }
    }
}

/// 输入框 Cancel/Esc（C# `onCancel`）：按 `cancel_hides_storage` 决定是否连仓库窗一起关。
pub fn storage_password_cancel(st: &mut StorageState, flow: &mut StoragePwdFlow) {
    flow.step = StoragePwdStep::None;
    st.unlock_prompt_open = false;
    if flow.cancel_hides_storage {
        // C# `CancelStoragePasswordSetup()`（`:3116-3121`）→ `Hide()`
        st.forcing_setup = false;
        st.unlocked = false;
        st.visible = false;
        st.close_requested = true;
    }
    flow.cancel_hides_storage = false;
}

/// C# `StoragePasswordLastSet.ToString("g")`：本端用同口径的本地时间串（仅作提示行）
fn format_pwd_last_set(epoch_secs: i64) -> String {
    let secs = epoch_secs.max(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    // 民用历换算（Howard Hinnant days_from_civil 的逆运算）
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y0 = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y0 + 1 } else { y0 };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, mth, d, h, m)
}

/// 改密确认框（C# `MirMessageBox(prompt, OKCancel)`）+ ProtectButton 入口的驱动系统。
#[allow(clippy::too_many_arguments)]
fn storage_pwd_flow_system(
    mut st: ResMut<StorageState>,
    mut mgr: ResMut<DialogManager>,
    mut ib: ResMut<crate::game::dialogs::input_box::InputBoxState>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut flow: ResMut<StoragePwdFlow>,
    pwd_btn: Query<(Entity, &Interaction), With<StoragePwdBtn>>,
    ok_btn: Query<
        (Entity, &Interaction),
        (With<StoragePwdChangeOk>, Without<StoragePwdChangeCancel>),
    >,
    cancel_btn: Query<
        (Entity, &Interaction),
        (With<StoragePwdChangeCancel>, Without<StoragePwdChangeOk>),
    >,
    mut panel: Query<&mut Visibility, With<StoragePwdChangeConfirm>>,
    mut text: Query<&mut Text, With<StoragePwdChangeConfirmText>>,
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
    // `storage_password_cancel` 只能改状态（它在 `input_box` 系统里被调用、拿不到 DialogManager），
    // 「关仓窗」这个副作用在这里落地（C# `Hide()`）。
    if st.close_requested {
        st.close_requested = false;
        mgr.close(DialogKind::Storage);
    }
    let open = st.change_confirm && st.visible && mgr.is_open(DialogKind::Storage);
    for mut vis in &mut panel {
        let want = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    if open {
        // C# `:3096-3102`：提示 +（若设过）「上次设置」行
        let mut want = TEXT_PWD_CHANGE_PROMPT.to_string();
        if st.password_last_set > 0 {
            want.push('\n');
            want.push_str(TEXT_PWD_LAST_SET_PREFIX);
            want.push_str(&format_pwd_last_set(st.password_last_set));
        }
        for mut t in &mut text {
            if t.0 != want {
                t.0 = want.clone();
            }
        }
    }
    for (e, inter) in &pwd_btn {
        if edge(e, inter, &mut prev_inter) {
            manage_storage_password(&mut ib, &mut input, &mut st, &mut flow);
        }
    }
    for (e, inter) in &ok_btn {
        if edge(e, inter, &mut prev_inter) && open {
            st.change_confirm = false;
            start_change_password_flow(&mut ib, &mut input, &mut flow);
        }
    }
    for (e, inter) in &cancel_btn {
        if edge(e, inter, &mut prev_inter) && open {
            st.change_confirm = false; // C# `MirMessageBox` 的 Cancel 只 Dispose
        }
    }
}

/// 仓库动态格子同步（#281）：按 StorageState.items.len() 生成/移除 StorageSlot 格子。
/// 对齐 C# StorageDialog Grid（10x8=80 上限）；缩容时移除多余格子。
fn storage_grid_sync_system(
    mut commands: Commands,
    state: Res<StorageState>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    slots: Query<(Entity, &StorageSlot)>,
    panel_origin: Query<&Node, With<StorageWidget>>,
) {
    if state.items.is_empty() && slots.is_empty() {
        return; // 进图 UserStorage 到达前：无格子可同步
    }
    // 当前页的格区间（C# `RefreshStorage1/2`：第 2 页在未扩容时整页隐藏）
    let (page_start, page_end) = state.page_range();
    // 面板当前原点（拖动/推位后新格与已平移格对齐；DIALOG 常量仅为初始值）
    let (ox, oy) = panel_origin
        .single()
        .map(|n| {
            (
                match n.left {
                    Val::Px(v) => v,
                    _ => DIALOG_X,
                },
                match n.top {
                    Val::Px(v) => v,
                    _ => DIALOG_Y,
                },
            )
        })
        .unwrap_or((DIALOG_X, DIALOG_Y));
    // 移除不属于当前页的格子（切页时旧页格子回收，新页重建）
    for (e, s) in &slots {
        if s.0 < page_start || s.0 >= page_end {
            commands.entity(e).despawn();
        }
    }
    let mut existing: Vec<usize> = slots
        .iter()
        .map(|(_, s)| s.0)
        .filter(|i| *i >= page_start && *i < page_end)
        .collect();
    existing.sort_unstable();
    if existing.len() == page_end - page_start {
        return;
    }
    // 补当前页缺失的格子
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let font = ui_font.0.clone();
    let mut next = 0usize;
    for i in page_start..page_end {
        if existing.get(next).copied() == Some(i) {
            next += 1;
            continue;
        }
        let (rx, ry) = cell_pos(i, page_start);
        let sx = ox + rx;
        let sy = oy + ry;
        let cell = spawn_item_cell_ui_root(
            &mut commands,
            &mut images,
            &font,
            sx,
            sy,
            CELL_W,
            CELL_H,
            STORAGE_CELL_Z,
            i,
        );
        commands.entity(cell).insert((
            StorageSlot(i),
            DialogRoot(DialogKind::Storage),
            // 格子用 `ZIndex`（相对）而非 `GlobalZIndex`，本身不是拖动根（`dialog_drag_system`
            // 的查询要求 `&GlobalZIndex`）；此处显式排除，防止后续误加 GlobalZIndex
            NotDraggable,
            StorageWidget,
        ));
    }
}

#[cfg(test)]
mod tests {
    /// 门禁（P3-3 / #782）：仓库格显示名必须走与商城同一条降级链
    /// （`线包名 → 本地物品名表 → 需要请求 → 兜底 #id`），且「表里也没有」时
    /// **要**发请求（不是静默显示 #id 就算完）；同一索引只请求一次。
    ///
    /// 阳性对照（落地时实做）：把 `resolve_wire_item_names` 里查表那一步去掉
    /// （直接落 `#id` 且不请求）→ 本测试第一条断言立即红。
    #[test]
    fn storage_item_names_follow_shared_fallback_chain() {
        use crate::game::dialogs::inventory::InvItem;
        let mk = |name: &str, idx: i32| InvItem {
            name: name.to_string(),
            item_index: idx,
            ..Default::default()
        };
        let mut st = super::StorageState::default();
        st.items = vec![
            // ① 线包自带名字（服务端补过名字的情形）：直接用，不请求
            Some(mk("屠龙", 1268)),
            // ② 线包无名（仓库线包的真实形态）：先占位 `#782`，并要求请求一次
            Some(mk("", 782)),
            // ③ 表里已有名字：用表里的，不请求
            Some(mk("", 1001)),
            None,
        ];
        crate::game::item_names::remember_item_name(&mut st.item_names, 1001, "乌木剑");

        let wanted = st.resolve_wire_item_names();
        assert_eq!(
            wanted,
            vec![782],
            "只有表里也没有的索引才发请求（#782 那一格）"
        );
        assert_eq!(st.display_name(st.items[0].as_ref().unwrap()), "屠龙");
        assert_eq!(
            st.display_name(st.items[1].as_ref().unwrap()),
            "#782",
            "回包未到前仍是占位（原版 `ItemIndexTitle`）"
        );
        assert_eq!(st.display_name(st.items[2].as_ref().unwrap()), "乌木剑");
        // 去重：同一索引再解析一次不得重复请求（原版 `RequestedItemInfo` 语义）
        assert!(
            st.resolve_wire_item_names().is_empty(),
            "同一索引不得重复请求"
        );

        // `ItemInfoReceived` 到货（消费臂的等价动作）：格子立刻显示真名，且不再请求
        assert!(crate::game::item_names::remember_item_name(
            &mut st.item_names,
            782,
            "马鞍"
        ));
        st.requested_item_info.remove(&782);
        assert!(st.resolve_wire_item_names().is_empty());
        assert_eq!(st.display_name(st.items[1].as_ref().unwrap()), "马鞍");
    }

    /// #2956 回归：物理关闭钮必须**双闸门同步清**（state.visible + mgr）——
    /// 只 `mgr.close` 会留 (visible=true, mgr=closed) 失配态，
    /// 此后 RPC `dialog storage toggle` 永远无法再开窗（只能 open 恢复）。
    #[test]
    fn storage_close_click_clears_both_gates() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = bevy::prelude::World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(
            crate::resources::libraries::Libraries::new(
                crate::resources::libraries::resolve_data_path(),
            ),
        ));
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Image>::default());
        world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        world.init_resource::<super::StorageState>();
        world.init_resource::<crate::game::dialogs::DialogManager>();
        // 预置开态：双闸门都真
        world.resource_mut::<super::StorageState>().visible = true;
        world
            .resource_mut::<crate::game::dialogs::DialogManager>()
            .open(crate::game::dialogs::DialogKind::Storage);
        // 假关闭钮：满足 buttons 查询 (Interaction, Option<StorageClose>) + With<StorageWidget>
        world.spawn((
            super::StorageWidget,
            super::StorageClose,
            bevy::prelude::Interaction::Pressed,
            bevy::prelude::Visibility::Visible,
        ));
        world
            .run_system_once(super::storage_ui_system)
            .expect("storage_ui_system 应运行");
        assert!(
            !world.resource::<super::StorageState>().visible,
            "点 X 后 state.visible 应为 false（否则与 mgr 失配，toggle 永久失效）"
        );
        assert!(
            !world
                .resource::<crate::game::dialogs::DialogManager>()
                .is_open(crate::game::dialogs::DialogKind::Storage),
            "点 X 后 mgr 应为 closed"
        );
    }

    /// 交互 sweep 复现：RPC/服务端打开仓库（`state.visible=true` + `mgr.is_open`）
    /// 后，根必须翻 Visible——否则 `dialog_rect` 永远报「close button not found」。
    #[test]
    fn storage_root_visible_when_gate_true() {
        use bevy::ecs::system::RunSystemOnce;
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip storage_root_visible_when_gate_true: 无 Data 资产");
            return;
        }
        let mut world = bevy::prelude::World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(
            crate::resources::libraries::Libraries::new(
                crate::resources::libraries::resolve_data_path(),
            ),
        ));
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Image>::default());
        world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        world.init_resource::<super::StorageState>();
        world.init_resource::<crate::game::dialogs::DialogManager>();
        let root = world
            .spawn((
                super::StorageWidget,
                crate::game::dialogs::DialogRoot(crate::game::dialogs::DialogKind::Storage),
                bevy::prelude::Visibility::Hidden,
            ))
            .id();
        // 关栈 + visible=true → 仍 Hidden；open + visible=true → Visible
        let gate = |world: &mut bevy::prelude::World, open: bool, visible: bool| {
            {
                let mut mgr = world.resource_mut::<crate::game::dialogs::DialogManager>();
                if open {
                    mgr.open(crate::game::dialogs::DialogKind::Storage);
                } else {
                    mgr.close(crate::game::dialogs::DialogKind::Storage);
                }
            }
            world.resource_mut::<super::StorageState>().visible = visible;
            world
                .run_system_once(super::storage_ui_system)
                .expect("storage_ui_system 应运行")
        };
        let vis = |world: &bevy::prelude::World| {
            *world
                .entity(root)
                .get::<bevy::prelude::Visibility>()
                .unwrap()
        };

        gate(&mut world, false, true);
        assert_eq!(
            vis(&world),
            bevy::prelude::Visibility::Hidden,
            "仅 visible=true 但栈未开 → 应 Hidden"
        );

        gate(&mut world, true, true);
        assert_eq!(
            vis(&world),
            bevy::prelude::Visibility::Visible,
            "RPC open 路径（visible+栈都真）→ 应 Visible"
        );
    }

    /// 回归（交互 sweep `storage closed=NO`）：仓库关闭钮必须满足
    /// `storage_ui_system.buttons` 查询的 `With<StorageWidget>` 过滤——
    /// 修复后 spawn 处同时挂 `(StorageWidget, StorageClose, CloseButton)`，
    /// 否则点击进入不了关闭分支。
    #[test]
    fn storage_close_button_matches_buttons_query() {
        use crate::game::dialogs::storage::StorageClose;
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip: 无 Data 资产");
            return;
        }
        let mut world = bevy::prelude::World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(
            crate::resources::libraries::Libraries::new(
                crate::resources::libraries::resolve_data_path(),
            ),
        ));
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Image>::default());
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiCjkFont::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        use bevy::ecs::system::RunSystemOnce;
        world
            .run_system_once(super::spawn_storage_dialog)
            .expect("spawn_storage_dialog 应成功");

        let mut qd =
            world.query_filtered::<bevy::prelude::Entity, super::With<super::StorageClose>>();
        let close_ents: Vec<bevy::prelude::Entity> = qd.iter(&world).collect();
        assert_eq!(close_ents.len(), 1, "spawn 应恰有一个仓库关闭钮");
        for e in close_ents {
            assert!(
                world.entity(e).contains::<super::StorageWidget>(),
                "StorageClose（{e:?}）必须带 StorageWidget，否则 buttons 查询永不命中"
            );
        }
    }

    /// #2825 单元①：C# `StorageDialog` 未设 `Movable`（`NPCDialogs.cs:2798` → 默认 false）→
    /// 面板/密码面板/解锁面板/格子等**所有** `DialogRoot(DialogKind::Storage)` 都要 `NotDraggable`
    #[test]
    fn storage_roots_are_not_draggable() {
        use crate::game::dialogs::{DialogKind, DialogRoot, NotDraggable};
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        // CI 无游戏资产（Data/ 不入库）→ 跳过（详见 libraries::data_assets_present）
        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip storage_roots_are_not_draggable: 无 Data 资产");
            return;
        }
        let mut world = bevy::prelude::World::new();
        world.insert_resource(crate::map_renderer::GameLibraries(Libraries::new(
            resolve_data_path(),
        )));
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Image>::default());
        world.insert_resource(bevy::prelude::Assets::<bevy::prelude::Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiCjkFont::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        world
            .run_system_once(super::spawn_storage_dialog)
            .expect("spawn_storage_dialog 应成功");

        let mut q = world.query::<(bevy::prelude::Entity, &DialogRoot)>();
        let roots: Vec<bevy::prelude::Entity> = q
            .iter(&world)
            .filter(|(_, r)| r.0 == DialogKind::Storage)
            .map(|(e, _)| e)
            .collect();
        assert!(!roots.is_empty(), "应生成仓库根面板");
        for e in roots {
            assert!(
                world.entity(e).contains::<NotDraggable>(),
                "Storage 根 {e:?} 缺 NotDraggable（C# Movable = false）"
            );
        }
        crate::game::dialogs::test_support::assert_no_drag_start(
            &mut world,
            bevy::math::Vec2::new(
                super::DIALOG_X + super::STORAGE_W / 2.0,
                super::DIALOG_Y + 173.0,
            ),
        );
    }

    /// 仓库格命中：初始原点等价于原固定坐标，拖动后跟随面板
    #[test]
    fn slot_at_origin_and_drag() {
        // 初始 (0,0)：首格 (9,60)，格 36x32
        assert_eq!(storage_slot_at(10.0, 61.0, 0, 80, 0.0, 0.0), Some(0));
        assert_eq!(storage_slot_at(8.0, 61.0, 0, 80, 0.0, 0.0), None);
        // 拖动到 (393,50)：首格绝对坐标 (402,110)
        assert_eq!(storage_slot_at(403.0, 111.0, 0, 80, 393.0, 50.0), Some(0));
        assert_eq!(storage_slot_at(401.0, 111.0, 0, 80, 393.0, 50.0), None);
        // 初始位不再命中（面板已移走）
        assert_eq!(storage_slot_at(10.0, 61.0, 0, 80, 393.0, 50.0), None);
        // 第 2 页：页内首格仍是 (9,60)，但返回**真实槽位** 80；未放行时整页为空
        assert_eq!(storage_slot_at(10.0, 61.0, 80, 160, 0.0, 0.0), Some(80));
        assert_eq!(storage_slot_at(10.0, 61.0, 80, 80, 0.0, 0.0), None);
    }

    use super::*;

    /// C# StorageDialog `Location = new Point(0, 0)`（NPCDialogs.cs:2807）→ 左上角原点。
    /// 旧值 (600,60) 是移植期自定右置。
    #[test]
    fn storage_origin_matches_csharp() {
        assert_eq!(DIALOG_X, 0.0);
        assert_eq!(DIALOG_Y, 0.0);
        // 格网起点 (9,60)、步进 (37,33)（C# x*36+9+x, y%8*32+60+y%8，NPCDialogs.cs:2945）
        assert_eq!(DIALOG_X + 9.0 + 0.0 * (CELL_W + 1.0), 9.0);
        assert_eq!(DIALOG_Y + 60.0 + 0.0 * (CELL_H + 1.0), 60.0);
        assert_eq!(DIALOG_X + 9.0 + 9.0 * (CELL_W + 1.0), 342.0);
        assert_eq!(DIALOG_Y + 60.0 + 7.0 * (CELL_H + 1.0), 291.0);
        // 页内坐标：第 2 页页内首格仍是 (9,60)，末格 (342,291)（C# `y%8` 回绕）
        assert_eq!(cell_pos(0, 0), (9.0, 60.0));
        assert_eq!(cell_pos(9, 0), (342.0, 60.0));
        assert_eq!(cell_pos(79, 0), (342.0, 291.0));
        assert_eq!(cell_pos(80, 80), (9.0, 60.0));
        assert_eq!(cell_pos(159, 80), (342.0, 291.0));
    }

    /// #2892 批B：C# `StorageDialog.RefreshStorage1/2` 的格子 `Visible` 规则——
    /// 第 1 页恒显示 `ItemSlot < StorageGridSize`；第 2 页只在**扩容中**显示 `>= 80`。
    ///
    /// 阳性对照：把 `page_range` 的第 2 页改成无视 `has_expanded_storage`（修正前本端
    /// 根本没有第 2 页、上限夹在 80）→ 本测试 FAILED。
    #[test]
    fn storage_page_range_matches_csharp_visibility_rules() {
        let mut st = StorageState::default();
        st.resize(160);
        assert_eq!(st.items.len(), 160, "扩容后应有 160 格");
        // 扩容中
        st.has_expanded_storage = true;
        assert_eq!(st.page_range(), (0, 80), "第 1 页 0..80");
        st.page = StoragePage::Two;
        assert_eq!(st.page_range(), (80, 160), "第 2 页 80..160（扩容中）");
        // 扩容过期：第 2 页整页隐藏（C# `!HasExpandedStorage` 分支）
        st.has_expanded_storage = false;
        assert_eq!(st.page_range(), (80, 80), "扩容过期 → 第 2 页无可见格");
        // 未扩容（只有 80 格）
        let mut st2 = StorageState::default();
        st2.resize(80);
        st2.page = StoragePage::Two;
        assert_eq!(st2.page_range(), (80, 80));
        st2.page = StoragePage::One;
        assert_eq!(st2.page_range(), (0, 80));
    }

    /// #2892 批B：`ResizeStorage` 上限从 80 放宽到 160（C# `Grid = new MirItemCell[10 * 16]`）。
    #[test]
    fn storage_resize_allows_expanded_160() {
        let mut st = StorageState::default();
        st.resize(160);
        assert_eq!(st.items.len(), 160);
        st.resize(999);
        assert_eq!(st.items.len(), MAX_CELLS, "上限 = 160（10×16）");
        st.resize(80);
        assert_eq!(st.items.len(), 80, "缩容仍可截断");
    }

    /// #2892 批B：真实 spawn + 真实 `storage_page_system`，断言子控件层按 C# 落位与分页门控：
    /// 第 1 页租用钮/遮罩/提示行全隐；第 2 页未扩容 → 租用钮 + 遮罩 + 红字「扩展仓库已锁定」；
    /// 第 2 页已扩容 → 遮罩隐藏、提示行白字「扩展仓库到期时间…」。
    ///
    /// 阳性对照：把 `RefreshStorage2` 的门控写成「第 2 页恒显示租用钮」（即忽略
    /// `has_expanded_storage`，修正前本端根本没有第 2 页）→ 本测试断言「第 1 页租用钮必须 Hidden」时 FAILED。
    #[test]
    fn storage_chrome_pages_gate_like_csharp() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        if !crate::resources::libraries::data_assets_present() {
            eprintln!("skip storage_chrome_pages_gate_like_csharp: 无 Data 资产");
            return;
        }
        let mut world = World::new();
        world.insert_resource(GameLibraries(Libraries::new(resolve_data_path())));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(crate::ui::sprite_ui::UiFont::default());
        world.insert_resource(crate::ui::sprite_ui::UiCjkFont::default());
        world.insert_resource(NetConnection::default());
        let mut mgr = DialogManager::default();
        mgr.open.push(DialogKind::Storage);
        world.insert_resource(mgr);
        world
            .run_system_once(spawn_storage_dialog)
            .expect("spawn_storage_dialog 应成功");

        // 子控件齐全（各 1 个）
        for (name, n) in [
            (
                "第1页页码钮",
                world
                    .query_filtered::<Entity, With<StoragePage1Tab>>()
                    .iter(&world)
                    .count(),
            ),
            (
                "第2页页码钮",
                world
                    .query_filtered::<Entity, With<StoragePage2Tab>>()
                    .iter(&world)
                    .count(),
            ),
            (
                "租用扩容钮",
                world
                    .query_filtered::<Entity, With<StorageRentBtn>>()
                    .iter(&world)
                    .count(),
            ),
            (
                "未扩容遮罩",
                world
                    .query_filtered::<Entity, With<StorageLockedPage>>()
                    .iter(&world)
                    .count(),
            ),
            (
                "提示行",
                world
                    .query_filtered::<Entity, With<StorageRentalLabel>>()
                    .iter(&world)
                    .count(),
            ),
            (
                "租用确认框",
                world
                    .query_filtered::<Entity, With<StorageRentConfirm>>()
                    .iter(&world)
                    .count(),
            ),
        ] {
            assert_eq!(n, 1, "{} 应恰好 1 个", name);
        }

        fn visibility_of<T: bevy::ecs::component::Component>(world: &mut World) -> Visibility {
            world
                .query_filtered::<&Visibility, With<T>>()
                .iter(world)
                .next()
                .copied()
                .expect("实体应存在")
        }
        fn label_of<T: bevy::ecs::component::Component>(world: &mut World) -> (String, Color) {
            let (text, color) = world
                .query_filtered::<(&Text, &TextColor), With<T>>()
                .iter(world)
                .next()
                .map(|(t, c)| (t.0.clone(), c.0))
                .expect("实体应存在");
            (text, color)
        }

        let mut state = StorageState {
            visible: true,
            ..Default::default()
        };
        state.resize(160);

        // ---- 第 1 页：租用钮/遮罩/提示行全隐 ----
        state.page = StoragePage::One;
        state.has_expanded_storage = true;
        world.insert_resource(state);
        world
            .run_system_once(storage_page_system)
            .expect("storage_page_system 应成功");
        assert_eq!(
            visibility_of::<StorageRentBtn>(&mut world),
            Visibility::Hidden,
            "第 1 页不显示租用钮（C# `RefreshStorage1`）"
        );
        assert_eq!(
            visibility_of::<StorageLockedPage>(&mut world),
            Visibility::Hidden,
            "第 1 页不显示遮罩"
        );
        assert_eq!(
            label_of::<StorageRentalLabel>(&mut world).0,
            "",
            "第 1 页提示行为空"
        );

        // ---- 第 2 页 + 未扩容：租用钮 + 遮罩 + 红字 ----
        {
            let mut s = world.resource_mut::<StorageState>();
            s.page = StoragePage::Two;
            s.has_expanded_storage = false;
        }
        world
            .run_system_once(storage_page_system)
            .expect("storage_page_system 应成功");
        assert_eq!(
            visibility_of::<StorageRentBtn>(&mut world),
            Visibility::Visible,
            "第 2 页显示租用钮"
        );
        assert_eq!(
            visibility_of::<StorageLockedPage>(&mut world),
            Visibility::Visible,
            "第 2 页未扩容时显示遮罩"
        );
        let (text, color) = label_of::<StorageRentalLabel>(&mut world);
        assert_eq!(text, TEXT_EXPANDED_LOCKED, "未扩容提示文案");
        assert_eq!(color, Color::srgb(1.0, 0.0, 0.0), "未扩容提示为红字");

        // ---- 第 2 页 + 已扩容：遮罩隐藏、白字到期时间 ----
        {
            let mut s = world.resource_mut::<StorageState>();
            s.has_expanded_storage = true;
            s.expiry_time = 12345;
        }
        world
            .run_system_once(storage_page_system)
            .expect("storage_page_system 应成功");
        assert_eq!(
            visibility_of::<StorageLockedPage>(&mut world),
            Visibility::Hidden,
            "扩容中不显示遮罩"
        );
        let (text, color) = label_of::<StorageRentalLabel>(&mut world);
        assert_eq!(text, format!("{}{}", TEXT_EXPANDED_EXPIRES_ON, 12345));
        assert_eq!(color, Color::WHITE, "扩容中提示为白字");
    }

    /// 格子与面板同为根节点（GlobalZIndex 参与根排序）：格子必须高于面板
    /// （否则被面板背景盖住），且低于密码/解锁覆盖层 45/46（覆盖层应罩住格子）
    #[test]
    fn storage_cell_z_above_panel() {
        assert!(
            STORAGE_CELL_Z > STORAGE_PANEL_Z,
            "格子 z({STORAGE_CELL_Z}) 必须高于面板 z({STORAGE_PANEL_Z})"
        );
        assert!(STORAGE_CELL_Z < 45, "格子应低于密码/解锁覆盖层(45/46)");
    }

    fn mk_item(uid: u64) -> InvItem {
        InvItem {
            unique_id: uid,
            ..Default::default()
        }
    }

    /// #2747+：C# `MirItemCell` 存入/取出的目标格选择 —— 点击格空则用它，否则取首个空格；
    /// 全满返回 None（`MirItemCell.cs:1360-1379` / :1069-1090）。
    #[test]
    fn store_target_slot_matches_csharp() {
        let items = vec![Some(1u8), None, Some(3)];
        // 点击格为空 → 用它
        assert_eq!(store_target_slot(&items, 1), Some(1));
        // 点击格被占用 → 首个空格
        assert_eq!(store_target_slot(&items, 0), Some(1));
        // 越界点击视同占用 → 首个空格
        assert_eq!(store_target_slot(&items, 99), Some(1));
        // 全满 → None
        let full = vec![Some(1u8), Some(2)];
        assert_eq!(store_target_slot(&full, 0), None);
    }

    /// #2747+：仓库存入回包（`S.StoreItem`）按 C# `GameScene.StoreItem` 解锁来源/目标两格
    /// —— 覆盖 `InvLockReason::Storage` 在 `LockGrid::Storage` 与 `LockGrid::Inventory` 两侧的锁。
    #[test]
    fn store_receipt_releases_grid_locks() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        app.update();
        {
            let mut locked = app.world_mut().resource_mut::<InvLockedSlots>();
            locked.lock_in(InvLockReason::Storage, LockGrid::Storage, 2);
            locked.lock_in(InvLockReason::Storage, LockGrid::Inventory, 7);
            // 其它来源不受影响
            locked.lock_in(InvLockReason::Craft, LockGrid::Inventory, 1);
        }
        app.world_mut().write_message(ServerEvent::ItemStored {
            from: 7,
            to: 2,
            success: false,
        });
        app.update();
        let locked = app.world().resource::<InvLockedSlots>();
        assert!(!locked.is_locked_in(LockGrid::Storage, 2));
        assert!(!locked.is_locked_in(LockGrid::Inventory, 7));
        assert!(
            locked.is_locked_in(LockGrid::Inventory, 1),
            "Craft 来源的锁不受仓储回包影响"
        );
    }

    fn storage_test_app() -> App {
        use crate::network::server_event::ServerEvent;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ServerEvent>();
        app.init_resource::<StorageState>();
        app.init_resource::<DialogManager>();
        // #2747+：storage_server_events 回包解锁仓储锁 → 需该资源
        app.init_resource::<InvLockedSlots>();
        // P3-3（#782）：storage_server_events 现在还会按需发 `RequestItemInfo` → 需该资源
        app.insert_resource(NetConnection::default());
        app.insert_resource(crate::game::dialogs::inventory::InventoryOrigin(0.0, 0.0));
        // #3258：storage_server_events 现在还要驱动密码流程（弹输入框 / 系统提示）
        app.init_resource::<StoragePwdFlow>();
        app.init_resource::<crate::game::dialogs::input_box::InputBoxState>();
        app.init_resource::<crate::game::dialogs::text_input::TextInputState>();
        app.init_resource::<crate::game::chat::ChatState>();
        app.add_systems(Update, storage_server_events);
        app
    }

    fn inv_items(app: &mut App) -> Vec<Option<u64>> {
        app.world_mut()
            .query_filtered::<&Inventory, With<LocalPlayer>>()
            .iter(app.world())
            .next()
            .map(|inv| {
                inv.items
                    .iter()
                    .map(|s| s.as_ref().map(|it| it.unique_id))
                    .collect()
            })
            .expect("LocalPlayer 应有 Inventory")
    }

    /// 存入仓库成功后背包物品直接从 `Inventory` 组件移除（#2633 批次4 步9 直写组件，R1 实体缺失跳过）。
    #[test]
    fn item_stored_removes_from_component() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        app.world_mut().spawn((
            LocalPlayer,
            Inventory {
                items: vec![Some(mk_item(31)), None],
                ..Default::default()
            },
        ));
        {
            let mut storage = app.world_mut().resource_mut::<StorageState>();
            storage.items = vec![None, None];
        }
        app.update(); // 初始化消息缓冲/系统状态

        // 背包格 0 (uid=31) 存入仓库格 0 → Inventory 组件背包格 0 清空
        app.world_mut().write_message(ServerEvent::ItemStored {
            from: 0,
            to: 0,
            success: true,
        });
        app.update();
        let storage = app.world().resource::<StorageState>();
        assert!(
            storage.items[0].as_ref().map(|it| it.unique_id) == Some(31),
            "仓库格 0 应收下 uid=31"
        );
        assert_eq!(
            inv_items(&mut app),
            vec![None, None],
            "Inventory 组件背包格 0 应被存入移除"
        );
    }

    /// 从仓库取回成功后背包物品直接写入 `Inventory` 组件（#2633 批次4 步9 直写组件，R1 实体缺失跳过）。
    #[test]
    fn item_taken_back_writes_component() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        app.world_mut().spawn((
            LocalPlayer,
            Inventory {
                items: vec![None, None],
                ..Default::default()
            },
        ));
        {
            let mut storage = app.world_mut().resource_mut::<StorageState>();
            storage.items = vec![Some(mk_item(47)), None];
        }
        app.update();

        // 仓库格 0 (uid=47) 取回背包格 1 → Inventory 组件背包格 1 收下
        app.world_mut().write_message(ServerEvent::ItemTakenBack {
            from: 0,
            to: 1,
            success: true,
        });
        app.update();
        let storage = app.world().resource::<StorageState>();
        assert!(storage.items[0].is_none(), "仓库格 0 应被取回清空");
        assert_eq!(
            inv_items(&mut app),
            vec![None, Some(47)],
            "Inventory 组件背包格 1 应收下 uid=47"
        );
    }

    /// #2960 回归：服务端 `StorageOpened{visible:false}` 必须**双闸门同步关**
    /// （state.visible=false 且 mgr 未 open Storage）——无条件 `mgr.open.push(Storage)`
    /// 会留 (visible=false, mgr=open) 失配态，此后 RPC `dialog storage toggle`
    /// 在 (false,open)↔(true,closed) 间振荡、永远到不了 (true,true)，窗口永久锁死
    /// （与 #2956 项 2 同族）。
    #[test]
    fn storage_server_event_invisible_closes_both_gates() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        // 预置开态：双闸门都真（仓库已打开）
        app.world_mut().resource_mut::<StorageState>().visible = true;
        app.world_mut()
            .resource_mut::<DialogManager>()
            .open(DialogKind::Storage);
        app.update(); // 初始化消息缓冲/系统状态

        app.world_mut().write_message(ServerEvent::StorageOpened {
            items: vec![None, None],
            visible: false,
        });
        app.update();

        assert!(
            !app.world().resource::<StorageState>().visible,
            "visible=false 事件后 state.visible 应为 false"
        );
        assert!(
            !app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::Storage),
            "visible=false 事件后 mgr 不应再 open Storage（否则双闸门失配，toggle 永久锁死）"
        );
    }

    /// #2960 配对守护：`visible:true` 事件仍须双闸门同开
    /// （state.visible=true 且 mgr open Storage）——分流修复不得破坏打开路径。
    #[test]
    fn storage_server_event_visible_opens_both_gates() {
        use crate::network::server_event::ServerEvent;
        let mut app = storage_test_app();
        app.update();

        app.world_mut().write_message(ServerEvent::StorageOpened {
            items: vec![None, None],
            visible: true,
        });
        app.update();

        assert!(
            app.world().resource::<StorageState>().visible,
            "visible=true 事件后 state.visible 应为 true"
        );
        assert!(
            app.world()
                .resource::<DialogManager>()
                .is_open(DialogKind::Storage),
            "visible=true 事件后 mgr 应 open Storage"
        );
    }

    // ========================================================================
    // #3258 仓库密码流程门禁（对齐 C# `NPCDialogs.cs:3085-3239`）
    // ========================================================================

    fn flow() -> super::StoragePwdFlow {
        super::StoragePwdFlow::default()
    }

    /// 设置流程：new → confirm → `C.SetStoragePassword{"", new}`；
    /// 两次不一致只提示、不前进（C# `:3138-3155`）。
    #[test]
    fn set_password_flow_matches_csharp() {
        let mut st = super::StorageState::default();
        let mut f = flow();
        f.step = super::StoragePwdStep::SetNew;

        let r = super::storage_password_step("", &mut st, &mut f);
        assert!(r.keep_open && r.packet.is_none(), "空输入：C# `return false`（原地）");

        let r = super::storage_password_step("abc123", &mut st, &mut f);
        assert!(r.keep_open, "第一步成功后仍要问确认（输入框保持打开）");
        let (next_step, next_prompt) = r.next.clone().expect("应给出下一问");
        assert_eq!(next_prompt, super::TEXT_PWD_CONFIRM);
        f.step = next_step; // 真实调用方（input_box）就是这么接的

        // 不一致 → 提示 + 不前进（非强制路径）
        let r = super::storage_password_step("xyz999", &mut st, &mut f);
        assert!(r.keep_open && r.packet.is_none());
        assert_eq!(r.message.as_deref(), Some(super::TEXT_PWD_MISMATCH));
        assert!(!r.restart_set, "非强制路径不重来（C# 只在 force 时重来）");

        // 一致 → 发包（current 为空串，C# `:3152`）
        let r = super::storage_password_step("abc123", &mut st, &mut f);
        assert!(!r.keep_open, "发完包就该关输入框");
        assert_eq!(
            r.packet,
            Some(super::PwdPacket::Set {
                current: String::new(),
                new: "abc123".to_string()
            })
        );
        assert_eq!(f.step, super::StoragePwdStep::None);
    }

    /// 强制设密码路径下两次不一致 → 从头重来（C# `:3145-3147`）。
    #[test]
    fn forced_set_restarts_on_mismatch() {
        let mut st = super::StorageState {
            forcing_setup: true,
            ..Default::default()
        };
        let mut f = flow();
        f.step = super::StoragePwdStep::SetConfirm {
            new: "abc123".to_string(),
        };
        let r = super::storage_password_step("nope", &mut st, &mut f);
        assert!(r.restart_set, "force 路径必须重来");
        assert_eq!(r.message.as_deref(), Some(super::TEXT_PWD_MISMATCH));
    }

    /// 改密流程：current → new → confirm → `C.SetStoragePassword{current, new}`（C# `:3159-3189`）。
    #[test]
    fn change_password_flow_matches_csharp() {
        let mut st = super::StorageState::default();
        let mut f = flow();
        f.step = super::StoragePwdStep::ChangeCurrent;

        let r = super::storage_password_step("old123", &mut st, &mut f);
        let (s1, p1) = r.next.clone().expect("应问新密码");
        assert_eq!(p1, super::TEXT_PWD_NEW);
        assert_eq!(
            s1,
            super::StoragePwdStep::ChangeNew {
                current: "old123".to_string()
            }
        );
        f.step = s1;
        let r = super::storage_password_step("new456", &mut st, &mut f);
        let (s2, p2) = r.next.clone().expect("应问确认");
        assert_eq!(p2, super::TEXT_PWD_CONFIRM);
        assert_eq!(
            s2,
            super::StoragePwdStep::ChangeConfirm {
                current: "old123".to_string(),
                new: "new456".to_string()
            }
        );
        f.step = s2;
        let r = super::storage_password_step("new456", &mut st, &mut f);
        assert_eq!(
            r.packet,
            Some(super::PwdPacket::Set {
                current: "old123".to_string(),
                new: "new456".to_string()
            })
        );
        assert!(!r.keep_open);
    }

    /// 解锁：非空才发 `C.UnlockStorage`；空输入原地（C# `:3194-3203` + `:3228`）。
    #[test]
    fn unlock_flow_matches_csharp() {
        let mut st = super::StorageState {
            unlock_prompt_open: true,
            ..Default::default()
        };
        let mut f = flow();
        f.step = super::StoragePwdStep::Unlock;
        let r = super::storage_password_step("", &mut st, &mut f);
        assert!(r.keep_open && r.packet.is_none());
        assert!(st.unlock_prompt_open, "空输入时解锁提示还开着");

        let r = super::storage_password_step("secret", &mut st, &mut f);
        assert_eq!(
            r.packet,
            Some(super::PwdPacket::Unlock {
                password: "secret".to_string()
            })
        );
        assert!(!st.unlock_prompt_open, "发出解锁包后提示关闭");
    }

    /// 取消语义：解锁/首次设置取消要**连仓库窗一起关**（C# `() => Hide()` / `CancelStoragePasswordSetup`），
    /// 改密路径取消只关输入框（C# 不传 onCancel）。
    #[test]
    fn cancel_semantics_match_csharp() {
        let mut st = super::StorageState {
            visible: true,
            unlocked: true,
            unlock_prompt_open: true,
            ..Default::default()
        };
        let mut f = flow();
        f.cancel_hides_storage = true;
        super::storage_password_cancel(&mut st, &mut f);
        assert!(!st.visible && st.close_requested && !st.unlocked);
        assert_eq!(f.step, super::StoragePwdStep::None);

        let mut st2 = super::StorageState {
            visible: true,
            unlocked: true,
            ..Default::default()
        };
        let mut f2 = flow();
        f2.cancel_hides_storage = false;
        super::storage_password_cancel(&mut st2, &mut f2);
        assert!(st2.visible && !st2.close_requested, "改密取消不应关仓库窗");
        assert!(st2.unlocked, "改密取消不改解锁态");
    }

    /// `ManageStoragePassword` 的三个分支（C# `:3085-3107`）：
    /// 未启用密码 → 什么都不做；未设过 → 直接进设置流程；已设过 → 弹 OKCancel 确认框。
    #[test]
    fn manage_storage_password_branches() {
        use crate::game::dialogs::input_box::InputBoxState;
        use crate::game::dialogs::text_input::TextInputState;

        // 未启用（服务端 RequireStoragePassword=false）
        let mut st = super::StorageState {
            require_password: false,
            ..Default::default()
        };
        let (mut ib, mut input) = (InputBoxState::default(), TextInputState::default());
        let mut f = flow();
        super::manage_storage_password(&mut ib, &mut input, &mut st, &mut f);
        assert!(!ib.open && !st.change_confirm, "未启用密码时不该弹任何东西");

        // 启用但未设过 → 设置流程（输入框打开、走密码遮罩）
        let mut st = super::StorageState {
            require_password: true,
            has_password: false,
            ..Default::default()
        };
        let (mut ib, mut input) = (InputBoxState::default(), TextInputState::default());
        let mut f = flow();
        super::manage_storage_password(&mut ib, &mut input, &mut st, &mut f);
        assert!(ib.open, "未设密码时必须弹输入框");
        assert_eq!(f.step, super::StoragePwdStep::SetNew);
        assert_eq!(ib.purpose, crate::game::dialogs::input_box::InputPurpose::StoragePassword);
        assert!(
            input.masked.contains(&crate::game::dialogs::input_box::INPUT_FIELD_ID),
            "C# `InputTextBox.Password = true` ⇒ 必须遮罩"
        );
        assert!(f.cancel_hides_storage, "首次设置取消要连窗一起关");

        // 已设过 → 先确认框（C# `MirMessageBox(changePrompt, OKCancel)`）
        let mut st = super::StorageState {
            require_password: true,
            has_password: true,
            ..Default::default()
        };
        let (mut ib, mut input) = (InputBoxState::default(), TextInputState::default());
        let mut f = flow();
        super::manage_storage_password(&mut ib, &mut input, &mut st, &mut f);
        assert!(st.change_confirm && !ib.open, "已设过先弹确认框、不直接问密码");
    }

    /// 源码级守卫（owner 反馈项）：密码流程**不得**再回到「自造三钮面板 + 在烘字按钮上叠中文标签」。
    /// 阳性对照：把「在按钮美术上叠一颗 `设置` 标签」那行加回来 → 本测试立即红。
    #[test]
    fn storage_password_ui_has_no_bespoke_buttons_or_stacked_labels() {
        let src = include_str!("storage.rs");
        // 名字**拼**出来而不是写全：否则 `include_str!` 会扫到本测试自己的字面量，永远红。
        let banned: Vec<String> = [
            ("StoragePwd", "Panel"),
            ("StoragePwd", "Set"),
            ("StoragePwd", "Remove"),
            ("StoragePwd", "Close"),
            ("StorageUnlock", "Panel"),
            ("StorageUnlock", "Ok"),
            ("StorageUnlock", "Cancel"),
        ]
        .iter()
        .map(|(a, b)| format!("{a}{b}"))
        .collect();
        for banned in &banned {
            assert!(
                !src.contains(banned),
                "{banned} 是自造面板的残留；密码流程必须走 MirInputBox + MirMessageBox(OKCancel)"
            );
        }
        for stacked in ["\"设置\"", "\"移除\"", "\"关闭\"", "\"确定\"", "\"取消\""] {
            assert!(
                !src.contains(&format!("spawn_label(p, &cjk, {stacked},")),
                "不得在烘了英文的按钮美术上再叠中文标签（{stacked}）"
            );
        }
        // 确认框必须是 `MirMessageBox(OKCancel)` 的 OK/Cancel 两帧组
        assert!(
            src.contains("StoragePwdChangeOk") && src.contains("StoragePwdChangeCancel"),
            "改密确认框应有 OK/Cancel 两颗"
        );
    }
}
