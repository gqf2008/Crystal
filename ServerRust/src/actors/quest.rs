// Quest system - 任务数据结构
// 纯数据结构，由 WorldActor 调用

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    Accepted,
    InProgress,
    Completed,
    Failed,
}

/// 任务进度项
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuestProgress {
    /// 进度描述 ID
    pub progress_id: i32,
    /// 当前进度
    pub current: i32,
    /// 目标进度
    pub target: i32,
}

/// 任务实例
#[derive(Debug, Clone)]
pub struct QuestInstance {
    /// 任务索引/ID
    pub quest_index: i32,
    /// 任务名称
    pub title: String,
    /// 当前状态
    pub status: QuestStatus,
    /// 进度项
    pub progress: Vec<QuestProgress>,
    /// 奖励经验
    pub exp_reward: i64,
    /// 奖励金币
    pub gold_reward: u64,
    /// 任务接受时的 Unix 时间戳（秒）
    pub start_time: u64,
    /// 时间限制（秒，0=无限制）
    pub time_limit_seconds: i32,
}

impl QuestInstance {
    /// 更新进度
    pub fn update_progress(&mut self, progress_id: i32, amount: i32) {
        if let Some(p) = self.progress.iter_mut().find(|p| p.progress_id == progress_id) {
            p.current = (p.current + amount).min(p.target);
        }
    }

    /// 检查是否所有进度都已完成
    pub fn is_progress_complete(&self) -> bool {
        self.progress.iter().all(|p| p.current >= p.target)
    }
}

/// 玩家任务列表
#[derive(Debug, Clone, Default)]
pub struct QuestLog {
    pub quests: Vec<QuestInstance>,
    /// 已完成的任务索引（用于追踪）
    pub completed_indices: Vec<i32>,
}

impl QuestLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 接受任务
    pub fn accept_quest(&mut self, quest: QuestInstance) -> bool {
        // 检查是否已接受相同任务
        if self.quests.iter().any(|q| q.quest_index == quest.quest_index) {
            return false;
        }
        self.quests.push(quest);
        true
    }

    /// 完成任务
    pub fn complete_quest(&mut self, quest_index: i32) -> Option<QuestInstance> {
        if let Some(idx) = self.quests.iter().position(|q| q.quest_index == quest_index) {
            let mut quest = self.quests.remove(idx);
            quest.status = QuestStatus::Completed;
            self.completed_indices.push(quest_index);
            Some(quest)
        } else {
            None
        }
    }

    /// 放弃任务
    pub fn abandon_quest(&mut self, quest_index: i32) -> bool {
        if let Some(idx) = self.quests.iter().position(|q| q.quest_index == quest_index) {
            self.quests.remove(idx);
            true
        } else {
            false
        }
    }

    /// 获取任务
    pub fn get_quest(&self, quest_index: i32) -> Option<&QuestInstance> {
        self.quests.iter().find(|q| q.quest_index == quest_index)
    }

    /// 获取任务（可变）
    pub fn get_quest_mut(&mut self, quest_index: i32) -> Option<&mut QuestInstance> {
        self.quests.iter_mut().find(|q| q.quest_index == quest_index)
    }

    /// 更新任务进度
    pub fn update_quest_progress(&mut self, quest_index: i32, progress_id: i32, amount: i32) {
        if let Some(quest) = self.get_quest_mut(quest_index) {
            quest.update_progress(progress_id, amount);
        }
    }

    /// 处理怪物击杀：为所有需要该怪物的活跃任务增加进度
    /// 返回更新的任务列表: (quest_index, progress_id, is_complete)
    pub fn process_kill(&mut self, monster_index: i32) -> Vec<(i32, i32, bool)> {
        let mut updated = Vec::new();
        for quest in &mut self.quests {
            let mut changed = false;
            for p in &mut quest.progress {
                if p.progress_id == monster_index && p.current < p.target {
                    p.current += 1;
                    changed = true;
                    break; // C# behavior: only first matching task per quest gets credit
                }
            }
            if changed {
                let complete = quest.is_progress_complete();
                updated.push((quest.quest_index, monster_index, complete));
            }
        }
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_quest(index: i32) -> QuestInstance {
        QuestInstance {
            quest_index: index,
            title: format!("Quest {}", index),
            status: QuestStatus::InProgress,
            progress: vec![
                QuestProgress { progress_id: 1, current: 0, target: 10 },
            ],
            exp_reward: 1000,
            gold_reward: 500,
            start_time: 0,
            time_limit_seconds: 0,
        }
    }

    #[test]
    fn test_accept_quest() {
        let mut log = QuestLog::new();
        assert!(log.accept_quest(make_quest(1)));
        assert_eq!(log.quests.len(), 1);

        // Duplicate should fail
        assert!(!log.accept_quest(make_quest(1)));
        assert_eq!(log.quests.len(), 1);
    }

    #[test]
    fn test_complete_quest() {
        let mut log = QuestLog::new();
        log.accept_quest(make_quest(1));

        let quest = log.complete_quest(1).unwrap();
        assert_eq!(quest.status, QuestStatus::Completed);
        assert_eq!(log.quests.len(), 0);
        assert!(log.completed_indices.contains(&1));

        // Already completed
        assert!(log.complete_quest(1).is_none());
    }

    #[test]
    fn test_abandon_quest() {
        let mut log = QuestLog::new();
        log.accept_quest(make_quest(1));
        assert!(log.abandon_quest(1));
        assert_eq!(log.quests.len(), 0);
        assert!(!log.abandon_quest(1));
    }

    #[test]
    fn test_update_progress() {
        let mut log = QuestLog::new();
        log.accept_quest(make_quest(1));

        log.update_quest_progress(1, 1, 5);
        let quest = log.get_quest(1).unwrap();
        assert_eq!(quest.progress[0].current, 5);

        log.update_quest_progress(1, 1, 10); // should cap at target
        let quest = log.get_quest(1).unwrap();
        assert_eq!(quest.progress[0].current, 10);
    }

    #[test]
    fn test_is_progress_complete() {
        let mut quest = make_quest(1);
        assert!(!quest.is_progress_complete());

        quest.update_progress(1, 10);
        assert!(quest.is_progress_complete());
    }
}
