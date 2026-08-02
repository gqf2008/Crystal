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
use crate::network::NetworkContext;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
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

#[derive(Component, Clone, Copy)]
pub struct TradeIcon(pub usize, pub usize);

#[derive(Component, Clone, Copy)]
pub struct TradeCount(pub usize, pub usize);

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

    // 双方物品槽（左 5x4 自己，右 5x4 对方；36x32）
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    for side in 0..2usize {
        let base_x = if side == 0 { 260.0 } else { 250.0 + 280.0 };
        for i in 0..20usize {
            let x = base_x + (i % 5) as f32 * 37.0;
            let y = 140.0 + (i / 5) as f32 * 34.0;
            let slot = commands
                .spawn((
                    UiEntity,
                    DialogRoot(DialogKind::Trade),
                    TradeWidget,
                    TradeSlot(side, i),
                    Sprite {
                        image: white.clone(),
                        color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                        custom_size: Some(Vec2::new(36.0, 32.0)),
                        ..default()
                    },
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(x, -y, 6.3),
                    Visibility::Hidden,
                ))
                .id();
            commands.entity(slot).with_children(|p| {
                p.spawn((
                    TradeIcon(side, i),
                    Sprite {
                        image: white.clone(),
                        custom_size: Some(Vec2::new(32.0, 28.0)),
                        ..default()
                    },
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(2.0, -2.0, 6.4),
                    Visibility::Hidden,
                ));
                p.spawn((
                    TradeCount(side, i),
                    Text2d::new(String::new()),
                    Anchor::TOP_LEFT,
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 0.6)),
                    Transform::from_xyz(20.0, -22.0, 6.5),
                    Visibility::Hidden,
                ));
            });
        }
    }

    // 邀请提示（MirMessageBox）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 360) {
        let e = spawn_ui_sprite(&mut commands, h, bx, by, 9.5, 1.0);
        commands
            .entity(e)
            .insert((TradeInviteWidget, Visibility::Hidden));
    }
    let t = spawn_ui_text(
        &mut commands, &font, "", bx + 35.0, by + 40.0, 12.0, Color::WHITE, 9.6,
    );
    commands.entity(t).insert((TradeInviteText, TradeInviteWidget));
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        bx + 240.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((TradeInviteYes, TradeInviteWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        bx + 340.0, by + 150.0, 9.7, 76.0, 25.0,
    ) {
        commands.entity(e).insert((TradeInviteNo, TradeInviteWidget));
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
            Without<TradeIcon>,
            Without<TradeCount>,
            Without<TradeInviteWidget>,
        ),
    >,
    mut icons: Query<
        (&mut Sprite, &mut Visibility, &TradeIcon),
        (
            Without<TradeWidget>,
            Without<TradeSlot>,
            Without<TradeCount>,
            Without<TradeInviteWidget>,
        ),
    >,
    mut counts: Query<
        (&mut Text2d, &mut Visibility, &TradeCount),
        (
            Without<TradeWidget>,
            Without<TradeSlot>,
            Without<TradeIcon>,
            Without<TradeGoldText>,
            Without<TradeInviteText>,
        ),
    >,
    mut gold_texts: Query<(&mut Text2d, &TradeGoldText), (Without<TradeCount>, Without<TradeInviteText>)>,
    mut invite_widgets: Query<
        &mut Visibility,
        (
            With<TradeInviteWidget>,
            Without<TradeWidget>,
            Without<TradeIcon>,
            Without<TradeCount>,
        ),
    >,
    mut invite_texts: Query<(&mut Text2d, &TradeInviteText), (Without<TradeCount>, Without<TradeGoldText>)>,
) {
    for (mut vis, _slot) in &mut widgets {
        *vis = if trade.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 物品图标
    for (mut sprite, mut vis, icon) in &mut icons {
        let item = if icon.0 == 0 {
            trade.my_items.get(icon.1).and_then(|s| s.as_ref())
        } else {
            trade.their_items.get(icon.1).and_then(|s| s.as_ref())
        };
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
    for (mut text, mut vis, count) in &mut counts {
        let item = if count.0 == 0 {
            trade.my_items.get(count.1).and_then(|s| s.as_ref())
        } else {
            trade.their_items.get(count.1).and_then(|s| s.as_ref())
        };
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
    for (mut t, _) in &mut gold_texts {
        t.0 = format!(
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
        text.0 = match trade.invite.as_ref() {
            Some(name) => format!("{} 想与你交易", name),
            None => String::new(),
        };
    }
}

/// 交易交互：存/取物品、金币输入、锁定、关闭
#[allow(clippy::too_many_arguments)]
fn trade_action_system(
    mut trade: ResMut<TradeState>,
    hud: Res<HudState>,
    net: Res<NetworkContext>,
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
    net: Res<NetworkContext>,
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