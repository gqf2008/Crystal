// ============================================================================
// UI 系统 - 统一管理所有 UI 更新
// ============================================================================
//
// 职责:
// - 处理 UI 事件 (聊天消息、金币变化、物品获得等)
// - 统一渲染所有 UI 组件
// - 解耦 UI 更新逻辑
//
// ============================================================================

use hecs::{World, Entity};
use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

use crate::ecs::ui::{
    MainDialogComp, InventoryDialogComp, CharacterDialogComp,
    SkillBarComp, ChatDialogComp, ChatType, MagicLearningDialogComp
};
use crate::network::game_client::GameEvent;
use crate::ecs::components::{LocalPlayer, PlayerComp, Mana, Health};

/// UI 系统
pub struct UISystem;

impl UISystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 更新所有 UI 组件
    pub fn update(&mut self, _world: &mut World) {
        // UI 更新主要通过事件驱动,在 process_event 中处理
        // 这里保留用于未来需要主动更新的UI逻辑
    }
    
    /// 处理游戏事件并更新UI
    pub fn process_event(world: &mut World, event: &GameEvent) {
        match event {
            // 聊天消息
            GameEvent::ChatReceived { message } => {
                let text = format!("[{}] {}", message.sender, message.text);
                
                // 暂时都显示为系统消息 (TODO: 实现完整的ChatType映射)
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(text.clone(), ChatType::System);
                }
            }
            
            // 系统消息
            GameEvent::SystemMessage { message } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(message.clone(), ChatType::System);
                }
            }
            
            // 金币变化
            GameEvent::GoldChanged { gold } => {
                Self::set_gold(world, *gold);
            }
            
            // 经验值获得
            GameEvent::ExperienceGained { amount } => {
                // 发送聊天消息
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(
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
                    for (_, (_, player_comp)) in world.query::<(&LocalPlayer, &PlayerComp)>().iter() {
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
                    for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                        chat_comp.dialog.add_message(
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
                    
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(
                        format!("获得物品: {} x{}", item_name, item.count),
                        ChatType::System
                    );
                }
            }
            
            // 物品丢失
            GameEvent::ItemLost { unique_id, count } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(
                        format!("失去物品 x{}", count),
                        ChatType::System
                    );
                }
            }
            
            // 技能学习
            GameEvent::MagicLearned { spell, level } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(
                        format!("✨ 学会技能: {:?} (等级 {})", spell, level),
                        ChatType::System
                    );
                }
            }
            
            // 技能升级
            GameEvent::MagicLevelUp { spell, level } => {
                for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                    chat_comp.dialog.add_message(
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
                    for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
                        chat_comp.dialog.add_message(
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
    
    /// 渲染所有 UI 组件
    /// 
    /// 注意: 需要传入 DialogManager 来控制对话框可见性
    pub fn draw(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        current_time: u64,
    ) -> GameResult {
        // 渲染主对话框 (始终显示)
        for (_, dialog_comp) in world.query::<&MainDialogComp>().iter() {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
        
        // 渲染背包对话框 (仅在打开时显示)
        for (_, dialog_comp) in world.query::<&InventoryDialogComp>().iter() {
            if dialog_comp.is_open {
                dialog_comp.dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染角色对话框 (仅在打开时显示)
        for (_, dialog_comp) in world.query::<&CharacterDialogComp>().iter() {
            if dialog_comp.is_open {
                dialog_comp.dialog.draw(ctx, canvas)?;
            }
        }
        
        // 渲染技能栏 (始终显示)
        for (_, skill_bar_comp) in world.query::<&SkillBarComp>().iter() {
            skill_bar_comp.dialog.draw(ctx, canvas, current_time)?;
        }
        
        // 渲染聊天对话框 (始终显示)
        for (_, dialog_comp) in world.query::<&ChatDialogComp>().iter() {
            dialog_comp.dialog.draw(ctx, canvas)?;
        }
        
        // 渲染技能学习对话框 (仅在打开时显示)
        for (_, dialog_comp) in world.query::<&MagicLearningDialogComp>().iter() {
            if dialog_comp.is_open {
                dialog_comp.dialog.draw(ctx, canvas)?;
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 辅助方法 - 通过 Entity 获取 UI 组件
    // ========================================================================
    
    /// 添加聊天消息
    pub fn add_chat_message(world: &mut World, _entity: Entity, text: String, chat_type: ChatType) {
        for (_, chat_comp) in world.query_mut::<&mut ChatDialogComp>() {
            chat_comp.dialog.add_message(text.clone(), chat_type);
        }
    }
    
    /// 设置金币
    pub fn set_gold(world: &mut World, gold: u32) {
        for (_, inv_comp) in world.query_mut::<&mut InventoryDialogComp>() {
            inv_comp.dialog.set_gold(gold);
        }
    }
}
