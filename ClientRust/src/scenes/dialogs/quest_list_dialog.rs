// QuestListDialog - Quest list and management
// Rust implementation of Client/MirScenes/Dialogs/QuestDialogs.cs (QuestListDialog)

use crate::scenes::dialogs::Dialog;

/// Quest status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    Available,  // Can be accepted
    Active,     // Currently doing
    Completed,  // Ready to finish
    Finished,   // Already finished
}

/// Quest type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestType {
    General,    // Normal quest
    Daily,      // Daily quest
    Story,      // Story quest
    Repeatable, // Repeatable quest
}

/// Quest reward item
#[derive(Debug, Clone)]
pub struct QuestRewardItem {
    pub item_index: u32,
    pub item_name: String,
    pub count: u32,
    pub selectable: bool, // Can player choose this reward?
}

/// Quest information
#[derive(Debug, Clone)]
pub struct QuestInfo {
    pub index: u32,
    pub name: String,
    pub quest_type: QuestType,
    pub level: u16,
    pub min_level: u16,
    pub max_level: u16,
    pub npc_index: u32,
    pub npc_name: String,
    pub finish_npc_index: u32,
    pub finish_npc_name: String,
    pub description: String,
    pub completion_description: String,
    pub rewards_gold: u32,
    pub rewards_exp: u64,
    pub rewards_items: Vec<QuestRewardItem>,
    pub rewards_select_item: Vec<QuestRewardItem>, // Choose one
}

impl QuestInfo {
    pub fn new(index: u32, name: String) -> Self {
        Self {
            index,
            name,
            quest_type: QuestType::General,
            level: 1,
            min_level: 1,
            max_level: 100,
            npc_index: 0,
            npc_name: String::new(),
            finish_npc_index: 0,
            finish_npc_name: String::new(),
            description: String::new(),
            completion_description: String::new(),
            rewards_gold: 0,
            rewards_exp: 0,
            rewards_items: Vec::new(),
            rewards_select_item: Vec::new(),
        }
    }

    pub fn has_selectable_rewards(&self) -> bool {
        !self.rewards_select_item.is_empty()
    }

    pub fn total_reward_items(&self) -> usize {
        self.rewards_items.len() + self.rewards_select_item.len()
    }
}

/// Quest progress tracking
#[derive(Debug, Clone)]
pub struct QuestProgress {
    pub quest_info: QuestInfo,
    pub status: QuestStatus,
    pub taken: bool,
    pub completed: bool,
    pub kill_counts: Vec<(u32, u32)>, // (monster_id, current_count / required_count)
    pub collect_counts: Vec<(u32, u32)>, // (item_id, current_count / required_count)
}

impl QuestProgress {
    pub fn new(quest_info: QuestInfo) -> Self {
        Self {
            quest_info,
            status: QuestStatus::Available,
            taken: false,
            completed: false,
            kill_counts: Vec::new(),
            collect_counts: Vec::new(),
        }
    }

    pub fn can_accept(&self) -> bool {
        !self.taken && self.status == QuestStatus::Available
    }

    pub fn can_finish(&self) -> bool {
        self.taken && self.completed
    }

    pub fn update_kill_count(&mut self, monster_id: u32, count: u32) {
        if let Some(entry) = self.kill_counts.iter_mut().find(|(id, _)| *id == monster_id) {
            entry.1 = count;
        } else {
            self.kill_counts.push((monster_id, count));
        }
        self.check_completion();
    }

    pub fn update_collect_count(&mut self, item_id: u32, count: u32) {
        if let Some(entry) = self.collect_counts.iter_mut().find(|(id, _)| *id == item_id) {
            entry.1 = count;
        } else {
            self.collect_counts.push((item_id, count));
        }
        self.check_completion();
    }

    fn check_completion(&mut self) {
        // Simplified completion check
        let kills_complete = self.kill_counts.iter().all(|(_, count)| *count >= 1);
        let collect_complete = self.collect_counts.iter().all(|(_, count)| *count >= 1);
        self.completed = kills_complete && collect_complete;
        if self.completed {
            self.status = QuestStatus::Completed;
        }
    }

    pub fn get_progress_text(&self) -> String {
        if self.completed {
            "Completed".to_string()
        } else if self.taken {
            "In Progress".to_string()
        } else {
            "Available".to_string()
        }
    }
}

/// Quest reward selection
pub struct QuestRewards {
    pub selected_item_index: i32, // -1 if not selected
    pub reward_items: Vec<QuestRewardItem>,
}

impl QuestRewards {
    pub fn new() -> Self {
        Self {
            selected_item_index: -1,
            reward_items: Vec::new(),
        }
    }

    pub fn select_reward(&mut self, index: usize) -> bool {
        if index < self.reward_items.len() {
            self.selected_item_index = index as i32;
            true
        } else {
            false
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selected_item_index >= 0
    }

    pub fn clear_selection(&mut self) {
        self.selected_item_index = -1;
    }
}

/// Quest List Dialog - Display and manage quests
pub struct QuestListDialog {
    visible: bool,
    pub quests: Vec<QuestProgress>,
    pub selected_index: Option<usize>,
    pub start_index: usize,
    pub rows_per_page: usize, // 5 in C#
    pub current_npc_id: u32,
    pub rewards: Option<QuestRewards>,
}

impl QuestListDialog {
    const MAX_ROWS: usize = 5;

    pub fn new() -> Self {
        Self {
            visible: false,
            quests: Vec::new(),
            selected_index: None,
            start_index: 0,
            rows_per_page: Self::MAX_ROWS,
            current_npc_id: 0,
            rewards: None,
        }
    }

    pub fn add_quest(&mut self, quest: QuestProgress) {
        self.quests.push(quest);
    }

    pub fn remove_quest(&mut self, quest_index: u32) -> bool {
        if let Some(pos) = self.quests.iter().position(|q| q.quest_info.index == quest_index) {
            self.quests.remove(pos);
            if self.selected_index == Some(pos) {
                self.selected_index = None;
            }
            true
        } else {
            false
        }
    }

    pub fn find_quest(&self, quest_index: u32) -> Option<&QuestProgress> {
        self.quests.iter().find(|q| q.quest_info.index == quest_index)
    }

    pub fn find_quest_mut(&mut self, quest_index: u32) -> Option<&mut QuestProgress> {
        self.quests.iter_mut().find(|q| q.quest_info.index == quest_index)
    }

    pub fn select_quest(&mut self, index: usize) -> bool {
        if index < self.quests.len() {
            self.selected_index = Some(index);
            self.load_quest_rewards(index);
            true
        } else {
            false
        }
    }

    pub fn get_selected_quest(&self) -> Option<&QuestProgress> {
        self.selected_index.and_then(|idx| self.quests.get(idx))
    }

    pub fn get_selected_quest_mut(&mut self) -> Option<&mut QuestProgress> {
        self.selected_index.and_then(|idx| self.quests.get_mut(idx))
    }

    fn load_quest_rewards(&mut self, index: usize) {
        if let Some(quest) = self.quests.get(index) {
            let mut rewards = QuestRewards::new();
            rewards.reward_items = quest.quest_info.rewards_select_item.clone();
            self.rewards = Some(rewards);
        }
    }

    pub fn scroll_up(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx > 0 {
                self.selected_index = Some(idx - 1);
            } else if self.start_index > 0 {
                self.start_index -= 1;
            }
        }
    }

    pub fn scroll_down(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.rows_per_page - 1 && self.start_index + idx + 1 < self.quests.len() {
                self.selected_index = Some(idx + 1);
            } else if self.start_index + self.rows_per_page < self.quests.len() {
                self.start_index += 1;
            }
        }
    }

    pub fn get_visible_quests(&self) -> Vec<&QuestProgress> {
        self.quests
            .iter()
            .skip(self.start_index)
            .take(self.rows_per_page)
            .collect()
    }

    pub fn can_accept_selected(&self) -> bool {
        if let Some(quest) = self.get_selected_quest() {
            quest.can_accept()
        } else {
            false
        }
    }

    pub fn can_finish_selected(&self) -> bool {
        if let Some(quest) = self.get_selected_quest() {
            quest.can_finish()
        } else {
            false
        }
    }

    pub fn accept_quest(&mut self) -> Option<u32> {
        if let Some(quest) = self.get_selected_quest_mut() {
            if quest.can_accept() {
                quest.taken = true;
                quest.status = QuestStatus::Active;
                return Some(quest.quest_info.index);
            }
        }
        None
    }

    pub fn finish_quest(&mut self, selected_reward_index: i32) -> Option<u32> {
        if let Some(quest) = self.get_selected_quest_mut() {
            if quest.can_finish() {
                // Validate reward selection if needed
                if quest.quest_info.has_selectable_rewards() && selected_reward_index < 0 {
                    return None; // Must select a reward
                }
                quest.status = QuestStatus::Finished;
                return Some(quest.quest_info.index);
            }
        }
        None
    }

    pub fn set_current_npc(&mut self, npc_id: u32) {
        self.current_npc_id = npc_id;
    }

    pub fn get_quests_by_npc(&self, npc_id: u32) -> Vec<&QuestProgress> {
        self.quests
            .iter()
            .filter(|q| q.quest_info.npc_index == npc_id || q.quest_info.finish_npc_index == npc_id)
            .collect()
    }

    pub fn get_quests_by_status(&self, status: QuestStatus) -> Vec<&QuestProgress> {
        self.quests
            .iter()
            .filter(|q| q.status == status)
            .collect()
    }

    pub fn total_quest_count(&self) -> usize {
        self.quests.len()
    }

    pub fn active_quest_count(&self) -> usize {
        self.quests.iter().filter(|q| q.taken).count()
    }

    pub fn clear_quests(&mut self) {
        self.quests.clear();
        self.selected_index = None;
        self.start_index = 0;
        self.rewards = None;
    }
}

impl Dialog for QuestListDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
    
    fn name(&self) -> &str { "QuestListDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 450 && y >= 0 && y < 550 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (450, 550) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_quest(index: u32, name: &str) -> QuestProgress {
        let info = QuestInfo::new(index, name.to_string());
        QuestProgress::new(info)
    }

    #[test]
    fn test_quest_info_creation() {
        let info = QuestInfo::new(1, "Test Quest".to_string());
        assert_eq!(info.index, 1);
        assert_eq!(info.name, "Test Quest");
        assert!(!info.has_selectable_rewards());
    }

    #[test]
    fn test_quest_progress_creation() {
        let quest = create_test_quest(1, "Kill Monsters");
        assert_eq!(quest.status, QuestStatus::Available);
        assert!(!quest.taken);
        assert!(!quest.completed);
    }

    #[test]
    fn test_quest_can_accept() {
        let quest = create_test_quest(1, "Test");
        assert!(quest.can_accept());
        
        let mut taken_quest = quest.clone();
        taken_quest.taken = true;
        assert!(!taken_quest.can_accept());
    }

    #[test]
    fn test_quest_list_dialog_creation() {
        let dialog = QuestListDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.total_quest_count(), 0);
    }

    #[test]
    fn test_add_remove_quest() {
        let mut dialog = QuestListDialog::new();
        let quest = create_test_quest(1, "Quest 1");
        
        dialog.add_quest(quest);
        assert_eq!(dialog.total_quest_count(), 1);
        
        assert!(dialog.remove_quest(1));
        assert_eq!(dialog.total_quest_count(), 0);
    }

    #[test]
    fn test_select_quest() {
        let mut dialog = QuestListDialog::new();
        dialog.add_quest(create_test_quest(1, "Quest 1"));
        dialog.add_quest(create_test_quest(2, "Quest 2"));
        
        assert!(dialog.select_quest(0));
        assert!(dialog.get_selected_quest().is_some());
        assert_eq!(dialog.get_selected_quest().unwrap().quest_info.index, 1);
    }

    #[test]
    fn test_scroll_quests() {
        let mut dialog = QuestListDialog::new();
        for i in 0..10 {
            dialog.add_quest(create_test_quest(i, &format!("Quest {}", i)));
        }
        
        dialog.select_quest(0);
        assert_eq!(dialog.start_index, 0);
        
        dialog.scroll_down();
        dialog.scroll_down();
        // Should have scrolled or changed selection
    }

    #[test]
    fn test_accept_quest() {
        let mut dialog = QuestListDialog::new();
        dialog.add_quest(create_test_quest(1, "Test Quest"));
        dialog.select_quest(0);
        
        let result = dialog.accept_quest();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 1);
        
        let quest = dialog.get_selected_quest().unwrap();
        assert!(quest.taken);
        assert_eq!(quest.status, QuestStatus::Active);
    }

    #[test]
    fn test_finish_quest() {
        let mut dialog = QuestListDialog::new();
        let mut quest = create_test_quest(1, "Test Quest");
        quest.taken = true;
        quest.completed = true;
        dialog.add_quest(quest);
        dialog.select_quest(0);
        
        let result = dialog.finish_quest(-1);
        assert!(result.is_some());
        
        let quest = dialog.get_selected_quest().unwrap();
        assert_eq!(quest.status, QuestStatus::Finished);
    }

    #[test]
    fn test_quest_rewards() {
        let mut rewards = QuestRewards::new();
        assert!(!rewards.has_selection());
        
        rewards.reward_items.push(QuestRewardItem {
            item_index: 1,
            item_name: "Sword".to_string(),
            count: 1,
            selectable: true,
        });
        
        assert!(rewards.select_reward(0));
        assert!(rewards.has_selection());
        assert_eq!(rewards.selected_item_index, 0);
    }

    #[test]
    fn test_get_quests_by_npc() {
        let mut dialog = QuestListDialog::new();
        let mut quest1 = create_test_quest(1, "Quest 1");
        quest1.quest_info.npc_index = 10;
        let mut quest2 = create_test_quest(2, "Quest 2");
        quest2.quest_info.npc_index = 20;
        
        dialog.add_quest(quest1);
        dialog.add_quest(quest2);
        
        let npc_quests = dialog.get_quests_by_npc(10);
        assert_eq!(npc_quests.len(), 1);
    }

    #[test]
    fn test_get_quests_by_status() {
        let mut dialog = QuestListDialog::new();
        let mut quest1 = create_test_quest(1, "Quest 1");
        quest1.status = QuestStatus::Active;
        quest1.taken = true;
        let quest2 = create_test_quest(2, "Quest 2");
        
        dialog.add_quest(quest1);
        dialog.add_quest(quest2);
        
        let active_quests = dialog.get_quests_by_status(QuestStatus::Active);
        assert_eq!(active_quests.len(), 1);
    }

    #[test]
    fn test_quest_progress_text() {
        let mut quest = create_test_quest(1, "Test");
        assert_eq!(quest.get_progress_text(), "Available");
        
        quest.taken = true;
        assert_eq!(quest.get_progress_text(), "In Progress");
        
        quest.completed = true;
        assert_eq!(quest.get_progress_text(), "Completed");
    }

    #[test]
    fn test_visible_quests() {
        let mut dialog = QuestListDialog::new();
        for i in 0..10 {
            dialog.add_quest(create_test_quest(i, &format!("Quest {}", i)));
        }
        
        let visible = dialog.get_visible_quests();
        assert_eq!(visible.len(), 5); // MAX_ROWS
    }

    #[test]
    fn test_active_quest_count() {
        let mut dialog = QuestListDialog::new();
        let mut quest1 = create_test_quest(1, "Quest 1");
        quest1.taken = true;
        let quest2 = create_test_quest(2, "Quest 2");
        
        dialog.add_quest(quest1);
        dialog.add_quest(quest2);
        
        assert_eq!(dialog.active_quest_count(), 1);
    }
}
