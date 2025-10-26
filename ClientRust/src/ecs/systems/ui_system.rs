// ============================================================================
// UI 系统 - 统一管理所有 UI 更新
// ============================================================================
//
// 职责:
// - 处理 UI 事件 (聊天消息、金币变化、物品获得等)
// - 管理 UI 状态和数据更新
// - 解耦 UI 更新逻辑
//
// 注意: UI渲染由 RenderSystem::draw_ui 负责，符合ECS设计原则
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
    
    // ========================================================================
    // 对话框管理方法 (供 InputSystem 调用)
    // ========================================================================
    
    /// 切换对话框显示/隐藏
    pub fn toggle_dialog(world: &mut World, dialog_type: crate::ecs::ui::DialogType) {
        use crate::ecs::ui::DialogType;
        
        match dialog_type {
            DialogType::Inventory => {
                for (_, inv) in world.query_mut::<&mut InventoryDialog>() {
                    inv.toggle();
                    break;
                }
            }
            DialogType::Character => {
                for (_, char_dlg) in world.query_mut::<&mut CharacterDialog>() {
                    char_dlg.toggle();
                    break;
                }
            }
            DialogType::Skills => {
                for (_, skills) in world.query_mut::<&mut SkillsDialog>() {
                    let is_open = skills.is_open();
                    skills.set_open(!is_open);
                    tracing::info!("⚔️ 技能: {}", if !is_open { "打开" } else { "关闭" });
                    break;
                }
            }
            DialogType::Quest => {
                // 先检查状态
                let should_open = {
                    let mut should_open = false;
                    for (_, quest) in world.query_mut::<&mut QuestDialog>() {
                        should_open = !quest.is_open;
                        if quest.is_open {
                            quest.close();
                        } else {
                            quest.open();
                        }
                        break;
                    }
                    should_open
                };
                
                // 打开时更新任务列表
                if should_open {
                    let active_quests = crate::ecs::systems::QuestSystem::get_active_quests(world);
                    for (_, quest) in world.query_mut::<&mut QuestDialog>() {
                        quest.update_active_quests(active_quests);
                        break;
                    }
                }
                
                tracing::info!("📜 任务: {}", if should_open { "打开" } else { "关闭" });
            }
            DialogType::MagicLearning => {
                // 先切换状态
                let was_closed = {
                    let mut was_closed = false;
                    for (_, magic) in world.query_mut::<&mut MagicLearningDialog>() {
                        was_closed = !magic.is_open();
                        magic.toggle();
                        break;
                    }
                    was_closed
                };
                
                // 打开时更新可学习技能列表
                if was_closed {
                    crate::ecs::systems::MagicLearningSystem::update_available_magics(world);
                }
                
                tracing::info!("📖 技能学习: {}", if was_closed { "打开" } else { "关闭" });
            }
            DialogType::Trade => {
                for (_, trade) in world.query_mut::<&mut crate::ecs::ui::TradeDialog>() {
                    if trade.is_open {
                        trade.close();
                        tracing::info!("🤝 交易: 关闭");
                    } else {
                        // 测试：创建虚拟交易数据
                        use crate::ecs::systems::TradeData;
                        let test_trade = TradeData::new(999, "测试玩家".to_string());
                        trade.open(test_trade);
                        tracing::info!("🤝 交易: 打开 (测试)");
                    }
                    break;
                }
            }
            _ => {
                tracing::warn!("⚠️ 对话框类型 {:?} 暂未实现", dialog_type);
            }
        }
    }
    
    /// 关闭最上层对话框
    pub fn close_top_dialog(world: &mut World) {
        // 简单实现：关闭所有打开的对话框
        // TODO: 实现 z-order 排序
        
        let mut closed = false;
        
        // 优先级：交易 > 任务 > 技能学习 > 技能 > 角色 > 背包
        for (_, trade) in world.query_mut::<&mut crate::ecs::ui::TradeDialog>() {
            if trade.is_open {
                trade.close();
                closed = true;
                break;
            }
        }
        
        if !closed {
            for (_, quest) in world.query_mut::<&mut QuestDialog>() {
                if quest.is_open {
                    quest.close();
                    closed = true;
                    break;
                }
            }
        }
        
        if !closed {
            for (_, magic) in world.query_mut::<&mut MagicLearningDialog>() {
                if magic.is_open() {
                    magic.toggle();
                    closed = true;
                    break;
                }
            }
        }
        
        if !closed {
            for (_, skills) in world.query_mut::<&mut SkillsDialog>() {
                if skills.is_open() {
                    skills.set_open(false);
                    closed = true;
                    break;
                }
            }
        }
        
        if !closed {
            for (_, char_dlg) in world.query_mut::<&mut CharacterDialog>() {
                if char_dlg.is_open() {
                    char_dlg.hide();
                    closed = true;
                    break;
                }
            }
        }
        
        if !closed {
            for (_, inv) in world.query_mut::<&mut InventoryDialog>() {
                if inv.is_open() {
                    inv.hide();
                    closed = true;
                    break;
                }
            }
        }
        
        if closed {
            tracing::info!("✖️ 关闭对话框");
        }
    }
    
    /// 处理 UI 点击（返回 true 表示事件被消费）
    pub fn handle_click(
        world: &mut World,
        button: ggez::winit::event::MouseButton,
        ui_x: f32,
        ui_y: f32,
    ) -> bool {
        use ggez::winit::event::MouseButton;
        use crate::ecs::ui::{CharacterAction, InventoryAction, MainDialogButton};
        
        if button != MouseButton::Left {
            return false;
        }
        
        // 按优先级检查对话框点击（从上到下）
        
        // 1. 角色对话框
        for (_, char_dlg) in world.query_mut::<&mut CharacterDialog>() {
            if !char_dlg.is_open() {
                continue;
            }
            
            if let Some(action) = char_dlg.on_mouse_down(ui_x, ui_y) {
                match action {
                    CharacterAction::Close => {
                        tracing::info!("👤 角色对话框关闭");
                    }
                    CharacterAction::SwitchTab(tab) => {
                        tracing::info!("👤 切换到标签页: {:?}", tab);
                    }
                    CharacterAction::EquipmentClick(slot) => {
                        tracing::info!("👤 点击装备槽: {:?}", slot);
                    }
                }
                return true;
            }
        }
        
        // 2. 背包对话框
        for (_, inv) in world.query_mut::<&mut InventoryDialog>() {
            if !inv.is_open() {
                continue;
            }
            
            if let Some(action) = inv.on_mouse_down(ui_x, ui_y) {
                match action {
                    InventoryAction::Close => {
                        tracing::info!("🎒 背包关闭");
                    }
                    InventoryAction::SelectSlot(slot) => {
                        tracing::info!("🎒 选中背包格子: {}", slot);
                    }
                    _ => {}
                }
                return true;
            }
        }
        
        // 3. 主对话框
        for (_, main) in world.query_mut::<&mut MainDialog>() {
            if let Some(button) = main.on_mouse_down(ui_x, ui_y) {
                tracing::info!("🖱️ 点击主对话框按钮: {:?}", button);
                
                match button {
                    MainDialogButton::Inventory => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::Inventory);
                    }
                    MainDialogButton::Character => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::Character);
                    }
                    MainDialogButton::Skills => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::Skills);
                    }
                    MainDialogButton::Quest => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::Quest);
                    }
                    MainDialogButton::Options => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::Options);
                    }
                    MainDialogButton::Menu => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::Menu);
                    }
                    MainDialogButton::GameShop => {
                        Self::toggle_dialog(world, crate::ecs::ui::DialogType::GameShop);
                    }
                }
                return true;
            }
        }
        
        false // UI 未消费事件
    }
    
    /// 更新 UI hover 状态
    pub fn update_hover(world: &mut World, ui_x: f32, ui_y: f32) {
        // 更新主对话框
        for (_, main) in world.query_mut::<&mut MainDialog>() {
            main.update_hover(ui_x, ui_y);
        }
        
        // 更新角色对话框
        for (_, char_dlg) in world.query_mut::<&mut CharacterDialog>() {
            if char_dlg.is_open() {
                char_dlg.update_hover(ui_x, ui_y);
            }
        }
        
        // 更新背包对话框
        for (_, inv) in world.query_mut::<&mut InventoryDialog>() {
            if inv.is_open() {
                inv.update_hover(ui_x, ui_y);
            }
        }
    }
}


