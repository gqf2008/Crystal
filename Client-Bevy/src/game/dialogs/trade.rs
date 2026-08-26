// ============================================================================
// 交易对话框（M23）
// 布局参考：C# Client/MirScenes/Dialogs/TradeDialogs.cs（双独立窗）
//   - TradeDialog（我方）：Prguse[389] 204x152 @ (ScreenWidth/2-204-10, ScreenHeight-350)
//     · 确认 Title[520/521/522] @(135,120) 48x25；锁定后常显 521 帧（ChangeLockState）
//     · 关闭 Prguse2[360/361/362] @(W-23,3)
//     · 名字标签 @(20,10) 150x14 居中（自己名字）
//     · 金币标签 @(35,123) 90x15 居中，可点击 → 数量框（TradeDialogs.cs:80-100）
//   - GuestTradeDialog（对方）：Prguse[390] 204x152 @ (ScreenWidth/2+10, ScreenHeight-350)
//     · 对方名字 @(0,10) 204x14 居中；对方金币 @(35,123) 90x15 居中（不可点）
//   - 双窗各 5x2 格，列主序 Grid[2*x+y] @(x*36+10+x, y*32+39+y)（TradeDialogs.cs:106-118）
//   - TradeAccept：背包推到屏幕右侧 (ScreenWidth-inv.W, 0)（TradeDialogs.cs:152-161）
//   - 邀请 = MirMessageBox YesNo（Prguse[360] 居中 + Yes Title[206-208] / No Title[210-212]
//     @(260/360,157)，MirMessageBox.cs:76-96；GameScene.cs:6303-6309）
// 交互（原版 C# 语义）：
//   点击背包物品 → C.DepositTradeItem{from=背包格, to=首个空交易槽}；点击我方槽 → 取回
//   金币标签 → 数量框 → C.TradeGold{amount}（累计）；确认 → C.TradeConfirm{locked}
//   关闭 → C.TradeCancel；邀请 Yes/No → C.TradeReply{accept}
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::actor::LocalPlayer;
use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::inventory::{InvItem, InvSlot, InventoryShiftRight};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::game::player_state::Inventory;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::controls::{ItemCellData, spawn_item_cell};
use crate::ui::outlined_text::{OutlineShadow, outline_on};
use crate::ui::sprite_ui::{
    ButtonFrames, UiButton, UiFont, UiImageCache, spawn_ui_text_anchored, ui_button_system,
    ui_image,
};

/// 交易物品（槽内显示用）
#[derive(Debug, Clone)]
pub struct TradeItem {
    pub uid: u64,
    pub item_index: i32,
    pub name: String,
    pub image: u16,
    pub count: u16,
}

impl From<&InvItem> for TradeItem {
    fn from(i: &InvItem) -> Self {
        Self {
            uid: i.unique_id,
            item_index: i.item_index,
            name: i.name.clone(),
            image: i.image,
            count: i.count,
        }
    }
}

// ---------------------------------------------------------------------------
// 布局常量（C# TradeDialogs.cs；1024x768 下的字面值见测试）
// ---------------------------------------------------------------------------

/// 我方窗原点：C# (ScreenWidth/2 - Size.Width - 10, ScreenHeight - 350)（:21）
pub const TRADE_X: f32 = 1024.0 / 2.0 - 204.0 - 10.0; // 298
pub const TRADE_Y: f32 = 768.0 - 350.0; // 418
/// 对方窗原点：C# (ScreenWidth/2 + 10, ScreenHeight - 350)（:211）
pub const GUEST_X: f32 = 1024.0 / 2.0 + 10.0; // 522
pub const GUEST_Y: f32 = 768.0 - 350.0; // 418
/// 双窗尺寸（C# Size = new Size(204, 152)；Prguse[389]/[390] 实测同尺寸）
pub const TRADE_W: f32 = 204.0;
pub const TRADE_H: f32 = 152.0;
/// 确认按钮 Title[520-522] @(135,120) 48x25（:25-35）
pub const CONFIRM_X: f32 = 135.0;
pub const CONFIRM_Y: f32 = 120.0;
pub const CONFIRM_W: f32 = 48.0;
pub const CONFIRM_H: f32 = 25.0;
/// 关闭按钮 @(Size.Width-23, 3)（:46）
pub const CLOSE_DX: f32 = TRADE_W - 23.0; // 181
/// 我方名字标签框 @(20,10) 150x14（:62-69）
pub const NAME_X: f32 = 20.0;
pub const NAME_Y: f32 = 10.0;
pub const NAME_W: f32 = 150.0;
pub const NAME_H: f32 = 14.0;
/// 金币标签框 @(35,123) 90x15（:71-79；对方窗同位）
pub const GOLD_X: f32 = 35.0;
pub const GOLD_Y: f32 = 123.0;
pub const GOLD_W: f32 = 90.0;
pub const GOLD_H: f32 = 15.0;
/// 对方名字标签框 @(0,10) 204x14（:215-222）
pub const GUEST_NAME_X: f32 = 0.0;
/// 格子（C# x*36+10+x, y*32+39+y；MirItemCell 默认 36x32）
pub const CELL_W: f32 = 36.0;
pub const CELL_H: f32 = 32.0;
/// 交易槽位数（C# Grid = new MirItemCell[5*2]；服务端 CharacterInfo.Trade = UserItem[10]）
pub const TRADE_SLOTS: usize = 10;
/// 邀请框：MirMessageBox Prguse[360] 实测 456x190，屏幕中心（C# int 整除居中）
pub const INVITE_W: f32 = 456.0;
pub const INVITE_H: f32 = 190.0;
pub const INVITE_X: f32 = (1024.0 - INVITE_W) / 2.0; // 284
pub const INVITE_Y: f32 = (768.0 - INVITE_H) / 2.0; // 289

/// C# 交易格位置（TradeDialogs.cs:106-118）：Grid[2*x+y] **列主序**
/// （x=列 0..5，y=行 0..2），格子左上 = (x*36+10+x, y*32+39+y)
pub fn trade_slot_pos(slot: usize) -> (f32, f32) {
    let x = (slot / 2) as f32; // 列
    let y = (slot % 2) as f32; // 行
    (x * 36.0 + 10.0 + x, y * 32.0 + 39.0 + y)
}

/// C# 金币格式 "{0:###,###,##0}"（千分位，0 显示 "0"）
pub fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

// ---------------------------------------------------------------------------
// 状态与组件
// ---------------------------------------------------------------------------

/// 交易状态
#[derive(Resource)]
pub struct TradeState {
    pub visible: bool,
    pub partner_name: String,
    /// 待处理邀请（发起者名）
    pub invite: Option<String>,
    /// 是否为交易发起者（TradeConfirm 包 [a][b] 映射用）
    pub is_initiator: bool,
    pub my_items: Vec<Option<TradeItem>>,
    pub their_items: Vec<Option<TradeItem>>,
    pub my_gold: u64,
    pub their_gold: u64,
    pub my_locked: bool,
    pub their_locked: bool,
    /// 待确认的存入操作（from=背包格, to=交易槽）
    pub pending_deposit: Option<(usize, usize)>,
}

impl Default for TradeState {
    fn default() -> Self {
        Self {
            visible: false,
            partner_name: String::new(),
            invite: None,
            is_initiator: false,
            my_items: vec![None; TRADE_SLOTS],
            their_items: vec![None; TRADE_SLOTS],
            my_gold: 0,
            their_gold: 0,
            my_locked: false,
            their_locked: false,
            pending_deposit: None,
        }
    }
}

#[derive(Component)]
pub struct TradeWidget;

#[derive(Component)]
pub struct TradeClose;

/// 确认按钮（C# ConfirmButton；锁定后 normal 帧常显 521 = ChangeLockState）
#[derive(Component)]
pub struct TradeConfirmBtn;

/// 交易文本标签（四枚 MirLabel，默认描边）
#[derive(Clone, Copy, PartialEq)]
pub enum TradeText {
    MyName,
    MyGold,
    GuestName,
    GuestGold,
}

#[derive(Component)]
pub struct TradeLabel(pub TradeText);

/// 我方金币标签命中区（C# GoldLabel 可点击开数量框）
#[derive(Component)]
pub struct TradeGoldHit;

/// 交易物品槽（side 0=自己 1=对方, idx 0..10）
#[derive(Component, Clone, Copy)]
pub struct TradeSlot(pub usize, pub usize);

// 邀请提示（MirMessageBox：模态、不可拖动 → 不挂 DialogRoot）
#[derive(Component)]
pub struct TradeInviteWidget;

#[derive(Component)]
pub struct TradeInviteText;

#[derive(Component)]
pub struct TradeInviteYes;

#[derive(Component)]
pub struct TradeInviteNo;

pub struct TradePlugin;

impl Plugin for TradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TradeState>();
        app.add_systems(Update, trade_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(OnEnter(AppState::Game), spawn_trade);
        app.add_systems(OnExit(AppState::Game), cleanup_trade);
        app.add_systems(
            Update,
            (
                trade_ui_system,
                trade_action_system,
                trade_invite_system,
                ui_button_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_trade(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_trade(
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

    // ---- 我方窗（TradeDialog）----
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 389) {
        let e = crate::ui::sprite_ui::spawn_ui_sprite(&mut commands, h, TRADE_X, TRADE_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Trade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }
    // 确认按钮 Title[520-522] @(135,120)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        520,
        521,
        522,
        TRADE_X + CONFIRM_X,
        TRADE_Y + CONFIRM_Y,
        7.0,
        CONFIRM_W,
        CONFIRM_H,
    ) {
        commands.entity(e).insert((
            TradeConfirmBtn,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }
    // 关闭按钮 @(W-23, 3)
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        TRADE_X + CLOSE_DX,
        TRADE_Y + 3.0,
        7.0,
        24.0,
        21.0,
    ) {
        commands.entity(e).insert((
            TradeClose,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }
    // 名字标签（框内居中，MirLabel 默认白字描边）
    let name = spawn_ui_text_anchored(
        &mut commands,
        &font,
        "",
        Anchor::CENTER,
        TRADE_X + NAME_X + NAME_W / 2.0,
        TRADE_Y + NAME_Y + NAME_H / 2.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(name).insert((
        TradeLabel(TradeText::MyName),
        DialogRoot(DialogKind::Trade),
        TradeWidget,
        Visibility::Hidden,
    ));
    outline_on(
        &mut commands,
        name,
        "",
        font.clone(),
        12.0,
        Anchor::CENTER,
        false,
    );
    // 金币标签（框内居中；可点击开数量框）
    let gold = spawn_ui_text_anchored(
        &mut commands,
        &font,
        "",
        Anchor::CENTER,
        TRADE_X + GOLD_X + GOLD_W / 2.0,
        TRADE_Y + GOLD_Y + GOLD_H / 2.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(gold).insert((
        TradeLabel(TradeText::MyGold),
        TradeGoldHit,
        DialogRoot(DialogKind::Trade),
        TradeWidget,
        Visibility::Hidden,
    ));
    outline_on(
        &mut commands,
        gold,
        "",
        font.clone(),
        12.0,
        Anchor::CENTER,
        false,
    );
    // 我方 5x2 格（列主序）
    for i in 0..TRADE_SLOTS {
        let (sx, sy) = trade_slot_pos(i);
        let cell = spawn_item_cell(
            &mut commands,
            &mut images,
            &font,
            TRADE_X + sx,
            TRADE_Y + sy,
            6.3,
            CELL_W,
            CELL_H,
            i,
        );
        commands.entity(cell).insert((
            TradeSlot(0, i),
            DialogRoot(DialogKind::Trade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }

    // ---- 对方窗（GuestTradeDialog）----
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 390) {
        let e = crate::ui::sprite_ui::spawn_ui_sprite(&mut commands, h, GUEST_X, GUEST_Y, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::GuestTrade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }
    let gname = spawn_ui_text_anchored(
        &mut commands,
        &font,
        "",
        Anchor::CENTER,
        GUEST_X + GUEST_NAME_X + TRADE_W / 2.0,
        GUEST_Y + NAME_Y + NAME_H / 2.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(gname).insert((
        TradeLabel(TradeText::GuestName),
        DialogRoot(DialogKind::GuestTrade),
        TradeWidget,
        Visibility::Hidden,
    ));
    outline_on(
        &mut commands,
        gname,
        "",
        font.clone(),
        12.0,
        Anchor::CENTER,
        false,
    );
    let ggold = spawn_ui_text_anchored(
        &mut commands,
        &font,
        "",
        Anchor::CENTER,
        GUEST_X + GOLD_X + GOLD_W / 2.0,
        GUEST_Y + GOLD_Y + GOLD_H / 2.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(ggold).insert((
        TradeLabel(TradeText::GuestGold),
        DialogRoot(DialogKind::GuestTrade),
        TradeWidget,
        Visibility::Hidden,
    ));
    outline_on(
        &mut commands,
        ggold,
        "",
        font.clone(),
        12.0,
        Anchor::CENTER,
        false,
    );
    for i in 0..TRADE_SLOTS {
        let (sx, sy) = trade_slot_pos(i);
        let cell = spawn_item_cell(
            &mut commands,
            &mut images,
            &font,
            GUEST_X + sx,
            GUEST_Y + sy,
            6.3,
            CELL_W,
            CELL_H,
            i,
        );
        commands.entity(cell).insert((
            TradeSlot(1, i),
            DialogRoot(DialogKind::GuestTrade),
            TradeWidget,
            Visibility::Hidden,
        ));
    }

    // ---- 邀请框（MirMessageBox YesNo；模态不可拖 → 无 DialogRoot）----
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e =
            crate::ui::sprite_ui::spawn_ui_sprite(&mut commands, h, INVITE_X, INVITE_Y, 9.5, 1.0);
        commands
            .entity(e)
            .insert((TradeInviteWidget, Visibility::Hidden));
    }
    let it = crate::ui::sprite_ui::spawn_ui_text(
        &mut commands,
        &font,
        "",
        INVITE_X + 35.0,
        INVITE_Y + 35.0,
        12.0,
        Color::WHITE,
        9.6,
    );
    commands
        .entity(it)
        .insert((TradeInviteText, TradeInviteWidget, Visibility::Hidden));
    outline_on(
        &mut commands,
        it,
        "",
        font.clone(),
        12.0,
        Anchor::TOP_LEFT,
        false,
    );
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        206,
        207,
        208,
        INVITE_X + 260.0,
        INVITE_Y + 157.0,
        9.7,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((TradeInviteYes, TradeInviteWidget, Visibility::Hidden));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        210,
        211,
        212,
        INVITE_X + 360.0,
        INVITE_Y + 157.0,
        9.7,
        76.0,
        25.0,
    ) {
        commands
            .entity(e)
            .insert((TradeInviteNo, TradeInviteWidget, Visibility::Hidden));
    }
}

/// C# TradeDialog.TradeAccept（TradeDialogs.cs:152-161）：交易开窗把背包推到屏幕右侧。
/// #2631：不再由本模块直接改写背包实体/原点（跨域直写已解耦）——交易开窗时发
/// [`InventoryShiftRight`]，背包自己的 `inventory_shift_right_system` 响应并自我重排。
/// C# TradeDialog.TradeReset（TradeDialogs.cs:163-177）：清双方物品/金币并解锁
fn trade_reset(trade: &mut TradeState) {
    trade.my_items = vec![None; TRADE_SLOTS];
    trade.their_items = vec![None; TRADE_SLOTS];
    trade.my_gold = 0;
    trade.their_gold = 0;
    trade.my_locked = false;
    trade.their_locked = false;
    trade.pending_deposit = None;
}

/// 显隐 + 槽位物品渲染 + 文本/锁定状态 + 邀请提示 + 开窗推背包
#[allow(clippy::type_complexity)]
fn trade_ui_system(
    trade: Res<TradeState>,
    // #2633 批次4 步7：MyName 改读 `PlayerName`（hud.name 双写保留，步9 删）
    name_q: Query<&crate::actor::PlayerName, With<crate::actor::LocalPlayer>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    // 显隐写方 1：双窗全部成员（邀请框独立、Without<TradeWidget> 可证互斥）
    mut widgets: Query<&mut Visibility, With<TradeWidget>>,
    mut cells: Query<(&mut ItemCellData, &TradeSlot)>,
    // Text2d 写方×3：labels / shadows / invite_text —— 两侧 Without 互斥（B0001）
    mut labels: Query<(&mut Text2d, &TradeLabel, Option<&Children>)>,
    mut shadows: Query<
        &mut Text2d,
        (
            With<OutlineShadow>,
            Without<TradeLabel>,
            Without<TradeInviteText>,
        ),
    >,
    mut invite_texts: Query<
        (&mut Text2d, &TradeInviteText, Option<&Children>),
        (Without<TradeLabel>, Without<OutlineShadow>),
    >,
    // 显隐写方 2（With<TradeInviteWidget> 且无 TradeWidget，与 widgets 互斥）
    mut invite_widgets: Query<&mut Visibility, (With<TradeInviteWidget>, Without<TradeWidget>)>,
    // 锁定后确认钮 normal 帧常显 521（C# ChangeLockState:128-132）
    mut confirm: Query<&mut ButtonFrames, With<TradeConfirmBtn>>,
    // 开窗瞬间通知背包右移让位（#2631：背包实体/Origin 归 inventory 所有，这里只发 Message）
    mut shift_right: MessageWriter<InventoryShiftRight>,
    mut was_visible: Local<bool>,
) {
    for mut vis in &mut widgets {
        *vis = if trade.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 开窗瞬间推背包（C# TradeAccept；服务器开窗与本地邀请接受两条路都汇聚于此）
    // #2631：解耦为发 InventoryShiftRight，由背包自我重排（可见行为不变：交易开时背包右移）
    if trade.visible && !*was_visible {
        shift_right.write(InventoryShiftRight);
    }
    *was_visible = trade.visible;

    // 物品图标 + 数量（#106 通用 ItemCell：只写数据，渲染由 item_cell_system 处理）
    for (mut data, slot) in &mut cells {
        let item = if slot.0 == 0 {
            trade.my_items.get(slot.1).and_then(|s| s.as_ref())
        } else {
            trade.their_items.get(slot.1).and_then(|s| s.as_ref())
        };
        let icon = item.and_then(|it| {
            ui_image(
                &mut libs,
                &mut images,
                &mut cache,
                LibraryName::Items,
                it.image as usize,
            )
        });
        let count = item.map(|it| it.count.max(1) as u32);
        // 性能（#112）：无变化不写
        if data.icon.as_ref() != icon.as_ref() {
            data.icon = icon;
        }
        if data.count != count {
            data.count = count;
        }
    }

    // 四枚标签（C# RefreshInterface:142-150）+ 描边副本直同步
    // （sync_outline_system 排序不可控，同帧晚写会陈旧——同行会名改名方案）
    // 实体缺失默认空串，同原 hud.name 默认
    let my_name = name_q.single().map(|n| n.0.clone()).unwrap_or_default();
    let new_texts = [
        (TradeText::MyName, my_name),
        (TradeText::MyGold, format_thousands(trade.my_gold)),
        (TradeText::GuestName, trade.partner_name.clone()),
        (TradeText::GuestGold, format_thousands(trade.their_gold)),
    ];
    for (mut t, label, children) in &mut labels {
        let new = new_texts
            .iter()
            .find(|(k, _)| *k == label.0)
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        if t.0 != new {
            t.0 = new.clone();
        }
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut s) = shadows.get_mut(child) {
                    if s.0 != new {
                        s.0 = new.clone();
                    }
                }
            }
        }
    }
    // 邀请文本（"{name} 想与你交易"）
    let invite_new = trade
        .invite
        .as_ref()
        .map(|name| format!("{} 想与你交易", name))
        .unwrap_or_default();
    for (mut t, _, children) in &mut invite_texts {
        if t.0 != invite_new {
            t.0 = invite_new.clone();
        }
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut s) = shadows.get_mut(child) {
                    if s.0 != invite_new {
                        s.0 = invite_new.clone();
                    }
                }
            }
        }
    }

    // 邀请框显隐
    let has_invite = trade.invite.is_some();
    for mut vis in &mut invite_widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 锁定 → 确认钮常显 521 帧（C# ChangeLockState）
    if let Ok(mut frames) = confirm.single_mut() {
        let idx = if trade.my_locked { 521 } else { 520 };
        if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, idx) {
            if frames.normal != h {
                frames.normal = h;
            }
        }
    }
}

/// 交易交互：存/取物品、金币输入、锁定、关闭
#[allow(clippy::too_many_arguments)]
fn trade_action_system(
    mut trade: ResMut<TradeState>,
    hud: Res<HudState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<TradeClose>>,
    confirm: Query<&UiButton, With<TradeConfirmBtn>>,
    // 只读 Transform 命中（拖动/推位后仍准确）：金币标签 / 我方槽 / 背包格
    gold_hit: Query<&Transform, With<TradeGoldHit>>,
    my_slots: Query<(&Transform, &TradeSlot)>,
    inv_cells: Query<(&Transform, &InvSlot, &Visibility)>,
    mut amount: ResMut<AmountBoxState>,
    mut result: MessageReader<AmountBoxResult>,
) {
    // 金币输入结果（C# User.TradeGoldAmount += amount 累计，TradeDialogs.cs:90）
    for r in result.read() {
        if let Some(n) = r.0 {
            if trade.visible && n > 0 {
                net.send_packet(&mir2_shared::packets::client::trade::TradeGold { amount: n });
                trade.my_gold += n as u64;
                tracing::info!("💰 交易金币: +{} (累计 {})", n, trade.my_gold);
            }
        }
    }
    if !trade.visible {
        return;
    }
    for btn in &close {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::trade::TradeCancel);
            trade.visible = false;
            trade.invite = None;
            trade_reset(&mut trade);
        }
    }
    for btn in &confirm {
        if btn.clicked {
            trade.my_locked = !trade.my_locked;
            net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm {
                locked: trade.my_locked,
            });
            tracing::info!("🔒 交易锁定: {}", trade.my_locked);
        }
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    // 我方交易槽点击 → 取回（C# RetrieveTradeItem；按实体实际 Transform 命中）
    for (tf, slot) in &my_slots {
        if slot.0 != 0 {
            continue;
        }
        let x = tf.translation.x;
        let y = -tf.translation.y;
        if cursor.x >= x && cursor.x <= x + CELL_W && cursor.y >= y && cursor.y <= y + CELL_H {
            if !trade.my_locked
                && trade
                    .my_items
                    .get(slot.1)
                    .and_then(|s| s.as_ref())
                    .is_some()
            {
                net.send_packet(&mir2_shared::packets::client::trade::RetrieveTradeItem {
                    from: slot.1 as i32,
                    to: 0,
                });
                trade.my_items[slot.1] = None;
                trade.my_locked = false;
                tracing::info!("↩️ 取回交易物品 槽{}", slot.1);
            }
            return;
        }
    }
    // 金币标签点击 → 数量框（C# GoldLabel.Click → MirAmountBox(…, GameScene.Gold)）
    for tf in &gold_hit {
        let cx = tf.translation.x;
        let cy = -tf.translation.y;
        if (cursor.x - cx).abs() <= GOLD_W / 2.0 && (cursor.y - cy).abs() <= GOLD_H / 2.0 {
            if !trade.my_locked {
                amount.ask("输入交易金币", hud.gold);
            }
            return;
        }
    }
    // 点击背包物品 → 存入（C# DepositTradeItem：点击空槽/找空槽，MirItemCell.cs:1553-1565；
    // 按背包格实体实际 Transform 命中——背包可能已被推到右侧或拖动过）
    if !trade.my_locked {
        let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
        for (tf, InvSlot(idx), vis) in &inv_cells {
            if *vis != Visibility::Visible {
                continue;
            }
            let x = tf.translation.x;
            let y = -tf.translation.y;
            if cursor.x >= x && cursor.x <= x + CELL_W && cursor.y >= y && cursor.y <= y + CELL_H {
                if let Some(item) = items.get(*idx).and_then(|s| s.as_ref()) {
                    if let Some(to) = trade.my_items.iter().position(|s| s.is_none()) {
                        net.send_packet(&mir2_shared::packets::client::trade::DepositTradeItem {
                            from: *idx as i32,
                            to: to as i32,
                        });
                        trade.pending_deposit = Some((*idx, to));
                        tracing::info!(
                            "📦 放入交易: {} (uid={}) 背包{} -> 槽{}",
                            item.name,
                            item.unique_id,
                            idx,
                            to
                        );
                    }
                }
                return;
            }
        }
    }
}

/// 邀请提示 Yes/No → C.TradeReply；接受后本地开窗
fn trade_invite_system(
    mut trade: ResMut<TradeState>,
    net: Res<NetConnection>,
    yes: Query<&UiButton, With<TradeInviteYes>>,
    no: Query<&UiButton, With<TradeInviteNo>>,
) {
    if trade.invite.is_none() {
        return;
    }
    let mut accept: Option<bool> = None;
    for btn in &yes {
        if btn.clicked {
            accept = Some(true);
        }
    }
    for btn in &no {
        if btn.clicked {
            accept = Some(false);
        }
    }
    if let Some(a) = accept {
        net.send_packet(&mir2_shared::packets::client::trade::TradeReply { accept_invite: a });
        tracing::info!("🤝 交易邀请回复: accept={}", a);
        if a {
            // 本地立即开窗（服务器随后发 TradeRequest open 包）
            let inviter = trade.invite.clone().unwrap_or_default();
            trade.partner_name = inviter;
            trade.is_initiator = false;
            trade.visible = true;
        }
        trade.invite = None;
    }
}

/// 消费服务端交易事件（网络层只广播 ServerEvent；关闭/金币由本模块应用）
fn trade_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut trade: ResMut<TradeState>,
    inv_q: Query<&Inventory, With<LocalPlayer>>,
) {
    use crate::network::server_event::ServerEvent;
    let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
    for ev in events.read() {
        match ev {
            ServerEvent::TradeGold { amount } => {
                trade.their_gold = *amount;
                // C# GameScene.TradeGold（:6321-6326）：对方变动金币 → 我方解锁
                trade.my_locked = false;
            }
            ServerEvent::TradeCancelled => {
                trade.visible = false;
                trade.invite = None;
                trade_reset(&mut trade);
            }
            ServerEvent::TradeRequested { name } => {
                if trade.visible {
                    // 打开包（服务器权威 partner）
                    trade.partner_name = name.clone();
                } else if trade.is_initiator {
                    trade.visible = true;
                    trade.partner_name = name.clone();
                } else if trade.invite.is_none() {
                    trade.invite = Some(name.clone());
                }
            }
            ServerEvent::TradeConfirm { a_locked, b_locked } => {
                if trade.is_initiator {
                    trade.my_locked = *a_locked;
                    trade.their_locked = *b_locked;
                } else {
                    trade.my_locked = *b_locked;
                    trade.their_locked = *a_locked;
                }
                if *a_locked && *b_locked {
                    // 交易完成：关闭窗口（C# TradeConfirm → TradeReset）
                    trade.visible = false;
                    trade.invite = None;
                    trade_reset(&mut trade);
                }
            }
            ServerEvent::TradeItemUpdate {
                uid,
                grid,
                count,
                is_add,
            } => {
                // C# GameScene.TradeItem（:6327-6332）：对方变动物品 → 我方解锁
                trade.my_locked = false;
                if *is_add {
                    if let Some(slot) = trade.their_items.get_mut(*grid) {
                        let prev = slot.take();
                        *slot = Some(TradeItem {
                            uid: *uid,
                            item_index: prev.as_ref().map(|p| p.item_index).unwrap_or(0),
                            name: prev.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
                            image: prev.as_ref().map(|p| p.image).unwrap_or(0),
                            count: *count,
                        });
                    }
                } else {
                    if let Some(slot) = trade.their_items.get_mut(*grid) {
                        *slot = None;
                    }
                    trade
                        .their_items
                        .retain(|s| s.as_ref().map(|i| i.uid) != Some(*uid));
                }
            }
            ServerEvent::TradeDeposit { from, to, success } => {
                if *success {
                    if let Some((from2, to2)) = trade.pending_deposit.take() {
                        let from = (*from).max(from2 as i32) as usize;
                        if let Some(item) = items.get(from).and_then(|s| s.as_ref()) {
                            if let Some(slot) = trade.my_items.get_mut(to2.max(*to as usize)) {
                                *slot = Some(TradeItem::from(item));
                            }
                        }
                        trade.my_locked = false;
                    }
                } else {
                    trade.pending_deposit = None;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# TradeDialogs.cs:21/211 原点公式锚点（1024x768）：
    /// 我方 (512-204-10, 768-350)=(298,418)；对方 (512+10, 418)=(522,418)。
    /// 旧实现单窗 (250,100) 且背景误用 Title[22]（实测仅 32x16，无真实底板）。
    #[test]
    fn trade_origins_match_csharp() {
        assert_eq!(TRADE_X, 298.0);
        assert_eq!(TRADE_Y, 418.0);
        assert_eq!(GUEST_X, 522.0);
        assert_eq!(GUEST_Y, 418.0);
        // 双窗互不重叠：我方右缘 502 < 对方左缘 522（C# 中缝 20px）
        assert!(TRADE_X + TRADE_W < GUEST_X);
        // 均在画布内
        assert!(GUEST_X + TRADE_W <= 1024.0 && GUEST_Y + TRADE_H <= 768.0);
        // 关闭钮 W-23（与 SocketDialog 同公式）
        assert_eq!(CLOSE_DX, 181.0);
    }

    /// C# Grid[2*x+y] 列主序（TradeDialogs.cs:106-118）：
    /// 槽 0=(0列,0行)=(10,39)、槽 1=(0列,1行)=(10,72)、槽 2=(1列,0行)=(47,39)、
    /// 槽 9=(4列,1行)=(158,72)（x*36+10+x = x*37+10）；整窗 204x152 内。
    #[test]
    fn trade_slot_grid_is_column_major() {
        assert_eq!(trade_slot_pos(0), (10.0, 39.0));
        assert_eq!(trade_slot_pos(1), (10.0, 72.0));
        assert_eq!(trade_slot_pos(2), (47.0, 39.0));
        assert_eq!(trade_slot_pos(9), (158.0, 72.0));
        for i in 0..TRADE_SLOTS {
            let (x, y) = trade_slot_pos(i);
            assert!(x + CELL_W <= TRADE_W, "槽{} 右缘 {} 超窗宽", i, x + CELL_W);
            assert!(y + CELL_H <= TRADE_H, "槽{} 下缘 {} 超窗高", i, y + CELL_H);
        }
    }

    /// C# 金币格式 "{0:###,###,##0}"（RefreshInterface:145 / GuestGoldLabel:258）
    #[test]
    fn trade_gold_format_thousands() {
        assert_eq!(format_thousands(0), "0");
        assert_eq!(format_thousands(999), "999");
        assert_eq!(format_thousands(1000), "1,000");
        assert_eq!(format_thousands(1234567), "1,234,567");
    }

    /// 邀请框居中：C# MirMessageBox（:26）(ScreenW-456)/2、(ScreenH-190)/2 int 整除
    #[test]
    fn trade_invite_box_centered() {
        assert_eq!(INVITE_X, 284.0);
        assert_eq!(INVITE_Y, 289.0);
    }
}
