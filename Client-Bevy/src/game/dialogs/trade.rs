// ============================================================================
// 交易对话框（M23）
// 布局参考：C# TradeDialogs.cs / macroquad trade_dialog.rs
//   - 背景 Title[22]，标题 Title[18]，位置 (250,100)；左（自己）右（对方）5x4 物品槽
//   - 邀请提示 MirMessageBox（Prguse[360] + Yes Title[206-208] / No Title[210-212]）
// 交互（原版 C# 语义）：
//   点击背包物品 → C.DepositTradeItem{from=背包格, to=交易槽}；点击我方槽 → 取回
//   金币按钮 → 数量框 → C.TradeGold{amount}；锁定 → C.TradeConfirm{locked}
//   关闭 → C.TradeCancel；邀请 Yes/No → C.TradeReply{accept}
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::amount_box::{AmountBoxResult, AmountBoxState};
use crate::game::dialogs::inventory::{inv_slot_at, InvItem};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::game::hud::HudState;
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};
use crate::ui::controls::{spawn_item_cell, ItemCellData, ItemCellIcon, ItemCellCount};

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
            my_items: vec![None; 20],
            their_items: vec![None; 20],
            my_gold: 0,
            their_gold: 0,
            my_locked: false,
            their_locked: false,
            pending_deposit: None,
        }
    }
}

const DIALOG_X: f32 = 250.0;
const DIALOG_Y: f32 = 100.0;

#[derive(Component)]
pub struct TradeWidget;

#[derive(Component)]
pub struct TradeClose;

#[derive(Component)]
pub struct TradeLock;

#[derive(Component)]
pub struct TradeGoldBtn;

#[derive(Component)]
pub struct TradeGoldText;

/// 交易物品槽（side 0=自己 1=对方, idx 0..20）
#[derive(Component, Clone, Copy)]
pub struct TradeSlot(pub usize, pub usize);

// 邀请提示
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
                app.add_systems(
            Update,
            trade_server_events.run_if(in_state(AppState::Game)),
        );
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

    // 背景 Title[22]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 22) {
        let e = spawn_ui_sprite(&mut commands, h, DIALOG_X, DIALOG_Y, 6.0, 1.0);
        commands
            .entity(e)
            .insert((DialogRoot(DialogKind::Trade), TradeWidget, Visibility::Hidden));
    }
    // 标题 Title[18]
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 18) {
        let e = spawn_ui_sprite(&mut commands, h, DIALOG_X + 18.0, DIALOG_Y + 9.0, 6.2, 1.0);
        commands
            .entity(e)
            .insert((DialogRoot(DialogKind::Trade), TradeWidget, Visibility::Hidden));
    }
    // 关闭
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        DIALOG_X + 520.0, DIALOG_Y + 3.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            TradeClose,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
        ));
    }
    // 金币输入按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 198, 199, 199,
        DIALOG_X + 120.0, DIALOG_Y + 300.0, 7.0, 60.0, 23.0,
    ) {
        commands.entity(e).insert((
            TradeGoldBtn,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
        ));
    }
    // 锁定按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 200, 201, 202,
        DIALOG_X + 230.0, DIALOG_Y + 320.0, 7.0, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            TradeLock,
            DialogRoot(DialogKind::Trade),
            TradeWidget,
        ));
    }
    // 金币文本
    let g = spawn_ui_text(
        &mut commands, &font, "金币: 0 | 对方: 0",
        DIALOG_X + 30.0, DIALOG_Y + 305.0, 12.0, Color::srgb(1.0, 0.85, 0.3), 8.0,
    );
    commands.entity(g).insert((
        TradeGoldText,
        DialogRoot(DialogKind::Trade),
        TradeWidget,
    ));

    // 双方物品槽（左 5x4 自己，右 5x4 对方；36x32，#106 通用 ItemCell）
    for side in 0..2usize {
        let base_x = if side == 0 { 260.0 } else { 250.0 + 280.0 };
        for i in 0..20usize {
            let x = base_x + (i % 5) as f32 * 37.0;
            let y = 140.0 + (i / 5) as f32 * 34.0;
            let slot = spawn_item_cell(&mut commands, &mut images, &font, x, y, 6.3, 36.0, 32.0, i);
            commands.entity(slot).insert((
                TradeSlot(side, i),
                DialogRoot(DialogKind::Trade),
                TradeWidget,
            ));
        }
    }
}
/// 显隐 + 槽位物品渲染 + 金币/锁定状态 + 邀请提示
#[allow(clippy::type_complexity)]
fn trade_ui_system(
    trade: Res<TradeState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut widgets: Query<
        (&mut Visibility, Option<&TradeSlot>),
        (
            With<TradeWidget>,
            Without<TradeGoldText>,
            Without<ItemCellIcon>,
            Without<ItemCellCount>,
            Without<TradeInviteWidget>,
        ),
    >,
    mut cells: Query<(&mut ItemCellData, &TradeSlot)>,
    mut gold_texts: Query<(&mut Text2d, &TradeGoldText), (Without<ItemCellCount>, Without<TradeInviteText>)>,
    mut invite_widgets: Query<
        &mut Visibility,
        (
            With<TradeInviteWidget>,
            Without<TradeWidget>,
            Without<ItemCellIcon>,
            Without<ItemCellCount>,
        ),
    >,
    mut invite_texts: Query<(&mut Text2d, &TradeInviteText), (Without<ItemCellCount>, Without<TradeGoldText>)>,
) {
    for (mut vis, _slot) in &mut widgets {
        *vis = if trade.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
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
    for (mut t, _) in &mut gold_texts {
        let new = format!(
            "金币: {} | 对方: {}{}",
            trade.my_gold,
            trade.their_gold,
            if trade.my_locked && trade.their_locked {
                "（交易完成）"
            } else if trade.their_locked {
                "（对方已锁定）"
            } else {
                ""
            }
        );
        if t.0 != new {
            t.0 = new;
        }
    }
    // 邀请提示
    let has_invite = trade.invite.is_some();
    for mut vis in &mut invite_widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut text, _) in &mut invite_texts {
        let new = match trade.invite.as_ref() {
            Some(name) => format!("{} 想与你交易", name),
            None => String::new(),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
}

/// 交易交互：存/取物品、金币输入、锁定、关闭
#[allow(clippy::too_many_arguments)]
fn trade_action_system(
    mut trade: ResMut<TradeState>,
    hud: Res<HudState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<TradeClose>>,
    lock: Query<&UiButton, With<TradeLock>>,
    gold_btn: Query<&UiButton, With<TradeGoldBtn>>,
    mut amount: ResMut<AmountBoxState>,
    mut result: MessageReader<AmountBoxResult>,
) {
    // 金币输入结果
    for r in result.read() {
        if let Some(n) = r.0 {
            if trade.visible && n > 0 {
                net.send_packet(&mir2_shared::packets::client::trade::TradeGold {
                    amount: n,
                });
                trade.my_gold = n as u64;
                trade.my_locked = false;
                tracing::info!("💰 交易金币: {}", n);
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
            trade.pending_deposit = None;
        }
    }
    for btn in &lock {
        if btn.clicked {
            trade.my_locked = !trade.my_locked;
            net.send_packet(&mir2_shared::packets::client::trade::TradeConfirm {
                locked: trade.my_locked,
            });
            tracing::info!("🔒 交易锁定: {}", trade.my_locked);
        }
    }
    for btn in &gold_btn {
        if btn.clicked {
            if !trade.my_locked {
                amount.ask("输入交易金币", 999_999_999);
            }
        }
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    // 我方交易槽点击 → 取回（C# RetrieveTradeItem）
    for i in 0..20usize {
        let x = 260.0 + (i % 5) as f32 * 37.0;
        let y = 140.0 + (i / 5) as f32 * 34.0;
        if cursor.x >= x && cursor.x <= x + 36.0 && cursor.y >= y && cursor.y <= y + 32.0 {
            if !trade.my_locked && trade.my_items.get(i).and_then(|s| s.as_ref()).is_some() {
                net.send_packet(&mir2_shared::packets::client::trade::RetrieveTradeItem {
                    from: i as i32,
                    to: 0,
                });
                trade.my_items[i] = None;
                trade.my_locked = false;
                tracing::info!("↩️ 取回交易物品 槽{}", i);
            }
            return;
        }
    }
    // 点击背包物品 → 存入（C# DepositTradeItem）
    if !trade.my_locked {
        if let Some(from) = inv_slot_at(cursor.x, cursor.y) {
            if let Some(item) = hud.inventory.items.get(from).and_then(|s| s.as_ref()) {
                if let Some(to) = trade.my_items.iter().position(|s| s.is_none()) {
                    net.send_packet(&mir2_shared::packets::client::trade::DepositTradeItem {
                        from: from as i32,
                        to: to as i32,
                    });
                    trade.pending_deposit = Some((from, to));
                    tracing::info!(
                        "📦 放入交易: {} (uid={}) 背包{} -> 槽{}",
                        item.name,
                        item.unique_id,
                        from,
                        to
                    );
                }
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
        net.send_packet(&mir2_shared::packets::client::trade::TradeReply {
            accept_invite: a,
        });
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
    hud: Res<crate::game::hud::HudState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::TradeGold { amount } => {
                trade.their_gold = *amount;
            }
            ServerEvent::TradeCancelled => {
                trade.visible = false;
                trade.invite = None;
                trade.pending_deposit = None;
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
                    // 交易完成：关闭窗口
                    trade.visible = false;
                    trade.invite = None;
                    trade.pending_deposit = None;
                }
            }
            ServerEvent::TradeItemUpdate { uid, grid, count, is_add } => {
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
                    trade.their_items.retain(|s| s.as_ref().map(|i| i.uid) != Some(*uid));
                }
            }
            ServerEvent::TradeDeposit { from, to, success } => {
                if *success {
                    if let Some((from2, to2)) = trade.pending_deposit.take() {
                        let from = (*from).max(from2 as i32) as usize;
                        if let Some(item) = hud.inventory.items.get(from).and_then(|s| s.as_ref()) {
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
