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
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
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
        app.add_systems(OnEnter(AppState::Game), spawn_item_rental);
        app.add_systems(OnExit(AppState::Game), cleanup_item_rental);
        app.add_systems(
            Update,
            (item_rental_ui_system, item_rental_action_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::ItemRental),
            ItemRentalWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            ItemRentalClose,
            DialogRoot(DialogKind::ItemRental),
            ItemRentalWidget,
        ));
    }
    // 状态行 6 行
    for i in 0..6usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 118.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            ItemRentalLine(i),
            DialogRoot(DialogKind::ItemRental),
            ItemRentalWidget,
        ));
    }
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    spawn_rental_input(&mut commands, &white, &font, 8, 298.0, 240.0, 120.0);
    spawn_rental_input(&mut commands, &white, &font, 9, 298.0, 272.0, 80.0);
    spawn_rental_input(&mut commands, &white, &font, 10, 430.0, 272.0, 80.0);
    spawn_rental_buttons(&mut commands, &mut libs, &mut images, &mut cache);
}

/// 租赁按钮
fn spawn_rental_buttons(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
) {
    // 按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 206, 207, 208,
        430.0, 238.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalRequest, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 206, 207, 208,
        298.0, 302.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalDeposit, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 302.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalSetFee, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 206, 207, 208,
        298.0, 336.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalSetPeriod, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 336.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalLockFee, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 206, 207, 208,
        298.0, 370.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalLockItem, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 370.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalConfirm, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        commands, libs, images, cache,
        LibraryName::Title, 206, 207, 208,
        480.0, 370.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((ItemRentalCancel, DialogRoot(DialogKind::ItemRental), ItemRentalWidget));
    }
}

/// 租赁输入框（TextInputField(id) + 子 TextInputDisplay(id)）
fn spawn_rental_input(
    commands: &mut Commands,
    white: &Handle<Image>,
    font: &Handle<Font>,
    id: usize,
    x: f32,
    y: f32,
    w: f32,
) {
    let box_e = commands
        .spawn((
            crate::ui::sprite_ui::UiEntity,
            DialogRoot(DialogKind::ItemRental),
            ItemRentalWidget,
            crate::game::dialogs::text_input::TextInputField(id),
            crate::game::dialogs::text_input::TextInputRect(x, y, w, 20.0),
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                custom_size: Some(Vec2::new(w, 20.0)),
                ..default()
            },
            bevy::sprite::Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, 8.1),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(box_e).with_children(|p| {
        p.spawn((
            crate::game::dialogs::text_input::TextInputDisplay(id),
            Text2d::new(String::new()),
            bevy::sprite::Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
            Transform::from_xyz(4.0, -2.0, 8.2),
        ));
    });
}

/// 显隐 + 渲染 + 全部按钮
#[allow(clippy::too_many_arguments)]
fn item_rental_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<ItemRentalState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    close: Query<&UiButton, With<ItemRentalClose>>,
    request_btn: Query<&UiButton, With<ItemRentalRequest>>,
    mut widgets: Query<&mut Visibility, With<ItemRentalWidget>>,
    mut lines: Query<(&mut Text2d, &ItemRentalLine)>,
) {
    let open = mgr.is_open(DialogKind::ItemRental);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
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
    for btn in &request_btn {
        if btn.clicked {
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
    hud: Res<crate::game::hud::HudState>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    deposit_btn: Query<&UiButton, With<ItemRentalDeposit>>,
    fee_btn: Query<&UiButton, With<ItemRentalSetFee>>,
    period_btn: Query<&UiButton, With<ItemRentalSetPeriod>>,
    lock_fee_btn: Query<&UiButton, With<ItemRentalLockFee>>,
    lock_item_btn: Query<&UiButton, With<ItemRentalLockItem>>,
    confirm_btn: Query<&UiButton, With<ItemRentalConfirm>>,
    cancel_btn: Query<&UiButton, With<ItemRentalCancel>>,
) {
    if !mgr.is_open(DialogKind::ItemRental) {
        return;
    }
    // 存入（物主）：选中背包物品
    for btn in &deposit_btn {
        if btn.clicked {
            let idx = inv_click
                .selected
                .filter(|i| hud.inventory.items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| hud.inventory.items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = hud.inventory.items.get(i).and_then(|s| s.as_ref()) {
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
    for btn in &fee_btn {
        if btn.clicked {
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
    for btn in &period_btn {
        if btn.clicked {
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
    for btn in &lock_fee_btn {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockFee);
            state.message = "已锁定费用".to_string();
            tracing::info!("📦 锁定费用");
        }
    }
    // 锁定物品（物主）
    for btn in &lock_item_btn {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::item::ItemRentalLockItem);
            state.message = "已锁定物品".to_string();
            tracing::info!("📦 锁定物品");
        }
    }
    // 确认
    for btn in &confirm_btn {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::item::ConfirmItemRental);
            state.message = "已发送确认".to_string();
            tracing::info!("📦 确认租赁");
        }
    }
    // 取消
    for btn in &cancel_btn {
        if btn.clicked {
            net.send_packet(&mir2_shared::packets::client::item::CancelItemRental);
            state.message = "已发送取消".to_string();
            tracing::info!("📦 取消租赁");
        }
    }
}
