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

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::game::dialogs::{DialogKind, DialogManager, DialogRoot};
use crate::map_renderer::GameLibraries;
use crate::network::NetConnection;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{
    spawn_ui_sprite, spawn_ui_text, ui_button_system, ui_image, UiButton, UiFont, UiImageCache,
};
use mir2_shared::data::client_data::ClientQuestInfo;
use mir2_shared::data::shared_data::QuestItemReward;

/// C# Globals.MaxConcurrentQuests（服务端 quest_log.can_accept 同值）
pub const MAX_CONCURRENT_QUESTS: usize = 20;

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

pub struct QuestLogPlugin;

impl Plugin for QuestLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLogState>();
        app.init_resource::<QuestCatalog>();
        app.add_systems(
            Update,
            quest_log_server_events.run_if(in_state(AppState::Game)),
        );
        app.add_systems(OnEnter(AppState::Game), spawn_quest_log);
        app.add_systems(OnExit(AppState::Game), cleanup_quest_log);
        app.add_systems(
            Update,
            (quest_log_ui_system, ui_button_system)
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
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 961) {
        let e = spawn_ui_sprite(&mut commands, h, 200.0, 60.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
            Visibility::Hidden,
        ));
    }
    // 标题 Title[15]（C# QuestDiaryDialog：Title[15] @(18,9)，绘制「QUEST DIARY」）
    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Title, 15) {
        let e = spawn_ui_sprite(&mut commands, h, 218.0, 69.0, 6.1, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Prguse2,
        360,
        361,
        362,
        489.0,
        63.0,
        7.0,
        20.0,
        20.0,
    ) {
        commands.entity(e).insert((
            QuestLogClose,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // 底部关闭按钮（C# QuestDiaryDialog：Title[193/194/195] @(200,436) 相对）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        193,
        194,
        195,
        400.0,
        496.0,
        7.1,
        76.0,
        25.0,
    ) {
        commands.entity(e).insert((
            QuestLogClose,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // 任务行 8 + 详情 6（#2535：8 详情名 / 9 任务 / 10 奖励 / 11 固定 / 12 可选 / 13 消息）
    for i in 0..14usize {
        let e = spawn_ui_text(
            &mut commands,
            &font,
            "",
            218.0,
            100.0 + i as f32 * 20.0,
            12.0,
            Color::WHITE,
            8.0,
        );
        commands.entity(e).insert((
            QuestLogLine(i),
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // 每行追踪按钮（对齐 C# QuestRow Track）
    for i in 0..8usize {
        let e = spawn_ui_text(
            &mut commands,
            &font,
            "追踪",
            450.0,
            100.0 + i as f32 * 20.0,
            11.0,
            Color::srgb(0.6, 0.9, 1.0),
            8.1,
        );
        commands.entity(e).insert((
            QuestLogTrack(i),
            UiButton {
                rect: (450.0, 100.0 + i as f32 * 20.0, 40.0, 18.0),
                clicked: false,
            },
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // #2535 子批2：已接计数标签（C# _takenQuestsLabel @标题栏 (210,7)）
    let e = spawn_ui_text(
        &mut commands,
        &font,
        "",
        218.0,
        80.0,
        12.0,
        Color::WHITE,
        8.0,
    );
    commands.entity(e).insert((
        QuestLogLine(14),
        DialogRoot(DialogKind::QuestLog),
        QuestLogWidget,
    ));
    // 放弃按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        206,
        207,
        208,
        400.0,
        345.0,
        8.3,
        76.0,
        25.0,
    ) {
        commands.entity(e).insert((
            QuestLogAbandon,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // #2535 接受/完成按钮（C# Title[270-272]/[273-275]；初始隐藏，由状态机驱动显隐/置灰）
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        270,
        271,
        272,
        220.0,
        345.0,
        8.4,
        76.0,
        25.0,
    ) {
        commands.entity(e).insert((
            QuestLogAccept,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands,
        &mut libs,
        &mut images,
        &mut cache,
        LibraryName::Title,
        273,
        274,
        275,
        310.0,
        345.0,
        8.4,
        76.0,
        25.0,
    ) {
        commands.entity(e).insert((
            QuestLogFinish,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
            Visibility::Hidden,
        ));
    }
}

/// 奖励物品显示名（目录无 ItemLibrary 全量名，未知名回退 物品#索引）
fn reward_item_display(catalog: &QuestCatalog, r: &QuestItemReward) -> String {
    catalog
        .item_names
        .get(&r.item_index)
        .cloned()
        .unwrap_or_else(|| format!("物品#{}", r.item_index))
        + &format!("×{}", r.count)
}

/// 显隐 + 渲染 + 选择 + 接受/完成/放弃（#2535）
#[allow(clippy::too_many_arguments)]
fn quest_log_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<QuestLogState>,
    catalog: Res<QuestCatalog>,
    hud: Res<crate::game::hud::HudState>,
    mut tracking: ResMut<crate::game::dialogs::quest_tracking::QuestTrackingState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<QuestLogClose>>,
    abandon_btn: Query<&UiButton, With<QuestLogAbandon>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    // #1290：Bevy B0001——两个 &mut Text2d Query 需用 Without 隔离（#1226 任务追踪合并后启动 panic）
    // #2535：接受/完成按钮独立 Query 改 Visibility，与 widgets 需互相 Without 隔离
    mut widgets: Query<
        &mut Visibility,
        (
            With<QuestLogWidget>,
            Without<QuestLogAccept>,
            Without<QuestLogFinish>,
        ),
    >,
    mut lines: Query<(&mut Text2d, &mut TextColor, &QuestLogLine), Without<QuestLogTrack>>,
    mut track_btns: Query<(&mut Text2d, &UiButton, &QuestLogTrack), Without<QuestLogLine>>,
    mut accept_btns: Query<
        (&UiButton, &mut Sprite, &mut Visibility),
        (With<QuestLogAccept>, Without<QuestLogFinish>),
    >,
    mut finish_btns: Query<
        (&UiButton, &mut Sprite, &mut Visibility),
        (With<QuestLogFinish>, Without<QuestLogAccept>),
    >,
) {
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
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::QuestLog);
        }
    }

    // #2535 可接任务段（已接在前、可接在后）
    let avail = available_quests(&catalog, &state, hud.level, hud.class);
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
                        .map(|c| (hud.level as i32 - c.min_level_needed) > 10)
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
                            .map(|r| reward_item_display(&catalog, r))
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
                                let s = reward_item_display(&catalog, r);
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
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..8usize {
                    let y = 100.0 + i as f32 * 20.0;
                    if cursor.x >= 218.0
                        && cursor.x <= 500.0
                        && cursor.y >= y
                        && cursor.y <= y + 18.0
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
                                tracing::info!("📜 选中任务: {}", state.quests[*qi].name);
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
                        let y = 100.0 + 12.0 * 20.0;
                        if cursor.x >= 218.0
                            && cursor.x <= 398.0
                            && cursor.y >= y
                            && cursor.y <= y + 18.0
                        {
                            let n = info.rewards_select_item.len() as f32;
                            let k = (((cursor.x - 218.0) / ((398.0 - 218.0) / n)) as usize)
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
    for (mut text, btn, track) in &mut track_btns {
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
        if btn.clicked {
            if let Some(q) = quest {
                let now_tracked = tracking.toggle(q.id);
                tracking.save();
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
    for (btn, mut sprite, mut vis) in &mut accept_btns {
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
        sprite.color = Color::WHITE;
        if show && btn.clicked {
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
    for (btn, mut sprite, mut vis) in &mut finish_btns {
        let sel = state.selected.and_then(|i| state.quests.get(i)).cloned();
        let show = sel.is_some();
        let enabled = sel.as_ref().map(|q| q.completed).unwrap_or(false);
        *vis = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        sprite.color = if enabled {
            Color::WHITE
        } else {
            Color::srgb(0.45, 0.45, 0.45)
        };
        if show && enabled && btn.clicked {
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
    // 放弃选中任务
    for btn in &abandon_btn {
        if btn.clicked {
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
            item_index,
            count: 1,
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
}
