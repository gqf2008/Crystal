// ============================================================================
// NPC 侧任务列表窗（C# `QuestListDialog`）
//
// C# 真源：`Client/MirScenes/Dialogs/QuestDialogs.cs:15-...`
//   面板 `Prguse[950]` 316x466、`Location = (NPCDialog.Size.Width + 47, 0) = (487,0)`
//   标题 `Title[14]` @(18,9)、关闭 `Prguse2[360..362]` @(289,3)、帮助 `Prguse2[257..259]` @(266,3)
//   上翻 `Prguse[951..953]` @(291,35)、下翻 `Prguse[957..959]` @(291,83)
//   行 `QuestRow` `Location = (9, 36 + i*19)`（5 行）、选中高亮 `Prguse[956]`
//   可接计数 `_availableQuestLabel` @(210,8)（文案 `AvailableQuestList` = "可接任务列表：{0}"）
//   接受 `Title[270..272]` / 完成 `Title[273..275]` @(40,436)、离开 `Title[276..278]` @(205,436)
//   消息区 `QuestMessage` @(10,135) 280x160、奖励区 `QuestRewards` @(5,307) 313x130
//   入口：NPC 窗的 Quest 按钮（`NPCDialogs.cs:181` `QuestListDialog.Toggle()`）；
//         `Hide()` 连带 `NPCDialog.Hide()`（`:242-247`）。
//
// 数据源（**不是** `S.ObjectNPC.QuestIDs`——C# 客户端根本不读那个字段）：
//   `NPCObject.GetAvailableQuests()`（`Client/MirObjects/NPCObject.cs:390-424`）
//   = ① 已接且 `FinishNPCIndex == 本 NPC ObjectID`；② 本 NPC 提供（`NPCIndex == ObjectID`）且 `CanAccept` 的。
//   本端这两列由服务端下发（#2867：`build_client_quest_info` → `quest_client_npc_ids`，
//   下发的就是本会话 NPC 的 **object_id**），故直接读 `QuestCatalog.infos` 即可。
//
// 本单元（①）范围：窗的身份/位置/五分行/计数/翻页/选中/关闭/离开/帮助。
// 单元②（未做）：消息区 + 奖励区 + 接受/完成钮（复用 `quest_log` 的 `QuestMessage`/`QuestRewards` 口径）。
// ============================================================================

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use std::collections::HashMap;

use crate::game::dialogs::npc::NpcDialogState;
use crate::game::dialogs::quest_log::{
    quest_line_is_title, quest_message_lines, quest_msg_scroll_down,
    quest_msg_scroll_up, quest_msg_wheel_top_line, quest_reward_offsets,
    quest_reward_visible_for_gender, reward_item_display_with_catalog, row_action, QuestCatalog,
    QuestLogState, QuestRewardOffsets, QuestRowAction, CLOSE_POS, QUEST_MSG_LINE_DY,
    QUEST_MSG_LINE_H, QUEST_MSG_TITLE_DY, QUEST_MSG_TITLE_INDENT, QUEST_REWARD_CELL_DX,
    QUEST_REWARD_CELL_X0, QUEST_REWARD_FIXED_Y, QUEST_REWARD_ORIGIN, QUEST_REWARD_SELECT_Y,
    MAX_CONCURRENT_QUESTS,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_icon_button, spawn_image, spawn_image_native,
    spawn_label, spawn_label_plain, spawn_panel,
};

/// C# `QuestListDialog.Index = 950; Library = Libraries.Prguse`（`QuestDialogs.cs:33-35`）
pub const LIST_PANEL: (LibraryName, usize) = (LibraryName::Prguse, 950);
pub const LIST_SIZE: (f32, f32) = (316.0, 466.0);
/// C# `Location = new Point(NPCDialog.Size.Width + 47, 0)`（`:36`）。
/// `NPCDialog.Size.Width` = 本端 `npc::PANEL_W`（440）⇒ (487,0)。
pub const LIST_POS: (f32, f32) = (crate::game::dialogs::npc::PANEL_W + 47.0, 0.0);

/// C# `Rows = new QuestRow[5]`（`:30`）+ `Location = new Point(9, 36 + i * 19)`（`:322`）
pub const LIST_ROW_COUNT: usize = 5;
pub const LIST_ROW_ORIGIN: (f32, f32) = (9.0, 36.0);
pub const LIST_ROW_DY: f32 = 19.0;
/// C# `QuestRow.Size = new Size(200, 17)`（`:927`）
pub const LIST_ROW_SIZE: (f32, f32) = (200.0, 17.0);
/// C# `SelectedImage = Prguse[956] @ (25, 0)`（`:931-937`）
pub const LIST_ROW_SEL_X: f32 = 25.0;
/// C# `_availableQuestLabel @ (210, 8)`（`:203-208`）
pub const LIST_AVAILABLE_POS: (f32, f32) = (210.0, 8.0);
/// C# 上/下翻页钮（`:47-70`）
pub const LIST_UP_POS: (f32, f32) = (291.0, 35.0);
pub const LIST_DOWN_POS: (f32, f32) = (291.0, 83.0);
/// C# `leaveButton = Title[276..278] @ (205, 436)`（`:144-154`）
pub const LIST_LEAVE_POS: (f32, f32) = (205.0, 436.0);
/// C# `helpButton = Prguse2[257..259] @ (266, 3)`（`:243-...`）
pub const LIST_HELP_POS: (f32, f32) = (266.0, 3.0);
/// C# `Title[530]` NPC 窗任务钮在面板内的位置——本端 `npc.rs` 同值（(172,194)）
pub const LIST_TITLE_INDEX: usize = 14;

/// C# 消息区：`Message = new QuestMessage(up, down, bar, 10) { Location = (10,135),
/// Size = (280,160), PosMinY = 149, PosMaxY = 263 }`（`QuestDialogs.cs:180-193`）。
/// 与任务详情窗的同一控件**只有落点不同**（详情是 (10,35)/46..261）。
pub const LIST_MSG_ORIGIN: (f32, f32) = (10.0, 135.0);
pub const LIST_MSG_W: f32 = 280.0;
pub const LIST_MSG_LINE_COUNT: usize = 10;
pub const LIST_MSG_POS_MIN_Y: i32 = 149;
pub const LIST_MSG_POS_MAX_Y: i32 = 263;
/// C# 消息区上/下钮与位置条（`QuestDialogs.cs:157-179`）
pub const LIST_MSG_UP_POS: (f32, f32) = (292.0, 136.0);
pub const LIST_MSG_DOWN_POS: (f32, f32) = (292.0, 282.0);
pub const LIST_MSG_BAR_POS: (f32, f32) = (292.0, 149.0);

/// C# `_acceptButton / _finishButton = Title[270..272] / [273..275] @ (40, 436)`（`:72-143`）
pub const LIST_ACCEPT_POS: (f32, f32) = (40.0, 436.0);
pub const LIST_ACCEPT_SPRITES: (usize, usize, usize) = (270, 271, 272);
pub const LIST_FINISH_SPRITES: (usize, usize, usize) = (273, 274, 275);
/// C# `QuestRewards.FixedItems/SelectItems = new QuestCell[5]`（`:1405-1406`）
pub const LIST_REWARD_CELL_COUNT: usize = 5;
/// C# 奖励区标题 `Title[17] @ (20,66)`（`QuestReward_BeforeDraw`，`:1459`）
pub const LIST_REWARD_TITLE_POS: (f32, f32) = (20.0, 66.0);

/// `ClientTextKeys.AvailableQuestList`（`Client/Localization/Chinese.json:682`）
pub fn available_quest_label(count: usize) -> String {
    format!("可接任务列表：{count}")
}

/// C# `ClientQuestProgress`（`:390-424`）在本端需要的投影。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcQuestEntry {
    pub quest: i32,
    /// 已接（C# `Taken`）
    pub taken: bool,
    /// 已达成待交付（C# `Completed`）
    pub completed: bool,
}

/// C# `ClientQuestInfo.FinishNPCIndex` 的 getter 回退语义
/// （`QuestInfo.cs:31-34`：`_finishNpcIndex == 0 ? NpcIndex : _finishNpcIndex`）。
/// 本端服务端下发时已做同样回退（`quest_client_npc_ids`），此处再兜一次，
/// 覆盖「任务定义来自别处、finish 列为 0」的情形。
pub fn finish_npc_index(info: &mir2_shared::data::client_data::ClientQuestInfo) -> u32 {
    if info.finish_npc_index == 0 {
        info.npc_index
    } else {
        info.finish_npc_index
    }
}

/// C# `NPCObject.GetAvailableQuests()`（`Client/MirObjects/NPCObject.cs:390-424`）。
///
/// 顺序照 C#：**先**已接可交付的（①），**后**本 NPC 提供且可接的（②）；
/// ②里跳过①已收录的、已完成的、以及 `CanAccept` 不过的（本端用 `row_action` 的 Accept）。
pub fn npc_available_quests(
    catalog: &QuestCatalog,
    log: &QuestLogState,
    npc_object_id: u32,
    level: u16,
    class: u8,
) -> Vec<NpcQuestEntry> {
    // C# `NPCObject npc = (NPCObject)MapControl.GetObject(GameScene.NPCID);` —— NPCID=0 时
    // 取不到对象（`GetAvailableQuests` 不会被调用、`CheckQuestButtonDisplay` 也不置显）。
    // 本端 object_id 0 是「未开对话」的哨兵值（见 `NpcDialogState.npc_object_id`）；
    // 不挡住的话会命中「没有 NPC 关联、服务端下发 npc_index=0」的那批任务定义。
    if npc_object_id == 0 {
        return Vec::new();
    }
    let mut out: Vec<NpcQuestEntry> = Vec::new();

    // ① `q.QuestInfo.FinishNPCIndex == ObjectID`（且未完成）
    for q in &log.quests {
        if catalog.completed.contains(&q.id) {
            continue;
        }
        let Some(info) = catalog.infos.iter().find(|c| c.index == q.id) else {
            continue;
        };
        if finish_npc_index(info) == npc_object_id {
            out.push(NpcQuestEntry {
                quest: q.id,
                taken: true,
                completed: q.completed,
            });
        }
    }

    // ② 本 NPC 提供的（`q.NPCIndex == ObjectID`）且 `CanAccept` 的
    for info in &catalog.infos {
        if info.npc_index != npc_object_id {
            continue;
        }
        if out.iter().any(|e| e.quest == info.index) {
            continue;
        }
        if catalog.completed.contains(&info.index) {
            continue;
        }
        let taken = log.quests.iter().find(|q| q.id == info.index);
        if !matches!(
            row_action(info, taken, level, class, log.quests.len()),
            QuestRowAction::Accept
        ) {
            continue;
        }
        out.push(NpcQuestEntry {
            quest: info.index,
            taken: taken.is_some(),
            completed: taken.map(|q| q.completed).unwrap_or(false),
        });
    }

    out
}

/// C# `QuestMessage.UpdateQuest` 在**列表窗**里的行模型（`QuestDialogs.cs:1142-1213`）。
///
/// 与详情窗共用 `quest_message_lines` 的行序；差别只在 `current_npc_at_finish` 的取值来源
/// （C# 是 `GameScene.Scene.QuestListDialog.CurrentNPCID == QuestInfo.FinishNPCIndex`，`:1151`）。
pub fn npc_quest_message_lines(
    catalog: &QuestCatalog,
    log: &QuestLogState,
    npc_object_id: u32,
    quest_id: i32,
) -> Vec<String> {
    let Some(info) = catalog.infos.iter().find(|c| c.index == quest_id) else {
        return Vec::new();
    };
    let taken_entry = log.quests.iter().find(|q| q.id == quest_id);
    let at_finish_npc = npc_object_id != 0 && npc_object_id == finish_npc_index(info);
    quest_message_lines(
        info,
        taken_entry.is_some(),
        taken_entry.map(|q| q.tasks.as_slice()).unwrap_or(&[]),
        at_finish_npc,
        true,
    )
}

/// C# `ReDisplayButtons`（`QuestDialogs.cs:402-425`）：
/// 接受钮 = `!Taken && CurrentQuests.Count < MaxConcurrentQuests`；完成钮 = `Completed`。
pub fn quest_list_buttons(entry: Option<&NpcQuestEntry>, taken_count: usize) -> (bool, bool) {
    match entry {
        None => (false, false),
        Some(e) => (!e.taken && taken_count < MAX_CONCURRENT_QUESTS, e.completed),
    }
}

/// C# `QuestCell.Location`（固定排 `(i*45 + 15, 24)`，可选排 `(i*45 + 15, 89)`，`:1537`/`:1561`）
/// ——相对奖励区原点。
pub fn reward_cell_offset(fixed: bool, slot: usize) -> (f32, f32) {
    (
        QUEST_REWARD_CELL_X0 + slot as f32 * QUEST_REWARD_CELL_DX,
        if fixed {
            QUEST_REWARD_FIXED_Y
        } else {
            QUEST_REWARD_SELECT_Y
        },
    )
}

/// C# `QuestMessage.UpdatePositionBar`（`QuestDialogs.cs:1120-1140`）在本窗的版本。
///
/// ⚠️ 不能直接复用 `quest_log::quest_msg_bar_y`：那支用的是**详情窗**的 `PosMinY/MaxY`
/// （46/263 系），本窗是 `PosMinY = 149 / PosMaxY = 263`（`:188-189`）。
pub fn quest_list_msg_bar_y(top: usize, len: usize, line_count: usize) -> Option<i32> {
    if len <= line_count {
        return None;
    }
    let span = len as i64 - line_count as i64;
    if span <= 0 {
        return None;
    }
    let interval = (LIST_MSG_POS_MAX_Y - LIST_MSG_POS_MIN_Y) / span as i32;
    let y = LIST_MSG_POS_MIN_Y + top as i32 * interval;
    Some(y.clamp(LIST_MSG_POS_MIN_Y, LIST_MSG_POS_MAX_Y))
}

/// C# `QuestListDialog.StartIndex` / `SelectedIndex` / `CurrentNPCID`（`:26-32`）的打包。
#[derive(Resource, Default)]
pub struct QuestListState {
    /// C# `CurrentNPCID`：本窗绑定到的 NPC object_id（0 = 未绑定）
    pub bound_npc: u32,
    /// C# `SelectedQuest.QuestInfo.Index`（选中任务；None = 尚未选中）
    pub selected: Option<i32>,
    /// C# `StartIndex`：五行窗口的首行下标
    pub start: usize,
    /// C# `Message.TopLine`（消息区首行，`QuestDialogs.cs:1014`）
    pub top_line: usize,
    /// C# `Reward.SelectedItemIndex`（`QuestDialogs.cs:1402`；**未过滤**下标，-1/None = 未选）
    pub selected_reward: Option<usize>,
}

impl QuestListState {
    /// C# `Show()` → `CurrentNPCID = GameScene.NPCID; Reset(); DisplayInfo()`（`:254-263`）。
    /// 换 NPC（或首次打开）时重置选中/翻页。
    pub fn bind(&mut self, npc_object_id: u32) {
        if self.bound_npc == npc_object_id {
            return;
        }
        self.bound_npc = npc_object_id;
        self.selected = None;
        self.start = 0;
        self.top_line = 0;
        self.selected_reward = None;
    }

    /// C# `RefreshInterface` 的 `maxIndex` 夹取（`:299-303`）。
    pub fn clamp_start(&mut self, len: usize) {
        let max_index = len.saturating_sub(LIST_ROW_COUNT);
        if self.start > max_index {
            self.start = max_index;
        }
    }

    /// C# `NewText`（`:1215-1234`）：`CurrentLines.Count` 变化后 `TopLine` 的钳位
    /// （本端每帧统一钳一次，避免换任务后留下空页）。
    pub fn clamp_top_line(&mut self, len: usize, line_count: usize) {
        if self.top_line + line_count > len {
            self.top_line = len.saturating_sub(line_count);
        }
    }
}

#[derive(Component)]
pub struct QuestListWidget;
#[derive(Component)]
pub struct QuestListClose;
#[derive(Component)]
pub struct QuestListLeave;
#[derive(Component)]
pub struct QuestListHelp;
#[derive(Component)]
pub struct QuestListUp;
#[derive(Component)]
pub struct QuestListDown;
/// 第 i 行（C# `Rows[i]`）
#[derive(Component)]
pub struct QuestListRow(pub usize);
/// 第 i 行的选中高亮 `Prguse[956]`
#[derive(Component)]
pub struct QuestListRowMark(pub usize);
#[derive(Component)]
pub struct QuestListAvailableLabel;
#[derive(Component)]
pub struct QuestListMsgUp;
#[derive(Component)]
pub struct QuestListMsgDown;
/// 接受钮（C# `QuestListDialog._acceptButton`，`Title[270..272]`）
#[derive(Component)]
pub struct QuestListAccept;
/// 完成钮（C# `QuestListDialog._finishButton`，`Title[273..275]`）
#[derive(Component)]
pub struct QuestListFinish;

/// 本窗「消息区 + 奖励区」的逐部件标记（同 `quest_log::QuestRewardPart` 的口径，但父窗是列表窗：
/// C# 两扇窗各持一个 `QuestMessage`/`QuestRewards` 实例，故部件各自成组）。
///
/// 合成**单个枚举**而不是多个 marker：`&mut Node`/`&mut Visibility`/`Option<&mut Text>` 这些
/// 可变访问集中在一个查询里，Bevy 的 B0001「两个查询访问同一组件但无法证明不相交」就不会被触发
/// （见 `quest_log` 里 `QuestDetailExtras` 的同类注释）。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum QuestListPart {
    /// 消息区行槽 0..10（C# `QuestMessage._textLabel[LineCount]`，`QuestDialogs.cs:1036`）
    MsgLine(usize),
    /// 标题行前的圆点 `Prguse[919]`（C# `QuestMessage_AfterDraw`，`:1065-1080`）
    Bullet(usize),
    /// 位置条 `Prguse2[205/206]`（C# `PositionBar`，`:1044-1052`）
    Bar,
    /// 固定格底 `Prguse[989]` / 可选格选中底 `Prguse[979]`（C# `QuestCell.DrawControl`，`:1690-1696`）
    Mark { fixed: bool, slot: usize },
    /// 物品图标（`Items[item.image]`，居中偏移 `(40-w)/2,(32-h)/2`）
    Icon { fixed: bool, slot: usize },
    /// 数量标签（C# `QuestCell.CreateDisposeLabel`，`:1725-1745`）
    Count { fixed: bool, slot: usize },
    /// 奖励区顶部图标：0=经验 `Prguse[966]`、1=金币 `Prguse[965]`、2=信用 `Prguse[2447]`
    /// （C# `QuestReward_BeforeDraw`，`:1445-1459`）
    RewardIcon(u8),
    /// 与图标同槽的数值标签（C# `_expLabel/_goldLabel/_creditLabel`，`:1400-1412`）
    RewardValue(u8),
}

pub struct QuestListPlugin;

/// 行命中矩形（C# `QuestRow.Size = (200,17)`、`Location = (9, 36 + i*19)`，`:927`/`:322`）。
pub fn quest_list_row_rect(i: usize, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        ox + LIST_ROW_ORIGIN.0,
        oy + LIST_ROW_ORIGIN.1 + i as f32 * LIST_ROW_DY,
        LIST_ROW_SIZE.0,
        LIST_ROW_SIZE.1,
    )
}

impl Plugin for QuestListPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestListState>();
        app.add_systems(OnEnter(AppState::Game), spawn_quest_list);
        app.add_systems(
            Update,
            (quest_list_ui_system, quest_list_detail_system)
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn spawn_quest_list(
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
    let cjk = shared_cjk_font(&mut fonts, &mut cjk_font);

    // 面板 Prguse[950]（316x466 @ (487,0)）
    let Some(bg) = load_lib_image(&mut libs, &mut images, LIST_PANEL.0, LIST_PANEL.1) else {
        return;
    };
    let panel = spawn_panel(
        &mut commands,
        bg,
        LIST_POS.0,
        LIST_POS.1,
        LIST_SIZE.0,
        LIST_SIZE.1,
        32,
    );
    commands
        .entity(panel)
        .insert((DialogRoot(DialogKind::QuestList), QuestListWidget));

    commands.entity(panel).with_children(|p| {
        // 标题 Title[14] @(18,9)
        if let Some(h) =
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_TITLE_INDEX)
        {
            let (iw, ih) = match libs.0.get_image(LibraryName::Title, LIST_TITLE_INDEX) {
                Some(i) => (i.width.max(0) as f32, i.height.max(0) as f32),
                None => (55.0, 17.0),
            };
            spawn_image(p, h, 18.0, 9.0, iw, ih, 9);
        }
        // 关闭 Prguse2[360..362] @(289,3)
        if let Some(mut btn) =
            spawn_close_button(p, &mut libs, &mut images, CLOSE_POS.0, CLOSE_POS.1, 12)
        {
            btn.insert(QuestListClose);
        }
        // 帮助 Prguse2[257..259] @(266,3)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 257),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 258),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 259),
        ) {
            spawn_icon_button(p, n, h, pr, LIST_HELP_POS.0, LIST_HELP_POS.1, 24.0, 21.0, 12)
                .insert(QuestListHelp);
        }
        // 可接计数 @(210,8)
        spawn_label(
            p,
            &cjk,
            "",
            LIST_AVAILABLE_POS.0,
            LIST_AVAILABLE_POS.1,
            12.0,
            Color::WHITE,
            9,
        )
        .insert(QuestListAvailableLabel);
        // 上翻 Prguse[951..953] @(291,35)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 951),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 952),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 953),
        ) {
            spawn_icon_button(p, n, h, pr, LIST_UP_POS.0, LIST_UP_POS.1, 16.0, 16.0, 12)
                .insert(QuestListUp);
        }
        // 下翻 Prguse[957..959] @(291,83)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 957),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 958),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 959),
        ) {
            spawn_icon_button(p, n, h, pr, LIST_DOWN_POS.0, LIST_DOWN_POS.1, 16.0, 16.0, 12)
                .insert(QuestListDown);
        }
        // 离开 Title[276..278] @(205,436)
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 276),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 277),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, 278),
        ) {
            spawn_icon_button(p, n, h, pr, LIST_LEAVE_POS.0, LIST_LEAVE_POS.1, 76.0, 25.0, 12)
                .insert(QuestListLeave);
        }
        // 五行（C# `Rows[i]`：选中高亮 + 名称）
        for i in 0..LIST_ROW_COUNT {
            let y = LIST_ROW_ORIGIN.1 + i as f32 * LIST_ROW_DY;
            // 选中高亮 Prguse[956] @(25,0)（C# `SelectedImage`，默认隐藏）
            if let Some(mut mark) = spawn_image_native(
                p,
                &mut libs,
                &mut images,
                LibraryName::Prguse,
                956,
                LIST_ROW_ORIGIN.0 + LIST_ROW_SEL_X,
                y,
                10,
            ) {
                mark.insert((QuestListRowMark(i), Visibility::Hidden));
            }
            // 行名（C# `QuestRow.NameLabel @ (60,0)`；本端直接以整行做点击面，
            // 文字起点贴 (9,36+19i) 便于整屏定位工具读数）
            spawn_label(
                p,
                &cjk,
                "",
                LIST_ROW_ORIGIN.0,
                y,
                12.0,
                Color::WHITE,
                11,
            )
            .insert((QuestListRow(i), Visibility::Hidden));
        }

        // ===== 消息区（C# `QuestMessage`，`:180-193`）=====
        // 上滚 `Prguse2[197..199]` @(292,136)、下滚 `[207..209]` @(292,282)
        // （C# 显式 `Size=(16,14)` 被 `AutoSize=true` 顶掉 → 用图头 12x12，同详情窗口径）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 197),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 198),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 199),
        ) {
            spawn_icon_button(p, n, h, pr, LIST_MSG_UP_POS.0, LIST_MSG_UP_POS.1, 12.0, 12.0, 11)
                .insert(QuestListMsgUp);
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 207),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 208),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 209),
        ) {
            spawn_icon_button(
                p, n, h, pr, LIST_MSG_DOWN_POS.0, LIST_MSG_DOWN_POS.1, 12.0, 12.0, 11,
            )
            .insert(QuestListMsgDown);
        }
        // 位置条 `Prguse2[205/206]` @(292,149)（C# 起始 `Visible=false`；行数不足一页恒隐）
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 205),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206),
            load_lib_image(&mut libs, &mut images, LibraryName::Prguse2, 206),
        ) {
            spawn_icon_button(p, n, h, pr, LIST_MSG_BAR_POS.0, LIST_MSG_BAR_POS.1, 12.0, 18.0, 12)
                .insert((QuestListPart::Bar, Visibility::Hidden));
        }
        // 10 行标签（宽 280、高 20 裁剪；位置/字号/颜色每帧按 C# `NewText` 重算）
        for i in 0..LIST_MSG_LINE_COUNT {
            let (ox, oy) = LIST_MSG_ORIGIN;
            let y = oy + i as f32 * QUEST_MSG_LINE_DY;
            spawn_label(p, &cjk, "", ox, y, 12.0, Color::WHITE, 9).insert((
                QuestListPart::MsgLine(i),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(ox),
                    top: Val::Px(y),
                    width: Val::Px(LIST_MSG_W),
                    height: Val::Px(QUEST_MSG_LINE_H),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ));
        }
        // 标题行圆点 `Prguse[919]`（12x10；初始藏在面板外，逐帧按标题行落位）
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, 919) {
            for i in 0..LIST_MSG_LINE_COUNT {
                spawn_image(p, h.clone(), LIST_MSG_ORIGIN.0 + 5.0, -60.0, 12.0, 10.0, 8)
                    .insert((QuestListPart::Bullet(i), Visibility::Hidden));
            }
        }

        // ===== 奖励区（C# `QuestRewards` @(5,307) 313x130，`:196-201`/`:1396-1459`）=====
        let (rx, ry) = QUEST_REWARD_ORIGIN;
        // 奖励区标题 `Title[17] @ (20,66)`
        if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Title, 17) {
            spawn_image(p, h, rx + LIST_REWARD_TITLE_POS.0, ry + LIST_REWARD_TITLE_POS.1, 68.0, 16.0, 8);
        }
        // 经验/金币/信用 图标与数值（C# `QuestReward_BeforeDraw` 的 x 偏移链，`:1445-1459`）
        for (idx, kind) in [(966usize, 0u8), (965, 1), (2447, 2)] {
            if let Some(h) = load_lib_image(&mut libs, &mut images, LibraryName::Prguse, idx) {
                spawn_image(p, h, rx, ry, 16.0, 14.0, 8)
                    .insert((QuestListPart::RewardIcon(kind), Visibility::Hidden));
            }
        }
        for kind in 0u8..3 {
            spawn_label(p, &cjk, "", rx, ry, 12.0, Color::WHITE, 9)
                .insert((QuestListPart::RewardValue(kind), Visibility::Hidden));
        }
        // 奖励格：固定排 y=24、可选排 y=89，各 5 格（C# `FixedItems/SelectItems`）
        for fixed in [true, false] {
            for slot in 0..LIST_REWARD_CELL_COUNT {
                let (cx, cy) = reward_cell_offset(fixed, slot);
                let (x, y) = (rx + cx, ry + cy);
                // 底：固定排恒 `Prguse[989]`（画在 y-1），可选排在选中时画 `Prguse[979]`（y-5）
                let (mark_idx, dy) = if fixed { (989usize, -1.0) } else { (979, -5.0) };
                if let Some(mut m) = spawn_image_native(
                    p,
                    &mut libs,
                    &mut images,
                    LibraryName::Prguse,
                    mark_idx,
                    x,
                    y + dy,
                    8,
                ) {
                    m.insert((
                        QuestListPart::Mark { fixed, slot },
                        Visibility::Hidden,
                    ));
                }
                // 物品图标（白图占位，逐帧换成 `Items[item.image]`）
                if let Some(white) = load_lib_image(&mut libs, &mut images, LibraryName::Items, 0)
                {
                    p.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x),
                            top: Val::Px(y),
                            width: Val::Px(0.0),
                            height: Val::Px(0.0),
                            ..default()
                        },
                        ImageNode::new(white),
                        QuestListPart::Icon { fixed, slot },
                        Visibility::Hidden,
                        ZIndex(9),
                    ));
                }
                // 数量（C# `Count.ToString("###0")`，右下角）
                spawn_label_plain(p, &cjk, "", x + 30.0, y + 19.0, 12.0, Color::srgb(1.0, 1.0, 0.0), 10)
                    .insert((
                        QuestListPart::Count { fixed, slot },
                        Visibility::Hidden,
                    ));
            }
        }

        // ===== 接受（`Title[270..272]` @(40,436)）/ 完成（`Title[273..275]` @(40,436)）=====
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_ACCEPT_SPRITES.0),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_ACCEPT_SPRITES.1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_ACCEPT_SPRITES.2),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                LIST_ACCEPT_POS.0,
                LIST_ACCEPT_POS.1,
                68.0,
                25.0,
                12,
            )
            .insert((QuestListAccept, Visibility::Hidden));
        }
        if let (Some(n), Some(h), Some(pr)) = (
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_FINISH_SPRITES.0),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_FINISH_SPRITES.1),
            load_lib_image(&mut libs, &mut images, LibraryName::Title, LIST_FINISH_SPRITES.2),
        ) {
            spawn_icon_button(
                p,
                n,
                h,
                pr,
                LIST_ACCEPT_POS.0,
                LIST_ACCEPT_POS.1,
                68.0,
                25.0,
                12,
            )
            .insert((QuestListFinish, Visibility::Hidden));
        }
    });
}

/// 本窗每帧：显隐跟 `DialogManager`、绑定当前 NPC、按 C# `RefreshInterface` 渲染行。
/// 按钮/行的按下边沿：`Interaction::Pressed` 的**上升沿**才算一次点击
/// （与 `npc.rs`/`quest_log.rs` 同款；否则按住不放会每帧触发）。
fn edge(e: Entity, inter: &Interaction, prev: &mut HashMap<Entity, Interaction>) -> bool {
    let was = prev.insert(e, *inter);
    *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
}

#[allow(clippy::too_many_arguments)]
fn quest_list_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<QuestListState>,
    catalog: Res<QuestCatalog>,
    log: Res<QuestLogState>,
    mut npc: ResMut<NpcDialogState>,
    player_q: Query<
        (
            &crate::game::player_state::Progression,
            &crate::actor::ActorAppearance,
        ),
        With<crate::actor::LocalPlayer>,
    >,
    mut widgets: Query<&mut Visibility, With<QuestListWidget>>,
    mut rows: Query<
        (&mut Text, &mut Visibility, &QuestListRow),
        (Without<QuestListWidget>, Without<QuestListAvailableLabel>),
    >,
    mut marks: Query<
        (&mut Visibility, &QuestListRowMark),
        (Without<QuestListWidget>, Without<QuestListRow>),
    >,
    mut label: Query<
        &mut Text,
        (With<QuestListAvailableLabel>, Without<QuestListRow>),
    >,
    buttons: Query<(
        Entity,
        &Interaction,
        Option<&QuestListClose>,
        Option<&QuestListLeave>,
        Option<&QuestListHelp>,
        Option<&QuestListUp>,
        Option<&QuestListDown>,
    )>,
    mouse: Res<ButtonInput<MouseButton>>,
    // 命中判定走统一光标来源：**探针优先**（#2767）——行是自绘文本标签，
    // click/cursor RPC 注入的 PointerInput/HoverMap 到不了它，只有探针能进这条路。
    cursor_src: crate::control::CursorSource,
    ui: (Query<&Window>, Query<&Node, With<QuestListWidget>>),
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
) {
    let open = mgr.is_open(DialogKind::QuestList);
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

    // 绑定：C# `Show()` 里 `CurrentNPCID = GameScene.NPCID`
    state.bind(npc.npc_object_id);

    let (me_level, me_class) = player_q
        .single()
        .map(|(p, a)| (p.level, a.class as u8))
        .unwrap_or((1, 0));
    let list = npc_available_quests(&catalog, &log, state.bound_npc, me_level, me_class);
    state.clamp_start(list.len());
    // C# `UpdateRows`：未选中时默认选第一行（`:336-344`）
    if state.selected.is_none() {
        state.selected = list.first().map(|e| e.quest);
    }

    // 计数文案（C# `_availableQuestLabel.Text = AvailableQuestList(Quests.Count)`）
    for mut text in &mut label {
        text.0 = available_quest_label(list.len());
    }

    // 行渲染（C# `RefreshInterface`：`Rows[i].Quest = Quests[i + StartIndex]`）
    for (mut text, mut vis, row) in &mut rows {
        let entry = list.get(state.start + row.0).copied();
        text.0 = entry
            .and_then(|e| catalog.infos.iter().find(|c| c.index == e.quest))
            .map(|c| c.name.clone())
            .unwrap_or_default();
        // 越界行清空并隐藏；C# `RefreshInterface` 里 `Rows[i]` 是**新建控件**（按 `Quests.Count`
        // 逐个 new），本端是固定 5 个槽位，故显隐要显式写——**漏写就是「窗开着但一行字都看不见」**
        // （#3368 实机取证抓到：`catalog_infos=1 / selected=2` 却 row 区亮像素 0）
        *vis = if open && entry.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut vis, mark) in &mut marks {
        let entry = list.get(state.start + mark.0).copied();
        *vis = if entry.map(|e| Some(e.quest) == state.selected).unwrap_or(false) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 点行 → 选中（C# `Rows[i].Click`，`:323-337`）。
    // 行不是 bevy_ui 按钮（`QuestRow` 在 C# 是自绘控件），故按面板原点 + 行矩形命中。
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(cursor) = cursor_src.pos() {
            let (ox, oy) = ui
                .1
                .single()
                .map(|n| crate::ui::theme::node_origin(n, LIST_POS))
                .unwrap_or(LIST_POS);
            for i in 0..LIST_ROW_COUNT {
                let (rx, ry, rw, rh) = quest_list_row_rect(i, ox, oy);
                if cursor.x >= rx && cursor.x <= rx + rw && cursor.y >= ry && cursor.y <= ry + rh {
                    if let Some(e) = list.get(state.start + i) {
                        state.selected = Some(e.quest);
                    }
                }
            }
        }
    }

    // 钮：关闭 / 离开 / 帮助 / 上翻 / 下翻
    for (e, inter, close, leave, help, up, down) in &buttons {
        if !edge(e, inter, &mut prev_inter) {
            continue;
        }
        if close.is_some() || leave.is_some() {
            // C# `closeButton.Click += Hide()` / `leaveButton.Click += Hide()`，而
            // `QuestListDialog.Hide()` 里还连着 `GameScene.Scene.NPCDialog.Hide()`
            // （`QuestDialogs.cs:242-247`）——关列表窗必然把 NPC 会话窗一起关掉。
            mgr.close(DialogKind::QuestList);
            npc.visible = false;
        } else if help.is_some() {
            // C# `helpButton.Click += HelpDialog.DisplayPage(Quests)`（`:246-250`）
            mgr.open(DialogKind::Help);
        } else if up.is_some() {
            // C# `upQuestButton.Click`（`:53-63`）：选中行非首行则上移一行，否则整表上翻
            let sel_idx = state
                .selected
                .and_then(|q| list.iter().position(|e| e.quest == q));
            match sel_idx {
                Some(i) if i > state.start => state.selected = Some(list[i - 1].quest),
                _ => {
                    state.start = state.start.saturating_sub(1);
                    state.selected = list.get(state.start).map(|e| e.quest);
                }
            }
        } else if down.is_some() {
            // C# `downQuestButton.Click`（`:68-78`）：选中行非末行则下移一行，否则整表下翻
            let sel_idx = state
                .selected
                .and_then(|q| list.iter().position(|e| e.quest == q));
            match sel_idx {
                Some(i) if i < state.start + LIST_ROW_COUNT - 1 && i + 1 < list.len() => {
                    state.selected = Some(list[i + 1].quest)
                }
                _ => {
                    if state.start + LIST_ROW_COUNT < list.len() {
                        state.start += 1;
                        state.selected = list.get(state.start).map(|e| e.quest);
                    }
                }
            }
        }
    }
}

/// 单元② 用到的可变查询打包（`&mut Node`/`&mut Visibility`/`Option<&mut Text>`/`Option<&mut ImageNode>`
/// 集中在一个查询里，避免 Bevy B0001；见 `QuestListPart` 的注释）。
#[derive(SystemParam)]
struct QuestListDetail<'w, 's> {
    /// 消息行/圆点/位置条/奖励格/奖励图标与数值（单查询 + `QuestListPart` 分派）
    parts: Query<
        'w,
        's,
        (
            &'static QuestListPart,
            &'static mut Node,
            &'static mut Visibility,
            Option<&'static mut Text>,
            Option<&'static mut TextColor>,
            Option<&'static mut ImageNode>,
        ),
        (
            Without<QuestListWidget>,
            Without<QuestListAccept>,
            Without<QuestListFinish>,
            Without<QuestListMsgUp>,
            Without<QuestListMsgDown>,
        ),
    >,
    /// 消息区上/下滚钮 + 接受/完成钮（`&mut ImageNode` 只在这里，故要对 `parts` 反向排除）
    buttons: Query<
        'w,
        's,
        (
            Entity,
            &'static Interaction,
            Option<&'static QuestListMsgUp>,
            Option<&'static QuestListMsgDown>,
            Option<&'static QuestListAccept>,
            Option<&'static QuestListFinish>,
            &'static mut Visibility,
            Option<&'static mut ImageNode>,
        ),
        Without<QuestListPart>,
    >,
    /// 面板原点（拖窗后按节点实测原点换算消息区/奖励格的屏幕位置）
    panel: Query<'w, 's, &'static Node, (With<QuestListWidget>, Without<QuestListPart>)>,
    wheels: MessageReader<'w, 's, MouseWheel>,
}

/// 单元②：消息区（C# `QuestMessage`）+ 奖励区（`QuestRewards`）+ 接受/完成钮（`ReDisplayButtons`）。
#[allow(clippy::too_many_arguments)]
fn quest_list_detail_system(
    mut state: ResMut<QuestListState>,
    mgr: Res<DialogManager>,
    catalog: Res<QuestCatalog>,
    log: Res<QuestLogState>,
    net: Res<NetConnection>,
    player: Query<
        (
            &crate::game::player_state::Progression,
            &crate::actor::ActorAppearance,
        ),
        With<crate::actor::LocalPlayer>,
    >,
    mut detail: QuestListDetail,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<crate::ui::sprite_ui::UiImageCache>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_src: crate::control::CursorSource,
    mut prev_inter: Local<HashMap<Entity, Interaction>>,
    mut last_selected: Local<Option<i32>>,
) {
    let open = mgr.is_open(DialogKind::QuestList);

    // 换选中任务 → C# `Message.UpdateQuest`（NewText(resetIndex:true) → TopLine=0）
    // 与 `Reward.CleanRewards()`（SelectedItemIndex=-1）（`QuestDialogs.cs:1217-1221`/`:1465-1477`）
    if *last_selected != state.selected {
        *last_selected = state.selected;
        state.top_line = 0;
        state.selected_reward = None;
    }

    let info = state
        .selected
        .and_then(|id| catalog.infos.iter().find(|c| c.index == id));
    let lines: Vec<String> = state
        .selected
        .map(|id| npc_quest_message_lines(&catalog, &log, state.bound_npc, id))
        .unwrap_or_default();
    if state.top_line + LIST_MSG_LINE_COUNT > lines.len() {
        state.top_line = lines.len().saturating_sub(LIST_MSG_LINE_COUNT);
    }
    let top = state.top_line;

    // 奖励偏移链（C# `quest_reward_offsets`，`:1424-1456`）
    let (exp, gold, credit) = info
        .map(|i| (i.reward_exp, i.reward_gold, i.reward_credit))
        .unwrap_or((0, 0, 0));
    let offs: QuestRewardOffsets = quest_reward_offsets(exp, gold);
    let (me_level, me_class, gender) = player
        .single()
        .map(|(p, a)| (p.level, a.class as u8, a.gender))
        .unwrap_or((1, 0, mir2_shared::enums::MirGender::Male));
    // 固定排不过滤、可选排过滤（C# `:1533-1553`，固定排那行 `FilterRewards` 被注释掉）
    let fixed: Vec<&mir2_shared::data::shared_data::QuestItemReward> = info
        .map(|i| i.rewards_fixed_item.iter().collect())
        .unwrap_or_default();
    let select: Vec<(usize, &mir2_shared::data::shared_data::QuestItemReward)> = info
        .map(|i| {
            i.rewards_select_item
                .iter()
                .enumerate()
                .filter(|(_, r)| quest_reward_visible_for_gender(&r.item, gender))
                .collect()
        })
        .unwrap_or_default();

    let (rx, ry) = QUEST_REWARD_ORIGIN;
    let (ox, oy) = LIST_MSG_ORIGIN;
    let panel_origin = detail
        .panel
        .single()
        .map(|n| crate::ui::theme::node_origin(n, LIST_POS))
        .unwrap_or(LIST_POS);
    let cursor = cursor_src.pos();
    let adjust_at = |idx: usize| -> f32 {
        QUEST_MSG_TITLE_DY
            * (top..idx.min(lines.len()))
                .filter(|i| quest_line_is_title(*i, &lines[*i]))
                .count() as f32
    };
    let bar_y = quest_list_msg_bar_y(top, lines.len(), LIST_MSG_LINE_COUNT);
    let show = |b: bool| {
        if open && b {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
    };
    let reward_at = |fixed_cell: bool,
                     slot: usize|
     -> Option<&mir2_shared::data::shared_data::QuestItemReward> {
        if fixed_cell {
            fixed.get(slot).copied()
        } else {
            select.get(slot).map(|(_, r)| *r)
        }
    };

    // ---- 滚轮（C# `QuestMessage_MouseWheel`，`:1082-1098`；仅光标在消息区内生效）----
    let mut wheel_count = 0i32;
    for ev in detail.wheels.read() {
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
                    && cur.x <= panel_origin.0 + ox + LIST_MSG_W
                    && cur.y >= panel_origin.1 + oy
                    && cur.y <= panel_origin.1 + oy + 160.0
            })
            .unwrap_or(false);
        if inside {
            wheel_count += c;
        }
    }
    if open && wheel_count != 0 {
        state.top_line =
            quest_msg_wheel_top_line(state.top_line, wheel_count, lines.len(), LIST_MSG_LINE_COUNT);
    }
    let top = state.top_line;

    // ---- 部件渲染 ----
    for (part, mut node, mut vis, text, color, image) in &mut detail.parts {
        match *part {
            QuestListPart::MsgLine(slot) => {
                let idx = top + slot;
                let is_title = lines
                    .get(idx)
                    .map(|l| quest_line_is_title(idx, l))
                    .unwrap_or(false);
                node.left = Val::Px(ox + if is_title { QUEST_MSG_TITLE_INDENT } else { 0.0 });
                node.top =
                    Val::Px(oy + (idx.saturating_sub(top)) as f32 * QUEST_MSG_LINE_DY + adjust_at(idx));
                *vis = show(true);
                if let Some(mut t) = text {
                    t.0 = lines.get(idx).cloned().unwrap_or_default();
                }
                if let Some(mut c) = color {
                    // C# `NewText`：首行黄、标题行白粗体、正文白（`:1242-1251`）
                    c.0 = if idx == 0 {
                        Color::srgb(1.0, 1.0, 0.0)
                    } else {
                        Color::WHITE
                    };
                }
            }
            QuestListPart::Bullet(slot) => {
                let idx = top + slot;
                let on = lines
                    .get(idx)
                    .map(|l| idx == 0 || quest_line_is_title(idx, l))
                    .unwrap_or(false);
                *vis = show(on);
                node.left = Val::Px(ox + 5.0);
                node.top = Val::Px(
                    oy + 5.0 + (idx.saturating_sub(top)) as f32 * QUEST_MSG_LINE_DY + adjust_at(idx),
                );
            }
            QuestListPart::Bar => match bar_y {
                Some(y) => {
                    *vis = show(true);
                    node.left = Val::Px(LIST_MSG_BAR_POS.0);
                    node.top = Val::Px(y as f32);
                }
                None => *vis = Visibility::Hidden,
            },
            QuestListPart::RewardIcon(kind) => {
                let (on, x) = match kind {
                    0 => (exp > 0, rx + 10.0),
                    1 => (gold > 0, rx + 100.0 + offs.gold),
                    _ => (credit > 0, rx + 190.0 + offs.credit),
                };
                *vis = show(on);
                node.left = Val::Px(x);
                node.top = Val::Px(ry + 2.0);
            }
            QuestListPart::RewardValue(kind) => {
                let (on, x, value) = match kind {
                    0 => (exp > 0, rx + 40.0, exp),
                    1 => (gold > 0, rx + 120.0 + offs.gold, gold),
                    _ => (credit > 0, rx + 210.0 + offs.credit, credit),
                };
                *vis = show(on);
                node.left = Val::Px(x);
                node.top = Val::Px(ry);
                if let Some(mut t) = text {
                    t.0 = if on {
                        value.to_string()
                    } else {
                        String::new()
                    };
                }
            }
            QuestListPart::Mark { fixed: f, slot } => {
                // C#：固定格恒画 `Prguse[989]`；可选格仅在选中时画 `Prguse[979]`（`:1690-1696`）
                let has = reward_at(f, slot).is_some();
                *vis = show(has && (f || state.selected_reward == select.get(slot).map(|(i, _)| *i)));
            }
            QuestListPart::Icon { fixed: f, slot } => {
                let Some(r) = reward_at(f, slot) else {
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
                let (cx, cy) = reward_cell_offset(f, slot);
                let (iox, ioy) = crate::game::dialogs::quest_log::quest_reward_item_offset(w, h);
                node.left = Val::Px(rx + cx + iox);
                node.top = Val::Px(ry + cy + ioy);
                node.width = Val::Px(w.max(0) as f32);
                node.height = Val::Px(h.max(0) as f32);
                if let Some(mut img) = image {
                    if img.image != handle {
                        img.image = handle;
                    }
                }
                *vis = show(true);
            }
            QuestListPart::Count { fixed: f, slot } => {
                let count = reward_at(f, slot).map(|r| r.count).unwrap_or(0);
                let on = count > 1;
                *vis = show(on);
                if let Some(mut t) = text {
                    t.0 = if on { count.to_string() } else { String::new() };
                }
            }
        }
    }

    // ---- 可选奖励格点击 = 多选一（C# `SelectItems[i].Click`，`:1497-1515`）----
    if open && mouse.just_pressed(MouseButton::Left) {
        if let Some(cur) = cursor {
            for slot in 0..LIST_REWARD_CELL_COUNT {
                let (cx, cy) = reward_cell_offset(false, slot);
                let (x, y) = (panel_origin.0 + rx + cx, panel_origin.1 + ry + cy);
                if cur.x >= x && cur.x <= x + 32.0 && cur.y >= y && cur.y <= y + 32.0 {
                    if let Some((idx, r)) = select.get(slot) {
                        state.selected_reward = Some(*idx);
                        tracing::info!(
                            "🎁 选择奖励：{}（未过滤下标 {}）",
                            reward_item_display_with_catalog(&catalog, r),
                            idx
                        );
                    }
                }
            }
        }
    }

    // ---- 钮：上/下滚、接受、完成 ----
    let list = npc_available_quests(&catalog, &log, state.bound_npc, me_level, me_class);
    let entry = state
        .selected
        .and_then(|id| list.iter().find(|e| e.quest == id).copied());
    let (show_accept, show_finish) = quest_list_buttons(entry.as_ref(), log.quests.len());
    for (e, inter, up, down, accept, finish, mut vis, img) in &mut detail.buttons {
        if up.is_some() || down.is_some() {
            if open && edge(e, inter, &mut prev_inter) {
                let t = state.top_line;
                if up.is_some() {
                    state.top_line = quest_msg_scroll_up(t);
                } else {
                    state.top_line = quest_msg_scroll_down(t, lines.len(), LIST_MSG_LINE_COUNT);
                }
            }
            continue;
        }
        let (on, enabled) = if accept.is_some() {
            (show_accept, true)
        } else if finish.is_some() {
            (show_finish, entry.as_ref().map(|x| x.completed).unwrap_or(false))
        } else {
            continue;
        };
        // C# `ReDisplayButtons`（`:402-425`）逐个置 `Visible`；两钮共用同一落点
        // （`_acceptButton`/`_finishButton` 同 `Location = (40,436)`），靠显隐二选一
        *vis = if open && on {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(mut i) = img {
            i.color = if enabled {
                Color::WHITE
            } else {
                Color::srgb(0.45, 0.45, 0.45)
            };
        }
        if !open || !on || !edge(e, inter, &mut prev_inter) {
            continue;
        }
        let Some(info) = info else { continue };
        if accept.is_some() {
            // C# `_acceptButton.Click`（`:86-95`）：`Reward == null || Taken` 时直接返回
            if entry.as_ref().map(|x| x.taken).unwrap_or(true) {
                continue;
            }
            net.send_packet(&mir2_shared::packets::client::quest::AcceptQuest {
                npc_index: info.npc_index,
                quest_index: info.index,
            });
            tracing::info!("📜 接受任务 #{} {}（NPC {}）", info.index, info.name, info.npc_index);
        } else if finish.is_some() {
            // C# `_finishButton.Click`（`:121-141`）：未完成直接返回；有可选奖励未选则弹提示框
            if !entry.as_ref().map(|x| x.completed).unwrap_or(false) {
                continue;
            }
            match crate::game::dialogs::quest_log::finish_selected_index(
                &info.rewards_select_item,
                state.selected_reward,
            ) {
                Ok(selected) => {
                    net.send_packet(&mir2_shared::packets::client::quest::FinishQuest {
                        quest_index: info.index,
                        selected_item_index: selected,
                    });
                    tracing::info!(
                        "📜 交付任务 #{} {}（选定奖励下标 {}）",
                        info.index,
                        info.name,
                        selected
                    );
                }
                Err(msg) => {
                    // C# 这里弹 `MirMessageBox(YouMustSelectRewardItem)`；本端只拦发包 + 记日志
                    // （提示框留待后续单元，见 README §3.2ax）
                    tracing::info!("📜 未选定奖励物品，不发送 FinishQuest：{msg}");
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::dialogs::quest_log::QuestEntry;
    use mir2_shared::data::client_data::ClientQuestInfo;
    use mir2_shared::enums::{QuestType, RequiredClass};

    const NPC_A: u32 = 4242;
    const NPC_B: u32 = 7777;

    fn info(index: i32, npc_index: u32, finish_npc_index: u32, min_level: i32) -> ClientQuestInfo {
        ClientQuestInfo {
            index,
            npc_index,
            name: format!("任务{}", index),
            group: String::new(),
            description: vec![],
            task_description: vec![],
            return_description: vec![],
            completion_description: vec![],
            min_level_needed: min_level,
            max_level_needed: 0,
            quest_needed: 0,
            class_needed: RequiredClass::from_bits_truncate(0),
            quest_type: QuestType::General,
            time_limit_in_seconds: 0,
            reward_gold: 0,
            reward_exp: 0,
            reward_credit: 0,
            rewards_fixed_item: vec![],
            rewards_select_item: vec![],
            finish_npc_index,
        }
    }

    fn entry(id: i32, completed: bool) -> QuestEntry {
        QuestEntry {
            id,
            name: format!("任务{}", id),
            tasks: vec![],
            taken: true,
            completed,
            is_new: false,
        }
    }

    fn catalog(infos: Vec<ClientQuestInfo>) -> QuestCatalog {
        QuestCatalog {
            infos,
            ..Default::default()
        }
    }

    /// C# `NPCObject.GetAvailableQuests`（`Client/MirObjects/NPCObject.cs:390-424`）：
    /// ① 已接且交付 NPC 是本 NPC 的排在前；② 本 NPC 提供且可接的排在后；别人家的不入表。
    #[test]
    fn npc_available_quests_matches_csharp_get_available_quests() {
        let cat = catalog(vec![
            info(1, NPC_A, NPC_A, 1),   // 本 NPC 可接
            info(2, NPC_B, NPC_B, 1),   // 别家 NPC
            info(3, NPC_B, NPC_A, 1),   // 别家接、本 NPC 交
            info(4, NPC_A, NPC_A, 60),  // 本 NPC 但等级不够
        ]);
        let log = QuestLogState {
            quests: vec![entry(3, true), entry(9, false)],
            ..Default::default()
        };

        let list = npc_available_quests(&cat, &log, NPC_A, 10, 0);
        let ids: Vec<i32> = list.iter().map(|e| e.quest).collect();
        // ①：任务 3（已接、完成待交付、finish = 本 NPC）在前
        // ②：任务 1（本 NPC 可接）；任务 4 等级不足被 CanAccept 挡掉；任务 2 不是本 NPC 的
        assert_eq!(ids, vec![3, 1]);
        assert!(list[0].taken && list[0].completed, "①必须是已接+已完成态");
        assert!(!list[1].taken, "②是可接：未接");
    }

    /// ②里的重复收录要挡掉（C# `!quests.Exists(p => p.QuestInfo.Index == q.Index)`）。
    #[test]
    fn npc_available_quests_dedups_taken_and_offered() {
        // finish 列回退到 npc_index（C# getter）⇒ 任务 1 既走①也符合②的选取条件
        let cat = catalog(vec![info(1, NPC_A, 0, 1)]);
        let log = QuestLogState {
            quests: vec![entry(1, false)],
            ..Default::default()
        };
        let list = npc_available_quests(&cat, &log, NPC_A, 10, 0);
        assert_eq!(list.len(), 1, "①已收录后②不得再收一遍");
        assert!(list[0].taken && !list[0].completed);
    }

    /// 本次会话已交（`QuestCatalog.completed`）的两侧都不再列（C# `User.CompletedQuests.Contains`）。
    #[test]
    fn npc_available_quests_skips_completed() {
        let mut cat = catalog(vec![info(1, NPC_A, NPC_A, 1)]);
        cat.completed.insert(1);
        let log = QuestLogState {
            quests: vec![entry(1, true)],
            ..Default::default()
        };
        assert!(npc_available_quests(&cat, &log, NPC_A, 99, 0).is_empty());
    }

    /// `npc_object_id = 0` = 本端「未开对话」哨兵（C# `MapControl.GetObject(0) == null`）：
    /// 不能把「没有 NPC 关联、服务端下发 `npc_index=0`」的任务当成这只 NPC 的。
    #[test]
    fn npc_available_quests_is_empty_without_npc() {
        let cat = catalog(vec![info(1, 0, 0, 1)]);
        let log = QuestLogState::default();
        assert!(npc_available_quests(&cat, &log, 0, 99, 0).is_empty());
    }

    /// 行版式：C# `Location = (9, 36 + i*19)`、`Size = (200,17)`（`QuestDialogs.cs:322`/`:927`）。
    #[test]
    fn row_rect_follows_csharp_questrow_layout() {
        let (x, y, w, h) = quest_list_row_rect(0, 487.0, 0.0);
        assert_eq!((x, y, w, h), (496.0, 36.0, 200.0, 17.0));
        let (_, y4, _, _) = quest_list_row_rect(4, 487.0, 0.0);
        assert_eq!(y4, 36.0 + 4.0 * 19.0);
    }

    /// C# `Show()` 换 NPC 才重置（`CurrentNPCID`/`Reset`，`QuestDialogs.cs:254-263`）。
    #[test]
    fn state_bind_resets_only_on_npc_change() {
        let mut s = QuestListState::default();
        s.bind(NPC_A);
        s.selected = Some(5);
        s.start = 3;
        s.bind(NPC_A);
        assert_eq!((s.selected, s.start), (Some(5), 3), "同一 NPC 重开不重置");
        s.bind(NPC_B);
        assert_eq!((s.bound_npc, s.selected, s.start), (NPC_B, None, 0));
    }

    /// 五行窗口的 `StartIndex` 夹取（C# `maxIndex = Quests.Count - Rows.Length`）。
    #[test]
    fn clamp_start_matches_csharp_max_index() {
        let mut s = QuestListState {
            start: 9,
            ..Default::default()
        };
        s.clamp_start(6); // maxIndex = 1
        assert_eq!(s.start, 1);
        s.clamp_start(3); // maxIndex = 0
        assert_eq!(s.start, 0);
        s.clamp_start(0);
        assert_eq!(s.start, 0);
    }

    /// 文案取 `ClientTextKeys.AvailableQuestList`（`Client/Localization/Chinese.json:682`）。
    #[test]
    fn available_label_matches_chinese_localization() {
        assert_eq!(available_quest_label(0), "可接任务列表：0");
        assert_eq!(available_quest_label(3), "可接任务列表：3");
    }

    // ---- 单元②：消息区 / 奖励区 / 接受完成钮 ----

    fn info_with_lines(index: i32, npc: u32, finish: u32) -> ClientQuestInfo {
        ClientQuestInfo {
            description: vec!["去村口找张三".to_string()],
            task_description: vec!["击杀稻草人 0/3".to_string()],
            return_description: vec!["把信交给李四".to_string()],
            completion_description: vec!["你做到了".to_string()],
            ..info(index, npc, finish, 1)
        }
    }

    /// C# `QuestMessage.UpdateQuest`（`:1142-1213`）：未接 → 描述 + 任务/交付段；
    /// 已接且交付 NPC 是本 NPC 且有完成描述 → 只显完成描述。
    #[test]
    fn message_lines_follow_csharp_update_quest() {
        let cat = catalog(vec![
            // 接/交同一个 NPC（`same_finish_npc`）→ 走描述分支
            info_with_lines(1, NPC_A, NPC_A),
            // 别家接、本 NPC 交（`same_finish_npc = false`）→ 已接时走完成描述分支
            info_with_lines(2, NPC_B, NPC_A),
        ]);
        let log = QuestLogState {
            quests: vec![entry(2, false)],
            ..Default::default()
        };

        let open_lines = npc_quest_message_lines(&cat, &log, NPC_A, 1);
        assert_eq!(open_lines[0], "任务1", "首行 = 任务名");
        assert!(open_lines.contains(&"击杀稻草人 0/3".to_string()), "带任务段");
        assert!(open_lines.contains(&"把信交给李四".to_string()), "带交付段");
        assert!(!open_lines.contains(&"你做到了".to_string()), "同交付 NPC 时不显完成描述");

        let turn_in = npc_quest_message_lines(&cat, &log, NPC_A, 2);
        assert_eq!(turn_in, vec!["任务2".to_string(), "你做到了".to_string()]);

        // 不是本 NPC（NPCIndex/FinishNPCIndex 都对不上）→ 不显完成描述，回落到描述 + 任务段
        let elsewhere = npc_quest_message_lines(&cat, &log, NPC_B, 2);
        assert!(elsewhere.contains(&"击杀稻草人 0/3".to_string()));
        assert!(!elsewhere.contains(&"你做到了".to_string()));

        // 目录里没有的任务 → 空行表（不发包也不画字）
        assert!(npc_quest_message_lines(&cat, &log, NPC_A, 999).is_empty());
    }

    /// C# `ReDisplayButtons`（`:402-425`）：接受 = 未接且未达并发上限；完成 = 已达成。
    #[test]
    fn buttons_follow_csharp_redisplaybuttons() {
        let open_quest = NpcQuestEntry {
            quest: 1,
            taken: false,
            completed: false,
        };
        assert_eq!(quest_list_buttons(Some(&open_quest), 0), (true, false));
        // 已达 `Globals.MaxConcurrentQuests` → 接受钮不显示
        assert_eq!(
            quest_list_buttons(Some(&open_quest), MAX_CONCURRENT_QUESTS),
            (false, false)
        );
        let in_progress = NpcQuestEntry {
            quest: 2,
            taken: true,
            completed: false,
        };
        assert_eq!(quest_list_buttons(Some(&in_progress), 0), (false, false));
        let finishable = NpcQuestEntry {
            quest: 3,
            taken: true,
            completed: true,
        };
        assert_eq!(quest_list_buttons(Some(&finishable), 0), (false, true));
        assert_eq!(quest_list_buttons(None, 0), (false, false));
    }

    /// C# `QuestCell.Location`：固定排 `(i*45+15, 24)`、可选排 `(i*45+15, 89)`（`:1537`/`:1561`）。
    #[test]
    fn reward_cells_match_csharp_locations() {
        assert_eq!(reward_cell_offset(true, 0), (15.0, 24.0));
        assert_eq!(reward_cell_offset(true, 4), (195.0, 24.0));
        assert_eq!(reward_cell_offset(false, 0), (15.0, 89.0));
        assert_eq!(reward_cell_offset(false, 4), (195.0, 89.0));
    }

    /// C# `QuestMessage.UpdatePositionBar`（`:1120-1140`）：不足一页 → `None`（条隐藏）；
    /// 否则按 `(PosMaxY-PosMinY)/(行数-行高)` 步进并钳在 149..263。
    #[test]
    fn position_bar_matches_csharp() {
        assert_eq!(quest_list_msg_bar_y(0, 10, LIST_MSG_LINE_COUNT), None);
        // 20 行、窗 10 行：span=10，interval=(263-149)/10=11
        assert_eq!(quest_list_msg_bar_y(0, 20, LIST_MSG_LINE_COUNT), Some(149));
        assert_eq!(quest_list_msg_bar_y(1, 20, LIST_MSG_LINE_COUNT), Some(160));
        // 越界顶行仍钳在 PosMaxY
        assert_eq!(quest_list_msg_bar_y(99, 20, LIST_MSG_LINE_COUNT), Some(263));
        assert_eq!(
            quest_list_msg_bar_y(0, LIST_MSG_LINE_COUNT, LIST_MSG_LINE_COUNT),
            None
        );
    }
}
