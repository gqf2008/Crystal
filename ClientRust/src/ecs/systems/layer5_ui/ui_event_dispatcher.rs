// ============================================================================
// UI事件分发系统 - 处理游戏事件并更新UI
// ============================================================================
//
// 职责:
// - 接收游戏事件(GameEvent)
// - 分发事件到对应的UI组件
// - 更新UI状态(聊天消息、金币、经验值等)
// - 同步玩家数据到UI
//
// 注意: 从 UISystem 中拆分出来，专注于事件处理
//
// ============================================================================

use hecs::{World, Entity};
use crate::ecs::ui::{
    MainDialog, InventoryDialog, CharacterDialog,
    SkillBarDialog, ChatDialog, ChatType, MagicLearningDialog,
    QuestDialog, SkillsDialog, OptionsDialog
};
use crate::network::game_client::GameEvent;
use crate::ecs::components::{LocalPlayer, PlayerData, Mana, Health};

/// UI事件分发系统
pub struct UIEventDispatcher;

impl UIEventDispatcher {
    pub fn new() -> Self {
        Self
    }
    
    /// 处理游戏事件并更新UI
    pub fn process_event(world: &mut World, event: &GameEvent) {
        match event {
            // 聊天消息
            GameEvent::ChatReceived { message } => {
                let text = format!("[{}] {}", message.sender, message.text);
                
                // 暂时都显示为系统消息 (TODO: 实现完整的ChatType映射)
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(text.clone(), ChatType::System);
                }
            }
            
            // 系统消息
            GameEvent::SystemMessage { message } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(message.clone(), ChatType::System);
                }
            }
            
            // 金币变化
            GameEvent::GoldChanged { gold } => {
                Self::set_gold(world, *gold);
            }
            
            // 经验值获得
            GameEvent::ExperienceGained { amount } => {
                // 发送聊天消息
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(
                        format!("📈 获得经验: +{}", amount),
                        ChatType::System
                    );
                }
            }
            
            // 等级提升
            GameEvent::LevelChanged { object_id, level } => {
                // 检查是否是本地玩家
                let is_local_player = {
                    let mut found = false;
                    for (_, (_, player_comp)) in world.query::<(&LocalPlayer, &PlayerData)>().iter() {
                        if player_comp.id == *object_id {
                            found = true;
                            break;
                        }
                    }
                    found
                };
                
                if is_local_player {
                    println!("🎉 恭喜升级! 等级: {}", level);
                    
                    // 显示升级消息
                    for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                        chat_comp.add_message(
                            format!("🎉 恭喜升级! 等级提升至 {}", level),
                            ChatType::System
                        );
                    }
                }
            }
            
            // 物品获得
            GameEvent::ItemGained { item, grid_type } => {
                let item_name = item.info.as_ref()
                    .map(|info| info.name.as_str())
                    .unwrap_or("未知物品");
                    
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(
                        format!("获得物品: {} x{}", item_name, item.count),
                        ChatType::System
                    );
                }
            }
            
            // 物品丢失
            GameEvent::ItemLost { unique_id, count } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(
                        format!("失去物品 x{}", count),
                        ChatType::System
                    );
                }
            }
            
            // 技能学习
            GameEvent::MagicLearned { spell, level } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(
                        format!("✨ 学会技能: {:?} (等级 {})", spell, level),
                        ChatType::System
                    );
                }
            }
            
            // 技能升级
            GameEvent::MagicLevelUp { spell, level } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                    chat_comp.add_message(
                        format!("⬆️ 技能升级: {:?} → 等级 {}", spell, level),
                        ChatType::System
                    );
                }
            }
            
            // 用户信息更新 (HP/MP等)
            GameEvent::UserInformation { user_info } => {
                // 更新血量
                for (_, (_, health)) in world.query_mut::<(&LocalPlayer, &mut Health)>() {
                    health.current = user_info.hp.max(0);
                }
                
                // 更新魔法值
                for (_, (_, mana)) in world.query_mut::<(&LocalPlayer, &mut Mana)>() {
                    mana.current = user_info.mp.max(0);
                }
            }
            
            // 玩家受伤
            GameEvent::PlayerStruck { attacker_id, damage, location } => {
                if *damage > 0 {
                    for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
                        chat_comp.add_message(
                            format!("受到伤害: -{}", damage),
                            ChatType::System
                        );
                    }
                }
            }
            
            // 其他事件暂不处理
            _ => {}
        }
    }
    
    // ========================================================================
    // 辅助方法 - 通过 Entity 获取 UI 组件
    // ========================================================================
    
    /// 添加聊天消息
    pub fn add_chat_message(world: &mut World, _entity: Entity, text: String, chat_type: ChatType) {
        for (_, chat_comp) in world.query_mut::<&mut ChatDialog>() {
            chat_comp.add_message(text.clone(), chat_type);
        }
    }
    
    /// 设置金币
    pub fn set_gold(world: &mut World, gold: u32) {
        for (_, inv_comp) in world.query_mut::<&mut InventoryDialog>() {
            inv_comp.set_gold(gold);
        }
    }
}
