// QuestRewards - 任务奖励控件
// 对应C#的QuestRewards类

/// Quest rewards - 任务奖励控件
#[derive(Debug)]
pub struct QuestRewards {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 奖励信息
    pub experience_reward: u64,
    pub gold_reward: u32,
    pub item_rewards: Vec<Option<u32>>, // 物品ID列表
    pub selectable_rewards: Vec<Option<u32>>, // 可选奖励
    pub selected_reward_index: Option<usize>,
}

impl Default for QuestRewards {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 200,
            height: 100,
            experience_reward: 0,
            gold_reward: 0,
            item_rewards: Vec::new(),
            selectable_rewards: Vec::new(),
            selected_reward_index: None,
        }
    }
}