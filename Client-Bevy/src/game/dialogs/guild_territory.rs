// ============================================================================
// 行会领地对话框（M36；#2892 批C 按 C# 整窗重建）
//
// 布局基准：C# `Client/MirScenes/Dialogs/GuildTerritoryDialog .cs`
//   - 面板 `Prguse[680]`（原生 568x241），**无 Location**（默认 (0,0)；`GameScene.cs:319`
//     里 `Parent = this`），`Movable = true`（`:89-92`）
//   - 标题 `Title[54]`(133x15) @(217,11)、关闭 `Prguse[361/362/363]`(16x15) @(544,8)
//   - 翻页 `Prguse2[240..242]`(16x16) @(214,213)、`[243..245]` @(317,213)
//   - 邮件 `Prguse[437..439]`(28x25) @(262,208)；购买同帧 @(292,208)（默认 `Visible=false`，
//     选中行状态为「正在出售」才显示，`:143-192`）
//   - 表头 5 列 @(15/60/230/380/480, 38)（Goldenrod 8F）
//   - 7 行 `GTRow`(550x17) @(5, 60+20i)，行内 5 列标签 (15/45/150/365/460, 0)，选中行 1px Lime 边框
//   - 宣战入口**不在本窗**：C# 由 NPC `RequestWarKey` → `S.GuildRequestWar` → `MirInputBox`
//     （见 `dialogs/input_box.rs`）；本窗原先自造的「宣战」按钮/输入框已删除
// 网络（ServerRust wire）：
//   C: GuildTerritoryPage[page u32] / PurchaseGuildTerritory[territory_id u32]
//   S: GuildTerritoryPage(276) = C# `length + count + ClientGTMap[]`（Rust 每项前置领地 id）
// ============================================================================

use bevy::prelude::*;

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_icon_button, spawn_image, spawn_label, spawn_panel, CloseButton,
};

// ---------------------------------------------------------------------------
// C# 几何常量（`GuildTerritoryDialog .cs:89-215`）
// ---------------------------------------------------------------------------
pub const PANEL_INDEX: usize = 680;
pub const PANEL_W: f32 = 568.0;
pub const PANEL_H: f32 = 241.0;
/// C# **未设 Location** → `MirControl` 默认 (0,0)（不是居中，也不是 (280,80)）
pub const PANEL_ORIGIN: (f32, f32) = (0.0, 0.0);
pub const TITLE_POS: (f32, f32) = (217.0, 11.0);
pub const TITLE_SIZE: (f32, f32) = (133.0, 15.0);
pub const CLOSE_POS: (f32, f32) = (544.0, 8.0);
pub const CLOSE_SIZE: (f32, f32) = (16.0, 15.0);
pub const PREV_POS: (f32, f32) = (214.0, 213.0);
pub const NEXT_POS: (f32, f32) = (317.0, 213.0);
pub const PAGE_SIZE: (f32, f32) = (16.0, 16.0);
pub const MAIL_POS: (f32, f32) = (262.0, 208.0);
pub const BUY_POS: (f32, f32) = (292.0, 208.0);
pub const ACTION_SIZE: (f32, f32) = (28.0, 25.0);
pub const HEADER_Y: f32 = 38.0;
pub const HEADER_X: [f32; 5] = [15.0, 60.0, 230.0, 380.0, 480.0];
/// C# 表头文案（`:199-262`，取自 `Client/Localization/Chinese.json`）
pub const HEADERS: [&str; 5] = ["GT #", "所属公会", "公会会长", "GT 状态", "GT 价格"];
pub const ROW_COUNT: usize = 7;
pub const ROW_X: f32 = 5.0;
pub const ROW_Y0: f32 = 60.0;
pub const ROW_STEP: f32 = 20.0;
pub const ROW_W: f32 = 550.0;
pub const ROW_H: f32 = 17.0;
/// 行内五列左边界：Idx(15)/GuildOwner(45)/OwnerName(150)/Status(365)/Price(460)
pub const ROW_LABEL_X: [f32; 5] = [15.0, 45.0, 150.0, 365.0, 460.0];
/// C# `ClientTextKeys.None`（Chinese.json「无」）
pub const NONE_TEXT: &str = "无";

const COLOR_GOLDENROD: Color = Color::srgb(0.855, 0.647, 0.125);
/// C# `GTRow.BorderColour = Color.Lime`（选中行 1px 边框）
const COLOR_ROW_BORDER: Color = Color::srgb(0.0, 1.0, 0.0);

/// C# `GuildTerritoryDialog.UpdateInterface`（`:318-365`）状态列文案：
/// 无主 →「可用」`Available`；有主且 `begin > 0` →「出售待定」`SalePending`；
/// 有主且 `price > 0` →「正在出售」`ForSale`；其余 →「不可用」`Unavailable`。
pub fn territory_status(owner: &str, price: i32, begin: i32) -> &'static str {
    if owner.is_empty() || owner == NONE_TEXT {
        "可用"
    } else if begin > 0 {
        "出售待定"
    } else if price > 0 {
        "正在出售"
    } else {
        "不可用"
    }
}

/// C# `gtMap.price.ToString("###,###,##0")`（千分位、无小数、0 → "0"）
pub fn format_price(price: i32) -> String {
    let neg = price < 0;
    let digits = price.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg {
        out.insert(0, '-');
    }
    out
}

/// C# `gtRow.OwnerName.Text = Leader`（+ `AndPlaceholder` =「和 {0}」当 `Leader2` 非空）
pub fn owner_name_text(leader: &str, leader2: &str) -> String {
    if leader2.is_empty() {
        leader.to_string()
    } else {
        format!("{leader} 和 {leader2}")
    }
}

/// C# `gtRow.Idx.Text = ((i + 1) + Page * 7).ToString()`
pub fn gt_index_text(row: usize, page: usize) -> String {
    (row + 1 + page * ROW_COUNT).to_string()
}

/// 领地行（`S.GuildTerritoryPage` / C# `ClientGTMap` 写入）
#[derive(Debug, Clone, Default)]
pub struct TerritoryRow {
    /// Rust 扩展：领地 ID（购买用）
    pub id: i32,
    /// C# `ClientGTMap.index`
    pub map_index: i32,
    /// C# `ClientGTMap.Name`
    pub name: String,
    /// C# `ClientGTMap.Owner`（无主为空串）
    pub owner: String,
    /// C# `ClientGTMap.Leader`
    pub leader: String,
    /// C# `ClientGTMap.Leader2`
    pub leader2: String,
    /// C# `ClientGTMap.price`
    pub price: i32,
    /// C# `ClientGTMap.days`
    pub days: i32,
    /// C# `ClientGTMap.begin`（租期剩余秒）
    pub begin: i32,
}

impl TerritoryRow {
    /// C# 状态列文案
    pub fn status(&self) -> &'static str {
        territory_status(&self.owner, self.price, self.begin)
    }

    /// C# `BuyButton.Visible = GT.Status.Text == ForSale`（`:188`）
    pub fn for_sale(&self) -> bool {
        self.status() == "正在出售"
    }
}

/// 行会领地状态
#[derive(Resource, Default)]
pub struct GuildTerritoryState {
    pub rows: Vec<TerritoryRow>,
    pub page: usize,
    pub selected: Option<usize>,
    /// 服务端条目总数（C# `Lenght`，翻页判定用）
    pub length: i32,
    /// 非 C# 提示文案（C# 窗口内无文本行；保留供日志/自动化）
    pub message: String,
}

#[derive(Component)]
pub struct GuildTerritoryWidget;

#[derive(Component)]
pub struct GuildTerritoryClose;

#[derive(Component)]
pub struct GuildTerritoryBuy;

/// #2786：发送邮件给公会会长（C# `GuildTerritoryDialog .cs:150-168`）
#[derive(Component)]
pub struct GuildTerritoryMail;

#[derive(Component)]
pub struct GuildTerritoryPrev;

#[derive(Component)]
pub struct GuildTerritoryNext;

/// 领地行容器（整行可点；C# `GTRow.Click`）
#[derive(Component)]
pub struct GuildTerritoryRow(pub usize);

/// 行内文字格（C# `GTRow` 的 Idx/GuildOwner/OwnerName/Status/Price）
#[derive(Component)]
pub struct GuildTerritoryCell {
    pub row: usize,
    pub field: u8,
}

pub struct GuildTerritoryPlugin;

impl Plugin for GuildTerritoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildTerritoryState>();
        app.add_systems(
            Update,
            territory_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_guild_territory);
        app.add_systems(OnExit(AppState::Game), cleanup_guild_territory);
        app.add_systems(
            Update,
            guild_territory_ui_system.run_if(in_state(AppState::Game)),
        );
        // #2786：邮件会长钮（独立系统——`guild_territory_ui_system` 参数已到上限）
        app.add_systems(
            Update,
            guild_territory_mail_system.run_if(in_state(AppState::Game)),
        );
        // #2892 批C：行选中 + 行文字/边框 + 购买键显隐（独立系统，避免参数超限）
        app.add_systems(
            Update,
            guild_territory_row_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_guild_territory(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_guild_territory(
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
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 `Prguse[680]`（C# 未设 Location → (0,0)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, PANEL_INDEX) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        PANEL_ORIGIN.0,
        PANEL_ORIGIN.1,
        PANEL_W,
        PANEL_H,
        30,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::GuildTerritory), GuildTerritoryWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 `Title[54]` @(217,11)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 54) {
            spawn_image(
                p,
                h,
                TITLE_POS.0,
                TITLE_POS.1,
                TITLE_SIZE.0,
                TITLE_SIZE.1,
                8,
            );
        }
        // 关闭 `Prguse[361..363]` @(544,8)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 361),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 362),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 363),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                CLOSE_POS.0,
                CLOSE_POS.1,
                CLOSE_SIZE.0,
                CLOSE_SIZE.1,
                10,
            )
            .insert((
                GuildTerritoryClose,
                CloseButton,
                crate::ui::tooltip::UiHint {
                    text: "退出".to_string(),
                },
            ));
        }
        // 表头 5 列（Goldenrod，C# `Font(..., 8F)` → 10px 近似）
        for (i, text) in HEADERS.iter().enumerate() {
            spawn_label(
                p,
                &cjk,
                text,
                HEADER_X[i],
                HEADER_Y,
                10.0,
                COLOR_GOLDENROD,
                9,
            );
        }
        // 7 行（`GTRow` 550x17 @(5, 60+20i)），行内 5 列
        for i in 0..ROW_COUNT {
            p.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(ROW_X),
                    top: Val::Px(ROW_Y0 + i as f32 * ROW_STEP),
                    width: Val::Px(ROW_W),
                    height: Val::Px(ROW_H),
                    border: UiRect::all(Val::Px(0.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(COLOR_ROW_BORDER),
                ZIndex(9),
                GuildTerritoryRow(i),
            ))
            .with_children(|row| {
                for (field, x) in ROW_LABEL_X.iter().enumerate() {
                    spawn_label(row, &cjk, "", *x, 0.0, 11.0, Color::WHITE, 9).insert(
                        GuildTerritoryCell {
                            row: i,
                            field: field as u8,
                        },
                    );
                }
            });
        }
        // 翻页 `Prguse2[240..242]` / `[243..245]` @(214/317,213)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 240),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 241),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 242),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                PREV_POS.0,
                PREV_POS.1,
                PAGE_SIZE.0,
                PAGE_SIZE.1,
                10,
            )
            .insert((
                GuildTerritoryPrev,
                crate::ui::tooltip::UiHint {
                    text: "上一页".to_string(),
                },
            ));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 243),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 244),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 245),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                NEXT_POS.0,
                NEXT_POS.1,
                PAGE_SIZE.0,
                PAGE_SIZE.1,
                10,
            )
            .insert((
                GuildTerritoryNext,
                crate::ui::tooltip::UiHint {
                    text: "下一页".to_string(),
                },
            ));
        }
        // 邮件 / 购买（同 `Prguse[437..439]` 28x25）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 437),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 438),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 439),
        ) {
            spawn_icon_button(
                p,
                n.clone(),
                h.clone(),
                pr.clone(),
                MAIL_POS.0,
                MAIL_POS.1,
                ACTION_SIZE.0,
                ACTION_SIZE.1,
                10,
            )
            .insert((
                GuildTerritoryMail,
                crate::ui::tooltip::UiHint {
                    text: "发送邮件给公会会长".to_string(),
                },
            ));
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                BUY_POS.0,
                BUY_POS.1,
                ACTION_SIZE.0,
                ACTION_SIZE.1,
                10,
            )
            .insert((
                GuildTerritoryBuy,
                // C# `BuyButton.Visible = false`（构造默认）
                Visibility::Hidden,
                crate::ui::tooltip::UiHint {
                    text: "购买".to_string(),
                },
            ));
        }
    });
}

/// 显隐 + 关闭 + 翻页（C# `prevButton/nextButton.Click` → `C.GuildTerritoryPage{Page}`）
fn guild_territory_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<GuildTerritoryState>,
    net: Res<NetConnection>,
    close: Query<(Entity, &Interaction), With<GuildTerritoryClose>>,
    prev_btn: Query<(Entity, &Interaction), With<GuildTerritoryPrev>>,
    next_btn: Query<(Entity, &Interaction), With<GuildTerritoryNext>>,
    buy_btn: Query<(Entity, &Interaction), With<GuildTerritoryBuy>>,
    mut widgets: Query<&mut Visibility, With<GuildTerritoryWidget>>,
    mut requested: Local<bool>,
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
    let open = mgr.is_open(DialogKind::GuildTerritory);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求第 0 页（C# `GuildTerritoryDialog.Show → GetGuildTerritories(0)`）
    if !*requested {
        *requested = true;
        state.page = 0;
        state.selected = None;
        net.send_packet(&crate::network::GuildTerritoryPageWire { page: 0 });
        tracing::info!("🏯 请求行会领地列表");
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            mgr.close(DialogKind::GuildTerritory);
        }
    }
    // 翻页：C# `(Page-1) >= 0` / `((Page+1)*7) < Lenght`
    for (e, inter) in &prev_btn {
        if edge(e, inter, &mut prev_inter) && state.page > 0 {
            state.page -= 1;
            state.selected = None;
            net.send_packet(&crate::network::GuildTerritoryPageWire {
                page: state.page as u32,
            });
        }
    }
    for (e, inter) in &next_btn {
        let next_start = ((state.page + 1) * ROW_COUNT) as i32;
        let total = state.length.max(state.rows.len() as i32);
        if edge(e, inter, &mut prev_inter) && next_start < total {
            state.page += 1;
            state.selected = None;
            net.send_packet(&crate::network::GuildTerritoryPageWire {
                page: state.page as u32,
            });
        }
    }
    // 购买：C# `BuyButton.Click` —— 仅「正在出售」的行可买（`:180-190`）
    for (e, inter) in &buy_btn {
        if edge(e, inter, &mut prev_inter) {
            match state.selected.and_then(|idx| state.rows.get(idx)) {
                Some(r) if r.for_sale() => {
                    let territory_id = r.id as u32;
                    net.send_packet(&crate::network::PurchaseGuildTerritoryWire { territory_id });
                    tracing::info!("🏯 购买领地 #{}", territory_id);
                    // 服务端无广播 → 重取当前页
                    net.send_packet(&crate::network::GuildTerritoryPageWire {
                        page: state.page as u32,
                    });
                }
                Some(_) => {
                    state.message = "该领地未在出售".to_string();
                }
                None => {
                    state.message = "请先点击选中一个领地".to_string();
                }
            }
        }
    }
}

/// 行选中（C# `GTRow.Click`）+ 行文字/选中边框 + 购买键显隐（C# `UpdateInterface`）
#[allow(clippy::too_many_arguments)]
fn guild_territory_row_system(
    mgr: Res<DialogManager>,
    mut state: ResMut<GuildTerritoryState>,
    rows: Query<(Entity, &Interaction, &GuildTerritoryRow)>,
    mut cells: Query<(&mut Text, &GuildTerritoryCell)>,
    mut row_nodes: Query<(&GuildTerritoryRow, &mut Node)>,
    mut buy_vis: Query<
        &mut Visibility,
        (
            With<GuildTerritoryBuy>,
            Without<GuildTerritoryWidget>,
            Without<GuildTerritoryRow>,
        ),
    >,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    if !mgr.is_open(DialogKind::GuildTerritory) {
        return;
    }
    // 行点击 → 选中（C# `Reset()` 先清空，再把点中的行 `Border = true`）
    for (e, inter, row) in &rows {
        let was = prev_inter.insert(e, *inter);
        if *inter == Interaction::Pressed && was != Some(Interaction::Pressed) {
            let idx = state.page * ROW_COUNT + row.0;
            if idx < state.rows.len() {
                state.selected = Some(idx);
                tracing::info!("🏯 选中领地 #{}", state.rows[idx].id);
            }
        }
    }
    // 行文字（C# 五列）
    for (mut text, cell) in &mut cells {
        let idx = state.page * ROW_COUNT + cell.row;
        let line = match state.rows.get(idx) {
            Some(r) => match cell.field {
                0 => gt_index_text(cell.row, state.page),
                1 => {
                    if r.owner.is_empty() {
                        NONE_TEXT.to_string()
                    } else {
                        r.owner.clone()
                    }
                }
                2 => owner_name_text(&r.leader, &r.leader2),
                3 => r.status().to_string(),
                _ => format_price(r.price),
            },
            None => String::new(),
        };
        if text.0 != line {
            text.0 = line;
        }
    }
    // 选中边框（C# `GTRow.Border = true`，1px Lime）
    for (row, mut node) in &mut row_nodes {
        let selected = state
            .selected
            .map(|idx| idx == state.page * ROW_COUNT + row.0)
            .unwrap_or(false);
        let want = if selected {
            UiRect::all(Val::Px(1.0))
        } else {
            UiRect::all(Val::Px(0.0))
        };
        if node.border != want {
            node.border = want;
        }
    }
    // 购买键显隐（C# `BuyButton.Visible = Status == ForSale`）
    let show_buy = state
        .selected
        .and_then(|idx| state.rows.get(idx))
        .map(|r| r.for_sale())
        .unwrap_or(false);
    for mut vis in &mut buy_vis {
        let want = if show_buy {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// #2786：发送邮件给公会会长（C# `GuildTerritoryDialog .cs:162-168`：
/// 取选中行 → `ComposeMail(GT.Owner1)`，`Owner1` = `gtMap.Leader`）。
///
/// 独立系统：`guild_territory_ui_system` 的参数已到 Bevy 上限（16）。
fn guild_territory_mail_system(
    mut state: ResMut<GuildTerritoryState>,
    mail_btn: Query<(Entity, &Interaction), With<GuildTerritoryMail>>,
    mut compose: MessageWriter<crate::game::dialogs::mail::ComposeMail>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    for (e, inter) in &mail_btn {
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        let leader = state
            .selected
            .and_then(|idx| state.rows.get(idx))
            .map(|r| r.leader.clone())
            .unwrap_or_default();
        if leader.is_empty() || leader == NONE_TEXT {
            state.message = "请先选中一个已有会长的领地".to_string();
            continue;
        }
        compose.write(crate::game::dialogs::mail::ComposeMail {
            to: leader.clone(),
            message: None,
        });
        tracing::info!("🏯 写信给领地会长 {}", leader);
    }
}

/// 消费服务端领地事件（网络层只广播 ServerEvent）
fn territory_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut territory: ResMut<GuildTerritoryState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        if let ServerEvent::TerritoryList { rows } = ev {
            territory.rows = rows.clone();
            territory.length = rows.len() as i32;
            // C# `UpdateInterface()` 后跟 `Reset()`：列表刷新即清空选中
            territory.selected = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn row(id: i32, owner: &str, leader: &str, price: i32, begin: i32) -> TerritoryRow {
        TerritoryRow {
            id,
            map_index: 0,
            name: String::new(),
            owner: owner.to_string(),
            leader: leader.to_string(),
            leader2: String::new(),
            price,
            days: 0,
            begin,
        }
    }

    /// #2892 批C：`S.GuildTerritoryPage` 线格式（C# `ClientGTMap` 字段序 + Rust 前置 id）往返无损
    #[test]
    fn gt_page_wire_roundtrip_keeps_csharp_fields() {
        use mir2_shared::packets::base::Packet;
        use mir2_shared::packets::server::special_systems::{GuildTerritoryPage, TerritoryInfo};
        let page = GuildTerritoryPage {
            length: 3,
            territories: vec![
                TerritoryInfo {
                    id: 7,
                    index: 5,
                    name: "GT 地图".to_string(),
                    owner: "行会A".to_string(),
                    leader: "会长甲".to_string(),
                    leader2: "副会长乙".to_string(),
                    price: 1_000_000,
                    days: 3,
                    begin: 7200,
                },
                TerritoryInfo::default(),
            ],
        };
        let mut buf = Vec::new();
        page.write_body(&mut buf).expect("write_body");
        let mut cur = std::io::Cursor::new(&buf);
        let back = GuildTerritoryPage::read_body(&mut cur).expect("read_body");
        assert_eq!(back.length, 3, "C# `length`（总数）应与条目数分开");
        assert_eq!(back.territories.len(), 2);
        let r = &back.territories[0];
        assert_eq!(
            (r.id, r.index, r.price, r.days, r.begin),
            (7, 5, 1_000_000, 3, 7200)
        );
        assert_eq!(
            (
                r.name.as_str(),
                r.owner.as_str(),
                r.leader.as_str(),
                r.leader2.as_str()
            ),
            ("GT 地图", "行会A", "会长甲", "副会长乙"),
            "字符串字段（Name/Owner/Leader/Leader2）往返应无损"
        );
        assert_eq!(back.territories[1].id, 0, "空行也应按字段序完整解析");
    }

    /// #2892 批C：状态列逐条对齐 C# `UpdateInterface`（`:329-352`）
    #[test]
    fn status_mapping_matches_csharp() {
        assert_eq!(territory_status("", 0, 0), "可用", "无主 → Available");
        assert_eq!(territory_status(NONE_TEXT, 0, 0), "可用");
        // 有主且租期未到期 → SalePending（优先于 price）
        assert_eq!(territory_status("行会甲", 100, 60), "出售待定");
        assert_eq!(territory_status("行会甲", 0, 60), "出售待定");
        // 有主、租期到（begin=0）、挂售价>0 → ForSale
        assert_eq!(territory_status("行会甲", 1_000_000, 0), "正在出售");
        // 有主、无挂售 → Unavailable
        assert_eq!(territory_status("行会甲", 0, 0), "不可用");
    }

    /// C# `gtMap.price.ToString("###,###,##0")`
    #[test]
    fn price_format_matches_csharp() {
        assert_eq!(format_price(0), "0");
        assert_eq!(format_price(999), "999");
        assert_eq!(format_price(1_000), "1,000");
        assert_eq!(format_price(10_000_000), "10,000,000");
        assert_eq!(format_price(1_234_567), "1,234,567");
        assert_eq!(format_price(-2_500), "-2,500");
    }

    /// C# `OwnerName`：`Leader`（+`AndPlaceholder`「和 {0}」）
    #[test]
    fn owner_name_uses_leader_and_leader2() {
        assert_eq!(owner_name_text("会长甲", ""), "会长甲");
        assert_eq!(owner_name_text("会长甲", "副会长乙"), "会长甲 和 副会长乙");
    }

    /// C# `gtRow.Idx.Text = (i + 1) + Page * 7`
    #[test]
    fn gt_index_text_offsets_by_page() {
        assert_eq!(gt_index_text(0, 0), "1");
        assert_eq!(gt_index_text(6, 0), "7");
        assert_eq!(gt_index_text(0, 1), "8");
        assert_eq!(gt_index_text(6, 2), "21");
    }

    /// C# 几何：面板 568x241 @(0,0)、表头/行/按钮坐标
    #[test]
    fn geometry_matches_csharp() {
        assert_eq!((PANEL_W, PANEL_H), (568.0, 241.0));
        assert_eq!(PANEL_ORIGIN, (0.0, 0.0), "C# 未设 Location → 默认 (0,0)");
        assert_eq!((TITLE_POS, TITLE_SIZE), ((217.0, 11.0), (133.0, 15.0)));
        assert_eq!(CLOSE_POS, (544.0, 8.0));
        assert_eq!(PREV_POS, (214.0, 213.0));
        assert_eq!(NEXT_POS, (317.0, 213.0));
        assert_eq!(MAIL_POS, (262.0, 208.0));
        assert_eq!(BUY_POS, (292.0, 208.0));
        assert_eq!(HEADER_X, [15.0, 60.0, 230.0, 380.0, 480.0]);
        assert_eq!(HEADER_Y, 38.0);
        assert_eq!((ROW_X, ROW_Y0, ROW_STEP), (5.0, 60.0, 20.0));
        assert_eq!((ROW_W, ROW_H), (550.0, 17.0));
        assert_eq!(ROW_LABEL_X, [15.0, 45.0, 150.0, 365.0, 460.0]);
        // 第 7 行底边 60 + 6*20 + 17 = 197 ≤ 面板高 241
        assert!(ROW_Y0 + 6.0 * ROW_STEP + ROW_H <= PANEL_H);
    }

    /// C# `BuyButton.Visible = Status == ForSale`：仅「正在出售」可买
    #[test]
    fn buy_only_for_listed_rows() {
        assert!(!row(1, "", "", 0, 0).for_sale(), "无主领地（走 NPC BUYGT）");
        assert!(
            !row(2, "行会甲", "会长甲", 0, 0).for_sale(),
            "无挂售 → 不可用"
        );
        assert!(row(3, "行会甲", "会长甲", 500, 0).for_sale(), "挂售中");
        assert!(
            !row(4, "行会甲", "会长甲", 500, 60).for_sale(),
            "租期未到期 → 出售待定，不显购买键"
        );
    }

    /// #2786：点「发送邮件给公会会长」→ 给选中领地**会长**写 `ComposeMail`
    ///（C# `GuildTerritoryDialog .cs:162-168`：`ComposeMail(GT.Owner1)`，`Owner1 = Leader`）
    #[test]
    fn mail_button_composes_mail_to_territory_owner() {
        let mut world = World::new();
        let mut st = GuildTerritoryState::default();
        st.rows = vec![row(7, "行会甲", "会长甲", 0, 0), row(8, "", "", 0, 0)];
        st.selected = Some(0);
        world.insert_resource(st);
        world.insert_resource(Messages::<crate::game::dialogs::mail::ComposeMail>::default());
        world.spawn((GuildTerritoryMail, Interaction::Pressed));
        world
            .run_system_once(guild_territory_mail_system)
            .expect("邮件系统应成功");
        let mut msgs = world.resource_mut::<Messages<crate::game::dialogs::mail::ComposeMail>>();
        let drained: Vec<_> = msgs.drain().collect();
        assert_eq!(drained.len(), 1, "应写出一条 ComposeMail");
        assert_eq!(drained[0].to, "会长甲", "收件人应是会长（C# `GT.Owner1`）");
        assert_eq!(drained[0].message, None, "C# ComposeMail(Owner1) 不带正文");

        // 负控：选中无主领地（无会长）→ 不写信，只提示
        world.resource_mut::<GuildTerritoryState>().selected = Some(1);
        world
            .run_system_once(guild_territory_mail_system)
            .expect("邮件系统应成功");
        let mut msgs = world.resource_mut::<Messages<crate::game::dialogs::mail::ComposeMail>>();
        assert_eq!(msgs.drain().count(), 0, "无会长时不得写信");
        assert_eq!(
            world.resource::<GuildTerritoryState>().message,
            "请先选中一个已有会长的领地"
        );
    }
}
