// Quest Dialogs - 任务系统对话框
// C# reference: Client/MirScenes/Dialogs/QuestDialogs.cs

use macroquad::prelude::*;
use super::native_ui_utils::*;

/// Quest status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestStatus {
    Available,
    InProgress,
    Completed,
    Finished,
}

/// Quest type/group
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestGroup {
    General,
    Daily,
    Repeatable,
    Story,
}

/// Quest reward item
#[derive(Debug, Clone)]
pub struct QuestRewardItem {
    pub item_id: u32,
    pub name: String,
    pub count: u32,
    pub icon_index: i32,
    pub selectable: bool,
}

/// Quest data
#[derive(Debug, Clone)]
pub struct QuestInfo {
    pub id: u32,
    pub name: String,
    pub group: QuestGroup,
    pub status: QuestStatus,
    pub description: String,
    pub objectives: Vec<String>,
    pub objective_progress: Vec<(u32, u32)>, // (current, required)
    pub npc_name: String,
    pub min_level: u16,
    pub gold_reward: u64,
    pub exp_reward: u64,
    pub fixed_rewards: Vec<QuestRewardItem>,
    pub select_rewards: Vec<QuestRewardItem>,
    pub tracked: bool,
}

/// Actions from quest dialogs
#[derive(Debug, Clone)]
pub enum QuestAction {
    Close,
    AcceptQuest(u32),
    CompleteQuest(u32),
    CancelQuest(u32),
    TrackQuest(u32),
    UntrackQuest(u32),
    SelectQuest(u32),
    ShareQuest(u32),
    PauseQuest(u32),
}

// ============================================================
// Quest List Dialog - 任务列表
// ============================================================

const QUEST_LIST_WIDTH: f32 = 323.0;
const QUEST_LIST_HEIGHT: f32 = 450.0;
const QUEST_ROWS_VISIBLE: usize = 5;

pub struct QuestListDialogHybrid {
    pub visible: bool,
    pub quests: Vec<QuestInfo>,
    pub selected_index: Option<usize>,
    pub current_npc_id: u32,
    start_index: usize,
    position: Vec2,
    bg_texture: BackgroundTexture,
    close_btn: CloseButton,
    drag_helper: DragHelper,
}

impl QuestListDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            quests: Vec::new(),
            selected_index: None,
            current_npc_id: 0,
            start_index: 0,
            position: vec2(400.0, 0.0),
            bg_texture: BackgroundTexture::empty(),
            close_btn: CloseButton::empty(),
            drag_helper: DragHelper::new(),
        }
    }

    pub fn selected_quest(&self) -> Option<&QuestInfo> {
        self.selected_index.and_then(|i| self.quests.get(i))
    }

    pub fn can_accept(&self) -> bool {
        self.selected_quest().map_or(false, |q| q.status == QuestStatus::Available)
    }

    pub fn can_complete(&self) -> bool {
        self.selected_quest().map_or(false, |q| q.status == QuestStatus::Completed)
    }

    pub fn draw(&mut self) -> Option<QuestAction> {
        if !self.visible {
            return None;
        }

        let pos = self.position;
        let rect = Rect::new(pos.x, pos.y, QUEST_LIST_WIDTH, QUEST_LIST_HEIGHT);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        let mut action = None;

        // Background
        draw_rectangle(pos.x, pos.y, QUEST_LIST_WIDTH, QUEST_LIST_HEIGHT, Color::new(0.1, 0.1, 0.15, 0.95));
        draw_rectangle_lines(pos.x, pos.y, QUEST_LIST_WIDTH, QUEST_LIST_HEIGHT, 2.0, GRAY);

        // Title
        draw_text("任务列表", pos.x + 18.0, pos.y + 22.0, 16.0, WHITE);

        // Close button
        let close_rect = Rect::new(pos.x + QUEST_LIST_WIDTH - 30.0, pos.y + 4.0, 24.0, 24.0);
        draw_text("✕", close_rect.x + 6.0, close_rect.y + 17.0, 16.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && close_rect.contains(mouse_pos) {
            self.visible = false;
            return Some(QuestAction::Close);
        }

        // Quest rows
        let visible_quests: Vec<(usize, &QuestInfo)> = self.quests.iter().enumerate()
            .skip(self.start_index)
            .take(QUEST_ROWS_VISIBLE)
            .collect();

        for (row_idx, (quest_idx, quest)) in visible_quests.iter().enumerate() {
            let ry = pos.y + 36.0 + (row_idx as f32) * 19.0;
            let row_rect = Rect::new(pos.x + 10.0, ry - 12.0, 280.0, 17.0);

            // Selection highlight
            if Some(*quest_idx) == self.selected_index {
                draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h,
                    Color::new(0.3, 0.3, 0.5, 0.5));
            }

            // Status icon color
            let name_color = match quest.status {
                QuestStatus::Available => YELLOW,
                QuestStatus::InProgress => WHITE,
                QuestStatus::Completed => GREEN,
                QuestStatus::Finished => GRAY,
            };

            draw_text(&quest.name, pos.x + 30.0, ry + 2.0, 13.0, name_color);

            // Level requirement
            draw_text(&format!("Lv.{}", quest.min_level), pos.x + 240.0, ry + 2.0, 11.0, GRAY);

            if is_mouse_button_pressed(MouseButton::Left) && row_rect.contains(mouse_pos) {
                self.selected_index = Some(*quest_idx);
                action = Some(QuestAction::SelectQuest(quest.id));
            }
        }

        // Scroll buttons
        if self.quests.len() > QUEST_ROWS_VISIBLE {
            let up_rect = Rect::new(pos.x + 292.0, pos.y + 36.0, 18.0, 18.0);
            let down_rect = Rect::new(pos.x + 292.0, pos.y + 36.0 + (QUEST_ROWS_VISIBLE as f32) * 19.0 - 18.0, 18.0, 18.0);
            draw_text("▲", up_rect.x + 2.0, up_rect.y + 13.0, 12.0, WHITE);
            draw_text("▼", down_rect.x + 2.0, down_rect.y + 13.0, 12.0, WHITE);

            if is_mouse_button_pressed(MouseButton::Left) {
                if up_rect.contains(mouse_pos) && self.start_index > 0 {
                    self.start_index = self.start_index.saturating_sub(1);
                }
                if down_rect.contains(mouse_pos) && self.start_index + QUEST_ROWS_VISIBLE < self.quests.len() {
                    self.start_index += 1;
                }
            }
        }

        // Message area (quest description)
        let msg_y = pos.y + 135.0;
        draw_rectangle(pos.x + 10.0, msg_y, 280.0, 160.0, Color::new(0.05, 0.05, 0.1, 0.8));
        if let Some(quest) = self.selected_quest() {
            let desc_lines: Vec<&str> = quest.description.lines().collect();
            for (i, line) in desc_lines.iter().enumerate().take(10) {
                draw_text(line, pos.x + 15.0, msg_y + 16.0 + (i as f32) * 15.0, 12.0, LIGHTGRAY);
            }
        }

        // Rewards area
        let reward_y = pos.y + 307.0;
        draw_rectangle(pos.x + 5.0, reward_y, 313.0, 120.0, Color::new(0.05, 0.05, 0.1, 0.5));
        draw_text("奖励:", pos.x + 10.0, reward_y + 16.0, 13.0, YELLOW);

        if let Some(quest) = self.selected_quest() {
            if quest.gold_reward > 0 {
                draw_text(&format!("金币: {}", quest.gold_reward), pos.x + 15.0, reward_y + 35.0, 12.0, Color::new(1.0, 0.84, 0.0, 1.0));
            }
            if quest.exp_reward > 0 {
                draw_text(&format!("经验: {}", quest.exp_reward), pos.x + 150.0, reward_y + 35.0, 12.0, Color::new(0.5, 0.8, 1.0, 1.0));
            }

            // Fixed rewards
            for (i, item) in quest.fixed_rewards.iter().enumerate().take(5) {
                let ix = pos.x + 15.0 + (i as f32) * 45.0;
                draw_rectangle(ix, reward_y + 50.0, 36.0, 36.0, Color::new(0.15, 0.15, 0.2, 1.0));
                draw_rectangle_lines(ix, reward_y + 50.0, 36.0, 36.0, 1.0, GRAY);
                if item.count > 1 {
                    draw_text(&format!("{}", item.count), ix + 22.0, reward_y + 84.0, 10.0, YELLOW);
                }
            }

            // Selectable rewards
            if !quest.select_rewards.is_empty() {
                draw_text("选择奖励:", pos.x + 10.0, reward_y + 95.0, 12.0, Color::new(0.8, 0.8, 0.2, 1.0));
                for (i, item) in quest.select_rewards.iter().enumerate().take(5) {
                    let ix = pos.x + 15.0 + (i as f32) * 45.0;
                    draw_rectangle(ix, reward_y + 105.0, 36.0, 36.0, Color::new(0.15, 0.2, 0.15, 1.0));
                    draw_rectangle_lines(ix, reward_y + 105.0, 36.0, 36.0, 1.0, GREEN);
                }
            }
        }

        // Action buttons
        let btn_y = pos.y + QUEST_LIST_HEIGHT - 30.0;
        if self.can_accept() {
            let accept_rect = Rect::new(pos.x + 40.0, btn_y, 80.0, 24.0);
            draw_rectangle(accept_rect.x, accept_rect.y, accept_rect.w, accept_rect.h,
                Color::new(0.2, 0.4, 0.2, 1.0));
            draw_text("接受", accept_rect.x + 22.0, accept_rect.y + 17.0, 14.0, WHITE);

            if is_mouse_button_pressed(MouseButton::Left) && accept_rect.contains(mouse_pos) {
                if let Some(quest) = self.selected_quest() {
                    action = Some(QuestAction::AcceptQuest(quest.id));
                }
            }
        }

        if self.can_complete() {
            let finish_rect = Rect::new(pos.x + 40.0, btn_y, 80.0, 24.0);
            draw_rectangle(finish_rect.x, finish_rect.y, finish_rect.w, finish_rect.h,
                Color::new(0.2, 0.2, 0.5, 1.0));
            draw_text("完成", finish_rect.x + 22.0, finish_rect.y + 17.0, 14.0, WHITE);

            if is_mouse_button_pressed(MouseButton::Left) && finish_rect.contains(mouse_pos) {
                if let Some(quest) = self.selected_quest() {
                    action = Some(QuestAction::CompleteQuest(quest.id));
                }
            }
        }

        let leave_rect = Rect::new(pos.x + 205.0, btn_y, 80.0, 24.0);
        draw_rectangle(leave_rect.x, leave_rect.y, leave_rect.w, leave_rect.h,
            Color::new(0.3, 0.2, 0.2, 1.0));
        draw_text("离开", leave_rect.x + 22.0, leave_rect.y + 17.0, 14.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && leave_rect.contains(mouse_pos) {
            self.visible = false;
            action = Some(QuestAction::Close);
        }

        // Dragging
        if let Some(new_pos) = self.drag_helper.update(rect, mouse_pos) {
            self.position = new_pos;
        }

        action
    }
}

// ============================================================
// Quest Detail Dialog - 任务详情
// ============================================================

const QUEST_DETAIL_WIDTH: f32 = 323.0;
const QUEST_DETAIL_HEIGHT: f32 = 450.0;

pub struct QuestDetailDialogHybrid {
    pub visible: bool,
    pub quest: Option<QuestInfo>,
    position: Vec2,
    bg_texture: BackgroundTexture,
    close_btn: CloseButton,
    drag_helper: DragHelper,
}

impl QuestDetailDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            quest: None,
            position: vec2(screen_width() / 2.0 + 20.0, 60.0),
            bg_texture: BackgroundTexture::empty(),
            close_btn: CloseButton::empty(),
            drag_helper: DragHelper::new(),
        }
    }

    pub fn show_quest(&mut self, quest: QuestInfo) {
        self.quest = Some(quest);
        self.visible = true;
    }

    pub fn draw(&mut self) -> Option<QuestAction> {
        if !self.visible {
            return None;
        }

        let pos = self.position;
        let rect = Rect::new(pos.x, pos.y, QUEST_DETAIL_WIDTH, QUEST_DETAIL_HEIGHT);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        let mut action = None;

        draw_rectangle(pos.x, pos.y, QUEST_DETAIL_WIDTH, QUEST_DETAIL_HEIGHT, Color::new(0.1, 0.1, 0.15, 0.95));
        draw_rectangle_lines(pos.x, pos.y, QUEST_DETAIL_WIDTH, QUEST_DETAIL_HEIGHT, 2.0, GRAY);

        // Title
        draw_text("任务详情", pos.x + 18.0, pos.y + 22.0, 16.0, WHITE);

        // Close
        let close_rect = Rect::new(pos.x + QUEST_DETAIL_WIDTH - 30.0, pos.y + 4.0, 24.0, 24.0);
        draw_text("✕", close_rect.x + 6.0, close_rect.y + 17.0, 16.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && close_rect.contains(mouse_pos) {
            self.visible = false;
            return Some(QuestAction::Close);
        }

        if let Some(quest) = &self.quest {
            // Quest name
            draw_text(&quest.name, pos.x + 15.0, pos.y + 50.0, 15.0, YELLOW);
            draw_text(&format!("NPC: {}", quest.npc_name), pos.x + 15.0, pos.y + 68.0, 12.0, GRAY);

            // Description
            let desc_y = pos.y + 85.0;
            draw_rectangle(pos.x + 10.0, desc_y, 280.0, 200.0, Color::new(0.05, 0.05, 0.1, 0.8));

            let lines: Vec<&str> = quest.description.lines().collect();
            for (i, line) in lines.iter().enumerate().take(13) {
                draw_text(line, pos.x + 15.0, desc_y + 16.0 + (i as f32) * 15.0, 12.0, LIGHTGRAY);
            }

            // Objectives
            let obj_y = desc_y + 210.0;
            draw_text("目标:", pos.x + 15.0, obj_y, 13.0, YELLOW);
            for (i, (obj, progress)) in quest.objectives.iter()
                .zip(quest.objective_progress.iter())
                .enumerate()
                .take(5)
            {
                let oy = obj_y + 18.0 + (i as f32) * 16.0;
                let complete = progress.0 >= progress.1;
                let color = if complete { GREEN } else { WHITE };
                let text = format!("• {} ({}/{})", obj, progress.0, progress.1);
                draw_text(&text, pos.x + 20.0, oy, 12.0, color);
            }

            // Rewards
            let reward_y = pos.y + 370.0;
            draw_text("奖励:", pos.x + 10.0, reward_y, 13.0, YELLOW);
            if quest.gold_reward > 0 {
                draw_text(&format!("金: {}", quest.gold_reward), pos.x + 60.0, reward_y, 12.0, Color::new(1.0, 0.84, 0.0, 1.0));
            }
            if quest.exp_reward > 0 {
                draw_text(&format!("经验: {}", quest.exp_reward), pos.x + 160.0, reward_y, 12.0, Color::new(0.5, 0.8, 1.0, 1.0));
            }

            // Action buttons
            let btn_y = pos.y + QUEST_DETAIL_HEIGHT - 30.0;
            let quest_id = quest.id;

            // Share
            let share_rect = Rect::new(pos.x + 40.0, btn_y, 70.0, 24.0);
            draw_rectangle(share_rect.x, share_rect.y, share_rect.w, share_rect.h,
                Color::new(0.2, 0.3, 0.4, 1.0));
            draw_text("分享", share_rect.x + 18.0, share_rect.y + 17.0, 14.0, WHITE);
            if is_mouse_button_pressed(MouseButton::Left) && share_rect.contains(mouse_pos) {
                action = Some(QuestAction::ShareQuest(quest_id));
            }

            // Cancel
            let cancel_rect = Rect::new(pos.x + 200.0, btn_y, 70.0, 24.0);
            draw_rectangle(cancel_rect.x, cancel_rect.y, cancel_rect.w, cancel_rect.h,
                Color::new(0.4, 0.2, 0.2, 1.0));
            draw_text("放弃", cancel_rect.x + 18.0, cancel_rect.y + 17.0, 14.0, WHITE);
            if is_mouse_button_pressed(MouseButton::Left) && cancel_rect.contains(mouse_pos) {
                action = Some(QuestAction::CancelQuest(quest_id));
            }
        } else {
            draw_text("无选中任务", pos.x + 100.0, pos.y + 200.0, 14.0, GRAY);
        }

        if let Some(new_pos) = self.drag_helper.update(rect, mouse_pos) {
            self.position = new_pos;
        }

        action
    }
}

// ============================================================
// Quest Diary Dialog - 任务日记 (grouped quest view)
// ============================================================

const QUEST_DIARY_WIDTH: f32 = 320.0;
const QUEST_DIARY_HEIGHT: f32 = 450.0;

/// Group of quests in diary view
#[derive(Debug, Clone)]
pub struct QuestGroupEntry {
    pub group: QuestGroup,
    pub quests: Vec<QuestInfo>,
    pub expanded: bool,
}

pub struct QuestDiaryDialogHybrid {
    pub visible: bool,
    pub groups: Vec<QuestGroupEntry>,
    scroll_offset: f32,
    position: Vec2,
    bg_texture: BackgroundTexture,
    close_btn: CloseButton,
    drag_helper: DragHelper,
}

impl QuestDiaryDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            groups: Vec::new(),
            scroll_offset: 0.0,
            position: vec2(screen_width() / 2.0 - 320.0, 60.0),
            bg_texture: BackgroundTexture::empty(),
            close_btn: CloseButton::empty(),
            drag_helper: DragHelper::new(),
        }
    }

    pub fn set_quests(&mut self, quests: Vec<QuestInfo>) {
        // Group quests by QuestGroup
        let mut general = Vec::new();
        let mut daily = Vec::new();
        let mut repeatable = Vec::new();
        let mut story = Vec::new();

        for q in quests {
            match q.group {
                QuestGroup::General => general.push(q),
                QuestGroup::Daily => daily.push(q),
                QuestGroup::Repeatable => repeatable.push(q),
                QuestGroup::Story => story.push(q),
            }
        }

        self.groups.clear();
        if !story.is_empty() {
            self.groups.push(QuestGroupEntry { group: QuestGroup::Story, quests: story, expanded: true });
        }
        if !general.is_empty() {
            self.groups.push(QuestGroupEntry { group: QuestGroup::General, quests: general, expanded: true });
        }
        if !daily.is_empty() {
            self.groups.push(QuestGroupEntry { group: QuestGroup::Daily, quests: daily, expanded: true });
        }
        if !repeatable.is_empty() {
            self.groups.push(QuestGroupEntry { group: QuestGroup::Repeatable, quests: repeatable, expanded: false });
        }
    }

    pub fn total_quests(&self) -> usize {
        self.groups.iter().map(|g| g.quests.len()).sum()
    }

    pub fn draw(&mut self) -> Option<QuestAction> {
        if !self.visible {
            return None;
        }

        let pos = self.position;
        let rect = Rect::new(pos.x, pos.y, QUEST_DIARY_WIDTH, QUEST_DIARY_HEIGHT);
        let mouse = mouse_position();
        let mouse_pos = vec2(mouse.0, mouse.1);
        let mut action = None;

        draw_rectangle(pos.x, pos.y, QUEST_DIARY_WIDTH, QUEST_DIARY_HEIGHT, Color::new(0.1, 0.1, 0.15, 0.95));
        draw_rectangle_lines(pos.x, pos.y, QUEST_DIARY_WIDTH, QUEST_DIARY_HEIGHT, 2.0, GRAY);

        draw_text("任务日记", pos.x + 18.0, pos.y + 22.0, 16.0, WHITE);

        // Close
        let close_rect = Rect::new(pos.x + QUEST_DIARY_WIDTH - 30.0, pos.y + 4.0, 24.0, 24.0);
        draw_text("✕", close_rect.x + 6.0, close_rect.y + 17.0, 16.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && close_rect.contains(mouse_pos) {
            self.visible = false;
            return Some(QuestAction::Close);
        }

        // Draw groups
        let mut y = pos.y + 40.0 - self.scroll_offset;

        for group in &mut self.groups {
            if y > pos.y + QUEST_DIARY_HEIGHT { break; }

            let group_label = match group.group {
                QuestGroup::General => "一般任务",
                QuestGroup::Daily => "每日任务",
                QuestGroup::Repeatable => "重复任务",
                QuestGroup::Story => "主线任务",
            };

            // Group header
            let header_rect = Rect::new(pos.x + 10.0, y, 280.0, 20.0);
            if y + 20.0 > pos.y + 30.0 {
                draw_rectangle(header_rect.x, header_rect.y, header_rect.w, header_rect.h,
                    Color::new(0.2, 0.2, 0.3, 0.8));
                let arrow = if group.expanded { "▼" } else { "►" };
                draw_text(&format!("{} {} ({})", arrow, group_label, group.quests.len()),
                    pos.x + 15.0, y + 15.0, 13.0, YELLOW);

                if is_mouse_button_pressed(MouseButton::Left) && header_rect.contains(mouse_pos) {
                    group.expanded = !group.expanded;
                }
            }
            y += 22.0;

            // Quest entries (if expanded)
            if group.expanded {
                for quest in &group.quests {
                    if y > pos.y + QUEST_DIARY_HEIGHT { break; }
                    if y + 15.0 > pos.y + 30.0 {
                        let quest_rect = Rect::new(pos.x + 25.0, y, 250.0, 15.0);

                        let color = match quest.status {
                            QuestStatus::Available => YELLOW,
                            QuestStatus::InProgress => WHITE,
                            QuestStatus::Completed => GREEN,
                            QuestStatus::Finished => GRAY,
                        };

                        let track_marker = if quest.tracked { "◉ " } else { "  " };
                        draw_text(&format!("{}{}", track_marker, quest.name),
                            pos.x + 25.0, y + 12.0, 12.0, color);

                        if is_mouse_button_pressed(MouseButton::Left) && quest_rect.contains(mouse_pos) {
                            action = Some(QuestAction::SelectQuest(quest.id));
                        }
                        if is_mouse_button_pressed(MouseButton::Right) && quest_rect.contains(mouse_pos) {
                            if quest.tracked {
                                action = Some(QuestAction::UntrackQuest(quest.id));
                            } else {
                                action = Some(QuestAction::TrackQuest(quest.id));
                            }
                        }
                    }
                    y += 16.0;
                }
            }
        }

        // Mouse wheel scrolling
        let (_wheel_x, wheel_y) = mouse_wheel();
        if rect.contains(mouse_pos) && wheel_y.abs() > 0.0 {
            self.scroll_offset = (self.scroll_offset - wheel_y * 20.0).max(0.0);
        }

        if let Some(new_pos) = self.drag_helper.update(rect, mouse_pos) {
            self.position = new_pos;
        }

        action
    }
}

// ============================================================
// Quest Tracking Dialog - 任务追踪（屏幕侧边显示）
// ============================================================

const MAX_TRACKED_QUESTS: usize = 5;

pub struct QuestTrackingDialogHybrid {
    pub visible: bool,
    pub tracked_quests: Vec<QuestInfo>,
    position: Vec2,
}

impl QuestTrackingDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: true, // default visible
            tracked_quests: Vec::new(),
            position: vec2(0.0, 100.0),
        }
    }

    pub fn add_quest(&mut self, quest: QuestInfo) {
        if self.tracked_quests.len() >= MAX_TRACKED_QUESTS {
            return;
        }
        if !self.tracked_quests.iter().any(|q| q.id == quest.id) {
            self.tracked_quests.push(quest);
        }
    }

    pub fn remove_quest(&mut self, quest_id: u32) {
        self.tracked_quests.retain(|q| q.id != quest_id);
    }

    pub fn draw(&mut self) -> Option<QuestAction> {
        if !self.visible || self.tracked_quests.is_empty() {
            return None;
        }

        let pos = self.position;
        let mut y = pos.y;

        for quest in &self.tracked_quests {
            // Quest name (green)
            draw_text(&quest.name, pos.x + 5.0, y + 15.0, 13.0, GREEN);
            y += 20.0;

            // Objectives
            for (obj, progress) in quest.objectives.iter().zip(quest.objective_progress.iter()) {
                let complete = progress.0 >= progress.1;
                let color = if complete { Color::new(0.5, 0.5, 0.5, 1.0) } else { WHITE };
                let text = format!("  {} ({}/{})", obj, progress.0, progress.1);
                draw_text(&text, pos.x + 25.0, y + 13.0, 11.0, color);
                y += 15.0;
            }

            y += 5.0; // spacing between quests
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_quest(id: u32, name: &str, status: QuestStatus, group: QuestGroup) -> QuestInfo {
        QuestInfo {
            id,
            name: name.to_string(),
            group,
            status,
            description: "Test description".to_string(),
            objectives: vec!["Kill monsters".to_string()],
            objective_progress: vec![(3, 10)],
            npc_name: "TestNPC".to_string(),
            min_level: 1,
            gold_reward: 100,
            exp_reward: 200,
            fixed_rewards: Vec::new(),
            select_rewards: Vec::new(),
            tracked: false,
        }
    }

    #[test]
    fn test_quest_list_new() {
        let dialog = QuestListDialogHybrid::new();
        assert!(!dialog.visible);
        assert!(dialog.quests.is_empty());
        assert!(dialog.selected_index.is_none());
    }

    #[test]
    fn test_quest_list_selection() {
        let mut dialog = QuestListDialogHybrid::new();
        dialog.quests = vec![
            make_quest(1, "Quest A", QuestStatus::Available, QuestGroup::General),
            make_quest(2, "Quest B", QuestStatus::Completed, QuestGroup::Daily),
        ];
        dialog.selected_index = Some(0);
        assert!(dialog.can_accept());
        assert!(!dialog.can_complete());

        dialog.selected_index = Some(1);
        assert!(!dialog.can_accept());
        assert!(dialog.can_complete());
    }

    #[test]
    fn test_quest_detail_show() {
        let mut dialog = QuestDetailDialogHybrid::new();
        assert!(!dialog.visible);
        let quest = make_quest(1, "Test Quest", QuestStatus::InProgress, QuestGroup::Story);
        dialog.show_quest(quest);
        assert!(dialog.visible);
        assert_eq!(dialog.quest.as_ref().unwrap().name, "Test Quest");
    }

    #[test]
    fn test_quest_diary_grouping() {
        let mut dialog = QuestDiaryDialogHybrid::new();
        let quests = vec![
            make_quest(1, "Story 1", QuestStatus::InProgress, QuestGroup::Story),
            make_quest(2, "Daily 1", QuestStatus::Available, QuestGroup::Daily),
            make_quest(3, "Story 2", QuestStatus::InProgress, QuestGroup::Story),
            make_quest(4, "General 1", QuestStatus::Completed, QuestGroup::General),
        ];
        dialog.set_quests(quests);
        assert_eq!(dialog.groups.len(), 3); // Story, General, Daily
        assert_eq!(dialog.total_quests(), 4);
        // Story group should be first
        assert_eq!(dialog.groups[0].group, QuestGroup::Story);
        assert_eq!(dialog.groups[0].quests.len(), 2);
    }

    #[test]
    fn test_quest_tracking() {
        let mut dialog = QuestTrackingDialogHybrid::new();
        assert!(dialog.tracked_quests.is_empty());

        let quest = make_quest(1, "Track Me", QuestStatus::InProgress, QuestGroup::General);
        dialog.add_quest(quest);
        assert_eq!(dialog.tracked_quests.len(), 1);

        // No duplicates
        let quest2 = make_quest(1, "Track Me", QuestStatus::InProgress, QuestGroup::General);
        dialog.add_quest(quest2);
        assert_eq!(dialog.tracked_quests.len(), 1);

        // Max 5
        for i in 2..=6 {
            dialog.add_quest(make_quest(i, &format!("Quest {}", i), QuestStatus::InProgress, QuestGroup::General));
        }
        assert_eq!(dialog.tracked_quests.len(), MAX_TRACKED_QUESTS);

        // Remove
        dialog.remove_quest(1);
        assert_eq!(dialog.tracked_quests.len(), MAX_TRACKED_QUESTS - 1);
    }
}
