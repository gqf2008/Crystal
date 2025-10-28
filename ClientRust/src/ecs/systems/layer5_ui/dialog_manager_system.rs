// ============================================================================
// 对话框管理系统 - 管理所有对话框的打开/关闭/切换
// ============================================================================
//
// 职责:
// - 对话框显示状态管理
// - 对话框切换逻辑
// - 对话框层级控制(z-order)
// - UI 点击事件处理
// - UI hover 状态更新
//
// 注意: 从 UISystem 中拆分出来，专注于对话框管理
//
// ============================================================================

use hecs::{World, Entity};
use crate::ecs::ui::{
    MainDialog, InventoryDialog, CharacterDialog,
    SkillBarDialog, ChatDialog, ChatType, MagicLearningDialog,
    QuestDialog, SkillsDialog, OptionsDialog, DialogType
};

/// 对话框管理系统
pub struct DialogManagerSystem;

impl DialogManagerSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 切换对话框显示/隐藏
    pub fn toggle_dialog(world: &mut World, dialog_type: DialogType) {
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
                        Self::toggle_dialog(world, DialogType::Inventory);
                    }
                    MainDialogButton::Character => {
                        Self::toggle_dialog(world, DialogType::Character);
                    }
                    MainDialogButton::Skills => {
                        Self::toggle_dialog(world, DialogType::Skills);
                    }
                    MainDialogButton::Quest => {
                        Self::toggle_dialog(world, DialogType::Quest);
                    }
                    MainDialogButton::Options => {
                        Self::toggle_dialog(world, DialogType::Options);
                    }
                    MainDialogButton::Menu => {
                        Self::toggle_dialog(world, DialogType::Menu);
                    }
                    MainDialogButton::GameShop => {
                        Self::toggle_dialog(world, DialogType::GameShop);
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
