// ============================================================================
// 邮件对话框（M22；#3103 写邮件按 C# 原版拆两窗）
// 布局参考：macroquad mail_dialog.rs / C# MailDialog
//   - 背景 Prguse[956]，标题 Title[20]，位置 (280,80)
//   - 邮件列表 y=60 起每 22px；点击列表项 → C.ReadMail{mail_id} → 内容区显示正文/金币
// 网络：ReceiveMail（新邮件条目 / 邮件全文，服务端同 opcode 双格式）→ 列表 + 详情
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputMultiline, TextInputRect,
};
use crate::game::dialogs::{AlwaysVisible, DialogKind, DialogManager, DialogRoot};
use crate::game::player_state::{Gold, Inventory};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::gray::UiGray;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_item_cell_ui, spawn_label, spawn_panel, spawn_scroll_bar_ui, ImageButton, UiItemCellData,
    UiScrollList,
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
    /// #3103 读侧：`MailInfo.Locked`——C# `MailListDialog.DeleteButton.Click` 的守卫
    /// （`MailDialogs.cs:236` `if (SelectedMail == null || SelectedMail.Locked) return;`）
    pub locked: bool,
    /// #3120 ①：列表行图标要看**第一个附件**（C# `MailItemRow.IconImage.Index = Mail.Items[0].Info.Image`，
    /// `MailDialogs.cs:519`）。此前本端 `MailEntry` 把包里带的附件丢了，所以行只能显示文本。
    pub items: Vec<MailAttachment>,
}

/// 邮件附件条目（C# `MailInfo.Items[i]`；读信窗的 5 个 `MirItemCell` 用它渲染）
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MailAttachment {
    pub name: String,
    /// `Items` 库图标索引（C# `MailItemRow.IconImage.Index = Mail.Items[0].Info.Image`）
    pub image: u16,
    pub count: u16,
    /// 耐久比例 0.0-1.0（`max_dura == 0` 时 `None` → 不画耐久条，同 `MirItemCell`）
    pub dura_ratio: Option<f32>,
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
    pub items: Vec<MailAttachment>,
    pub collected: bool,
    /// C# `MailInfo.SendDate`（Unix 秒）→ 读信窗 `DateSentLabel`
    /// （`MailDialogs.cs:1102` `Mail.DateSent.ToString("dd/MM/yy H:mm:ss")`）
    pub send_date: i64,
    /// C# `MailInfo.Locked`：锁定后读信窗「删除」拒发（`:1049`）
    pub locked: bool,
}

/// 邮件状态（网络 ReceiveMail 写入）
#[derive(Resource)]
pub struct MailState {
    pub mails: Vec<MailEntry>,
    pub detail: Option<MailDetail>,
    /// 写邮件界面是否打开（输入框用通用 TextInputState id 0=收件人 1=主题 2=正文 3=金币）
    pub compose: bool,
    /// #3103：写邮件窗种类——`false`=写信窗（C# `MailComposeLetterDialog`，回复/好友/玩家菜单…），
    /// `true`=待寄包裹窗（C# `MailComposeParcelDialog`，邮局 `S.MailSendRequest` 触发）
    pub compose_parcel: bool,
    /// 选中的邮件行（删除用，#132）
    pub selected: Option<usize>,
    /// 写邮件附加金币（C# `MailComposeParcelDialog.GiftGoldAmount`）
    pub compose_gold: u32,
    /// 写邮件附件（最多 5 个背包 unique_id，C# items_idx[5]）
    pub attach: Vec<Option<u64>>,
    /// #2538：贴票（C# MailComposeParcelDialog Stamped；true 时解锁 5 附件格）
    pub stamped: bool,
    /// #2538：邮资（C# ParcelCostLabel ← S.MailCost）
    pub parcel_cost: u32,
    /// #3103：赠金数量框是否由邮件窗发起（区分背包拆分/丢弃的数量框结果）
    pub gold_ask_pending: bool,
    /// #3120 ③：列表当前页（1 起；C# `MailDialogs.cs:26` `CurrentPage`）。
    /// 行区间 = `mail_page_start(page) .. +10`，与 C# `StartIndex` 等价。
    pub page: usize,
}

impl Default for MailState {
    fn default() -> Self {
        Self {
            mails: Vec::new(),
            detail: None,
            compose: false,
            compose_parcel: false,
            selected: None,
            compose_gold: 0,
            attach: vec![None; 5],
            stamped: false,
            parcel_cost: 0,
            gold_ask_pending: false,
            page: 1,
        }
    }
}

/// #2538：可用附件格数（C# SendMail hasStamp?5:1；客户端按 Stamped 估计）
pub fn stamp_slots(stamped: bool) -> usize {
    if stamped {
        5
    } else {
        1
    }
}

// C# MailListDialog（MailDialogs.cs:32-35）布局锚点。
const MAIL_W: f32 = 312.0;
const MAIL_H: f32 = 444.0;
/// #2892 批B：列表面板精灵（C# `MailListDialog.Index = 670; Library = Libraries.Title`）
pub const PANEL: (LibraryName, usize) = (LibraryName::Title, 670);
pub const PANEL_SIZE: (f32, f32) = (MAIL_W, MAIL_H);
/// 关闭键 `Prguse2[360..362]` @ (`Size.Width`-24, 3)（`MailDialogs.cs:78-79`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (MAIL_W - 24.0, 3.0);
/// #3103：两张**写邮件**窗自己的拖动/置顶分组。
///
/// `dialog_drag_system`（按 `DialogRoot` 的 kind 聚合包围盒与位移）与 `bump_dialog_z`
/// （同 kind 一起平移）都是**按 kind** 工作的——写邮件窗若继续挂在 `DialogKind::Mail` 上，
/// 拖邮件列表就会把它一起拖走（owner 2026-09-24 截图现象）。C# 里
/// `MailComposeLetterDialog` / `MailComposeParcelDialog` 与 `MailListDialog` 是互不相关的独立窗
/// （各自 `Movable = true`），故这里给独立 kind。
pub const COMPOSE_DRAG_KIND: DialogKind = DialogKind::MailCompose;
/// #3103 读侧：两张**读邮件**窗自己的拖动/置顶分组（理由同 [`COMPOSE_DRAG_KIND`]——
/// C# 里 `MailReadLetterDialog` / `MailReadParcelDialog` 与邮件列表窗各自 `Movable`）。
pub const READ_DRAG_KIND: DialogKind = DialogKind::MailRead;
const MAIL_SCREEN_W: f32 = 1024.0;
const MAIL_VISIBLE_ROWS: usize = 10;
const MAIL_ROW_H: f32 = 33.0;
const MAIL_BUTTON_Y: f32 = 414.0;

// ---- 列表分页（#3120 ③）----------------------------------------------------
// C# `MailDialogs.cs:25-26`：`StartIndex`（首行下标）与 `CurrentPage/PageCount` 成对维护；
// `:99-151` 建上一页/下一页按钮与页号标签，`:303-306` 每次 `UpdateInterface` 重算
// `PageCount = ceil(Mail.Count/10)`（至少 1）并写 `PageLabel.Text = "{CurrentPage} / {PageCount}"`。
const MAIL_PAGE_ROWS: usize = MAIL_VISIBLE_ROWS;
/// 上一页 `Prguse2[240..242]` @ (102, H-55)（H=444 → y=389）
pub const PAGE_PREV_POS: (f32, f32) = (102.0, MAIL_H - 55.0);
/// 页号 `PageLabel` @ (120, H-55)，67x15，垂直+水平居中（`MailDialogs.cs:122-129`）
pub const PAGE_LABEL_POS: (f32, f32) = (120.0, MAIL_H - 55.0);
/// 下一页 `Prguse2[243..245]` @ (192, H-55)（`MailDialogs.cs:131-140`）
pub const PAGE_NEXT_POS: (f32, f32) = (192.0, MAIL_H - 55.0);
pub const PAGE_LABEL_SIZE: (f32, f32) = (67.0, 15.0);
pub const PAGE_PREV_FRAMES: (usize, usize, usize) = (240, 241, 242);
pub const PAGE_NEXT_FRAMES: (usize, usize, usize) = (243, 244, 245);

/// C# `MailDialogs.cs:303-304`：`PageCount = ceil(Count/10)`，且 `PageCount < 1 → 1`
pub fn mail_page_count(total: usize) -> usize {
    total.div_ceil(MAIL_PAGE_ROWS).max(1)
}

/// C# `MailDialogs.cs:25/117/148`：`StartIndex = (CurrentPage - 1) * 10`
pub fn mail_page_start(page: usize) -> usize {
    page.saturating_sub(1) * MAIL_PAGE_ROWS
}

/// C# `MailDialogs.cs:306`：`PageLabel.Text = string.Format("{0} / {1}", CurrentPage, PageCount)`
pub fn mail_page_label(page: usize, total: usize) -> String {
    format!("{} / {}", page.max(1), mail_page_count(total))
}

/// 点「上一页」的新页码：`None` = 按 C# `MailDialogs.cs:111` `if (CurrentPage <= 1) return;` 原地不动
pub fn mail_page_after_prev(page: usize) -> Option<usize> {
    if page <= 1 {
        None
    } else {
        Some(page - 1)
    }
}

/// 点「下一页」的新页码：`None` = 按 C# `MailDialogs.cs:143` `if (CurrentPage >= PageCount) return;` 原地不动
pub fn mail_page_after_next(page: usize, total: usize) -> Option<usize> {
    if page >= mail_page_count(total) {
        None
    } else {
        Some(page + 1)
    }
}

// ============================================================================
// #3103 写邮件两窗（C# `Client/MirScenes/Dialogs/MailDialogs.cs` 逐项抄录）
//   `MailComposeLetterDialog`（`:596-684`）：Title[671] 236x300 @ (100,100)
//   `MailComposeParcelDialog`（`:687-...`）：Title[674] 236x384 @ (背包宽+10, 0)
// 两张底图自带「收件人/正文/邮票/邮资/赠送」等印刷标签，故本端**不自绘文字标签**，
// 只放数值标签与控件（原先自绘的「主题:」「附件:」「背包选择:」等既非原版也无底图）。
// ============================================================================
pub const LETTER_PANEL: (LibraryName, usize) = (LibraryName::Title, 671);
pub const LETTER_SIZE: (f32, f32) = (236.0, 300.0);
pub const LETTER_POS: (f32, f32) = (100.0, 100.0);
/// 两窗关闭钮同为 `Prguse2[360..362]` @ `(Size.Width - 27, 3)`（`MailDialogs.cs:615/719`；
/// 注意与邮件列表窗的 `-24` 不同）
pub const COMPOSE_CLOSE_DX: f32 = 27.0;
pub const COMPOSE_CLOSE_Y: f32 = 3.0;
/// 收件人标签（C# `RecipientNameLabel`，`NotControl = true` 即不可编辑）
pub const COMPOSE_RECIPIENT_POS: (f32, f32) = (70.0, 35.0);
pub const COMPOSE_RECIPIENT_SIZE: (f32, f32) = (150.0, 15.0);
pub const LETTER_BODY_POS: (f32, f32) = (15.0, 92.0);
pub const PARCEL_BODY_POS: (f32, f32) = (15.0, 98.0);
pub const COMPOSE_BODY_SIZE: (f32, f32) = (202.0, 165.0);
pub const LETTER_SEND_POS: (f32, f32) = (30.0, 265.0);
pub const LETTER_CANCEL_POS: (f32, f32) = (135.0, 265.0);
pub const PARCEL_PANEL: (LibraryName, usize) = (LibraryName::Title, 674);
pub const PARCEL_SIZE: (f32, f32) = (236.0, 384.0);
/// C# `Location = new Point(GameScene.Scene.InventoryDialog.Size.Width + 10, 0)`
pub const PARCEL_X_GAP: f32 = 10.0;
pub const PARCEL_STAMP_POS: (f32, f32) = (73.0, 56.0);
pub const PARCEL_STAMP_SIZE: (f32, f32) = (20.0, 20.0);
pub const PARCEL_COST_POS: (f32, f32) = (63.0, 269.0);
pub const PARCEL_GOLD_POS: (f32, f32) = (63.0, 290.0);
pub const PARCEL_VALUE_SIZE: (f32, f32) = (143.0, 15.0);
/// C# `ItemCover`（`Title[676]`）@ (63,310) 144x33：未贴票时盖住 5 个附件格
pub const PARCEL_COVER_POS: (f32, f32) = (63.0, 310.0);
pub const PARCEL_COVER_SIZE: (f32, f32) = (144.0, 33.0);
pub const PARCEL_COVER_INDEX: usize = 676;
/// 5 个 `MirItemCell` 35x31 @ (27 + 36i, 311)
pub const PARCEL_CELL_X0: f32 = 27.0;
pub const PARCEL_CELL_STEP: f32 = 36.0;
pub const PARCEL_CELL_Y: f32 = 311.0;
pub const PARCEL_CELL_SIZE: (f32, f32) = (35.0, 31.0);
pub const PARCEL_SEND_POS: (f32, f32) = (30.0, 350.0);
pub const PARCEL_CANCEL_POS: (f32, f32) = (135.0, 350.0);
/// 发送/取消键 `Title[607/608/609]`、`Title[193/194/195]`（两窗同帧）
pub const COMPOSE_SEND_FRAMES: (usize, usize, usize) = (607, 608, 609);
pub const COMPOSE_CANCEL_FRAMES: (usize, usize, usize) = (193, 194, 195);
/// 邮票键 `Prguse2[203]`，贴票后 `[204]`（`MailComposeParcelDialog.UpdateParcel`）
pub const STAMP_FRAMES: (usize, usize) = (203, 204);

/// C# `MailComposeParcelDialog.Location`（背包真实宽 + 10，0）
pub fn parcel_origin(inventory_w: f32) -> (f32, f32) {
    (inventory_w + PARCEL_X_GAP, 0.0)
}

// ============================================================================
// #3103 读侧：读邮件两窗（C# `Client/MirScenes/Dialogs/MailDialogs.cs` 逐项抄录）
//   `MailReadLetterDialog`（`:979-1107`）：Title[672] 236x300 @ (100,100)
//   `MailReadParcelDialog`（`:1108-1272`）：Title[675] 236x300 @ (100,100)
//
// **原版自身的不一致（如实处理）**：`MailReadParcelDialog.Size = (236,300)`，但其控件 y 一直排到
// 350、附件格 y=311——按美术实际高度落地（`Title[675]` 与 `Title[674]` 同族，内建边框到 384），
// 与写侧 `MailComposeParcelDialog`（C# 显式 `Size = 236x384`）口径一致。若压成 300 高，
// 面板 `Overflow::clip()` 会把收取/取消两键裁掉（既看不见也点不到，同 #3107 的坐骑窗）。
// ============================================================================
pub const READ_LETTER_PANEL: (LibraryName, usize) = (LibraryName::Title, 672);
pub const READ_PARCEL_PANEL: (LibraryName, usize) = (LibraryName::Title, 675);
pub const READ_SIZE: (f32, f32) = (236.0, 300.0);
pub const READ_PARCEL_SIZE: (f32, f32) = (236.0, 384.0);
pub const READ_POS: (f32, f32) = (100.0, 100.0);
/// 两窗共用：发件人 @ (70,35)、发信时间 @ (70,56)，均 150x15 白字
pub const READ_SENDER_POS: (f32, f32) = (70.0, 35.0);
pub const READ_DATE_POS: (f32, f32) = (70.0, 56.0);
pub const READ_LABEL_SIZE: (f32, f32) = (150.0, 15.0);
/// 正文：书信 `MessageLabel @ (15,92) 202x165`；包裹 `@ (15,98) 202x165`
pub const READ_LETTER_BODY_POS: (f32, f32) = (15.0, 92.0);
pub const READ_PARCEL_BODY_POS: (f32, f32) = (15.0, 98.0);
pub const READ_BODY_SIZE: (f32, f32) = (202.0, 165.0);
/// 书信三键（C# `MirButton` 均不设 `Size` → 取精灵原生尺寸）
pub const READ_DELETE_POS: (f32, f32) = (12.0, 265.0);
pub const READ_LOCK_POS: (f32, f32) = (81.0, 265.0);
pub const READ_LETTER_CANCEL_POS: (f32, f32) = (154.0, 265.0);
pub const READ_DELETE_FRAMES: (usize, usize, usize) = (540, 541, 542);
pub const READ_LOCK_FRAMES: (usize, usize, usize) = (686, 687, 688);
/// 包裹窗赠金标签 `GoldSendLabel @ (63,290) 143x15`
pub const READ_GOLD_POS: (f32, f32) = (63.0, 290.0);
pub const READ_VALUE_SIZE: (f32, f32) = (143.0, 15.0);
/// 包裹窗 5 个附件格 35x31 @ (27+36i, 311)（与 `MailComposeParcelDialog.Cells` 同格位）
pub const READ_CELL_X0: f32 = 27.0;
pub const READ_CELL_STEP: f32 = 36.0;
pub const READ_CELL_Y: f32 = 311.0;
pub const READ_CELL_SIZE: (f32, f32) = (35.0, 31.0);
pub const READ_COLLECT_POS: (f32, f32) = (30.0, 350.0);
pub const READ_PARCEL_CANCEL_POS: (f32, f32) = (135.0, 350.0);
/// 收取钮：**已领取**`Title[680/681/682]`、**未领取（禁用）**`Title[683/684/685]`
/// （`MailDialogs.cs:1243-1256` `if (!Mail.Collected) {Index=683..685; Enabled=false;}`）
pub const READ_COLLECT_OK_FRAMES: (usize, usize, usize) = (680, 681, 682);
pub const READ_COLLECT_DISABLED_FRAMES: (usize, usize, usize) = (683, 684, 685);

/// C# `MailReadLetterDialog.ReadMail` / `MailReadParcelDialog.ReadMail` 的**选窗规则**
/// （`MailDialogs.cs:213-220` 与 `:334-341` 两处同式）：
/// `if (SelectedMail.Gold > 0 || SelectedMail.Items.Count > 0) → 包裹窗 else 书信窗`。
pub fn read_window_for(detail: &MailDetail) -> ReadWin {
    if detail.gold > 0 || !detail.items.is_empty() {
        ReadWin::Parcel
    } else {
        ReadWin::Letter
    }
}

/// 收取钮三帧（C# `UpdateParcel`/`ReadMail` 的 Index/Hover/Pressed 整套替换）
pub fn collect_frames(collected: bool) -> (usize, usize, usize) {
    if collected {
        READ_COLLECT_OK_FRAMES
    } else {
        READ_COLLECT_DISABLED_FRAMES
    }
}

/// C# `Mail.DateSent.ToString("dd/MM/yy H:mm:ss")`（本地墙钟；`H` 不补零）。
/// `send_date <= 0`（时间戳缺失）→ 空串，不显示假日期。
pub fn format_send_date(send_date: i64) -> String {
    if send_date <= 0 {
        return String::new();
    }
    chrono::DateTime::from_timestamp(send_date, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%d/%m/%y %-H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
}

/// C# `Gold.ToString("###,###,##0")`（读包裹窗 `MailDialogs.cs:1221`、待寄窗 `UpdateParcel`）：
/// 千分位分隔，**0 照样输出 `"0"`**（末位占位符是 `0` 不是 `#`）。
/// 实测口径（.NET `string.Format("{0:###,###,##0}", v)`）：`0→"0"`、`7→"7"`、`1234→"1,234"`、
/// `1234567→"1,234,567"`——本函数逐项对齐，读/写两侧共用（避免两处各写一份漂移）。
pub fn format_gold(gold: u32) -> String {
    let digits = gold.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 写邮件输入框槽位（`TextInputState.texts`；两窗正文各自独立，与 C# 两张窗各持
/// 一个 `MirTextBox` 一致）。
///
/// **槽位必须是本模块私有的**：`TextInputState.texts` 是全客户端共用的一维数组，
/// 槽位即身份。旧覆盖层用 0..3 时，正文槽 2 与行会公告槽 2 撞车——`guild.rs` 的公告
/// 同步系统在 `input.active != Some(2)` 时**每帧**用服务端公告回写该槽（无公告即空串），
/// 于是写邮件正文在未聚焦时被反复清空（实机取证：`ui_nodes_at` 里正文文本节点高度 0）。
/// 5/6 当前无其它模块使用（已用：0-4、7、10、12、13、31-33、40）。
pub const INPUT_RECIPIENT: usize = 0;
pub const INPUT_LETTER_BODY: usize = 5;
pub const INPUT_PARCEL_BODY: usize = 6;

fn mail_panel_origin(screen_w: f32) -> (f32, f32) {
    (screen_w - MAIL_W - 150.0, 5.0)
}

fn mail_row_y(index: usize) -> f32 {
    55.0 + index as f32 * MAIL_ROW_H
}

// ---------------------------------------------------------------------------
// C# `MailItemRow`（`Client/MirScenes/Dialogs/MailDialogs.cs:421-569`）的行几何与判定。
// 抽成纯函数/常量：探针 `mail_probe` 与实机夹具按同一份口径对账，避免"渲染改了一处、
// 判据还按旧口径"的漂移。
// ---------------------------------------------------------------------------

/// `IconArea = new Size(34, 32)`（:430）——图标在其中的**居中**偏移见 `mail_row_icon_offset`
pub const MAIL_ROW_ICON_AREA: (f32, f32) = (34.0, 32.0);
/// 角标默认位置 `(5,17)`（:451/:460/:469）
pub const MAIL_ROW_BADGE_POS: (f32, f32) = (5.0, 17.0);
/// 未读角标被"挤到第二位"时的位置 `(20,17)`（:553/:561）
pub const MAIL_ROW_BADGE_POS_SECOND: (f32, f32) = (20.0, 17.0);
/// 选中底图 `(-5,-3)`（:479）
pub const MAIL_ROW_SELECTED_OFFSET: (f32, f32) = (-5.0, -3.0);
/// `SenderLabel @ (35,0) 130x31 VerticalCenter`（:487-489）
pub const MAIL_ROW_SENDER_RECT: (f32, f32, f32, f32) = (35.0, 0.0, 130.0, 31.0);
/// `MessageLabel @ (170,0) 115x31 VerticalCenter`（:496-498）
pub const MAIL_ROW_INFO_RECT: (f32, f32, f32, f32) = (170.0, 0.0, 115.0, 31.0);
/// 图标库/索引：`Items[Info.Image]` / `Prguse[541]`（有金币）/ `Prguse[540]`（空）（:517-531）
pub const MAIL_ROW_ICON_EMPTY: u16 = 540;
pub const MAIL_ROW_ICON_GOLD: u16 = 541;
/// 角标索引：未读 550 / 锁定 551 / 包裹 552（:449/:458/:467）
pub const MAIL_ROW_BADGE_UNREAD: u16 = 550;
pub const MAIL_ROW_BADGE_LOCKED: u16 = 551;
pub const MAIL_ROW_BADGE_PARCEL: u16 = 552;
/// 选中底图 545（:477）
pub const MAIL_ROW_SELECTED_BG: u16 = 545;

/// 行图标来源（C# `UpdateInterface` 的三分支，:517-531）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailRowIcon {
    /// 有附件 → `Libraries.Items[Items[0].Info.Image]`
    Item(u16),
    /// 无附件但有金币 → `Prguse[541]`
    Gold,
    /// 都没有 → `Prguse[540]`
    Empty,
}

pub fn mail_row_icon(m: &MailEntry) -> MailRowIcon {
    // C# 只看**第一个**附件（`Mail.Items[0].Info.Image`）
    match m.items.first() {
        Some(a) => MailRowIcon::Item(a.image),
        None if m.gold > 0 => MailRowIcon::Gold,
        None => MailRowIcon::Empty,
    }
}

/// 图标在 `IconArea` 里的居中偏移（C# :533 `((34-w)/2, (32-h)/2)`）
pub fn mail_row_icon_offset(w: f32, h: f32) -> (f32, f32) {
    (
        (MAIL_ROW_ICON_AREA.0 - w) / 2.0,
        (MAIL_ROW_ICON_AREA.1 - h) / 2.0,
    )
}

/// 未读角标的 x：**未取回包裹**或**已锁定**时挤到第二位 (20,17)（C# :546-562）
pub fn mail_row_unread_x(m: &MailEntry) -> f32 {
    if !m.collected || m.locked {
        MAIL_ROW_BADGE_POS_SECOND.0
    } else {
        MAIL_ROW_BADGE_POS.0
    }
}

/// 信息列文本：锁定邮件加 `[*] ` 前缀；换行压成空格（C# :566）
pub fn mail_row_info_text(m: &MailEntry) -> String {
    let flat = m.subject.replace("\r\n", " ").replace('\n', " ");
    if m.locked {
        format!("[*] {flat}")
    } else {
        flat
    }
}

/// #3120 ② 残余：物品能否邮寄（C# `MirItemCell.cs:1793` 守卫 `Info.Bind.HasFlag(BindMode.DontTrade)`；
/// 服务端同口径见 `ServerRust/src/actors/world/mail.rs:572` —— 那里拦的是 `DontTrade(0x10) | NoMail(0x4000)`）。
///
/// 只做判定、不碰状态：客户端**拿到**绑定位时用它做早提示；**拿不到**时调用方不得拦截，
/// 交回服务端拒绝（保证与当前行为一致，不制造假拒收）。
pub fn can_mail_item(bind_flags: u16) -> bool {
    let dont_trade = mir2_shared::enums::BindMode::DONT_TRADE.bits();
    let no_mail = mir2_shared::enums::BindMode::NO_MAIL.bits();
    (bind_flags & (dont_trade | no_mail)) == 0
}

/// 请求写邮件（#2631 跨对话框解耦 Message）。
/// friend 等外部对话框不再直写 [`MailState`]，改发本 Message；邮件对话框的
/// [`mail_compose_request_system`] 消费并自行预填收件人 + 打开写邮件界面。
/// `to` = 收件人名（预填到写邮件输入框 id 0，C# FriendDialog EmailButton 语义）。
#[derive(Message, Debug)]
pub struct ComposeMail {
    pub to: String,
    /// C# `MailComposeLetterDialog.ComposeMail(recipient, message)` 的正文预填（输入框 id 2）；
    /// `None` = 只预填收件人（C# FriendDialog EmailButton 语义）
    pub message: Option<String>,
    /// #3103：`true` = 打开**待寄包裹窗**（C# `MailComposeParcelDialog`，邮局 `S.MailSendRequest`
    /// 路径）；`false` = **写信窗**（C# `MailComposeLetterDialog`，回复/好友/玩家菜单等路径）。
    /// C# 的 `ComposeMail(string)` 重载只出现在包裹窗，故用同一 Message 带种类即可。
    pub parcel: bool,
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

/// 阅读所选邮件（C# MailListDialog.ReadButton → C.ReadMail）
#[derive(Component)]
pub struct MailReadBtn;

/// #2786：回复按钮（C# `MailDialogs.cs:180-196` `ReplyButton` `Prguse[569..571]` @(102,414)）
#[derive(Component)]
pub struct MailReplyBtn;

/// 列表操作按钮种类（写/回复/读/删）
#[derive(Clone, Copy)]
enum MailAction {
    Write,
    Reply,
    Read,
    Delete,
}

/// 写邮件附件槽（C# `MailComposeParcelDialog.Cells[5]` @ (27+36i, 311) 35x31）
#[derive(Component)]
pub struct MailAttachSlot(pub usize);

#[derive(Component)]
pub struct MailLine(usize);

/// #3120 ①：行内视觉层的角色（C# `MailItemRow` 的五个 `MirImageControl` + 两个 `MirLabel`）
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MailRowRole {
    /// `SelectedImage` `Prguse[545] @(-5,-3)`（:477-483）
    Selected,
    /// `IconImage`（`IconArea 34x32` 居中；库/帧由模型三分支决定，:517-533）
    Icon,
    /// `UnreadImage` `Prguse[550]`（:449）
    Unread,
    /// `LockedImage` `Prguse[551]`（:458）
    Locked,
    /// `ParcelImage` `Prguse[552]`（:467）
    Parcel,
    /// `SenderLabel @(35,0)`（:487）
    Sender,
    /// `MessageLabel @(170,0)`（:496）
    Info,
}

/// #3120 ①：行内一个视觉层实体（探针据此把"渲染真值"与模型派生的 C# 规则对账）
#[derive(Component)]
pub(crate) struct MailRowSlot {
    pub(crate) row: usize,
    pub(crate) role: MailRowRole,
    /// 图标层**实际应用的**库与帧（仅 `Icon` 有含义）——夹具拿它与 `mail_probe.mails[i].icon` 比
    pub(crate) icon_lib: &'static str,
    pub(crate) icon_index: u16,
}

/// 行在列表里的左原点（与行点击命中矩形同一口径：`x ∈ [ROW_X, ROW_X + 290]`）
pub(crate) const MAIL_ROW_X: f32 = 10.0;

/// 铺一个行内图片层（`Node`/`ImageNode` 空间，与邮件窗其它元素一致），默认 `Hidden`。
#[allow(clippy::too_many_arguments)]
fn spawn_row_image(
    p: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    lib: LibraryName,
    index: u16,
    x: f32,
    y: f32,
    z: i32,
    row: usize,
    role: MailRowRole,
) {
    let Some(handle) = load_lib_image(libs, images, lib, index as usize) else {
        return;
    };
    let (w, h) = libs
        .0
        .get_image(lib, index as usize)
        .map(|i| (i.width as f32, i.height as f32))
        .unwrap_or((1.0, 1.0));
    // 图标层先按 IconArea 居中摆（帧尺寸由 C# `((34-w)/2,(32-h)/2)` 决定）
    let (x, y) = if role == MailRowRole::Icon {
        let (ox, oy) = mail_row_icon_offset(w, h);
        (x + ox, y + oy)
    } else {
        (x, y)
    };
    crate::ui::theme::spawn_image(p, handle, x, y, w, h, z).insert((
        MailRowSlot {
            row,
            role,
            icon_lib: if lib == LibraryName::Items {
                "Items"
            } else {
                "Prguse"
            },
            icon_index: index,
        },
        Visibility::Hidden,
    ));
}

/// #3120 ③：上一页键（C# `MailDialogs.cs:99` `PreviousButton`，`Prguse2[240..242]`）
#[derive(Component)]
pub struct MailPagePrev;

/// #3120 ③：下一页键（C# `MailDialogs.cs:131` `NextButton`，`Prguse2[243..245]`）
#[derive(Component)]
pub struct MailPageNext;

/// #3120 ③：页号标签（C# `MailDialogs.cs:122` `PageLabel`，"当前页 / 总页数"）
#[derive(Component)]
pub struct MailPageLabel;

// 写邮件界面
#[derive(Component)]
pub struct MailWrite;

/// #3103：写邮件窗种类（C# `MailComposeLetterDialog` / `MailComposeParcelDialog` 两张窗）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComposeWin {
    /// `Title[671]` 236x300 @ (100,100)
    Letter,
    /// `Title[674]` 236x384 @ (背包宽+10, 0)
    Parcel,
}

/// 写邮件窗根面板。挂 [`AlwaysVisible`]：包裹窗在**邮件列表窗未打开**时也要能显示
/// （C# 邮局路径只 `InventoryDialog.Show()`），故显隐由本模块自己驱动，不走
/// `DialogsPlugin` 的「未 open 的 kind 一律隐藏」兜底。
#[derive(Component)]
pub struct MailComposeRoot(pub ComposeWin);

/// 写邮件窗按钮种类（C# `SendButton` / `CancelButton` / `CloseButton`）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComposeBtn {
    Send,
    Cancel,
    Close,
}

/// 写邮件窗按钮（窗种类 + 按钮种类）
#[derive(Component)]
pub struct MailComposeBtn(pub ComposeWin, pub ComposeBtn);

/// 收件人标签（C# `RecipientNameLabel` @ (70,35) 150x15；由 `ComposeMail` 预填，不可编辑）
#[derive(Component)]
pub struct MailRecipientLabel;

/// 赠金数值标签（C# `GoldSendLabel` @ (63,290)；点击 → `MirAmountBox`）
#[derive(Component)]
pub struct MailGoldLabel;

/// 附件格遮罩（C# `ItemCover` `Title[676]`：未贴票时可见，贴票后 `UpdateParcel` 隐藏）
#[derive(Component)]
pub struct MailItemCover;

/// #2538：邮票按钮（C# StampButton Prguse2[203]，点击切换贴票）
#[derive(Component)]
pub struct MailStampBtn;

/// #2538：贴票状态覆层（C# UpdateParcel StampButton.Index=204）
#[derive(Component)]
pub struct MailStampOn;

/// #2538：邮资标签（C# ParcelCostLabel ← S.MailCost）
#[derive(Component)]
pub struct MailCostLabel;

// ---- #3103 读侧：读邮件两窗 ----

/// 读邮件窗种类（C# `MailReadLetterDialog` / `MailReadParcelDialog` 两张窗）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReadWin {
    /// `Title[672]` 236x300 @ (100,100)
    Letter,
    /// `Title[675]` @ (100,100)，美术 236x384
    Parcel,
}

/// 读邮件窗根面板（`AlwaysVisible` + 按 `MailState.detail` 驱动显隐，同 [`MailComposeRoot`]）
#[derive(Component)]
pub struct MailReadRoot(pub ReadWin);

/// 读邮件窗按钮种类（C# `DeleteButton` / `LockButton` / `CancelButton` / `CloseButton` / `CollectButton`）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReadBtn {
    Delete,
    Lock,
    Cancel,
    Close,
    Collect,
}

/// 读邮件窗按钮（窗种类 + 按钮种类）
#[derive(Component)]
pub struct MailReadButton(pub ReadWin, pub ReadBtn);

/// 读邮件窗的文本标签种类（C# 两窗 `SenderNameLabel`/`DateSentLabel`/`MessageLabel`
/// 与包裹窗 `GoldSendLabel`；用一个查询统一刷新，避免同系统内多条 `&mut Text` 查询冲突）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReadLabel {
    /// `@ (70,35) 150x15`
    Sender,
    /// `@ (70,56) 150x15`
    Date,
    /// 书信 `@ (15,92)`、包裹 `@ (15,98)`，均 202x165
    Body(ReadWin),
    /// 包裹窗 `@ (63,290) 143x15`
    Gold,
}

/// 读邮件窗文本标签
#[derive(Component)]
pub struct MailReadLabel(pub ReadLabel);

/// 包裹窗附件格（C# `MailReadParcelDialog.Cells[5]`，只读展示）
#[derive(Component)]
pub struct MailReadCell(pub usize);

/// 收取钮的两套三帧（C# `Index/HoverIndex/PressedIndex` 按 `Mail.Collected` 整组替换，
/// `MailDialogs.cs:1243-1256`）——两套都在 spawn 时备好，状态翻转只换句柄，不重载资产。
#[derive(Component)]
pub struct MailCollectFrames {
    pub ok: [Handle<Image>; 3],
    pub disabled: [Handle<Image>; 3],
}

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiCjkFont>();
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
                // #2786：回复钮（独立系统，避免 mail_ui_system 参数超上限）
                mail_reply_system,
                // #3103：写邮件两窗（显隐/附件/按钮/贴票/赠金）
                mail_compose_ui_system,
                mail_compose_click_system,
                mail_compose_btn_system,
                mail_compose_follow_system,
                mail_stamp_system,
                mail_gold_system,
                // #3103 读侧：读邮件两窗（显隐/字段/按钮），排在写侧之后以免同帧抢状态
                mail_read_ui_system,
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

/// 消费 [`ComposeMail`]：按种类打开对应写邮件窗并预填收件人（#2631 跨对话框解耦）。
///
/// - `parcel = false`（写信窗，C# `MailComposeLetterDialog.ComposeMail`）：打开邮件列表窗
///   + 预填收件人（槽位 [`INPUT_RECIPIENT`]）与可选正文（槽位 [`INPUT_LETTER_BODY`]）；
/// - `parcel = true`（待寄窗，C# `MailComposeParcelDialog.ComposeMail`）：只 `InventoryDialog.Show()`
///   （**不开邮件列表窗**）、清正文、`UpdateParcel()` 复位贴票/附件、邮资与赠金归零，并
///   按 `CalculatePostage()` 查一次邮资。
///
/// 收货人名为空直接忽略（C#：`if (string.IsNullOrEmpty(recipientName)) return;`）。
fn mail_compose_request_system(
    mut events: MessageReader<ComposeMail>,
    mut mail: ResMut<MailState>,
    mut mgr: ResMut<DialogManager>,
    net: Res<NetConnection>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    for ev in events.read() {
        if ev.to.is_empty() {
            tracing::warn!("✉️ 收件人为空，忽略写邮件请求");
            continue;
        }
        if input.texts.len() <= INPUT_PARCEL_BODY {
            input.texts.resize(INPUT_PARCEL_BODY + 1, String::new());
        }
        input.texts[INPUT_RECIPIENT] = ev.to.clone();
        if ev.parcel {
            mgr.open(DialogKind::Inventory);
            mail.compose = true;
            mail.compose_parcel = true;
            mail.attach = vec![None; 5];
            mail.compose_gold = 0;
            mail.stamped = false;
            mail.parcel_cost = 0;
            // C# `MessageTextBox.Text = string.Empty`
            input.texts[INPUT_PARCEL_BODY].clear();
            input.active = None;
            request_mail_cost(&net, 0, &mail.attach, false);
            tracing::info!("📦 给 {} 寄包裹", ev.to);
        } else {
            mgr.open(DialogKind::Mail);
            mail.compose = true;
            mail.compose_parcel = false;
            mail.detail = None;
            mail.attach = vec![None; 5];
            mail.compose_gold = 0;
            if let Some(body) = ev.message.as_ref() {
                input.texts[INPUT_LETTER_BODY] = body.clone();
            }
            input.active = None;
            tracing::info!("✉️ 给 {} 写邮件", ev.to);
        }
    }
}

/// #3103：关闭写邮件窗并把草稿态复位（C# `Hide()` + 包裹窗 `Reset()` 退赠金/贴票）
pub fn close_compose(
    mail: &mut MailState,
    input: &mut crate::game::dialogs::text_input::TextInputState,
) {
    mail.compose = false;
    mail.attach = vec![None; 5];
    mail.compose_gold = 0;
    mail.stamped = false;
    mail.parcel_cost = 0;
    mail.gold_ask_pending = false;
    input.active = None;
}

/// #3103：两窗显隐 + 收件人/赠金/附件图标同步。
///
/// - 显隐：`MailState.compose` + `MailState.compose_parcel` 决定哪张窗可见；
/// - 收件人：C# `RecipientNameLabel`（`NotControl`，不可编辑），值取输入框槽位
///   [`INPUT_RECIPIENT`]（由 `ComposeMail` 预填）；
/// - 赠金：C# `GoldSendLabel.Text = GiftGoldAmount.ToString("###,###,##0")`，是**裸数字**
///   （底图 `Title[674]` 自带「赠送」印刷标签）；
/// - 附件格图标：C# `Cells[i].Item`，按 `unique_id` 在背包里查 `Items` 图。
fn mail_compose_ui_system(
    mail: Res<MailState>,
    input: Res<crate::game::dialogs::text_input::TextInputState>,
    inv_q: Query<&Inventory, With<crate::actor::LocalPlayer>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut roots: Query<(&MailComposeRoot, &Node, &mut Visibility)>,
    mut recipients: Query<&mut Text, (With<MailRecipientLabel>, Without<MailGoldLabel>)>,
    mut golds: Query<&mut Text, (With<MailGoldLabel>, Without<MailRecipientLabel>)>,
    mut attach_cells: Query<(&mut UiItemCellData, &MailAttachSlot)>,
    mut body_rects: Query<(&mut TextInputRect, &TextInputField)>,
) {
    let mut origins = [(0.0f32, 0.0f32); 2];
    for (root, node, mut vis) in &mut roots {
        let want = mail.compose && ((root.0 == ComposeWin::Parcel) == mail.compose_parcel);
        *vis = if want {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let idx = if root.0 == ComposeWin::Parcel { 1 } else { 0 };
        let default = if root.0 == ComposeWin::Parcel {
            parcel_origin(PARCEL_SIZE.0)
        } else {
            LETTER_POS
        };
        origins[idx] = crate::ui::theme::node_origin(node, default);
    }
    // 输入框命中区/IME 候选框按窗原点跟随（窗可拖动）；窗不可见时挪到屏外，
    // 否则隐藏窗的 `TextInputRect` 仍会在 `text_input_system` 里抢走点击焦点。
    for (mut rect, field) in &mut body_rects {
        let (origin, rel, visible) = match field.0 {
            INPUT_PARCEL_BODY => (
                origins[1],
                PARCEL_BODY_POS,
                mail.compose && mail.compose_parcel,
            ),
            INPUT_LETTER_BODY => (
                origins[0],
                LETTER_BODY_POS,
                mail.compose && !mail.compose_parcel,
            ),
            _ => continue,
        };
        let want = if visible {
            (
                origin.0 + rel.0,
                origin.1 + rel.1,
                COMPOSE_BODY_SIZE.0,
                COMPOSE_BODY_SIZE.1,
            )
        } else {
            (-10_000.0, -10_000.0, 0.0, 0.0)
        };
        if (rect.0, rect.1, rect.2, rect.3) != want {
            *rect = TextInputRect(want.0, want.1, want.2, want.3);
        }
    }
    let recipient = input
        .texts
        .get(INPUT_RECIPIENT)
        .cloned()
        .unwrap_or_default();
    for mut t in &mut recipients {
        if t.0 != recipient {
            t.0 = recipient.clone();
        }
    }
    // C# 待寄窗 `UpdateParcel`：`GoldSendLabel.Text = GiftGoldAmount.ToString("###,###,##0")`
    // ——与读包裹窗同一条格式串（`format_gold` 单一来源）
    let gold_text = format_gold(mail.compose_gold);
    for mut t in &mut golds {
        if t.0 != gold_text {
            t.0 = gold_text.clone();
        }
    }
    let items = inv_q
        .single()
        .map(|inv| inv.items.as_slice())
        .unwrap_or(&[]);
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
}

/// #3103：待寄窗 5 个附件格点击。
///
/// C# 靠 `MirItemCell`（`GridType = Mail`）**拖放**装入；本端沿用仓库既有的
/// 「先点背包物品选中 → 再点目标格」两击语义（同仓库存入/制作放入）：
/// 空格 + 背包有选中 → 装入；已装的格子再点 → 取出。
/// 未贴票时只有第 1 格可装（C# `UpdateParcel`：`Cells[1..].Enabled = false`）。
fn mail_compose_click_system(
    mut mail: ResMut<MailState>,
    mut click: ResMut<crate::game::dialogs::inventory::InvClickState>,
    inv_q: Query<&Inventory, With<crate::actor::LocalPlayer>>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    panel: Query<(&MailComposeRoot, &Node)>,
) {
    if !(mail.compose && mail.compose_parcel) || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some((ox, oy)) = panel
        .iter()
        .find(|(r, _)| r.0 == ComposeWin::Parcel)
        .map(|(_, n)| crate::ui::theme::node_origin(n, parcel_origin(PARCEL_SIZE.0)))
    else {
        return;
    };
    let items = inv_q
        .single()
        .map(|inv| inv.items.as_slice())
        .unwrap_or(&[]);
    let slots = stamp_slots(mail.stamped);
    for i in 0..5usize {
        let x = ox + PARCEL_CELL_X0 + i as f32 * PARCEL_CELL_STEP;
        let y = oy + PARCEL_CELL_Y;
        if cursor.x < x
            || cursor.x > x + PARCEL_CELL_SIZE.0
            || cursor.y < y
            || cursor.y > y + PARCEL_CELL_SIZE.1
        {
            continue;
        }
        if mail.attach.get(i).is_some_and(|s| s.is_some()) {
            mail.attach[i] = None;
            request_mail_cost(&net, mail.compose_gold, &mail.attach, mail.stamped);
            return;
        }
        if i >= slots {
            return;
        }
        let Some(slot_idx) = click.selected() else {
            return;
        };
        let Some(uid) = items
            .get(slot_idx)
            .and_then(|s| s.as_ref())
            .map(|it| it.unique_id)
        else {
            return;
        };
        if mail.attach.iter().flatten().any(|u| *u == uid) {
            return;
        }
        mail.attach[i] = Some(uid);
        click.clear_selected();
        request_mail_cost(&net, mail.compose_gold, &mail.attach, mail.stamped);
        return;
    }
}

/// #3103：写邮件窗发送/取消/关闭（两窗各一套，[`MailComposeBtn`] 自带窗种类）。
///
/// - 写信窗发送：C# `MailComposeLetterDialog.SendButton` → `C.SendMail{Name, Message}`
///   （**不带**金币/附件/贴票）；
/// - 待寄窗发送：C# `MailComposeParcelDialog.SendButton` →
///   `C.SendMail{Name, Message, Gold, ItemsIdx[5], Stamped}`。
fn mail_compose_btn_system(
    mut mail: ResMut<MailState>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
    net: Res<NetConnection>,
    btns: Query<(Entity, &Interaction, &MailComposeBtn)>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    for (e, inter, btn) in &btns {
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        if !mail.compose || ((btn.0 == ComposeWin::Parcel) != mail.compose_parcel) {
            continue;
        }
        match btn.1 {
            ComposeBtn::Send => {
                let body_id = if mail.compose_parcel {
                    INPUT_PARCEL_BODY
                } else {
                    INPUT_LETTER_BODY
                };
                let to = input
                    .texts
                    .get(INPUT_RECIPIENT)
                    .cloned()
                    .unwrap_or_default();
                let body = input.texts.get(body_id).cloned().unwrap_or_default();
                if mail.compose_parcel {
                    let gold = mail.compose_gold;
                    let attach = mail.attach.clone();
                    send_composed_mail(&net, &to, &body, gold, &attach, mail.stamped);
                } else {
                    send_composed_mail(&net, &to, &body, 0, &[], false);
                }
                close_compose(&mut mail, &mut input);
            }
            ComposeBtn::Cancel | ComposeBtn::Close => close_compose(&mut mail, &mut input),
        }
    }
}

/// #2538：邮票交互（C# `StampParcel`/`UpdateParcel`）+ 贴票覆层/遮罩/邮资显示
fn mail_stamp_system(
    mut mail: ResMut<MailState>,
    inv_q: Query<&Inventory, With<crate::actor::LocalPlayer>>,
    net: Res<NetConnection>,
    stamp_btn: Query<(Entity, &Interaction), With<MailStampBtn>>,
    mut stamp_on: Query<&mut Visibility, (With<MailStampOn>, Without<MailItemCover>)>,
    mut cover: Query<&mut Visibility, (With<MailItemCover>, Without<MailStampOn>)>,
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
    let parcel_open = mail.compose && mail.compose_parcel;
    // C# `UpdateParcel`：贴票 → `StampButton.Index = 204` + `ItemCover.Visible = false`
    for mut vis in &mut stamp_on {
        *vis = if mail.stamped && parcel_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in &mut cover {
        *vis = if parcel_open && !mail.stamped {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // C# `ParcelCostLabel.Text = p.Cost.ToString()`（底图自带「邮资」印刷标签）
    let cost_text = mail.parcel_cost.to_string();
    for mut text in &mut cost_label {
        if text.0 != cost_text {
            text.0 = cost_text.clone();
        }
    }
    for (e, inter) in &stamp_btn {
        if edge(e, inter, &mut prev_inter) && parcel_open {
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

/// #3103：赠金（C# `GoldSendLabel.Click` → `new MirAmountBox(SendAmount, 116, GameScene.Gold)`；
/// OK 时 `GiftGoldAmount += Amount` 并 `CalculatePostage`）。
///
/// `AmountBoxResult` 是全客户端共享的数量框结果（背包拆分/丢弃也用它），
/// 故用 [`MailState::gold_ask_pending`] 只认领本窗发起的那一次。
fn mail_gold_system(
    mut mail: ResMut<MailState>,
    mut amount: ResMut<crate::game::dialogs::amount_box::AmountBoxState>,
    mut results: MessageReader<crate::game::dialogs::amount_box::AmountBoxResult>,
    gold_q: Query<&Gold, With<crate::actor::LocalPlayer>>,
    label: Query<(Entity, &Interaction), With<MailGoldLabel>>,
    net: Res<NetConnection>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    for (e, inter) in &label {
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        if !(mail.compose && mail.compose_parcel) {
            continue;
        }
        // C#：`GameScene.SelectedCell == null && GameScene.Gold > 0` 才弹数量框
        let have = gold_q.single().map(|g| g.0).unwrap_or(0);
        if have == 0 {
            continue;
        }
        amount.ask("赠送金额", have);
        mail.gold_ask_pending = true;
    }
    for r in results.read() {
        if !mail.gold_ask_pending {
            continue;
        }
        mail.gold_ask_pending = false;
        if let Some(n) = r.0 {
            if n > 0 {
                mail.compose_gold = mail.compose_gold.saturating_add(n);
                request_mail_cost(&net, mail.compose_gold, &mail.attach, mail.stamped);
            }
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

/// 发送写好的邮件（C# 两窗的 `SendButton` 同发 `C.SendMail{Name, Message, Gold, ItemsIdx[5], Stamped}`；
/// `Message` 就是各自 `MessageTextBox.Text`——写信窗不带金币/附件/贴票）。
pub fn send_composed_mail(
    net: &NetConnection,
    to: &str,
    body: &str,
    gold: u32,
    attach: &[Option<u64>],
    stamped: bool,
) {
    if to.is_empty() {
        // C# `ComposeMail`：`if (string.IsNullOrEmpty(recipientName)) return;`
        tracing::warn!("✉️ 收件人为空，不发送");
        return;
    }
    let message = body.to_string();
    let mut items_idx = [0u64; 5];
    for (i, slot) in attach.iter().enumerate().take(5) {
        if let Some(uid) = slot {
            items_idx[i] = *uid;
        }
    }
    net.send_packet(&mir2_shared::packets::client::mail::SendMail {
        name: to.to_string(),
        message,
        gold,
        items_idx,
        stamped,
    });
    tracing::info!(
        "✉️ 发送邮件: {}（金币 {}，附件 {}，贴票 {}）",
        to,
        gold,
        attach.iter().flatten().count(),
        stamped
    );
}

/// #3103：写邮件两窗的 spawn（C# `MailComposeLetterDialog` / `MailComposeParcelDialog`）。
///
/// - 写信窗 `Title[671]` 236x300 @ (100,100)、待寄窗 `Title[674]` 236x384 @ (背包真实宽+10, 0)；
/// - 两窗 `Movable = true`（可拖动）、关闭钮同为 `Prguse2[360..362]` @ `(W-27, 3)`；
/// - 底图缺失（资产未装）时整窗不建（与其它对话框同款「无帧则不建」语义）；
/// - 显隐由 [`mail_compose_ui_system`] 驱动（挂 [`AlwaysVisible`]：包裹窗要在邮件列表窗
///   未打开时也能显示——C# 邮局路径只 `InventoryDialog.Show()`）。
fn spawn_compose_windows(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cjk: &Handle<Font>,
) {
    // ---- 写信窗：C# `MailComposeLetterDialog` ----
    if let Some(bg) = load_lib_image(libs, images, LETTER_PANEL.0, LETTER_PANEL.1) {
        let letter = spawn_panel(
            commands,
            bg,
            LETTER_POS.0,
            LETTER_POS.1,
            LETTER_SIZE.0,
            LETTER_SIZE.1,
            40,
        );
        commands.entity(letter).insert((
            DialogRoot(COMPOSE_DRAG_KIND),
            MailComposeRoot(ComposeWin::Letter),
            AlwaysVisible,
        ));
        commands.entity(letter).with_children(|p| {
            spawn_label(
                p,
                cjk,
                "",
                COMPOSE_RECIPIENT_POS.0,
                COMPOSE_RECIPIENT_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailRecipientLabel);
            spawn_compose_body_input(p, cjk, INPUT_LETTER_BODY, LETTER_BODY_POS);
            if let Some(mut btn) = spawn_close_button(
                p,
                libs,
                images,
                LETTER_SIZE.0 - COMPOSE_CLOSE_DX,
                COMPOSE_CLOSE_Y,
                12,
            ) {
                btn.insert(MailComposeBtn(ComposeWin::Letter, ComposeBtn::Close));
            }
            spawn_compose_send_cancel(
                p,
                libs,
                images,
                ComposeWin::Letter,
                LETTER_SEND_POS,
                LETTER_CANCEL_POS,
            );
        });
    }

    // ---- 待寄包裹窗：C# `MailComposeParcelDialog` ----
    if let Some(bg) = load_lib_image(libs, images, PARCEL_PANEL.0, PARCEL_PANEL.1) {
        let (inv_w, _inv_h) = crate::game::dialogs::inventory::inventory_real_size(libs);
        let (px, py) = parcel_origin(inv_w);
        let parcel = spawn_panel(commands, bg, px, py, PARCEL_SIZE.0, PARCEL_SIZE.1, 40);
        commands.entity(parcel).insert((
            DialogRoot(COMPOSE_DRAG_KIND),
            MailComposeRoot(ComposeWin::Parcel),
            AlwaysVisible,
        ));
        commands.entity(parcel).with_children(|p| {
            spawn_label(
                p,
                cjk,
                "",
                COMPOSE_RECIPIENT_POS.0,
                COMPOSE_RECIPIENT_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailRecipientLabel);
            spawn_compose_body_input(p, cjk, INPUT_PARCEL_BODY, PARCEL_BODY_POS);
            // 邮票键 `Prguse2[203]` 20x20 @ (73,56) + 贴票覆层 `[204]`
            if let Some(h) = load_lib_image(libs, images, LibraryName::Prguse2, STAMP_FRAMES.0) {
                spawn_container(
                    p,
                    PARCEL_STAMP_POS.0,
                    PARCEL_STAMP_POS.1,
                    PARCEL_STAMP_SIZE.0,
                    PARCEL_STAMP_SIZE.1,
                    11,
                )
                .insert((Button, ImageNode::new(h.clone()), MailStampBtn));
                spawn_image(
                    p,
                    h,
                    PARCEL_STAMP_POS.0,
                    PARCEL_STAMP_POS.1,
                    PARCEL_STAMP_SIZE.0,
                    PARCEL_STAMP_SIZE.1,
                    12,
                )
                .insert((MailStampOn, Visibility::Hidden));
            }
            // 邮资 / 赠金裸数值（`ParcelCostLabel` @(63,269)、`GoldSendLabel` @(63,290) 143x15）
            spawn_label(
                p,
                cjk,
                "",
                PARCEL_COST_POS.0,
                PARCEL_COST_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailCostLabel);
            spawn_label(
                p,
                cjk,
                "",
                PARCEL_GOLD_POS.0,
                PARCEL_GOLD_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert((MailGoldLabel, Button));
            // 5 个附件格 35x31 @ (27+36i, 311)，遮罩 `ItemCover Title[676]` @ (63,310) 144x33
            for i in 0..5usize {
                spawn_item_cell_ui(
                    p,
                    images,
                    cjk,
                    PARCEL_CELL_X0 + i as f32 * PARCEL_CELL_STEP,
                    PARCEL_CELL_Y,
                    PARCEL_CELL_SIZE.0,
                    PARCEL_CELL_SIZE.1,
                    11,
                    i,
                )
                .insert(MailAttachSlot(i));
            }
            if let Some(h) = load_lib_image(libs, images, LibraryName::Title, PARCEL_COVER_INDEX) {
                spawn_image(
                    p,
                    h,
                    PARCEL_COVER_POS.0,
                    PARCEL_COVER_POS.1,
                    PARCEL_COVER_SIZE.0,
                    PARCEL_COVER_SIZE.1,
                    12,
                )
                .insert((MailItemCover, Visibility::Hidden));
            }
            if let Some(mut btn) = spawn_close_button(
                p,
                libs,
                images,
                PARCEL_SIZE.0 - COMPOSE_CLOSE_DX,
                COMPOSE_CLOSE_Y,
                13,
            ) {
                btn.insert(MailComposeBtn(ComposeWin::Parcel, ComposeBtn::Close));
            }
            spawn_compose_send_cancel(
                p,
                libs,
                images,
                ComposeWin::Parcel,
                PARCEL_SEND_POS,
                PARCEL_CANCEL_POS,
            );
        });
    }
}

/// 写邮件正文输入框（C# `MirTextBox`：`BackColour = Color.Black`、多行、无边框；
/// `MirTextBox.cs:262-283`）。`TextInputRect` 由 [`mail_compose_ui_system`] 按窗原点逐帧
/// 同步（窗可拖动），窗关闭时挪到屏外，避免隐藏窗的输入框抢走点击焦点。
fn spawn_compose_body_input(
    p: &mut ChildSpawnerCommands,
    cjk: &Handle<Font>,
    id: usize,
    rel: (f32, f32),
) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(rel.0),
            top: Val::Px(rel.1),
            width: Val::Px(COMPOSE_BODY_SIZE.0),
            height: Val::Px(COMPOSE_BODY_SIZE.1),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        TextInputField(id),
        TextInputMultiline,
        TextInputRect(0.0, 0.0, 0.0, 0.0),
    ))
    .with_children(|ic| {
        ic.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(4.0),
                top: Val::Px(2.0),
                width: Val::Px(COMPOSE_BODY_SIZE.0 - 8.0),
                ..default()
            },
            Text::new(String::new()),
            TextFont {
                font: FontSource::Handle(cjk.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            ZIndex(11),
            TextInputDisplay(id),
        ));
    });
}

/// 读邮件两窗（C# `MailReadLetterDialog` / `MailReadParcelDialog`）。
///
/// 与写侧同构：两窗都是**独立可拖窗**（`DialogRoot(READ_DRAG_KIND)` + [`AlwaysVisible`]，
/// 显隐由 [`mail_read_ui_system`] 按 `MailState.detail` 驱动）。
fn spawn_read_windows(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cjk: &Handle<Font>,
) {
    fn native_size(libs: &mut GameLibraries, index: usize) -> (f32, f32) {
        libs.0
            .get_image(LibraryName::Title, index)
            .map(|i| (i.width as f32, i.height as f32))
            .unwrap_or((76.0, 25.0))
    }

    // ---- 读书信窗：C# `MailReadLetterDialog`（`:979-1107`） ----
    if let Some(bg) = load_lib_image(libs, images, READ_LETTER_PANEL.0, READ_LETTER_PANEL.1) {
        let letter = spawn_panel(
            commands,
            bg,
            READ_POS.0,
            READ_POS.1,
            READ_SIZE.0,
            READ_SIZE.1,
            40,
        );
        commands.entity(letter).insert((
            DialogRoot(READ_DRAG_KIND),
            MailReadRoot(ReadWin::Letter),
            AlwaysVisible,
        ));
        commands.entity(letter).with_children(|p| {
            spawn_label(
                p,
                cjk,
                "",
                READ_SENDER_POS.0,
                READ_SENDER_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Sender));
            spawn_label(
                p,
                cjk,
                "",
                READ_DATE_POS.0,
                READ_DATE_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Date));
            spawn_label(
                p,
                cjk,
                "",
                READ_LETTER_BODY_POS.0,
                READ_LETTER_BODY_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Body(ReadWin::Letter)));
            // C# 三键：删除 `Title[540..542]` @(12,265)、锁定 `Title[686..688]` @(81,265)、
            // 取消 `Title[193..195]` @(154,265)——均不设 Size → 原生尺寸
            for (frames, pos, tag) in [
                (READ_DELETE_FRAMES, READ_DELETE_POS, ReadBtn::Delete),
                (READ_LOCK_FRAMES, READ_LOCK_POS, ReadBtn::Lock),
                (
                    COMPOSE_CANCEL_FRAMES,
                    READ_LETTER_CANCEL_POS,
                    ReadBtn::Cancel,
                ),
            ] {
                if let (Some(n), Some(h), Some(pr)) = (
                    load_lib_image(libs, images, LibraryName::Title, frames.0),
                    load_lib_image(libs, images, LibraryName::Title, frames.1),
                    load_lib_image(libs, images, LibraryName::Title, frames.2),
                ) {
                    let (w, hh) = native_size(libs, frames.0);
                    spawn_icon_button(p, n, h, pr, pos.0, pos.1, w, hh, 11)
                        .insert(MailReadButton(ReadWin::Letter, tag));
                }
            }
            if let Some(mut btn) = spawn_close_button(
                p,
                libs,
                images,
                READ_SIZE.0 - COMPOSE_CLOSE_DX,
                COMPOSE_CLOSE_Y,
                12,
            ) {
                btn.insert(MailReadButton(ReadWin::Letter, ReadBtn::Close));
            }
        });
    }

    // ---- 读包裹窗：C# `MailReadParcelDialog`（`:1108-1272`） ----
    if let Some(bg) = load_lib_image(libs, images, READ_PARCEL_PANEL.0, READ_PARCEL_PANEL.1) {
        let parcel = spawn_panel(
            commands,
            bg,
            READ_POS.0,
            READ_POS.1,
            READ_PARCEL_SIZE.0,
            READ_PARCEL_SIZE.1,
            40,
        );
        commands.entity(parcel).insert((
            DialogRoot(READ_DRAG_KIND),
            MailReadRoot(ReadWin::Parcel),
            AlwaysVisible,
        ));
        commands.entity(parcel).with_children(|p| {
            spawn_label(
                p,
                cjk,
                "",
                READ_SENDER_POS.0,
                READ_SENDER_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Sender));
            spawn_label(
                p,
                cjk,
                "",
                READ_DATE_POS.0,
                READ_DATE_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Date));
            spawn_label(
                p,
                cjk,
                "",
                READ_PARCEL_BODY_POS.0,
                READ_PARCEL_BODY_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Body(ReadWin::Parcel)));
            spawn_label(
                p,
                cjk,
                "",
                READ_GOLD_POS.0,
                READ_GOLD_POS.1,
                12.0,
                Color::WHITE,
                10,
            )
            .insert(MailReadLabel(ReadLabel::Gold));
            // 5 个附件格 35x31 @ (27+36i, 311)（只读展示；C# 是 `MirItemCell` GridType=Mail）
            for i in 0..5usize {
                spawn_item_cell_ui(
                    p,
                    images,
                    cjk,
                    READ_CELL_X0 + i as f32 * READ_CELL_STEP,
                    READ_CELL_Y,
                    READ_CELL_SIZE.0,
                    READ_CELL_SIZE.1,
                    11,
                    i,
                )
                .insert(MailReadCell(i));
            }
            // 收取钮：已领取 `Title[680..682]` / 未领取（禁用）`Title[683..685]` @(30,350)
            let frames = [READ_COLLECT_OK_FRAMES, READ_COLLECT_DISABLED_FRAMES].map(|f| {
                (
                    load_lib_image(libs, images, LibraryName::Title, f.0),
                    load_lib_image(libs, images, LibraryName::Title, f.1),
                    load_lib_image(libs, images, LibraryName::Title, f.2),
                )
            });
            if let (Some(a), Some(b)) = (triple(&frames[0]), triple(&frames[1])) {
                let (w, hh) = native_size(libs, READ_COLLECT_OK_FRAMES.0);
                spawn_icon_button(
                    p,
                    a.0.clone(),
                    a.1.clone(),
                    a.2.clone(),
                    READ_COLLECT_POS.0,
                    READ_COLLECT_POS.1,
                    w,
                    hh,
                    11,
                )
                .insert((
                    MailReadButton(ReadWin::Parcel, ReadBtn::Collect),
                    MailCollectFrames {
                        ok: [a.0, a.1, a.2],
                        disabled: [b.0, b.1, b.2],
                    },
                ));
            }
            if let Some(mut btn) = spawn_close_button(
                p,
                libs,
                images,
                READ_PARCEL_SIZE.0 - COMPOSE_CLOSE_DX,
                COMPOSE_CLOSE_Y,
                12,
            ) {
                btn.insert(MailReadButton(ReadWin::Parcel, ReadBtn::Close));
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(libs, images, LibraryName::Title, COMPOSE_CANCEL_FRAMES.0),
                load_lib_image(libs, images, LibraryName::Title, COMPOSE_CANCEL_FRAMES.1),
                load_lib_image(libs, images, LibraryName::Title, COMPOSE_CANCEL_FRAMES.2),
            ) {
                let (w, hh) = native_size(libs, COMPOSE_CANCEL_FRAMES.0);
                spawn_icon_button(
                    p,
                    n,
                    h,
                    pr,
                    READ_PARCEL_CANCEL_POS.0,
                    READ_PARCEL_CANCEL_POS.1,
                    w,
                    hh,
                    11,
                )
                .insert(MailReadButton(ReadWin::Parcel, ReadBtn::Cancel));
            }
        });
    }
}

/// 三帧句柄元组（缺任一帧 → `None`，对应控件不建）
fn triple(
    f: &(
        Option<Handle<Image>>,
        Option<Handle<Image>>,
        Option<Handle<Image>>,
    ),
) -> Option<(Handle<Image>, Handle<Image>, Handle<Image>)> {
    Some((f.0.clone()?, f.1.clone()?, f.2.clone()?))
}

/// 写邮件窗「发送 / 取消」（C# 两窗同为 `Title[607/608/609]` 与 `Title[193/194/195]`，
/// 均不设 `Size` → 取精灵原生尺寸）。
fn spawn_compose_send_cancel(
    p: &mut ChildSpawnerCommands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    win: ComposeWin,
    send_pos: (f32, f32),
    cancel_pos: (f32, f32),
) {
    fn native_size(libs: &mut GameLibraries, index: usize) -> (f32, f32) {
        libs.0
            .get_image(LibraryName::Title, index)
            .map(|i| (i.width as f32, i.height as f32))
            .unwrap_or((76.0, 25.0))
    }
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Title, COMPOSE_SEND_FRAMES.0),
        load_lib_image(libs, images, LibraryName::Title, COMPOSE_SEND_FRAMES.1),
        load_lib_image(libs, images, LibraryName::Title, COMPOSE_SEND_FRAMES.2),
    ) {
        let (w, hh) = native_size(libs, COMPOSE_SEND_FRAMES.0);
        spawn_icon_button(p, n, h, pr, send_pos.0, send_pos.1, w, hh, 11)
            .insert(MailComposeBtn(win, ComposeBtn::Send));
    }
    if let (Some(n), Some(h), Some(pr)) = (
        load_lib_image(libs, images, LibraryName::Title, COMPOSE_CANCEL_FRAMES.0),
        load_lib_image(libs, images, LibraryName::Title, COMPOSE_CANCEL_FRAMES.1),
        load_lib_image(libs, images, LibraryName::Title, COMPOSE_CANCEL_FRAMES.2),
    ) {
        let (w, hh) = native_size(libs, COMPOSE_CANCEL_FRAMES.0);
        spawn_icon_button(p, n, h, pr, cancel_pos.0, cancel_pos.1, w, hh, 11)
            .insert(MailComposeBtn(win, ComposeBtn::Cancel));
    }
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

    // C# MailListDialog: Title[670] 312x444 @ (ScreenWidth-W-150, 5).
    let (panel_x, panel_y) = mail_panel_origin(MAIL_SCREEN_W);
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 670) else {
        return;
    };
    let list = spawn_panel(&mut commands, bg, panel_x, panel_y, MAIL_W, MAIL_H, 30);
    commands.entity(list).insert((
        DialogRoot(DialogKind::Mail),
        MailWidget,
        UiScrollList {
            rect_rel: (
                10.0,
                mail_row_y(0),
                290.0,
                MAIL_ROW_H * MAIL_VISIBLE_ROWS as f32,
            ),
            row_h: MAIL_ROW_H,
            visible: MAIL_VISIBLE_ROWS,
            total: 0,
            offset: 0,
            // 每格 1 行：C# `MailDialogs.cs` 全文**没有** MouseWheel 处理，本端
            // 的列表滚动是增补；行数取与其余列表（NPC/商品/行会/商城）同一口径
            step: 1,
            track_rel: (
                300.0,
                mail_row_y(0),
                8.0,
                MAIL_ROW_H * MAIL_VISIBLE_ROWS as f32,
            ),
            thumb: None,
            z: 9,
        },
    ));
    // #3106 残留③：列表行不是 `Button`（`mail_ui_system` 自算命中），给它这块矩形挂「按下不起拖」，
    // 否则选信时的按下+轻微移动会把整个邮件窗拖走（独立复核实测的形态）。
    commands
        .entity(list)
        .insert(crate::game::dialogs::DragBlockArea((
            10.0,
            mail_row_y(0),
            290.0,
            MAIL_ROW_H * MAIL_VISIBLE_ROWS as f32,
        )));

    commands.entity(list).with_children(|p| {
        spawn_scroll_bar_ui(
            p,
            (
                300.0,
                mail_row_y(0),
                8.0,
                MAIL_ROW_H * MAIL_VISIBLE_ROWS as f32,
            ),
            9,
        );
        // C# TitleLabel = Title[7]，不是 NEW CHARACTER（Title[20]）。
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 7) {
            let (w, h_px) = libs
                .0
                .get_image(LibraryName::Title, 7)
                .map(|i| (i.width as f32, i.height as f32))
                .unwrap_or((276.0, 25.0));
            crate::ui::theme::spawn_image(p, h, 18.0, 9.0, w, h_px, 8);
        }
        // C# 三段表头 @ y=34。
        spawn_label(p, &cjk, "类型", 8.0, 38.0, 12.0, Color::WHITE, 9);
        spawn_label(p, &cjk, "发件人", 47.0, 38.0, 12.0, Color::WHITE, 9);
        spawn_label(p, &cjk, "信息", 181.0, 38.0, 12.0, Color::WHITE, 9);
        // C# CloseButton @ (W-24, 3)。
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(MailClose);
        }
        // C# 10 行 @ 55 + 33*i；行点击由 mail_ui_system 按同一常量命中。
        // #3120 ①：每行按 C# `MailItemRow` 铺 7 个 UI 层（选中底图 / 图标 / 未读·锁定·包裹 /
        // 发件人 / 信息）；可见性与位置由 `mail_ui_system` 每帧按模型 + `mail_row_*` 口径写。
        for i in 0..MAIL_VISIBLE_ROWS {
            let ry = mail_row_y(i);
            // ① 选中底图 Prguse[545] @(-5,-3)
            spawn_row_image(
                p,
                &mut libs,
                &mut images,
                LibraryName::Prguse,
                MAIL_ROW_SELECTED_BG,
                MAIL_ROW_X + MAIL_ROW_SELECTED_OFFSET.0,
                ry + MAIL_ROW_SELECTED_OFFSET.1,
                7,
                i,
                MailRowRole::Selected,
            );
            // ② 图标（先按"空图标"占位；每帧按模型三分支换帧 + 居中）
            spawn_row_image(
                p,
                &mut libs,
                &mut images,
                LibraryName::Prguse,
                MAIL_ROW_ICON_EMPTY,
                MAIL_ROW_X,
                ry,
                8,
                i,
                MailRowRole::Icon,
            );
            // ③④⑤ 未读/锁定/包裹角标（默认 @(5,17)；未读在"未取回或锁定"时让位 (20,17)）
            for (index, role) in [
                (MAIL_ROW_BADGE_UNREAD, MailRowRole::Unread),
                (MAIL_ROW_BADGE_LOCKED, MailRowRole::Locked),
                (MAIL_ROW_BADGE_PARCEL, MailRowRole::Parcel),
            ] {
                spawn_row_image(
                    p,
                    &mut libs,
                    &mut images,
                    LibraryName::Prguse,
                    index,
                    MAIL_ROW_X + MAIL_ROW_BADGE_POS.0,
                    ry + MAIL_ROW_BADGE_POS.1,
                    9,
                    i,
                    role,
                );
            }
            // ⑥⑦ 发件人 @(35,0) / 信息 @(170,0)（C# 都是 31 高的 VerticalCenter → 12px 字取 +9）
            let mut sender = spawn_label(
                p,
                &cjk,
                "",
                MAIL_ROW_X + MAIL_ROW_SENDER_RECT.0,
                ry + 9.0,
                12.0,
                Color::WHITE,
                9,
            );
            sender.insert(MailRowSlot {
                row: i,
                role: MailRowRole::Sender,
                icon_lib: "",
                icon_index: 0,
            });
            let mut info = spawn_label(
                p,
                &cjk,
                "",
                MAIL_ROW_X + MAIL_ROW_INFO_RECT.0,
                ry + 9.0,
                12.0,
                Color::WHITE,
                9,
            );
            info.insert(MailRowSlot {
                row: i,
                role: MailRowRole::Info,
                icon_lib: "",
                icon_index: 0,
            });
        }
        // #3120 ③：分页键 + 页号（C# `MailDialogs.cs:99-151`；H=444 → y=389）
        for (pos, frames, is_next) in [
            (PAGE_PREV_POS, PAGE_PREV_FRAMES, false),
            (PAGE_NEXT_POS, PAGE_NEXT_FRAMES, true),
        ] {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, frames.0),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, frames.1),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, frames.2),
            ) {
                // MirButton 用帧的原生尺寸（不显式给 Size）
                let (bw, bh) = libs
                    .0
                    .get_image(LibraryName::Prguse2, frames.0)
                    .map(|i| (i.width as f32, i.height as f32))
                    .unwrap_or((15.0, 15.0));
                let mut btn = spawn_icon_button(p, n, h, pr, pos.0, pos.1, bw, bh, 10);
                if is_next {
                    btn.insert(MailPageNext);
                } else {
                    btn.insert(MailPagePrev);
                }
            }
        }
        spawn_label(
            p,
            &cjk,
            "",
            PAGE_LABEL_POS.0,
            PAGE_LABEL_POS.1,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(MailPageLabel);
        // C# 列表操作按钮 y=414：写邮件 @75 / 回复 @102 / 阅读 @129 / 删除 @156
        let actions = [
            (75.0, 563usize, 564usize, 565usize, MailAction::Write),
            (102.0, 569, 570, 571, MailAction::Reply),
            (129.0, 572, 573, 574, MailAction::Read),
            (156.0, 557, 558, 559, MailAction::Delete),
        ];
        for (x, normal, hover, pressed, action) in actions {
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, normal),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, hover),
                load_lib_image(&mut libs, &mut images, LibraryName::Prguse, pressed),
            ) {
                let mut e = spawn_icon_button(p, n, h, pr, x, MAIL_BUTTON_Y, 24.0, 24.0, 10);
                match action {
                    MailAction::Write => {
                        // #2771：C# `MailDialogs.cs:163` `SendButton` Hint（发送；同位置同三帧 563..565）
                        e.insert((
                            MailWrite,
                            crate::ui::tooltip::UiHint {
                                text: "发送".to_string(),
                            },
                        ));
                    }
                    MailAction::Reply => {
                        // #2786：C# `MailDialogs.cs:189` `ReplyButton` Hint（回复）
                        e.insert((
                            MailReplyBtn,
                            crate::ui::tooltip::UiHint {
                                text: "回复".to_string(),
                            },
                        ));
                    }
                    MailAction::Read => {
                        // #2771：C# `MailDialogs.cs:207` `ReadButton` Hint（读取）
                        e.insert((
                            MailReadBtn,
                            crate::ui::tooltip::UiHint {
                                text: "读取".to_string(),
                            },
                        ));
                    }
                    MailAction::Delete => {
                        // #2771：C# `MailDialogs.cs:232` `DeleteButton` Hint（删除）
                        e.insert((
                            MailDelete,
                            crate::ui::tooltip::UiHint {
                                text: "删除".to_string(),
                            },
                        ));
                    }
                }
            }
        }
        // C# `MailDialogs.cs:257-283`：`BlockListButton`/`BugReportButton` 是构造即
        // `GrayScale = true, Enabled = false` 的禁用占位键（`Prguse[520]` @(183,414)、
        // `Prguse[523]` @(210,414)，均 28x25）；两键无 Click 处理，且 `MirControl` 的
        // `AllowDisabledMouseOver` 默认 false（连 Hint 都不弹）→ Bevy 用批12 的灰度绘制
        // 纯占位图，不挂 Button/Interaction。
        for (idx, x) in [(520usize, 183.0f32), (523, 210.0)] {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx) {
                spawn_image(p, h, x, 414.0, 28.0, 25.0, 10).insert(UiGray::new(true));
            }
        }
    });

    spawn_compose_windows(&mut commands, &mut libs, &mut images, &cjk);
    spawn_read_windows(&mut commands, &mut libs, &mut images, &cjk);
}

/// #3120 ①：行相关的三个查询打包成一个 SystemParam——
/// `mail_ui_system` 的参数已接近 Bevy 的 16 个上限，直接加两个会编译不过。
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MailRowQueries<'w, 's> {
    pub(crate) widgets:
        Query<'w, 's, &'static mut Visibility, (With<MailWidget>, Without<MailRowSlot>)>,
    pub(crate) row_slots: Query<
        'w,
        's,
        (
            &'static mut MailRowSlot,
            &'static mut Visibility,
            &'static mut Node,
            Option<&'static mut ImageNode>,
            Option<&'static mut Text>,
        ),
        // B0001：`panel_origin: Query<&Node, With<MailWidget>>` 与本查询都要 `Node`，
        // 用 `Without<MailWidget>` 证明两者不相交（行层不是面板本身）。
        Without<MailWidget>,
    >,
    pub(crate) page_labels:
        Query<'w, 's, &'static mut Text, (With<MailPageLabel>, Without<MailRowSlot>)>,
}

fn mail_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut mail: ResMut<MailState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    close: Query<(Entity, &Interaction), With<MailClose>>,
    delete_btn: Query<(Entity, &Interaction), With<MailDelete>>,
    read_btn: Query<(Entity, &Interaction), With<MailReadBtn>>,
    // #3103 读侧：阅读态不再画在列表窗内（内容改由独立读邮件窗渲染）。
    // #3120 ①：列表行不再是"一行文本"，而是每行 7 个 `MailRowSlot` 层——
    // 本系统按模型 + `mail_row_*` 口径每帧写它们的可见性/位置/帧/文本。
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut rows: MailRowQueries,
    mut scroll: Query<&mut UiScrollList, With<MailWidget>>,
    page_prev: Query<(Entity, &Interaction), With<MailPagePrev>>,
    page_next: Query<(Entity, &Interaction), With<MailPageNext>>,
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
    for mut vis in &mut rows.widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter) in &close {
        if edge(e, inter, &mut prev_inter) {
            // C# `MailDialogs.cs:85` `CloseButton.Click += Hide()`——只关列表窗本身。
            // 读邮件窗是**独立窗**（`:701-703`），不随列表关闭；只有 ESC 的 Closeall
            // 会把它们一起隐藏（见 `mail_compose_follow_system`）。
            mail.selected = None;
            mgr.close(DialogKind::Mail);
        }
    }
    // #3120 ③：分页键（C# `MailDialogs.cs:109-120` / `:141-151`）——
    // 到边界直接 return（不夹住、不清选中），翻页时 `SelectedMail = null`。
    // 放在行渲染之前，这样点完当帧就能看到新一页。
    let pc_now = mail_page_count(mail.mails.len());
    // 本帧是否由**分页键**换了页：换页后必须把滚轮 offset 拉回页首（见下面的对账），
    // 否则下一帧会把这个「offset 与页首不一致」误判成滚轮动作，把页码又推回去。
    let mut page_changed = false;
    for (e, inter) in &page_prev {
        if edge(e, inter, &mut prev_inter) {
            if let Some(p) = mail_page_after_prev(mail.page) {
                mail.selected = None;
                mail.page = p;
                page_changed = true;
                tracing::info!("📧 邮件列表上一页 → {}/{}", mail.page, pc_now);
            }
        }
    }
    for (e, inter) in &page_next {
        if edge(e, inter, &mut prev_inter) {
            if let Some(p) = mail_page_after_next(mail.page, mail.mails.len()) {
                mail.selected = None;
                mail.page = p;
                page_changed = true;
                tracing::info!("📧 邮件列表下一页 → {}/{}", mail.page, pc_now);
            }
        }
    }
    // 列表（#89 支持滚轮滚动；#3120 ③ 起滚轮按「一格 = 一页」走，与分页键同一份状态）
    let mut sl = scroll.single_mut();
    if let Ok(sl) = sl.as_mut() {
        let pc = mail_page_count(mail.mails.len());
        // 让滚轮偏移量**就表示「页码-1」**：UiScrollList 的 `max_offset = total - visible`，
        // 所以把 total 设成 `page_count + visible - 1` → `max_offset = page_count - 1`；
        // 步长 1 ⇒ 一格滚轮翻一页。这样 offset 永远落在合法区间内，
        // 不会被 max_offset 夹到（旧写法把 total 设成邮件数，21 封时 max_offset=11，
        // 第 3 页要的 offset=20 被夹成 11 → 下一帧又被当成「滚轮上滚」，页码来回跳）。
        sl.set_total(pc + MAIL_VISIBLE_ROWS - 1);
        sl.step = 1;
        if mail.page > pc {
            // 邮件被删/读走后页数变少：夹住当前页（C# 每次 UpdateInterface 重算 PageCount）
            mail.page = pc;
        }
        let want_off = mail.page - 1;
        if sl.offset != want_off {
            if page_changed {
                // 分页键换了页：只是把 offset 对齐到新页（不是滚轮输入）
                sl.offset = want_off;
            } else {
                // 滚轮把 offset 推动了：它本身就是目标页码-1
                let p = (sl.offset + 1).clamp(1, pc);
                if p != mail.page {
                    mail.selected = None;
                    mail.page = p;
                }
                sl.offset = mail.page - 1;
            }
        }
        let start = mail_page_start(mail.page);
        for (mut slot, mut vis, mut node, img, text) in &mut rows.row_slots {
            let row = slot.row;
            let idx = start + row;
            let m = mail.mails.get(idx);
            let ry = mail_row_y(row);
            let (mut x, mut y) = (MAIL_ROW_X, ry);
            let mut want = Visibility::Hidden;
            match slot.role {
                MailRowRole::Selected => {
                    if m.is_some() && mail.selected == Some(idx) {
                        want = Visibility::Visible;
                    }
                    x = MAIL_ROW_X + MAIL_ROW_SELECTED_OFFSET.0;
                    y = ry + MAIL_ROW_SELECTED_OFFSET.1;
                }
                MailRowRole::Icon => {
                    if let Some(m) = m {
                        // C# 三分支（:517-531）：有附件 → `Items[Items[0].Info.Image]`；有金币 → 541；否则 540
                        let (lib, index) = match mail_row_icon(m) {
                            MailRowIcon::Item(image) => (LibraryName::Items, image),
                            MailRowIcon::Gold => (LibraryName::Prguse, MAIL_ROW_ICON_GOLD),
                            MailRowIcon::Empty => (LibraryName::Prguse, MAIL_ROW_ICON_EMPTY),
                        };
                        slot.icon_lib = if lib == LibraryName::Items {
                            "Items"
                        } else {
                            "Prguse"
                        };
                        slot.icon_index = index;
                        if let (Some(mut img), Some(handle)) = (
                            img,
                            load_lib_image(&mut libs, &mut images, lib, index as usize),
                        ) {
                            img.image = handle;
                        }
                        // 图标在 `IconArea 34x32` 内居中（:533）：尺寸取该帧真实像素
                        let (w, h) = libs
                            .0
                            .get_image(lib, index as usize)
                            .map(|i| (i.width as f32, i.height as f32))
                            .unwrap_or((1.0, 1.0));
                        let (ox, oy) = mail_row_icon_offset(w, h);
                        x = MAIL_ROW_X + ox;
                        y = ry + oy;
                        want = Visibility::Visible;
                    }
                }
                MailRowRole::Unread => {
                    if let Some(m) = m {
                        if m.unread {
                            want = Visibility::Visible;
                        }
                        // C# 让位规则（:546-562）：未取回包裹 或 已锁定 → 第二位 (20,17)
                        x = MAIL_ROW_X + mail_row_unread_x(m);
                    }
                    y = ry + MAIL_ROW_BADGE_POS.1;
                }
                MailRowRole::Locked => {
                    if m.map(|m| m.locked).unwrap_or(false) {
                        want = Visibility::Visible;
                    }
                    x = MAIL_ROW_X + MAIL_ROW_BADGE_POS.0;
                    y = ry + MAIL_ROW_BADGE_POS.1;
                }
                MailRowRole::Parcel => {
                    if m.map(|m| !m.collected).unwrap_or(false) {
                        want = Visibility::Visible;
                    }
                    x = MAIL_ROW_X + MAIL_ROW_BADGE_POS.0;
                    y = ry + MAIL_ROW_BADGE_POS.1;
                }
                MailRowRole::Sender => {
                    if let Some(mut t) = text {
                        let want_text = m.map(|m| m.sender.clone()).unwrap_or_default();
                        if t.0 != want_text {
                            t.0 = want_text;
                        }
                    }
                    x = MAIL_ROW_X + MAIL_ROW_SENDER_RECT.0;
                    y = ry + 9.0;
                }
                MailRowRole::Info => {
                    if let Some(mut t) = text {
                        let want_text = m.map(mail_row_info_text).unwrap_or_default();
                        if t.0 != want_text {
                            t.0 = want_text;
                        }
                    }
                    x = MAIL_ROW_X + MAIL_ROW_INFO_RECT.0;
                    y = ry + 9.0;
                }
            }
            if *vis != want {
                *vis = want;
            }
            if node.left != Val::Px(x) {
                node.left = Val::Px(x);
            }
            if node.top != Val::Px(y) {
                node.top = Val::Px(y);
            }
        }
    }
    // 页号（C# `MailDialogs.cs:306`：`"{CurrentPage} / {PageCount}"`）
    if let Ok(mut t) = rows.page_labels.single_mut() {
        let want = mail_page_label(mail.page, mail.mails.len());
        if t.0 != want {
            t.0 = want;
        }
    }
    // 阅读按钮（C# `MailDialogs.cs:209-221`）：有选中行就开对应的读邮件窗
    // （窗的显隐由 `MailState.detail` 驱动，见 `mail_read_ui_system`）。
    for (e, inter) in &read_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = mail.selected {
                if let Some(m) = mail.mails.get(idx) {
                    net.send_packet(&mir2_shared::packets::client::mail::ReadMail {
                        mail_id: m.mail_id,
                    });
                    tracing::info!("📧 读取邮件按钮: {} ({})", m.subject, m.mail_id);
                }
            }
        }
    }

    // 行点击（C# `MailDialogs.cs:322-343`；行号 = 滚动偏移 + 可视槽位）：
    // 点**未选中**行 = 只选中（不发包）；点**已选中**行 = 打开读邮件窗（C.ReadMail）。
    if mouse.just_pressed(MouseButton::Left) {
        let Ok(window) = windows.single() else { return };
        let Some(cursor) = window.cursor_position() else {
            return;
        };
        // #3120 ③：行号 = 页首 + 可视槽位（与渲染同一份分页状态；不再读滚轮 offset）
        let off = mail_page_start(mail.page);
        let (ox, oy) = panel_origin
            .single()
            .map(|n| crate::ui::theme::node_origin(n, mail_panel_origin(MAIL_SCREEN_W)))
            .unwrap_or(mail_panel_origin(MAIL_SCREEN_W));
        for i in 0..MAIL_VISIBLE_ROWS {
            let y = oy + mail_row_y(i);
            if cursor.x >= ox + 10.0
                && cursor.x <= ox + MAIL_W - 12.0
                && cursor.y >= y
                && cursor.y <= y + MAIL_ROW_H
            {
                if let Some(m) = mail.mails.get(off + i) {
                    let mail_id = m.mail_id;
                    let subject = m.subject.clone();
                    if mail.selected == Some(off + i) {
                        net.send_packet(&mir2_shared::packets::client::mail::ReadMail { mail_id });
                        tracing::info!("📧 读取邮件: {} ({})", subject, mail_id);
                    } else {
                        mail.selected = Some(off + i);
                        tracing::info!("📧 选中邮件: {} ({})", subject, mail_id);
                    }
                }
                break;
            }
        }
    }
    // 删除邮件（#132；C# `MailDialogs.cs:234-236`：`SelectedMail.Locked` 时拒发）
    for (e, inter) in &delete_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(idx) = mail.selected {
                if mail.mails.get(idx).map(|m| m.locked).unwrap_or(false) {
                    tracing::info!("📧 邮件已锁定，拒绝删除");
                    continue;
                }
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

/// #3103 读侧：两张读邮件窗的显隐、内容与按钮。
///
/// 逐条对齐 C#：
/// - **选窗**：`MailDialogs.cs:213-220`（阅读钮）与 `:322-343`（行点击）同式——
///   `Gold > 0 || Items.Count > 0` → `MailReadParcelDialog`，否则 `MailReadLetterDialog`
///   （见 [`read_window_for`]）；
/// - **两窗互斥**：C# 未显式隐藏另一张（两张底图同位置，真同开会互相压住）；
///   本端「只显示当前这封该开的那张」，等价于后读覆盖先读且不留叠影（PR 已说明）；
/// - **字段**：发件人 `@(70,35)`、发信时间 `@(70,56)`（`dd/MM/yy H:mm:ss`）、正文
///   `@(15,92|98) 202x165`、赠金 `@(63,290)`；附件格 5 个 `35x31 @(27+36i,311)`；
/// - **关闭**：X / 取消 = `MailDialogs.cs:1006/1087/1137/1202` 的 `Hide()`（本端清 `detail`）；
/// - **删除**：`:1047-1056`，`Mail.Locked` 时直接 return（不发 `C.DeleteMail`）；
/// - **锁定**：`:1068-1075`，`Mail.Locked = !Mail.Locked` + `C.LockMail{lock}`；
/// - **收取**：`:1177-1190`，`C.CollectParcel`；未领取（`!Collected`）时按钮换成
///   `Title[683/684/685]` 且禁用（`:1243-1256`）。
#[allow(clippy::too_many_arguments)]
fn mail_read_ui_system(
    mut mail: ResMut<MailState>,
    net: Res<NetConnection>,
    mut roots: Query<(&MailReadRoot, &mut Visibility)>,
    mut labels: Query<(&MailReadLabel, &mut Text)>,
    mut buttons: Query<(
        Entity,
        &Interaction,
        &MailReadButton,
        Option<&mut ImageButton>,
        Option<&mut ImageNode>,
        Option<&MailCollectFrames>,
    )>,
    mut cells: Query<(&MailReadCell, &mut UiItemCellData)>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
    mut rendered: Local<Option<(u64, bool, usize)>>,
) {
    let want = mail.detail.as_ref().map(read_window_for);
    for (root, mut vis) in &mut roots {
        *vis = if want == Some(root.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let Some(d) = mail.detail.clone() else {
        return;
    };

    for (label, mut text) in &mut labels {
        let s = match label.0 {
            ReadLabel::Sender => d.sender.clone(),
            ReadLabel::Date => format_send_date(d.send_date),
            ReadLabel::Body(_) => d.body.clone(),
            ReadLabel::Gold => format_gold(d.gold),
        };
        if text.0 != s {
            text.0 = s;
        }
    }

    // 附件格：只在「读了另一封 / 领取状态变了 / 附件个数变了」时重建图标，
    // 否则每帧 `load_lib_image` 会往 `Assets<Image>` 里塞新图（资产泄漏）。
    let key = (d.mail_id, d.collected, d.items.len());
    if *rendered != Some(key) {
        *rendered = Some(key);
        for (cell, mut data) in &mut cells {
            let att = d.items.get(cell.0);
            let icon = att.and_then(|a| {
                load_lib_image(&mut libs, &mut images, LibraryName::Items, a.image as usize)
            });
            *data = UiItemCellData {
                icon,
                count: att.map(|a| a.count as u32),
                dura_ratio: att.and_then(|a| a.dura_ratio),
            };
        }
    }

    // C# `UpdateInterface` 口径：`Collected` 才有可领取内容（`:1243-1256`）
    let can_collect = d.collected && (d.gold > 0 || !d.items.is_empty());
    for (e, inter, btn, mut image_btn, mut node, collect_frames) in &mut buttons {
        if let Some(cf) = collect_frames {
            let want_frames = if can_collect { &cf.ok } else { &cf.disabled };
            if let (Some(ib), Some(n)) = (image_btn.as_deref_mut(), node.as_deref_mut()) {
                if ib.normal != want_frames[0] {
                    ib.normal = want_frames[0].clone();
                    ib.hover = want_frames[1].clone();
                    ib.pressed = want_frames[2].clone();
                    n.image = want_frames[0].clone();
                }
            }
        }
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        match btn.1 {
            ReadBtn::Delete => {
                // C# `:1049` `if (Mail.Locked) return;`
                if d.locked {
                    tracing::info!("📧 邮件已锁定，拒绝删除（读信窗）");
                    continue;
                }
                net.send_packet(&mir2_shared::packets::client::mail::DeleteMail {
                    mail_id: d.mail_id,
                });
                tracing::info!("📧 删除邮件（读信窗）: {}", d.mail_id);
                mail.mails.retain(|m| m.mail_id != d.mail_id);
                mail.selected = None;
                mail.detail = None;
            }
            ReadBtn::Lock => {
                // C# `:1070-1074`：本地翻转 + `C.LockMail`
                let lock = !d.locked;
                net.send_packet(&mir2_shared::packets::client::mail::LockMail {
                    mail_id: d.mail_id,
                    lock,
                });
                if let Some(dd) = mail.detail.as_mut() {
                    dd.locked = lock;
                }
                if let Some(m) = mail.mails.iter_mut().find(|m| m.mail_id == d.mail_id) {
                    m.locked = lock;
                }
                tracing::info!("🔒 邮件锁定: {} → {}", d.mail_id, lock);
            }
            ReadBtn::Cancel | ReadBtn::Close => {
                mail.detail = None;
            }
            ReadBtn::Collect => {
                if can_collect {
                    net.send_packet(&mir2_shared::packets::client::mail::CollectParcel {
                        mail_id: d.mail_id,
                    });
                    tracing::info!("📦 收取附件（读信窗）: mail_id={}", d.mail_id);
                }
            }
        }
    }
}

/// 邮件列表窗的「写邮件 / 回复」两键（独立系统：`mail_ui_system` 的参数已到 Bevy 上限）。
///
/// - 写邮件（#3103）：C# `MailDialogs.cs:163-172` → `new MirInputBox(EnterMailToName)`，
///   OK 后 `MailComposeLetterDialog.ComposeMail(输入的名字)`；
/// - 回复（#2786）：C# `:191-196` → `if (SelectedMail == null) return;
///   ComposeMail(SelectedMail.SenderName)`（直接开写信窗，不再问名字）。
fn mail_reply_system(
    mail: Res<MailState>,
    write_btn: Query<(Entity, &Interaction), With<MailWrite>>,
    reply_btn: Query<(Entity, &Interaction), With<MailReplyBtn>>,
    mut input_box: ResMut<crate::game::dialogs::input_box::InputBoxState>,
    mut compose: MessageWriter<ComposeMail>,
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
    for (e, inter) in &write_btn {
        if edge(e, inter, &mut prev_inter) {
            input_box.open = true;
            input_box.title = "请输入收件人姓名".to_string();
            input_box.purpose =
                crate::game::dialogs::input_box::InputPurpose::MailRecipient { parcel: false };
            tracing::info!("✉️ 写邮件：先问收件人姓名");
        }
    }
    for (e, inter) in &reply_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(sender) = mail
                .selected
                .and_then(|idx| mail.mails.get(idx))
                .map(|m| m.sender.clone())
            {
                compose.write(ComposeMail {
                    to: sender.clone(),
                    // C# `ComposeMail(SenderName)` 不带正文预填
                    message: None,
                    parcel: false,
                });
                tracing::info!("📧 回复邮件: 收件人 {}", sender);
            }
        }
    }
}

/// #3103：写邮件两窗的收尾规则。
///
/// - 写信窗跟随邮件列表窗：C# `GameScene.cs:699-700` 关邮件窗时
///   `MailComposeLetterDialog.Hide()`；待寄包裹窗独立（C# 由 X/取消关，不随邮件列表窗）；
/// - ESC：C# `GameScene.cs:699-703` 的 `Closeall` 会 `Hide()` 两个写邮件窗**和两张读邮件窗**
///   （`:702-703`），故这里一并清 `MailState.detail`（读邮件窗的显隐由它驱动）。
fn mail_compose_follow_system(
    mgr: Res<DialogManager>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mail: ResMut<MailState>,
    mut input: ResMut<crate::game::dialogs::text_input::TextInputState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        // 读邮件窗：C# `GameScene.cs:702-703`（Closeall 一并 Hide）
        mail.detail = None;
    }
    if !mail.compose {
        return;
    }
    let letter_orphaned = !mail.compose_parcel && !mgr.is_open(DialogKind::Mail);
    if letter_orphaned || keys.just_pressed(KeyCode::Escape) {
        close_compose(&mut mail, &mut input);
    }
}

/// 消费服务端邮件事件（网络层只广播 ServerEvent）
fn mail_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut mail: ResMut<MailState>,
    mut input_box: ResMut<crate::game::dialogs::input_box::InputBoxState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        // #3103：S.MailSendRequest（邮局邮寄键）→ 先问收件人姓名（C# `MirInputBox` 文案
        // `EnterMailRecipientName`），OK 后开待寄包裹窗 + 打开背包（`GameScene.cs:6478-6490`）
        if let ServerEvent::MailSendRequest = ev {
            input_box.open = true;
            input_box.title = "请输入收件人姓名".to_string();
            input_box.purpose =
                crate::game::dialogs::input_box::InputPurpose::MailRecipient { parcel: true };
            tracing::info!("📦 邮局邮寄：先问收件人姓名");
        }
        // #2538：S.MailCost → 邮资标签
        if let ServerEvent::MailCost { cost } = ev {
            mail.parcel_cost = *cost;
            tracing::debug!("✉️ 邮资更新: {}", cost);
        }
        if let ServerEvent::ParcelCollected { result } = ev {
            match *result {
                1 => {
                    // C# Result=1：邮箱领取成功 → `MailReadParcelDialog.Hide()`（`GameScene.cs:6528`）
                    // + 列表行标记已领取（`MailItemRow.UpdateInterface` 的包裹角标随之消失）
                    mail.detail = None;
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

    /// 门禁（#3120 ①）：列表行的几何与判定必须与 C# `MailItemRow` 逐字一致
    /// （`Client/MirScenes/Dialogs/MailDialogs.cs:421-569`）。
    /// **阳性对照**：改动任一常量（例如把角标第二位写成 (5,17)、把 `[*] ` 前缀去掉、
    /// 或把图标三分支顺序调换）→ 本测试立即红。
    #[test]
    fn mail_row_matches_csharp_mail_item_row() {
        // 几何常量（逐条引 C# 行号）
        assert_eq!(MAIL_ROW_ICON_AREA, (34.0, 32.0), ":430 IconArea");
        assert_eq!(MAIL_ROW_BADGE_POS, (5.0, 17.0), ":451/:460/:469");
        assert_eq!(MAIL_ROW_BADGE_POS_SECOND, (20.0, 17.0), ":553/:561");
        assert_eq!(MAIL_ROW_SELECTED_OFFSET, (-5.0, -3.0), ":479");
        assert_eq!(MAIL_ROW_SENDER_RECT, (35.0, 0.0, 130.0, 31.0), ":487-489");
        assert_eq!(MAIL_ROW_INFO_RECT, (170.0, 0.0, 115.0, 31.0), ":496-498");
        assert_eq!(
            (MAIL_ROW_ICON_EMPTY, MAIL_ROW_ICON_GOLD),
            (540, 541),
            ":524/:529"
        );
        assert_eq!(
            (
                MAIL_ROW_BADGE_UNREAD,
                MAIL_ROW_BADGE_LOCKED,
                MAIL_ROW_BADGE_PARCEL
            ),
            (550, 551, 552),
            ":449/:458/:467"
        );
        assert_eq!(MAIL_ROW_SELECTED_BG, 545, ":477");

        // 图标三分支：有附件取 Items[0].Info.Image；无附件有金币 → 541；否则 540
        let mut m = MailEntry {
            gold: 0,
            ..Default::default()
        };
        assert_eq!(mail_row_icon(&m), MailRowIcon::Empty);
        m.gold = 250;
        assert_eq!(mail_row_icon(&m), MailRowIcon::Gold);
        m.items = vec![MailAttachment {
            name: "Saddle".into(),
            image: 1234,
            count: 1,
            dura_ratio: None,
        }];
        assert_eq!(
            mail_row_icon(&m),
            MailRowIcon::Item(1234),
            "有附件时金币图标让位"
        );

        // 图标在 IconArea 内居中：((34-w)/2, (32-h)/2)
        assert_eq!(mail_row_icon_offset(34.0, 32.0), (0.0, 0.0));
        assert_eq!(mail_row_icon_offset(20.0, 20.0), (7.0, 6.0));

        // 未读角标位置：未取回包裹 或 已锁定 → 第二位 (20)；否则 (5)
        let mut u = MailEntry {
            unread: true,
            collected: false,
            locked: false,
            ..Default::default()
        };
        assert_eq!(mail_row_unread_x(&u), 20.0, "未取回包裹 → 让位");
        u.collected = true;
        assert_eq!(mail_row_unread_x(&u), 5.0, "已取回且未锁定 → 第一位");
        u.locked = true;
        assert_eq!(mail_row_unread_x(&u), 20.0, "已锁定 → 让位");

        // 信息列文本：锁定加 `[*] `；换行压空格
        let mut t = MailEntry {
            subject: "第一行\r\n第二行".into(),
            ..Default::default()
        };
        assert_eq!(mail_row_info_text(&t), "第一行 第二行");
        t.locked = true;
        assert_eq!(mail_row_info_text(&t), "[*] 第一行 第二行");
    }

    /// 门禁（#3120 ② 残余）：不可邮寄物品的判定必须与服务端同口径
    /// （`ServerRust/src/actors/world/mail.rs:572` 拦 `DontTrade(0x10) | NoMail(0x4000)`；
    /// C# 客户端守卫读 `Info.Bind.HasFlag(BindMode.DontTrade)`）。
    /// **阳性对照**：把任一 flag 从判定里去掉 → 立即红。
    #[test]
    fn can_mail_item_matches_dont_trade_and_no_mail() {
        use mir2_shared::enums::BindMode;
        assert!(can_mail_item(0), "无绑定 → 可邮寄");
        assert!(
            can_mail_item(BindMode::DONT_SELL.bits()),
            "仅 DontSell 不影响邮寄"
        );
        assert!(
            !can_mail_item(BindMode::DONT_TRADE.bits()),
            "DontTrade 不可邮寄（C# 守卫 + 服务端 mail.rs:572）"
        );
        assert!(
            !can_mail_item(BindMode::NO_MAIL.bits()),
            "NoMail 不可邮寄（服务端同口径）"
        );
        assert!(
            !can_mail_item((BindMode::DONT_TRADE | BindMode::DONT_SELL).bits()),
            "组合位里含 DontTrade 仍不可邮寄"
        );
    }

    /// #2786：点「回复」→ 打开写信窗并把收件人预填为选中邮件的发件人
    ///（C# `MailDialogs.cs:191-196` `ComposeMail(SelectedMail.SenderName)`）
    #[test]
    fn reply_button_composes_mail_to_selected_sender() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        let mut st = MailState::default();
        st.mails = vec![
            MailEntry {
                mail_id: 11,
                sender: "张三".to_string(),
                subject: "你好".to_string(),
                unread: true,
                gold: 0,
                collected: false,
                locked: false,
                items: Vec::new(),
            },
            MailEntry {
                mail_id: 12,
                sender: "李四".to_string(),
                subject: "在吗".to_string(),
                unread: false,
                gold: 0,
                collected: false,
                locked: false,
                items: Vec::new(),
            },
        ];
        st.selected = Some(1);
        world.insert_resource(st);
        world.insert_resource(Messages::<ComposeMail>::default());
        world.insert_resource(crate::game::dialogs::input_box::InputBoxState::default());
        world.spawn((MailReplyBtn, Interaction::Pressed));
        world
            .run_system_once(mail_reply_system)
            .expect("回复系统应成功");
        let mut msgs = world.resource_mut::<Messages<ComposeMail>>();
        let drained: Vec<_> = msgs.drain().collect();
        assert_eq!(drained.len(), 1, "应写出一条 ComposeMail");
        assert_eq!(drained[0].to, "李四", "收件人 = 选中邮件发件人");
        assert_eq!(
            drained[0].message, None,
            "C# ComposeMail(SenderName) 不带正文"
        );
        assert!(
            !drained[0].parcel,
            "回复走写信窗（MailComposeLetterDialog）"
        );

        // 负控：未选中任何邮件 → C# `if (SelectedMail == null) return;` → 不写信
        world.resource_mut::<MailState>().selected = None;
        world
            .run_system_once(mail_reply_system)
            .expect("回复系统应成功");
        let mut msgs = world.resource_mut::<Messages<ComposeMail>>();
        assert_eq!(msgs.drain().count(), 0, "未选中邮件不得写信");
    }

    #[test]
    fn mail_list_layout_matches_csharp_anchor() {
        assert_eq!(mail_panel_origin(MAIL_SCREEN_W), (562.0, 5.0));
        assert_eq!(mail_row_y(0), 55.0);
        assert_eq!(mail_row_y(MAIL_VISIBLE_ROWS - 1), 352.0);
        assert!(mail_row_y(MAIL_VISIBLE_ROWS - 1) + MAIL_ROW_H < MAIL_BUTTON_Y);
        assert!(MAIL_BUTTON_Y + 24.0 <= MAIL_H);
    }

    /// #3120 ③：分页键/页号按 C# `MailDialogs.cs:99-151` 的锚点与帧号（H=444 → y=389）。
    #[test]
    fn mail_pager_layout_matches_csharp_anchor() {
        assert_eq!(MAIL_H - 55.0, 389.0);
        assert_eq!(
            PAGE_PREV_POS,
            (102.0, 389.0),
            "C# PreviousButton @(102,H-55)"
        );
        assert_eq!(PAGE_LABEL_POS, (120.0, 389.0), "C# PageLabel @(120,H-55)");
        assert_eq!(PAGE_NEXT_POS, (192.0, 389.0), "C# NextButton @(192,H-55)");
        assert_eq!(PAGE_LABEL_SIZE, (67.0, 15.0), "C# PageLabel.Size = 67x15");
        assert_eq!(PAGE_PREV_FRAMES, (240, 241, 242), "C# Prguse2[240..242]");
        assert_eq!(PAGE_NEXT_FRAMES, (243, 244, 245), "C# Prguse2[243..245]");
        // 分页键不与 10 行列表、底部操作按钮（y=414..438）相撞
        assert!(mail_row_y(MAIL_VISIBLE_ROWS - 1) + MAIL_ROW_H < PAGE_PREV_POS.1);
        assert!(PAGE_PREV_POS.1 + 15.0 <= MAIL_BUTTON_Y);
    }

    /// #3120 ③：页数/页首/页号文本＝C# `MailDialogs.cs:25/117/148/303-306`
    #[test]
    fn mail_pager_math_matches_csharp() {
        // PageCount = ceil(Count/10)，且至少 1（C# `:303-304`）
        assert_eq!(mail_page_count(0), 1);
        assert_eq!(mail_page_count(1), 1);
        assert_eq!(mail_page_count(10), 1);
        assert_eq!(mail_page_count(11), 2);
        assert_eq!(mail_page_count(20), 2);
        assert_eq!(mail_page_count(21), 3);
        assert_eq!(mail_page_count(200), 20);
        // StartIndex = (CurrentPage-1)*10（C# `:25/117/148`）
        assert_eq!(mail_page_start(0), 0, "页码夹到 1 → 页首 0");
        assert_eq!(mail_page_start(1), 0);
        assert_eq!(mail_page_start(2), 10);
        assert_eq!(mail_page_start(3), 20);
        // PageLabel.Text = "{CurrentPage} / {PageCount}"（C# `:306`）
        assert_eq!(mail_page_label(1, 0), "1 / 1");
        assert_eq!(mail_page_label(1, 10), "1 / 1");
        assert_eq!(mail_page_label(2, 11), "2 / 2");
        assert_eq!(mail_page_label(3, 21), "3 / 3");
    }

    /// #3120 ③：翻页边界＝C# 的「到边界直接 return」（不夹住、不清选中由调用方负责）
    #[test]
    fn mail_pager_buttons_stop_at_bounds() {
        assert_eq!(
            mail_page_after_prev(1),
            None,
            "C# `:111` CurrentPage<=1 → return"
        );
        assert_eq!(mail_page_after_prev(2), Some(1));
        assert_eq!(mail_page_after_prev(3), Some(2));
        // 0 封邮件时只有 1 页：下一页也不动
        assert_eq!(mail_page_after_next(1, 0), None);
        assert_eq!(
            mail_page_after_next(1, 10),
            None,
            "C# `:143` CurrentPage>=PageCount → return"
        );
        assert_eq!(mail_page_after_next(1, 11), Some(2));
        assert_eq!(mail_page_after_next(2, 21), Some(3));
        assert_eq!(mail_page_after_next(3, 21), None, "末页再点不动");
    }

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
        world.insert_resource(crate::network::NetConnection::default());
        world
            .resource_mut::<Messages<ComposeMail>>()
            .write(ComposeMail {
                to: "小明".to_string(),
                message: None,
                parcel: false,
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

    /// #3103：C# 两张写邮件窗的几何/底图/触发路径逐项对照
    ///（`Client/MirScenes/Dialogs/MailDialogs.cs:596-684 / 687-1010`）。
    #[test]
    fn compose_windows_match_csharp_geometry() {
        // 写信窗 `MailComposeLetterDialog`
        assert_eq!(LETTER_PANEL, (LibraryName::Title, 671));
        assert_eq!(LETTER_SIZE, (236.0, 300.0));
        assert_eq!(LETTER_POS, (100.0, 100.0));
        assert_eq!(LETTER_BODY_POS, (15.0, 92.0));
        assert_eq!(LETTER_SEND_POS, (30.0, 265.0));
        assert_eq!(LETTER_CANCEL_POS, (135.0, 265.0));
        // 待寄包裹窗 `MailComposeParcelDialog`
        assert_eq!(PARCEL_PANEL, (LibraryName::Title, 674));
        assert_eq!(PARCEL_SIZE, (236.0, 384.0));
        assert_eq!(parcel_origin(316.0), (326.0, 0.0), "背包宽 316 + 10");
        assert_eq!(parcel_origin(0.0), (10.0, 0.0));
        assert_eq!(PARCEL_BODY_POS, (15.0, 98.0));
        assert_eq!(PARCEL_STAMP_POS, (73.0, 56.0));
        assert_eq!(PARCEL_STAMP_SIZE, (20.0, 20.0));
        assert_eq!(PARCEL_COST_POS, (63.0, 269.0));
        assert_eq!(PARCEL_GOLD_POS, (63.0, 290.0));
        assert_eq!(PARCEL_COVER_POS, (63.0, 310.0));
        assert_eq!(PARCEL_COVER_SIZE, (144.0, 33.0));
        assert_eq!(PARCEL_CELL_X0, 27.0);
        assert_eq!(PARCEL_CELL_STEP, 36.0);
        assert_eq!(PARCEL_CELL_Y, 311.0);
        assert_eq!(PARCEL_CELL_SIZE, (35.0, 31.0));
        assert_eq!(PARCEL_SEND_POS, (30.0, 350.0));
        assert_eq!(PARCEL_CANCEL_POS, (135.0, 350.0));
        // 两窗共用：关闭钮 `(W-27, 3)`、收件人 (70,35) 150x15、正文 202x165
        assert_eq!(COMPOSE_CLOSE_DX, 27.0);
        assert_eq!(COMPOSE_CLOSE_Y, 3.0);
        assert_eq!(LETTER_SIZE.0 - COMPOSE_CLOSE_DX, 209.0);
        assert_eq!(PARCEL_SIZE.0 - COMPOSE_CLOSE_DX, 209.0);
        assert_eq!(COMPOSE_RECIPIENT_POS, (70.0, 35.0));
        assert_eq!(COMPOSE_RECIPIENT_SIZE, (150.0, 15.0));
        assert_eq!(COMPOSE_BODY_SIZE, (202.0, 165.0));
        assert_eq!(PARCEL_VALUE_SIZE, (143.0, 15.0));
        // 子控件不越出面板（面板自带 `Overflow::clip`）
        assert!(PARCEL_SEND_POS.1 + 25.0 <= PARCEL_SIZE.1);
        assert!(PARCEL_CELL_Y + PARCEL_CELL_SIZE.1 <= PARCEL_SIZE.1);
        assert!(LETTER_SEND_POS.1 + 25.0 <= LETTER_SIZE.1);
    }

    /// #3103：`S.MailSendRequest` → 问收件人姓名（`MirRecipient`，parcel=true）
    #[test]
    fn mail_send_request_opens_recipient_prompt_for_parcel() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(Messages::<crate::network::server_event::ServerEvent>::default());
        world.insert_resource(MailState::default());
        world.insert_resource(crate::game::dialogs::input_box::InputBoxState::default());
        world
            .resource_mut::<Messages<crate::network::server_event::ServerEvent>>()
            .write(crate::network::server_event::ServerEvent::MailSendRequest);

        world
            .run_system_once(mail_server_events)
            .expect("邮件事件系统应成功");

        let box_state = world.resource::<crate::game::dialogs::input_box::InputBoxState>();
        assert!(box_state.open, "应弹出 MirInputBox 问收件人姓名");
        assert_eq!(
            box_state.purpose,
            crate::game::dialogs::input_box::InputPurpose::MailRecipient { parcel: true },
            "邮局路径 → 待寄包裹窗"
        );
    }

    /// #3103：`ComposeMail{parcel:true}`（输入框 OK 后的待寄路径）→
    /// 开背包 + 进待寄窗 + 清正文/贴票/赠金（C# `GameScene.cs:6478-6490`）
    #[test]
    fn parcel_compose_request_opens_inventory_and_parcel_window() {
        use crate::game::dialogs::text_input::TextInputState;
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(Messages::<ComposeMail>::default());
        world.insert_resource(MailState::default());
        world.insert_resource(DialogManager::default());
        world.insert_resource(TextInputState::default());
        world.insert_resource(crate::network::NetConnection::default());
        world
            .resource_mut::<Messages<ComposeMail>>()
            .write(ComposeMail {
                to: "小明".to_string(),
                message: None,
                parcel: true,
            });

        world
            .run_system_once(mail_compose_request_system)
            .expect("包裹写信请求应成功");

        let mail = world.resource::<MailState>();
        assert!(mail.compose && mail.compose_parcel, "应进入待寄包裹窗");
        assert!(!mail.stamped, "C# UpdateParcel：开窗未贴票");
        assert_eq!(mail.compose_gold, 0);
        assert!(mail.attach.iter().all(|s| s.is_none()));
        let mgr = world.resource::<DialogManager>();
        assert!(
            mgr.is_open(DialogKind::Inventory),
            "C# 会 InventoryDialog.Show()"
        );
        assert!(
            !mgr.is_open(DialogKind::Mail),
            "C# 待寄路径**不**打开邮件列表窗"
        );
        let input = world.resource::<TextInputState>();
        assert_eq!(input.texts[INPUT_RECIPIENT], "小明");
        assert_eq!(input.texts[INPUT_PARCEL_BODY], "", "C# MessageTextBox 清空");
    }

    /// #3103：收件人为空 → C# `if (string.IsNullOrEmpty(recipientName)) return;` 不开窗
    #[test]
    fn empty_recipient_request_is_ignored() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(Messages::<ComposeMail>::default());
        world.insert_resource(MailState::default());
        world.insert_resource(DialogManager::default());
        world.insert_resource(crate::game::dialogs::text_input::TextInputState::default());
        world.insert_resource(crate::network::NetConnection::default());
        world
            .resource_mut::<Messages<ComposeMail>>()
            .write(ComposeMail {
                to: String::new(),
                message: None,
                parcel: false,
            });
        world
            .run_system_once(mail_compose_request_system)
            .expect("compose request 应成功");
        assert!(!world.resource::<MailState>().compose, "空收件人不得开窗");
        assert!(!world.resource::<DialogManager>().is_open(DialogKind::Mail));
    }

    /// #3103：写邮件按钮 → `MirInputBox`（`EnterMailToName`），不是直接开写信窗
    #[test]
    fn write_button_asks_recipient_name_first() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(MailState::default());
        world.insert_resource(Messages::<ComposeMail>::default());
        world.insert_resource(crate::game::dialogs::input_box::InputBoxState::default());
        world.spawn((MailWrite, Interaction::Pressed));
        world
            .run_system_once(mail_reply_system)
            .expect("写信按钮系统应成功");
        let box_state = world.resource::<crate::game::dialogs::input_box::InputBoxState>();
        assert!(box_state.open, "C# MailDialogs.cs:163-172 先弹 MirInputBox");
        assert_eq!(
            box_state.purpose,
            crate::game::dialogs::input_box::InputPurpose::MailRecipient { parcel: false }
        );
        assert!(
            !world.resource::<MailState>().compose,
            "问名字阶段不得先开写信窗"
        );
    }

    // ---------------------------------------------------------------------
    // #3103 读侧：读邮件两窗
    // ---------------------------------------------------------------------

    /// 读信窗几何/面板帧逐项对 C#（`MailDialogs.cs:979-1272`）。
    ///
    /// 这是「常量对字面量」的基础断言；**实现级**结构判据在
    /// `interact_gate::mail_panels_have_native_background_sprites`（真底图 + 纹理尺寸）。
    #[test]
    fn read_windows_match_csharp_geometry() {
        assert_eq!(READ_LETTER_PANEL, (LibraryName::Title, 672));
        assert_eq!(READ_PARCEL_PANEL, (LibraryName::Title, 675));
        // C# `MailReadLetterDialog.Size = (236,300)`；`MailReadParcelDialog` 声明同为 236x300
        // 但控件排到 y=350 → 按美术 236x384（资产实测 `Title[675] = 236x384`）
        assert_eq!(READ_SIZE, (236.0, 300.0));
        assert_eq!(READ_PARCEL_SIZE, (236.0, 384.0));
        assert_eq!(READ_POS, (100.0, 100.0));
        assert_eq!(READ_SENDER_POS, (70.0, 35.0));
        assert_eq!(READ_DATE_POS, (70.0, 56.0));
        assert_eq!(READ_LABEL_SIZE, (150.0, 15.0));
        assert_eq!(READ_LETTER_BODY_POS, (15.0, 92.0));
        assert_eq!(READ_PARCEL_BODY_POS, (15.0, 98.0));
        assert_eq!(READ_BODY_SIZE, (202.0, 165.0));
        assert_eq!(READ_DELETE_POS, (12.0, 265.0));
        assert_eq!(READ_LOCK_POS, (81.0, 265.0));
        assert_eq!(READ_LETTER_CANCEL_POS, (154.0, 265.0));
        assert_eq!(READ_GOLD_POS, (63.0, 290.0));
        assert_eq!(READ_CELL_X0 + 4.0 * READ_CELL_STEP, 171.0);
        assert_eq!(READ_CELL_SIZE, (35.0, 31.0));
        assert_eq!(READ_COLLECT_POS, (30.0, 350.0));
        assert_eq!(READ_PARCEL_CANCEL_POS, (135.0, 350.0));
        assert_eq!(READ_DELETE_FRAMES, (540, 541, 542));
        assert_eq!(READ_LOCK_FRAMES, (686, 687, 688));
        assert_eq!(READ_COLLECT_OK_FRAMES, (680, 681, 682));
        assert_eq!(READ_COLLECT_DISABLED_FRAMES, (683, 684, 685));
    }

    /// 选窗规则 = C# `Gold > 0 || Items.Count > 0` → 包裹窗，否则书信窗
    /// （`MailDialogs.cs:213-220` 阅读钮 与 `:334-341` 行点击，两处同式）。
    ///
    /// 阳性对照：把判据改成只看 `gold > 0` → 第二条断言红。
    #[test]
    fn read_window_choice_matches_csharp() {
        let plain = MailDetail::default();
        assert_eq!(read_window_for(&plain), ReadWin::Letter);

        let with_item = MailDetail {
            items: vec![MailAttachment {
                name: "金创药(小)".into(),
                image: 1,
                count: 1,
                dura_ratio: None,
            }],
            ..Default::default()
        };
        assert_eq!(read_window_for(&with_item), ReadWin::Parcel);

        let with_gold = MailDetail {
            gold: 100,
            ..Default::default()
        };
        assert_eq!(read_window_for(&with_gold), ReadWin::Parcel);
    }

    /// 收取钮两套帧：`Collected` 才有 680/681/682，否则 683/684/685 且禁用
    /// （`MailDialogs.cs:1243-1256`）。
    #[test]
    fn collect_button_frames_follow_collected_flag() {
        assert_eq!(collect_frames(true), READ_COLLECT_OK_FRAMES);
        assert_eq!(collect_frames(false), READ_COLLECT_DISABLED_FRAMES);
        assert_ne!(READ_COLLECT_OK_FRAMES, READ_COLLECT_DISABLED_FRAMES);
    }

    /// 发信时间文本 = C# `Mail.DateSent.ToString("dd/MM/yy H:mm:ss")`。
    ///
    /// 期望串由**日期分量**拼出（不复用 `format` 串），故「日/月写反」「小时补零」
    /// 这类格式回归会红；`send_date <= 0`（时间戳缺失）必须回空串，不编假日期。
    #[test]
    fn send_date_matches_csharp_format() {
        use chrono::{Datelike, Timelike};

        let ts = 1_709_600_000; // UTC 2024-03-05 12:53:20
        let dt = chrono::DateTime::from_timestamp(ts, 0)
            .unwrap()
            .with_timezone(&chrono::Local);
        let expect = format!(
            "{:02}/{:02}/{:02} {}:{:02}:{:02}",
            dt.day(),
            dt.month(),
            dt.year() % 100,
            dt.hour(),
            dt.minute(),
            dt.second()
        );
        assert_eq!(format_send_date(ts), expect);
        assert_eq!(format_send_date(0), "");
        assert_eq!(format_send_date(-1), "");
        // 日/月不同（2024-03-05 的 3 与 5）——写反会被上面抓；这里再钉一次顺序特征
        assert!(format_send_date(ts).starts_with("05/03/24 "));
    }

    /// 赠金文本 = C# `Gold.ToString("###,###,##0")`：千分位，**0 输出 "0"**
    /// （.NET 实测 `Format("{0:###,###,##0}", 0) == "0"`；末位占位符是 `0` 不是 `#`）。
    #[test]
    fn gold_label_matches_csharp_thousands() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(7), "7");
        assert_eq!(format_gold(999), "999");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(1234567), "1,234,567");
        assert_eq!(format_gold(1_000_000_000), "1,000,000,000");
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

    /// #3103 单元：**写邮件窗必须与邮件列表窗分属不同的拖动/置顶组**。
    ///
    /// 机制：`dialog_drag_system` 按 `DialogRoot(kind)` 聚合包围盒后整体平移，
    /// `bump_dialog_z` 也按 kind 一起改 z（见 `dialogs/mod.rs` 的同名函数）。
    /// 所以「写邮件窗挂在 Mail 上」= 拖邮件列表会把写邮件窗一起拖走 ——
    /// owner 2026-09-24 截图里那块"飘在世界中间、压住邮件列表"的面板就是这么来的。
    ///
    /// 阳性对照（落地时实做）：把 `COMPOSE_DRAG_KIND` 改回 `DialogKind::Mail` → 本测试立即红。
    #[test]
    fn compose_windows_use_their_own_drag_group() {
        assert_eq!(
            COMPOSE_DRAG_KIND,
            DialogKind::MailCompose,
            "写邮件窗必须有独立 kind（C# 两张写邮件窗各自 Movable）"
        );
        assert_ne!(
            COMPOSE_DRAG_KIND,
            DialogKind::Mail,
            "写邮件窗不得与邮件列表同 kind，否则拖动按 kind 聚合会把两窗一起拖走"
        );
        // 邮件列表窗本体的 kind 未变（否则邮件窗自身的打开/关闭语义会断）
        assert_eq!(PANEL, (LibraryName::Title, 670));
    }
}
