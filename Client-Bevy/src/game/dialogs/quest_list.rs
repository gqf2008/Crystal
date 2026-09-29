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
use std::collections::HashMap;

use crate::game::dialogs::npc::NpcDialogState;
use crate::game::dialogs::quest_log::{
    row_action, QuestCatalog, QuestLogState, QuestRowAction, CLOSE_POS,
};
use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{shared_cjk_font, UiCjkFont, UiFont};
use crate::ui::theme::{
    load_lib_image, spawn_close_button, spawn_icon_button, spawn_image, spawn_image_native,
    spawn_label, spawn_panel,
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

/// C# `QuestListDialog.StartIndex` / `SelectedIndex` / `CurrentNPCID`（`:26-32`）的打包。
#[derive(Resource, Default)]
pub struct QuestListState {
    /// C# `CurrentNPCID`：本窗绑定到的 NPC object_id（0 = 未绑定）
    pub bound_npc: u32,
    /// C# `SelectedQuest.QuestInfo.Index`（选中任务；None = 尚未选中）
    pub selected: Option<i32>,
    /// C# `StartIndex`：五行窗口的首行下标
    pub start: usize,
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
    }

    /// C# `RefreshInterface` 的 `maxIndex` 夹取（`:299-303`）。
    pub fn clamp_start(&mut self, len: usize) {
        let max_index = len.saturating_sub(LIST_ROW_COUNT);
        if self.start > max_index {
            self.start = max_index;
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
            quest_list_ui_system.run_if(in_state(AppState::Game)),
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
    });
}

/// 本窗每帧：显隐跟 `DialogManager`、绑定当前 NPC、按 C# `RefreshInterface` 渲染行。
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
        (&mut Text, &QuestListRow),
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
    fn edge(
        e: Entity,
        inter: &Interaction,
        prev: &mut HashMap<Entity, Interaction>,
    ) -> bool {
        let was = prev.insert(e, *inter);
        *inter == Interaction::Pressed && was != Some(Interaction::Pressed)
    }

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
    for (mut text, row) in &mut rows {
        let entry = list.get(state.start + row.0).copied();
        text.0 = entry
            .and_then(|e| catalog.infos.iter().find(|c| c.index == e.quest))
            .map(|c| c.name.clone())
            .unwrap_or_default();
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
}
