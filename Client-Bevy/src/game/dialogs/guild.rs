// ============================================================================
// 行会对话框（M27 → 批 47 bevy_ui 迁移）
// 布局参考：C# GuildDialog.cs / macroquad guild_dialog.rs
//   - 背景 Prguse[180]（实测 590x432），标题 Title[15]，位置 (280,80)
//   - 行会名/会长/金币、成员列表（职务+在线）、公告、创建输入框
// 网络：GuildStatus（1 字节 in_guild / 完整信息，同 opcode 双格式）、GuildNoticeChange、GuildMemberChange
// 迁移说明：
//   - 原版 C# GuildDialog 是分页窗口（Member/Buff/Rank/Storage 各一页，590x432）。
//     本移植为单窗垂直堆叠：根容器 590x740 @ (280,80)，背景图以自然尺寸作子图，
//     下方 432..740 为深色延伸区容纳职务/仓库/金币区块（原 sprite 版这些区块
//     溢出面板裸奔，正是"UI 堆屏幕"病灶之一）。
//   - 邀请提示 = 独立覆盖层 Prguse[360]（456x190）@ (284,289)。
// ============================================================================

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::game::dialogs::text_input::{
    TextInputDisplay, TextInputField, TextInputRect, TextInputState,
};
use crate::game::dialogs::{AlwaysVisible, DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_container, spawn_dropdown_ui, spawn_icon_button, spawn_image,
    spawn_label, spawn_panel, spawn_scroll_bar_ui, CloseButton, UiDropDown, UiScrollList,
};

/// #2892 批B：面板精灵（C# `GuildDialog.Index = 180; Library = Libraries.Prguse`；实测 590x432）
pub const PANEL: (LibraryName, usize) = (LibraryName::Prguse, 180);

/// 根容器尺寸（容纳堆叠的成员/职务/仓库/金币区块；背景图保持自然尺寸）
pub const GUILD_X: f32 = 217.0;
/// C# `GuildDialog.Location = Center` → `((1024-590)/2, (768-432)/2)` = (217,168)
pub const GUILD_Y: f32 = 168.0;
pub const GUILD_W: f32 = 590.0;
/// C# 面板 `Prguse[180]` 实测 590x432（旧值为自造 740：背景图 + 下方深色延伸区）
pub const GUILD_H: f32 = 432.0;
/// 背景图 Prguse[180] 自然尺寸
pub const BG_W: f32 = 590.0;
pub const BG_H: f32 = 432.0;

// ============================================================================
// #2892 批B 单元7：按 C# `GuildDialog`（`GuildDialog.cs:120-905`）拆回「6 页签 + 6 页」。
// 页签精灵/坐标逐字取自 `:130-215`；页矩形取自 Notice/Members/Storage/Rank（@(0,60) 352x372）、
// StatusPage（@(355,60) 230x372）、BuffPage（@(360,61) 352x372）。
// ============================================================================

/// 页签（C# `NoticeButton`/`MembersButton`/`StorageButton`/`RankButton`/`StatusButton`/`BuffButton`）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GuildPage {
    #[default]
    Notice,
    Members,
    Storage,
    Rank,
    Status,
    Buff,
}

/// 页签实体
#[derive(Component)]
pub struct GuildTab(pub GuildPage);

/// 页面容器实体（切换页签时显隐）
#[derive(Component)]
pub struct GuildPageRoot(pub GuildPage);

/// 页签定义：`(页, 正常帧, pressed 帧, x, y)`（C# `GuildDialog.cs:138-199`，均 `Libraries.Title`）
pub const GUILD_TABS: [(
    GuildPage,
    (LibraryName, usize),
    (LibraryName, usize),
    f32,
    f32,
); 6] = [
    (
        GuildPage::Notice,
        (LibraryName::Title, 93),
        (LibraryName::Title, 94),
        20.0,
        38.0,
    ),
    (
        GuildPage::Members,
        (LibraryName::Title, 99),
        (LibraryName::Title, 100),
        91.0,
        38.0,
    ),
    (
        GuildPage::Storage,
        (LibraryName::Title, 105),
        (LibraryName::Title, 106),
        162.0,
        38.0,
    ),
    (
        GuildPage::Rank,
        (LibraryName::Title, 101),
        (LibraryName::Title, 101),
        233.0,
        38.0,
    ),
    (
        GuildPage::Status,
        (LibraryName::Title, 103),
        (LibraryName::Title, 103),
        501.0,
        38.0,
    ),
    (
        GuildPage::Buff,
        (LibraryName::Title, 95),
        (LibraryName::Title, 95),
        430.0,
        38.0,
    ),
];
/// 页签精灵尺寸（`Title[93..106]` 实测 72x24，C# 未设 Size → 用 art 尺寸）
pub const TAB_SIZE: (f32, f32) = (72.0, 24.0);
/// 标题 `Title[25]` @(18,9)（实测 49x15）
pub const TITLE_SPRITE: ((LibraryName, usize), f32, f32) = ((LibraryName::Title, 25), 18.0, 9.0);
/// 关闭钮 `Prguse2[360/361/362]` @(565,4) 24x21
pub const CLOSE_SPRITE: (LibraryName, usize, usize, usize) = (LibraryName::Prguse2, 360, 361, 362);
pub const CLOSE_POS: (f32, f32) = (565.0, 4.0);
pub const CLOSE_SIZE: (f32, f32) = (24.0, 21.0);
/// 左侧四页矩形（Notice/Members/Storage/Rank 共用）
pub const PAGE_LEFT: (f32, f32, f32, f32) = (0.0, 60.0, 352.0, 372.0);
/// 右侧 `StatusPage`
pub const PAGE_STATUS: (f32, f32, f32, f32) = (355.0, 60.0, 230.0, 372.0);
/// `BuffPage`
pub const PAGE_BUFF: (f32, f32, f32, f32) = (360.0, 61.0, 352.0, 372.0);
/// 页面底图：`MembersPageBase`=`Prguse[1852]`@(13,1)、`StoragePageBase`=`[1851]`@(30,19)、
/// `StatusPageBase`=`[1850]`@(10,2)、`BuffPage.Index`=`[1853]`@(0,0)
pub const PAGE_BASE: (LibraryName, usize) = (LibraryName::Prguse, 1852);
pub const STORAGE_BASE: (LibraryName, usize, f32, f32) = (LibraryName::Prguse, 1851, 30.0, 19.0);
pub const STATUS_BASE: (LibraryName, usize, f32, f32) = (LibraryName::Prguse, 1850, 10.0, 2.0);
pub const BUFF_BASE: (LibraryName, usize) = (LibraryName::Prguse, 1853);
/// C# `RanksOptionsTexts[i]` 文案（`:859-871` 的顺序：改/招/踢/存/取/盟/告/益）
pub const GUILD_PERM_LABELS: [&str; 8] = ["改", "招", "踢", "存", "取", "盟", "告", "益"];
/// MembersPage 行数（C# `MemberPageRows = 18`）与行几何（`MembersName[i] @ (125, 30 + i*15)`）
pub const MEMBER_ROWS: usize = 18;
pub const MEMBER_ROW_Y0: f32 = 30.0;
pub const MEMBER_ROW_DY: f32 = 15.0;
pub const MEMBER_COL_NAME: f32 = 125.0;
pub const MEMBER_COL_STATUS: f32 = 225.0;
/// 删除钮列（C# `MembersDelete[i] @ (210, 30 + i*15)`，`Prguse[917]` 16x14）
pub const MEMBER_COL_DELETE: f32 = 210.0;
pub const MEMBER_DELETE_SPRITE: (LibraryName, usize) = (LibraryName::Prguse, 917);
/// `GuildLine` 行号分段（成员 1..=18 / 仓库 20..=27 / 仓库页头 28）
pub const MEMBER_LINE_BASE: usize = 1;
pub const STORAGE_LINE_BASE: usize = MEMBER_LINE_BASE + MEMBER_ROWS;
pub const STORAGE_HEADER_LINE: usize = STORAGE_LINE_BASE + 8;

/// 成员行删除钮（C# `MembersDelete[i]`，i = 页内行号 0..18）
#[derive(Component)]
pub struct GuildMemberDelete(pub usize);

/// 成员行职务下拉（C# `MembersRanks[i]` @(24, 30 + i*15) 100x14）
#[derive(Component)]
pub struct GuildMemberRankDrop(pub usize);

/// 成员行状态列（C# `MembersStatus[i]` @(225, 30 + i*15) 100x14：在线 LimeGreen / 离线 White）
#[derive(Component)]
pub struct GuildMemberStatusLine(pub usize);

/// C# `UpdateMembers`：某行能否改职务 —— `CanChangeRank && 该成员职务下标 >= 自己职务下标`
/// （C# 职务 0 最高，故 `>=` 表示「同级或更低」）
pub fn can_change_member_rank(opts: Option<u8>, my_rank_id: u8, member_rank_index: u8) -> bool {
    opts.is_some_and(|o| o & GUILD_OPT_CHANGE_RANK != 0) && member_rank_index >= my_rank_id
}

/// C# `UpdateMembers`：某行能否踢人 —— `CanKick && 该成员职务下标 >= 自己 && 不是自己`
pub fn can_kick_member(
    opts: Option<u8>,
    my_rank_id: u8,
    member_rank_index: u8,
    member_name: &str,
    my_name: Option<&str>,
) -> bool {
    opts.is_some_and(|o| o & GUILD_OPT_KICK != 0)
        && member_rank_index >= my_rank_id
        && Some(member_name) != my_name
}

/// 本地玩家自己的职务下标（C# `GuildDialog.MyRankId` 的等价推导）
pub fn guild_my_rank_index(guild: &GuildState, my_name: Option<&str>) -> Option<u8> {
    let me = my_name?;
    guild
        .members
        .iter()
        .find(|m| m.name == me)
        .map(|m| m.rank_index)
}

/// StoragePage 格阵（C# `StorageGrid = new MirItemCell[8 * 14]`）
pub const STORAGE_COLS: usize = 8;
/// 数据行数（C# 14 行）
pub const STORAGE_ROWS_TOTAL: usize = 14;
/// 可见行窗口（C# `if (y > 7) StorageGrid[idx].Visible = false`）
pub const STORAGE_WINDOW_ROWS: usize = 8;
/// 单元格尺寸/步进（C# `Size = (35,35)`、`Location = (x*35+31+x, y*35+20+y)`）
pub const STORAGE_CELL: f32 = 35.0;
pub const STORAGE_CELL_STEP: f32 = 36.0;
pub const STORAGE_GRID_X: f32 = 31.0;
pub const STORAGE_GRID_Y: f32 = 20.0;
/// 行窗口起点上限（C# `if (StorageIndex >= 6) StorageIndex = 5;` 到 6 为止）
pub const STORAGE_MAX_START: usize = STORAGE_ROWS_TOTAL - STORAGE_WINDOW_ROWS;

/// 格阵单元（`idx = STORAGE_COLS*y + x`，即 C# `StorageGrid[idx]`）
#[derive(Component)]
pub struct GuildStorageCell(pub usize);
/// 单元格物品图标
#[derive(Component)]
pub struct GuildStorageIcon(pub usize);
/// 单元格数量标签
#[derive(Component)]
pub struct GuildStorageCount(pub usize);

/// NoticePage 正文行数/行高（C# `Notice` 文本框 322x330，本端按行渲染 16px）
pub const NOTICE_ROWS: usize = 20;
pub const NOTICE_ROW_DY: f32 = 16.0;
/// 公告正文显示实体（多行框内的文本；翻页时按行高平移，模拟 C# `ScrollToCaret`）
#[derive(Component)]
pub struct GuildNoticeText;

/// C# `NoticeUpButton`/`NoticeDownButton` 的滚动语义：`NoticeScrollIndex` 为首行下标，
/// 上到 0 停、下到 `len-1` 停（等价于对 `0..=len-1` 做 clamp）。
pub fn notice_next_scroll(current: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let last = (len - 1) as i64;
    (current as i64 + delta as i64).clamp(0, last) as usize
}
/// 公告翻页钮（C# `NoticeUpButton`/`NoticeDownButton` @(337,1)/(337,318)）
#[derive(Component)]
pub struct GuildNoticeUp;
#[derive(Component)]
pub struct GuildNoticeDown;

// ============================================================================
// #2892：NoticePage 位置条与滚轮（C# `GuildDialog.NoticePositionBar` + `NoticePanel_MouseWheel`）
//   - `NoticePositionBar` = `Prguse2[206]` @(337,16)、`Movable`（`GuildDialog.cs:301-311`）
//   - 定位 `UpdateNoticeScrollPosition`（`:1343-1354`）：`y = 16 + index*interval`，
//     `interval = 289 / (len - 25)`（**整数除**），y 夹在 `[16, NoticeDownButton.Y - 20 = 298]`
//   - 拖动 `NoticePositionBar_OnMoving`（`:1356-1389`）：`index = floor((y-16)/interval)`，
//     再夹在 `[0, len-25]`
//   - 滚轮 `NoticePanel_MouseWheel`（`:1390-1410`）：已到顶且向上滚 / 已到底且向下滚 → 直接返回，
//     否则 index 减一 / 加一（每个滚轮事件只走一行，与 `e.Delta/120` 的绝对值无关）
// ============================================================================
/// 位置条相对 NoticePage 的 x（C# `Location = new Point(337, 16)`）
pub const NOTICE_BAR_X: f32 = 337.0;
/// 位置条 y 下限（C# `Location = new Point(337, 16)`）
pub const NOTICE_BAR_Y_MIN: f32 = 16.0;
/// y 上限：C# `if (y >= NoticeDownButton.Location.Y - 20) y = NoticeDownButton.Location.Y - 20;`（318-20）
pub const NOTICE_BAR_Y_MAX: f32 = 298.0;
/// 位置条精灵 `Prguse2[206]` 尺寸（与筛选树拖动手柄同图，实测 12x18）
pub const NOTICE_BAR_W: f32 = 12.0;
pub const NOTICE_BAR_H: f32 = 18.0;
/// C# 位置条/滚轮的可见行数：最大首行下标 = `len - 25`
pub const NOTICE_BAR_ROWS: usize = 25;
/// C# interval 基数：`289 / (len - 25)`
pub const NOTICE_BAR_TRAVEL: f32 = 289.0;

/// C# `289 / (len - 25)` 是 **int/int 截断除**（`GuildDialog.cs:1345`/`:1366`；
/// 与 `NoticeDialog` 同款口径）。公告不足一屏（`len <= 25`）时 C# 会除零/负步长，
/// 本端按「不可滚动」处理（位置条隐藏）。
fn notice_bar_interval(len: usize) -> f32 {
    if len <= NOTICE_BAR_ROWS {
        return 0.0;
    }
    (NOTICE_BAR_TRAVEL as i32 / (len - NOTICE_BAR_ROWS) as i32) as f32
}

/// 首行下标 → 位置条 y（C# `UpdateNoticeScrollPosition`）；`None` = 公告不足一屏，位置条隐藏
pub fn notice_bar_y(index: usize, len: usize) -> Option<f32> {
    if len <= NOTICE_BAR_ROWS {
        return None;
    }
    let interval = notice_bar_interval(len);
    Some((NOTICE_BAR_Y_MIN + index as f32 * interval).clamp(NOTICE_BAR_Y_MIN, NOTICE_BAR_Y_MAX))
}

/// 位置条 y → 首行下标（C# `NoticePositionBar_OnMoving`）
pub fn notice_index_from_bar_y(y: f32, len: usize) -> usize {
    let max = len.saturating_sub(NOTICE_BAR_ROWS);
    let interval = notice_bar_interval(len);
    if interval <= 0.0 {
        return 0;
    }
    let y = y.max(NOTICE_BAR_Y_MIN);
    (((y - NOTICE_BAR_Y_MIN) / interval).floor() as i64).clamp(0, max as i64) as usize
}

/// 滚轮滚一行（C# `NoticePanel_MouseWheel`）：`count > 0` = 向上滚
pub fn notice_wheel_scroll(index: usize, count: i32, len: usize) -> usize {
    if count == 0 {
        return index;
    }
    let max = len.saturating_sub(NOTICE_BAR_ROWS);
    if count > 0 {
        index.saturating_sub(1)
    } else if index >= max {
        index
    } else {
        index + 1
    }
}

/// 位置条精灵（C# `NoticePositionBar` = `Prguse2[206]`，`Movable`）
#[derive(Component)]
pub struct GuildNoticeBar;

/// 位置条命中的绝对矩形（面板原点 ox/oy + NoticePage 偏移 (0,60) + 位置条相对坐标）。
/// 与 `guild_member_row_rect` 等同一口径，UI 对齐测试也用它核对落位。
pub fn guild_notice_bar_rect(bar_y: f32, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        ox + PAGE_LEFT.0 + NOTICE_BAR_X,
        oy + PAGE_LEFT.1 + bar_y,
        NOTICE_BAR_W,
        NOTICE_BAR_H,
    )
}

// C# `GuildRankOptions`（`Shared/Enums.cs:1898-1908`）的位值
pub const GUILD_OPT_CHANGE_RANK: u8 = 1;
pub const GUILD_OPT_RECRUIT: u8 = 2;
pub const GUILD_OPT_KICK: u8 = 4;
pub const GUILD_OPT_STORE_ITEM: u8 = 8;
pub const GUILD_OPT_RETRIEVE_ITEM: u8 = 16;
pub const GUILD_OPT_ALTER_ALLIANCE: u8 = 32;
pub const GUILD_OPT_CHANGE_NOTICE: u8 = 64;
pub const GUILD_OPT_ACTIVATE_BUFF: u8 = 128;

/// 本地玩家在自己行会里的权限位（C# `GuildDialog.MyOptions` 的等价推导）：
/// 按名字在成员列表里找到自己 → 用 `rank_index` 查 `rank_defs` 的 options。
/// `None` = 找不到（未入会/成员表未到）→ 调用方按「不隐藏页签」处理。
pub fn guild_my_options(guild: &GuildState, my_name: Option<&str>) -> Option<u8> {
    let me = my_name?;
    let member = guild.members.iter().find(|m| m.name == me)?;
    guild
        .rank_defs
        .get(member.rank_index as usize)
        .map(|(_, options)| *options)
}

/// 页签是否可见（C# `RefreshInterface`/`GuildStatus` 处理 + `BuffButton` 规则）。
/// `opts = None`（拿不到自己的权限）按「不隐藏」处理，避免服务端未同步时功能整块消失。
pub fn guild_tab_visible(page: GuildPage, opts: Option<u8>, has_buff_catalog: bool) -> bool {
    match page {
        GuildPage::Members | GuildPage::Status => true,
        GuildPage::Notice => opts.is_none_or(|o| o & GUILD_OPT_CHANGE_NOTICE != 0),
        GuildPage::Storage => {
            opts.is_none_or(|o| o & (GUILD_OPT_STORE_ITEM | GUILD_OPT_RETRIEVE_ITEM) != 0)
        }
        GuildPage::Rank => opts.is_none_or(|o| o & GUILD_OPT_CHANGE_RANK != 0),
        // C# `RefreshInterface`：`GuildBuffInfos.Count == 0 → BuffButton.Visible = false`
        GuildPage::Buff => has_buff_catalog,
    }
}

/// 显示离线复选框图（C# `MembersShowOfflineButton` = `Prguse[1346]`）
#[derive(Component)]
pub struct GuildShowOfflineCheck;
/// 显示离线选中态图（C# `MembersShowOfflineStatus` = `Prguse[1347]`）
#[derive(Component)]
pub struct GuildShowOfflineStatus;
/// 职务权限位选中态图（C# `RanksOptionsStatus[i]` = `Prguse[1347]`）
#[derive(Component)]
pub struct GuildRankPermCheck(pub u8);
/// Buff 槽行（C# `GuildBuffButton[i].Name`，8 槽 @(4, 27+i*38)）
#[derive(Component)]
pub struct GuildBuffLine(pub usize);
/// Buff 页剩余点数（C# `PointsLeft` @(118,3)）
#[derive(Component)]
pub struct GuildBuffPoints;

/// 行会成员
#[derive(Debug, Clone, Default)]
pub struct GuildMember {
    pub name: String,
    pub rank: u8,
    /// #1395：职务定义索引（C# rank_index）
    pub rank_index: u8,
    pub online: bool,
}

/// 行会仓库物品条目（GuildStorageList，M32）
#[derive(Debug, Clone, Default)]
pub struct StorageItem {
    pub unique_id: u64,
    pub item_index: i32,
    pub name: String,
    pub count: u16,
    /// 物品图标索引（`ItemInfo.image`，C# `StorageGrid[idx]` 画的就是它）
    pub image: i32,
}

/// 行会状态
#[derive(Resource, Default)]
pub struct GuildState {
    pub in_guild: bool,
    pub name: String,
    pub leader: String,
    pub notice: Vec<String>,
    pub members: Vec<GuildMember>,
    pub gold: u32,
    /// 行会仓库物品（100 格，GuildStorageList 写入）
    pub storage_items: Vec<Option<StorageItem>>,
    /// 仓库列表是否已收到（E2E/UI 等待标记）
    pub storage_received: bool,
    /// 仓库翻页（每页 8 格，共 13 页）
    pub storage_page: usize,
    /// 选中的仓库格子（取出用）
    pub selected_storage: Option<usize>,
    /// 物品名缓存（item_index → name，来自 UserInformation 内嵌 ItemInfo）
    pub item_names: HashMap<i32, String>,
    /// 待处理行会邀请（行会名）
    pub invite: Option<String>,
    /// 选中的成员行（踢出用）
    pub selected_member: Option<usize>,
    /// #1348：是否显示离线成员（C# MembersShowOfflinesetting，默认 true）
    pub show_offline: bool,
    /// #1395：职务定义（name, options；服务端 GuildStatus 下发，C# GuildObject.Ranks）
    pub rank_defs: Vec<(String, u8)>,
    /// #2537：Buff 定义目录（S.GuildBuffList 第三段，服务端 ini GuildSettings 全量）
    pub buff_catalog: Vec<mir2_shared::data::client_data::GuildBuffInfo>,
    /// #2537：已激活 Buff id（S.GuildBuffList ActiveBuffs）
    pub active_buffs: Vec<i32>,
    /// #2537：Buff 页显示开关（C# BuffButton/BuffPage 切换）
    pub show_buff_page: bool,
    /// #2892 批B 单元7：当前页（C# `LeftDialog`/`RightDialog` 选中的 `*Page`）
    pub page: GuildPage,
    /// #2892 批B 单元11：公告滚动首行（C# `GuildDialog.NoticeScrollIndex`）
    pub notice_scroll: usize,
    /// #2537：Buff 页滚动起点（C# StartIndex，8 行/页）
    pub buff_start: usize,
}

impl GuildState {
    /// #1348：可见成员下标（show_offline=false 时过滤离线；C# MembersShowOfflineSwitch）
    pub fn visible_member_indices(&self) -> Vec<usize> {
        self.members
            .iter()
            .enumerate()
            .filter(|(_, m)| self.show_offline || m.online)
            .map(|(i, _)| i)
            .collect()
    }

    /// 物品显示名：优先缓存名，回退 #index
    pub fn item_name(&self, index: i32) -> String {
        self.item_names
            .get(&index)
            .cloned()
            .unwrap_or_else(|| format!("#{}", index))
    }

    /// #2537 Buff 是否已激活
    pub fn buff_active(&self, buff_id: i32) -> bool {
        self.active_buffs.contains(&buff_id)
    }
}

/// #2537 Buff 行文本（C# GuildBuffButton：名称 + 等级/点数/费用 + 状态；纯函数便于头测）
pub fn buff_row_text(info: &mir2_shared::data::client_data::GuildBuffInfo, active: bool) -> String {
    format!(
        "{}  Lv{} 点{} 金{}{}",
        info.name,
        info.level_requirement,
        info.points_requirement,
        info.activation_cost,
        if active { " [已激活]" } else { "" }
    )
}

/// #2537 Buff 页数（C# 8 个 GuildBuffButton/页；空目录仍算 1 页，C# Count<8 不翻页）
pub fn buff_page_count(catalog_len: usize) -> usize {
    catalog_len.div_ceil(8).max(1)
}

#[derive(Component)]
pub struct GuildWidget;

/// 创建行会输入框（TextInputState id 0）
#[derive(Component)]
pub struct GuildNameField;

#[derive(Component)]
pub struct GuildCreateBtn;

/// 邀请玩家输入框（TextInput id 1）
#[derive(Component)]
pub struct GuildInviteField;

/// #1362：职务改名下拉（C# RanksSelectBox）
#[derive(Component)]
pub struct GuildRankDrop;
/// #1362：职务改名输入框（TextInput id 4）
#[derive(Component)]
pub struct GuildRankRenameField;
/// #1362：职务改名保存按钮（C# RanksSaveName）
#[derive(Component)]
pub struct GuildRankSaveBtn;
/// #1395 子批2：加职务输入框（TextInput id 7）/ 按钮（C# AddRank）
#[derive(Component)]
pub struct GuildAddRankField;
#[derive(Component)]
pub struct GuildAddRankBtn;
/// #1395 子批2：职务权限位按钮（C# RanksOptionsButtons[8]，bit 0..7）
#[derive(Component)]
pub struct GuildRankPermBtn(u8);
#[derive(Component)]
pub struct GuildRankPermText;
/// #1395 子批2：调职按钮（C# 升职 ChangeType=2，把选中成员调到下拉职务）
#[derive(Component)]
pub struct GuildPromoteBtn;

/// 公告输入框（TextInput id 2）
#[derive(Component)]
pub struct GuildNoticeField;

/// 仓库金币输入框（TextInput id 3）
#[derive(Component)]
pub struct GuildGoldField;

/// #1348：显示离线成员切换（C# MembersShowOfflineButton）
#[derive(Component)]
pub struct GuildShowOfflineBtn;

/// #2537：Buff 页开关（C# BuffButton）
#[derive(Component)]
pub struct GuildBuffToggleBtn;

/// #2537：Buff 页翻页（C# UpButton/DownButton，8 行/页）
#[derive(Component)]
pub struct GuildBuffUp;

#[derive(Component)]
pub struct GuildBuffDown;

#[derive(Component)]
pub struct GuildItemDeposit;

#[derive(Component)]
pub struct GuildItemWithdraw;

#[derive(Component)]
pub struct GuildStorageUp;

#[derive(Component)]
pub struct GuildStorageDown;

/// 仓库页按钮打包（存/取/上/下；全只读查询，控系统参数个数）
#[derive(SystemParam)]
struct GuildStorageBtns<'w, 's> {
    deposit: Query<'w, 's, (Entity, &'static Interaction), With<GuildItemDeposit>>,
    withdraw: Query<'w, 's, (Entity, &'static Interaction), With<GuildItemWithdraw>>,
    up: Query<'w, 's, (Entity, &'static Interaction), With<GuildStorageUp>>,
    down: Query<'w, 's, (Entity, &'static Interaction), With<GuildStorageDown>>,
}

/// 成员列表滚动条（挂在 Members 页容器；C# `MembersPositionBar`）。
/// 2026-09-19 修复：原挂在根面板而滑块在页容器下——父子不匹配，滑块永不跟随/不可拖。
#[derive(Component)]
pub struct GuildMembersScroll;

/// 仓库滚动条（挂在 Storage 页容器；C# `StoragePositionBar`）
#[derive(Component)]
pub struct GuildStorageScroll;

// 邀请提示
#[derive(Component)]
pub struct GuildInviteWidget;

#[derive(Component)]
pub struct GuildInviteText;

#[derive(Component)]
pub struct GuildInviteYes;

#[derive(Component)]
pub struct GuildInviteNo;

#[derive(Component)]
pub struct GuildLine(usize);

/// 行会窗口主按钮（单查询分发，避免多 With<marker> 查询超 SystemParam 上限）
#[derive(Component, Clone, Copy)]
pub enum GuildBtnKind {
    Close,
    Invite,
    Kick,
    Notice,
    GoldDeposit,
    GoldWithdraw,
}
#[derive(Component)]
pub struct GuildBtn(pub GuildBtnKind);

/// #1348：显示离线按钮文本子节点
#[derive(Component)]
pub struct GuildShowOfflineText;

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildState>();
        app.add_systems(OnEnter(AppState::Game), spawn_guild);
        app.add_systems(OnExit(AppState::Game), cleanup_guild);
        app.add_systems(Update, guild_server_events.run_if(in_state(AppState::Game)));
        app.add_systems(
            Update,
            (
                guild_page_system,
                guild_notice_system,
                guild_member_rows_system,
                guild_ui_system,
                guild_buff_system,
                guild_storage_system,
                guild_invite_system,
                guild_show_offline_system,
                guild_rank_rename_system,
                guild_rank_manage_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_guild(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_guild(
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

    // 根面板 = C# `GuildDialog`：`Prguse[180]`（实测 590x432）+ `Location = Center` → (217,168)。
    // 旧实现是自造 590x740（背景图 + 下方深色延伸区）单窗垂直堆叠，本单元按 C# 拆回
    // 「标题 + 6 页签 + 6 页」，内容按归属搬进各页容器（页面底图用 C# 的 1850/1851/1852/1853）。
    let Some(bg) = load_lib_image(&mut libs, &mut images, PANEL.0, PANEL.1) else {
        return;
    };
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(GUILD_X),
                top: Val::Px(GUILD_Y),
                width: Val::Px(GUILD_W),
                height: Val::Px(GUILD_H),
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode::new(bg),
            DialogRoot(DialogKind::Guild),
            GuildWidget,
            GlobalZIndex(30),
            Visibility::Hidden,
        ))
        .id();

    // 标题 `Title[25]` @(18,9) + 6 页签 + 关闭钮（C# `GuildDialog.cs:130-215`，全挂面板）
    commands.entity(root).with_children(|p| {
        if let Some(h) =
            load_lib_image(&mut libs, &mut images, TITLE_SPRITE.0 .0, TITLE_SPRITE.0 .1)
        {
            spawn_image(p, h, TITLE_SPRITE.1, TITLE_SPRITE.2, 49.0, 15.0, 1);
        }
        for (page, normal, pressed, x, y) in GUILD_TABS {
            if let (Some(n), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, normal.0, normal.1),
                load_lib_image(&mut libs, &mut images, pressed.0, pressed.1),
            ) {
                spawn_icon_button(p, n.clone(), n, pr, x, y, TAB_SIZE.0, TAB_SIZE.1, 9)
                    .insert(GuildTab(page));
            }
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, CLOSE_SPRITE.0, CLOSE_SPRITE.1),
            load_lib_image(&mut libs, &mut images, CLOSE_SPRITE.0, CLOSE_SPRITE.2),
            load_lib_image(&mut libs, &mut images, CLOSE_SPRITE.0, CLOSE_SPRITE.3),
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
                9,
            )
            .insert((GuildBtn(GuildBtnKind::Close), CloseButton));
        }
    });

    // 六个页面容器（C# 页矩形；非当前页由 `guild_page_system` 置 Hidden）
    let mut page_entities: Vec<(GuildPage, Entity)> = Vec::new();
    for (page, rect) in [
        (GuildPage::Notice, PAGE_LEFT),
        (GuildPage::Members, PAGE_LEFT),
        (GuildPage::Storage, PAGE_LEFT),
        (GuildPage::Rank, PAGE_LEFT),
        (GuildPage::Status, PAGE_STATUS),
        (GuildPage::Buff, PAGE_BUFF),
    ] {
        let e = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(rect.0),
                    top: Val::Px(rect.1),
                    width: Val::Px(rect.2),
                    height: Val::Px(rect.3),
                    ..default()
                },
                ChildOf(root),
                GuildPageRoot(page),
                Visibility::Hidden,
                ZIndex(8),
            ))
            .id();
        page_entities.push((page, e));
    }
    let page_of = |want: GuildPage| -> Entity {
        page_entities
            .iter()
            .find(|(p, _)| *p == want)
            .map(|(_, e)| *e)
            .expect("页面容器应已创建")
    };
    let page_notice = page_of(GuildPage::Notice);
    let page_members = page_of(GuildPage::Members);
    let page_storage = page_of(GuildPage::Storage);
    let page_rank = page_of(GuildPage::Rank);
    let page_status = page_of(GuildPage::Status);
    let page_buff = page_of(GuildPage::Buff);

    // #89 成员列表滚动（C# `MembersPage` 内 (125,30) 起；行高见 §7 偏差记录）——
    // 挂在页容器上（滑块同为页容器子节点，父子匹配才跟随/可拖；屏幕原点沿父链累加）
    commands.entity(page_members).insert((
        GuildMembersScroll,
        UiScrollList {
            rect_rel: (
                MEMBER_COL_NAME,
                MEMBER_ROW_Y0,
                200.0,
                MEMBER_ROW_DY * MEMBER_ROWS as f32,
            ),
            row_h: MEMBER_ROW_DY,
            visible: MEMBER_ROWS,
            total: 0,
            offset: 0,
            step: 3,
            track_rel: (337.0, 1.0, 16.0, 331.0),
            thumb: None,
            z: 8,
        },
    ));
    // 仓库滚动（C# `StoragePage.MouseWheel` + `StoragePositionBar` @(337,16)，
    // 行程 16..318 = 302；可见 8 行 / 总 14 行 → 偏移上限 6 = `STORAGE_MAX_START`）
    commands.entity(page_storage).insert((
        GuildStorageScroll,
        UiScrollList {
            rect_rel: (0.0, 0.0, 336.0, 332.0),
            row_h: 36.0,
            visible: STORAGE_WINDOW_ROWS,
            total: STORAGE_ROWS_TOTAL,
            offset: 0,
            step: 1,
            track_rel: (337.0, 16.0, 16.0, 302.0),
            thumb: None,
            z: 8,
        },
    ));

    // 页面底图（C#：Members=`Prguse[1852]`@(13,1)、Storage=`[1851]`@(30,19)、
    // Status=`[1850]`@(10,2)、Buff=页面自身 `[1853]`@(0,0)；NoticePage 无底图）
    for (page, lib, idx, x, y, w, h) in [
        (
            page_members,
            PAGE_BASE.0,
            PAGE_BASE.1,
            13.0,
            1.0,
            324.0,
            332.0,
        ),
        (
            page_storage,
            STORAGE_BASE.0,
            STORAGE_BASE.1,
            STORAGE_BASE.2,
            STORAGE_BASE.3,
            292.0,
            308.0,
        ),
        (
            page_status,
            STATUS_BASE.0,
            STATUS_BASE.1,
            STATUS_BASE.2,
            STATUS_BASE.3,
            208.0,
            316.0,
        ),
        (page_buff, BUFF_BASE.0, BUFF_BASE.1, 0.0, 0.0, 216.0, 332.0),
    ] {
        if let Some(handle) = load_lib_image(&mut libs, &mut images, lib, idx) {
            commands.entity(page).with_children(|p| {
                spawn_image(p, handle, x, y, w, h, 0);
            });
        }
    }

    // ---- NoticePage：公告（C# `GuildDialog.cs:215-316`）
    //   `Notice` 文本框 322x330 @(13,1)（多行、按 `NoticeScrollIndex` 翻行）、
    //   `NoticeEditButton Prguse[560/561/562]` / `NoticeSaveButton Prguse[554/555/556]` @(20,342)（二选一显示）、
    //   上 `Prguse2[197/198/199]` @(337,1)、下 `Prguse2[207/208/209]` @(337,318)、位置条 `Prguse2[206]` @(337,16)。
    //   本端：公告正文按行只读显示（服务端 `GuildNotice` 给的就是行数组）+ 单行编辑框（Bevy 扩展）发 `EditGuildNotice`。
    commands.entity(page_notice).with_children(|p| {
        // 公告正文 = **多行可编辑框**（C# `Notice` 322x330 @(13,1)，`MirTextBox.MultiLine()`）：
        // 容器裁剪 + 显示实体定宽折行；翻页钮按 `NoticeScrollIndex` 平移显示实体（模拟
        // C# `UpdateNotice` 的 `ScrollToCaret()` 逐行滚动）。
        p.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(13.0),
                top: Val::Px(1.0),
                width: Val::Px(322.0),
                height: Val::Px(330.0),
                // 翻页平移显示实体时，超出框外的部分裁掉（等价 C# 文本框的可见区）
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.95)),
            ZIndex(2),
            GuildNoticeField,
            TextInputField(2),
            crate::game::dialogs::text_input::TextInputMultiline,
            TextInputRect(GUILD_X + 13.0, GUILD_Y + 61.0, 322.0, 330.0),
            Visibility::Hidden,
        ))
        .with_children(|ic| {
            ic.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(3.0),
                    top: Val::Px(2.0),
                    // 定宽 → 按容器宽度折行（多行框）
                    width: Val::Px(316.0),
                    ..default()
                },
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                ZIndex(9),
                TextInputDisplay(2),
                GuildNoticeText,
            ));
        });
        // 翻页（C# `NoticeUpButton`/`NoticeDownButton` + `NoticePositionBar`）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            spawn_icon_button(p, n, h, pr, 337.0, 1.0, 16.0, 14.0, 9).insert(GuildNoticeUp);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, 337.0, 318.0, 16.0, 14.0, 9).insert(GuildNoticeDown);
        }
        // 位置条 `Prguse2[206]` @(337,16)（C# `NoticePositionBar`，`Movable` 可拖动；
        // 显隐由 `guild_notice_system` 按「Notice 页激活 + 公告超一屏」控制）
        if let Some(bar) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206) {
            spawn_image(
                p,
                bar,
                NOTICE_BAR_X,
                NOTICE_BAR_Y_MIN,
                NOTICE_BAR_W,
                NOTICE_BAR_H,
                9,
            )
            .insert((GuildNoticeBar, Interaction::default(), Visibility::Hidden));
        }
        // C# `NoticeSaveButton` = `Prguse[554/555/556]` @(20,342)（`NoticeEditButton` 560..562 同位置，二选一显示）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 554),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 555),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 556),
        ) {
            spawn_icon_button(p, n, h, pr, 20.0, 342.0, 28.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::Notice));
        }
    });

    // ---- MembersPage：成员列表 + 滚动条 + 显示离线（C# `GuildDialog.cs:318-487`）----
    commands.entity(page_members).with_children(|p| {
        spawn_scroll_bar_ui(p, (337.0, 1.0, 16.0, 331.0), 8);
        // C# `MemberPageRows = 18`，`MembersName[i] @ (125, 30 + i*15)`（7F 字体 → 11px）
        for i in 0..MEMBER_ROWS {
            spawn_label(
                p,
                &cjk,
                "",
                MEMBER_COL_NAME,
                MEMBER_ROW_Y0 + i as f32 * MEMBER_ROW_DY,
                11.0,
                Color::WHITE,
                8,
            )
            .insert(GuildLine(MEMBER_LINE_BASE + i));
        }
        // C# `MembersDelete[i] = Prguse[917] @(210, 30 + i*15)`
        if let Some(h) = load_lib_image(
            &mut libs,
            &mut images,
            MEMBER_DELETE_SPRITE.0,
            MEMBER_DELETE_SPRITE.1,
        ) {
            for i in 0..MEMBER_ROWS {
                spawn_icon_button(
                    p,
                    h.clone(),
                    h.clone(),
                    h.clone(),
                    MEMBER_COL_DELETE,
                    MEMBER_ROW_Y0 + i as f32 * MEMBER_ROW_DY,
                    16.0,
                    14.0,
                    9,
                )
                .insert(GuildMemberDelete(i));
            }
        }
        // C# `MembersStatus[i]` @(225, 30 + i*15) 100x14（在线 LimeGreen / 离线 White）
        for i in 0..MEMBER_ROWS {
            spawn_label(
                p,
                &cjk,
                "",
                MEMBER_COL_STATUS,
                MEMBER_ROW_Y0 + i as f32 * MEMBER_ROW_DY,
                11.0,
                Color::WHITE,
                8,
            )
            .insert(GuildMemberStatusLine(i));
        }
        // C# `MembersRanks[i]` = `MirDropDownBox` @(24, 30 + i*15) 100x14
        // （`Enabled = CanChangeRank && 成员职务下标 >= 自己`；`SelectedIndex` = 该成员职务）
        for i in 0..MEMBER_ROWS {
            spawn_dropdown_ui(
                p,
                &font,
                Vec::new(),
                None,
                (GUILD_X, GUILD_Y + PAGE_LEFT.1),
                24.0,
                MEMBER_ROW_Y0 + i as f32 * MEMBER_ROW_DY,
                100.0,
                14.0,
                3,
                8,
            )
            .insert((GuildMemberRankDrop(i), Visibility::Hidden));
        }
        // C# `MembersShowOfflineButton` `Prguse[1346]` + `MembersShowOfflineStatus` `Prguse[1347]`
        // @(230,310)，标签 `MembersShowOffline` @(245,309)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1346) {
            spawn_image(p, h, 230.0, 310.0, 12.0, 12.0, 8).insert(GuildShowOfflineCheck);
        }
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1347) {
            spawn_image(p, h, 230.0, 310.0, 16.0, 12.0, 9)
                .insert((GuildShowOfflineStatus, Visibility::Hidden));
        }
        spawn_container(p, 228.0, 308.0, 120.0, 16.0, 9)
            .insert((Button, GuildShowOfflineBtn))
            .with_children(|b| {
                spawn_label(b, &cjk, "显示离线", 17.0, 1.0, 11.0, Color::WHITE, 1)
                    .insert(GuildShowOfflineText);
            });
    });

    // ---- StatusPage：行会名/等级/成员 + 招募/创建（C# `GuildDialog.cs:489-611`）----
    commands.entity(page_status).with_children(|p| {
        // C# `StatusHeaders` @(7,47) 75x300（行头列表）
        spawn_label(
            p,
            &cjk,
            "行会\n等级\n成员",
            7.0,
            47.0,
            11.0,
            Color::WHITE,
            8,
        );
        // C# `StatusGuildName` @(82,47)（行会名 + 会长 + 金币，由 `guild_ui_system` 填充）
        spawn_label(p, &cjk, "", 82.0, 47.0, 11.0, Color::srgb(1.0, 0.9, 0.5), 8)
            .insert(GuildLine(0));
        // C# `StatusLevel` @(82,73) / `StatusMembers` @(82,99)：原版**从未被赋值**（死控件），同坐标建空标签
        spawn_label(p, &cjk, "", 82.0, 73.0, 11.0, Color::WHITE, 8);
        spawn_label(p, &cjk, "", 82.0, 99.0, 11.0, Color::WHITE, 8);
        // 招募行（C# `RecruitMemberLabel` @(36,283)、`MembersRecruitName` @(40,300) 130x21、
        // `RecruitMemberButton` = `Title[356/357/358]` @(170,298) 24x24）
        spawn_label(p, &cjk, "招募成员", 36.0, 283.0, 11.0, Color::WHITE, 8);
        spawn_container(p, 40.0, 300.0, 130.0, 21.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildInviteField,
                TextInputField(1),
                TextInputRect(GUILD_X + 355.0 + 40.0, GUILD_Y + 60.0 + 300.0, 130.0, 21.0),
                Visibility::Hidden,
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
                    ZIndex(9),
                    TextInputDisplay(1),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 356),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 357),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 358),
        ) {
            spawn_icon_button(p, n, h, pr, 170.0, 298.0, 24.0, 24.0, 9)
                .insert(GuildBtn(GuildBtnKind::Invite));
        }
        // ↓ 以下两行是 **Bevy 扩展**（C# `GuildDialog` 无「创建行会」「踢出」控件，见 §7）
        spawn_container(p, 36.0, 330.0, 130.0, 21.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildNameField,
                TextInputField(0),
                TextInputRect(GUILD_X + 355.0 + 36.0, GUILD_Y + 60.0 + 330.0, 130.0, 21.0),
                Visibility::Hidden,
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
                    ZIndex(9),
                    TextInputDisplay(0),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(
                p,
                n.clone(),
                h.clone(),
                pr.clone(),
                170.0,
                328.0,
                76.0,
                25.0,
                9,
            )
            .insert(GuildCreateBtn);
            spawn_icon_button(p, n, h, pr, 106.0, 328.0, 60.0, 25.0, 9)
                .insert(GuildBtn(GuildBtnKind::Kick));
        }
    });

    // ---- StoragePage：金币 + 物品格 + 翻页（C# `GuildDialog.cs:617-750`）----
    commands.entity(page_storage).with_children(|p| {
        // C# `StorageGoldText` @(194,312) 125x12（本端由 `GuildLine(STORAGE_HEADER_LINE)` 填充）
        spawn_label(p, &cjk, "", 194.0, 312.0, 11.0, Color::WHITE, 8)
            .insert(GuildLine(STORAGE_HEADER_LINE));
        // C# `StorageGoldAdd` `Prguse[918]` @(158,313) / `StorageGoldRemove` `Prguse[917]` @(142,313)
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 918) {
            spawn_icon_button(p, h.clone(), h.clone(), h, 158.0, 313.0, 16.0, 14.0, 9)
                .insert(GuildBtn(GuildBtnKind::GoldDeposit));
        }
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 917) {
            spawn_icon_button(p, h.clone(), h.clone(), h, 142.0, 313.0, 16.0, 14.0, 9)
                .insert(GuildBtn(GuildBtnKind::GoldWithdraw));
        }
        // 金币输入（**Bevy 扩展**：C# 无输入框，金币数由 C# 的 `StorageGoldText` 直接展示）
        spawn_container(p, 60.0, 313.0, 78.0, 14.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildGoldField,
                TextInputField(3),
                TextInputRect(GUILD_X + 60.0, GUILD_Y + 60.0 + 313.0, 78.0, 14.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(3.0),
                        top: Val::Px(1.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(9),
                    TextInputDisplay(3),
                ));
            });
        // C# `StorageGrid = new MirItemCell[8 * 14]`：8 列 × 14 行、`Size 35x35`、
        // `Location = (x*35+31+x, (y-StorageIndex)*35+20+(y-StorageIndex))`，可见窗口 8 行。
        // 本端把可见窗口的 64 格全部实体化（图标 + 数量），行窗口由 `storage_page`(=StorageIndex) 平移。
        let white = images.add(crate::map_renderer::make_image(
            vec![255, 255, 255, 255],
            1,
            1,
        ));
        for r in 0..STORAGE_WINDOW_ROWS {
            for c in 0..STORAGE_COLS {
                let slot = STORAGE_COLS * r + c;
                let x = STORAGE_GRID_X + c as f32 * STORAGE_CELL_STEP;
                let y = STORAGE_GRID_Y + r as f32 * STORAGE_CELL_STEP;
                spawn_container(p, x, y, STORAGE_CELL, STORAGE_CELL, 8)
                    .insert((
                        GuildStorageCell(slot),
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.10)),
                    ))
                    .with_children(|cell| {
                        cell.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(STORAGE_CELL),
                                height: Val::Px(STORAGE_CELL),
                                ..default()
                            },
                            ImageNode::new(white.clone()),
                            GuildStorageIcon(slot),
                            Visibility::Hidden,
                        ));
                        spawn_label(cell, &cjk, "", 1.0, 1.0, 9.0, Color::WHITE, 1)
                            .insert(GuildStorageCount(slot));
                    });
            }
        }
        // 存入/取出（**Bevy 扩展**；C# 靠点击格子搬运）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(
                p,
                n.clone(),
                h.clone(),
                pr.clone(),
                31.0,
                200.0,
                76.0,
                25.0,
                9,
            )
            .insert(GuildItemDeposit);
            spawn_label(p, &cjk, "存入", 51.0, 205.0, 11.0, Color::WHITE, 10);
            spawn_icon_button(p, n, h, pr, 120.0, 200.0, 76.0, 25.0, 9).insert(GuildItemWithdraw);
            spawn_label(p, &cjk, "取出", 140.0, 205.0, 11.0, Color::WHITE, 10);
        }
        // C# `StoragePositionBar` `Prguse2[206]` @(337,16)：共享滚动条（滚轮+拖动+跟随）
        spawn_scroll_bar_ui(p, (337.0, 16.0, 16.0, 302.0), 8);
        // C# 翻页 `Prguse2[197/198/199]` @(337,1)、`[207/208/209]` @(337,318)、`[206]` @(337,16)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            spawn_icon_button(p, n, h, pr, 337.0, 1.0, 16.0, 14.0, 9).insert(GuildStorageUp);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, 337.0, 318.0, 16.0, 14.0, 9).insert(GuildStorageDown);
        }
    });

    // ---- RankPage：职务下拉/改名/权限位（C# `GuildDialog.cs:752-872`）----
    commands.entity(page_rank).with_children(|p| {
        // C# `RanksSelectTextL` @(42,18) / `RanksSelectTextR` @(198,18)
        spawn_label(p, &cjk, "名称", 42.0, 18.0, 11.0, Color::WHITE, 8);
        spawn_label(p, &cjk, "职务", 198.0, 18.0, 11.0, Color::WHITE, 8);
        // C# `RanksName` 文本框 @(42,36) 130x16
        spawn_container(p, 42.0, 36.0, 130.0, 16.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildRankRenameField,
                TextInputField(4),
                TextInputRect(GUILD_X + 42.0, GUILD_Y + 60.0 + 36.0, 130.0, 16.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(3.0),
                        top: Val::Px(1.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(9),
                    TextInputDisplay(4),
                ));
            });
        // C# `RanksSelectBox` @(198,36) 130x16
        spawn_dropdown_ui(
            p,
            &font,
            vec!["会长".to_string(), "副会长".to_string(), "成员".to_string()],
            Some(0),
            (GUILD_X, GUILD_Y + 60.0),
            198.0,
            36.0,
            130.0,
            16.0,
            3,
            8,
        )
        .insert(GuildRankDrop);
        // C# `RanksSaveName` = `Title[90/91/92]` @(155,290) 40x25
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 90),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 91),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 92),
        ) {
            spawn_icon_button(p, n, h, pr, 155.0, 290.0, 40.0, 25.0, 9).insert(GuildRankSaveBtn);
        }
        // C# `RanksOptionsButtons[i]` `Prguse[1346]` + `RanksOptionsStatus[i]` `Prguse[1347]`，
        // 位置 `(i%2==0 ? 42 : 202, 120 + i*20 / 120 + (i-1)*20)`，标签 @(+17, -2)
        spawn_label(p, &cjk, "权限位", 42.0, 96.0, 11.0, Color::WHITE, 8);
        for i in 0..8usize {
            let x = if i % 2 == 0 { 42.0 } else { 202.0 };
            let y = if i % 2 == 0 {
                120.0 + i as f32 * 20.0
            } else {
                120.0 + (i - 1) as f32 * 20.0
            };
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1346) {
                spawn_image(p, h, x, y, 12.0, 12.0, 8);
            }
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 1347) {
                spawn_image(p, h, x, y, 16.0, 12.0, 9)
                    .insert((GuildRankPermCheck(i as u8), Visibility::Hidden));
            }
            spawn_container(p, x - 2.0, y - 2.0, 90.0, 16.0, 9)
                .insert((Button, GuildRankPermBtn(i as u8)))
                .with_children(|b| {
                    spawn_label(
                        b,
                        &cjk,
                        GUILD_PERM_LABELS[i],
                        19.0,
                        2.0,
                        11.0,
                        Color::WHITE,
                        1,
                    );
                });
        }
        // C# `PointsLeft` 同级：权限位汇总（Bevy 扩展）
        spawn_label(
            p,
            &cjk,
            "",
            42.0,
            258.0,
            10.0,
            Color::srgb(0.8, 0.9, 0.6),
            8,
        )
        .insert(GuildRankPermText);
        // **Bevy 扩展**：加职务（C# 职务由服务端定义，无新建入口）
        spawn_container(p, 42.0, 80.0, 130.0, 16.0, 8)
            .insert((
                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                GuildAddRankField,
                TextInputField(7),
                TextInputRect(GUILD_X + 42.0, GUILD_Y + 60.0 + 80.0, 130.0, 16.0),
                Visibility::Hidden,
            ))
            .with_children(|ic| {
                ic.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(3.0),
                        top: Val::Px(1.0),
                        ..default()
                    },
                    Text::new(String::new()),
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ZIndex(9),
                    TextInputDisplay(7),
                ));
            });
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(
                p,
                n.clone(),
                h.clone(),
                pr.clone(),
                200.0,
                290.0,
                60.0,
                25.0,
                9,
            )
            .insert(GuildPromoteBtn);
            spawn_label(p, &cjk, "调职", 219.0, 295.0, 11.0, Color::WHITE, 10);
            spawn_icon_button(p, n, h, pr, 200.0, 320.0, 60.0, 25.0, 9).insert(GuildAddRankBtn);
            spawn_label(p, &cjk, "加职务", 219.0, 325.0, 11.0, Color::WHITE, 10);
        }
    });

    // ---- BuffPage：C# `BuffPage` @(360,61) + `PointsLeft` + 8 槽 ----
    commands.entity(page_buff).with_children(|p| {
        spawn_label(p, &cjk, "", 118.0, 3.0, 11.0, Color::WHITE, 8).insert(GuildBuffPoints);
        for i in 0..8usize {
            spawn_label(
                p,
                &cjk,
                "",
                4.0,
                27.0 + i as f32 * 38.0,
                11.0,
                Color::WHITE,
                8,
            )
            .insert(GuildBuffLine(i));
        }
        // C# `UpButton`/`DownButton` @(337,1)/(337,318)
        spawn_container(p, 337.0, 1.0, 16.0, 14.0, 8)
            .insert((Button, GuildBuffUp))
            .with_children(|b| {
                spawn_label(b, &font, "▲", 0.0, 0.0, 11.0, Color::WHITE, 1);
            });
        spawn_container(p, 337.0, 318.0, 16.0, 14.0, 8)
            .insert((Button, GuildBuffDown))
            .with_children(|b| {
                spawn_label(b, &font, "▼", 0.0, 0.0, 11.0, Color::WHITE, 1);
            });
    });

    // 邀请提示（MirMessageBox，独立覆盖层 `Prguse[360]` 456x190 @ (284,289)）
    let (bx, by) = (284.0, 289.0);
    if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let popup = spawn_panel(&mut commands, h, bx, by, 456.0, 190.0, 45);
        commands.entity(popup).insert((
            DialogRoot(DialogKind::Guild),
            // 独立弹窗不随 Guild 开关门控；挂 DialogRoot 仅为 OnExit 时随行会窗口一起清理
            AlwaysVisible,
            GuildInviteWidget,
            Visibility::Hidden,
        ));
        commands.entity(popup).with_children(|p| {
            spawn_label(p, &cjk, "", 35.0, 40.0, 12.0, Color::WHITE, 9).insert(GuildInviteText);
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 240.0, 150.0, 76.0, 25.0, 10).insert(GuildInviteYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 340.0, 150.0, 76.0, 25.0, 10).insert(GuildInviteNo);
            }
        });
    }
}

/// #2892 批B 单元7：页签切换（C# `GuildDialog.LeftDialog(0..3)` / `RightDialog(0..1)`）。
/// #2892 批B 单元12：MembersPage 行内职务下拉 / 状态列 / 删除钮可见性
/// （C# `GuildDialog.UpdateMembers`，`:1596-1640`）：
/// 超出成员数的行整体隐藏；`MembersRanks[i].Enabled = CanChangeRank && 成员职务下标 >= MyRankId`；
/// `MembersDelete[i].Visible = CanKick && 职务下标 >= MyRankId && 不是自己`；
/// `MembersStatus[i]` 在线 `LimeGreen` / 离线 `White`；下拉改选 → `EditGuildMember{change_type=2}`。
fn guild_member_rows_system(
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    mgr: Res<DialogManager>,
    local_name: Query<&crate::actor::PlayerName, With<crate::actor::LocalPlayer>>,
    scroll: Query<&UiScrollList, With<GuildMembersScroll>>,
    mut status: Query<(&GuildMemberStatusLine, &mut Text, &mut TextColor)>,
    mut del: Query<
        (&GuildMemberDelete, &mut Visibility),
        (Without<GuildMemberStatusLine>, Without<GuildMemberRankDrop>),
    >,
    mut drops: Query<
        (&GuildMemberRankDrop, &mut UiDropDown, &mut Visibility),
        (Without<GuildMemberStatusLine>, Without<GuildMemberDelete>),
    >,
    mut last_sent: Local<HashMap<usize, usize>>,
) {
    // 只在「行会窗打开 + 停在成员页」时同步行内容（其它页隐藏时不必逐帧写）
    if !mgr.is_open(DialogKind::Guild) || guild.page != GuildPage::Members {
        return;
    }
    let me = local_name.iter().next().map(|n| n.0.clone());
    let opts = guild_my_options(&guild, me.as_deref());
    let my_rank = guild_my_rank_index(&guild, me.as_deref());
    let scroll_offset = scroll.iter().next().map(|s| s.offset).unwrap_or(0);
    let visible = guild.visible_member_indices();
    let rank_names: Vec<String> = guild.rank_defs.iter().map(|(n, _)| n.clone()).collect();

    for (row, mut text, mut color) in &mut status {
        let idx = scroll_offset + row.0;
        let member = visible.get(idx).and_then(|&mi| guild.members.get(mi));
        match member {
            Some(m) => {
                let want = if m.online { "在线" } else { "离线" }.to_string();
                if text.0 != want {
                    text.0 = want;
                }
                // C#：在线 `Color.LimeGreen`，离线 `Color.White`
                let want_color = if m.online {
                    Color::srgb(0.196, 0.804, 0.196)
                } else {
                    Color::WHITE
                };
                if color.0 != want_color {
                    color.0 = want_color;
                }
            }
            None => {
                if !text.0.is_empty() {
                    text.0.clear();
                }
            }
        }
    }

    for (btn, mut vis) in &mut del {
        let idx = scroll_offset + btn.0;
        let member = visible.get(idx).and_then(|&mi| guild.members.get(mi));
        let want = member.is_some_and(|m| {
            can_kick_member(
                opts,
                my_rank.unwrap_or(u8::MAX),
                m.rank_index,
                &m.name,
                me.as_deref(),
            )
        });
        let want_vis = if want {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want_vis {
            *vis = want_vis;
        }
    }

    for (row, mut dd, mut vis) in &mut drops {
        let idx = scroll_offset + row.0;
        let member = visible.get(idx).and_then(|&mi| guild.members.get(mi));
        let want_vis = if member.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want_vis {
            *vis = want_vis;
        }
        let Some(m) = member else {
            dd.open = false;
            continue;
        };
        if dd.items != rank_names {
            dd.items = rank_names.clone();
        }
        let enabled = can_change_member_rank(opts, my_rank.unwrap_or(u8::MAX), m.rank_index);
        if !enabled {
            // C# `Enabled = false`：本端下拉控件没有 enabled 态 → 强制收起弹层
            dd.open = false;
        }
        let want_sel = m.rank_index as usize;
        match dd.selected {
            Some(sel) if sel != want_sel => {
                // 用户刚改选：发 `C.EditGuildMember{ChangeType = 2}`（C# `OnNewRank`）
                if enabled && last_sent.get(&row.0).copied() != Some(sel) {
                    net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                        change_type: 2,
                        rank_index: sel as u8,
                        name: m.name.clone(),
                        rank_name: rank_names.get(sel).cloned().unwrap_or_default(),
                    });
                    tracing::info!("🏰 调整成员职务: {} -> {}（第 {} 行）", m.name, sel, row.0);
                    last_sent.insert(row.0, sel);
                }
            }
            _ => {
                last_sent.insert(row.0, want_sel);
            }
        }
        if dd.selected != Some(want_sel) {
            dd.selected = Some(want_sel);
        }
    }
}

/// #2892 批B 单元11（+ 位置条/滚轮）：NoticePage 正文与翻页
/// （C# `Notice` + `NoticeUpButton`/`NoticeDownButton` + `NoticePositionBar` + `NoticePanel_MouseWheel`，
/// `NoticeScrollIndex` 语义：首行下标，上下钮按 `0..=len-1`，位置条/滚轮按 `0..=len-25`）。
#[allow(clippy::type_complexity)]
fn guild_notice_system(
    mut guild: ResMut<GuildState>,
    // #2892：公告改为多行可编辑框后，翻页 = 平移显示实体（模拟 C# `ScrollToCaret()` 逐行滚动）
    mut texts: Query<&mut Node, (With<GuildNoticeText>, Without<GuildNoticeBar>)>,
    up: Query<(Entity, &Interaction), With<GuildNoticeUp>>,
    down: Query<(Entity, &Interaction), With<GuildNoticeDown>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    // #2892：位置条（C# `NoticePositionBar` = `Prguse2[206]`，`Movable`）
    mut bar: Query<(&Interaction, &mut Node, &mut Visibility), With<GuildNoticeBar>>,
    panel_origin: Query<
        &Node,
        (
            With<GuildWidget>,
            Without<GuildNoticeBar>,
            Without<GuildNoticeText>,
        ),
    >,
    mut wheels: MessageReader<bevy::input::mouse::MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    probe: Res<crate::control::CursorProbe>,
    mut grab: Local<Option<f32>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = guild.page == GuildPage::Notice;
    let len = guild.notice.len();
    for (e, inter) in &up {
        if edge(e, inter, &mut prev_inter) {
            // C# `NoticeUpButton.Click`：`if (NoticeScrollIndex == 0) return;`
            guild.notice_scroll = notice_next_scroll(guild.notice_scroll, -1, len);
        }
    }
    for (e, inter) in &down {
        if edge(e, inter, &mut prev_inter) {
            // C# `NoticeDownButton.Click`：`if (NoticeScrollIndex == Notice.MultiText.Length - 1) return;`
            guild.notice_scroll = notice_next_scroll(guild.notice_scroll, 1, len);
        }
    }
    // 滚动位置随公告长度收敛（公告变短时不会停在越界行）
    guild.notice_scroll = notice_next_scroll(guild.notice_scroll, 0, len);

    // 面板原点（含窗口拖动偏移）：NoticePage 是面板内 (0,60) 352x372 的子页
    let (ox, oy) = panel_origin
        .single()
        .map(|n| crate::ui::theme::node_origin(n, (GUILD_X, GUILD_Y)))
        .unwrap_or((GUILD_X, GUILD_Y));
    let cursor = crate::control::resolve_cursor(
        probe.pos,
        windows.single().ok().and_then(|w| w.cursor_position()),
    );

    // ---- 滚轮（C# `NoticePanel_MouseWheel`，:1390-1410；仅光标在 NoticePage 内生效）----
    if open {
        let inside = cursor
            .map(|c| {
                let px = ox + PAGE_LEFT.0;
                let py = oy + PAGE_LEFT.1;
                c.x >= px && c.x <= px + PAGE_LEFT.2 && c.y >= py && c.y <= py + PAGE_LEFT.3
            })
            .unwrap_or(false);
        if inside {
            for ev in wheels.read() {
                // C# `count = e.Delta / MouseWheelScrollDelta`：本端按事件符号取 ±1
                // （每个事件只走一行，与 C# 一致）
                let c = match ev.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => ev.y.signum() as i32,
                    bevy::input::mouse::MouseScrollUnit::Pixel => ev.y.signum() as i32,
                };
                if c != 0 {
                    guild.notice_scroll = notice_wheel_scroll(guild.notice_scroll, c, len);
                }
            }
        }
    }

    // ---- 位置条（C# `NoticePositionBar`）：位置随 `notice_scroll`；按住可拖动 ----
    let target = notice_bar_y(guild.notice_scroll, len);
    for (inter, mut node, mut vis) in &mut bar {
        // `NoticePage` 隐藏时位置条必须一起藏（页面用 Visibility::Hidden，显式 Visible 的子节点会漏渲染）
        let want_vis = if open && target.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want_vis {
            *vis = want_vis;
        }
        let Some(y) = target else {
            *grab = None;
            continue;
        };
        let mut dragged = false;
        if open && *inter == Interaction::Pressed {
            if let Some(c) = cursor {
                // 光标 y → 位置条相对页面的 y；抓取点偏移保证拖动不跳变
                let local = c.y - (oy + PAGE_LEFT.1);
                let off = *grab.get_or_insert(local - y);
                let moved = (local - off).clamp(NOTICE_BAR_Y_MIN, NOTICE_BAR_Y_MAX);
                guild.notice_scroll = notice_index_from_bar_y(moved, len);
                node.top = Val::Px(moved);
                dragged = true;
            }
        }
        if !dragged {
            if !mouse.pressed(MouseButton::Left) {
                *grab = None;
            }
            if node.top != Val::Px(y) {
                node.top = Val::Px(y);
            }
        }
    }

    // 显示实体上移 `scroll * 行高`（框有 `Overflow::clip`，超出部分不可见）
    for mut node in &mut texts {
        let want = Val::Px(2.0 - guild.notice_scroll as f32 * NOTICE_ROW_DY);
        if node.top != want {
            node.top = want;
        }
    }
}

/// 非当前页整页 `Visibility::Hidden`（页面是根面板的子实体，关闭窗口时随根一起不渲染）。
fn guild_page_system(
    mut guild: ResMut<GuildState>,
    mut pages: Query<(&GuildPageRoot, &mut Visibility), Without<GuildTab>>,
    // #2892 批B 单元10：页签可见性按玩家行会权限门控（C# `RefreshInterface` 与 `GuildStatus` 处理）
    mut tab_vis: Query<(&GuildTab, &mut Visibility), Without<GuildPageRoot>>,
    local_name: Query<&crate::actor::PlayerName, With<crate::actor::LocalPlayer>>,
    tabs: Query<(Entity, &GuildTab, &Interaction)>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    for (e, tab, inter) in &tabs {
        if edge(e, inter, &mut prev_inter) {
            guild.page = tab.0;
            // C#：`BuffPage` 就是 `BuffButton` 切出来的页，Buff 行的渲染分支沿用 `show_buff_page`
            guild.show_buff_page = tab.0 == GuildPage::Buff;
        }
    }
    let want_page = guild.page;
    for (root, mut vis) in &mut pages {
        let want = if root.0 == want_page {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    // 页签可见性（C# `RefreshInterface`：`CanChangeNotice→NoticeButton`、`CanChangeRank→RankButton`、
    // `CanStoreItem|CanRetrieveItem→StorageButton`、缓存非空→`BuffButton`；Members/Status 恒可见）。
    // 本端 `my_options` 由「本地玩家成员行 → `rank_defs[rank_index].options`」推导
    // （服务端自定义信息体未带 C# `MyOptions`，见 §7）。
    let me = local_name.iter().next().map(|n| n.0.as_str());
    let opts = guild_my_options(&guild, me);
    let mut page_hidden = false;
    for (tab, mut vis) in &mut tab_vis {
        let want = guild_tab_visible(tab.0, opts, !guild.buff_catalog.is_empty());
        let want_vis = if want {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != want_vis {
            *vis = want_vis;
        }
        if !want && tab.0 == want_page {
            page_hidden = true;
        }
    }
    // 当前页被权限关掉时退回 `Members`（C# 是按钮消失后玩家自行切页，本端补一次兜底切页）
    if page_hidden {
        guild.page = GuildPage::Members;
        guild.show_buff_page = false;
        for (root, mut vis) in &mut pages {
            let want = if root.0 == GuildPage::Members {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            if *vis != want {
                *vis = want;
            }
        }
    }
}

/// 显隐 + 渲染 + 打开时请求行会信息 + 创建按钮
#[allow(clippy::too_many_arguments)]
/// guild_ui_system 辅助参数包（删除钮/面板原点/两个 Local；控 Bevy 16 参上限）
#[derive(SystemParam)]
struct GuildUiAux<'w, 's> {
    /// #2892 批B 单元8：C# `MembersDelete[i].Click → DeleteMember(i)`
    del_btns: Query<'w, 's, (Entity, &'static Interaction, &'static GuildMemberDelete)>,
    prev_inter: Local<'s, HashMap<Entity, Interaction>>,
    requested: Local<'s, bool>,
    panel_origin: Query<'w, 's, &'static Node, With<GuildWidget>>,
}

fn guild_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut create_btns: Query<(Entity, &Interaction, &mut Visibility), With<GuildCreateBtn>>,
    btns: Query<(Entity, &Interaction, &GuildBtn)>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<
        &mut Visibility,
        (With<GuildWidget>, Without<GuildCreateBtn>),
    >,
    mut members_scroll: Query<&mut UiScrollList, With<GuildMembersScroll>>,
    mut lines: Query<(&mut Text, &mut TextColor, &GuildLine)>,
    // #2892 批B 单元7：Buff 槽行与剩余点数（C# `BuffPage` 的 `GuildBuffButton[i].Name`/`PointsLeft`）
    mut buff_lines: Query<(&mut Text, &mut TextColor, &GuildBuffLine), Without<GuildLine>>,
    mut buff_points: Query<
        &mut Text,
        (
            With<GuildBuffPoints>,
            Without<GuildLine>,
            Without<GuildBuffLine>,
        ),
    >,
    aux: GuildUiAux,
) {
    let GuildUiAux {
        del_btns,
        mut prev_inter,
        mut requested,
        panel_origin,
    } = aux;
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Guild);
    for mut vis in &mut widgets {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut sl in &mut members_scroll {
        // #89 成员列表行数（滚动夹紧）
        sl.set_total(if guild.in_guild {
            guild.visible_member_indices().len()
        } else {
            0
        });
    }
    // 创建行会按钮：仅对话框打开且未入会时显示（此前完全没管理显隐，一直残留屏幕）；
    // 点击动作在下方"创建按钮 → GuildNameReturn"统一处理
    for (_, _, mut vis) in &mut create_btns {
        *vis = if open && !guild.in_guild {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        *requested = false;
        return;
    }
    // 打开瞬间请求行会信息（原版 C# GuildDialog.Show → RequestGuildInfo）
    if !*requested {
        *requested = true;
        net.send_packet(&mir2_shared::packets::client::guild::RequestGuildInfo { info_type: 0 });
        // #2537：Buff 列表（C.GuildBuffUpdate action=0，C# RequestGuildBuffList；每次打开刷新）
        net.send_packet(&mir2_shared::packets::client::guild::GuildBuffUpdate {
            action: 0,
            buff_id: 0,
        });
        tracing::info!("🏰 请求行会信息 + 行会技能列表");
    }
    // 关闭（bevy_ui Interaction 边沿）
    for (e, inter, k) in &btns {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        match k.0 {
            GuildBtnKind::Close => {
                mgr.close(DialogKind::Guild);
            }
            GuildBtnKind::Invite => {
                // 邀请按钮 → EditGuildMember{0=add member}（C# GuildDialog 邀请）
                let name = input.texts.get(1).cloned().unwrap_or_default();
                let name = name.trim().to_string();
                if !name.is_empty() && guild.in_guild {
                    net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                        change_type: 0,
                        rank_index: 0,
                        name: name.clone(),
                        rank_name: String::new(),
                    });
                    tracing::info!("🏰 邀请玩家加入行会: {}", name);
                    input.texts[1].clear();
                    input.active = None;
                }
            }
            GuildBtnKind::Kick => {
                // 踢出按钮 → EditGuildMember{1=delete member}（对选中的成员）
                if let Some(idx) = guild.selected_member {
                    let visible = guild.visible_member_indices();
                    if let Some(&mi) = visible.get(idx) {
                        if let Some(m) = guild.members.get(mi) {
                            net.send_packet(
                                &mir2_shared::packets::client::guild::EditGuildMember {
                                    change_type: 1,
                                    rank_index: 0,
                                    name: m.name.clone(),
                                    rank_name: String::new(),
                                },
                            );
                            tracing::info!("🏰 踢出行会成员: {}", m.name);
                            guild.selected_member = None;
                        }
                    }
                }
            }
            GuildBtnKind::Notice => {
                // 公告按钮 → EditGuildNotice（C# GuildDialog 公告编辑）
                let notice = input.texts.get(2).cloned().unwrap_or_default();
                let notice = notice.trim().to_string();
                if !notice.is_empty() && guild.in_guild {
                    net.send_packet(&mir2_shared::packets::client::guild::EditGuildNotice {
                        notice_lines: vec![notice.clone()],
                    });
                    tracing::info!("🏰 更新行会公告: {}", notice);
                    input.texts[2].clear();
                    input.active = None;
                }
            }
            GuildBtnKind::GoldDeposit => {
                // 仓库金币：存入（C# GuildDialog 仓库语义：GuildStorageGoldChange）
                if guild.in_guild {
                    let amount = input
                        .texts
                        .get(3)
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(0);
                    if amount > 0 {
                        net.send_packet(
                            &mir2_shared::packets::client::guild::GuildStorageGoldChange {
                                change_type: 0,
                                amount,
                            },
                        );
                        tracing::info!("🏰 存入行会仓库 {} 金币", amount);
                        input.texts[3].clear();
                        input.active = None;
                    }
                }
            }
            GuildBtnKind::GoldWithdraw => {
                if guild.in_guild {
                    let amount = input
                        .texts
                        .get(3)
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(0);
                    if amount > 0 {
                        net.send_packet(
                            &mir2_shared::packets::client::guild::GuildStorageGoldChange {
                                change_type: 1,
                                amount,
                            },
                        );
                        tracing::info!("🏰 取出行会仓库 {} 金币", amount);
                        input.texts[3].clear();
                        input.active = None;
                    }
                }
            }
        }
    }
    // 渲染（#89 成员列表支持滚轮滚动）
    let scroll_offset = members_scroll
        .iter()
        .next()
        .map(|s| s.offset)
        .unwrap_or(0);
    // #1348：可见成员下标（过滤离线）
    let visible = guild.visible_member_indices();
    // #2892 批B 单元7：BuffPage 的 8 个槽 + 剩余点数（C# `GuildBuffButton[i].Name` / `PointsLeft`）
    for (mut text, mut color, bl) in &mut buff_lines {
        let idx = guild.buff_start + bl.0;
        let info = guild.buff_catalog.get(idx);
        let want = match info {
            Some(info) => buff_row_text(info, guild.buff_active(info.id)),
            None => String::new(),
        };
        if text.0 != want {
            text.0 = want;
        }
        let want_color = if info.map(|info| guild.buff_active(info.id)).unwrap_or(false) {
            Color::srgb(0.5, 1.0, 0.5)
        } else {
            Color::WHITE
        };
        if color.0 != want_color {
            color.0 = want_color;
        }
    }
    for mut text in &mut buff_points {
        let want = format!(
            "技能 第{}/{}页（已激活 {}/{}）",
            guild.buff_start / 8 + 1,
            buff_page_count(guild.buff_catalog.len()),
            guild.active_buffs.len(),
            guild.buff_catalog.len()
        );
        if text.0 != want {
            text.0 = want;
        }
    }
    for (mut text, mut color, line) in &mut lines {
        text.0 = match line.0 {
            0 => {
                // C# `StatusGuildName` @(82,47)（行会名 + 会长 + 金币；公告另在 NoticePage）
                // C# 该行只放行会名（金币在 `StorageGoldText`、公告在 `Notice` 文本框）
                if guild.in_guild {
                    guild.name.clone()
                } else {
                    "未加入行会".to_string()
                }
            }
            i if (MEMBER_LINE_BASE..MEMBER_LINE_BASE + MEMBER_ROWS).contains(&i) => {
                // C# `MembersName[i].Text = 成员名`（状态另由 `MembersStatus[i]` 渲染）
                let idx = scroll_offset + i - MEMBER_LINE_BASE;
                // #1348：按 show_offline 过滤后的可见成员映射
                match visible.get(idx).and_then(|&mi| guild.members.get(mi)) {
                    Some(m) => m.name.clone(),
                    None => String::new(),
                }
            }
            // C# `StorageGoldText`（`:634`）：`Gold > 0 ? "{0:###,###,###}" : "0"`
            i if i == STORAGE_HEADER_LINE => {
                if guild.gold > 0 {
                    format!("{}", guild.gold)
                } else {
                    "0".to_string()
                }
            }
            _ => String::new(),
        };
        // #140 成员选中行高亮（踢出目标可见）
        let c = if (MEMBER_LINE_BASE..MEMBER_LINE_BASE + MEMBER_ROWS).contains(&line.0)
            && guild.selected_member == Some(scroll_offset + line.0 - MEMBER_LINE_BASE)
        {
            Color::srgb(1.0, 0.9, 0.3)
        } else {
            Color::WHITE
        };
        if color.0 != c {
            color.0 = c;
        }
    }
    // 创建按钮 → GuildNameReturn（原版 C#：输入行会名 → 创建）
    for (e, inter, _) in &create_btns {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(0).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::GuildNameReturn {
                    name: name.clone(),
                });
                tracing::info!("🏰 创建行会: {}", name);
                input.texts[0].clear();
                input.active = None;
            }
        }
    }
    // 点击成员行选中（踢出目标）；Buff 页模式下由 guild_buff_system 处理点击
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) = panel_origin
                    .single()
                    .map(|n| crate::ui::theme::node_origin(n, (GUILD_X, GUILD_Y)))
                    .unwrap_or((GUILD_X, GUILD_Y));
                if !guild.show_buff_page {
                    let visible = guild.visible_member_indices();
                    // C# `MembersDelete[i].Click → DeleteMember(i)`：直接对**该行**的成员发踢出
                    for (e, inter, del) in &del_btns {
                        if edge(e, inter, &mut prev_inter) {
                            let idx = scroll_offset + del.0;
                            if let Some(&mi) = visible.get(idx) {
                                if let Some(m) = guild.members.get(mi) {
                                    net.send_packet(
                                        &mir2_shared::packets::client::guild::EditGuildMember {
                                            change_type: 1,
                                            rank_index: 0,
                                            name: m.name.clone(),
                                            rank_name: String::new(),
                                        },
                                    );
                                    tracing::info!("🏰 踢出行会成员（行内删除钮）: {}", m.name);
                                    guild.selected_member = None;
                                }
                            }
                            return;
                        }
                    }
                    for i in MEMBER_LINE_BASE..MEMBER_LINE_BASE + MEMBER_ROWS {
                        let (rx, ry, rw, rh) = guild_member_row_rect(i, ox, oy);
                        if cursor.x >= rx
                            && cursor.x <= rx + rw
                            && cursor.y >= ry
                            && cursor.y <= ry + rh
                        {
                            let idx = scroll_offset + i - MEMBER_LINE_BASE;
                            if let Some(&mi) = visible.get(idx) {
                                guild.selected_member = Some(idx);
                                tracing::info!("🏰 选中行会成员: {}", guild.members[mi].name);
                            }
                            break;
                        }
                    }
                }
                // 仓库格子点击选中（取出目标，原版 C# StorageGrid 点击语义）
                // C# `StorageGrid[idx]`：列 `x`、窗口行 `r` → `idx = 8*(StorageIndex + r) + x`
                'grid: for r in 0..STORAGE_WINDOW_ROWS {
                    for c in 0..STORAGE_COLS {
                        let (rx, ry, rw, rh) = guild_storage_cell_rect(r, c, ox, oy);
                        if cursor.x >= rx
                            && cursor.x <= rx + rw
                            && cursor.y >= ry
                            && cursor.y <= ry + rh
                        {
                            let slot = STORAGE_COLS * (guild.storage_page + r) + c;
                            if slot < guild.storage_items.len() {
                                guild.selected_storage = Some(slot);
                                tracing::info!("🏰 选中仓库格子 {}", slot);
                            }
                            break 'grid;
                        }
                    }
                }
            }
        }
    }
}

/// 成员行命中矩形（面板原点 ox/oy + 相对坐标；i = `MEMBER_LINE_BASE + 行号`）
/// C# `MembersName[i] @ (125, 30 + i*15)`，页面原点 (0,60)
fn guild_member_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        ox + MEMBER_COL_NAME,
        oy + PAGE_LEFT.1 + MEMBER_ROW_Y0 + (i - MEMBER_LINE_BASE) as f32 * MEMBER_ROW_DY,
        212.0,
        15.0,
    )
}

/// 仓库格命中矩形（窗口行列 `r`/`c`）：StoragePage 内 `(31 + c*36, 20 + r*36)` 35x35
fn guild_storage_cell_rect(r: usize, c: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        ox + STORAGE_GRID_X + c as f32 * STORAGE_CELL_STEP,
        oy + PAGE_LEFT.1 + STORAGE_GRID_Y + r as f32 * STORAGE_CELL_STEP,
        STORAGE_CELL,
        STORAGE_CELL,
    )
}

/// Buff 槽命中矩形（i 1..=8）：BuffPage 内 (4, 27 + (i-1)*38) 36 高
fn guild_buff_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        ox + PAGE_BUFF.0 + 4.0,
        oy + PAGE_BUFF.1 + 27.0 + (i - 1) as f32 * 38.0,
        216.0,
        36.0,
    )
}

/// #2537 Buff 页交互（独立系统：guild_ui_system 已满 16 参 Bevy SystemParam 上限）
/// 开关/翻页 + 行点击（C# BuffButton/RequestBuff/UpButton/DownButton）
fn guild_buff_system(
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    buff_toggle_btn: Query<(Entity, &Interaction), With<GuildBuffToggleBtn>>,
    buff_up_btn: Query<(Entity, &Interaction), With<GuildBuffUp>>,
    buff_down_btn: Query<(Entity, &Interaction), With<GuildBuffDown>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    panel_origin: Query<&Node, With<GuildWidget>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // Buff 页开关（C# BuffButton 切换 BuffPage）
    for (e, inter) in &buff_toggle_btn {
        if edge(e, inter, &mut prev_inter) {
            guild.show_buff_page = !guild.show_buff_page;
            tracing::info!(
                "🏴 行会技能页: {}",
                if guild.show_buff_page { "开" } else { "关" }
            );
        }
    }
    for (e, inter) in &buff_up_btn {
        if edge(e, inter, &mut prev_inter) {
            guild.buff_start = guild.buff_start.saturating_sub(8);
        }
    }
    for (e, inter) in &buff_down_btn {
        if edge(e, inter, &mut prev_inter) && guild.buff_start + 8 < guild.buff_catalog.len() {
            guild.buff_start += 8;
        }
    }
    if !guild.show_buff_page || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // 行点击 → C.GuildBuffUpdate；服务端 toggle 语义（未激活→激活收费校验，已激活→停用），结果走系统消息
    let (ox, oy) = panel_origin
        .single()
        .map(|n| crate::ui::theme::node_origin(n, (GUILD_X, GUILD_Y)))
        .unwrap_or((GUILD_X, GUILD_Y));
    for i in 1..=8usize {
        let (rx, ry, rw, rh) = guild_buff_row_rect(i, ox, oy);
        if cursor.x >= rx && cursor.x <= rx + rw && cursor.y >= ry && cursor.y <= ry + rh {
            if let Some(info) = guild.buff_catalog.get(guild.buff_start + i - 1) {
                net.send_packet(&mir2_shared::packets::client::guild::GuildBuffUpdate {
                    action: 2,
                    buff_id: info.id,
                });
                tracing::info!(
                    "🏴 行会技能: {} #{}（服务端 toggle）",
                    if guild.buff_active(info.id) {
                        "停用"
                    } else {
                        "激活"
                    },
                    info.id
                );
            }
            break;
        }
    }
}

/// #1362：职务改名（C# RanksSelectBox + RanksName + RanksSaveName → EditGuildMember ChangeType=6）
fn guild_rank_rename_system(
    guild: Res<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut rank_dd: Query<(&mut UiDropDown, &GuildRankDrop)>,
    save_btn: Query<(Entity, &Interaction), With<GuildRankSaveBtn>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    // #1395：下拉同步服务端职务定义（顺序即索引）
    let defs = guild.rank_defs.clone();
    let idx = if let Ok((mut dd, _)) = rank_dd.single_mut() {
        if dd.items != defs.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>() {
            dd.items = defs.iter().map(|(n, _)| n.clone()).collect();
            dd.selected = dd.selected.filter(|&s| s < dd.items.len());
        }
        dd.selected.unwrap_or(0)
    } else {
        0
    };
    for (e, inter) in &save_btn {
        if edge(e, inter, &mut prev_inter) && guild.in_guild {
            let name = input.texts.get(4).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 6,
                    rank_index: idx as u8,
                    name: String::new(),
                    rank_name: name.clone(),
                });
                tracing::info!("🏰 职务改名: {} -> {}", idx, name);
                if input.texts.len() > 4 {
                    input.texts[4].clear();
                }
                input.active = None;
            }
        }
    }
}

/// #1395 子批2：加职务/权限勾选/调职（C# EditGuildMember 4/5/2）
fn guild_rank_manage_system(
    guild: Res<GuildState>,
    net: Res<NetConnection>,
    mut input: ResMut<TextInputState>,
    mut rank_dd: Query<(&mut UiDropDown, &GuildRankDrop)>,
    add_btn: Query<(Entity, &Interaction), With<GuildAddRankBtn>>,
    promote_btn: Query<(Entity, &Interaction), With<GuildPromoteBtn>>,
    perm_btns: Query<(Entity, &Interaction, &GuildRankPermBtn)>,
    mut perm_text: Query<&mut Text, With<GuildRankPermText>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let idx = rank_dd
        .single_mut()
        .map(|(dd, _)| dd.selected.unwrap_or(0))
        .unwrap_or(0);
    let options = guild.rank_defs.get(idx).map(|(_, o)| *o).unwrap_or(0);
    for mut t in &mut perm_text {
        let s = format!("权限:{:08b}", options);
        if t.0 != s {
            t.0 = s;
        }
    }
    if !guild.in_guild {
        return;
    }
    for (e, inter) in &add_btn {
        if edge(e, inter, &mut prev_inter) {
            let name = input.texts.get(7).cloned().unwrap_or_default();
            let name = name.trim().to_string();
            if !name.is_empty() {
                net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                    change_type: 4,
                    rank_index: 0,
                    name: String::new(),
                    rank_name: name.clone(),
                });
                tracing::info!("🏰 添加职务: {}", name);
                if input.texts.len() > 7 {
                    input.texts[7].clear();
                }
                input.active = None;
            }
        }
    }
    for (e, inter, p) in &perm_btns {
        if edge(e, inter, &mut prev_inter) {
            let bit = p.0;
            let on = options & (1 << bit) == 0;
            net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                change_type: 5,
                rank_index: idx as u8,
                name: if on {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
                rank_name: bit.to_string(),
            });
            tracing::info!("🏰 职务 #{} 权限位 {} -> {}", idx, bit, on);
        }
    }
    for (e, inter) in &promote_btn {
        if edge(e, inter, &mut prev_inter) {
            if let Some(si) = guild.selected_member {
                if let Some(m) = guild.members.get(si) {
                    net.send_packet(&mir2_shared::packets::client::guild::EditGuildMember {
                        change_type: 2,
                        rank_index: idx as u8,
                        name: m.name.clone(),
                        rank_name: String::new(),
                    });
                    tracing::info!("🏰 调职 {} -> 职务 #{}", m.name, idx);
                }
            }
        }
    }
}

/// #1348：显示离线成员切换（C# MembersShowOfflineButton/Status，纯本地过滤）
fn guild_show_offline_system(
    mut guild: ResMut<GuildState>,
    btn: Query<(Entity, &Interaction), With<GuildShowOfflineBtn>>,
    mut texts: Query<&mut Text, With<GuildShowOfflineText>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    for (e, inter) in &btn {
        if edge(e, inter, &mut prev_inter) {
            guild.show_offline = !guild.show_offline;
            if !guild.show_offline {
                guild.selected_member = None;
            }
        }
    }
    for mut t in &mut texts {
        t.0 = if guild.show_offline {
            "✓显示离线".to_string()
        } else {
            "显示离线".to_string()
        };
    }
}

/// 仓库物品交互（M32）：打开时请求列表 + 存入/取出/翻页
/// 原版 C# GuildDialog：StorageGrid 点击选中 → 拖拽/按钮存入取出；列表由
/// S.GuildStorageList 推送（C# GuildStorageItemChange type=3 请求）
#[allow(clippy::too_many_arguments)]
fn guild_storage_system(
    mgr: ResMut<DialogManager>,
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    inv_q: Query<&crate::game::player_state::Inventory, With<crate::actor::LocalPlayer>>,
    inv_click: Res<crate::game::dialogs::inventory::InvClickState>,
    btns: GuildStorageBtns,
    mut storage_scroll: Query<&mut UiScrollList, With<GuildStorageScroll>>,
    // #2892 批B 单元9：C# `StorageGrid` 的 64 个可见格（图标 + 数量）
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut icons: Query<
        (&GuildStorageIcon, &mut ImageNode, &mut Visibility),
        (With<GuildStorageCell>, Without<GuildStorageCount>),
    >,
    mut counts: Query<(&GuildStorageCount, &mut Text), Without<GuildStorageIcon>>,
    mut loaded: Local<HashMap<usize, i32>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    mut requested: Local<bool>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::Guild);
    if !open {
        *requested = false;
        loaded.clear();
        return;
    }
    // 格阵渲染：`slot = 8*(StorageIndex + r) + c`（本端实体按窗口行 0..8 排布）
    for (icon, mut node, mut vis) in &mut icons {
        let slot = STORAGE_COLS * guild.storage_page + icon.0;
        let item = guild.storage_items.get(slot).and_then(|s| s.as_ref());
        match item {
            Some(it) => {
                if loaded.get(&icon.0).copied() != Some(it.item_index) {
                    if let Some(h) = load_lib_image(
                        &mut libs,
                        &mut images,
                        LibraryName::Items,
                        it.image as usize,
                    ) {
                        node.image = h;
                        loaded.insert(icon.0, it.item_index);
                    }
                }
                if *vis != Visibility::Visible {
                    *vis = Visibility::Visible;
                }
            }
            None => {
                loaded.remove(&icon.0);
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
    for (cell, mut text) in &mut counts {
        let slot = STORAGE_COLS * guild.storage_page + cell.0;
        let want = guild
            .storage_items
            .get(slot)
            .and_then(|s| s.as_ref())
            .map(|it| {
                if it.count > 1 {
                    format!("{}", it.count)
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();
        if text.0 != want {
            text.0 = want;
        }
    }
    // 打开瞬间请求仓库物品列表（原版 C# GuildStorageItemChange type=3 语义）
    if !*requested {
        *requested = true;
        net.send_packet(&crate::network::GuildStorageItemChangeWire {
            change_type: 3,
            grid: 0,
            unique_id: 0,
            count: 0,
        });
        tracing::info!("🏰 请求仓库物品列表");
    }
    // 滚动偏移唯一源 = UiScrollList（滚轮/滑块拖动直写；C# `StoragePage.MouseWheel` +
    // `StoragePositionBar_OnMoving`）；`storage_page` 每帧镜像供格阵渲染
    if let Ok(mut sl) = storage_scroll.single_mut() {
        sl.set_total(STORAGE_ROWS_TOTAL);
        if guild.storage_page != sl.offset {
            guild.storage_page = sl.offset;
        }
    }
    for (e, inter) in &btns.up {
        if edge(e, inter, &mut prev_inter) {
            if let Ok(mut sl) = storage_scroll.single_mut() {
                sl.offset = sl.offset.saturating_sub(1);
            }
        }
    }
    for (e, inter) in &btns.down {
        // C# `StorageDownButton.Click`：`StorageIndex` 上限 6（14 行 - 8 行窗口）
        if edge(e, inter, &mut prev_inter) {
            if let Ok(mut sl) = storage_scroll.single_mut() {
                if sl.offset < STORAGE_MAX_START {
                    sl.offset += 1;
                }
            }
        }
    }
    for (e, inter) in &btns.deposit {
        if edge(e, inter, &mut prev_inter) && guild.in_guild {
            // 选中背包物品 → 存入（原版 C#：选中物品 → GuildStorageItemChange type=0）
            let items = inv_q
                .single()
                .map(|inv| inv.items.as_slice())
                .unwrap_or(&[]);
            let idx = inv_click
                .selected
                .filter(|i| items.get(*i).and_then(|s| s.as_ref()).is_some())
                .or_else(|| items.iter().position(|s| s.is_some()));
            if let Some(i) = idx {
                if let Some(item) = items.get(i).and_then(|s| s.as_ref()) {
                    net.send_packet(&crate::network::GuildStorageItemChangeWire {
                        change_type: 0,
                        grid: 0,
                        unique_id: item.unique_id,
                        count: item.count as u32,
                    });
                    tracing::info!(
                        "🏰 存入背包物品 [{}] uid={} x{}",
                        item.name,
                        item.unique_id,
                        item.count
                    );
                }
            } else {
                tracing::warn!("🏰 背包没有可存入的物品");
            }
        }
    }
    for (e, inter) in &btns.withdraw {
        if edge(e, inter, &mut prev_inter) && guild.in_guild {
            if let Some(slot) = guild.selected_storage {
                if slot < guild.storage_items.len() && guild.storage_items[slot].is_some() {
                    net.send_packet(&crate::network::GuildStorageItemChangeWire {
                        change_type: 1,
                        grid: slot as u8,
                        unique_id: 0,
                        count: 0,
                    });
                    tracing::info!("🏰 取出仓库格子 {}", slot);
                }
            } else {
                tracing::warn!("🏰 请先点击选中一个仓库格子");
            }
        }
    }
}

/// 行会邀请提示：Yes/No → C.GuildInvite{accept}
fn guild_invite_system(
    mut guild: ResMut<GuildState>,
    net: Res<NetConnection>,
    yes: Query<(Entity, &Interaction), With<GuildInviteYes>>,
    no: Query<(Entity, &Interaction), With<GuildInviteNo>>,
    mut widgets: Query<&mut Visibility, With<GuildInviteWidget>>,
    mut texts: Query<&mut Text, With<GuildInviteText>>,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let has_invite = guild.invite.is_some();
    for mut vis in &mut widgets {
        *vis = if has_invite {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut text in &mut texts {
        text.0 = match guild.invite.as_ref() {
            Some(name) => format!("{} 邀请你加入行会", name),
            None => String::new(),
        };
    }
    if guild.invite.is_none() {
        return;
    }
    let mut accept: Option<bool> = None;
    for (e, inter) in &yes {
        if edge(e, inter, &mut prev_inter) {
            accept = Some(true);
        }
    }
    for (e, inter) in &no {
        if edge(e, inter, &mut prev_inter) {
            accept = Some(false);
        }
    }
    if let Some(a) = accept {
        net.send_packet(&mir2_shared::packets::client::guild::GuildInvite { accept_invite: a });
        tracing::info!("🏰 行会邀请回复: accept={}", a);
        guild.invite = None;
    }
}

/// 消费服务端行会事件（网络层只广播 ServerEvent）
fn guild_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut guild: ResMut<GuildState>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::GuildInGuild { in_guild } => {
                guild.in_guild = *in_guild;
                if !guild.in_guild {
                    guild.name.clear();
                    guild.leader.clear();
                    guild.members.clear();
                    guild.notice.clear();
                    guild.gold = 0;
                    guild.storage_items.clear();
                    guild.storage_received = false;
                }
            }
            ServerEvent::GuildData {
                name,
                leader,
                rank_defs,
                notice,
                members,
                gold,
            } => {
                guild.in_guild = true;
                guild.name = name.clone();
                guild.leader = leader.clone();
                guild.rank_defs = rank_defs.clone();
                guild.notice = notice.clone();
                guild.members = members.clone();
                guild.gold = *gold;
            }
            ServerEvent::GuildStorageGoldChanged {
                amount,
                change_type,
                name,
            } => {
                // #295：行会仓库金币实时同步（C# GuildDialog.Gold +/-）
                if *change_type == 0 {
                    guild.gold = guild.gold.saturating_add(*amount);
                } else {
                    guild.gold = guild.gold.saturating_sub(*amount);
                }
                tracing::info!(
                    "💰 行会仓库金币 {} {}（by {}）",
                    if *change_type == 0 {
                        "存入"
                    } else {
                        "取出"
                    },
                    amount,
                    name
                );
            }
            ServerEvent::GuildStorageItemChanged {
                change_type,
                to,
                from,
                item,
            } => {
                // #295：行会仓库物品实时同步（C# 0=存入 1=取出 2=移动）
                match *change_type {
                    0 => {
                        if let Some(item) = item {
                            if *to >= 0 {
                                // 未收到全量列表时先扩容（C# StorageGrid 固定 100 格）
                                let need = (*to as usize).saturating_add(1);
                                if guild.storage_items.len() < need {
                                    guild.storage_items.resize(need, None);
                                }
                                guild.storage_items[*to as usize] = Some(StorageItem {
                                    unique_id: item.unique_id,
                                    item_index: item.item_index,
                                    name: item.name.clone(),
                                    count: item.count,
                                    image: item.image as i32,
                                });
                            }
                        }
                    }
                    1 => {
                        if *from >= 0 && (*from as usize) < guild.storage_items.len() {
                            guild.storage_items[*from as usize] = None;
                        }
                    }
                    2 => {
                        if *from >= 0
                            && *to >= 0
                            && (*from as usize) < guild.storage_items.len()
                            && (*to as usize) < guild.storage_items.len()
                        {
                            let moved = guild.storage_items[*from as usize].take();
                            if let Some(item) = item {
                                guild.storage_items[*to as usize] = Some(StorageItem {
                                    unique_id: item.unique_id,
                                    item_index: item.item_index,
                                    name: item.name.clone(),
                                    count: item.count,
                                    image: item.image as i32,
                                });
                            } else {
                                guild.storage_items[*to as usize] = moved;
                            }
                        }
                    }
                    _ => {}
                }
                tracing::info!(
                    "📦 行会仓库物品变化 type={} to={} from={}",
                    change_type,
                    to,
                    from
                );
            }
            ServerEvent::GuildStorage { items } => {
                guild.storage_items = items
                    .iter()
                    .map(|slot| {
                        slot.as_ref()
                            .map(|(unique_id, item_index, count, info_name, image)| {
                                let name = if !info_name.is_empty() {
                                    info_name.clone()
                                } else {
                                    guild
                                        .item_names
                                        .get(item_index)
                                        .cloned()
                                        .unwrap_or_default()
                                };
                                StorageItem {
                                    unique_id: *unique_id,
                                    item_index: *item_index,
                                    name,
                                    count: *count,
                                    image: *image,
                                }
                            })
                    })
                    .collect();
                guild.storage_received = true;
            }
            ServerEvent::GuildNotice { notice } => {
                guild.notice = notice.clone();
            }
            ServerEvent::GuildMemberChanged {
                name,
                rank,
                online,
                joined,
                removed,
            } => {
                if *removed {
                    guild.members.retain(|m| m.name != *name);
                } else if *joined {
                    if !guild.members.iter().any(|m| m.name == *name) {
                        guild.members.push(GuildMember {
                            name: name.clone(),
                            rank: *rank,
                            rank_index: *rank,
                            online: *online,
                        });
                    }
                } else if let Some(m) = guild.members.iter_mut().find(|m| m.name == *name) {
                    m.rank = *rank;
                    m.rank_index = *rank;
                    m.online = *online;
                }
            }
            ServerEvent::GuildInvited { name } => {
                guild.invite = Some(name.clone());
            }
            ServerEvent::GuildBuffList { active, catalog } => {
                // #2537：行会技能同步（目录 + 激活列表；打开对话框/他人变更时刷新）
                guild.active_buffs = active.clone();
                guild.buff_catalog = catalog.clone();
                // 目录变短时夹紧到最后一页起点（8 行/页）
                let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
                if guild.buff_start > max_start {
                    guild.buff_start = max_start;
                }
                tracing::info!(
                    "🏴 行会技能已同步: 目录 {} 项（激活 {}）",
                    catalog.len(),
                    active.len()
                );
            }
            ServerEvent::UserInformation { item_names, .. } => {
                for (idx, name) in item_names {
                    guild.item_names.insert(*idx, name.clone());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    /// #2892：NoticePage 位置条与滚轮（C# `GuildDialog` `:1343-1410`）——
    /// `interval = 289/(len-25)`（**整数除**）、`y ∈ [16,298]`、`index ∈ [0,len-25]`、
    /// 滚轮到顶/到底停住；公告不足一屏（`len <= 25`）时位置条隐藏（C# 那里是除零）。
    #[test]
    fn notice_bar_and_wheel_match_csharp() {
        // 不足一屏 → 无位置条、滚轮无效
        assert_eq!(notice_bar_y(0, 25), None);
        assert_eq!(notice_bar_y(3, 10), None);
        assert_eq!(notice_index_from_bar_y(200.0, 20), 0);
        assert_eq!(notice_wheel_scroll(0, -1, 20), 0);
        // len=50 → interval = 289/25 = 11（整数除，非 11.56）
        assert_eq!(notice_bar_interval(50), 11.0);
        assert_eq!(notice_bar_y(0, 50), Some(NOTICE_BAR_Y_MIN));
        assert_eq!(notice_bar_y(25, 50), Some(NOTICE_BAR_Y_MIN + 25.0 * 11.0));
        // 上限夹到 298（len=39 → interval=289/14=20；index=20 → 16+400 → 夹到 298）
        assert_eq!(notice_bar_interval(39), 20.0);
        assert_eq!(notice_bar_y(20, 39), Some(NOTICE_BAR_Y_MAX));
        // y 反算（C# `NoticePositionBar_OnMoving`）
        assert_eq!(notice_index_from_bar_y(10.0, 50), 0);
        assert_eq!(notice_index_from_bar_y(16.0, 50), 0);
        assert_eq!(notice_index_from_bar_y(27.0, 50), 1);
        assert_eq!(notice_index_from_bar_y(298.0, 50), 25);
        assert_eq!(notice_index_from_bar_y(999.0, 50), 25);
        // 往返自洽
        for i in 0..=25usize {
            assert_eq!(
                notice_index_from_bar_y(notice_bar_y(i, 50).unwrap(), 50),
                i,
                "index={i} 的往返应自洽"
            );
        }
        // 滚轮：到顶向上停、到底向下停，其余逐行
        assert_eq!(notice_wheel_scroll(0, 1, 50), 0);
        assert_eq!(notice_wheel_scroll(0, -1, 50), 1);
        assert_eq!(notice_wheel_scroll(25, -1, 50), 25);
        assert_eq!(notice_wheel_scroll(25, 1, 50), 24);
        assert_eq!(notice_wheel_scroll(3, 0, 50), 3);
        // 位置条矩形：面板原点 (217,168) + 页偏移 (0,60) + 条 (337,16)
        let (bx, by, bw, bh) = guild_notice_bar_rect(NOTICE_BAR_Y_MIN, GUILD_X, GUILD_Y);
        assert_eq!((bx, by, bw, bh), (554.0, 244.0, 12.0, 18.0));
        // 条与翻页钮同列（x=337）、在页面内、（拖动下限）不越过下翻钮 (337,318)
        assert_eq!(NOTICE_BAR_X + NOTICE_BAR_W, 349.0);
        assert!(NOTICE_BAR_X + NOTICE_BAR_W <= PAGE_LEFT.2);
        assert!(NOTICE_BAR_Y_MAX + NOTICE_BAR_H <= 318.0);
    }

    /// 成员行命中：初始原点等价于原固定坐标，拖动后跟随面板
    #[test]
    fn member_row_rect_origin_and_drag() {
        // #2892 批B 单元7：MembersPage @(0,60)，行 (125, 30+20i) → 面板内 (125, 90+20i)
        let (rx, ry, rw, rh) = guild_member_row_rect(1, GUILD_X, GUILD_Y);
        assert_eq!((rx, ry, rw, rh), (342.0, 258.0, 212.0, 15.0));
        assert_eq!(
            guild_member_row_rect(10, GUILD_X, GUILD_Y).1,
            258.0 + 9.0 * 15.0
        );
        // 拖动到 (330,100)：同一相对位置命中跟随（原始坐标 + delta(50,20)）
        let (rx2, ry2, _, _) = guild_member_row_rect(1, 330.0, 100.0);
        assert_eq!((rx2, ry2), (455.0, 190.0));
        // C# `MemberPageRows = 18`：末行页内 y = 30 + 17*15 = 285 → 屏幕 y = 168+60+285 = 513
        assert_eq!(
            guild_member_row_rect(MEMBER_LINE_BASE + MEMBER_ROWS - 1, GUILD_X, GUILD_Y).1,
            513.0
        );
    }

    /// #2892 批B 单元9：仓库**格阵**命中（C# `StorageGrid[idx]`，列 `x`/窗口行 `r`）。
    /// 页面原点 (0,60)、格阵起点 (31,20)、步进 36、格 35x35。
    #[test]
    fn storage_cell_rect_origin_and_drag() {
        let (rx, ry, rw, rh) = guild_storage_cell_rect(0, 0, GUILD_X, GUILD_Y);
        assert_eq!((rx, ry, rw, rh), (248.0, 248.0, 35.0, 35.0));
        // 第 8 列 / 第 8 窗口行：31+7*36=283 → 屏幕 500；20+7*36=272 → 屏幕 500
        let (rx2, ry2, _, _) = guild_storage_cell_rect(7, 7, GUILD_X, GUILD_Y);
        assert_eq!((rx2, ry2), (500.0, 500.0));
        // 拖动后跟随面板原点
        let (rx3, ry3, _, _) = guild_storage_cell_rect(0, 0, 330.0, 100.0);
        assert_eq!((rx3, ry3), (361.0, 180.0));
        // 8×14 数据格、可见窗口 8 行、行窗口上限 6（C# `if (StorageIndex >= 6) StorageIndex = 5;` 上下钳位）
        assert_eq!(
            (
                STORAGE_COLS * STORAGE_ROWS_TOTAL,
                STORAGE_WINDOW_ROWS,
                STORAGE_MAX_START
            ),
            (112, 8, 6)
        );
    }

    /// Buff 行命中：初始等价 + 拖动跟随
    #[test]
    fn buff_row_rect_origin_and_drag() {
        let (rx, ry, rw, _) = guild_buff_row_rect(1, GUILD_X, GUILD_Y);
        assert_eq!(
            (rx, ry, rw),
            (581.0, 256.0, 216.0),
            "BuffPage @(360,61) 内 (4,27)"
        );
        let (rx2, ry2, _, _) = guild_buff_row_rect(1, 330.0, 100.0);
        assert_eq!((rx2, ry2), (694.0, 188.0));
    }

    use super::*;

    fn buff_info(id: i32, name: &str) -> mir2_shared::data::client_data::GuildBuffInfo {
        mir2_shared::data::client_data::GuildBuffInfo {
            id,
            icon: 24,
            name: name.to_string(),
            level_requirement: 3,
            points_requirement: 2,
            time_limit: 60,
            activation_cost: 500,
            stats: mir2_shared::data::stats::Stats::new(),
        }
    }

    /// #2537 Buff 行文本（激活态标记，C# GuildBuffButton Name/Info）
    #[test]
    fn buff_row_text_marks_active() {
        let info = buff_info(1, "经验加成");
        assert!(buff_row_text(&info, true).contains("[已激活]"));
        assert!(!buff_row_text(&info, false).contains("[已激活]"));
        assert!(buff_row_text(&info, false).contains("Lv3"));
        assert!(buff_row_text(&info, false).contains("点2"));
        assert!(buff_row_text(&info, false).contains("金500"));
    }

    /// #2537 页数（C# 8 GuildBuffButton/页）：0/8 → 1 页，9/16 → 2 页
    #[test]
    fn buff_page_count_rounds_up() {
        assert_eq!(buff_page_count(0), 1);
        assert_eq!(buff_page_count(8), 1);
        assert_eq!(buff_page_count(9), 2);
        assert_eq!(buff_page_count(16), 2);
    }

    /// #2537 目录同步夹紧：buff_start 超出末页回夹（8 行/页）
    #[test]
    fn buff_start_clamped_on_sync() {
        let mut guild = GuildState::default();
        guild.buff_catalog = vec![buff_info(1, "a"); 16];
        guild.buff_start = 8;
        guild.active_buffs = vec![1];
        // 复现 GuildBuffList arm 的夹紧逻辑
        let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
        if guild.buff_start > max_start {
            guild.buff_start = max_start;
        }
        assert_eq!(guild.buff_start, 8);
        assert!(guild.buff_active(1));
        assert!(!guild.buff_active(2));
        // 目录缩到 9 项 → 末页起点 0…wait 9 项末页起点 = 8/8*8 = 8? (9-1)/8*8 = 8
        guild.buff_catalog.truncate(9);
        let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
        if guild.buff_start > max_start {
            guild.buff_start = max_start;
        }
        assert_eq!(guild.buff_start, 8);
        // 目录缩到 8 项 → 末页起点 0
        guild.buff_catalog.truncate(8);
        let max_start = guild.buff_catalog.len().saturating_sub(1) / 8 * 8;
        if guild.buff_start > max_start {
            guild.buff_start = max_start;
        }
        assert_eq!(guild.buff_start, 0);
    }
    #[test]
    fn guild_origin_is_centered() {
        assert_eq!(
            crate::game::dialogs::center_origin(GUILD_W, GUILD_H),
            (GUILD_X, GUILD_Y)
        );
    }
    /// #2892 批B 单元10：行会页签权限门控（C# `GuildRankOptions` 位值 + `RefreshInterface` 规则）。
    ///
    /// 阳性对照：把 `Storage` 一档改成 `GUILD_OPT_CHANGE_NOTICE`（= 用错权限位）→ 本测试 FAILED。
    #[test]
    fn guild_tab_visibility_follows_csharp_options() {
        use GuildPage::*;
        // 位值来自 `Shared/Enums.cs:1898-1908`
        assert_eq!(
            (
                GUILD_OPT_CHANGE_RANK,
                GUILD_OPT_RECRUIT,
                GUILD_OPT_KICK,
                GUILD_OPT_STORE_ITEM,
                GUILD_OPT_RETRIEVE_ITEM,
                GUILD_OPT_ALTER_ALLIANCE,
                GUILD_OPT_CHANGE_NOTICE,
                GUILD_OPT_ACTIVATE_BUFF
            ),
            (1, 2, 4, 8, 16, 32, 64, 128)
        );
        // 全权限：该显示的都显示
        let all = Some(255u8);
        for p in [Notice, Members, Storage, Rank, Status, Buff] {
            assert!(guild_tab_visible(p, all, true), "{p:?} 全权限应可见");
        }
        // 无权限：Notice/Storage/Rank 隐藏，Members/Status 恒可见
        let none = Some(0u8);
        assert!(!guild_tab_visible(Notice, none, true));
        assert!(!guild_tab_visible(Storage, none, true));
        assert!(!guild_tab_visible(Rank, none, true));
        assert!(guild_tab_visible(Members, none, true));
        assert!(guild_tab_visible(Status, none, true));
        // 只有存取其一 → Storage 可见（C# `CanStoreItem || CanRetrieveItem`）
        assert!(guild_tab_visible(Storage, Some(GUILD_OPT_STORE_ITEM), true));
        assert!(guild_tab_visible(
            Storage,
            Some(GUILD_OPT_RETRIEVE_ITEM),
            true
        ));
        // 公告位只管 Notice
        assert!(guild_tab_visible(
            Notice,
            Some(GUILD_OPT_CHANGE_NOTICE),
            true
        ));
        assert!(!guild_tab_visible(
            Storage,
            Some(GUILD_OPT_CHANGE_NOTICE),
            true
        ));
        // 职务位只管 Rank
        assert!(guild_tab_visible(Rank, Some(GUILD_OPT_CHANGE_RANK), true));
        assert!(!guild_tab_visible(
            Notice,
            Some(GUILD_OPT_CHANGE_RANK),
            true
        ));
        // Buff 只看目录是否非空
        assert!(!guild_tab_visible(Buff, all, false));
        assert!(guild_tab_visible(Buff, none, true));
        // 拿不到自己的权限（未入会/成员表未到）→ 不隐藏
        for p in [Notice, Storage, Rank] {
            assert!(guild_tab_visible(p, None, true), "{p:?} 权限未知时应可见");
        }
    }

    /// #2892 批B 单元10：`my_options` 推导 —— 本地玩家成员行 → `rank_defs[rank_index].options`
    #[test]
    fn guild_my_options_derives_from_member_rank() {
        let mut st = GuildState::default();
        st.rank_defs = vec![("会长".to_string(), 255u8), ("成员".to_string(), 0u8)];
        st.members = vec![
            GuildMember {
                name: "bob".to_string(),
                rank: 0,
                rank_index: 0,
                online: true,
            },
            GuildMember {
                name: "alice".to_string(),
                rank: 1,
                rank_index: 1,
                online: true,
            },
        ];
        assert_eq!(guild_my_options(&st, Some("bob")), Some(255));
        assert_eq!(guild_my_options(&st, Some("alice")), Some(0));
        assert_eq!(
            guild_my_options(&st, Some("carol")),
            None,
            "不在成员表 → None"
        );
        assert_eq!(guild_my_options(&st, None), None, "无本地名 → None");
        // rank_index 越界 → None（不 panic）
        st.members[0].rank_index = 9;
        assert_eq!(guild_my_options(&st, Some("bob")), None);
    }
    /// #2892 批B 单元11：公告滚动（C# `NoticeScrollIndex`：首行下标，0..=len-1 钳位）。
    ///
    /// 阳性对照：把下钳位去掉（只 `+1` 不 clamp）→ 本测试的「到底再加不动」断言 FAILED。
    #[test]
    fn notice_scroll_clamps_like_csharp() {
        assert_eq!(notice_next_scroll(0, -1, 5), 0, "到顶再加不动");
        assert_eq!(notice_next_scroll(0, 1, 5), 1);
        assert_eq!(notice_next_scroll(3, 1, 5), 4);
        assert_eq!(notice_next_scroll(4, 1, 5), 4, "到底再加不动");
        assert_eq!(notice_next_scroll(4, -1, 5), 3);
        // 空公告：恒 0（C# `Notice.MultiText.Length - 1` 会是 -1，本端取 0 避免下溢）
        assert_eq!(notice_next_scroll(3, -1, 0), 0);
        assert_eq!(notice_next_scroll(3, 1, 0), 0);
        // 与可见行数常数一致：正文区 330px / 16px 行高 = 20 行
        assert_eq!(NOTICE_ROWS, 20);
        assert_eq!(NOTICE_ROW_DY, 16.0);
        assert_eq!(NOTICE_ROWS as f32 * NOTICE_ROW_DY, 320.0);
        assert!(
            NOTICE_ROWS as f32 * NOTICE_ROW_DY <= 330.0,
            "[越界] 公告 20 行不越出 C# `Notice` 文本框高 330"
        );
    }
    /// #2892 批B 单元12：成员行「能否改职务 / 能否踢人」规则（C# `UpdateMembers`，`:1611-1618`）。
    ///
    /// 阳性对照：把 `can_kick_member` 的「不是自己」判断去掉 → 本测试的「不能踢自己」断言 FAILED。
    #[test]
    fn member_row_permission_rules_match_csharp() {
        let full = Some(GUILD_OPT_CHANGE_RANK | GUILD_OPT_KICK);
        // 改职务：CanChangeRank && 成员职务下标 >= 自己
        assert!(can_change_member_rank(full, 2, 2), "同级可改");
        assert!(can_change_member_rank(full, 2, 3), "更低可改");
        assert!(!can_change_member_rank(full, 2, 1), "更高不可改");
        assert!(
            !can_change_member_rank(Some(GUILD_OPT_KICK), 2, 3),
            "无 CanChangeRank 不可改"
        );
        assert!(
            !can_change_member_rank(None, 2, 3),
            "拿不到权限位时不放行（与页签门控的「不隐藏」策略相反，见 §7）"
        );
        // 踢人：CanKick && 成员职务下标 >= 自己 && 不是自己
        assert!(can_kick_member(full, 2, 2, "alice", Some("bob")));
        assert!(can_kick_member(full, 2, 3, "alice", Some("bob")));
        assert!(
            !can_kick_member(full, 2, 1, "alice", Some("bob")),
            "更高不可踢"
        );
        assert!(
            !can_kick_member(full, 2, 3, "bob", Some("bob")),
            "不能踢自己（C# `Members[j].Name != MapControl.User.Name`）"
        );
        assert!(
            !can_kick_member(Some(GUILD_OPT_CHANGE_RANK), 2, 3, "alice", Some("bob")),
            "无 CanKick 不可踢"
        );
        // 自己的职务下标推导（C# `MyRankId`）
        let mut st = GuildState::default();
        st.members = vec![GuildMember {
            name: "bob".to_string(),
            rank: 0,
            rank_index: 2,
            online: true,
        }];
        assert_eq!(guild_my_rank_index(&st, Some("bob")), Some(2));
        assert_eq!(guild_my_rank_index(&st, Some("carol")), None);
        assert_eq!(guild_my_rank_index(&st, None), None);
    }
}
