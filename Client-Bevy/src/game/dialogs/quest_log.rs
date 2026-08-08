// ============================================================================
// 任务日志对话框（M43）
// 参考：C# QuestLogDialog + ServerRust quest.rs
// 网络：
//   C: AcceptQuest[npc_index u32][quest_index i32] / FinishQuest / AbandonQuest[i32]
//   S: ChangeQuest[id i32][count i32][task dotnet...][taken u8][completed u8][new u8]
//      CompleteQuest[quest_index i32]
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
    pub message: String,
}

#[derive(Component)]
pub struct QuestLogWidget;

#[derive(Component)]
pub struct QuestLogClose;

#[derive(Component)]
pub struct QuestLogAbandon;

#[derive(Component)]
pub struct QuestLogLine(usize);

/// 每行“追踪/取消追踪”按钮（C# QuestRow Track 按钮）
#[derive(Component)]
pub struct QuestLogTrack(usize);

pub struct QuestLogPlugin;

impl Plugin for QuestLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLogState>();
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

    if let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse, 170) {
        let e = spawn_ui_sprite(&mut commands, h, 280.0, 80.0, 6.0, 1.0);
        commands.entity(e).insert((
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
            Visibility::Hidden,
        ));
    }
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Prguse2, 360, 361, 362,
        280.0 + 300.0, 83.0, 7.0, 20.0, 20.0,
    ) {
        commands.entity(e).insert((
            QuestLogClose,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // 任务行 8 + 详情 4
    for i in 0..12usize {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            298.0, 120.0 + i as f32 * 20.0,
            12.0, Color::WHITE, 8.0,
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
            &mut commands, &font, "追踪",
            612.0, 120.0 + i as f32 * 20.0,
            11.0, Color::srgb(0.6, 0.9, 1.0), 8.1,
        );
        commands.entity(e).insert((
            QuestLogTrack(i),
            UiButton {
                rect: (612.0, 120.0 + i as f32 * 20.0, 40.0, 18.0),
                clicked: false,
            },
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
    // 放弃按钮
    if let Some(e) = crate::ui::sprite_ui::spawn_ui_button(
        &mut commands, &mut libs, &mut images, &mut cache,
        LibraryName::Title, 206, 207, 208,
        480.0, 365.0, 8.3, 76.0, 25.0,
    ) {
        commands.entity(e).insert((
            QuestLogAbandon,
            DialogRoot(DialogKind::QuestLog),
            QuestLogWidget,
        ));
    }
}

/// 显隐 + 渲染 + 选择 + 放弃
#[allow(clippy::too_many_arguments)]
fn quest_log_ui_system(
    mut mgr: ResMut<DialogManager>,
    mut state: ResMut<QuestLogState>,
    mut tracking: ResMut<crate::game::dialogs::quest_tracking::QuestTrackingState>,
    net: Res<NetConnection>,
    close: Query<&UiButton, With<QuestLogClose>>,
    abandon_btn: Query<&UiButton, With<QuestLogAbandon>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut widgets: Query<&mut Visibility, With<QuestLogWidget>>,
    // #1290：Bevy B0001——两个 &mut Text2d Query 需用 Without 隔离（#1226 任务追踪合并后启动 panic）
    mut lines: Query<(&mut Text2d, &QuestLogLine), Without<QuestLogTrack>>,
    mut track_btns: Query<(&mut Text2d, &UiButton, &QuestLogTrack), Without<QuestLogLine>>,
) {
    let open = mgr.is_open(DialogKind::QuestLog);
    for mut vis in widgets.iter_mut() {
        *vis = if open { Visibility::Visible } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    for btn in &close {
        if btn.clicked {
            mgr.close(DialogKind::QuestLog);
        }
    }
    for (mut text, line) in &mut lines {
        text.0 = match line.0 {
            i if i < 8 => match state.quests.get(i) {
                Some(q) => format!(
                    "{}: {}{}",
                    q.id,
                    q.name,
                    if q.completed { "（完成）" } else { "" }
                ),
                None => String::new(),
            },
            8 => format!("已接任务: {} 个", state.quests.len()),
            9 => {
                let sel = state.selected.and_then(|i| state.quests.get(i));
                match sel {
                    Some(q) => format!("详情: {}", q.name),
                    None => "点击任务行查看详情".to_string(),
                }
            }
            10 => {
                let sel = state.selected.and_then(|i| state.quests.get(i));
                match sel {
                    Some(q) => q.tasks.join(" / "),
                    None => String::new(),
                }
            }
            11 => state.message.clone(),
            _ => String::new(),
        };
    }
    // 行点击选中
    if mouse.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                for i in 0..8usize {
                    let y = 120.0 + i as f32 * 20.0;
                    if cursor.x >= 298.0 && cursor.x <= 600.0 && cursor.y >= y && cursor.y <= y + 18.0 {
                        if i < state.quests.len() {
                            state.selected = Some(i);
                            tracing::info!("📜 选中任务: {}", state.quests[i].name);
                        }
                        break;
                    }
                }
            }
        }
    }
    // 追踪按钮：标签（追踪/取消）+ 点击切换（C# QuestRow Track，上限 5）
    for (mut text, btn, track) in &mut track_btns {
        let quest = state.quests.get(track.0).cloned();
        let tracked = quest.as_ref().map(|q| tracking.is_tracked(q.id)).unwrap_or(false);
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
                tracing::info!("📌 任务追踪 {}: {}", if now_tracked { "开启" } else { "关闭" }, q.name);
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
                    quest_log.quests.last().map(|q| q.name.clone()).unwrap_or_default()
                );
            }
            ServerEvent::QuestInfo { id, name, tasks } => {
                // #260：任务完整信息（C# S.NewQuestInfo）→ 添加/更新任务日志
                let entry = QuestEntry {
                    id: *id,
                    name: name.clone(),
                    tasks: tasks.clone(),
                    taken: true,
                    completed: false,
                    is_new: true,
                };
                if let Some(e) = quest_log.quests.iter_mut().find(|q| q.id == *id) {
                    *e = entry;
                } else {
                    quest_log.quests.push(entry);
                }
                quest_log.message = format!("接受任务: {}", name);
                tracing::info!("📜 任务日志新增: {}", name);
            }
            ServerEvent::QuestShared { quest_id } => {
                // #260：共享任务提示
                quest_log.message = format!("收到共享任务 #{}", quest_id);
                tracing::info!("🔗 共享任务 #{}", quest_id);
            }
            ServerEvent::QuestCompleted { id } => {
                quest_log.quests.retain(|q| q.id != *id);
                quest_log.message = format!("任务 {} 完成！", id);
            }
            _ => {}
        }
    }
}
