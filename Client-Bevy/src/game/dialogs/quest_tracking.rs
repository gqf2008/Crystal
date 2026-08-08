// ============================================================================
// 任务追踪小窗（C# QuestTrackingDialog，Client/MirScenes/Dialogs/QuestDialogs.cs:796）
//   - 纯客户端：无网络报文/服务端参与
//   - 任务日志每行可“追踪”，小窗显示已追踪任务名（LimeGreen）+ 任务进度行（白色缩进）
//   - 可拖动（C# Movable），最多 5 个，无追踪任务时自动隐藏
//   - TrackedQuestsIds 按角色存盘（C# Settings.TrackedQuests + SaveTrackedQuests(name)）
// ============================================================================

use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;

use crate::actor::{LocalPlayer, PlayerName};
use crate::game::dialogs::quest_log::QuestLogState;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_text, UiEntity, UiFont};

/// 最多同时追踪 5 个任务（C# QuestTrackingDialog：Count >= 5 return）
pub const MAX_TRACKED: usize = 5;

/// 追踪状态（C# TrackedQuestsIds）
#[derive(Resource, Default)]
pub struct QuestTrackingState {
    pub tracked: Vec<i32>,
    /// 当前角色名（持久化文件名 key；C# SaveTrackedQuests(name)）
    pub char_name: String,
    /// 窗口屏幕坐标（左上角；C# 默认 (0,100)，Movable 可拖动）
    pub pos: (f32, f32),
    /// 拖动中：按下点与窗口左上角的偏移
    pub drag_offset: Option<(f32, f32)>,
}

impl QuestTrackingState {
    pub fn add(&mut self, id: i32) -> bool {
        if self.tracked.contains(&id) || self.tracked.len() >= MAX_TRACKED {
            return false;
        }
        self.tracked.push(id);
        true
    }

    pub fn remove(&mut self, id: i32) -> bool {
        let before = self.tracked.len();
        self.tracked.retain(|x| *x != id);
        before != self.tracked.len()
    }

    /// 返回操作后是否处于追踪状态
    pub fn toggle(&mut self, id: i32) -> bool {
        if self.tracked.contains(&id) {
            self.remove(id);
            false
        } else {
            self.add(id);
            true
        }
    }

    pub fn is_tracked(&self, id: i32) -> bool {
        self.tracked.contains(&id)
    }

    fn file_path(char_name: &str) -> Option<PathBuf> {
        if char_name.is_empty() {
            return None;
        }
        Some(PathBuf::from(format!("quest_tracking_{}.txt", char_name)))
    }

    /// 按角色加载（C# Settings.LoadTrackedQuests(name)）
    pub fn load(&mut self, char_name: &str) {
        self.char_name = char_name.to_string();
        self.tracked.clear();
        let Some(path) = Self::file_path(char_name) else { return };
        let Ok(content) = fs::read_to_string(&path) else { return };
        for line in content.lines() {
            if let Ok(id) = line.trim().parse::<i32>() {
                if self.tracked.len() < MAX_TRACKED && !self.tracked.contains(&id) {
                    self.tracked.push(id);
                }
            }
        }
    }

    /// 按角色保存（C# Settings.SaveTrackedQuests(name)）
    pub fn save(&self) {
        let Some(path) = Self::file_path(&self.char_name) else { return };
        let content = self
            .tracked
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = fs::write(path, content);
    }
}

#[derive(Component)]
pub struct QuestTrackingWidget;

#[derive(Component)]
pub struct QuestTrackingText(usize);

/// 小窗面板尺寸（屏幕像素）
const PANEL_W: f32 = 170.0;
const PANEL_H: f32 = 200.0;
/// 预生成文本行数（5 任务 × 任务行，够用）
const TEXT_LINES: usize = 30;

pub struct QuestTrackingPlugin;

impl Plugin for QuestTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestTrackingState>();
        app.add_systems(OnEnter(AppState::Game), spawn_quest_tracking);
        app.add_systems(OnExit(AppState::Game), cleanup_quest_tracking);
        app.add_systems(
            Update,
            quest_tracking_ui_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_quest_tracking(mut commands: Commands, roots: Query<Entity, With<QuestTrackingWidget>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_quest_tracking(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(&mut fonts);
    }
    let font = ui_font.0.clone();
    let white = images.add(crate::map_renderer::make_image(vec![255, 255, 255, 255], 1, 1));
    // 半透明面板（C# 无背景，这里用深色半透明便于阅读）
    commands.spawn((
        UiEntity,
        QuestTrackingWidget,
        Sprite {
            image: white.clone(),
            color: Color::srgba(0.1, 0.1, 0.14, 0.9),
            custom_size: Some(Vec2::new(PANEL_W, PANEL_H)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(0.0, 0.0, 20.0),
        Visibility::Hidden,
    ));
    for i in 0..TEXT_LINES {
        let e = spawn_ui_text(
            &mut commands, &font, "",
            0.0, 0.0,
            11.0, Color::WHITE, 20.2,
        );
        commands.entity(e).insert((QuestTrackingText(i), UiEntity));
    }
}

/// 渲染追踪任务 + 拖动
fn quest_tracking_ui_system(
    mut state: ResMut<QuestTrackingState>,
    quest_log: Res<QuestLogState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    local_player: Query<&PlayerName, (With<LocalPlayer>, Without<QuestTrackingWidget>)>,
    mut widgets: Query<(&mut Transform, &mut Visibility), (With<QuestTrackingWidget>, Without<QuestTrackingText>)>,
    mut texts: Query<(&mut Transform, &mut Text2d, &mut TextColor, &QuestTrackingText), Without<QuestTrackingWidget>>,
) {
    // 角色名变化 → 按角色加载持久化追踪列表（C# SaveTrackedQuests(name)）
    if let Ok(name) = local_player.single() {
        if state.char_name != name.0 {
            state.load(&name.0);
            tracing::info!("📌 任务追踪已加载（角色 {}，{} 个）", name.0, state.tracked.len());
        }
    }

    // 已追踪任务 → 从任务日志取当前进度（C# DisplayQuests：CurrentQuests 匹配 TrackedQuestsIds）
    let mut quests_to_track: Vec<(String, Vec<String>)> = Vec::new();
    for id in &state.tracked {
        if let Some(q) = quest_log.quests.iter().find(|q| &q.id == id) {
            quests_to_track.push((q.name.clone(), q.tasks.clone()));
        }
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };

    // 拖动（C# Movable：按住面板移动）
    let in_panel = cursor.x >= state.pos.0
        && cursor.x <= state.pos.0 + PANEL_W
        && cursor.y >= state.pos.1
        && cursor.y <= state.pos.1 + PANEL_H;
    if mouse.just_pressed(MouseButton::Left) && in_panel {
        state.drag_offset = Some((cursor.x - state.pos.0, cursor.y - state.pos.1));
    }
    if let Some(off) = state.drag_offset {
        if mouse.pressed(MouseButton::Left) {
            state.pos = (cursor.x - off.0, cursor.y - off.1);
        } else {
            state.drag_offset = None;
        }
    }

    // 无追踪任务或都不在当前任务列表 → 隐藏（C# questsToTrack.Count < 1 → Hide）
    let visible = !quests_to_track.is_empty();
    for (mut tf, mut vis) in &mut widgets {
        *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
        if visible {
            tf.translation.x = state.pos.0;
            tf.translation.y = -state.pos.1;
        }
    }
    if !visible {
        return;
    }

    // 渲染：任务名（LimeGreen）+ 任务行（白色缩进）
    let mut lines: Vec<(String, Color, f32)> = Vec::new();
    for (name, tasks) in &quests_to_track {
        lines.push((name.clone(), Color::srgb(0.196, 0.804, 0.196), 20.0));
        for t in tasks {
            lines.push((t.clone(), Color::WHITE, 40.0));
        }
    }
    let mut y = 0.0f32;
    for (mut tf, mut text, mut color, idx) in &mut texts {
        if let Some((content, c, indent)) = lines.get(idx.0) {
            text.0 = content.clone();
            color.0 = *c;
            tf.translation.x = state.pos.0 + indent;
            tf.translation.y = -state.pos.1 - 20.0 - y;
            y += 15.0;
        } else {
            text.0 = String::new();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_toggle() {
        let mut s = QuestTrackingState::default();
        assert!(s.add(1));
        assert!(!s.add(1)); // 重复
        assert!(s.is_tracked(1));
        assert!(s.remove(1));
        assert!(!s.is_tracked(1));
        // toggle
        assert!(s.toggle(2)); // 开启
        assert!(!s.toggle(2)); // 关闭
    }

    #[test]
    fn test_max_tracked() {
        let mut s = QuestTrackingState::default();
        for i in 1..=5 {
            assert!(s.add(i), "id {} 应可追踪", i);
        }
        assert!(!s.add(6), "第 6 个应被拒绝（C# 上限 5）");
        assert_eq!(s.tracked.len(), MAX_TRACKED);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let name = format!("__qt_test_{}", std::process::id());
        let path = QuestTrackingState::file_path(&name).unwrap();
        let _ = std::fs::remove_file(&path);

        let mut s = QuestTrackingState::default();
        s.load(&name); // 文件不存在 → 空
        assert!(s.tracked.is_empty());
        assert!(s.add(7));
        assert!(s.add(8));
        s.save();

        let mut s2 = QuestTrackingState::default();
        s2.load(&name);
        assert_eq!(s2.tracked, vec![7, 8]);
        assert_eq!(s2.char_name, name);

        let _ = std::fs::remove_file(&path);
    }
}
