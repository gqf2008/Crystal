// ============================================================================
// 任务相关组件
// ============================================================================

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestState {
    Available, // 可接取
    Active,    // 进行中
    Completed, // 已完成
    Failed,    // 已失败
}

/// 任务奖励物品
#[derive(Debug, Clone)]
pub struct QuestRewardItem {
    /// 物品唯一标识（对应 SharedRust 中的 ItemIndex）
    pub item_index: u32,
    /// 物品名称（用于显示）
    pub item_name: String,
    /// 数量
    pub count: u32,
}

/// 任务目标类型
#[derive(Debug, Clone, PartialEq)]
pub enum QuestObjective {
    /// 击杀指定数量的怪物
    KillMonsters {
        monster_name: String,
        current: u32,
        required: u32,
    },
    /// 收集指定数量的物品
    CollectItems {
        item_name: String,
        current: u32,
        required: u32,
    },
    /// 与NPC对话
    TalkToNPC { npc_name: String, completed: bool },
    /// 到达指定地点
    ReachLocation {
        location_name: String,
        x: i32,
        y: i32,
        completed: bool,
    },
}

/// 任务数据
#[derive(Debug, Clone)]
pub struct Quest {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub state: QuestState,
    pub objectives: Vec<QuestObjective>,
    pub reward_exp: i64,
    pub reward_gold: u32,
    pub reward_items: Vec<QuestRewardItem>,
}

impl Quest {
    pub fn new(id: u32, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            state: QuestState::Available,
            objectives: Vec::new(),
            reward_exp: 0,
            reward_gold: 0,
            reward_items: Vec::new(),
        }
    }

    /// 检查任务是否完成
    pub fn is_completed(&self) -> bool {
        self.objectives.iter().all(|obj| obj.is_completed())
    }

    /// 获取任务进度百分比
    pub fn get_progress_percentage(&self) -> f32 {
        if self.objectives.is_empty() {
            return 0.0;
        }

        let completed_count = self
            .objectives
            .iter()
            .filter(|obj| obj.is_completed())
            .count();

        (completed_count as f32 / self.objectives.len() as f32) * 100.0
    }
}

impl QuestObjective {
    pub fn is_completed(&self) -> bool {
        match self {
            QuestObjective::KillMonsters {
                current, required, ..
            } => current >= required,
            QuestObjective::CollectItems {
                current, required, ..
            } => current >= required,
            QuestObjective::TalkToNPC { completed, .. } => *completed,
            QuestObjective::ReachLocation { completed, .. } => *completed,
        }
    }

    pub fn get_progress_text(&self) -> String {
        match self {
            QuestObjective::KillMonsters {
                monster_name,
                current,
                required,
            } => {
                format!("击杀 {}: {}/{}", monster_name, current, required)
            }
            QuestObjective::CollectItems {
                item_name,
                current,
                required,
            } => {
                format!("收集 {}: {}/{}", item_name, current, required)
            }
            QuestObjective::TalkToNPC {
                npc_name,
                completed,
            } => {
                format!(
                    "与 {} 对话: {}",
                    npc_name,
                    if *completed { "✓" } else { "○" }
                )
            }
            QuestObjective::ReachLocation {
                location_name,
                completed,
                ..
            } => {
                format!(
                    "到达 {}: {}",
                    location_name,
                    if *completed { "✓" } else { "○" }
                )
            }
        }
    }
}
