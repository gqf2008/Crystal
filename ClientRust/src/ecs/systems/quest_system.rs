// ============================================================================
// 任务系统 - 处理任务接受、进度跟踪、完成
// ============================================================================

use hecs::World;
use crate::ecs::components::LocalPlayer;
use crate::network::NetworkCommand;
use tokio::sync::mpsc;

/// 任务类型 (匹配C# QuestType)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestType {
    General = 0,      // 普通任务
    Daily = 1,        // 日常任务
    Repeatable = 2,   // 可重复任务
    Story = 3,        // 剧情任务
}

/// 任务图标 (匹配C# QuestIcon)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestIcon {
    None = 0,
    QuestionWhite = 1,
    ExclamationYellow = 2,
    QuestionYellow = 3,
    ExclamationBlue = 5,
    QuestionBlue = 6,
    ExclamationGreen = 52,
    QuestionGreen = 53,
}

/// 任务状态 (保留用于内部逻辑)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestState {
    Available,      // 可接取
    InProgress,     // 进行中
    Completed,      // 已完成(可交付)
    Finished,       // 已交付
    Failed,         // 失败
}

/// 任务目标类型
#[derive(Debug, Clone, PartialEq)]
pub enum QuestObjective {
    KillMonster { 
        monster_name: String, 
        required: u32, 
        current: u32 
    },
    CollectItem { 
        item_id: u32, 
        required: u32, 
        current: u32 
    },
    ReachLocation { 
        map_name: String, 
        x: i32, 
        y: i32, 
        reached: bool 
    },
    TalkToNPC { 
        npc_name: String, 
        completed: bool 
    },
}

impl QuestObjective {
    /// 是否完成
    pub fn is_complete(&self) -> bool {
        match self {
            QuestObjective::KillMonster { required, current, .. } => current >= required,
            QuestObjective::CollectItem { required, current, .. } => current >= required,
            QuestObjective::ReachLocation { reached, .. } => *reached,
            QuestObjective::TalkToNPC { completed, .. } => *completed,
        }
    }
    
    /// 获取进度文本
    pub fn get_progress_text(&self) -> String {
        match self {
            QuestObjective::KillMonster { monster_name, required, current } => {
                format!("击杀 {}: {}/{}", monster_name, current, required)
            }
            QuestObjective::CollectItem { item_id, required, current } => {
                format!("收集物品 {}: {}/{}", item_id, current, required)
            }
            QuestObjective::ReachLocation { map_name, x, y, reached } => {
                if *reached {
                    format!("到达 {} ({}, {}) ✓", map_name, x, y)
                } else {
                    format!("到达 {} ({}, {})", map_name, x, y)
                }
            }
            QuestObjective::TalkToNPC { npc_name, completed } => {
                if *completed {
                    format!("与 {} 对话 ✓", npc_name)
                } else {
                    format!("与 {} 对话", npc_name)
                }
            }
        }
    }
}

/// 任务奖励
#[derive(Debug, Clone)]
pub struct QuestReward {
    pub gold: u32,
    pub experience: u64,
    pub items: Vec<u32>,  // 物品ID列表
}

/// 任务数据 (匹配C# ClientQuestProgress + ClientQuestInfo)
#[derive(Debug, Clone)]
pub struct Quest {
    pub id: i32,                          // 任务ID (匹配C# int)
    pub quest_type: QuestType,            // 任务类型
    pub npc_index: u32,                   // NPC索引 (匹配C# NPCIndex)
    pub name: String,                     // 任务名称
    pub description: String,              // 任务描述
    pub objectives: Vec<QuestObjective>,  // 任务目标
    pub task_list: Vec<String>,           // 任务步骤描述 (匹配C# TaskList)
    pub reward: QuestReward,              // 任务奖励
    
    // 任务状态字段 (匹配C# ClientQuestProgress)
    pub taken: bool,                      // 是否已接取
    pub completed: bool,                  // 是否已完成
    pub new: bool,                        // 是否为新任务
}

impl Quest {
    /// 获取任务图标 (匹配C# GetQuestIcon逻辑)
    pub fn get_icon(&self) -> QuestIcon {
        if !self.taken {
            // 未接取 - 显示感叹号
            match self.quest_type {
                QuestType::General | QuestType::Repeatable => QuestIcon::ExclamationYellow,
                QuestType::Daily => QuestIcon::ExclamationBlue,
                QuestType::Story => QuestIcon::ExclamationGreen,
            }
        } else if self.completed {
            // 已完成 - 显示问号
            match self.quest_type {
                QuestType::General | QuestType::Repeatable => QuestIcon::QuestionYellow,
                QuestType::Daily => QuestIcon::QuestionBlue,
                QuestType::Story => QuestIcon::QuestionGreen,
            }
        } else {
            // 进行中 - 白色问号
            QuestIcon::QuestionWhite
        }
    }
    
    /// 所有目标是否完成
    pub fn all_objectives_complete(&self) -> bool {
        self.objectives.iter().all(|obj| obj.is_complete())
    }
    
    /// 更新目标进度
    pub fn update_objective(&mut self, index: usize, progress: u32) -> bool {
        if index >= self.objectives.len() {
            return false;
        }
        
        match &mut self.objectives[index] {
            QuestObjective::KillMonster { current, .. } => {
                *current = progress;
                true
            }
            QuestObjective::CollectItem { current, .. } => {
                *current = progress;
                true
            }
            _ => false
        }
    }
}

/// 任务组件 (挂载到玩家实体上)
#[derive(Debug, Clone)]
pub struct QuestLog {
    pub active_quests: Vec<Quest>,
    pub completed_quests: Vec<u32>,  // 已完成的任务ID列表
}

impl QuestLog {
    pub fn new() -> Self {
        Self {
            active_quests: Vec::new(),
            completed_quests: Vec::new(),
        }
    }
    
    /// 添加任务
    pub fn add_quest(&mut self, quest: Quest) {
        self.active_quests.push(quest);
    }
    
    /// 通过ID查找任务
    pub fn find_quest_mut(&mut self, quest_id: i32) -> Option<&mut Quest> {
        self.active_quests.iter_mut().find(|q| q.id == quest_id)
    }
    
    /// 完成任务
    pub fn complete_quest(&mut self, quest_id: i32) -> Option<Quest> {
        if let Some(index) = self.active_quests.iter().position(|q| q.id == quest_id) {
            let quest = self.active_quests.remove(index);
            self.completed_quests.push(quest_id as u32);
            Some(quest)
        } else {
            None
        }
    }
}

/// 任务系统
pub struct QuestSystem;

impl QuestSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 接受任务 (匹配C# AcceptButton.Click逻辑)
    pub fn accept_quest(
        world: &mut World,
        quest: Quest,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        // ✅ 检查任务是否已接取 (匹配C# if (Reward == null || SelectedQuest.Taken) return;)
        if quest.taken {
            println!("❌ 任务已接取");
            return Err("任务已接取".to_string());
        }
        
        println!("📜 接受任务: {}", quest.name);
        
        // 添加到任务日志，并设置taken状态
        for (_, (_, quest_log)) in world.query_mut::<(&LocalPlayer, &mut QuestLog)>() {
            let mut accepted_quest = quest.clone();
            accepted_quest.taken = true;  // ✅ 设置taken状态
            quest_log.add_quest(accepted_quest);
            break;
        }
        
        // 发送到服务器 (匹配C# Network.Enqueue(new C.AcceptQuest {...}))
        network_tx.send(NetworkCommand::AcceptQuest { 
            npc_index: quest.npc_index,  // ✅ 使用npc_index而非npc_id
            quest_index: quest.id,
        }).map_err(|e| format!("发送AcceptQuest失败: {}", e))?;
        
        Ok(())
    }
    
    /// 更新任务进度 (击杀怪物)
    pub fn update_kill_progress(
        world: &mut World,
        monster_name: String,
    ) {
        for (_, (_, quest_log)) in world.query_mut::<(&LocalPlayer, &mut QuestLog)>() {
            for quest in &mut quest_log.active_quests {
                let mut updated = false;
                for objective in &mut quest.objectives {
                    if let QuestObjective::KillMonster { 
                        monster_name: target_name, 
                        current, 
                        required 
                    } = objective {
                        if *target_name == monster_name && *current < *required {
                            *current += 1;
                            updated = true;
                            println!("📊 任务进度更新: {} ({}/{})", 
                                quest.name, current, required);
                        }
                    }
                }
                
                // 检查是否全部完成 (在可变借用结束后)
                if updated && quest.all_objectives_complete() {
                    quest.completed = true;  //  设置completed字段
                    println!("✅ 任务完成: {}", quest.name);
                }
            }
            break;
        }
    }
    
    /// 更新任务进度 (收集物品)
    pub fn update_collect_progress(
        world: &mut World,
        item_id: u32,
        count: u32,
    ) {
        for (_, (_, quest_log)) in world.query_mut::<(&LocalPlayer, &mut QuestLog)>() {
            for quest in &mut quest_log.active_quests {
                let mut updated = false;
                for objective in &mut quest.objectives {
                    if let QuestObjective::CollectItem { 
                        item_id: target_item, 
                        current, 
                        .. 
                    } = objective {
                        if *target_item == item_id {
                            *current = count;
                            updated = true;
                            println!("📊 任务进度更新: {} (物品 {})", quest.name, item_id);
                        }
                    }
                }
                
                if updated && quest.all_objectives_complete() {
                    quest.completed = true;  //  设置completed字段
                    println!("✅ 任务完成: {}", quest.name);
                }
            }
            break;
        }
    }
    
    /// 更新任务进度 (到达地点)
    pub fn check_location_progress(
        world: &mut World,
        map_name: String,
        x: i32,
        y: i32,
    ) {
        for (_, (_, quest_log)) in world.query_mut::<(&LocalPlayer, &mut QuestLog)>() {
            for quest in &mut quest_log.active_quests {
                let mut updated = false;
                for objective in &mut quest.objectives {
                    if let QuestObjective::ReachLocation { 
                        map_name: target_map, 
                        x: target_x, 
                        y: target_y, 
                        reached 
                    } = objective {
                        if *target_map == map_name && *target_x == x && *target_y == y {
                            *reached = true;
                            updated = true;
                            println!("📍 到达任务地点: {}", quest.name);
                        }
                    }
                }
                
                if updated && quest.all_objectives_complete() {
                    quest.completed = true;  //  设置completed字段
                    println!("✅ 任务完成: {}", quest.name);
                }
            }
            break;
        }
    }
    
    /// 完成与NPC对话的任务目标
    pub fn complete_npc_dialogue(
        world: &mut World,
        npc_name: String,
    ) {
        for (_, (_, quest_log)) in world.query_mut::<(&LocalPlayer, &mut QuestLog)>() {
            for quest in &mut quest_log.active_quests {
                let mut updated = false;
                for objective in &mut quest.objectives {
                    if let QuestObjective::TalkToNPC { 
                        npc_name: target_npc, 
                        completed 
                    } = objective {
                        if *target_npc == npc_name {
                            *completed = true;
                            updated = true;
                            println!("💬 完成对话任务: {}", quest.name);
                        }
                    }
                }
                
                if updated && quest.all_objectives_complete() {
                    quest.completed = true;  //  设置completed字段
                    println!("✅ 任务完成: {}", quest.name);
                }
            }
            break;
        }
    }
    
    /// 提交任务 (领取奖励) - 匹配C# FinishButton.Click逻辑
    pub fn submit_quest(
        world: &mut World,
        quest_id: i32,
        selected_item_index: i32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> Result<(), String> {
        for (_, (_, quest_log)) in world.query_mut::<(&LocalPlayer, &mut QuestLog)>() {
            // 查找任务
            let quest = quest_log.find_quest_mut(quest_id)
                .ok_or("任务不存在")?;
            
            // ✅ 检查完成状态 (匹配C# if (Reward == null || !SelectedQuest.Completed) return;)
            if !quest.completed {
                return Err("任务未完成".to_string());
            }
            
            // ✅ 检查是否需要选择奖励物品 (匹配C# if (Reward.SelectedItemIndex < 0 && ...)
            if selected_item_index < 0 && !quest.reward.items.is_empty() {
                return Err("必须选择奖励物品".to_string());
            }
            
            println!("🎁 提交任务: {}", quest.name);
            println!("  金币: {}", quest.reward.gold);
            println!("  经验: {}", quest.reward.experience);
            if selected_item_index >= 0 {
                println!("  选择奖励物品索引: {}", selected_item_index);
            }
            
            // 发送到服务器 (匹配C# Network.Enqueue(new C.FinishQuest {...}))
            network_tx.send(NetworkCommand::FinishQuest { 
                quest_index: quest_id,
                selected_item_index,
            }).map_err(|e| format!("发送FinishQuest失败: {}", e))?;
            
            // 从日志中移除
            if let Some(completed_quest) = quest_log.complete_quest(quest_id) {
                println!("✅ 任务已完成并移除: {}", completed_quest.name);
            }
            
            return Ok(());
        }
        
        Err("未找到玩家".to_string())
    }
    
    /// 获取可接取的任务列表 (UI用)
    pub fn get_available_quests(_world: &World) -> Vec<Quest> {
        // TODO: 从NPC或其他来源获取
        Vec::new()
    }
    
    /// 获取进行中的任务列表 (UI用)
    pub fn get_active_quests(world: &World) -> Vec<Quest> {
        for (_, (_, quest_log)) in world.query::<(&LocalPlayer, &QuestLog)>().iter() {
            return quest_log.active_quests.clone();
        }
        Vec::new()
    }
}
