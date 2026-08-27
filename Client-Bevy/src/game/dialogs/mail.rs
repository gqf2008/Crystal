// ============================================================================
// 邮件对话框（M22）
// 布局参考：macroquad mail_dialog.rs / C# MailDialog
//   - 背景 Prguse[956]，标题 Title[20]，位置 (280,80)
//   - 邮件列表 y=60 起每 22px；点击列表项 → C.ReadMail{mail_id} → 内容区显示正文/金币
// 网络：ReceiveMail（新邮件条目 / 邮件全文，服务端同 opcode 双格式）→ 列表 + 详情
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_icon_button, spawn_item_cell_ui, spawn_label,
    spawn_panel, spawn_scroll_bar_ui, UiItemCellData, UiScrollList,
};

/// 邮件列表条目
#[derive(Debug, Clone, Default)]
pub struct MailEntry {
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    pub unread: bool,
    pub gold: u32,
    pub collected: bool,
}

/// 邮件详情（ReadMail 响应全文）
#[derive(Debug, Clone, Default)]
pub struct MailDetail {
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub gold: u32,
    /// 附件名列表
    pub items: Vec<String>,
    pub collected: bool,
}

/// 邮件状态（网络 ReceiveMail 写入）
#[derive(Resource)]
pub struct MailState {
    pub mails: Vec<MailEntry>,
    pub detail: Option<MailDetail>,
    /// 写邮件界面是否打开（输入框用通用 TextInputState id 0=收件人 1=主题 2=正文 3=金币）
    pub compose: bool,
    /// 选中的邮件行（删除用，#132）
    pub selected: Option<usize>,
    /// 写邮件附加金币（C# MailComposeParcelDialog GoldSend）
    pub compose_gold: u32,
    /// 写邮件附件（最多 5 个背包 unique_id，C# items_idx[5]）
    pub attach: Vec<Option<u64>>,
    /// #2538：贴票（C# MailComposeParcelDialog Stamped；true 时解锁 5 附件格）
    pub stamped: bool,
    /// #2538：邮资（C# ParcelCostLabel ← S.MailCost）
    pub parcel_cost: u32,
}

impl Default for MailState {
    fn default() -> Self {
        Self {
            mails: Vec::new(),
            detail: None,
            compose: false,
            selected: None,
            compose_gold: 0,
            attach: vec![None; 5],
            stamped: false,
            parcel_cost: 0,
        }
    }
}

/// #2538：可用附件格数（C# SendMail hasStamp?5:1；客户端按 Stamped 估计）
pub fn stamp_slots(stamped: bool) -> usize {
    if stamped { 5 } else { 1 }
}

/// 请求写邮件（#2631 跨对话框解耦 Message）。
/// friend 等外部对话框不再直写 [`MailState`]，改发本 Message；邮件对话框的
/// [`mail_compose_request_system`] 消费并自行预填收件人 + 打开写邮件界面。
/// `to` = 收件人名（预填到写邮件输入框 id 0，C# FriendDialog EmailButton 语义）。
#[derive(Message, Debug)]
pub struct ComposeMail {
    pub to: String,
}

/// #2538：邮票判定（C# ItemType.Nothing && Shape==1；客户端 InvItem.item_type 为
/// SharedRust 枚举值 Nothing=3）
pub fn is_stamp_item(it: &crate::game::dialogs::inventory::InvItem) -> bool {
    it.item_type == mir2_shared::enums::ItemType::Nothing as u8 && it.shape == 1
}

#[derive(Component)]
pub struct MailWidget;

#[derive(Component)]
pub struct MailClose;

#[derive(Component)]
pub struct MailDelete;

/// 收取附件按钮（C# MailReadParcelDialog.CollectButton → C.CollectParcel）
#[derive(Component)]
pub struct MailCollect;

/// 写邮件附件槽（C# MailComposeParcelDialog 附件 5 格）
#[derive(Component)]
pub struct MailAttachSlot(pub usize);

/// 写邮件背包物品选择格
#[derive(Component)]
pub struct MailInvPick(pub usize);

#[derive(Component)]
pub struct MailLine(usize);

#[derive(Component)]
pub struct MailDetailText;

// 写邮件界面
#[derive(Component)]
pub struct MailWrite;

#[derive(Component)]
pub struct MailComposeWidget;

#[derive(Component)]
pub struct MailSendBtn;

#[derive(Component)]
pub struct MailCancelBtn;

/// #2538：邮票按钮（C# StampButton Prguse2[203]，点击切换贴票）
#[derive(Component)]
pub struct MailStampBtn;

/// #2538：贴票状态覆层（C# UpdateParcel StampButton.Index=204）
#[derive(Component)]
pub struct MailStampOn;

/// #2538：邮资标签（C# ParcelCostLabel ← S.MailCost）
#[derive(Component)]
pub struct MailCostLabel;

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>();
        // #2631：跨对话框写邮件请求（friend 发、本模块消费并自行预填+打开）
        app.add_message::<ComposeMail>();
        app.add_systems(Update, mail_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(OnEnter(AppState::Game), spawn_mail);
        app.add_systems(OnExit(AppState::Game), cleanup_mail);
        app.add_systems(
            Update,
            (
                mail_compose_request_system,
                mail_ui_system,
                mail_compose_system,
                mail_stamp_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_mail(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 消费 [`ComposeMail`]：打开邮件窗 + 预填收件人并进入写邮件界面（#2631 跨对话框解耦）。
/// 复刻旧 friend.rs EmailButton 对本模块状态的写入（可见行为不变）；本系统置于
/// mail_compose_system 之前，同帧反映 compose 显隐。注意与 mail_compose_system 的
/// 写按钮路径差异：本条不重置 stamped/parcel_cost、不发 MailCost 查询（保持原 friend 语义）。
fn mail_compose_request_system(
    mut events: MessageReader<ComposeMail>,
    mut mail: ResMut<MailState>,
    mut mgr: ResMut<DialogManager>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    for ev in events.read() {
        mgr.open(DialogKind::Mail);
        mail.compose = true;
        mail.detail = None;
        mail.attach = vec![None; 5];
        mail.compose_gold = 0;
        if input.texts.len() < 4 {
            input.texts.resize(4, String::new());
        }
        input.texts[0] = ev.to.clone();
        input.active = None;
        tracing::info!("✉️ 给 {} 写邮件", ev.to);
    }
}

/// 写邮件界面：写按钮 → 打开；附件/金币选择；发送/取消
#[allow(clippy::too_many_arguments)]
fn mail_compose_system(
    mut mail: ResMut<MailState>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    write_btn: Query<(Entity, &Interaction), With<MailWrite>>,
    send_btn: Query<(Entity, &Interaction), With<MailSendBtn>>,
    cancel_btn: Query<(Entity, &Interaction), With<MailCancelBtn>>,
    mut compose_widgets: Query<&mut Visibility, (With<MailComposeWidget>, Without<MailWidget>)>,
    mut attach_cells: Query<(&mut UiItemCellData, &MailAttachSlot), Without<MailInvPick>>,
    mut pick_cells: Query<(&mut UiItemCellData, &MailInvPick), Without<MailAttachSlot>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<MailComposeWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mail.compose;
    for mut vis in &mut compose_widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (e, inter) in &write_btn {
        if edge(e, inter, &mut prev_inter) {
            mail.compose = true;
            mail.detail = None;
            mail.attach = vec![None; 5];
            mail.compose_gold = 0;
            mail.stamped = false;
            mail.parcel_cost = 0;
            if input.texts.len() < 4 {
                input.texts.resize(4, String::new());
            }
            tracing::info!("✉️ 打开写邮件");
            // #2538：C# ComposeMail → CalculatePostage（打开时查询邮资）
            request_mail_cost(&net, 0, &mail.attach, false);
        }
    }

    if open {
        let items = inv_q.single().map(|inv| inv.items.as_slice()).unwrap_or(&[]);
        // 附件槽图标（按 unique_id 在背包中查找）
        for (mut data, slot) in &mut attach_cells {
            let uid = mail.attach.get(slot.0).and_then(|s| *s);
            data.icon = uid.and_then(|uid| {
                items
                    .iter()
                    .flatten()
                    .find(|it| it.unique_id == uid)
                    .and_then(|it| {
                        load_lib_image(
                            &mut libs,
                            &mut images,
                            LibraryName::Items,
                            it.image as usize,
                        )
                    })
            });
            data.count = None;
        }
        // 背包选择格（最多 20 个，跳过已附加；记录对应背包槽位）
        let attached: Vec<u64> = mail.attach.iter().flatten().copied().collect();
        let mut pick_slots: Vec<Option<usize>> = Vec::new();
        for (slot_idx, item) in items.iter().enumerate() {
            if pick_slots.len() >= 20 {
                break;
            }
            if let Some(it) = item {
                if attached.contains(&it.unique_id) {
                    continue;
                }
                pick_slots.push(Some(slot_idx));
            }
        }
        while pick_slots.len() < 20 {
            pick_slots.push(None);
        }
        for (mut data, pick) in &mut pick_cells {
            match pick_slots.get(pick.0).and_then(|s| *s) {
                Some(slot_idx) => {
                    match items.get(slot_idx).and_then(|s| s.as_ref()) {
                        Some(it) => {
                            let icon = load_lib_image(
                                &mut libs,
                                &mut images,
                                LibraryName::Items,
                                it.image as usize,
                            );
                            data.icon = icon;
                            data.count = if it.count > 1 {
                                Some(it.count as u32)
                            } else {
                                None
                            };
                        }
                        None => {
                            data.icon = None;
                            data.count = None;
                        }
                    }
                }
                None => {
                    data.icon = None;
                    data.count = None;
                }
            }
        }

        // 点击：附件槽 → 移除；背包格 → 填入空附件槽（C# MailComposeParcelDialog 语义）
        if mouse.just_pressed(MouseButton::Left) {
            let Ok(window) = windows.single() else { return };
            let Some(cursor) = window.cursor_position() else {
                return;
            };
            let (ox, oy) = panel_origin
                .single()
                .map(|n| crate::ui::theme::node_origin(n, (290.0, 80.0)))
                .unwrap_or((290.0, 80.0));
            let mut attach_changed = false;
            for i in 0..5usize {
                let x = ox + 10.0 + i as f32 * 46.0;
                let y = oy + 146.0;
                if cursor.x >= x && cursor.x <= x + 40.0 && cursor.y >= y && cursor.y <= y + 40.0 {
                    if mail.attach.get(i).is_some_and(|s| s.is_some()) {
                        mail.attach[i] = None;
                        attach_changed = true;
                    }
                    if attach_changed {
                        // #2538：C# CalculatePostage（附件变化重查邮资）
                        let gold = input
                            .texts
                            .get(3)
                            .cloned()
                            .unwrap_or_default()
                            .trim()
                            .parse::<u32>()
                            .unwrap_or(0);
                        mail.compose_gold = gold;
                        request_mail_cost(&net, gold, &mail.attach, mail.stamped);
                    }
                    return;
                }
            }
            for (cell_idx, slot_idx) in pick_slots.iter().enumerate() {
                if let Some(slot_idx) = slot_idx {
                    let col = (cell_idx % 5) as f32;
                    let row = (cell_idx / 5) as f32;
                    let x = ox + 10.0 + col * 46.0;
                    let y = oy + 202.0 + row * 46.0;
                    if cursor.x >= x
                        && cursor.x <= x + 40.0
                        && cursor.y >= y
                        && cursor.y <= y + 40.0
                    {
                        if let Some(it) =
                            items.get(*slot_idx).and_then(|s| s.as_ref())
                        {
                            // #2538：未贴票仅第 1 格可用（C# UpdateParcel Cells[1..] Enabled=false）
                            let slots = stamp_slots(mail.stamped);
                            if let Some(empty) =
                                mail.attach.iter_mut().take(slots).find(|s| s.is_none())
                            {
                                *empty = Some(it.unique_id);
                                let gold = input
                                    .texts
                                    .get(3)
                                    .cloned()
                                    .unwrap_or_default()
                                    .trim()
                                    .parse::<u32>()
                                    .unwrap_or(0);
                                mail.compose_gold = gold;
                                request_mail_cost(&net, gold, &mail.attach, mail.stamped);
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    for (e, inter) in &send_btn {
        if edge(e, inter, &mut prev_inter) && mail.compose {
            let gold = input
                .texts
                .get(3)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            send_composed_mail(&net, &input, gold, &mail.attach, mail.stamped);
            mail.compose = false;
            mail.attach = vec![None; 5];
            mail.compose_gold = gold;
            mail.stamped = false;
            mail.parcel_cost = 0;
            input.active = None;
        }
    }
    for (e, inter) in &cancel_btn {
        if edge(e, inter, &mut prev_inter) && mail.compose {
            mail.compose = false;
            mail.attach = vec![None; 5];
            mail.stamped = false;
            mail.parcel_cost = 0;
            input.active = None;
        }
    }
}

/// #2538：邮票交互（C# StampParcel/UpdateParcel）+ 贴票覆层/邮资显示
fn mail_stamp_system(
    mut mail: ResMut<MailState>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    net: Res<NetConnection>,
    stamp_btn: Query<(Entity, &Interaction), With<MailStampBtn>>,
    mut stamp_on: Query<&mut Visibility, With<MailStampOn>>,
    mut cost_label: Query<&mut Text, With<MailCostLabel>>,
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
    for mut vis in &mut stamp_on {
        *vis = if mail.stamped && mail.compose {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut text in &mut cost_label {
        text.0 = format!("邮资: {}", mail.parcel_cost);
    }
    for (e, inter) in &stamp_btn {
        if edge(e, inter, &mut prev_inter) && mail.compose {
            if !mail.stamped {
                // C# StampParcel：背包须有邮票（Nothing/Shape==1）
                let has_stamp = inv_q
                    .single()
                    .map(|inv| inv.items.iter().flatten().any(is_stamp_item))
                    .unwrap_or(false);
                if has_stamp {
                    mail.stamped = true;
                } else {
                    tracing::info!("✉️ 背包无邮票，无法贴票");
                }
            } else {
                mail.stamped = false;
                // C# UpdateParcel：未贴票仅第 1 格 → 清空多余槽位
                for slot in mail.attach.iter_mut().skip(1) {
                    *slot = None;
                }
            }
            // C# StampParcel → CalculatePostage
            request_mail_cost(&net, mail.compose_gold, &mail.attach, mail.stamped);
        }
    }
}

/// #2538：C# CalculatePostage —— C.MailCost{Gold, ItemsIdx[5], Stamped}（邮资查询）
pub fn request_mail_cost(net: &NetConnection, gold: u32, attach: &[Option<u64>], stamped: bool) {
    let mut items_idx = [0u64; 5];
    for (i, slot) in attach.iter().enumerate().take(5) {
        if let Some(uid) = slot {
            items_idx[i] = *uid;
        }
    }
    net.send_packet(&mir2_shared::packets::client::mail::MailCost {
        gold,
        items_idx,
        stamped,
    });
}

/// 发送写好的邮件（C# MailComposeParcelDialog 发送 → C.SendMail{Name, Message, Gold, ItemsIdx[5], Stamped}；subject 由正文首行派生）
pub fn send_composed_mail(
    net: &NetConnection,
    input: &crate::game::dialogs::text_input::TextInputState,
    gold: u32,
    attach: &[Option<u64>],
    stamped: bool,
) {
    let to = input.texts.get(0).cloned().unwrap_or_default();
    let subject = input.texts.get(1).cloned().unwrap_or_default();
    let body = input.texts.get(2).cloned().unwrap_or_default();
    if to.is_empty() {
        tracing::warn!("✉️ 收件人为空");
        return;
    }
    let message = if body.is_empty() {
        subject.clone()
    } else {
        format!("{}\n{}", subject, body)
    };
    let mut items_idx = [0u64; 5];
    for (i, slot) in attach.iter().enumerate().take(5) {
        if let Some(uid) = slot {
            items_idx[i] = *uid;
        }
    }
    net.send_packet(&mir2_shared::packets::client::mail::SendMail {
        name: to.clone(),
        message,
        gold,
        items_idx,
        stamped,
    });
    tracing::info!(
        "✉️ 发送邮件: {} - {}（金币 {}，附件 {}，贴票 {}）",
        to,
        subject,
        gold,
        attach.iter().flatten().count(),
        stamped
    );
}

fn spawn_mail(
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

    // 邮件列表视图：根容器（透明，承载顶栏/标题/按钮/行/详情/滚动条；内容到 rel y=260）
    let list = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(280.0),
                top: Val::Px(80.0),
                width: Val::Px(360.0),
                height: Val::Px(300.0),
                ..default()
            },
            DialogRoot(DialogKind::Mail),
            MailWidget,
            GlobalZIndex(30),
            Visibility::Hidden,
            // #89 可滚动邮件列表：8 行 × 22px
            UiScrollList {
                rect_rel: (18.0, 60.0, 300.0, 176.0),
                row_h: 22.0,
                visible: 8,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (330.0, 60.0, 4.0, 176.0),
                thumb: None,
                z: 9,
            },
        ))
        .id();

    commands.entity(list).with_children(|p| {
        // 滚动条（面板子节点）
        spawn_scroll_bar_ui(p, (330.0, 60.0, 4.0, 176.0), 9);
        // 顶栏 Prguse[956] @(0,0) 252x16
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 956) {
            crate::ui::theme::spawn_image(p, h, 0.0, 0.0, 252.0, 16.0, 8);
        }
        // 标题 Title[20] @(18,8)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 20) {
            crate::ui::theme::spawn_image(p, h, 18.0, 8.0, 187.0, 20.0, 8);
        }
        // 关闭 Prguse2[360-362] @(340,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 360),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 362),
        ) {
            spawn_icon_button(p, n, h, pr, 340.0, 3.0, 20.0, 20.0, 10).insert(MailClose);
        }
        // 写邮件 / 删除 / 收取
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 300.0, 200.0, 60.0, 23.0, 10)
                .insert(MailWrite);
            spawn_icon_button(p, n.clone(), h.clone(), pr.clone(), 240.0, 226.0, 60.0, 23.0, 10)
                .insert(MailDelete);
            spawn_label(p, &cjk, "删除", 254.0, 230.0, 12.0, Color::WHITE, 11);
            spawn_icon_button(p, n, h, pr, 170.0, 226.0, 60.0, 23.0, 10).insert(MailCollect);
            spawn_label(p, &cjk, "收取", 184.0, 230.0, 12.0, Color::WHITE, 11);
        }
        // 邮件列表（8 行）@(18,60+22i)
        for i in 0..8usize {
            spawn_label(p, &cjk, "", 18.0, 60.0 + i as f32 * 22.0, 12.0, Color::WHITE, 9)
                .insert(MailLine(i));
        }
        // 内容区（正文/金币）@(18,260)
        spawn_label(p, &cjk, "", 18.0, 260.0, 12.0, Color::srgb(0.95, 0.95, 0.8), 9)
            .insert(MailDetailText);
    });

    // ---- 写邮件界面（C# MailComposeParcelDialog 覆盖层 360x430 @ (290,80)）----
    let compose = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(290.0),
                top: Val::Px(80.0),
                width: Val::Px(360.0),
                height: Val::Px(430.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            MailComposeWidget,
            DialogRoot(DialogKind::Mail),
            GlobalZIndex(40),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(compose).with_children(|p| {
        // 标签 + 输入框（收件人/主题/正文/金币）
        let fields: [(usize, &str, f32); 4] = [
            (0, "收件人:", 20.0),
            (1, "主题:", 50.0),
            (2, "正文:", 80.0),
            (3, "金币:", 110.0),
        ];
        for (id, label, y) in fields {
            spawn_label(p, &cjk, label, 10.0, y, 12.0, Color::WHITE, 10);
            spawn_container(p, 70.0, y, 270.0, 20.0, 10)
                .insert((
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                    crate::game::dialogs::text_input::TextInputField(id),
                    crate::game::dialogs::text_input::TextInputRect(360.0, y + 80.0, 270.0, 20.0),
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
                        TextColor(Color::WHITE),
                        ZIndex(11),
                        crate::game::dialogs::text_input::TextInputDisplay(id),
                    ));
                });
        }
        // 附件 5 格（C# MailComposeParcelDialog）@(10+46i,146)
        spawn_label(p, &cjk, "附件:", 10.0, 138.0, 12.0, Color::WHITE, 10);
        for i in 0..5usize {
            spawn_item_cell_ui(p, &mut images, &font, 10.0 + i as f32 * 46.0, 146.0, 40.0, 40.0, 10, i)
                .insert(MailAttachSlot(i));
        }
        // #2538：邮票按钮（C# StampButton Prguse2[203] 20x20）+ 贴票覆层 [204]
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 203) {
            spawn_container(p, 250.0, 144.0, 20.0, 20.0, 10)
                .insert((Button, ImageNode::new(h.clone()), MailStampBtn));
            crate::ui::theme::spawn_image(p, h, 250.0, 144.0, 20.0, 20.0, 11)
                .insert((MailStampOn, Visibility::Hidden));
        }
        // 邮资标签 + 值
        spawn_label(p, &cjk, "邮票:", 232.0, 152.0, 11.0, Color::WHITE, 10);
        spawn_label(p, &cjk, "", 300.0, 152.0, 11.0, Color::WHITE, 10).insert(MailCostLabel);
        // 背包选择 20 格 @(10+col*46, 202+row*46)
        spawn_label(p, &cjk, "背包选择:", 10.0, 194.0, 12.0, Color::WHITE, 10);
        for i in 0..20usize {
            let col = (i % 5) as f32;
            let row = (i / 5) as f32;
            spawn_item_cell_ui(p, &mut images, &font, 10.0 + col * 46.0, 202.0 + row * 46.0, 40.0, 40.0, 10, i)
                .insert(MailInvPick(i));
        }
        // 发送 / 取消 @(10/100,390)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 10.0, 390.0, 76.0, 25.0, 10).insert(MailSendBtn);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
        ) {
            spawn_icon_button(p, n, h, pr, 100.0, 390.0, 76.0, 25.0, 10).insert(MailCancelBtn);
        }
    });
}

fn mail_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut mail: ResMut<MailState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<(Entity, &Interaction), With<MailClose>>,
    delete_btn: Query<(Entity, &Interaction), With<MailDelete>>,
    mut collect_btn: Query<(Entity, &Interaction, &mut Visibility), With<MailCollect>>,
    mut widgets: Query<
        (&mut Visibility, Option<&MailLine>, Option<&MailDetailText>),
        (
            With<MailWidget>,
            Without<MailComposeWidget>,
            Without<MailCollect>,
        ),
    >,
    mut lines: Query<(&mut Text, &mut TextColor, &MailLine), Without<MailDetailText>>,
    mut detail_texts: Query<(&mut Text, &MailDetailText), Without<MailLine>>,
    mut scroll: Query<&mut UiScrollList, With<MailWidget>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<MailWidget>>,
) {
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Mail);
    for (mut vis, line, _det) in &mut widgets {
        if line.is_some() || _det.is_some() {
            continue;
        }
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        // 收取附件按钮（MailCollect）不在 widgets 查询里，关闭时必须隐藏
        for (_, _, mut vis) in &mut collect_btn {
            *vis = Visibility::Hidden;
        }
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::Mail);
        }
    }
    // 列表（#89 支持滚轮滚动）
    let mut sl = scroll.single_mut();
    if let Ok(sl) = sl.as_mut() {
        sl.set_total(mail.mails.len());
        let off = sl.offset;
        for (mut text, mut color, line) in &mut lines {
            let idx = off + line.0;
            text.0 = match mail.mails.get(idx) {
                Some(m) => {
                    let mark = if m.unread { "（未读）" } else { "" };
                    format!("{} - {}{}", m.sender, m.subject, mark)
                }
                None => String::new(),
            };
            let c = if mail.selected == Some(idx) {
                Color::srgb(1.0, 0.9, 0.3)
            } else {
                Color::WHITE
            };
            if color.0 != c {
                color.0 = c;
            }
        }
    }
    // 内容区
    for (mut text, _) in &mut detail_texts {
        text.0 = match mail.detail.as_ref() {
            Some(d) => {
                let mut s = format!("发件人: {}\n主题: {}\n\n{}", d.sender, d.subject, d.body);
                if d.gold > 0 {
                    s.push_str(&format!("\n金币: {}", d.gold));
                }
                if !d.items.is_empty() || d.gold > 0 {
                    if d.collected {
                        s.push_str("\n（附件待领取）");
                    } else {
                        s.push_str("\n（附件需到邮局取回）");
                    }
                }
                if !d.items.is_empty() {
                    s.push_str(&format!("\n附件: {}", d.items.join(", ")));
                }
                s
            }
            None => "点击上方邮件查看内容".to_string(),
        };
    }
    // 收取附件（#166 C# MailReadParcelDialog.CollectButton → C.CollectParcel）
    let can_collect = mail
        .detail
        .as_ref()
        .map(|d| d.collected && (!d.items.is_empty() || d.gold > 0))
        .unwrap_or(false);
    for (e, inter, mut vis) in &mut collect_btn {
        *vis = if open && can_collect {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if edge(e, inter, &mut prev_inter) && can_collect {
            if let Some(d) = mail.detail.as_ref() {
                net.send_packet(&mir2_shared::packets::client::mail::CollectParcel {
                    mail_id: d.mail_id,
                });
                tracing::info!("📦 收取附件: mail_id={}", d.mail_id);
            }
        }
    }

    // 点击列表项 → ReadMail（#89：行号 = 滚动偏移 + 可视槽位）
    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.single() else { return };
        let Some(cursor) = window.cursor_position() else {
            return;
        };
        let off = scroll.single().map(|s| s.offset).unwrap_or(0);
        let (ox, oy) = panel_origin
            .single()
            .map(|n| crate::ui::theme::node_origin(n, (280.0, 80.0)))
            .unwrap_or((280.0, 80.0));
        for i in 0..8usize {
            let y = oy + 60.0 + i as f32 * 22.0;
            if cursor.x >= ox + 18.0 && cursor.x <= ox + 320.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                if let Some(m) = mail.mails.get(off + i) {
                    let mail_id = m.mail_id;
                    let subject = m.subject.clone();
                    mail.selected = Some(off + i);
                    net.send_packet(&mir2_shared::packets::client::mail::ReadMail { mail_id });
                    tracing::info!("📧 读取邮件: {} ({})", subject, mail_id);
                }
                break;
            }
        }
    }
    // 删除邮件（#132）
    for (e, inter) in &delete_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = mail.selected {
                if let Some(m) = mail.mails.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::mail::DeleteMail {
                        mail_id: m.mail_id,
                    });
                    tracing::info!("📧 删除邮件: {} ({})", m.subject, m.mail_id);
                    mail.mails.remove(idx);
                    mail.selected = None;
                    mail.detail = None;
                }
            }
        }
    }
}

/// 消费服务端邮件事件（网络层只广播 ServerEvent）
fn mail_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mail: ResMut<MailState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        // #2538：S.MailCost → 邮资标签
        if let ServerEvent::MailCost { cost } = ev {
            mail.parcel_cost = *cost;
            tracing::debug!("✉️ 邮资更新: {}", cost);
        }
        if let ServerEvent::ParcelCollected { result } = ev {
            match *result {
                1 => {
                    // C# Result=1：邮箱领取成功 → 本地标记已领取并清空附件显示
                    if let Some(d) = mail.detail.as_mut() {
                        d.collected = true;
                        d.gold = 0;
                        d.items.clear();
                    }
                    if let Some(idx) = mail.selected {
                        if let Some(m) = mail.mails.get_mut(idx) {
                            m.collected = true;
                            m.gold = 0;
                        }
                    }
                    tracing::info!("📦 附件已领取");
                }
                0 => {
                    // C# Result=0：邮局取回成功，列表将由服务端 GetMail 刷新
                    tracing::info!("📦 已从邮局取回附件");
                }
                _ => {
                    tracing::warn!("📦 收取附件失败: result={}", result);
                }
            }
        }
        if let ServerEvent::MailReceived { entry, detail } = ev {
            // 去重：同 mail_id 已存在则替换（全文包会更新未读标记）
            if let Some(existing) = mail.mails.iter_mut().find(|m| m.mail_id == entry.mail_id) {
                *existing = entry.clone();
            } else {
                mail.mails.insert(0, entry.clone());
            }
            if let Some(d) = detail {
                mail.detail = Some(d.clone());
            }
        }
    }
}

/// 由附件槽列表生成 C# C.SendMail.items_idx[5]（空槽为 0）
pub fn build_mail_items_idx(attach: &[Option<u64>]) -> [u64; 5] {
    let mut items_idx = [0u64; 5];
    for (i, slot) in attach.iter().enumerate().take(5) {
        if let Some(uid) = slot {
            items_idx[i] = *uid;
        }
    }
    items_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_mail_items_idx_empty() {
        assert_eq!(build_mail_items_idx(&[]), [0; 5]);
        assert_eq!(
            build_mail_items_idx(&[None, None, None, None, None]),
            [0; 5]
        );
    }

    #[test]
    fn build_mail_items_idx_fills_slots() {
        assert_eq!(
            build_mail_items_idx(&[Some(7), None, Some(9)]),
            [7, 0, 9, 0, 0]
        );
        // 超过 5 个只取前 5
        let long: Vec<Option<u64>> = (1..=7u64).map(Some).collect();
        assert_eq!(build_mail_items_idx(&long), [1, 2, 3, 4, 5]);
    }

    /// #2631：ComposeMail → 邮件窗自行打开 + 预填收件人进写邮件界面（替代旧 friend 直写 MailState）。
    #[test]
    fn compose_mail_opens_and_prefills_recipient() {
        use crate::game::dialogs::text_input::TextInputState;
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(Messages::<ComposeMail>::default());
        world.insert_resource(MailState::default());
        world.insert_resource(DialogManager::default());
        world.insert_resource(TextInputState::default());
        world
            .resource_mut::<Messages<ComposeMail>>()
            .write(ComposeMail {
                to: "小明".to_string(),
            });

        world
            .run_system_once(mail_compose_request_system)
            .expect("compose request 应成功");

        let mail = world.resource::<MailState>();
        assert!(mail.compose, "应进入写邮件界面");
        assert!(mail.detail.is_none());
        assert!(mail.attach.iter().all(|s| s.is_none()));
        assert_eq!(mail.compose_gold, 0);
        assert!(
            world.resource::<DialogManager>().is_open(DialogKind::Mail),
            "邮件窗应打开"
        );
        let input = world.resource::<TextInputState>();
        assert_eq!(input.texts[0], "小明", "收件人应预填到输入框 id 0");
        assert_eq!(input.active, None, "输入框不应聚焦");
    }
}

#[cfg(test)]
mod stamp_tests {
    use super::*;
    use crate::game::dialogs::inventory::InvItem;

    fn inv(item_type: u8, shape: i16) -> InvItem {
        InvItem {
            item_type,
            shape,
            ..Default::default()
        }
    }

    /// #2538：可用附件格数（未贴票 1 格 / 贴票 5 格；C# hasStamp?5:1）
    #[test]
    fn stamp_slots_gates_attach_count() {
        assert_eq!(stamp_slots(false), 1);
        assert_eq!(stamp_slots(true), 5);
    }

    /// #2538：邮票判定（C# ItemType.Nothing && Shape==1；客户端为 Shared 值 3）
    #[test]
    fn stamp_item_detection() {
        assert!(is_stamp_item(&inv(3, 1)));
        assert!(!is_stamp_item(&inv(3, 2))); // Shape!=1
        assert!(!is_stamp_item(&inv(4, 1))); // 非 Nothing
    }
}
