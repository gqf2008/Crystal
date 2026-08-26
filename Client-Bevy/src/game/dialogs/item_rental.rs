// ============================================================================
// 物品租赁对话框（M42）
// 参考：C# ItemRentalDialog + ServerRust market.rs 租赁流程
// 流程：租方发起 → 物主存入物品 → 双方设费用/期限 → 双方锁定 → 确认成交
// 网络（ServerRust gate 实际 wire）：
//   C: ItemRentalRequest[target dotnet] / DepositRentalItem[uid u64] / RetrieveRentalItem[uid u64]
//      ItemRentalFee[u32] / ItemRentalPeriod[u32] / LockFee/ LockItem / Confirm / Cancel（空）
//   S: ItemRentalRequest / UpdateRentalItem[hasdata u8][fee u32][period i32] / ItemRentalFee / Period
//      DepositRentalItem / RetrieveRentalItem / Lock / PartnerLock / CanConfirm / Confirm / Cancel
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::UiFont;
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_label, spawn_panel,
};

/// 租赁状态
#[derive(Resource, Default)]
pub struct ItemRentalState {
    /// 收到租赁请求（物主视角）
    pub request_received: bool,
    /// 会话物品（UpdateRentalItem 语义）
    pub has_item: bool,
    pub fee: u32,
    pub period: i32,
    pub can_confirm: bool,
    pub message: String,
    /// 是否已确认成交
    pub confirmed: bool,
    /// 最近存入物品 uid
    pub deposit_uid: Option<u64>,
}

#[derive(Component)]
pub struct ItemRentalWidget;

#[derive(Component)]
pub struct ItemRentalClose;

#[derive(Component)]
pub struct ItemRentalRequest;

#[derive(Component)]
pub struct ItemRentalDeposit;

#[derive(Component)]
pub struct ItemRentalSetFee;

#[derive(Component)]
pub struct ItemRentalSetPeriod;

#[derive(Component)]
pub struct ItemRentalLockFee;

#[derive(Component)]
pub struct ItemRentalLockItem;

#[derive(Component)]
pub struct ItemRentalConfirm;

#[derive(Component)]
pub struct ItemRentalCancel;

#[derive(Component)]
pub struct ItemRentalLine(usize);

/// 目标名输入框（TextInput 8）/ 费用（9）/ 期限（10）
#[derive(Component)]
pub struct ItemRentalTargetField;

#[derive(Component)]
pub struct ItemRentalFeeField;

#[derive(Component)]
pub struct ItemRentalPeriodField;

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
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    // 面板 Prguse[170] @ (280,80)。加宽加高到 320x320：8 按钮 3 列 + 3 输入框
    // + 关闭按钮全在面板内（旧 sprite 布局底部按钮 rel y=290 悬空 207 高面板外）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 170) else {
        return;
    };
    let panel = spawn_panel(&mut commands, bg, 280.0, 80.0, 320.0, 320.0, 30);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::ItemRental), ItemRentalWidget));

    commands.entity(panel).with_children(|p| {
        // 关闭 Prguse2[360/361/362] @(300,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 300.0, 3.0, 20.0, 20.0, 10).insert(ItemRentalClose);
        }
        // 状态行 6 行 @(18,38+20i)
        for i in 0..6usize {
            spawn_label(p, &font, "", 18.0, 38.0 + i as f32 * 20.0, 12.0, Color::WHITE, 9)
                .insert(ItemRentalLine(i));
        }
        // 目标名（8）/费用（9）/期限（10）输入框 @(18,160)/(18,192)/(150,192)
        spawn_rental_input(p, &mut images, &font, 8, 18.0, 160.0, 120.0, 298.0, 240.0);
        spawn_rental_input(p, &mut images, &font, 9, 18.0, 192.0, 80.0, 298.0, 272.0);
        spawn_rental_input(p, &mut images, &font, 10, 150.0, 192.0, 80.0, 430.0, 272.0);
        // 按钮：发起（150,158）；存入/设费（18/110,222）；设期/锁费（18/110,256）；
        // 锁物/确认/取消（18/110/200,290）
        spawn_rental_buttons(p, &mut libs, &mut images);
    });
}

/// 租赁输入框（TextInputField(id) + 子 TextInputDisplay(id)）；面板子节点
#[allow(clippy::too_many_arguments)]
fn spawn_rental_input(
    parent: &mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    images: &mut Assets<Image>,
    font: &Handle<Font>,
    id: usize,
    x: f32,
    y: f32,
    w: f32,
    rect_x: f32,
    rect_y: f32,
) {
    spawn_container(parent, x, y, w, 20.0, 10)
        .insert((
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(rect_x, rect_y, w, 20.0),
            BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
        ))
        .with_children(|ic| {
            ic.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    top: Val::Px(2.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ZIndex(11),
                crate::game::dialogs::text_input::TextInputDisplay(id),
            ));
        });
    let _ = images;
}

/// 租赁按钮（面板子节点）
fn spawn_rental_buttons(
    parent: &mut bevy::ecs::hierarchy::ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
) {
    // 发起 Title[206/207/208] @(150,158)
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Title, 206),
        load_lib_image(libs, images, LibraryName::Title, 207),
        load_lib_image(libs, images, LibraryName::Title, 208),
    ) {
        spawn_icon_button(parent, n.clone(), h.clone(), pr.clone(), 150.0, 158.0, 76.0, 25.0, 10)
            .insert(ItemRentalRequest);
        spawn_icon_button(parent, n.clone(), h.clone(), pr.clone(), 18.0, 222.0, 76.0, 25.0, 10)
            .insert(ItemRentalDeposit);
        spawn_icon_button(parent, n.clone(), h.clone(), pr.clone(), 18.0, 256.0, 76.0, 25.0, 10)
            .insert(ItemRentalSetPeriod);
        spawn_icon_button(parent, n.clone(), h.clone(), pr.clone(), 18.0, 290.0, 76.0, 25.0, 10)
            .insert(ItemRentalLockItem);
        spawn_icon_button(parent, n, h, pr, 200.0, 290.0, 76.0, 25.0, 10).insert(ItemRentalCancel);
    }
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Title, 210),
        load_lib_image(libs, images, LibraryName::Title, 211),
        load_lib_image(libs, images, LibraryName::Title, 212),
    ) {
        spawn_icon_button(parent, n.clone(), h.clone(), pr.clone(), 110.0, 222.0, 76.0, 25.0, 10)
            .insert(ItemRentalSetFee);
        spawn_icon_button(parent, n.clone(), h.clone(), pr.clone(), 110.0, 256.0, 76.0, 25.0, 10)
            .insert(ItemRentalLockFee);
        spawn_icon_button(parent, n, h, pr, 110.0, 290.0, 76.0, 25.0, 10).insert(ItemRentalConfirm);
    }
}

/// 显隐 + 渲染 + 全部按钮
#[allow(clippy::too_many_arguments)]
fn item_rental_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ItemRentalState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<(Entity, &Interaction), With<ItemRentalClose>>,
    request_btn: Query<(Entity, &Interaction), With<ItemRentalRequest>>,
    mut widgets: Query<&mut Visibility, With<ItemRentalWidget>>,
    mut lines: Query<(&mut Text, &ItemRentalLine)>,
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
    let open = mgr.is_open(DialogKind::ItemRental);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::ItemRental);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            0 => "物品租赁".to_string(),
            1 => format!(
                "会话: {} / 物品: {}",
                if state.request_received { "请求中" } else { "无" },
                if state.has_item { "已存入" } else { "无" }
            ),
            2 => format!("费用: {} 金币 / 期限: {} 小时", state.fee, state.period),
            3 => format!("锁定: {} / 可确认: {}", if state.can_confirm { "双方已锁定" } else { "未锁定" }, state.can_confirm),
            4 => state.message.clone(),
            5 => "租方：输入目标名→发起；物主：存入→设费/期→锁定物品；双方锁定后确认".to_string(),
            _ => String::new(),
        };
    }
    // 发起租赁（租方）
    for (e, inter) in &request_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(8).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&crate::network::RentalRequestWire {
                    target_name: name.clone(),
                });
                state.message = format!("已向 {} 发起租赁请求", name);
                tracing::info!("📦 租赁请求 → {}", name);
                input.texts[8].clear();
                input.active = None;
            }
        }
    }
}

/// 租赁动作：存入/费用/期限/锁定/确认/取消（独立系统避免 Bevy 16 参数上限）
#[allow(clippy::too_many_arguments)]
fn item_rental_action_system(
    mgr: Res<DialogManager>,
    mut state: ResMut<ItemRentalState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    deposit_btn: Query<(Entity, &Interaction), With<ItemRentalDeposit>>,
    fee_btn: Query<(Entity, &Interaction), With<ItemRentalSetFee>>,
    period_btn: Query<(Entity, &Interaction), With<ItemRentalSetPeriod>>,
    lock_fee_btn: Query<(Entity, &Interaction), With<ItemRentalLockFee>>,
    lock_item_btn: Query<(Entity, &Interaction), With<ItemRentalLockItem>>,
    confirm_btn: Query<(Entity, &Interaction), With<ItemRentalConfirm>>,
    cancel_btn: Query<(Entity, &Interaction), With<ItemRentalCancel>>,
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
    if !mgr.is_open(DialogKind::ItemRental) {
        return;
    }
    // 存入（物主）：选中背包物品
    for (e, inter) in &deposit_btn {
        if edge(e, inter, &mut prev_inter) {
            let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
            let idx = inv_click
                .selected
                .filter(|i| items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&crate::network::RentalDepositWire {
                        unique_id: item.unique_id,
                    });
                    state.deposit_uid = Some(item.unique_id);
                    state.message = format!("存入租赁物品 uid={}", item.unique_id);
                    tracing::info!("📦 存入租赁物品 uid={}", item.unique_id);
                }
            }
        }
    }
    // 设置费用
    for (e, inter) in &fee_btn {
        if edge(e, inter, &mut prev_inter) {
            let fee = input
                .texts
                .get(9)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalFee { amount: fee });
            state.message = format!("设置租赁费用 {}", fee);
            tracing::info!("📦 租赁费用 {}", fee);
        }
    }
    // 设置期限
    for (e, inter) in &period_btn {
        if edge(e, inter, &mut prev_inter) {
            let days = input
                .texts
                .get(10)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalPeriod { days });
            state.message = format!("设置租赁期限 {} 小时", days);
            tracing::info!("📦 租赁期限 {}", days);
        }
    }
    // 锁定费用（租方）
    for (e, inter) in &lock_fee_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockFee);
            state.message = "已锁定费用".to_string();
            tracing::info!("📦 锁定费用");
        }
    }
    // 锁定物品（物主）
    for (e, inter) in &lock_item_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockItem);
            state.message = "已锁定物品".to_string();
            tracing::info!("📦 锁定物品");
        }
    }
    // 确认
    for (e, inter) in &confirm_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::ConfirmItemRental);
            state.message = "已发送确认".to_string();
            tracing::info!("📦 确认租赁");
        }
    }
    // 取消
    for (e, inter) in &cancel_btn {
        if edge(e, inter, &mut prev_inter) {
            net.send_packet(&mir2_shared::packets::client::item::CancelItemRental);
            state.message = "已发送取消".to_string();
            tracing::info!("📦 取消租赁");
        }
    }
}


/// 消费服务端租赁事件（网络层只广播 ServerEvent；文案在此构造）
fn rental_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut rental: ResMut<ItemRentalState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::RentalRequestReceived => {
                rental.request_received = true;
                rental.message = "收到租赁请求（物主）".to_string();
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
                rental.message = format!("租赁期限更新: {} 小时", rental.period);
            }
            ServerEvent::RentalDeposit { uid, success } => {
                rental.deposit_uid = Some(*uid);
                rental.message = format!(
                    "存入租赁物品: {} ({})",
                    uid,
                    if *success { "成功" } else { "失败" }
                );
            }
            ServerEvent::RentalRetrieve { uid, success } => {
                rental.message = format!(
                    "取回租赁物品: {} ({})",
                    uid,
                    if *success { "成功" } else { "失败" }
                );
                rental.has_item = false;
            }
            ServerEvent::RentalLocked => {
                rental.message = "锁定状态更新".to_string();
            }
            ServerEvent::RentalPartnerLocked => {
                rental.message = "对方已锁定".to_string();
            }
            ServerEvent::RentalCanConfirm { can_confirm } => {
                rental.can_confirm = *can_confirm;
                rental.message = if rental.can_confirm {
                    "双方已锁定，可以确认成交".to_string()
                } else {
                    "尚未可确认".to_string()
                };
            }
            ServerEvent::RentalConfirmed { success } => {
                rental.confirmed = *success;
                rental.message = if *success {
                    "租赁成交！".to_string()
                } else {
                    "确认失败".to_string()
                };
            }
            ServerEvent::RentalCancelled => {
                rental.request_received = false;
                rental.has_item = false;
                rental.can_confirm = false;
                rental.message = "租赁已取消".to_string();
            }
            _ => {}
        }
    }
}
