// ============================================================================
// 任务日志对话框（M43 + #2535 任务流程 UI）
// 参考：C# QuestLogDialog/QuestListDialog + ServerRust quest.rs
// 网络：
//   C: AcceptQuest[npc_index u32][quest_index i32] / FinishQuest[quest_index i32][selected_item_index i32]
//      / AbandonQuest[i32] / ShareQuest[i32]
//   S: ChangeQuest[id i32][count i32][task dotnet...][taken u8][completed u8][new u8]
//      NewQuestInfo（任务定义全量目录，登录下发）→ QuestCatalog
//      CompleteQuest[quest_index i32]
// #2535：已接任务由 ChangeQuest 驱动（行首段），可接任务由目录推导（行尾段）；
//        接受/完成按钮状态机 + 可选奖励多选一（C# _acceptButton/_finishButton/UpdateRewards）
// ============================================================================

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot, UI_SCREEN_W};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_container, spawn_icon_button, spawn_image,
    spawn_label, spawn_label_plain, spawn_panel,
};
use mir2_shared::data::client_data::ClientQuestInfo;
use mir2_shared::data::shared_data::QuestItemReward;

/// #2892 批B：日记窗面板与 C# 原点（C# `QuestDiaryDialog.Index = 961;
/// Location = (ScreenWidth/2 - 300 - 20, 60)` = **(192, 60)**）
pub const DIARY_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 961);
pub const DIARY_SIZE: (f32, f32) = (316.0, 466.0);
pub const DIARY_POS: (f32, f32) = (192.0, 60.0);
/// 关闭键 `Prguse2[360..362]` @(289,3)（日志 `QuestDialogs.cs:243` / 详情 `:611`，无 `Size` → 原生 24x21）
pub const CLOSE_POS: (f32, f32) = (289.0, 3.0);
/// C# `QuestDetailDialog.Index = 960` @ `(ScreenWidth/2 + 20, 60)` = (532,60)
pub const DETAIL_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 960);
pub const DETAIL_POS: (f32, f32) = (532.0, 60.0);

/// C# Globals.MaxConcurrentQuests（服务端 quest_log.can_accept 同值）
pub const MAX_CONCURRENT_QUESTS: usize = 20;

/// 「任务：{已接}/{上限}」计数标签的面板内位置。
///
/// C# `QuestDiaryDialog` 构造器：`_takenQuestsLabel { Parent = this, AutoSize = true,
/// Location = new Point(210, 7) }`——在**标题栏右侧**、关闭钮左边。
/// 此前写死在 (18,20)，实测正好压在标题栏下沿、切到面板边框。
pub const DIARY_COUNT_POS: (f32, f32) = (210.0, 7.0);

/// 任务条目（ChangeQuest 写入）
#[derive(Debug, Clone, Default)]
pub struct QuestEntry {
    pub id: i32,
    pub name: String,
    pub tasks: Vec<String>,
    pub taken: bool,
    pub completed: bool,
    pub is_new: bool,
}

/// 任务日志状态
#[derive(Resource, Default)]
pub struct QuestLogState {
    pub quests: Vec<QuestEntry>,
    pub selected: Option<usize>,
    /// #2535 可接任务段的选中行（下标属 available_quests 结果）
    pub selected_avail: Option<usize>,
    /// #2535 可选奖励当前选择（下标属 ClientQuestInfo.rewards_select_item；换行时重置）
    pub selected_reward: Option<usize>,
    pub message: String,
    /// #2535 子批2：展开的组（C# QuestDiaryDialog.ExpandedGroups；空集=全部展开，
    /// 跨开合面板持久，对齐 C# 字段生命周期）
    pub expanded_groups: HashSet<String>,
}

/// #2535 任务定义目录（S.NewQuestInfo 登录全量下发；C# QuestInfo 缓存）
/// 目录 ≠ 日志：可接任务从这里推导，已接任务仍由 ChangeQuest 驱动
#[derive(Resource, Default)]
pub struct QuestCatalog {
    pub infos: Vec<ClientQuestInfo>,
    /// 本次会话已交任务（QuestCompleted 累计）。历史完成列表服务端未同步，
    /// 误点的接受请求由服务端 HasCompletedQuest 校验兜底（"该任务已完成"）
    pub completed: HashSet<i32>,
    /// 物品名（UserInformation 随包下发；奖励物品不在背包时显示 物品#索引）
    pub item_names: HashMap<i32, String>,
}

/// #2801 单元①：任务详情窗状态（C# `QuestDetailDialog.Quest`，`QuestDialogs.cs:469`）。
///
/// 窗口内容（奖励区）在单元③补；单元②已接入消息区分页（`top_line`）。
#[derive(Resource, Default)]
pub struct QuestDetailState {
    /// 当前展示的任务 id（None=未展示过）
    pub quest_id: Option<i32>,
    /// C# `QuestMessage.TopLine`（`:1013`；分页首行下标，换任务时归零）
    pub top_line: usize,
    /// #2801 单元③：C# `MirMessageBox(AskCancelQuest, YesNo)` 是否在弹（`:588-599`）
    pub confirm_cancel: bool,
    /// #2801 单元③：C# `QuestDetailDialog.Reward.SelectedItemIndex`（`:475`）——
    /// 可选奖励**未过滤**列表的下标（`FindSelectedItemIndex`，`:1268-1284`）
    pub selected_reward: Option<usize>,
}

/// #2535 C# QuestListDialog.ReDisplayButtons 按钮状态机（纯函数）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestRowAction {
    /// 未接且符合条件 → 显示"接受"（可点）
    Accept,
    /// 已接未完成 → "完成"置灰
    InProgress,
    /// 已接且完成 → 显示"完成"（可点）
    Finishable,
    /// 未接但不可接（数量上限/等级/职业）→ "接受"置灰，行内标注原因
    Locked(&'static str),
}

/// #2535 按钮状态判定：C# ReDisplayButtons + NPCObject.CanAccept 的客户端可判部分。
/// 前置任务/历史完成依赖服务端数据（未同步），不在此过滤——误点由服务端拒绝并回系统消息。
pub fn row_action(
    info: &ClientQuestInfo,
    taken: Option<&QuestEntry>,
    level: u16,
    class: u8,
    taken_count: usize,
) -> QuestRowAction {
    if let Some(e) = taken {
        return if e.completed {
            QuestRowAction::Finishable
        } else {
            QuestRowAction::InProgress
        };
    }
    if taken_count >= MAX_CONCURRENT_QUESTS {
        return QuestRowAction::Locked("任务数量已达上限");
    }
    if (level as i32) < info.min_level_needed {
        return QuestRowAction::Locked("等级不足");
    }
    if info.max_level_needed > 0 && (level as i32) > info.max_level_needed {
        return QuestRowAction::Locked("等级过高");
    }
    if !class_allowed(info.class_needed, class) {
        return QuestRowAction::Locked("职业不符合");
    }
    QuestRowAction::Accept
}

/// RequiredClass 位掩码 × HudState.class（MirClass as u8；0=不限）
fn class_allowed(required: mir2_shared::enums::RequiredClass, class: u8) -> bool {
    use mir2_shared::enums::RequiredClass;
    if required.bits() == 0 {
        return true;
    }
    let bit = match class {
        0 => RequiredClass::WARRIOR,
        1 => RequiredClass::WIZARD,
        2 => RequiredClass::TAOIST,
        3 => RequiredClass::ASSASSIN,
        _ => RequiredClass::ARCHER,
    };
    required.contains(bit)
}

/// #2535 C# _finishButton.Click L138-146：有可选奖励但未选时阻止交任务
pub fn finish_selected_index(
    select_rewards: &[QuestItemReward],
    selected: Option<usize>,
) -> Result<i32, &'static str> {
    if select_rewards.is_empty() {
        // 无可选奖励：-1（服务端语义：selected_item_index<0 不发放）
        return Ok(-1);
    }
    match selected {
        Some(i) if i < select_rewards.len() => Ok(i as i32),
        _ => Err("请先选择一件奖励物品"),
    }
}

/// #2535 可接任务段：目录 − 已接 − 本次会话已完成，再经 row_action 过滤
/// （Locked 不显示，对齐 C# GetAvailableQuests 只列可接）
pub fn available_quests<'a>(
    catalog: &'a QuestCatalog,
    log: &QuestLogState,
    level: u16,
    class: u8,
) -> Vec<&'a ClientQuestInfo> {
    catalog
        .infos
        .iter()
        .filter(|info| !log.quests.iter().any(|q| q.id == info.index))
        .filter(|info| !catalog.completed.contains(&info.index))
        .filter(|info| {
            matches!(
                row_action(info, None, level, class, log.quests.len()),
                QuestRowAction::Accept
            )
        })
        .collect()
}

/// #2535 目录写入（S.NewQuestInfo 幂等合并，纯函数）
pub fn upsert_catalog_info(infos: &mut Vec<ClientQuestInfo>, info: &ClientQuestInfo) {
    if let Some(e) = infos.iter_mut().find(|c| c.index == info.index) {
        *e = info.clone();
    } else {
        infos.push(info.clone());
    }
}

/// #2535 子批2：C# QuestDiaryDialog.DisplayQuests L710——已接任务按 QuestInfo.Group 分组。
/// 组序=首次出现序（C# GroupBy 保持 encounter order）；QuestEntry 下标 → 目录组名，
/// 目录缺失回退空串（C# QuestInfo 常驻内存，此为包未到时的兜底）
pub fn diary_groups(quests: &[QuestEntry], infos: &[ClientQuestInfo]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (idx, q) in quests.iter().enumerate() {
        let group = infos
            .iter()
            .find(|c| c.index == q.id)
            .map(|c| c.group.clone())
            .unwrap_or_default();
        match groups.iter_mut().find(|(g, _)| *g == group) {
            Some((_, list)) => list.push(idx),
            None => groups.push((group, vec![idx])),
        }
    }
    groups
}

/// #2535 子批2：C# L718 展开判定——ExpandedGroups 空集=全部展开
pub fn group_expanded(expanded: &HashSet<String>, group: &str) -> bool {
    expanded.is_empty() || expanded.contains(group)
}

/// #2535 子批2：C# QuestGroupQuestItem.ChangeExpand + ExpandedChanged L726-742——
/// 翻转目标组后物化全量展开集（空集语义下首次收起会把其余组记为展开）
pub fn toggle_group(
    groups: &[(String, Vec<usize>)],
    expanded: &HashSet<String>,
    target: &str,
) -> HashSet<String> {
    groups
        .iter()
        .map(|(g, _)| (g, group_expanded(expanded, g)))
        .map(|(g, was)| (g, if g == target { !was } else { was }))
        .filter(|(_, on)| *on)
        .map(|(g, _)| g.clone())
        .collect()
}

/// #2535 子批2：日记行模型——组头 + 展开组内任务 + 可接段（C# 组头 15px/行 15px 纵排）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiaryRow {
    /// 组头（下标属 diary_groups 结果）
    Header(usize),
    /// 已接任务（下标属 QuestLogState.quests）
    Quest(usize),
    /// 可接任务（下标属 available_quests 结果）
    Avail(usize),
}

/// #2535 子批2：行模型扁平化（收起组仅留组头；供 8 行视窗取前 N）
pub fn diary_rows(
    quests: &[QuestEntry],
    infos: &[ClientQuestInfo],
    avail_len: usize,
    expanded: &HashSet<String>,
) -> Vec<DiaryRow> {
    let mut rows = Vec::new();
    for (gi, (group, list)) in diary_groups(quests, infos).iter().enumerate() {
        rows.push(DiaryRow::Header(gi));
        if group_expanded(expanded, group) {
            for &qi in list {
                rows.push(DiaryRow::Quest(qi));
            }
        }
    }
    for k in 0..avail_len {
        rows.push(DiaryRow::Avail(k));
    }
    rows
}

#[derive(Component)]
pub struct QuestLogWidget;

#[derive(Component)]
pub struct QuestLogClose;

#[derive(Component)]
pub struct QuestLogAbandon;

/// #2535 接受按钮（C# QuestListDialog._acceptButton Title[270-272]）
#[derive(Component)]
pub struct QuestLogAccept;

/// #2535 完成按钮（C# QuestListDialog._finishButton Title[273-275]）
#[derive(Component)]
pub struct QuestLogFinish;

#[derive(Component)]
pub struct QuestLogLine(usize);

/// 每行“追踪/取消追踪”按钮（C# QuestRow Track 按钮）
#[derive(Component)]
pub struct QuestLogTrack(usize);

/// #2801 单元①：任务详情窗根面板（C# `QuestDetailDialog`，`Prguse[960]`）
#[derive(Component)]
pub struct QuestDetailWidget;

/// #2801 单元①：任务详情窗关闭键（C# `QuestDetailDialog.closeButton`，`Prguse2[360..362]`）
#[derive(Component)]
pub struct QuestDetailClose;

/// #2801 单元②：消息区行标签槽 0..16（C# `QuestMessage._textLabel[LineCount]`，`:1036`）
#[derive(Component)]
pub struct QuestDetailLine(pub usize);

/// #2810 单元①：消息区行的**彩色叠加段**（C# `NewColour` 每段一个叠加 `MirLabel`，`:1336-1353`）。
/// 每行一个固定池（`QUEST_MSG_MAX_SEGMENTS`），按解析结果显隐/落位。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct QuestDetailSegment {
    /// 所属行槽（`QuestDetailLine` 的下标）
    pub slot: usize,
    /// 行内第几个彩色段（超出池容量或本行无段则隐藏）
    pub seg: usize,
}

/// #2810 单元①：每行叠加段池容量（C# 不限段数，实际脚本行 1-2 段；池化避免每帧增删实体）
pub const QUEST_MSG_MAX_SEGMENTS: usize = 6;

/// #2810 单元②：行内链接类型（C# `NPCDialog.MonsterLink/NPCLink/ItemLink` 三条正则，`NPCDialogs.cs:24-26`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestLinkKind {
    Monster,
    Npc,
    Item,
}

impl QuestLinkKind {
    /// C# `GetDisplayNameForLink` 的 linkType 字面量（`:920-955`）
    pub fn type_name(self) -> &'static str {
        match self {
            QuestLinkKind::Monster => "MONSTER",
            QuestLinkKind::Npc => "NPC",
            QuestLinkKind::Item => "ITEM",
        }
    }
    /// C# 名字缺失时的回退字面量（`Item {idx}` / `Monster {idx}` / `Npc {idx}`，`:930-955`）
    pub fn fallback_name(self, idx: &str) -> String {
        match self {
            QuestLinkKind::Monster => format!("Monster {idx}"),
            QuestLinkKind::Npc => format!("Npc {idx}"),
            QuestLinkKind::Item => format!("Item {idx}"),
        }
    }
}

/// #2810 单元②：一条链接标记（原文范围 + 类型 + 下标 + 可选内嵌名）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLink {
    pub kind: QuestLinkKind,
    /// C# 捕获组 `idx`（字符串形态，原样参与名字回退拼接）
    pub index: String,
    /// C# 捕获组 `name`（`[ITEM:1|力量戒指]` 的内嵌名，优先于查表）
    pub provided_name: Option<String>,
    /// 在传入文本中的字节范围（含标记本身）
    pub range: std::ops::Range<usize>,
}

/// #2810 单元②：扫描一行里的链接标记——三条 C# 正则的等价手写扫描（大小写不敏感）：
/// `[KIND:idx(|name)]` 与 `<$KIND:idx>`（KIND ∈ MONSTER/NPC/ITEM，`NPCDialogs.cs:24-26`）。
pub fn quest_line_links(line: &str) -> Vec<QuestLink> {
    fn kind_of(s: &str) -> Option<QuestLinkKind> {
        let up = s.to_ascii_uppercase();
        match up.as_str() {
            "MONSTER" => Some(QuestLinkKind::Monster),
            "NPC" => Some(QuestLinkKind::Npc),
            "ITEM" => Some(QuestLinkKind::Item),
            _ => None,
        }
    }
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        // 形式 1：`[KIND:idx(|name)]`
        if b[i] == b'[' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b':' && b[j] != b']' && (j - i) < 12 {
                j += 1;
            }
            if j < b.len() && b[j] == b':' {
                if let Some(kind) = kind_of(&line[i + 1..j]) {
                    let mut k = j + 1;
                    let dstart = k;
                    while k < b.len() && b[k].is_ascii_digit() {
                        k += 1;
                    }
                    if k > dstart {
                        let idx = line[dstart..k].to_string();
                        let mut provided = None;
                        if k < b.len() && b[k] == b'|' {
                            let ns = k + 1;
                            let mut ne = ns;
                            while ne < b.len() && b[ne] != b']' {
                                ne += 1;
                            }
                            provided = Some(line[ns..ne].to_string());
                            k = ne;
                        }
                        if k < b.len() && b[k] == b']' {
                            out.push(QuestLink {
                                kind,
                                index: idx,
                                provided_name: provided,
                                range: i..k + 1,
                            });
                            i = k + 1;
                            continue;
                        }
                    }
                }
            }
        }
        // 形式 2：`<$KIND:idx>`
        if b[i] == b'<' && i + 1 < b.len() && b[i + 1] == b'$' {
            let mut j = i + 2;
            while j < b.len() && b[j] != b':' && b[j] != b'>' && (j - i) < 13 {
                j += 1;
            }
            if j < b.len() && b[j] == b':' {
                if let Some(kind) = kind_of(&line[i + 2..j]) {
                    let ds = j + 1;
                    let mut k = ds;
                    while k < b.len() && b[k].is_ascii_digit() {
                        k += 1;
                    }
                    if k > ds && k < b.len() && b[k] == b'>' {
                        out.push(QuestLink {
                            kind,
                            index: line[ds..k].to_string(),
                            provided_name: None,
                            range: i..k + 1,
                        });
                        i = k + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// #2810 单元②：把链接标记替换成显示名 →（显示文本, [(显示名在其中的字节偏移, 链接)]）。
/// `name_of` 由调用方按 C# `GetDisplayNameForLink`（`:920-955`）解析：内嵌名 > 查表 > 回退字面量。
pub fn quest_line_with_links(
    line: &str,
    mut name_of: impl FnMut(&QuestLink) -> String,
) -> (String, Vec<(usize, QuestLink)>) {
    let links = quest_line_links(line);
    if links.is_empty() {
        return (line.to_string(), Vec::new());
    }
    let mut out = String::with_capacity(line.len());
    let mut placed = Vec::new();
    let mut cursor = 0usize;
    for link in links {
        out.push_str(&line[cursor..link.range.start]);
        let name = name_of(&link);
        placed.push((out.len(), link.clone()));
        out.push_str(&name);
        cursor = link.range.end;
    }
    out.push_str(&line[cursor..]);
    (out, placed)
}

/// #2810 单元②：C# `GetDisplayNameForLink`（`NPCDialogs.cs:920-955`）——
/// 内嵌名（`[ITEM:1|力量戒指]`）优先；否则查表；查不到回退 `Item {idx}` / `Monster {idx}` / `Npc {idx}`。
///
/// 查表来源（本端无本地物品库、也无 C# 的按需 `RequestItemInfo/RequestMonsterInfo`）：
/// 物品 → `QuestCatalog.item_names`（`UserInformation` 下发的物品名表）；
/// 怪物/NPC → `InfoCache`（#279 `NewMonsterInfo/NewNPCInfo` 缓存）。缺失回退已记入 §7。
pub fn quest_link_display_name(
    link: &QuestLink,
    catalog: &QuestCatalog,
    info: &crate::game::object_state::InfoCache,
) -> String {
    if let Some(n) = link.provided_name.as_deref() {
        if !n.is_empty() {
            return n.to_string();
        }
    }
    let idx = link.index.parse::<i32>().ok();
    let looked_up = match link.kind {
        QuestLinkKind::Monster => idx.and_then(|i| info.monsters.get(&i)).map(|m| {
            if m.game_name.is_empty() {
                m.name.clone()
            } else {
                m.game_name.clone()
            }
        }),
        QuestLinkKind::Npc => idx
            .and_then(|i| info.npcs.get(&(i.max(0) as u32)))
            .map(|n| n.name.clone()),
        QuestLinkKind::Item => idx.and_then(|i| catalog.item_names.get(&i).cloned()),
    };
    looked_up
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| link.kind.fallback_name(&link.index))
}

/// #2810 单元②：链接悬停提示内容（C# `NPCDialog.ShowTooltip`，`NPCDialogs.cs:608-700+`）。
/// 本端按缓存可得的字段给最小对齐集：怪物给等级/经验，物品与 NPC 给名字（C# 的物品走完整
/// `ItemLabel`、怪物另画形象图；本端缺按需请求与本地物品库，差异记入 §7）。
pub fn quest_link_tooltip_lines(
    kind: QuestLinkKind,
    index: &str,
    info: &crate::game::object_state::InfoCache,
) -> Vec<String> {
    let mut lines = Vec::new();
    if kind == QuestLinkKind::Monster {
        if let Some(m) = index
            .parse::<i32>()
            .ok()
            .and_then(|i| info.monsters.get(&i))
        {
            if m.level > 0 {
                lines.push(format!("等级: {}", m.level));
            }
            if m.experience > 0 {
                lines.push(format!("经验: {}", m.experience));
            }
        }
    }
    lines
}

/// #2810 单元③：奖励格悬停物品说明（C# `QuestCell.OnMouseEnter`，`QuestDialogs.cs:1663-1673`）：
/// `new UserItem(Item) { MaxDura = Item.Durability, CurrentDura = Item.Durability }` → `CreateItemLabel`。
/// 本端复用背包的物品 tooltip 行构造（`inventory::item_tooltip_lines`，逐条对齐 C# `MirItemCell`）——
/// 奖励物品的完整 `ItemInfo` 随任务定义下发（#2801 单元③），故类型/耐久/属性/需求/重量价格都能给。
pub fn quest_reward_item_tooltip_lines(item: &mir2_shared::data::item::ItemInfo) -> Vec<String> {
    use crate::game::dialogs::inventory::InvItem;
    let tooltip_item = InvItem {
        item_index: item.index,
        grade: u8::from(item.grade),
        name: item.name.clone(),
        image: item.image,
        count: 1,
        item_type: u8::from(item.item_type),
        shape: item.shape,
        // C# `QuestCell.OnMouseEnter` 把当前/最大耐久都设成 `Item.Durability`
        current_dura: item.durability,
        max_dura: item.durability,
        stats: item.stats.iter().map(|(k, v)| (k as u8, v)).collect(),
        required_type: u8::from(item.required_type),
        required_amount: item.required_amount,
        required_class: item.required_class.bits(),
        required_gender: item.required_gender.bits(),
        soul_bound_id: -1,
        weight: item.weight as u16,
        price: item.price,
        ..Default::default()
    };
    crate::game::dialogs::inventory::item_tooltip_lines(&tooltip_item)
}

/// #2801 单元②：消息区标题行圆点（C# `QuestMessage_AfterDraw` 的 `Prguse[919]`，`:1066-1080`）
#[derive(Component)]
pub struct QuestDetailBullet(pub usize);

/// #2801 单元②：消息区上滚键（C# `upButton` `Prguse2[197/198/199]` @(293,33)，`:489-500`）
#[derive(Component)]
pub struct QuestDetailScrollUp;

/// #2801 单元②：消息区下滚键（C# `downButton` `Prguse2[207/208/209]` @(293,280)，`:502-513`）
#[derive(Component)]
pub struct QuestDetailScrollDown;

/// #2801 单元②：消息区位置条（C# `positionBar` `Prguse2[205/206]` @(293,48)，`:515-526`）
#[derive(Component)]
pub struct QuestDetailPositionBar;

/// #2801 单元③：分享按钮（C# `_shareButton` `Title[616/617/618]` @(40,436)，`:560-575`）
#[derive(Component)]
pub struct QuestDetailShare;

/// #2801 单元③：取消按钮（C# `_cancelButton` `Title[203/204/205]` @(200,436)，`:585-599`）
#[derive(Component)]
pub struct QuestDetailCancel;

/// #2801 单元③：取消确认框（C# `MirMessageBox(AskCancelQuest, YesNo)`，`:590-598`）
#[derive(Component)]
pub struct QuestCancelConfirm;

#[derive(Component)]
pub struct QuestCancelYes;

#[derive(Component)]
pub struct QuestCancelNo;

/// #2801 单元③：奖励区币种图标（C# `BeforeDraw` 的 `Prguse[966/965/2447]`，`:1424-1443`）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QuestRewardIcon {
    Exp,
    Gold,
    Credit,
}

/// #2801 单元③：奖励区币种数值（C# `_expLabel/_goldLabel/_creditLabel`，`:1400-1402`）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QuestRewardValue {
    Exp,
    Gold,
    Credit,
}

/// #2801 单元③：奖励物品格（C# `QuestCell`，`:1650-1745`）；`fixed` 决定底图与排位
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct QuestRewardCell {
    pub fixed: bool,
    pub slot: usize,
}

/// #2801 单元③：奖励区部件标记。**一个枚举一种部件**——Bevy 无法证明多个
/// `&mut Node`/`&mut Visibility` 查询互斥，用单组件分派可免去成对 `Without` 过滤
/// （同 LESSON_Option-Marker分派须配Or过滤 的反面用法：能合并就别拆）。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum QuestRewardPart {
    Icon(QuestRewardIcon),
    Value(QuestRewardValue),
    /// 格底图（固定排 `Prguse[989]` / 可选排选中时 `Prguse[979]`）
    CellBg(QuestRewardCell),
    /// 格内物品图（`LibraryName::Items` 的 `Item.Image`）
    CellItem(QuestRewardCell),
    /// 格内数量（C# `QuestCell.CreateDisposeLabel`，`:1725-1745`）
    CellCount(QuestRewardCell),
}

/// C# `ClientTextKeys.AskCancelQuest`（`Client/Localization/Chinese.json`）
pub const QUEST_CANCEL_ASK: &str = "你确定要取消这个任务吗？";

/// #2801 单元①：任务日志 UI 的对话框状态打包。
/// `quest_log_ui_system` 原本已是 16 个系统参数（Bevy `SystemParam` 上限，
/// 同 `<control.rs>` 的 `ControlQueries`），新增「详情窗状态」必须与 `DialogManager` 打包，
/// 否则参数变 17 个直接编译失败。
#[derive(SystemParam)]
pub struct QuestDialogAccess<'w> {
    /// 打开栈（本函数读 QuestLog 开关、写 QuestDetail 开关）
    pub mgr: ResMut<'w, DialogManager>,
    /// 详情窗当前任务（单元②渲染用；本单元承载"展示哪个任务"的事实源）
    pub detail: ResMut<'w, QuestDetailState>,
}

/// #2810 单元①：详情窗消息区的「滚轮 + 彩色叠加段」打包参数。
/// `quest_detail_ui_system` 已到 Bevy 的 16 参上限（见 `QuestDialogAccess` 注释），
/// 新增叠加段查询必须与既有参数合并。
#[derive(SystemParam)]
pub struct QuestDetailExtras<'w, 's> {
    /// 滚轮（C# `QuestMessage_MouseWheel`，`:1085-1100`）
    pub wheels: MessageReader<'w, 's, MouseWheel>,
    /// #2810 单元②：怪物/NPC 信息缓存（链接换名 + 提示内容，C# `MonsterInfoList/NPCInfoList`）
    pub info: Res<'w, crate::game::object_state::InfoCache>,
    /// #2810 单元②：链接悬停提示（C# `NPCDialog.ShowTooltipForLink`，`NPCDialogs.cs:957-967`）
    pub tooltip: ResMut<'w, crate::ui::tooltip::TooltipState>,
    /// #2810 单元②：光标探针（自动化环境 winit 收不到真实光标，悬停命中走探针）
    pub probe: Res<'w, crate::control::CursorProbe>,
    /// 彩色叠加段（C# `NewColour` 的叠加 `MirLabel`）
    pub segments: Query<
        'w,
        's,
        (
            &'static mut Text,
            &'static mut TextColor,
            &'static mut TextFont,
            &'static mut Node,
            &'static mut Visibility,
            &'static QuestDetailSegment,
        ),
        (
            Without<QuestDetailWidget>,
            Without<QuestDetailLine>,
            Without<QuestDetailBullet>,
            Without<QuestDetailPositionBar>,
        ),
    >,
}

pub struct QuestLogPlugin;

impl Plugin for QuestLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLogState>();
        app.init_resource::<QuestCatalog>();
        app.init_resource::<QuestDetailState>();
        app.add_systems(
            Update,
            quest_log_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_quest_log);
        app.add_systems(OnEnter(AppState::Game), spawn_quest_detail);
        app.add_systems(OnExit(AppState::Game), cleanup_quest_log);
        app.add_systems(
            Update,
            (
                quest_log_ui_system,
                quest_detail_ui_system,
                // 单元③：奖励区渲染 + 取消确认框（独立系统，避开 16 参数上限）
                quest_detail_reward_system,
                quest_detail_confirm_system,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_quest_log(mut commands: Commands, roots: Query<Entity, With<DialogRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_quest_log(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        crate::ui::sprite_ui::ensure_ui_font(&mut fonts, &mut ui_font);
    }
    let font = ui_font.0.clone();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 Prguse[961]（C# QuestDiaryDialog，316x466 @ (192,60)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 961) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        DIARY_POS.0,
        DIARY_POS.1,
        DIARY_SIZE.0,
        DIARY_SIZE.1,
        30,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::QuestLog), QuestLogWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[15]（C# QuestDiaryDialog：Title[15] @(18,9)）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 15) {
            let (iw, ih) = match libs.0.get_image(LibraryName::Title, 15) {
                Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                None => (100.0, 17.0),
            };
            spawn_image(p, h, 18.0, 9.0, iw, ih, 8);
        }
        // 关闭 Prguse2[360-362] @(289,3)
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(QuestLogClose);
        }
        // 底部关闭 Title[193/194/195] @(200,436)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 193),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 194),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 195),
        ) {
            // `Title[193]` 图头 68x25（C# `QuestDialogs.cs:676-686` `_closeButton` 无显式 Size）
            spawn_icon_button(p, n, h, pr, 200.0, 436.0, 68.0, 25.0, 10).insert(QuestLogClose);
        }
        // 已接计数标签（C# `_takenQuestsLabel @(210,7)`；原写死 (18,20) 压住标题栏）
        spawn_label(
            p,
            &cjk,
            "",
            DIARY_COUNT_POS.0,
            DIARY_COUNT_POS.1,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(QuestLogLine(14));
        // 任务行 8 + 详情 6 @(18,40+20i)
        for i in 0..14usize {
            spawn_label(
                p,
                &cjk,
                "",
                18.0,
                40.0 + i as f32 * 20.0,
                12.0,
                Color::WHITE,
                9,
            )
            .insert(QuestLogLine(i));
        }
        // 每行追踪按钮（Text 节点本身作为 Button，C# QuestRow Track）
        for i in 0..8usize {
            spawn_label(
                p,
                &cjk,
                "追踪",
                250.0,
                40.0 + i as f32 * 20.0,
                11.0,
                Color::srgb(0.6, 0.9, 1.0),
                10,
            )
            .insert((Button, QuestLogTrack(i)));
        }
        // 放弃 @(200,285)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
        ) {
            spawn_icon_button(p, n, h, pr, 200.0, 285.0, 76.0, 25.0, 10).insert(QuestLogAbandon);
        }
        // #2535 接受/完成（C# Title[270-272]/[273-275]；初始隐藏，状态机驱动显隐/置灰）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 270),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 271),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 272),
        ) {
            // `Title[270]` 图头 68x25（C# `QuestDialogs.cs:105-114` `_acceptButton` 无显式 Size）
            spawn_icon_button(p, n, h, pr, 20.0, 285.0, 68.0, 25.0, 10)
                .insert((QuestLogAccept, Visibility::Hidden));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 273),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 274),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 275),
        ) {
            // `Title[273]` 图头 68x25（C# `QuestDialogs.cs:123-133` `_finishButton` 无显式 Size）
            spawn_icon_button(p, n, h, pr, 110.0, 285.0, 68.0, 25.0, 10)
                .insert((QuestLogFinish, Visibility::Hidden));
        }
    });
}

// ============================================================================
// #2801 任务详情窗（C# `QuestDetailDialog`，`Client/MirScenes/Dialogs/QuestDialogs.cs:463-628`）
// 坐标/精灵逐条对 C#：
//   面板 `Prguse[960]` 316x466 @(ScreenWidth/2+20, 60) = (532,60)（`QuestDialogs.cs:471-479`）
//   标题 `Title[16]` @(18,9)（`:479-485`）
//   关闭键 `Prguse2[360/361/362]` @(289,3)（`:604-614`，`Click += Hide()` 在 `:614`）
// 打开入口：任务日记行左键（`QuestSingleQuestItem._questLabel.Click` → `DisplayQuestDetails`，
// `QuestDialogs.cs:1925-1955`，左键分支 `:1933-1935`）。`Movable = true`（`:475`）→ 独立 `DialogKind::QuestDetail`
// 独立拖动，不复用 `QuestLog`（复用会被 kind 级拖动/置顶连带）。
// 单元②补消息区（上 `Prguse2[197..199]` @(293,33)、下 `Prguse2[207..209]` @(293,280)、
// 位置条 `Prguse2[205/206]` @(293,48)），单元③补分享/取消按钮与奖励区。
// ============================================================================
// ---------------------------------------------------------------------------
// #2801 单元②：消息区（C# `QuestMessage`，`QuestDialogs.cs:1003-1390`）
// 行模型 = `UpdateQuest` + `AdjustDescription`；翻页 = 上/下键 + 滚轮 + 位置条拖动。
// ---------------------------------------------------------------------------

/// C# `QuestDetailDialog` 传给 `QuestMessage` 的 `lineCount = 16`（`QuestDialogs.cs:528`）
pub const QUEST_MSG_LINE_COUNT: usize = 16;
/// C# `QuestMessage.PosMinY / PosMaxY`（`QuestDialogs.cs:534-535`；面板内相对 y）
pub const QUEST_MSG_POS_MIN_Y: i32 = 46;
pub const QUEST_MSG_POS_MAX_Y: i32 = 261;
/// 消息区原点（C# `Message.Location = new Point(10, 35)`，`:533`）
pub const QUEST_MSG_ORIGIN: (f32, f32) = (10.0, 35.0);
/// 消息区宽（C# `Size = new Size(280, 320)` 的宽，`:532`；`MirLabel` Size 宽 = WordBreak 折行宽）
pub const QUEST_MSG_W: f32 = 280.0;
/// 行高（C# `MirLabel.Size = new Size(Size.Width, 20)`，`:1260`；折行溢出被裁，同 C#）
pub const QUEST_MSG_LINE_H: f32 = 20.0;
/// 行距 15（C# `0 + (i - TopLine) * 15 + adjust`，`:1261`）
pub const QUEST_MSG_LINE_DY: f32 = 15.0;
/// 标题行额外占位 5（C# `adjust += 5`，`:1267`）
pub const QUEST_MSG_TITLE_DY: f32 = 5.0;
/// 标题行缩进 15（C# `title ? 15 : 0`，`:1261`）
pub const QUEST_MSG_TITLE_INDENT: f32 = 15.0;
/// 正文字号：C# `new Font(Settings.FontName, 9F)`（`:530`）。GDI `Font(name, pt)` 在
/// 96dpi 下 ≈ pt×4/3 px（9F≈12px），与本文件任务行标签同口径。
pub const QUEST_MSG_FONT_PX: f32 = 12.0;
/// 标题字号：C# `new Font(Settings.FontName, 10F, FontStyle.Bold)`（`:1244`）。Bevy
/// `TextFont.weight` 仅对可变字重字体生效（SimSun 无可变轴），故以 +1px 近似粗体。
pub const QUEST_MSG_TITLE_FONT_PX: f32 = 13.0;

/// 消息区四个本地化标题（C# `QuestMessage.TaskTitle/ProgressTitle/ReturnTitle/TimeLimitTitle`，
/// `:1023`；取 `Client/Localization/Chinese.json` 的 Tasks/Progress/QuestReturn/TimeLimit）
pub const QUEST_TASK_TITLE: &str = "任务";
pub const QUEST_PROGRESS_TITLE: &str = "进度";
pub const QUEST_RETURN_TITLE: &str = "任务交付";
pub const QUEST_TIME_LIMIT_TITLE: &str = "时间限制";

/// C# `NewText`（`:1242-1251`）：`i == 0` 或行文本命中四个标题之一 → 标题行
/// （标题行：粗体 10F、缩进 15、额外 +5 行距；`i == 0` 另加黄色）
pub fn quest_line_is_title(i: usize, line: &str) -> bool {
    i == 0
        || line == QUEST_TASK_TITLE
        || line == QUEST_PROGRESS_TITLE
        || line == QUEST_RETURN_TITLE
        || line == QUEST_TIME_LIMIT_TITLE
}

/// #2810 单元①：C# `QuestMessage` 的彩色段（`NewColour` 叠加对象，`QuestDialogs.cs:1336-1353`）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLineSegment {
    /// 段文本（`{` 后第一个 `/` 之前的内容，C# `:1321-1323` 的 `values[0]`）
    pub text: String,
    /// 颜色名（`/` 与 `}` 之间，交给 C# `Color.FromName(values[1])`）
    pub color_name: String,
    /// 段首在**去标记后整行**中的字节偏移（叠加标签定位用）
    pub byte_offset: usize,
    /// #2810 单元②：标记在**原行**中的字节范围（与链接标记合并排序用，C# `OrderBy(match.Index)`）
    pub range: std::ops::Range<usize>,
}

/// #2810 单元①：一行文本的标记解析 → （去标记后的整行文本, 彩色段列表）。
///
/// 标记语法逐字对齐 C# `NewText` 的 `private static readonly Regex C = new Regex(@"{(.*?/.*?)}")`
/// （`QuestDialogs.cs:1008`）：`{` 之后遇的第一个 `/` 切文本、其后再遇 `}` 收尾；
/// 段文本非空（`close > i + 1`）才成立——与 #2801 的 `quest_line_display_text` 同判据，两者现共用本函数。
pub fn quest_line_markup(line: &str) -> (String, Vec<QuestLineSegment>) {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut segs = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{' {
            // C# 非贪婪 `{(.*?/.*?)}`：`{` 之后遇的第一个 `/` 切文本，其后再遇 `}` 收尾
            if let Some(slash) = (i + 1..chars.len()).find(|&j| chars[j] == '/') {
                if let Some(close) = (slash + 1..chars.len()).find(|&j| chars[j] == '}') {
                    if close > i + 1 {
                        let text: String = chars[i + 1..slash].iter().collect();
                        let color_name: String = chars[slash + 1..close].iter().collect();
                        // 原行字节范围：char 下标 → 字节下标（多字节字符下必须按字节算）
                        let byte_at = |ci: usize| -> usize {
                            chars[..ci].iter().map(|c| c.len_utf8()).sum::<usize>()
                        };
                        segs.push(QuestLineSegment {
                            byte_offset: out.len(),
                            text: text.clone(),
                            color_name,
                            range: byte_at(i)..byte_at(close + 1),
                        });
                        out.push_str(&text);
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    (out, segs)
}

/// #2801 单元②（#2810 起共用）：去标记后的整行文本（`{文本/颜色}` → `文本`）
pub fn quest_line_display_text(line: &str) -> String {
    quest_line_markup(line).0
}

/// #2810 单元①：彩色段在**折行后**的落位 →（行序, 行内 x 偏移）。
///
/// C# 用 `TextRenderer.MeasureText(前缀 + " ", font, label.Size, TextBoxControl)` 量前缀
/// （`QuestDialogs.cs:1331-1334`），再以 `宽度 - 10` 定位叠加标签；本端复用
/// `text_markup::wrap_text` 的同一套贪心折行 + `est_text_width`（宋体双宽度量，与基础标签同字体同尺寸），
/// 多行时按「段所在可视行 + 行内前缀宽」定位（C# 的盒子量宽在换行场景只返回盒宽，属近似，§7 记录）。
pub fn quest_segment_offset(prefix: &str, size: f32, max_w: f32) -> (usize, f32) {
    let lines = crate::ui::text_markup::wrap_text(prefix, size, max_w);
    let row = lines.len().saturating_sub(1);
    (
        row,
        crate::ui::text_markup::est_text_width(
            lines.last().map(String::as_str).unwrap_or(""),
            size,
        ),
    )
}

/// #2801 单元②：C# `QuestMessage.UpdateQuest` + `AdjustDescription`（`:1142-1213`）→ 行模型。
///
/// 顺序逐条照抄：首行 = 任务名；`Taken && !SameFinishNPC && 有完成描述 && 当前 NPC 是交付 NPC`
/// 时显示完成描述，否则显示描述 + `AdjustDescription`（任务 / 任务交付 / 时间限制 / 进度）。
/// `same_finish_npc` = C# `QuestInfo.SameFinishNPC`（`Shared/Data/ClientData.cs:377`，
/// `NPCIndex == FinishNPCIndex`）。
pub fn quest_message_lines(
    info: &ClientQuestInfo,
    taken: bool,
    task_list: &[String],
    current_npc_at_finish: bool,
    display_progress: bool,
) -> Vec<String> {
    let mut lines = vec![info.name.clone()];
    let same_finish_npc = info.npc_index == info.finish_npc_index;
    if taken && !same_finish_npc && !info.completion_description.is_empty() && current_npc_at_finish
    {
        lines.extend(info.completion_description.iter().cloned());
        return lines;
    }
    lines.extend(info.description.iter().cloned());
    if !info.task_description.is_empty() {
        lines.push(" ".to_string());
        lines.push(QUEST_TASK_TITLE.to_string());
        lines.extend(info.task_description.iter().cloned());
    }
    if !info.return_description.is_empty() {
        lines.push(" ".to_string());
        lines.push(QUEST_RETURN_TITLE.to_string());
        lines.extend(info.return_description.iter().cloned());
    }
    if info.time_limit_in_seconds > 0 {
        lines.push(" ".to_string());
        lines.push(QUEST_TIME_LIMIT_TITLE.to_string());
        lines.push(crate::game::time_format::format_time_span(
            info.time_limit_in_seconds as f64,
        ));
    }
    if taken && !task_list.is_empty() && display_progress {
        lines.push(" ".to_string());
        lines.push(QUEST_PROGRESS_TITLE.to_string());
        lines.extend(task_list.iter().cloned());
    }
    lines
}

/// C# `ScrollUpButton.Click`（`:1042-1050`）：`TopLine <= 0` 不动
pub fn quest_msg_scroll_up(top: usize) -> usize {
    top.saturating_sub(1)
}

/// C# `ScrollDownButton.Click`（`:1052-1060`）：`TopLine + LineCount >= 行数` 不动
pub fn quest_msg_scroll_down(top: usize, len: usize, line_count: usize) -> usize {
    if top + line_count >= len {
        top
    } else {
        top + 1
    }
}

/// C# `QuestMessage_MouseWheel`（`:1082-1098`）：`count = delta / 120`（本端归一到 ±1），
/// 含「末行钳位用 `Count - 1`」这一原版怪癖（逐字照抄，不"修正"）
pub fn quest_msg_wheel_top_line(top: usize, count: i32, len: usize, line_count: usize) -> usize {
    if len <= line_count || count == 0 {
        return top;
    }
    if top == 0 && count >= 0 {
        return top;
    }
    if top + 1 == len && count <= 0 {
        return top;
    }
    let mut t = top as i64 - count as i64;
    if t < 0 {
        t = 0;
    }
    if t + line_count as i64 > len as i64 - 1 {
        t = len as i64 - line_count as i64;
    }
    t.max(0) as usize
}

/// C# `UpdatePositionBar`（`:1120-1140`）的整数区间（`(PosMaxY-PosMinY) / (Count-LineCount)`，
/// 整数除法 = 向下取整，与 C# 同；`len <= line_count` 时返回 0）
fn quest_msg_bar_interval(len: usize, line_count: usize) -> i32 {
    let span = len as i64 - line_count as i64;
    if span <= 0 {
        return 0;
    }
    (QUEST_MSG_POS_MAX_Y - QUEST_MSG_POS_MIN_Y) / span as i32
}

/// C# `UpdatePositionBar`（`:1120-1140`）：`None` = 位置条隐藏（行数不足一页）；
/// 否则返回位置条顶端的**面板内相对 y**
pub fn quest_msg_bar_y(top: usize, len: usize, line_count: usize) -> Option<i32> {
    if len <= line_count {
        return None;
    }
    let interval = quest_msg_bar_interval(len, line_count);
    let y = QUEST_MSG_POS_MIN_Y + top as i32 * interval;
    Some(y.clamp(QUEST_MSG_POS_MIN_Y, QUEST_MSG_POS_MAX_Y))
}

/// C# `PositionBar_OnMoving`（`:1100-1118`）：位置条 y（面板内相对）→ `TopLine`
pub fn quest_msg_top_line_at_bar(y: i32, len: usize, line_count: usize) -> usize {
    if len <= line_count {
        return 0;
    }
    let interval = quest_msg_bar_interval(len, line_count);
    if interval <= 0 {
        return 0;
    }
    let location = y.clamp(QUEST_MSG_POS_MIN_Y, QUEST_MSG_POS_MAX_Y) - QUEST_MSG_POS_MIN_Y;
    (location / interval).max(0) as usize
}

// ---------------------------------------------------------------------------
// #2801 单元③：分享/暂停/取消按钮 + 奖励区（C# `QuestDetailDialog` `:560-599`
// 与 `QuestRewards` `:1396-1530`）
// ---------------------------------------------------------------------------

/// 奖励区原点（C# `Reward = new QuestRewards { Size=(315,130), Location=(5,307) }`，`:544-548`）
pub const QUEST_REWARD_ORIGIN: (f32, f32) = (5.0, 307.0);
/// 奖励物品格尺寸（C# `QuestCell.Size = new Size(32,32)`，`:1657`）
pub const QUEST_REWARD_CELL: f32 = 32.0;
/// 物品格间距 45（C# `i * 45 + 15`，`:1476/1496`）
pub const QUEST_REWARD_CELL_DX: f32 = 45.0;
/// 物品格 x 起点偏移 15（C# `i * 45 + 15`）
pub const QUEST_REWARD_CELL_X0: f32 = 15.0;
/// 固定奖励排 y（C# `Location = new Point(i * 45 + 15, 24)`，`:1476`）
pub const QUEST_REWARD_FIXED_Y: f32 = 24.0;
/// 可选奖励排 y（C# `Location = new Point(i * 45 + 15, 89)`，`:1496`）
pub const QUEST_REWARD_SELECT_Y: f32 = 89.0;
/// 两排各 5 格（C# `static QuestCell[] FixedItems/SelectItems = new QuestCell[5]`，`:1404-1405`）
pub const QUEST_REWARD_SLOTS: usize = 5;

/// C# `QuestRewards.UpdateInterface`/`BeforeDraw`（`:1424-1456`）的横向偏移链：
/// 无经验奖励 → 金币与信用各左移 90；无金币奖励 → 信用再左移 90
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuestRewardOffsets {
    /// 金币列的 x 偏移（加到 C# `100`/`120` 基址上）
    pub gold: f32,
    /// 信用列的 x 偏移（加到 C# `190`/`210` 基址上）
    pub credit: f32,
}

/// 见 [`QuestRewardOffsets`]；逐条照抄 C# 的两级 `-= 90`
pub fn quest_reward_offsets(reward_exp: u32, reward_gold: u32) -> QuestRewardOffsets {
    let mut gold = 0.0f32;
    let mut credit = 0.0f32;
    if reward_exp == 0 {
        gold = -90.0;
        credit -= 90.0;
    }
    if reward_gold == 0 {
        credit -= 90.0;
    }
    QuestRewardOffsets { gold, credit }
}

/// C# `QuestRewards.FilterRewards`（定义 `:1588-1610`）：只保留与玩家性别匹配的奖励物品
/// （`None`/未设位不显示——C# 用 `RequiredGender.HasFlag`，位掩码 0 对任何性别都是 false）。
/// **只在可选排调用**（`:1551-1553`）；固定排不过滤（见 `quest_detail_reward_system`）。
pub fn quest_reward_visible_for_gender(
    item: &mir2_shared::data::item::ItemInfo,
    gender: mir2_shared::enums::MirGender,
) -> bool {
    use mir2_shared::enums::{MirGender, RequiredGender};
    let want = match gender {
        MirGender::Male => RequiredGender::MALE,
        MirGender::Female => RequiredGender::FEMALE,
    };
    item.required_gender.contains(want)
}

/// C# `QuestCell.DrawControl`（`:1690-1696`）：物品图居中偏移 `(40 - 图宽)/2, (32 - 图高)/2`
/// （整数除法，负数向零截断——与 C# 一致）
pub fn quest_reward_item_offset(w: i32, h: i32) -> (f32, f32) {
    (((40 - w) / 2) as f32, ((32 - h) / 2) as f32)
}

/// #2801 单元①：详情窗面板原点（C# `QuestDialogs.cs:476`
/// `Location = new Point(Settings.ScreenWidth / 2 + 20, 60)`；1024/2+20 = 532）
pub fn quest_detail_origin() -> (f32, f32) {
    (UI_SCREEN_W / 2.0 + 20.0, 60.0)
}

fn spawn_quest_detail(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    libs.0.ensure_initialized();
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);
    // 面板 Prguse[960]（316x466）@(532,60)
    let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 960) else {
        return;
    };
    let (px, py) = quest_detail_origin();
    let panel = spawn_panel(&mut commands, bg, px, py, 316.0, 466.0, 31);
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::QuestDetail), QuestDetailWidget));
    commands.entity(panel).with_children(|p| {
        // 标题 Title[16] @(18,9)（55x17；按 .Lib 真实尺寸落，避免拉伸）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 16) {
            let (iw, ih) = match libs.0.get_image(LibraryName::Title, 16) {
                Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                None => (55.0, 17.0),
            };
            spawn_image(p, h, 18.0, 9.0, iw, ih, 8);
        }
        // 关闭键 Prguse2[360/361/362] @(289,3)（24x21）
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 10)
        {
            btn.insert(QuestDetailClose);
        }
        // ===== 消息区（C# `QuestMessage`，`:528-536`）=====
        // 上滚 Prguse2[197/198/199] @(293,33)（C# 显式 Size=(16,14)）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            // `Prguse2[197]/[207]` 图头 12x12（C# 显式 Size 被 `AutoSize=true` 顶掉，见 MirImageControl ctor）
            spawn_icon_button(p, n, h, pr, 293.0, 33.0, 12.0, 12.0, 11).insert(QuestDetailScrollUp);
        }
        // 下滚 Prguse2[207/208/209] @(293,280)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(p, n, h, pr, 293.0, 280.0, 12.0, 12.0, 11)
                .insert(QuestDetailScrollDown);
        }
        // 位置条 Prguse2[205/206] @(293,48) 12x18（C# `Visible=false` 起始；行数不足一页恒隐）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 205),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206),
        ) {
            spawn_icon_button(p, n, h, pr, 293.0, 48.0, 12.0, 18.0, 12)
                .insert((QuestDetailPositionBar, Visibility::Hidden));
        }
        // 16 行标签：宽 280 折行（C# `MirLabel.Size=(Size.Width,20)` + WordBreak），
        // 高 20 裁剪溢出——位置/字号/颜色每帧由 `quest_detail_ui_system` 重算
        for i in 0..QUEST_MSG_LINE_COUNT {
            let (ox, oy) = QUEST_MSG_ORIGIN;
            let y = oy + i as f32 * QUEST_MSG_LINE_DY;
            spawn_label(p, &cjk, "", ox, y, QUEST_MSG_FONT_PX, Color::WHITE, 9).insert((
                QuestDetailLine(i),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(ox),
                    top: Val::Px(y),
                    width: Val::Px(QUEST_MSG_W),
                    height: Val::Px(QUEST_MSG_LINE_H),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ));
            // #2810 单元①：彩色叠加段池（C# `NewColour` 的叠加 `MirLabel`，`:1336-1353`）
            for s in 0..QUEST_MSG_MAX_SEGMENTS {
                spawn_label(p, &cjk, "", ox, y, QUEST_MSG_FONT_PX, Color::WHITE, 10)
                    .insert((QuestDetailSegment { slot: i, seg: s }, Visibility::Hidden));
            }
        }
        // 标题行圆点 Prguse[919]（12x10；初始藏在面板上方，逐帧按标题行落位）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 919) {
            for i in 0..QUEST_MSG_LINE_COUNT {
                spawn_image(p, h.clone(), QUEST_MSG_ORIGIN.0 + 5.0, -60.0, 12.0, 10.0, 8)
                    .insert((QuestDetailBullet(i), Visibility::Hidden));
            }
        }
        // ===== 单元③：分享/暂停/取消（C# `:560-599`）=====
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 616),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 617),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 618),
        ) {
            spawn_icon_button(p, n, h, pr, 40.0, 436.0, 76.0, 25.0, 11).insert(QuestDetailShare);
        }
        // `_pauseButton`：C# 建了控件但 `Visible = false` 且无 Click（`:577-584`，死控件）→
        // 结构上保留（下单元/后续可复用时只改显隐），本端恒隐藏
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 270),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 271),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 272),
        ) {
            // `Title[270]` 图头 68x25（C# `QuestDialogs.cs:568-578` `_pauseButton` 无显式 Size）
            spawn_icon_button(p, n, h, pr, 120.0, 436.0, 68.0, 25.0, 11).insert(Visibility::Hidden);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 203),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 204),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 205),
        ) {
            spawn_icon_button(p, n, h, pr, 200.0, 436.0, 76.0, 25.0, 11).insert(QuestDetailCancel);
        }
        // ===== 单元③：奖励区（C# `QuestRewards` @(5,307) 315x130，`:544-548`）=====
        let (rx, ry) = QUEST_REWARD_ORIGIN;
        // 奖励区标题 Title[17] @(20,66)（68x16）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 17) {
            spawn_image(p, h, rx + 20.0, ry + 66.0, 68.0, 16.0, 8);
        }
        // 币种图标：经验 Prguse[966] 28x13 / 金币 Prguse[965] 16x12 / 信用 Prguse[2447]
        // （本端 Data 的 Prguse.Lib 只有 2447 张（0..2446），2447 越界 → 信用图标拿不到，
        //  按缺失跳过；C# 客户端 Data 版本更全才有该图。数值/偏移链仍按 C# 计算）
        let reward_icons: [(QuestRewardIcon, usize, f32, f32); 3] = [
            (QuestRewardIcon::Exp, 966, 28.0, 13.0),
            (QuestRewardIcon::Gold, 965, 16.0, 12.0),
            (QuestRewardIcon::Credit, 2447, 16.0, 16.0),
        ];
        for (kind, idx, w, h) in reward_icons {
            if let Some(ih) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx) {
                spawn_image(p, ih, rx, ry, w, h, 8)
                    .insert((QuestRewardPart::Icon(kind), Visibility::Hidden));
            }
        }
        // 币种数值（C# `_expLabel/_goldLabel/_creditLabel` 75x20，`:1400-1402/1424-1456`）
        for kind in [
            QuestRewardValue::Exp,
            QuestRewardValue::Gold,
            QuestRewardValue::Credit,
        ] {
            spawn_label(p, &cjk, "", rx, ry, QUEST_MSG_FONT_PX, Color::WHITE, 8)
                .insert((QuestRewardPart::Value(kind), Visibility::Hidden));
        }
        // 物品格：固定排 y=24（`Prguse[989]` 底）/ 可选排 y=89（选中时 `Prguse[979]` 底）
        let bg_fixed = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 989);
        let bg_select = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 979);
        for fixed in [true, false] {
            for slot in 0..QUEST_REWARD_SLOTS {
                let cell = QuestRewardCell { fixed, slot };
                let x = rx + QUEST_REWARD_CELL_X0 + slot as f32 * QUEST_REWARD_CELL_DX;
                let y = ry
                    + if fixed {
                        QUEST_REWARD_FIXED_Y
                    } else {
                        QUEST_REWARD_SELECT_Y
                    };
                let holder = spawn_container(p, x, y, QUEST_REWARD_CELL, QUEST_REWARD_CELL, 8)
                    .insert(cell)
                    .id();
                if !fixed {
                    // 只有可选排响应点击（C# `SelectItems[i].Click`，`:1497-1515`）
                    p.commands_mut().entity(holder).insert(Button);
                }
                let mut cmds = p.commands_mut();
                cmds.entity(holder).with_children(|c| {
                    // 底图按 .Lib 真实尺寸（`Prguse[989]`=40x34 固定框 / `Prguse[979]`=40x41 选中框），
                    // y 偏移照抄 C#：固定 `-1`、选中 `-5`（`:1692-1695`）
                    let (bg, dy) = if fixed {
                        (bg_fixed.clone(), -1.0)
                    } else {
                        (bg_select.clone(), -5.0)
                    };
                    if let Some(bg) = bg {
                        let (w, h) = if fixed { (40.0, 34.0) } else { (40.0, 41.0) };
                        spawn_image(c, bg, 0.0, dy, w, h, 1)
                            .insert((QuestRewardPart::CellBg(cell), Visibility::Hidden));
                    }
                    spawn_image(c, Handle::default(), 0.0, 0.0, 0.0, 0.0, 2)
                        .insert((QuestRewardPart::CellItem(cell), Visibility::Hidden));
                    // #2817：奖励格数量黄字 = C# `QuestCell.CountLabel`，显式 `OutLine = false`
                    // （`QuestDialogs.cs:1721-1728`，与 `MirItemCell.cs:2610-2617` 同款）→ 无描边
                    spawn_label_plain(
                        c,
                        &cjk,
                        "",
                        0.0,
                        0.0,
                        QUEST_MSG_FONT_PX,
                        Color::srgb(1.0, 1.0, 0.0),
                        3,
                    )
                    .insert((QuestRewardPart::CellCount(cell), Visibility::Hidden));
                });
            }
        }
    });

    // 取消任务询问框（C# `MirMessageBox(AskCancelQuest, YesNo)`，`:590-598`）——规格同
    // hero.rs 的 MakeActiveHero 询问框：`Prguse[360]` 456x190 居中 @(284,289)，
    // Yes `Title[206..208]` @(260,157)、No `Title[210..212]` @(360,157)。
    // 挂 `AlwaysVisible`（显隐由 `detail.confirm_cancel` 驱动，同 kind 不随主窗开合强隐）
    if let Some(bg) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 360) {
        let confirm = spawn_panel(&mut commands, bg, 284.0, 289.0, 456.0, 190.0, 41);
        commands.entity(confirm).insert((
            QuestCancelConfirm,
            DialogRoot(DialogKind::QuestDetail),
            crate::game::dialogs::AlwaysVisible,
            Visibility::Hidden,
        ));
        commands.entity(confirm).with_children(|p| {
            spawn_label(
                p,
                &cjk,
                QUEST_CANCEL_ASK,
                35.0,
                35.0,
                QUEST_MSG_FONT_PX,
                Color::WHITE,
                9,
            );
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 206),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 207),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 208),
            ) {
                spawn_icon_button(p, n, h, pr, 260.0, 157.0, 76.0, 25.0, 10).insert(QuestCancelYes);
            }
            if let (Some(n), Some(h), Some(pr)) = (
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 210),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 211),
                load_lib_image(&mut libs, &mut images, LibraryName::Title, 212),
            ) {
                spawn_icon_button(p, n, h, pr, 360.0, 157.0, 76.0, 25.0, 10).insert(QuestCancelNo);
            }
        });
    }
}

/// #2801 单元①②：任务详情窗显隐 + 关闭键（C# `closeButton.Click += Hide()`，`QuestDialogs.cs:611`）
/// + 消息区渲染/翻页（C# `QuestMessage`，`:1003-1390`）。
///
/// 查询两两用 `With`/`Without` 显式隔离（`&mut Visibility`/`&mut Node` 三处共用，
/// Bevy 无法自行证明不相交 → B0001 运行期冲突）。
#[allow(clippy::too_many_arguments)]
fn quest_detail_ui_system(
    mut dialogs: QuestDialogAccess,
    log: Res<QuestLogState>,
    catalog: Res<QuestCatalog>,
    npc: Res<crate::game::dialogs::npc::NpcDialogState>,
    mut widgets: Query<&mut Visibility, With<QuestDetailWidget>>,
    mut lines: Query<
        (
            &mut Text,
            &mut TextColor,
            &mut TextFont,
            &mut Node,
            &QuestDetailLine,
        ),
        Without<QuestDetailWidget>,
    >,
    mut bullets: Query<
        (&mut Node, &mut Visibility, &QuestDetailBullet),
        (
            Without<QuestDetailLine>,
            Without<QuestDetailPositionBar>,
            Without<QuestDetailWidget>,
        ),
    >,
    mut bar: Query<
        (
            &Interaction,
            &mut Node,
            &mut Visibility,
            &QuestDetailPositionBar,
        ),
        (
            Without<QuestDetailLine>,
            Without<QuestDetailBullet>,
            Without<QuestDetailWidget>,
        ),
    >,
    scroll: Query<
        (
            Entity,
            &Interaction,
            Option<&QuestDetailScrollUp>,
            Option<&QuestDetailScrollDown>,
        ),
        Or<(With<QuestDetailScrollUp>, With<QuestDetailScrollDown>)>,
    >,
    actions: Query<
        (
            Entity,
            &Interaction,
            Option<&QuestDetailShare>,
            Option<&QuestDetailCancel>,
        ),
        Or<(With<QuestDetailShare>, With<QuestDetailCancel>)>,
    >,
    close: Query<(Entity, &Interaction), With<QuestDetailClose>>,
    net: Res<NetConnection>,
    // #2810 单元①：滚轮 + 彩色叠加段查询打包（本系统参数已到 Bevy 上限 16）
    mut extras: QuestDetailExtras,
    panels: Query<
        &Node,
        (
            With<QuestDetailWidget>,
            Without<QuestDetailLine>,
            Without<QuestDetailBullet>,
            Without<QuestDetailPositionBar>,
        ),
    >,
    windows: Query<&Window>,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    let open = dialogs.mgr.is_open(DialogKind::QuestDetail);
    for mut vis in widgets.iter_mut() {
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
        let was = prev_inter.insert(e, *inter);
        if *inter == Interaction::Pressed && was != Some(Interaction::Pressed) {
            dialogs.mgr.close(DialogKind::QuestDetail);
            dialogs.detail.confirm_cancel = false;
            tracing::info!("📜 关闭任务详情窗");
        }
    }

    // 分享 / 取消（C# `_shareButton.Click` `:568-575`、`_cancelButton.Click` `:585-599`）
    for (e, inter, is_share, is_cancel) in &actions {
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        let Some(qid) = dialogs.detail.quest_id else {
            continue;
        };
        if is_share.is_some() {
            net.send_packet(&mir2_shared::packets::client::quest::ShareQuest { quest_index: qid });
            tracing::info!("🔗 分享任务 #{}", qid);
        } else if is_cancel.is_some() {
            // C#：弹 YesNo 询问框，Yes 才发 `C.AbandonQuest`（`:590-598`）
            dialogs.detail.confirm_cancel = true;
            tracing::info!("📜 取消任务询问框：任务 #{}", qid);
        }
    }

    // ---- 行模型（C# `QuestMessage.UpdateQuest` + `AdjustDescription`）----
    let info = dialogs
        .detail
        .quest_id
        .and_then(|id| catalog.infos.iter().find(|c| c.index == id));
    let taken_entry = info.and_then(|i| log.quests.iter().find(|q| q.id == i.index));
    let taken = taken_entry.is_some();
    let task_list: &[String] = taken_entry.map(|q| q.tasks.as_slice()).unwrap_or(&[]);
    // C# `QuestListDialog.CurrentNPCID == Quest.QuestInfo.FinishNPCIndex`（`:1151`）——
    // 本端当前 NPC 记在 `NpcDialogState.npc_object_id`（0 = 未开对话）
    let at_finish_npc = match info {
        Some(i) => npc.npc_object_id != 0 && npc.npc_object_id == i.finish_npc_index,
        None => false,
    };
    let all: Vec<String> = match info {
        Some(i) => quest_message_lines(i, taken, task_list, at_finish_npc, true),
        None => Vec::new(),
    };
    let line_count = QUEST_MSG_LINE_COUNT;
    // C# `TopLine` 只在 NewText(resetIndex=true) 归零，其余路径靠各自钳位；
    // 本端每帧统一钳一次，避免换任务/行数变少后越界留下空页
    if dialogs.detail.top_line + line_count > all.len() {
        dialogs.detail.top_line = all.len().saturating_sub(line_count);
    }

    let (ox, oy) = QUEST_MSG_ORIGIN;
    let panel_origin = panels
        .single()
        .map(|n| crate::ui::theme::node_origin(n, quest_detail_origin()))
        .unwrap_or_else(|_| quest_detail_origin());
    // #2810 单元②：光标优先取探针（自动化环境 winit 收不到真实光标，见 #2767）
    let cursor = crate::control::resolve_cursor(
        extras.probe.pos,
        windows.single().ok().and_then(|w| w.cursor_position()),
    );

    // ---- 滚轮（C# `QuestMessage_MouseWheel`，`:1082-1098`；仅光标在消息区内生效）----
    let mut wheel_count = 0i32;
    for ev in extras.wheels.read() {
        // C# `count = e.Delta / MouseWheelScrollDelta`：LineDelta 即行数，PixelDelta 按符号归一
        let c = match ev.unit {
            MouseScrollUnit::Line => ev.y.round() as i32,
            MouseScrollUnit::Pixel => ev.y.signum() as i32,
        };
        if c == 0 {
            continue;
        }
        let inside = cursor
            .map(|cur| {
                cur.x >= panel_origin.0 + ox
                    && cur.x <= panel_origin.0 + ox + QUEST_MSG_W
                    && cur.y >= panel_origin.1 + oy
                    && cur.y <= panel_origin.1 + oy + 320.0
            })
            .unwrap_or(false);
        if inside {
            wheel_count += c;
        }
    }
    if wheel_count != 0 {
        dialogs.detail.top_line =
            quest_msg_wheel_top_line(dialogs.detail.top_line, wheel_count, all.len(), line_count);
    }

    // ---- 上/下滚键（C# `ScrollUpButton.Click` / `ScrollDownButton.Click`）----
    for (e, inter, is_up, is_down) in &scroll {
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        let t = dialogs.detail.top_line;
        if is_up.is_some() {
            dialogs.detail.top_line = quest_msg_scroll_up(t);
        } else if is_down.is_some() {
            dialogs.detail.top_line = quest_msg_scroll_down(t, all.len(), line_count);
        }
    }
    let top = dialogs.detail.top_line;

    // ---- 行渲染（C# `NewText`）：越界槽位清空；`adjust` = 可见区内之前的标题行数 × 5 ----
    let adjust_at = |idx: usize| -> f32 {
        QUEST_MSG_TITLE_DY
            * (top..idx.min(all.len()))
                .filter(|i| quest_line_is_title(*i, &all[*i]))
                .count() as f32
    };
    // #2810 单元①②：可见行的（槽位, 行原点, 字号, 显示文本, 叠加部件）——供叠加段定位
    let mut line_spans: Vec<(usize, f32, f32, f32, String, Vec<QuestOverlayPart>)> = Vec::new();
    for (mut text, mut color, mut font, mut node, line) in &mut lines {
        let idx = top + line.0;
        let (s, parts, is_title, accent) = if idx < all.len() {
            // 标题判定用**原文**（C# `NewText` 拿 `lines[i]` 与四个标题常量比对），
            // 显示文本走 `{文本/颜色}` 去标记 + 链接换名（叠加池渲染彩色段与链接）
            let is_title = quest_line_is_title(idx, &all[idx]);
            let (s, parts) = quest_line_overlays(&all[idx], |link| {
                quest_link_display_name(link, &catalog, &extras.info)
            });
            (s, parts, is_title, idx == 0)
        } else {
            (String::new(), Vec::new(), false, false)
        };
        let top_y = oy + line.0 as f32 * QUEST_MSG_LINE_DY + adjust_at(idx);
        node.top = Val::Px(top_y);
        let left = ox
            + if is_title {
                QUEST_MSG_TITLE_INDENT
            } else {
                0.0
            };
        node.left = Val::Px(left);
        let size = if is_title {
            QUEST_MSG_TITLE_FONT_PX
        } else {
            QUEST_MSG_FONT_PX
        };
        font.font_size = FontSize::Px(size);
        text.0 = s;
        // C# `i == 0` 用 `Color.Yellow`（`:1247-1250`）
        let c = if accent {
            Color::srgb(1.0, 1.0, 0.0)
        } else {
            Color::WHITE
        };
        if color.0 != c {
            color.0 = c;
        }
        line_spans.push((line.0, left, top_y, size, text.0.clone(), parts));
    }

    // ---- #2810 单元①②：叠加部件（彩色段 `NewColour` + 链接 `NewLink`）----
    // C# 对每个 `{文本/颜色}` 段叠加彩色 `MirLabel`（`:1336-1353`），对每个链接叠加青色
    // `MirLabel` 并接 MouseEnter/Leave（`:1355-1382`）。本端用固定池 + 逐帧显隐/落位；
    // 颜色名走 `Color.FromName` 子集（`text_markup::known_color`）：未知名 C# 取到的是
    // 透明色（叠加层不可见），此处直接隐藏——基础白字已含该词，视觉等价。
    let mut hovered_link: Option<(String, Vec<String>, f32, f32)> = None;
    for (mut text, mut color, mut font, mut node, mut vis, seg_marker) in &mut extras.segments {
        let Some((_, left, top_y, size, stripped, parts)) = line_spans
            .iter()
            .find(|(slot, ..)| *slot == seg_marker.slot)
        else {
            *vis = Visibility::Hidden;
            continue;
        };
        let Some(part) = parts.get(seg_marker.seg) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let (part_text, part_offset, col) = match part {
            QuestOverlayPart::Colour {
                text,
                color_name,
                offset,
            } => {
                let Some(c) = crate::ui::text_markup::known_color(color_name) else {
                    *vis = Visibility::Hidden;
                    continue;
                };
                (text.clone(), *offset, c)
            }
            QuestOverlayPart::Link {
                text,
                kind,
                index,
                offset,
            } => {
                // 命中判定用与渲染同一套度量（绝对坐标 = 面板原点 + 行内位置）
                let prefix = stripped.get(..*offset).unwrap_or("");
                let (row, x) = quest_segment_offset(prefix, *size, QUEST_MSG_W);
                let (x0, y0) = (
                    panel_origin.0 + left + x,
                    panel_origin.1 + top_y + row as f32 * (*size * 1.2),
                );
                let (x1, y1) = (
                    x0 + crate::ui::text_markup::est_text_width(text, *size),
                    y0 + *size * 1.2,
                );
                let hovered = cursor
                    .map(|c| c.x >= x0 && c.x <= x1 && c.y >= y0 && c.y <= y1)
                    .unwrap_or(false);
                if hovered {
                    // C# `temp.MouseEnter`：转橙 + `ShowTooltipForLink`（`:1368-1376`）
                    hovered_link = Some((
                        text.clone(),
                        quest_link_tooltip_lines(*kind, index, &extras.info),
                        x0,
                        y1,
                    ));
                    (text.clone(), *offset, Color::srgb(1.0, 0.65, 0.0))
                } else {
                    // C# `NewLink` 初值 `ForeColour = Color.Cyan`（`:1360-1366`）
                    (text.clone(), *offset, Color::srgb(0.0, 1.0, 1.0))
                }
            }
        };
        let prefix = stripped.get(..part_offset).unwrap_or("");
        let (row, x) = quest_segment_offset(prefix, *size, QUEST_MSG_W);
        node.left = Val::Px(left + x);
        // 折行后的行高：bevy 文本默认行高 = 字号 × 1.2（与基础标签同一排版参数）
        node.top = Val::Px(top_y + row as f32 * (*size * 1.2));
        font.font_size = FontSize::Px(*size);
        if text.0 != part_text {
            text.0 = part_text;
        }
        if color.0 != col {
            color.0 = col;
        }
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
    // C# `HideTooltipForLink`（`NPCDialogs.cs:963-967`）：离开链接即清提示
    match hovered_link {
        Some((title, lines, x, y)) => extras.tooltip.update(13, true, title, lines, x, y),
        None => extras
            .tooltip
            .update(13, false, String::new(), Vec::new(), 0.0, 0.0),
    }

    // ---- 标题圆点（C# `QuestMessage_AfterDraw`，`:1066-1080`）----
    for (mut node, mut vis, bullet) in &mut bullets {
        let idx = top + bullet.0;
        let show = idx < all.len() && quest_line_is_title(idx, &all[idx]);
        if show {
            node.left = Val::Px(ox + 5.0);
            node.top = Val::Px(oy + 5.0 + bullet.0 as f32 * QUEST_MSG_LINE_DY + adjust_at(idx));
        }
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // ---- 位置条（C# `UpdatePositionBar` + `PositionBar_OnMoving`）----
    for (inter, mut node, mut vis, _) in &mut bar {
        let Some(y) = quest_msg_bar_y(top, all.len(), line_count) else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Visible;
        if *inter == Interaction::Pressed {
            // 拖动中：条跟手（C# `PositionBar.Location = new Point(x, y)`，不做吸附）
            if let Some(cur) = cursor {
                let raw = (cur.y - panel_origin.1).round() as i32;
                let clamped = raw.clamp(QUEST_MSG_POS_MIN_Y, QUEST_MSG_POS_MAX_Y);
                dialogs.detail.top_line = quest_msg_top_line_at_bar(clamped, all.len(), line_count);
                node.top = Val::Px(clamped as f32);
            }
        } else {
            node.top = Val::Px(y as f32);
        }
    }
}

/// #2801 单元③：取消任务询问框（C# `MirMessageBox(AskCancelQuest, YesNo)`，`:590-598`）。
/// Yes → `C.AbandonQuest{QuestIndex}` + `Hide()`；No → 仅关框。
fn quest_detail_confirm_system(
    mut dialogs: QuestDialogAccess,
    net: Res<NetConnection>,
    mut confirm: Query<
        &mut Visibility,
        (
            With<QuestCancelConfirm>,
            Without<QuestRewardPart>,
            Without<QuestRewardCell>,
        ),
    >,
    yesno: Query<
        (
            Entity,
            &Interaction,
            Option<&QuestCancelYes>,
            Option<&QuestCancelNo>,
        ),
        Or<(With<QuestCancelYes>, With<QuestCancelNo>)>,
    >,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    let open = dialogs.mgr.is_open(DialogKind::QuestDetail) && dialogs.detail.confirm_cancel;
    for mut vis in &mut confirm {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter, is_yes, is_no) in &yesno {
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        if is_yes.is_some() {
            if let Some(qid) = dialogs.detail.quest_id {
                net.send_packet(&mir2_shared::packets::client::quest::AbandonQuest {
                    quest_index: qid,
                });
                tracing::info!("📜 取消任务 #{}（AbandonQuest）", qid);
            }
            dialogs.detail.confirm_cancel = false;
            dialogs.mgr.close(DialogKind::QuestDetail);
        } else if is_no.is_some() {
            dialogs.detail.confirm_cancel = false;
            tracing::info!("📜 取消任务询问框：否");
        }
    }
}

/// #2801 单元③：奖励区渲染 + 可选奖励多选一
/// （C# `QuestRewards.UpdateInterface` `:1420-1530` / `QuestCell.DrawControl` `:1685-1700`）。
///
/// 部件用单个 `QuestRewardPart` 分派（Bevy 无法证明多个 `&mut Node` 查询互斥，
/// 拆组件会逼出成对 `Without` 过滤；见 `QuestRewardPart` 注释）。
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn quest_detail_reward_system(
    mut dialogs: QuestDialogAccess,
    catalog: Res<QuestCatalog>,
    player: Query<&crate::actor::ActorAppearance, With<crate::actor::LocalPlayer>>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mut parts: Query<(
        &QuestRewardPart,
        &mut Visibility,
        &mut Node,
        Option<&mut Text>,
        Option<&mut ImageNode>,
    )>,
    cells: Query<(Entity, &QuestRewardCell, Option<&Interaction>), Without<QuestRewardPart>>,
    // #2810 单元③：奖励格悬停物品说明（C# `QuestCell.OnMouseEnter`）
    mut tooltip: ResMut<crate::ui::tooltip::TooltipState>,
    probe: Res<crate::control::CursorProbe>,
    windows: Query<&Window>,
    panels: Query<
        &Node,
        (
            With<QuestDetailWidget>,
            Without<QuestRewardPart>,
            Without<QuestDetailLine>,
        ),
    >,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    let open = dialogs.mgr.is_open(DialogKind::QuestDetail);
    let info = dialogs
        .detail
        .quest_id
        .and_then(|id| catalog.infos.iter().find(|c| c.index == id));
    // C# `FilterRewards`（`:1330-1350`）用 `MapObject.User.Gender` 过滤
    let gender = player
        .single()
        .map(|a| a.gender)
        .unwrap_or(mir2_shared::enums::MirGender::Male);
    let (rx, ry) = QUEST_REWARD_ORIGIN;
    let (exp, gold, credit) = info
        .map(|i| (i.reward_exp, i.reward_gold, i.reward_credit))
        .unwrap_or((0, 0, 0));
    let offs = quest_reward_offsets(exp, gold);
    // 固定排**不做**性别过滤：C# `UpdateInterface` 的固定排直接用 `quest.RewardsFixedItem`
    // （`:1533-1548`），`FilterRewards` 那一行在 `:1534` 被注释掉了——只有可选排在 `:1551-1553`
    // 过滤。照抄原版：固定排原样显示（含性别不符的物品），且槽位下标不因过滤前移。
    let fixed: Vec<&QuestItemReward> = info
        .map(|i| i.rewards_fixed_item.iter().collect())
        .unwrap_or_default();
    // 可选排：过滤后的显示序 + 原（未过滤）下标——C# `SelectedItemIndex` 用原下标
    let select: Vec<(usize, &QuestItemReward)> = info
        .map(|i| {
            i.rewards_select_item
                .iter()
                .enumerate()
                .filter(|(_, r)| quest_reward_visible_for_gender(&r.item, gender))
                .collect()
        })
        .unwrap_or_default();
    let reward_at = |cell: QuestRewardCell| -> Option<&QuestItemReward> {
        if cell.fixed {
            fixed.get(cell.slot).copied()
        } else {
            select.get(cell.slot).map(|(_, r)| *r)
        }
    };
    let selected_at = |cell: QuestRewardCell| -> bool {
        !cell.fixed
            && select
                .get(cell.slot)
                .map(|(idx, _)| dialogs.detail.selected_reward == Some(*idx))
                .unwrap_or(false)
    };

    for (part, mut vis, mut node, text, image) in &mut parts {
        let show = |b: bool| {
            if open && b {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        };
        match *part {
            QuestRewardPart::Icon(kind) => {
                let (vis_on, x) = match kind {
                    QuestRewardIcon::Exp => (exp > 0, rx + 10.0),
                    QuestRewardIcon::Gold => (gold > 0, rx + 100.0 + offs.gold),
                    QuestRewardIcon::Credit => (credit > 0, rx + 190.0 + offs.credit),
                };
                *vis = show(vis_on);
                node.left = Val::Px(x);
                node.top = Val::Px(ry + 2.0);
            }
            QuestRewardPart::Value(kind) => {
                let (vis_on, x, value) = match kind {
                    QuestRewardValue::Exp => (exp > 0, rx + 40.0, exp),
                    QuestRewardValue::Gold => (gold > 0, rx + 120.0 + offs.gold, gold),
                    QuestRewardValue::Credit => (credit > 0, rx + 210.0 + offs.credit, credit),
                };
                *vis = show(vis_on);
                node.left = Val::Px(x);
                node.top = Val::Px(ry);
                if let Some(mut t) = text {
                    t.0 = if vis_on {
                        value.to_string()
                    } else {
                        String::new()
                    };
                }
            }
            QuestRewardPart::CellBg(cell) => {
                // C#：固定格恒画 `Prguse[989]`；可选格仅在选中时画 `Prguse[979]`（`:1690-1696`）
                let has = reward_at(cell).is_some();
                let on = has && (cell.fixed || selected_at(cell));
                *vis = show(on);
            }
            QuestRewardPart::CellItem(cell) => {
                let Some(r) = reward_at(cell) else {
                    *vis = Visibility::Hidden;
                    continue;
                };
                let idx = r.item.image as usize;
                let Some(handle) = crate::ui::sprite_ui::ui_image(
                    &mut libs,
                    &mut images,
                    &mut cache,
                    LibraryName::Items,
                    idx,
                ) else {
                    *vis = Visibility::Hidden;
                    continue;
                };
                let (w, h) = libs
                    .0
                    .get_image(LibraryName::Items, idx)
                    .map(|i| (i.width as i32, i.height as i32))
                    .unwrap_or((0, 0));
                let (ox, oy) = quest_reward_item_offset(w, h);
                if let Some(mut img) = image {
                    if img.image != handle {
                        img.image = handle;
                    }
                }
                node.left = Val::Px(ox);
                node.top = Val::Px(oy);
                node.width = Val::Px(w.max(0) as f32);
                node.height = Val::Px(h.max(0) as f32);
                *vis = show(true);
            }
            QuestRewardPart::CellCount(cell) => {
                let count = reward_at(cell).map(|r| r.count).unwrap_or(0);
                let on = count > 1;
                *vis = show(on);
                if let Some(mut t) = text {
                    // C# `Count.ToString("###0")`（`QuestCell.CreateDisposeLabel`，`:1743`）
                    t.0 = if on { count.to_string() } else { String::new() };
                }
                // C# 用标签实测宽度贴右下角（`:1743`：`Size.Width - 宽 + 8, Size.Height - 高`）；
                // Bevy 无法同帧量文字宽 → 按两位数近似贴右下
                node.left = Val::Px(QUEST_REWARD_CELL - 2.0);
                node.top = Val::Px(QUEST_REWARD_CELL - 13.0);
            }
        }
    }

    // 可选排点击 = 多选一（C# `SelectItems[i].Click`，`:1497-1515`；其余格取消选中）
    for (e, cell, inter) in &cells {
        let Some(inter) = inter else {
            continue;
        };
        let was = prev_inter.insert(e, *inter);
        if !(*inter == Interaction::Pressed && was != Some(Interaction::Pressed)) {
            continue;
        }
        let Some((idx, r)) = select.get(cell.slot) else {
            continue;
        };
        dialogs.detail.selected_reward = Some(*idx);
        tracing::info!(
            "🎁 选择奖励：{}（未过滤下标 {}）",
            reward_item_display_with_catalog(&catalog, r),
            idx
        );
    }

    // #2810 单元③：奖励格悬停物品说明（C# `QuestCell.OnMouseEnter/OnMouseLeave`，`:1663-1683`）
    // `CreateItemLabel(new UserItem(Item){MaxDura=CurrentDura=Item.Durability})` → 本端复用背包
    // 的物品 tooltip 行构造；光标取 `resolve_cursor`（探针优先，自动化可驱动）。
    let cursor = crate::control::resolve_cursor(
        probe.pos,
        windows.single().ok().and_then(|w| w.cursor_position()),
    );
    let origin = panels
        .single()
        .map(|n| crate::ui::theme::node_origin(n, quest_detail_origin()))
        .unwrap_or_else(|_| quest_detail_origin());
    let mut hovered: Option<(String, Vec<String>)> = None;
    if let Some(c) = cursor {
        for (_, cell, _) in &cells {
            let x = origin.0 + QUEST_REWARD_CELL_X0 + cell.slot as f32 * QUEST_REWARD_CELL_DX;
            let y = origin.1
                + if cell.fixed {
                    QUEST_REWARD_FIXED_Y
                } else {
                    QUEST_REWARD_SELECT_Y
                };
            if c.x >= x && c.x <= x + QUEST_REWARD_CELL && c.y >= y && c.y <= y + QUEST_REWARD_CELL
            {
                if let Some(r) = reward_at(*cell) {
                    hovered = Some((
                        r.item.name.clone(),
                        quest_reward_item_tooltip_lines(&r.item),
                    ));
                }
                break;
            }
        }
    }
    match (hovered, cursor) {
        (Some((title, lines)), Some(c)) => tooltip.update(14, true, title, lines, c.x, c.y),
        _ => tooltip.update(14, false, String::new(), Vec::new(), 0.0, 0.0),
    }
}

/// #2801 单元③：奖励物品显示名。C# `QuestItemReward.Item.Name` 随任务定义下发
/// （`Shared/Data/SharedData.cs:77`）；名缺失时回退 `UserInformation` 的物品名表，
/// 再回退 `物品#索引`（旧实现的兜底路径保留）。
pub fn reward_item_display(r: &QuestItemReward) -> String {
    let name = if r.item.name.is_empty() {
        format!("物品#{}", r.item.index)
    } else {
        r.item.name.clone()
    };
    format!("{name}×{}", r.count)
}

/// #2801 单元③：奖励名称回退表（`RewardsFixedItem/RewardsSelectItem` 显示用）。
/// 名字优先取随包下发的 `ItemInfo.Name`，否则查目录物品名表。
pub fn reward_item_display_with_catalog(catalog: &QuestCatalog, r: &QuestItemReward) -> String {
    if !r.item.name.is_empty() {
        return reward_item_display(r);
    }
    let name = catalog
        .item_names
        .get(&r.item.index)
        .cloned()
        .unwrap_or_else(|| format!("物品#{}", r.item.index));
    format!("{name}×{}", r.count)
}

/// 显隐 + 渲染 + 选择 + 接受/完成/放弃（#2535）
#[allow(clippy::too_many_arguments)]
/// 任务行命中矩形（面板原点 ox/oy + 相对坐标；i 0..8）
fn quest_log_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    // 宽度=右界−左界（500−218）：旧实现误把绝对右界当宽度，命中带右扩 18px
    (ox + 18.0, oy + 40.0 + i as f32 * 20.0, 282.0, 18.0)
}

fn quest_log_ui_system(
    // #2801：mgr + detail 打包（见 `QuestDialogAccess`；参数已达 Bevy 上限 16）
    dialogs: QuestDialogAccess,
    mut state: ResMut<QuestLogState>,
    catalog: Res<QuestCatalog>,
    // #2633 批次4 步7：level→`Progression`、class→`ActorAppearance`（HudState 已于步9 删除）；
    // 实体缺失按 HudState 默认（level=1/class=0）
    player_q: Query<
        (
            &crate::game::player_state::Progression,
            &crate::actor::ActorAppearance,
        ),
        With<crate::actor::LocalPlayer>,
    >,
    // #2631：追踪状态只读（渲染追踪/取消按钮）；切换改发 ToggleQuestTracking 由 quest_tracking 处理
    tracking: Res<crate::game::dialogs::quest_tracking::QuestTrackingState>,
    mut toggle_tracking: MessageWriter<crate::game::dialogs::quest_tracking::ToggleQuestTracking>,
    net: Res<NetConnection>,
    // 必须限定 `Or<(…)>`：`Option<&Marker>` 不是过滤条件，裸查询会匹配「全部带
    // Interaction 的实体」（含接受/完成/追踪按钮），下面第一段循环就会替它们消费
    // `prev_inter` 的按下边沿 → 后续循环里 `edge()` 恒为 false，接受/完成键永不触发。
    // （同 LESSON_Option-Marker分派须配Or过滤 / b22 商城付款复选框踩坑）
    close: Query<
        (
            Entity,
            &Interaction,
            Option<&QuestLogClose>,
            Option<&QuestLogAbandon>,
        ),
        Or<(With<QuestLogClose>, With<QuestLogAbandon>)>,
    >,
    mouse: Res<ButtonInput<MouseButton>>,
    ui: (Query<&Window>, Query<&Node, With<QuestLogWidget>>),
    // #1290：Bevy B0001——多个 &mut Text/Visibility Query 需用 Without 隔离
    mut widgets: Query<
        &mut Visibility,
        (
            With<QuestLogWidget>,
            Without<QuestLogAccept>,
            Without<QuestLogFinish>,
        ),
    >,
    mut lines: Query<(&mut Text, &mut TextColor, &QuestLogLine), Without<QuestLogTrack>>,
    mut track_btns: Query<(Entity, &Interaction, &mut Text, &QuestLogTrack), Without<QuestLogLine>>,
    mut accept_btns: Query<
        (Entity, &Interaction, &mut ImageNode, &mut Visibility),
        (With<QuestLogAccept>, Without<QuestLogFinish>),
    >,
    mut finish_btns: Query<
        (Entity, &Interaction, &mut ImageNode, &mut Visibility),
        (With<QuestLogFinish>, Without<QuestLogAccept>),
    >,
    mut prev_inter: Local<std::collections::HashMap<Entity, Interaction>>,
) {
    // #2801：打包参数解到局部名，函数体其余部分保持原样
    let QuestDialogAccess {
        mut mgr,
        mut detail,
    } = dialogs;
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut std::collections::HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }
    let open = mgr.is_open(DialogKind::QuestLog);
    for mut vis in widgets.iter_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !open {
        return;
    }
    for (e, inter, is_close, is_abandon) in &close {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if is_close.is_some() {
            mgr.close(DialogKind::QuestLog);
        } else if is_abandon.is_some() {
            if let Some(i) = state.selected {
                let q = state.quests[i].clone();
                net.send_packet(&mir2_shared::packets::client::quest::AbandonQuest {
                    quest_index: q.id,
                });
                state.quests.remove(i);
                state.selected = None;
                state.selected_reward = None;
                state.message = format!("已放弃任务 {}", q.name);
                tracing::info!("📜 放弃任务 {}", q.name);
            } else {
                state.message = "请先选中一个任务".to_string();
            }
        }
    }

    // #2535 可接任务段（已接在前、可接在后）
    // #2633 批次4 步7：level/class 读组件；实体缺失按 HudState 默认（level=1/class=0）
    let (me_level, me_class) = player_q
        .single()
        .map(|(p, a)| (p.level, a.class as u8))
        .unwrap_or((1, 0));
    let avail = available_quests(&catalog, &state, me_level, me_class);
    // #2535 子批2：日记行模型（组头+展开组内已接+可接，前 8 行入视窗）
    let groups = diary_groups(&state.quests, &catalog.infos);
    let diary = diary_rows(
        &state.quests,
        &catalog.infos,
        avail.len(),
        &state.expanded_groups,
    );
    // 当前选中任务的定义（已接行 → 目录按 id 反查；可接行 → avail 下标）
    let sel_info: Option<&ClientQuestInfo> = state
        .selected
        .and_then(|i| state.quests.get(i))
        .and_then(|q| catalog.infos.iter().find(|c| c.index == q.id))
        .or_else(|| state.selected_avail.and_then(|i| avail.get(i)).copied());

    for (mut text, mut color, line) in &mut lines {
        let (s, c) = match line.0 {
            // 行 0-7：#2535 子批2 日记行模型——组头（展开/收起）/已接（展开组内）/可接
            i if i < 8 => match diary.get(i) {
                Some(DiaryRow::Header(gi)) => {
                    let (group, _) = &groups[*gi];
                    (
                        format!(
                            "{} {}",
                            if group_expanded(&state.expanded_groups, group) {
                                "▼"
                            } else {
                                "▶"
                            },
                            if group.is_empty() {
                                "未分组"
                            } else {
                                group.as_str()
                            }
                        ),
                        // C# 组头 LimeGreen（QuestGroupQuestItem._groupLabel）
                        Color::srgb(0.2, 0.8, 0.2),
                    )
                }
                Some(DiaryRow::Quest(qi)) => {
                    let q = &state.quests[*qi];
                    let info = catalog.infos.iter().find(|c| c.index == q.id);
                    // C# L1900/1919 颜色优先级：低级任务灰 > 新任务黄 > 白（完成绿为沿用）
                    let low_level = info
                        .map(|c| (me_level as i32 - c.min_level_needed) > 10)
                        .unwrap_or(false);
                    let c = if low_level {
                        Color::srgb(0.5, 0.5, 0.5)
                    } else if q.is_new {
                        Color::srgb(1.0, 0.9, 0.3)
                    } else if q.completed {
                        Color::srgb(0.5, 1.0, 0.5)
                    } else {
                        Color::WHITE
                    };
                    // C# L1916 "{0,-4} {1}"：等级 + 名称；状态后缀（完成/进行中）
                    (
                        format!(
                            "Lv{} {}（{}）",
                            info.map(|c| c.min_level_needed).unwrap_or(0),
                            q.name,
                            if q.completed { "完成" } else { "进行中" }
                        ),
                        c,
                    )
                }
                Some(DiaryRow::Avail(k)) => (
                    format!("{}: {}（可接）", avail[*k].index, avail[*k].name),
                    Color::srgb(1.0, 0.9, 0.4),
                ),
                None => (String::new(), Color::WHITE),
            },
            8 => (
                if sel_info.is_some() {
                    format!("详情: {}", sel_info.map(|i| i.name.as_str()).unwrap_or(""))
                } else {
                    "点击任务行查看详情".to_string()
                },
                Color::WHITE,
            ),
            9 => (
                match state.selected.and_then(|i| state.quests.get(i)) {
                    Some(q) => q.tasks.join(" / "),
                    None => sel_info
                        .map(|i| i.task_description.join(" / "))
                        .unwrap_or_default(),
                },
                Color::WHITE,
            ),
            // #2535 奖励区（C# QuestRewards.UpdateRewards）
            10 => (
                match sel_info {
                    Some(i) => format!(
                        "奖励: 金币{} 经验{} 信用{}",
                        i.reward_gold, i.reward_exp, i.reward_credit
                    ),
                    None => String::new(),
                },
                Color::srgb(1.0, 0.95, 0.6),
            ),
            11 => (
                match sel_info {
                    Some(i) if !i.rewards_fixed_item.is_empty() => format!(
                        "固定: {}",
                        i.rewards_fixed_item
                            .iter()
                            .map(|r| reward_item_display_with_catalog(&catalog, r))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    _ => String::new(),
                },
                Color::srgb(1.0, 0.95, 0.6),
            ),
            12 => (
                match sel_info {
                    Some(i) if !i.rewards_select_item.is_empty() => format!(
                        "可选: {}",
                        i.rewards_select_item
                            .iter()
                            .enumerate()
                            .map(|(k, r)| {
                                let s = reward_item_display_with_catalog(&catalog, r);
                                if state.selected_reward == Some(k) {
                                    format!("【{}】", s)
                                } else {
                                    format!("{}{}", "①②③④⑤⑥⑦⑧⑨".chars().nth(k).unwrap_or('·'), s)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("  ")
                    ),
                    _ => String::new(),
                },
                Color::srgb(1.0, 0.95, 0.6),
            ),
            13 => (state.message.clone(), Color::srgb(0.8, 0.9, 1.0)),
            // #2535 子批2：已接计数（C# _takenQuestsLabel，中文资源 "任务：{0}/{1}"）
            14 => (
                format!("任务：{}/{}", state.quests.len(), MAX_CONCURRENT_QUESTS),
                Color::srgb(0.9, 0.95, 1.0),
            ),
            _ => (String::new(), Color::WHITE),
        };
        text.0 = s;
        if color.0 != c {
            color.0 = c;
        }
    }
    // #2535 子批2：行点击——组头展开/收起（C# ChangeExpand）、任务选中（DeselectQuests 单选）
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = ui.0.single() {
            if let Some(cursor) = window.cursor_position() {
                let (ox, oy) =
                    ui.1.single()
                        .map(|n| crate::ui::theme::node_origin(n, DIARY_POS))
                        .unwrap_or(DIARY_POS);
                for i in 0..8usize {
                    let (rx, ry, rw, rh) = quest_log_row_rect(i, ox, oy);
                    if cursor.x >= rx
                        && cursor.x <= rx + rw
                        && cursor.y >= ry
                        && cursor.y <= ry + rh
                    {
                        match diary.get(i) {
                            Some(DiaryRow::Header(gi)) => {
                                let group = groups[*gi].0.clone();
                                state.expanded_groups =
                                    toggle_group(&groups, &state.expanded_groups, &group);
                                state.message = format!(
                                    "{}分组 {}",
                                    if group_expanded(&state.expanded_groups, &group) {
                                        "展开"
                                    } else {
                                        "收起"
                                    },
                                    group
                                );
                            }
                            Some(DiaryRow::Quest(qi)) => {
                                state.selected = Some(*qi);
                                state.selected_avail = None;
                                state.selected_reward = None;
                                // #2801 单元①：C# 任务日记行左键 = 打开任务详情窗
                                // （`QuestSingleQuestItem._questLabel.Click` → `DisplayQuestDetails`，
                                //  `QuestDialogs.cs:1925-1955` 左键分支 `:1933-1935`；
                                //  右键才是追踪开关 `:1936-1951`）
                                let qid = state.quests[*qi].id;
                                detail.quest_id = Some(qid);
                                // C# `DisplayQuestDetails` → `Message.UpdateQuest` →
                                // `NewText(resetIndex: true)` 把 TopLine 归零（`:1130-1136`）
                                detail.top_line = 0;
                                // C# `Reward.UpdateRewards` → `CleanRewards`：
                                // `SelectedItemIndex = -1`、`SelectedItem = null`（`:1451-1462`）
                                detail.selected_reward = None;
                                detail.confirm_cancel = false;
                                mgr.open(DialogKind::QuestDetail);
                                tracing::info!(
                                    "📜 打开任务详情: {}（任务 {}）",
                                    state.quests[*qi].name,
                                    qid
                                );
                            }
                            Some(DiaryRow::Avail(k)) => {
                                state.selected = None;
                                state.selected_avail = Some(*k);
                                state.selected_reward = None;
                                tracing::info!("📜 选中可接任务: {}", avail[*k].name);
                            }
                            None => {}
                        }
                        break;
                    }
                }
                // #2535 可选奖励点击分段选择（行 12，横向等分；x 上限避开放弃按钮）
                if let Some(info) = sel_info {
                    if !info.rewards_select_item.is_empty() {
                        let y = oy + 40.0 + 12.0 * 20.0;
                        if cursor.x >= ox + 18.0
                            && cursor.x <= ox + 198.0
                            && cursor.y >= y
                            && cursor.y <= y + 18.0
                        {
                            let n = info.rewards_select_item.len() as f32;
                            let k = (((cursor.x - (ox + 18.0)) / (180.0 / n)) as usize)
                                .min(info.rewards_select_item.len() - 1);
                            state.selected_reward = Some(k);
                        }
                    }
                }
            }
        }
    }
    // 追踪按钮：标签（追踪/取消）+ 点击切换（C# QuestRow Track，上限 5）；
    // #2535 子批2：行模型映射——仅任务行显示，组头/可接行/空行置空
    for (e, inter, mut text, track) in &mut track_btns {
        let quest = diary
            .get(track.0)
            .and_then(|r| match r {
                DiaryRow::Quest(qi) => state.quests.get(*qi),
                _ => None,
            })
            .cloned();
        let tracked = quest
            .as_ref()
            .map(|q| tracking.is_tracked(q.id))
            .unwrap_or(false);
        text.0 = match &quest {
            Some(_) if tracked => "取消".to_string(),
            Some(_) => "追踪".to_string(),
            None => String::new(),
        };
        if edge(e, inter, &mut prev_inter) {
            if let Some(q) = quest {
                // #2631：toggle + save 归 quest_tracking 处理（数据所有权），这里只发 Message。
                // toggle() 返回值恒等于「点击前未追踪」，故 now_tracked = !tracked（只读推导，
                // 反馈文案与旧直写版完全一致）。
                let now_tracked = !tracked;
                toggle_tracking.write(crate::game::dialogs::quest_tracking::ToggleQuestTracking {
                    quest_id: q.id,
                });
                state.message = if now_tracked {
                    format!("已追踪任务 {}", q.name)
                } else {
                    format!("取消追踪任务 {}", q.name)
                };
                tracing::info!(
                    "📌 任务追踪 {}: {}",
                    if now_tracked { "开启" } else { "关闭" },
                    q.name
                );
            }
        }
    }
    // #2535 接受按钮（可接行选中才显示；本列表只列 Accept 行，故显示即可点）
    for (e, inter, mut img, mut vis) in &mut accept_btns {
        let show = state.selected_avail.is_some()
            && state
                .selected_avail
                .map(|i| i < avail.len())
                .unwrap_or(false);
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        img.color = Color::WHITE;
        if show && edge(e, inter, &mut prev_inter) {
            if let Some(i) = state.selected_avail {
                if let Some(info) = avail.get(i) {
                    net.send_packet(&mir2_shared::packets::client::quest::AcceptQuest {
                        npc_index: info.npc_index,
                        quest_index: info.index,
                    });
                    state.message = format!("已请求接受任务 {}", info.name);
                    tracing::info!(
                        "📜 接受任务 #{} {}（NPC {}）",
                        info.index,
                        info.name,
                        info.npc_index
                    );
                    state.selected_avail = None;
                }
            }
        }
    }
    // #2535 完成按钮（已接行选中才显示；未完成置灰）
    for (e, inter, mut img, mut vis) in &mut finish_btns {
        let sel = state.selected.and_then(|i| state.quests.get(i)).cloned();
        let show = sel.is_some();
        let enabled = sel.as_ref().map(|q| q.completed).unwrap_or(false);
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        img.color = if enabled {
            Color::WHITE
        } else {
            Color::srgb(0.45, 0.45, 0.45)
        };
        if show && enabled && edge(e, inter, &mut prev_inter) {
            let q = sel.unwrap();
            let select_rewards: &[QuestItemReward] = catalog
                .infos
                .iter()
                .find(|c| c.index == q.id)
                .map(|c| c.rewards_select_item.as_slice())
                .unwrap_or(&[]);
            match finish_selected_index(select_rewards, state.selected_reward) {
                Ok(selected_item_index) => {
                    net.send_packet(&mir2_shared::packets::client::quest::FinishQuest {
                        quest_index: q.id,
                        selected_item_index,
                    });
                    state.message = format!("已请求交付任务 {}", q.name);
                    tracing::info!(
                        "📜 交付任务 #{}（可选奖励下标 {}）",
                        q.id,
                        selected_item_index
                    );
                    state.selected = None;
                    state.selected_reward = None;
                }
                Err(msg) => {
                    // C# MirMessageBox(YouMustSelectRewardItem)
                    state.message = msg.to_string();
                    tracing::info!("📜 交付任务 #{} 被阻止: {}", q.id, msg);
                }
            }
        }
    }
}

/// 消费服务端任务事件（网络层只广播 ServerEvent）
fn quest_log_server_events(
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
    mut quest_log: ResMut<QuestLogState>,
    mut catalog: ResMut<QuestCatalog>,
) {
    use crate::network::server_event::ServerEvent;
    for ev in events.read() {
        match ev {
            ServerEvent::QuestChanged { entry } => {
                // C# 语义：只更新进度，移除由 CompleteQuest 负责
                if let Some(e) = quest_log.quests.iter_mut().find(|q| q.id == entry.id) {
                    *e = entry.clone();
                } else {
                    quest_log.quests.push(entry.clone());
                }
                quest_log.message = format!(
                    "任务更新: {}",
                    quest_log
                        .quests
                        .last()
                        .map(|q| q.name.clone())
                        .unwrap_or_default()
                );
            }
            ServerEvent::QuestInfo { info } => {
                // #2535：定义入目录（NewQuestInfo ≠ 接受任务；登录全量下发曾误写入日志）
                upsert_catalog_info(&mut catalog.infos, info);
                tracing::info!("📜 任务定义: #{} {}", info.index, info.name);
            }
            ServerEvent::QuestShared { quest_id } => {
                // #260：共享任务提示
                quest_log.message = format!("收到共享任务 #{}", quest_id);
                tracing::info!("🔗 共享任务 #{}", quest_id);
            }
            ServerEvent::QuestCompleted { id } => {
                quest_log.quests.retain(|q| q.id != *id);
                // #2535：会话内已完成 → 从可接段隐藏（Repeatable 除外按 C# 语义本就不该隐藏，
                // 但服务端未同步历史完成列表，交由服务端校验兜底）
                catalog.completed.insert(*id);
                quest_log.message = format!("任务 {} 完成！", id);
            }
            ServerEvent::UserInformation { item_names, .. } => {
                // #2535：物品名表（奖励显示用；与 guild.rs 同一事件多读无冲突）
                for (idx, name) in item_names {
                    catalog.item_names.insert(*idx, name.clone());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    /// #2801 单元①：详情窗面板原点 = C# `Settings.ScreenWidth / 2 + 20, 60`
    /// （`QuestDialogs.cs:476`；1024/2+20 = 532，与任务日记窗 961 @(200,60) 同 y）
    #[test]
    fn quest_detail_origin_matches_csharp_anchor() {
        assert_eq!(quest_detail_origin(), (532.0, 60.0));
    }

    /// #2801 单元①：详情窗显隐只由 `DialogManager` 决定；关闭键把窗口移出管理栈
    /// （C# `closeButton.Click += Hide()`，`QuestDialogs.cs:611`）
    #[test]
    fn quest_detail_visibility_follows_manager_and_close_button() {
        let mut world = World::new();
        world.insert_resource(DialogManager::default());
        // 单元②接入消息区后，本系统还需任务状态/目录/当前 NPC/滚轮消息
        world.insert_resource(QuestDetailState::default());
        world.insert_resource(QuestLogState::default());
        world.insert_resource(QuestCatalog::default());
        world.insert_resource(crate::game::dialogs::npc::NpcDialogState::default());
        world.insert_resource(crate::network::NetConnection::default());
        world.init_resource::<bevy::ecs::message::Messages<MouseWheel>>();
        // #2810 单元②：叠加段/链接悬停所需资源（与 QuestDetailExtras 字段一一对应）
        world.insert_resource(crate::game::object_state::InfoCache::default());
        world.insert_resource(crate::ui::tooltip::TooltipState::default());
        world.insert_resource(crate::control::CursorProbe { pos: None });
        let root = world
            .spawn((
                QuestDetailWidget,
                DialogRoot(DialogKind::QuestDetail),
                Visibility::Visible,
            ))
            .id();
        let close = world.spawn((QuestDetailClose, Interaction::None)).id();

        // 未 open → 根隐藏（开窗前不得渲染；由 enforce 兜底前的同帧门控保证）
        world
            .run_system_once(quest_detail_ui_system)
            .expect("详情窗 UI 系统应运行");
        assert_eq!(
            world.get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "未打开时详情窗根必须隐藏"
        );

        // open → 显示（打开入口：日记行左键 / RPC）
        world
            .resource_mut::<DialogManager>()
            .open(DialogKind::QuestDetail);
        world
            .run_system_once(quest_detail_ui_system)
            .expect("详情窗 UI 系统应可重复运行");
        assert_eq!(
            world.get::<Visibility>(root),
            Some(&Visibility::Visible),
            "打开后详情窗根必须可见"
        );

        // 关闭键按下 → 移出管理栈，下一帧隐藏
        *world.get_mut::<Interaction>(close).unwrap() = Interaction::Pressed;
        world
            .run_system_once(quest_detail_ui_system)
            .expect("关闭键分支应可运行");
        assert!(
            !world
                .resource::<DialogManager>()
                .is_open(DialogKind::QuestDetail),
            "关闭键必须把详情窗移出管理栈"
        );
        world
            .run_system_once(quest_detail_ui_system)
            .expect("详情窗 UI 系统应可重复运行");
        assert_eq!(
            world.get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "关闭后详情窗根必须隐藏"
        );
    }

    /// #2801 单元①附带修复：`close` 宽查询会替其它按钮消费按下边沿，导致
    /// 「接受」/「完成」键永不触发（实机表现：按下态精灵切换，但无 AcceptQuest 包）。
    /// 修复 = 该查询加 `Or<(With<QuestLogClose>, With<QuestLogAbandon>)>` 过滤。
    #[test]
    fn accept_button_fires_despite_close_query() {
        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open(DialogKind::QuestLog);
        world.insert_resource(mgr);
        world.insert_resource(QuestDetailState::default());
        world.insert_resource(QuestCatalog {
            infos: vec![info(
                1,
                1,
                mir2_shared::enums::RequiredClass::from_bits_truncate(0),
            )],
            ..Default::default()
        });
        world.insert_resource(QuestLogState {
            selected_avail: Some(0),
            ..Default::default()
        });
        world.insert_resource(crate::network::NetConnection::default());
        world.init_resource::<crate::game::dialogs::quest_tracking::QuestTrackingState>();
        world.init_resource::<
            bevy::ecs::message::Messages<
                crate::game::dialogs::quest_tracking::ToggleQuestTracking,
            >,
        >();
        world.insert_resource(ButtonInput::<MouseButton>::default());

        // 接受键：本帧刚被按下（Interaction=Pressed）
        world.spawn((
            Button,
            QuestLogAccept,
            Interaction::Pressed,
            Node::default(),
            ImageNode::default(),
            Visibility::Visible,
        ));
        // 任务日记关闭键（同帧 Hovered）：修复前它会替接受键消费边沿
        world.spawn((
            Button,
            QuestLogClose,
            Interaction::Hovered,
            Node::default(),
            ImageNode::default(),
        ));

        world
            .run_system_once(quest_log_ui_system)
            .expect("任务日志 UI 系统应运行");

        assert_eq!(
            world.resource::<QuestLogState>().message,
            "已请求接受任务 任务1",
            "接受键按下必须发出 AcceptQuest（修复前边沿被 close 宽查询吞掉）"
        );
        assert_eq!(world.resource::<QuestLogState>().selected_avail, None);
    }

    /// #2801 单元②：行模型逐条对齐 C# `QuestMessage.UpdateQuest`/`AdjustDescription`
    /// （`QuestDialogs.cs:1142-1213`）：名 → 描述 → 任务 → 任务交付 → 时间限制 → 进度
    #[test]
    fn quest_message_lines_match_csharp_order() {
        let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
        q.name = "消灭稻草人".to_string();
        q.npc_index = 10;
        q.finish_npc_index = 20;
        q.description = vec!["说明一".to_string(), "说明二".to_string()];
        q.task_description = vec!["击杀 稻草人 0/3".to_string()];
        q.return_description = vec!["交给 张三".to_string()];
        q.completion_description = vec!["完成描述".to_string()];
        q.time_limit_in_seconds = 3661;

        // 未接：无「进度」段（C# `Quest.Taken && TaskList.Count > 0 && DisplayProgress`）
        assert_eq!(
            quest_message_lines(&q, false, &[], false, true),
            vec![
                "消灭稻草人",
                "说明一",
                "说明二",
                " ",
                "任务",
                "击杀 稻草人 0/3",
                " ",
                "任务交付",
                "交给 张三",
                " ",
                "时间限制",
                "1h 01m 01s",
            ]
        );
        // 已接：尾部追加「进度 + 任务列表」
        assert_eq!(
            quest_message_lines(&q, true, &["击杀 稻草人 1/3".to_string()], false, true)
                .iter()
                .rev()
                .take(3)
                .cloned()
                .collect::<Vec<_>>(),
            vec!["击杀 稻草人 1/3", "进度", " "]
        );
        // 未接：即便带任务列表也不出「进度」段（C# 条件含 `Quest.Taken`）
        assert!(
            !quest_message_lines(&q, false, &["击杀 稻草人 1/3".to_string()], false, true)
                .contains(&"进度".to_string())
        );
        // 已接 + 非同一交付 NPC + 在交付 NPC 处 → 只显示完成描述（C# `:1151-1157`）
        assert_eq!(
            quest_message_lines(&q, true, &[], true, true),
            vec!["消灭稻草人", "完成描述"]
        );
        // `SameFinishNPC`（NPCIndex == FinishNPCIndex）→ 即便在交付 NPC 处也走普通分支
        let same = ClientQuestInfo {
            finish_npc_index: 10,
            ..q.clone()
        };
        assert!(quest_message_lines(&same, true, &[], true, true).contains(&"任务".to_string()));
    }

    /// #2801 单元②：标题行判定（C# `NewText` `:1242-1251`）
    #[test]
    fn quest_line_title_detection() {
        assert!(
            quest_line_is_title(0, "消灭稻草人"),
            "首行恒为标题（黄色粗体）"
        );
        assert!(quest_line_is_title(5, "任务"));
        assert!(quest_line_is_title(5, "进度"));
        assert!(quest_line_is_title(5, "任务交付"));
        assert!(quest_line_is_title(5, "时间限制"));
        assert!(!quest_line_is_title(5, "击杀 稻草人 0/3"));
        assert!(!quest_line_is_title(1, " "), "空行不是标题行");
    }

    /// #2801 单元②：上/下滚键（C# `:1042-1060`）与滚轮（`:1082-1098`，含原版 `Count-1` 怪癖）
    #[test]
    fn quest_message_scroll_helpers_match_csharp() {
        assert_eq!(quest_msg_scroll_up(0), 0, "已在首行不再上滚");
        assert_eq!(quest_msg_scroll_up(3), 2);
        assert_eq!(quest_msg_scroll_down(0, 16, 16), 0, "一页放得下不滚");
        assert_eq!(quest_msg_scroll_down(0, 20, 16), 1);
        assert_eq!(quest_msg_scroll_down(4, 20, 16), 4, "已到末页不滚");
        // 滚轮：count=+1 = 上滚一行；末行钳位用 `Count - 1`（C# 原样）
        assert_eq!(quest_msg_wheel_top_line(0, 1, 20, 16), 0);
        assert_eq!(quest_msg_wheel_top_line(5, 3, 20, 16), 2);
        assert_eq!(quest_msg_wheel_top_line(0, -3, 20, 16), 3);
        assert_eq!(quest_msg_wheel_top_line(3, -5, 20, 16), 4);
        assert_eq!(
            quest_msg_wheel_top_line(9, 1, 16, 16),
            9,
            "行数不足一页不动"
        );
    }

    /// #2801 单元②：位置条换算（C# `UpdatePositionBar` `:1120-1140` / `PositionBar_OnMoving` `:1100-1118`）
    #[test]
    fn quest_message_position_bar_matches_csharp() {
        assert_eq!(quest_msg_bar_y(0, 16, 16), None, "不足一页隐藏位置条");
        // len=20 → interval = (261-46)/(20-16) = 53（整数除法）
        assert_eq!(quest_msg_bar_y(0, 20, 16), Some(46));
        assert_eq!(quest_msg_bar_y(1, 20, 16), Some(99));
        assert_eq!(quest_msg_bar_y(4, 20, 16), Some(258));
        assert_eq!(quest_msg_top_line_at_bar(46, 20, 16), 0);
        assert_eq!(quest_msg_top_line_at_bar(99, 20, 16), 1);
        assert_eq!(quest_msg_top_line_at_bar(261, 20, 16), 4);
        assert_eq!(
            quest_msg_top_line_at_bar(0, 20, 16),
            0,
            "越界向下钳到 PosMinY"
        );
        // len=17 → interval = 215 → 第二页贴底 PosMaxY
        assert_eq!(quest_msg_bar_y(1, 17, 16), Some(261));
        assert_eq!(quest_msg_top_line_at_bar(261, 17, 16), 1);
    }

    /// #2810 单元①：`{文本/颜色}` 段解析（C# `NewText` 的 `C` 正则 `:1008` + `:1321-1323`）
    #[test]
    fn quest_line_markup_splits_colour_segments() {
        // 无标记
        let (text, segs) = quest_line_markup("普通文本");
        assert_eq!(text, "普通文本");
        assert!(segs.is_empty());

        // 单段：正文去掉 `{...}`、段记录文本/颜色/偏移
        let (text, segs) = quest_line_markup("前{红字/Red}后");
        assert_eq!(text, "前红字后");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "红字");
        assert_eq!(segs[0].color_name, "Red");
        assert_eq!(
            segs[0].byte_offset,
            "前".len(),
            "段首偏移 = 去标记后整行内的字节位置"
        );

        // 多段 + 行首/行尾
        let (text, segs) = quest_line_markup("{A/Red}中{B/Green}");
        assert_eq!(text, "A中B");
        assert_eq!(segs.len(), 2);
        assert_eq!(
            (segs[0].byte_offset, segs[0].color_name.as_str()),
            (0, "Red")
        );
        assert_eq!(
            (segs[1].byte_offset, segs[1].color_name.as_str()),
            ("A中".len(), "Green")
        );

        // 非标记原样保留（与 #2801 判据一致）
        assert_eq!(quest_line_markup("{无斜杠}").0, "{无斜杠}");
        assert_eq!(quest_line_markup("<链接/@key>").0, "<链接/@key>");
        // 空段文本 `{/Red}`：C# `close > i+1` 只要求 `}` 在 `{` 后至少两格，故仍成立——
        // `values[0]` 为空串 → 原地插入空串（整段被删掉），段文本为空（叠加层画空字，无视觉）
        let (text, segs) = quest_line_markup("a{/Red}b");
        assert_eq!(text, "ab");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, "");
        assert_eq!(segs[0].byte_offset, 1);
    }

    /// #2810 单元②：链接标记扫描（C# `NPCDialogs.cs:24-26` 三条正则的等价手写扫描）
    #[test]
    fn quest_line_links_parse_csharp_forms() {
        // 两种写法 + 内嵌名
        let l = quest_line_links("去[ITEM:1001|力量戒指]看看");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].kind, QuestLinkKind::Item);
        assert_eq!(l[0].index, "1001");
        assert_eq!(l[0].provided_name.as_deref(), Some("力量戒指"));
        assert_eq!(
            &"去[ITEM:1001|力量戒指]看看"[l[0].range.clone()],
            "[ITEM:1001|力量戒指]"
        );

        let l = quest_line_links("<$NPC:110>");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].kind, QuestLinkKind::Npc);
        assert_eq!(l[0].index, "110");
        assert_eq!(l[0].provided_name, None);

        // 大小写不敏感（C# `RegexOptions.IgnoreCase`）
        assert_eq!(
            quest_line_links("[monster:101]")[0].kind,
            QuestLinkKind::Monster
        );
        assert_eq!(quest_line_links("<$item:5>")[0].kind, QuestLinkKind::Item);

        // 非标记 / 非法形式原样（不产生链接）
        for s in [
            "[ITEM:]",
            "[ITEM:abc]",
            "[FOO:1]",
            "[ITEM:1", // 缺 `]`
            "<$ITEM:>",
            "<ITEM:1>", // 缺 `$`
            "普通文本",
        ] {
            assert!(quest_line_links(s).is_empty(), "{s} 不应解析出链接");
        }

        // 多链接：按出现顺序 + 各自范围
        let l = quest_line_links("[ITEM:1]与<$MONSTER:2>");
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].range.start, 0);
        assert_eq!(l[1].kind, QuestLinkKind::Monster);
        assert!(l[0].range.end <= l[1].range.start);
    }

    /// #2810 单元②：彩色段与链接按 C# `OrderBy(match.Index)` 合并处理，
    /// 部件偏移必须是**最终显示文本**内的字节偏移（颜色段在链接之后时易错）
    #[test]
    fn quest_line_overlays_merges_marks_in_order() {
        let (text, parts) =
            quest_line_overlays("前{红字/Red}[ITEM:7|剑]后{蓝字/Blue}", |l| {
                format!("[{}]", l.kind.fallback_name(&l.index))
            });
        assert_eq!(text, "前红字[Item 7]后蓝字");
        assert_eq!(parts.len(), 3);
        let QuestOverlayPart::Colour {
            text: t0,
            color_name,
            offset,
        } = &parts[0]
        else {
            panic!("第 1 个应为彩色段");
        };
        assert_eq!(
            (t0.as_str(), color_name.as_str(), *offset),
            ("红字", "Red", "前".len())
        );
        let QuestOverlayPart::Link {
            text: t1,
            index,
            offset,
            ..
        } = &parts[1]
        else {
            panic!("第 2 个应为链接");
        };
        assert_eq!(
            (t1.as_str(), index.as_str(), *offset),
            ("[Item 7]", "7", "前红字".len())
        );
        let QuestOverlayPart::Colour { offset: o2, .. } = &parts[2] else {
            panic!("第 3 个应为彩色段");
        };
        assert_eq!(
            *o2,
            "前红字[Item 7]后".len(),
            "链接之后的彩色段偏移要含链接名长度"
        );
    }

    /// #2810 单元②：链接显示名（C# `GetDisplayNameForLink` `NPCDialogs.cs:920-955`）
    /// —— 内嵌名 > 查表 > 回退字面量
    #[test]
    fn quest_link_display_name_prefers_provided_then_cache() {
        let mut catalog = QuestCatalog::default();
        catalog.item_names.insert(1001, "力量戒指".to_string());
        let info = crate::game::object_state::InfoCache::default();

        let link = |s: &str| quest_line_links(s).remove(0);
        // 内嵌名优先（即便表里有名字）
        assert_eq!(
            quest_link_display_name(&link("[ITEM:1001|内嵌名]"), &catalog, &info),
            "内嵌名"
        );
        // 查表
        assert_eq!(
            quest_link_display_name(&link("[ITEM:1001]"), &catalog, &info),
            "力量戒指"
        );
        // 回退字面量（C# `Item {idx}` 等）
        assert_eq!(
            quest_link_display_name(&link("[ITEM:9999]"), &catalog, &info),
            "Item 9999"
        );
        assert_eq!(
            quest_link_display_name(&link("<$MONSTER:101>"), &catalog, &info),
            "Monster 101"
        );
        assert_eq!(
            quest_link_display_name(&link("[NPC:110]"), &catalog, &info),
            "Npc 110"
        );
    }

    /// #2810 单元①：彩色段折行落位（前缀宽 = 同字体同尺寸的宋体双宽度量）
    #[test]
    fn quest_segment_offset_wraps_like_text() {
        // 单行：x = 前缀估宽（CJK 1.0em / ASCII 0.5em）
        assert_eq!(quest_segment_offset("ab", 12.0, 280.0), (0, 12.0));
        assert_eq!(quest_segment_offset("古", 12.0, 280.0), (0, 12.0));
        assert_eq!(quest_segment_offset("a古", 12.0, 280.0), (0, 18.0));
        // 折行：前缀超过 280 → 段落在第 2 行，x 用行内前缀宽
        let prefix = "古".repeat(24); // 24*12 = 288 > 280 → 折行
        let (row, x) = quest_segment_offset(&prefix, 12.0, 280.0);
        assert_eq!(row, 1, "越过一行的前缀把段推到第 2 行");
        assert_eq!(x, 12.0, "第 2 行内只剩 4 个字之前的偏移（23 字前缀跨行）");
    }

    /// #2801 单元②（#2810 起共用解析）：`{文本/颜色}` 去标记
    #[test]
    fn quest_line_display_text_strips_colour_markup() {
        assert_eq!(quest_line_display_text("普通文本"), "普通文本");
        assert_eq!(quest_line_display_text("{红字/Red}尾巴"), "红字尾巴");
        assert_eq!(quest_line_display_text("前{绿/Green}后"), "前绿后");
        assert_eq!(
            quest_line_display_text("{无斜杠}"),
            "{无斜杠}",
            "无 `/` 的不是颜色标记，原样保留"
        );
        assert_eq!(
            quest_line_display_text("<链接/@key>"),
            "<链接/@key>",
            "链接标记本单元不处理（见函数注释的未移植清单）"
        );
    }

    /// #2810 单元①：彩色叠加段系统级渲染（C# `NewColour` 叠加标签，`:1336-1353`）
    #[test]
    fn quest_detail_colour_segments_render_over_line() {
        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open(DialogKind::QuestDetail);
        world.insert_resource(mgr);
        world.insert_resource(QuestDetailState {
            quest_id: Some(1),
            top_line: 0,
            ..Default::default()
        });
        let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
        q.name = "任务名".to_string();
        // 描述首行（= 行槽 1）含一个彩色段；第 2 段不存在 → 池内第 2 个应保持隐藏
        q.description = vec!["前{红字/Red}后".to_string()];
        q.task_description = vec![];
        q.return_description = vec![];
        q.completion_description = vec![];
        q.time_limit_in_seconds = 0;
        world.insert_resource(QuestCatalog {
            infos: vec![q],
            ..Default::default()
        });
        world.insert_resource(QuestLogState::default());
        world.insert_resource(crate::game::dialogs::npc::NpcDialogState::default());
        world.insert_resource(crate::network::NetConnection::default());
        world.init_resource::<bevy::ecs::message::Messages<MouseWheel>>();
        // #2810 单元②：叠加段/链接悬停所需资源（与 QuestDetailExtras 字段一一对应）
        world.insert_resource(crate::game::object_state::InfoCache::default());
        world.insert_resource(crate::ui::tooltip::TooltipState::default());
        world.insert_resource(crate::control::CursorProbe { pos: None });
        world.spawn((
            QuestDetailWidget,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(532.0),
                top: Val::Px(60.0),
                ..default()
            },
        ));
        let lines: Vec<Entity> = (0..QUEST_MSG_LINE_COUNT)
            .map(|i| {
                world
                    .spawn((
                        QuestDetailLine(i),
                        Node::default(),
                        Text::new(""),
                        TextColor(Color::WHITE),
                        TextFont::default(),
                    ))
                    .id()
            })
            .collect();
        // 行槽 1 的段池：0 = 真实段、1 = 空槽
        let segs: Vec<Entity> = (0..2)
            .map(|s| {
                world
                    .spawn((
                        QuestDetailSegment { slot: 1, seg: s },
                        Node::default(),
                        Text::new(""),
                        TextColor(Color::WHITE),
                        TextFont::default(),
                        Visibility::Hidden,
                    ))
                    .id()
            })
            .collect();

        world
            .run_system_once(quest_detail_ui_system)
            .expect("详情窗消息区系统应运行");

        // 基线行文本：去标记后的 `前红字后`
        assert_eq!(world.get::<Text>(lines[1]).unwrap().0, "前红字后");
        assert_eq!(
            world.get::<Visibility>(segs[0]).copied(),
            Some(Visibility::Visible),
            "彩色段应可见"
        );
        assert_eq!(world.get::<Text>(segs[0]).unwrap().0, "红字");
        assert_eq!(
            world.get::<TextColor>(segs[0]).unwrap().0,
            crate::ui::text_markup::known_color("Red").unwrap(),
            "颜色名走 C# Color.FromName 子集"
        );
        let line_left = match world.get::<Node>(lines[1]).unwrap().left {
            Val::Px(v) => v,
            other => panic!("行 left 应为 Px，实为 {other:?}"),
        };
        let seg_left = match world.get::<Node>(segs[0]).unwrap().left {
            Val::Px(v) => v,
            other => panic!("段 left 应为 Px，实为 {other:?}"),
        };
        assert_eq!(
            seg_left,
            line_left + crate::ui::text_markup::est_text_width("前", QUEST_MSG_FONT_PX),
            "段 x = 行原点 + 前缀估宽"
        );
        assert_eq!(
            world.get::<Visibility>(segs[1]).copied(),
            Some(Visibility::Hidden),
            "本行无第 2 段 → 池内空槽保持隐藏"
        );
    }

    /// #2810 单元③：奖励物品说明行（C# `QuestCell.OnMouseEnter` `:1663-1673` → `CreateItemLabel`；
    /// 本端复用背包 `item_tooltip_lines`，耐久取 `Item.Durability` 作当前/最大）
    #[test]
    fn quest_reward_item_tooltip_lines_from_item_info() {
        use mir2_shared::data::stats::Stats;
        use mir2_shared::enums::{ItemType, Stat};
        let mut stats = Stats::new();
        stats.set(Stat::MinDC, 5);
        stats.set(Stat::MaxDC, 12);
        let item = mir2_shared::data::item::ItemInfo {
            index: 5,
            name: "木剑".to_string(),
            item_type: ItemType::Weapon,
            durability: 30,
            stats,
            ..Default::default()
        };
        let lines = quest_reward_item_tooltip_lines(&item);
        assert!(lines.iter().any(|l| l == "类型: 武器"), "{lines:?}");
        assert!(lines.iter().any(|l| l == "耐久: 30/30"), "{lines:?}");
        assert!(lines.iter().any(|l| l == "攻击: 5-12"), "{lines:?}");
    }

    /// #2810 单元③：奖励格悬停弹物品说明（探针驱动；离开则清）——C# `OnMouseEnter/OnMouseLeave`
    #[test]
    fn quest_detail_reward_cell_hover_shows_item_label() {
        fn setup(probe: Option<Vec2>) -> World {
            let mut world = World::new();
            let mut mgr = DialogManager::default();
            mgr.open(DialogKind::QuestDetail);
            world.insert_resource(mgr);
            world.insert_resource(QuestDetailState {
                quest_id: Some(1),
                ..Default::default()
            });
            world.insert_resource(QuestLogState::default());
            world.insert_resource(GameLibraries::default());
            world.insert_resource(Assets::<Image>::default());
            world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
            world.insert_resource(crate::ui::tooltip::TooltipState::default());
            world.insert_resource(crate::control::CursorProbe { pos: probe });
            let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
            q.rewards_fixed_item = vec![reward(10)];
            q.rewards_select_item = vec![];
            world.insert_resource(QuestCatalog {
                infos: vec![q],
                ..Default::default()
            });
            world.spawn((
                QuestDetailWidget,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(532.0),
                    top: Val::Px(60.0),
                    ..default()
                },
            ));
            // 固定排第 0 格 @(15,24) 32x32（面板内相对）
            world.spawn((
                QuestRewardCell {
                    fixed: true,
                    slot: 0,
                },
                Node::default(),
            ));
            world
        }

        // 探针在格子中心（532+15+16, 60+24+16）
        let mut world = setup(Some(Vec2::new(563.0, 100.0)));
        world
            .run_system_once(quest_detail_reward_system)
            .expect("奖励区系统应运行");
        let tip = world.resource::<crate::ui::tooltip::TooltipState>();
        assert!(tip.visible, "悬停奖励格应弹物品说明");
        assert_eq!(tip.source, 14, "归属方 = 任务奖励物品（14）");
        assert_eq!(tip.title, "奖励物品10");

        // 探针移开 → 清掉自己归属的提示
        let mut world = setup(Some(Vec2::new(100.0, 700.0)));
        world
            .run_system_once(quest_detail_reward_system)
            .expect("奖励区系统应运行");
        assert!(!world.resource::<crate::ui::tooltip::TooltipState>().visible);
    }

    /// #2810 单元②：链接系统级渲染——常色青、探针命中转橙 + 弹提示（C# `NewLink` `:1355-1382`）
    #[test]
    fn quest_detail_link_hover_turns_orange_and_shows_tooltip() {
        fn setup(probe: Option<Vec2>) -> (World, Vec<Entity>) {
            let mut world = World::new();
            let mut mgr = DialogManager::default();
            mgr.open(DialogKind::QuestDetail);
            world.insert_resource(mgr);
            world.insert_resource(QuestDetailState {
                quest_id: Some(1),
                top_line: 0,
                ..Default::default()
            });
            let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
            q.name = "任务名".to_string();
            q.description = vec!["看[ITEM:1001|剑]呀".to_string()];
            q.task_description = vec![];
            q.return_description = vec![];
            q.completion_description = vec![];
            q.time_limit_in_seconds = 0;
            world.insert_resource(QuestCatalog {
                infos: vec![q],
                ..Default::default()
            });
            world.insert_resource(QuestLogState::default());
            world.insert_resource(crate::game::dialogs::npc::NpcDialogState::default());
            world.insert_resource(crate::network::NetConnection::default());
            world.init_resource::<bevy::ecs::message::Messages<MouseWheel>>();
            world.insert_resource(crate::game::object_state::InfoCache::default());
            world.insert_resource(crate::ui::tooltip::TooltipState::default());
            world.insert_resource(crate::control::CursorProbe { pos: probe });
            world.spawn((
                QuestDetailWidget,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(532.0),
                    top: Val::Px(60.0),
                    ..default()
                },
            ));
            for i in 0..QUEST_MSG_LINE_COUNT {
                world.spawn((
                    QuestDetailLine(i),
                    Node::default(),
                    Text::new(""),
                    TextColor(Color::WHITE),
                    TextFont::default(),
                ));
            }
            // 行槽 1（描述首行）的段池
            let segs: Vec<Entity> = (0..2)
                .map(|s| {
                    world
                        .spawn((
                            QuestDetailSegment { slot: 1, seg: s },
                            Node::default(),
                            Text::new(""),
                            TextColor(Color::WHITE),
                            TextFont::default(),
                            Visibility::Hidden,
                        ))
                        .id()
                })
                .collect();
            (world, segs)
        }

        // 探针不在链接上 → 青色（C# `Color.Cyan`）、无提示
        let (mut world, segs) = setup(None);
        world
            .run_system_once(quest_detail_ui_system)
            .expect("消息区系统应运行");
        assert_eq!(
            world.get::<Text>(segs[0]).unwrap().0,
            "剑",
            "链接换成内嵌名"
        );
        assert_eq!(
            world.get::<TextColor>(segs[0]).unwrap().0,
            Color::srgb(0.0, 1.0, 1.0),
            "未悬停 → 青色"
        );
        assert!(!world.resource::<crate::ui::tooltip::TooltipState>().visible);

        // 探针落在链接矩形内 → 橙色 + 提示（标题 = 链接名）
        // 行槽 1 原点 = 面板(532,60) + (10, 35+15) + 前缀「看」宽 12 → (554,110)，链接框高 ≈ 12*1.2
        let (mut world, segs) = setup(Some(Vec2::new(558.0, 116.0)));
        world
            .run_system_once(quest_detail_ui_system)
            .expect("消息区系统应运行");
        assert_eq!(
            world.get::<TextColor>(segs[0]).unwrap().0,
            Color::srgb(1.0, 0.65, 0.0),
            "悬停 → 橙（C# `temp.ForeColour = Color.Orange`）"
        );
        let tip = world.resource::<crate::ui::tooltip::TooltipState>();
        assert!(tip.visible, "悬停链接应弹提示");
        assert_eq!(tip.source, 13, "归属方 = 任务链接（13）");
        assert_eq!(tip.title, "剑");
        assert_eq!(
            tip.lines,
            Vec::<String>::new(),
            "物品链接本端只有名字（§7 记录）"
        );
    }

    /// #2801 单元②：消息区渲染落位——行位（行距 15 + 标题行额外 5 + 标题缩进 15）、
    /// 标题圆点、位置条显隐，逐条对 C# `NewText`（`:1260-1268`）/`QuestMessage_AfterDraw`
    /// （`:1066-1080`）/`UpdatePositionBar`（`:1120-1140`）
    #[test]
    fn quest_detail_message_area_lays_out_lines_and_bullets() {
        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open(DialogKind::QuestDetail);
        world.insert_resource(mgr);
        world.insert_resource(QuestDetailState {
            quest_id: Some(1),
            top_line: 0,
            ..Default::default()
        });
        let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
        q.name = "消灭稻草人".to_string();
        q.description = vec!["说明".to_string()];
        q.task_description = vec!["击杀 稻草人 0/3".to_string()];
        q.return_description = vec![];
        q.completion_description = vec![];
        q.time_limit_in_seconds = 0;
        world.insert_resource(QuestCatalog {
            infos: vec![q],
            ..Default::default()
        });
        world.insert_resource(QuestLogState::default());
        world.insert_resource(crate::game::dialogs::npc::NpcDialogState::default());
        world.insert_resource(crate::network::NetConnection::default());
        world.init_resource::<bevy::ecs::message::Messages<MouseWheel>>();
        // #2810 单元②：叠加段/链接悬停所需资源（与 QuestDetailExtras 字段一一对应）
        world.insert_resource(crate::game::object_state::InfoCache::default());
        world.insert_resource(crate::ui::tooltip::TooltipState::default());
        world.insert_resource(crate::control::CursorProbe { pos: None });

        // 根面板（提供 `panel_origin`，滚轮命中区用）
        world.spawn((
            QuestDetailWidget,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(532.0),
                top: Val::Px(60.0),
                ..default()
            },
        ));
        let slots: Vec<Entity> = (0..QUEST_MSG_LINE_COUNT)
            .map(|i| {
                world
                    .spawn((
                        QuestDetailLine(i),
                        Node::default(),
                        Text::new(""),
                        TextColor(Color::WHITE),
                        TextFont::default(),
                    ))
                    .id()
            })
            .collect();
        let bullets: Vec<Entity> = (0..QUEST_MSG_LINE_COUNT)
            .map(|i| {
                world
                    .spawn((QuestDetailBullet(i), Node::default(), Visibility::Hidden))
                    .id()
            })
            .collect();
        let bar = world
            .spawn((
                QuestDetailPositionBar,
                Interaction::None,
                Node::default(),
                Visibility::Visible,
            ))
            .id();

        world
            .run_system_once(quest_detail_ui_system)
            .expect("详情窗消息区系统应运行");

        let text = |i: usize| world.get::<Text>(slots[i]).expect("行槽存在").0.clone();
        let px = |v: &Val| match v {
            Val::Px(v) => *v,
            other => panic!("期望 Px，实际 {other:?}"),
        };
        let top = |i: usize| px(&world.get::<Node>(slots[i]).expect("行槽有 Node").top);
        let left = |i: usize| px(&world.get::<Node>(slots[i]).expect("行槽有 Node").left);

        // C# 行模型：名称 / 说明 / 空行 / 任务 / 任务正文（未接任务 → 无「进度」段）
        assert_eq!(text(0), "消灭稻草人");
        assert_eq!(text(1), "说明");
        assert_eq!(text(2), " ");
        assert_eq!(text(3), "任务");
        assert_eq!(text(4), "击杀 稻草人 0/3");
        assert_eq!(
            text(5),
            "",
            "越界槽位清空（C# `i >= lines.Count` 分支 `:1290-1293`）"
        );

        // 落位：oy=35、行距 15、标题行额外 +5、标题行缩进 15（C# `:1261`）
        assert_eq!((left(0), top(0)), (25.0, 35.0), "首行=标题：缩进 15");
        assert_eq!(top(1), 55.0, "首行标题自身占 5 → 第 2 行 35+15+5");
        assert_eq!(top(2), 70.0, "空行自身不加占位，仍带首行标题的 +5");
        assert_eq!(
            (left(3), top(3)),
            (25.0, 85.0),
            "「任务」标题行：前 1 个标题占 5"
        );
        assert_eq!(
            (left(4), top(4)),
            (10.0, 105.0),
            "正文行无缩进，前 2 个标题共占 10"
        );
        // 字号/颜色：标题 13px、正文 12px；首行黄色（C# `Color.Yellow` `:1249`）
        let size = |i: usize| {
            world
                .get::<TextFont>(slots[i])
                .expect("行槽有字体")
                .font_size
        };
        assert_eq!(size(0), FontSize::Px(QUEST_MSG_TITLE_FONT_PX));
        assert_eq!(size(4), FontSize::Px(QUEST_MSG_FONT_PX));
        assert_eq!(
            world.get::<TextColor>(slots[0]).map(|c| c.0),
            Some(Color::srgb(1.0, 1.0, 0.0)),
            "首行标题黄色"
        );
        assert_eq!(
            world.get::<TextColor>(slots[1]).map(|c| c.0),
            Some(Color::WHITE)
        );

        // 标题圆点：只画标题行，位置 = 行位 +5（C# `AfterDraw` `:1076`）
        let bvis = |i: usize| world.get::<Visibility>(bullets[i]).copied();
        let btop = |i: usize| px(&world.get::<Node>(bullets[i]).expect("圆点有 Node").top);
        assert_eq!((bvis(0), btop(0)), (Some(Visibility::Visible), 40.0));
        assert_eq!((bvis(3), btop(3)), (Some(Visibility::Visible), 90.0));
        assert_eq!(bvis(1), Some(Visibility::Hidden), "非标题行不画圆点");
        assert_eq!(bvis(5), Some(Visibility::Hidden), "越界槽位不画圆点");

        // 位置条：5 行 < 一页 16 行 → 隐藏（C# `UpdatePositionBar` `:1122-1126`）
        assert_eq!(world.get::<Visibility>(bar), Some(&Visibility::Hidden));
    }

    /// #2801 单元③：奖励区横向偏移链（C# `QuestRewards.UpdateInterface` `:1424-1456`）
    #[test]
    fn quest_reward_offsets_match_csharp() {
        assert_eq!(
            quest_reward_offsets(100, 50),
            QuestRewardOffsets {
                gold: 0.0,
                credit: 0.0
            }
        );
        assert_eq!(
            quest_reward_offsets(0, 50),
            QuestRewardOffsets {
                gold: -90.0,
                credit: -90.0
            },
            "无经验奖励 → 金币与信用各左移 90"
        );
        assert_eq!(
            quest_reward_offsets(100, 0),
            QuestRewardOffsets {
                gold: 0.0,
                credit: -90.0
            }
        );
        assert_eq!(
            quest_reward_offsets(0, 0),
            QuestRewardOffsets {
                gold: -90.0,
                credit: -180.0
            },
            "两级 `-= 90` 叠加"
        );
    }

    /// #2801 单元③：奖励性别过滤（C# `QuestRewards.FilterRewards` `:1330-1350`）
    #[test]
    fn quest_reward_gender_filter_matches_csharp() {
        use mir2_shared::enums::{MirGender, RequiredGender};
        let mk = |g: RequiredGender| mir2_shared::data::item::ItemInfo {
            required_gender: g,
            ..Default::default()
        };
        assert!(quest_reward_visible_for_gender(
            &mk(RequiredGender::MALE),
            MirGender::Male
        ));
        assert!(!quest_reward_visible_for_gender(
            &mk(RequiredGender::MALE),
            MirGender::Female
        ));
        assert!(quest_reward_visible_for_gender(
            &mk(RequiredGender::NONE),
            MirGender::Female
        ));
        // 未设性别位（0）：C# `HasFlag` 对两种性别都是 false → 不显示
        assert!(!quest_reward_visible_for_gender(
            &mk(RequiredGender::empty()),
            MirGender::Male
        ));
    }

    /// #2801 单元③：物品图居中偏移（C# `QuestCell.DrawControl` `:1690` 整数除法，
    /// 负数向零截断）
    #[test]
    fn quest_reward_item_offset_matches_csharp() {
        assert_eq!(quest_reward_item_offset(40, 32), (0.0, 0.0));
        assert_eq!(quest_reward_item_offset(45, 37), (-2.0, -2.0));
        assert_eq!(quest_reward_item_offset(20, 20), (10.0, 6.0));
    }

    /// #2801 单元③：分享键发 `C.ShareQuest{QuestIndex}`（C# `_shareButton.Click` `:568-575`）
    #[test]
    fn quest_detail_share_sends_share_quest() {
        use mir2_shared::packets::base::Packet;

        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open(DialogKind::QuestDetail);
        world.insert_resource(mgr);
        world.insert_resource(QuestDetailState {
            quest_id: Some(7),
            ..Default::default()
        });
        world.insert_resource(QuestLogState::default());
        world.insert_resource(QuestCatalog::default());
        world.insert_resource(crate::game::dialogs::npc::NpcDialogState::default());
        world.init_resource::<bevy::ecs::message::Messages<MouseWheel>>();
        // #2810 单元②：叠加段/链接悬停所需资源（与 QuestDetailExtras 字段一一对应）
        world.insert_resource(crate::game::object_state::InfoCache::default());
        world.insert_resource(crate::ui::tooltip::TooltipState::default());
        world.insert_resource(crate::control::CursorProbe { pos: None });
        world.insert_resource(ButtonInput::<MouseButton>::default());
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
        world.insert_resource(crate::network::NetConnection {
            to_server: Some(tx),
            ..Default::default()
        });
        world.spawn((
            Button,
            QuestDetailShare,
            Interaction::Pressed,
            Node::default(),
        ));

        world
            .run_system_once(quest_detail_ui_system)
            .expect("详情窗 UI 系统应运行");

        let sent = rx.try_recv().expect("分享键必须发出 ShareQuest");
        let opcode = i16::from_le_bytes([sent[2], sent[3]]);
        assert_eq!(
            opcode,
            mir2_shared::enums::ClientPacketIds::ShareQuest as i16,
            "opcode 必须是 C.ShareQuest"
        );
        let body = mir2_shared::packets::client::quest::ShareQuest::read_body(
            &mut std::io::Cursor::new(&sent[4..]),
        )
        .expect("ShareQuest body 应可解析");
        assert_eq!(body.quest_index, 7);
    }

    /// #2801 单元③：取消询问框 Yes → `C.AbandonQuest` + 关窗；No → 只关框（C# `:590-598`）
    #[test]
    fn quest_detail_cancel_confirm_yes_and_no() {
        fn setup(press_yes: bool) -> (World, Entity, crossbeam_channel::Receiver<Vec<u8>>) {
            let mut world = World::new();
            let mut mgr = DialogManager::default();
            mgr.open(DialogKind::QuestDetail);
            world.insert_resource(mgr);
            world.insert_resource(QuestDetailState {
                quest_id: Some(9),
                confirm_cancel: true,
                ..Default::default()
            });
            let (tx, rx) = crossbeam_channel::unbounded::<Vec<u8>>();
            world.insert_resource(crate::network::NetConnection {
                to_server: Some(tx),
                ..Default::default()
            });
            let root = world.spawn((QuestCancelConfirm, Visibility::Hidden)).id();
            let btn = world
                .spawn((Button, Interaction::Pressed, Node::default()))
                .id();
            if press_yes {
                world.entity_mut(btn).insert(QuestCancelYes);
            } else {
                world.entity_mut(btn).insert(QuestCancelNo);
            }
            (world, root, rx)
        }

        // Yes：发 AbandonQuest + 关闭主窗 + 关框
        let (mut world, root, rx) = setup(true);
        world
            .run_system_once(quest_detail_confirm_system)
            .expect("取消确认系统应运行");
        assert_eq!(
            world.get::<Visibility>(root),
            Some(&Visibility::Visible),
            "开框时确认框可见（Yes 同帧先置可见再收尾）"
        );
        assert!(!world.resource::<QuestDetailState>().confirm_cancel);
        assert!(!world
            .resource::<DialogManager>()
            .is_open(DialogKind::QuestDetail));
        let sent = rx.try_recv().expect("Yes 必须发出 AbandonQuest");
        let opcode = i16::from_le_bytes([sent[2], sent[3]]);
        assert_eq!(
            opcode,
            mir2_shared::enums::ClientPacketIds::AbandonQuest as i16
        );

        // No：不发包、主窗保持打开
        let (mut world, _, rx) = setup(false);
        world
            .run_system_once(quest_detail_confirm_system)
            .expect("取消确认系统应运行");
        assert!(!world.resource::<QuestDetailState>().confirm_cancel);
        assert!(world
            .resource::<DialogManager>()
            .is_open(DialogKind::QuestDetail));
        assert!(rx.try_recv().is_err(), "No 不应发包");
    }

    /// #2801 单元③：奖励区系统级渲染 + 多选一
    /// （C# `QuestRewards.UpdateInterface` `:1420-1530` / `FilterRewards` `:1588-1610`）
    #[test]
    fn quest_detail_reward_area_lays_out_and_selects() {
        use mir2_shared::enums::{MirClass, MirGender, RequiredGender};

        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open(DialogKind::QuestDetail);
        world.insert_resource(mgr);
        world.insert_resource(QuestDetailState {
            quest_id: Some(1),
            ..Default::default()
        });
        // 玩家性别 = 女：可选奖励第 0 件（仅男性）被过滤 → 过滤序 0 对应**未过滤下标 1**
        world.spawn((
            crate::actor::ActorAppearance {
                class: MirClass::Warrior,
                gender: MirGender::Female,
                armour: 0,
                hair: 0,
                weapon: 0,
                weapon_effect: 0,
                wing_effect: 0,
            },
            crate::actor::LocalPlayer,
        ));
        // 未初始化 GameLibraries → 无磁盘 IO（物品图取不到，本测试只钉布局/显隐/选择）
        world.insert_resource(GameLibraries::default());
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        // #2810 单元③：奖励格悬停物品说明所需资源
        world.insert_resource(crate::ui::tooltip::TooltipState::default());
        world.insert_resource(crate::control::CursorProbe { pos: None });

        let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
        q.reward_exp = 100;
        q.reward_gold = 0;
        q.reward_credit = 0;
        q.rewards_fixed_item = vec![reward_gendered(10, RequiredGender::NONE, 3)];
        q.rewards_select_item = vec![
            reward_gendered(20, RequiredGender::MALE, 1),
            reward_gendered(21, RequiredGender::FEMALE, 1),
        ];
        world.insert_resource(QuestCatalog {
            infos: vec![q],
            ..Default::default()
        });

        let exp_icon = world
            .spawn((
                QuestRewardPart::Icon(QuestRewardIcon::Exp),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();
        let gold_value = world
            .spawn((
                QuestRewardPart::Value(QuestRewardValue::Gold),
                Visibility::Hidden,
                Node::default(),
                Text::new(""),
            ))
            .id();
        let exp_value = world
            .spawn((
                QuestRewardPart::Value(QuestRewardValue::Exp),
                Visibility::Hidden,
                Node::default(),
                Text::new(""),
            ))
            .id();
        let fixed_cell = QuestRewardCell {
            fixed: true,
            slot: 0,
        };
        let fixed_bg = world
            .spawn((
                QuestRewardPart::CellBg(fixed_cell),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();
        let fixed_count = world
            .spawn((
                QuestRewardPart::CellCount(fixed_cell),
                Visibility::Hidden,
                Node::default(),
                Text::new(""),
            ))
            .id();
        let select_cell = QuestRewardCell {
            fixed: false,
            slot: 0,
        };
        let select_bg = world
            .spawn((
                QuestRewardPart::CellBg(select_cell),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();
        let select_btn = world
            .spawn((Button, select_cell, Interaction::Pressed, Node::default()))
            .id();

        world
            .run_system_once(quest_detail_reward_system)
            .expect("奖励区系统应运行");

        let vis = |w: &World, e: Entity| w.get::<Visibility>(e).copied();
        assert_eq!(
            vis(&world, exp_icon),
            Some(Visibility::Visible),
            "有经验奖励 → 图标可见"
        );
        assert_eq!(
            world.get::<Node>(exp_icon).unwrap().left,
            Val::Px(QUEST_REWARD_ORIGIN.0 + 10.0),
            "经验图标 @(10,2)（C# `:1424`）"
        );
        assert_eq!(world.get::<Text>(exp_value).unwrap().0, "100");
        assert_eq!(
            vis(&world, gold_value),
            Some(Visibility::Hidden),
            "无金币奖励 → 数值隐藏"
        );
        assert_eq!(
            vis(&world, fixed_bg),
            Some(Visibility::Visible),
            "固定奖励格底图恒显（C# `Prguse[989]`）"
        );
        assert_eq!(
            world.get::<Text>(fixed_count).unwrap().0,
            "3",
            "数量 >1 显示 `###0`"
        );
        assert_eq!(vis(&world, fixed_count), Some(Visibility::Visible));
        assert_eq!(
            vis(&world, select_bg),
            Some(Visibility::Hidden),
            "可选格未选中时无底图"
        );

        // 点可选排过滤序 0 = 未过滤下标 1（C# `FindSelectedItemIndex` `:1268-1284`）
        assert_eq!(
            world.resource::<QuestDetailState>().selected_reward,
            Some(1)
        );
        let _ = select_btn;
        world
            .run_system_once(quest_detail_reward_system)
            .expect("奖励区系统应可重复运行");
        assert_eq!(
            vis(&world, select_bg),
            Some(Visibility::Visible),
            "选中后底图切 `Prguse[979]`"
        );
    }

    /// #2801 后续修复：固定排**不做**性别过滤
    /// （C# `QuestRewards.UpdateInterface` `:1533-1548` 直接用 `quest.RewardsFixedItem`，
    /// `FilterRewards` 那一行在 `:1534` 被注释掉；只有可选排 `:1551-1553` 过滤）。
    /// 玩家女 + 固定奖励仅限男性 → 仍必须显示；对照：可选排同样仅限男性 → 过滤后不可选。
    #[test]
    fn quest_detail_fixed_rewards_ignore_gender_filter() {
        use mir2_shared::enums::{MirClass, MirGender, RequiredGender};

        let mut world = World::new();
        let mut mgr = DialogManager::default();
        mgr.open(DialogKind::QuestDetail);
        world.insert_resource(mgr);
        world.insert_resource(QuestDetailState {
            quest_id: Some(1),
            ..Default::default()
        });
        world.spawn((
            crate::actor::ActorAppearance {
                class: MirClass::Warrior,
                gender: MirGender::Female,
                armour: 0,
                hair: 0,
                weapon: 0,
                weapon_effect: 0,
                wing_effect: 0,
            },
            crate::actor::LocalPlayer,
        ));
        world.insert_resource(GameLibraries::default());
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(crate::ui::sprite_ui::UiImageCache::default());
        // #2810 单元③：奖励格悬停物品说明所需资源
        world.insert_resource(crate::ui::tooltip::TooltipState::default());
        world.insert_resource(crate::control::CursorProbe { pos: None });

        let mut q = info(1, 1, RequiredClass::from_bits_truncate(0));
        q.reward_exp = 0;
        q.reward_gold = 0;
        q.reward_credit = 0;
        q.rewards_fixed_item = vec![reward_gendered(10, RequiredGender::MALE, 3)];
        q.rewards_select_item = vec![reward_gendered(20, RequiredGender::MALE, 1)];
        world.insert_resource(QuestCatalog {
            infos: vec![q],
            ..Default::default()
        });

        let fixed_cell = QuestRewardCell {
            fixed: true,
            slot: 0,
        };
        let fixed_bg = world
            .spawn((
                QuestRewardPart::CellBg(fixed_cell),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();
        let fixed_count = world
            .spawn((
                QuestRewardPart::CellCount(fixed_cell),
                Visibility::Hidden,
                Node::default(),
                Text::new(""),
            ))
            .id();
        let select_cell = QuestRewardCell {
            fixed: false,
            slot: 0,
        };
        let select_bg = world
            .spawn((
                QuestRewardPart::CellBg(select_cell),
                Visibility::Hidden,
                Node::default(),
            ))
            .id();
        world
            .spawn((Button, select_cell, Interaction::Pressed, Node::default()))
            .id();

        world
            .run_system_once(quest_detail_reward_system)
            .expect("奖励区系统应运行");

        let vis = |w: &World, e: Entity| w.get::<Visibility>(e).copied();
        assert_eq!(
            vis(&world, fixed_bg),
            Some(Visibility::Visible),
            "固定排不过滤性别（C# `:1534` 注释掉的 FilterRewards）"
        );
        assert_eq!(world.get::<Text>(fixed_count).unwrap().0, "3");
        assert_eq!(
            world.resource::<QuestDetailState>().selected_reward,
            None,
            "可选排仍按性别过滤：过滤后无可选项，点击不改选择"
        );
        assert_eq!(vis(&world, select_bg), Some(Visibility::Hidden));
    }

    /// 任务行命中：初始原点等价于原固定坐标，拖动后跟随面板
    #[test]
    fn row_rect_origin_and_drag() {
        // 初始 (192,60)（C# `QuestDiaryDialog`）：首行 y=100（=60+40），x 起 210（=192+18）
        let (rx, ry, rw, rh) = quest_log_row_rect(0, DIARY_POS.0, DIARY_POS.1);
        assert_eq!((rx, ry, rw, rh), (210.0, 100.0, 282.0, 18.0));
        assert_eq!(
            quest_log_row_rect(7, DIARY_POS.0, DIARY_POS.1).1,
            100.0 + 7.0 * 20.0
        );
        // 拖动到 (250,100)：跟随
        let (rx2, ry2, _, _) = quest_log_row_rect(0, 250.0, 100.0);
        assert_eq!((rx2, ry2), (268.0, 140.0));
    }

    use super::*;
    use mir2_shared::enums::{QuestType, RequiredClass};

    fn info(index: i32, min_level: i32, class: RequiredClass) -> ClientQuestInfo {
        ClientQuestInfo {
            index,
            npc_index: 10,
            name: format!("任务{}", index),
            group: String::new(),
            description: vec![],
            task_description: vec!["击杀 X 0/3".to_string()],
            return_description: vec![],
            completion_description: vec![],
            min_level_needed: min_level,
            max_level_needed: 0,
            quest_needed: 0,
            class_needed: class,
            quest_type: QuestType::General,
            time_limit_in_seconds: 0,
            reward_gold: 100,
            reward_exp: 200,
            reward_credit: 0,
            rewards_fixed_item: vec![],
            rewards_select_item: vec![],
            finish_npc_index: 10,
        }
    }

    /// #2535 子批2：带组名的任务定义（info() 的 group 变体）
    fn grouped_info(index: i32, group: &str) -> ClientQuestInfo {
        ClientQuestInfo {
            group: group.to_string(),
            ..info(index, 1, RequiredClass::from_bits_truncate(0))
        }
    }

    fn entry(id: i32, completed: bool) -> QuestEntry {
        QuestEntry {
            id,
            name: format!("任务{}", id),
            tasks: vec!["击杀 X 0/3".to_string()],
            taken: true,
            completed,
            is_new: false,
        }
    }

    fn reward(item_index: i32) -> QuestItemReward {
        QuestItemReward {
            item: mir2_shared::data::item::ItemInfo {
                index: item_index,
                name: format!("奖励物品{item_index}"),
                ..Default::default()
            },
            count: 1,
        }
    }

    /// #2801 单元③：带性别/数量的奖励项（性别过滤与数量角标用）
    fn reward_gendered(
        item_index: i32,
        gender: mir2_shared::enums::RequiredGender,
        count: u16,
    ) -> QuestItemReward {
        QuestItemReward {
            item: mir2_shared::data::item::ItemInfo {
                index: item_index,
                name: format!("奖励物品{item_index}"),
                required_gender: gender,
                ..Default::default()
            },
            count,
        }
    }

    /// #2535 状态机：已接完成→Finishable / 已接未完成→InProgress / 可接→Accept
    #[test]
    fn row_action_taken_states() {
        let i = info(1, 1, RequiredClass::from_bits_truncate(0));
        assert_eq!(
            row_action(&i, Some(&entry(1, true)), 10, 0, 1),
            QuestRowAction::Finishable
        );
        assert_eq!(
            row_action(&i, Some(&entry(1, false)), 10, 0, 1),
            QuestRowAction::InProgress
        );
        assert_eq!(row_action(&i, None, 10, 0, 0), QuestRowAction::Accept);
    }

    /// #2535 状态机：数量上限/等级/职业锁定
    #[test]
    fn row_action_locked_reasons() {
        let i = info(1, 1, RequiredClass::from_bits_truncate(0));
        assert_eq!(
            row_action(&i, None, 10, 0, MAX_CONCURRENT_QUESTS),
            QuestRowAction::Locked("任务数量已达上限")
        );
        assert_eq!(
            row_action(&i, None, 0, 0, 0),
            QuestRowAction::Locked("等级不足")
        );
        let hi = ClientQuestInfo {
            max_level_needed: 20,
            ..info(2, 1, RequiredClass::from_bits_truncate(0))
        };
        assert_eq!(
            row_action(&hi, None, 30, 0, 0),
            QuestRowAction::Locked("等级过高")
        );
        // 仅战士可接，法师（class=1）不符；位掩码 0=不限
        let war_only = info(3, 1, RequiredClass::WARRIOR);
        assert_eq!(
            row_action(&war_only, None, 10, 1, 0),
            QuestRowAction::Locked("职业不符合")
        );
        assert_eq!(
            row_action(&war_only, None, 10, 0, 0),
            QuestRowAction::Accept
        );
    }

    /// #2535 交付校验：无可选→-1 直发；有可选未选→阻止（C# L138-146）
    #[test]
    fn finish_reward_validation() {
        assert_eq!(finish_selected_index(&[], None), Ok(-1));
        let rewards = vec![reward(100), reward(200)];
        assert_eq!(
            finish_selected_index(&rewards, None),
            Err("请先选择一件奖励物品")
        );
        assert_eq!(finish_selected_index(&rewards, Some(1)), Ok(1));
        // 越界视为未选
        assert_eq!(
            finish_selected_index(&rewards, Some(2)),
            Err("请先选择一件奖励物品")
        );
    }

    /// #2535 可接段过滤：目录 − 已接 − 会话已完成；Locked 不列
    #[test]
    fn available_rows_filter() {
        let mut catalog = QuestCatalog::default();
        catalog.infos = vec![
            info(1, 1, RequiredClass::from_bits_truncate(0)), // 已接 → 排除
            info(2, 1, RequiredClass::from_bits_truncate(0)), // 会话已完成 → 排除
            info(3, 1, RequiredClass::from_bits_truncate(0)), // 可接 → 保留
            info(4, 50, RequiredClass::from_bits_truncate(0)), // 等级不足 → 排除
        ];
        catalog.completed.insert(2);
        let mut log = QuestLogState::default();
        log.quests.push(entry(1, false));
        let avail = available_quests(&catalog, &log, 10, 0);
        assert_eq!(avail.len(), 1);
        assert_eq!(avail[0].index, 3);
    }

    /// #2535 回归：NewQuestInfo 只入目录不写日志（登录全量下发曾把所有定义当已接任务）
    #[test]
    fn catalog_upsert_keeps_log_empty() {
        let mut infos = Vec::new();
        upsert_catalog_info(
            &mut infos,
            &info(1, 1, RequiredClass::from_bits_truncate(0)),
        );
        upsert_catalog_info(
            &mut infos,
            &info(2, 1, RequiredClass::from_bits_truncate(0)),
        );
        // 幂等：同 index 覆盖
        let updated = info(1, 5, RequiredClass::from_bits_truncate(0));
        upsert_catalog_info(&mut infos, &updated);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].min_level_needed, 5);
        // 日志侧不由目录驱动
        let log = QuestLogState::default();
        assert!(log.quests.is_empty());
    }

    /// #2535 子批2：分组——按目录 Group 聚类、组序=首次出现序、目录缺失回退空组
    #[test]
    fn diary_groups_cluster_by_group() {
        let mut infos = vec![
            grouped_info(1, "比奇省"),
            grouped_info(2, "毒蛇山谷"),
            grouped_info(3, "比奇省"),
        ];
        let quests = vec![entry(3, false), entry(1, false), entry(2, true)];
        let groups = diary_groups(&quests, &infos);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], ("比奇省".to_string(), vec![0, 1]));
        assert_eq!(groups[1], ("毒蛇山谷".to_string(), vec![2]));
        // 目录缺失（包未到）→ 空组兜底，不丢任务
        infos.clear();
        let groups = diary_groups(&quests, &infos);
        assert_eq!(groups, vec![("".to_string(), vec![0, 1, 2])]);
    }

    /// #2535 子批2：展开语义——空集=全部展开（C# L718）
    #[test]
    fn group_expanded_empty_means_all() {
        let empty = HashSet::new();
        assert!(group_expanded(&empty, "A"));
        assert!(group_expanded(&empty, ""));
        let mut set = HashSet::new();
        set.insert("A".to_string());
        assert!(group_expanded(&set, "A"));
        assert!(!group_expanded(&set, "B"));
    }

    /// #2535 子批2：切换组——物化全量展开集（空集语义下首次收起保留其余组展开）
    #[test]
    fn toggle_group_materializes_full_set() {
        let groups = vec![
            ("A".to_string(), vec![0usize]),
            ("B".to_string(), vec![1usize]),
        ];
        // 初始空集=全展开；收起 A → {B}
        let set = toggle_group(&groups, &HashSet::new(), "A");
        assert_eq!(set, HashSet::from(["B".to_string()]));
        assert!(!group_expanded(&set, "A"));
        assert!(group_expanded(&set, "B"));
        // 再展开 A → {A,B}（全展开的物化形态，语义等同空集）
        let set = toggle_group(&groups, &set, "A");
        assert_eq!(set, HashSet::from(["A".to_string(), "B".to_string()]));
    }

    /// #2535 子批2：行模型——组头+展开组内任务+可接段；收起组仅留组头
    #[test]
    fn diary_rows_layout() {
        let infos = vec![grouped_info(1, "A"), grouped_info(2, "B")];
        let quests = vec![entry(1, false), entry(2, true)];
        let all = HashSet::new();
        let rows = diary_rows(&quests, &infos, 1, &all);
        assert_eq!(
            rows,
            vec![
                DiaryRow::Header(0),
                DiaryRow::Quest(0),
                DiaryRow::Header(1),
                DiaryRow::Quest(1),
                DiaryRow::Avail(0),
            ]
        );
        // 收起 A → 组头 A 保留、任务 0 隐藏
        let collapsed = HashSet::from(["B".to_string()]);
        let rows = diary_rows(&quests, &infos, 0, &collapsed);
        assert_eq!(
            rows,
            vec![DiaryRow::Header(0), DiaryRow::Header(1), DiaryRow::Quest(1)]
        );
    }

    /// #2985 B3：日记「任务：x/y」计数标签落在 C# 的位置上。
    ///
    /// C# `QuestDiaryDialog` 的 `_takenQuestsLabel` 是 `Location = new Point(210, 7)`
    /// （标题栏右侧、关闭钮左边），不是左上角；写死 (18,20) 会压住标题栏下沿、切到面板边框。
    #[test]
    fn diary_count_label_sits_where_csharp_puts_it() {
        use crate::resources::libraries::Libraries;
        use bevy::ecs::system::RunSystemOnce;

        if !crate::resources::libraries::data_assets_present() {
            eprintln!(
                "skip diary_count_label_sits_where_csharp_puts_it: 无 Data 资产（CI 只 checkout 仓库）"
            );
            return;
        }
        let mut world = World::new();
        world.insert_resource(GameLibraries(Libraries::new("Data")));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiFont::default());
        world.insert_resource(UiCjkFont::default());
        world
            .run_system_once(spawn_quest_log)
            .expect("spawn_quest_log 应成功");

        let mut q = world.query::<(&QuestLogLine, &Node)>();
        let mut found = 0;
        for (line, node) in q.iter(&world) {
            if line.0 != 14 {
                continue;
            }
            found += 1;
            assert_eq!(
                (node.left, node.top),
                (Val::Px(DIARY_COUNT_POS.0), Val::Px(DIARY_COUNT_POS.1)),
                "计数标签应在 C# 的 (210,7)，而不是标题栏左上角"
            );
        }
        assert_eq!(found, 1, "计数标签应恰好一个（QuestLogLine(14)）");
        // 与标题栏解耦：必须在面板右半边，否则又会压住 Title[15]
        assert!(DIARY_COUNT_POS.0 > 150.0 && DIARY_COUNT_POS.1 < 20.0);
    }
}
/// #2810 单元②：一行里要叠加渲染的部件（彩色段 / 链接）——偏移均为**最终显示文本**内的字节偏移
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestOverlayPart {
    /// `{文本/颜色}` 段（C# `NewColour`，`QuestDialogs.cs:1336-1353`）
    Colour {
        text: String,
        color_name: String,
        offset: usize,
    },
    /// 链接（C# `NewLink`，`:1355-1382`；青色 + 悬停橙 + tooltip）
    Link {
        text: String,
        kind: QuestLinkKind,
        index: String,
        offset: usize,
    },
}

/// #2810 单元②：按 C# `NewText` 的 `matchList.OrderBy(o => o.Index)`（`:1326-1329`）顺序，
/// 把彩色段与链接标记一次性处理为「最终显示文本 + 叠加部件」。
/// `link_name` 由调用方按 C# `GetDisplayNameForLink`（`NPCDialogs.cs:920-955`）解析。
pub fn quest_line_overlays(
    line: &str,
    mut link_name: impl FnMut(&QuestLink) -> String,
) -> (String, Vec<QuestOverlayPart>) {
    #[derive(Clone)]
    enum Mark {
        Colour(QuestLineSegment),
        Link(QuestLink),
    }
    let (_, colours) = quest_line_markup(line);
    let mut marks: Vec<Mark> = colours.into_iter().map(Mark::Colour).collect();
    marks.extend(quest_line_links(line).into_iter().map(Mark::Link));
    if marks.is_empty() {
        return (line.to_string(), Vec::new());
    }
    marks.sort_by_key(|m| match m {
        Mark::Colour(c) => c.range.start,
        Mark::Link(l) => l.range.start,
    });
    let mut out = String::with_capacity(line.len());
    let mut parts = Vec::new();
    let mut cursor = 0usize;
    for mark in marks {
        let (start, end) = match &mark {
            Mark::Colour(c) => (c.range.start, c.range.end),
            Mark::Link(l) => (l.range.start, l.range.end),
        };
        if start < cursor {
            continue; // 防御：标记不重叠（C# 同）
        }
        out.push_str(&line[cursor..start]);
        match mark {
            Mark::Colour(c) => {
                let offset = out.len();
                out.push_str(&c.text);
                parts.push(QuestOverlayPart::Colour {
                    text: c.text,
                    color_name: c.color_name,
                    offset,
                });
            }
            Mark::Link(l) => {
                let name = link_name(&l);
                let offset = out.len();
                out.push_str(&name);
                parts.push(QuestOverlayPart::Link {
                    text: name,
                    kind: l.kind,
                    index: l.index,
                    offset,
                });
            }
        }
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    (out, parts)
}
