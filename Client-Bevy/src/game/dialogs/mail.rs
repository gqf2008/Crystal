// ============================================================================
// 邮件对话框（M22）
// 布局参考：macroquad mail_dialog.rs / C# MailDialog
//   - 背景 Prguse[956]，标题 Title[20]，位置 (280,80)
//   - 邮件列表 y=60 起每 22px；点击列表项 → C.ReadMail{mail_id} → 内容区显示正文/金币
// 网络：ReceiveMail（新邮件条目 / 邮件全文，服务端同 opcode 双格式）→ 列表 + 详情
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::game::dialogs::text_input::{TextInputDisplay, TextInputField, TextInputRect};
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiEntity, UiFont,
    UiImageCache,
};
use crate::ui::scroll_list::{spawn_scroll_bar, ScrollList};
use crate::ui::controls::ItemCellData;

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
        }
    }
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

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>();
                app.add_systems(
            Update,
            mail_server_events.run_if(in_state(AppState::Game)),
        );
app.add_systems(OnEnter(AppState::Game), spawn_mail);
        app.add_systems(OnExit(AppState::Game), cleanup_mail);
        app.add_systems(
            Update,
            (mail_ui_system, mail_compose_system, ui_button_system)
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

/// 写邮件界面：写按钮 → 打开；附件/金币选择；发送/取消
#[allow(clippy::too_many_arguments)]
fn mail_compose_system(
    mut mail: ResMut<MailState>,
    hud: Res<crate::game::hud::HudState>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    write_btn: Query<&UiButton, With<MailWrite>>,
    send_btn: Query<&UiButton, With<MailSendBtn>>,
    cancel_btn: Query<&UiButton, With<MailCancelBtn>>,
    mut compose_widgets: Query<
        &mut Visibility,
        (With<MailComposeWidget>, Without<MailWidget>),
    >,
    mut attach_cells: Query<(&mut ItemCellData, &MailAttachSlot), Without<MailInvPick>>,
    mut pick_cells: Query<(&mut ItemCellData, &MailInvPick), Without<MailAttachSlot>>,
) {
    let open = mail.compose;
    for mut vis in &mut compose_widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for btn in &write_btn {
        if btn.clicked {
            mail.compose = true;
            mail.detail = None;
            mail.attach = vec![None; 5];
            mail.compose_gold = 0;
            if input.texts.len() < 4 {
                input.texts.resize(4, String::new());
            }
            tracing::info!("✉️ 打开写邮件");
        }
    }

    if open {
        // 附件槽图标（按 unique_id 在背包中查找）
        for (mut data, slot) in &mut attach_cells {
            let uid = mail.attach.get(slot.0).and_then(|s| *s);
            data.icon = uid.and_then(|uid| {
                hud.inventory
                    .items
                    .iter()
                    .flatten()
                    .find(|it| it.unique_id == uid)
                    .and_then(|it| {
                        ui_image(
                            &mut libs,
                            &mut images,
                            &mut cache,
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
        for (slot_idx, item) in hud.inventory.items.iter().enumerate() {
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
                Some(slot_idx) => match hud.inventory.items.get(slot_idx).and_then(|s| s.as_ref()) {
                    Some(it) => {
                        let icon = ui_image(
                            &mut libs,
                            &mut images,
                            &mut cache,
                            LibraryName::Items,
                            it.image as usize,
                        );
                        data.icon = icon;
                        data.count = if it.count > 1 { Some(it.count as u32) } else { None };
                    }
                    None => {
                        data.icon = None;
                        data.count = None;
                    }
                },
                None => {
                    data.icon = None;
                    data.count = None;
                }
            }
        }

        // 点击：附件槽 → 移除；背包格 → 填入空附件槽（C# MailComposeParcelDialog 语义）
        if mouse.just_pressed(MouseButton::Left) {
            let Ok(window) = windows.single() else { return };
            let Some(cursor) = window.cursor_position() else { return };
            for i in 0..5usize {
                let x = 300.0 + i as f32 * 46.0;
                let y = 226.0;
                if cursor.x >= x && cursor.x <= x + 40.0 && cursor.y >= y && cursor.y <= y + 40.0 {
                    if mail.attach.get(i).is_some_and(|s| s.is_some()) {
                        mail.attach[i] = None;
                    }
                    return;
                }
            }
            for (cell_idx, slot_idx) in pick_slots.iter().enumerate() {
                if let Some(slot_idx) = slot_idx {
                    let col = (cell_idx % 5) as f32;
                    let row = (cell_idx / 5) as f32;
                    let x = 300.0 + col * 46.0;
                    let y = 282.0 + row * 46.0;
                    if cursor.x >= x && cursor.x <= x + 40.0 && cursor.y >= y && cursor.y <= y + 40.0 {
                        if let Some(it) = hud.inventory.items.get(*slot_idx).and_then(|s| s.as_ref()) {
                            if let Some(empty) = mail.attach.iter_mut().find(|s| s.is_none()) {
                                *empty = Some(it.unique_id);
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    for btn in &send_btn {
        if btn.clicked && mail.compose {
            let gold = input
                .texts
                .get(3)
                .cloned()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            send_composed_mail(&net, &input, gold, &mail.attach);
            mail.compose = false;
            mail.attach = vec![None; 5];
            mail.compose_gold = gold;
            input.active = None;
        }
    }
    for btn in &cancel_btn {
        if btn.clicked && mail.compose {
            mail.compose = false;
            mail.attach = vec![None; 5];
            input.active = None;
        }
    }
}

/// 发送写好的邮件（C# MailComposeParcelDialog 发送 → C.SendMail{Name, Message, Gold, ItemsIdx[5]}；subject 由正文首行派生）
pub fn send_composed_mail(
    net: &NetConnection,
    input: &crate::game::dialogs::text_input::TextInputState,
    gold: u32,
    attach: &[Option<u64>],
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
        stamped: false,
    });
    tracing::info!(
        "✉️ 发送邮件: {} - {}（金币 {}，附件 {}）",
        to,
        subject,
        gold,
        attach.iter().flatten().count()
    );
}


fn spawn_mail(
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 956) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        // #89 可滚动邮件列表：8 行 × 22px
        let (track, thumb) = spawn_scroll_bar(&mut commands, &mut images, (610.0, 140.0, 4.0, 176.0), 6.3);
        commands.entity(track).insert((DialogRoot(DialogKind::Mail), MailWidget, Visibility::Visible));
        commands.entity(thumb).insert((
            DialogRoot(DialogKind::Mail),
            MailWidget,
            Visibility::Visible,
        ));
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mail),
            MailWidget,
            Visibility::Hidden,
            ScrollList {
                rect_rel: (18.0, 60.0, 300.0, 176.0),
                row_h: 22.0,
                visible: 8,
                total: 0,
                offset: 0,
                step: 3,
                track_rel: (330.0, 60.0, 4.0, 176.0),
                thumb: Some(thumb),
                z: 8.0,
            },
        ));
    }
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 20) {
        let e = spawn_ui_sprite(&mut commands, h, 298.0, 88.0, 6.2, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::Mail),
            MailWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 340.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            MailClose,
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    // 写邮件按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        280.0 + 300.0, 280.0, 7.0, 60.0, 23.0,
    ) {
        commands.entity(e).insert((
            MailWrite,
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    // 删除邮件按钮（#132 C# MailListDialog 删除）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        520.0, 306.0, 8.3, 60.0, 23.0,
    ) {
        commands.entity(e).insert((
            MailDelete,
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    let t = spawn_ui_text(&mut commands, &font, "删除", 534.0, 310.0, 12.0, Color::WHITE, 8.4);

    // 收取附件按钮（#166 C# MailReadParcelDialog.CollectButton → C.CollectParcel）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        450.0, 306.0, 8.3, 60.0, 23.0,
    ) {
        commands.entity(e).insert((
            MailCollect,
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    let tc = spawn_ui_text(&mut commands, &font, "收取", 464.0, 310.0, 12.0, Color::WHITE, 8.4);
    commands.entity(tc).insert((DialogRoot(DialogKind::Mail), MailWidget));
    commands.entity(t).insert((DialogRoot(DialogKind::Mail), MailWidget));

    // 邮件列表（8 行）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 140.0 + i as f32 * 22.0,
            12.0, Color::WHITE, 8.0,
        );
        commands.entity(e).insert((
            MailLine(i),
            DialogRoot(DialogKind::Mail),
            MailWidget,
        ));
    }
    // 内容区（正文/金币）
    let detail = spawn_ui_text(
        &mut commands, &font, "",
        298.0, 340.0, 12.0, Color::srgb(0.95, 0.95, 0.8), 8.0,
    );
    commands.entity(detail).insert((
        MailDetailText,
        DialogRoot(DialogKind::Mail),
        MailWidget,
    ));

    // ---- 写邮件界面（C# MailComposeParcelDialog：收件人/主题/正文/金币 + 附件 5 格 + 背包选择）----
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    let compose_bg = commands
        .spawn((
            UiEntity,
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
            Sprite {
                image: white.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                custom_size: Some(Vec2::new(360.0, 430.0)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(290.0, -80.0, 8.0),
            Visibility::Hidden,
        ))
        .id();
    // 标签 + 输入框（收件人/主题/正文/金币）
    let fields: [(usize, &str, f32); 4] = [
        (0, "收件人:", 100.0),
        (1, "主题:", 130.0),
        (2, "正文:", 160.0),
        (3, "金币:", 190.0),
    ];
    for (id, label, y) in fields {
        let t = spawn_ui_text(
            &mut commands, &font, label,
            300.0, y, 12.0, Color::WHITE, 8.1,
        );
        commands.entity(t).insert((DialogRoot(DialogKind::Mail), MailComposeWidget));
        let box_e = commands
            .spawn((
                UiEntity,
                DialogRoot(DialogKind::Mail),
                MailComposeWidget,
                TextInputField(id),
                TextInputRect(360.0, y, 270.0, 20.0),
                Sprite {
                    image: white.clone(),
                    color: Color::srgba(0.2, 0.2, 0.25, 0.9),
                    custom_size: Some(Vec2::new(270.0, 20.0)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(360.0, -y, 8.1),
                Visibility::Hidden,
            ))
            .id();
        commands.entity(box_e).with_children(|p| {
            p.spawn((
                TextInputDisplay(id),
                Text2d::new(String::new()),
                Anchor::TOP_LEFT,
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
    // 附件 5 格（C# MailComposeParcelDialog）
    let t = spawn_ui_text(&mut commands, &font, "附件:", 300.0, 218.0, 12.0, Color::WHITE, 8.1);
    commands.entity(t).insert((DialogRoot(DialogKind::Mail), MailComposeWidget));
    for i in 0..5usize {
        let e = crate::ui::controls::spawn_item_cell(
            &mut commands, &mut images, &font,
            300.0 + i as f32 * 46.0, 226.0, 8.2, 40.0, 40.0, i,
        );
        commands.entity(e).insert((
            MailAttachSlot(i),
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
            Visibility::Visible,
        ));
    }
    // 背包物品选择（最多 20 格）
    let t = spawn_ui_text(&mut commands, &font, "背包物品:", 300.0, 274.0, 12.0, Color::WHITE, 8.1);
    commands.entity(t).insert((DialogRoot(DialogKind::Mail), MailComposeWidget));
    for i in 0..20usize {
        let col = (i % 5) as f32;
        let row = (i / 5) as f32;
        let e = crate::ui::controls::spawn_item_cell(
            &mut commands, &mut images, &font,
            300.0 + col * 46.0, 282.0 + row * 46.0, 8.2, 40.0, 40.0, 100 + i,
        );
        commands.entity(e).insert((
            MailInvPick(i),
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
            Visibility::Visible,
        ));
    }
    let _ = compose_bg;
    // 发送 / 取消（C# MailComposeParcelDialog 底部）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        300.0, 470.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            MailSendBtn,
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 210, 211, 212,
        390.0, 470.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            MailCancelBtn,
            DialogRoot(DialogKind::Mail),
            MailComposeWidget,
        ));
    }
}

/// 显示/隐藏 + 列表渲染 + 内容区 + 点击读取
#[allow(clippy::too_many_arguments)]
fn mail_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut mail: ResMut<MailState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<&UiButton, With<MailClose>>,
    delete_btn: Query<&UiButton, With<MailDelete>>,
    mut collect_btn: Query<(&UiButton, &mut Visibility), With<MailCollect>>,
    mut widgets: Query<
        (&mut Visibility, Option<&MailLine>, Option<&MailDetailText>),
        (With<MailWidget>, Without<MailComposeWidget>, Without<MailCollect>),
    >,
    mut lines: Query<(&mut Text2d, &mut TextColor, &MailLine), Without<MailDetailText>>,
    mut detail_texts: Query<(&mut Text2d, &MailDetailText), Without<MailLine>>,
    mut scroll: Query<&mut ScrollList, With<MailWidget>>,
) {
    let open = mgr.is_open(DialogKind::Mail);
    for (mut vis, line, _det) in &mut widgets {
        if line.is_some() || _det.is_some() {
            continue;
        }
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        // 收取附件按钮（MailCollect）不在 widgets 查询里，关闭时必须隐藏
        for (_, mut vis) in &mut collect_btn {
            *vis = Visibility::Hidden;
        }
        return;
    }
    for btn in &close {
        if btn.clicked {
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
                if d.collected {
                    s.push_str("\n（附件已收取）");
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
        .map(|d| !d.collected && (!d.items.is_empty() || d.gold > 0))
        .unwrap_or(false);
    for (btn, mut vis) in &mut collect_btn {
        *vis = if open && can_collect {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if btn.clicked && can_collect {
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
        let Some(cursor) = window.cursor_position() else { return };
        let off = scroll.single().map(|s| s.offset).unwrap_or(0);
        for i in 0..8usize {
            let y = 140.0 + i as f32 * 22.0;
            if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 20.0 {
                if let Some(m) = mail.mails.get(off + i) {
                    let mail_id = m.mail_id;
                    let subject = m.subject.clone();
                    mail.selected = Some(off + i);
                    net.send_packet(&mir2_shared::packets::client::mail::ReadMail {
                        mail_id,
                    });
                    tracing::info!("📧 读取邮件: {} ({})", subject, mail_id);
                }
                break;
            }
        }
    }
    // 删除邮件（#132）
    for btn in &delete_btn {
        if btn.clicked {
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
        if let ServerEvent::ParcelCollected { result } = ev {
            if *result == 1 {
                // C# Result=1：收取成功 → 本地标记已收取并清空附件显示
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
                tracing::info!("📦 附件已收取");
            } else {
                tracing::warn!("📦 收取附件失败: result={}", result);
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
        assert_eq!(build_mail_items_idx(&[None, None, None, None, None]), [0; 5]);
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
}
