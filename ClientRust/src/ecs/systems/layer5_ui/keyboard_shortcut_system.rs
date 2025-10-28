// ============================================================================
// Keyboard Shortcut System (Layer 5 - UI)
// ============================================================================
//
// 职责：
// - 处理所有键盘快捷键
// - 将按键转化为用户意图（切换UI、拾取物品、释放技能等）
// - 检查UI焦点状态（文本输入框激活时禁用游戏快捷键）
//
// 调用时机：
// - 在 on_key_down_event 中调用
// - 检查 UI 焦点后才处理游戏快捷键
//
// 依赖的系统：
// - UISystem (Layer 5) - 切换对话框
// - ItemSystem (Layer 2) - 整理背包
// - NPCSystem (Layer 3) - NPC交互
// - MagicCastSystem (Layer 2) - 释放技能
//
// ============================================================================

use hecs::World;
use tokio::sync::mpsc;
use ggez::input::keyboard::KeyCode;
use crate::network::NetworkCommand;
use crate::ecs::systems::{UISystem, ItemSystem, NPCSystem, MagicCastSystem};
use crate::ecs::ui::{DialogType, ChatDialog};

pub struct KeyboardShortcutSystem;

impl KeyboardShortcutSystem {
    /// 处理键盘输入
    /// 
    /// # 参数
    /// - `world`: ECS 世界
    /// - `keycode`: 按键代码
    /// - `network_tx`: 网络命令发送器
    pub fn process_keyboard(
        world: &mut World,
        keycode: KeyCode,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        // 检查是否有 UI 焦点（如果有输入框激活，不处理游戏快捷键）
        if Self::has_text_input_focus(world) {
            // 文本输入激活时，只处理 ESC 关闭
            if keycode == KeyCode::Escape {
                Self::close_text_input(world);
            }
            return;
        }
        
        // 处理游戏快捷键
        Self::handle_game_shortcuts(world, keycode, network_tx);
    }
    
    /// 处理游戏快捷键
    fn handle_game_shortcuts(
        world: &mut World,
        keycode: KeyCode,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use KeyCode::*;
        
        match keycode {
            // === UI 对话框快捷键 ===
            KeyI => UISystem::toggle_dialog(world, DialogType::Inventory),
            KeyC => UISystem::toggle_dialog(world, DialogType::Character),
            KeyS => UISystem::toggle_dialog(world, DialogType::Skills),
            KeyK => UISystem::toggle_dialog(world, DialogType::MagicLearning),
            KeyQ => UISystem::toggle_dialog(world, DialogType::Quest),
            KeyT => UISystem::toggle_dialog(world, DialogType::Trade),
            
            // === 游戏操作快捷键 ===
            
            // 空格键 - 拾取物品
            Space => {
                Self::pickup_item(world, network_tx);
            }
            
            // Z键 - 整理背包
            KeyZ => {
                ItemSystem::organize_inventory(world);
                tracing::info!("📦 整理背包");
            }
            
            // N键 - 与最近的NPC对话
            KeyN => {
                if let Some(npc_id) = NPCSystem::find_nearest_npc(world) {
                    NPCSystem::click_npc(world, npc_id, network_tx);
                } else {
                    tracing::warn!("⚠️ 附近没有NPC");
                }
            }
            
            // Tab键 - 切换目标
            Tab => {
                MagicCastSystem::cycle_target(world);
            }
            
            // === 技能快捷键 F1-F8 ===
            F1 => Self::cast_spell_in_slot(world, 0, network_tx),
            F2 => Self::cast_spell_in_slot(world, 1, network_tx),
            F3 => Self::cast_spell_in_slot(world, 2, network_tx),
            F4 => Self::cast_spell_in_slot(world, 3, network_tx),
            F5 => Self::cast_spell_in_slot(world, 4, network_tx),
            F6 => Self::cast_spell_in_slot(world, 5, network_tx),
            F7 => Self::cast_spell_in_slot(world, 6, network_tx),
            F8 => Self::cast_spell_in_slot(world, 7, network_tx),
            
            // === 数字键快捷栏 1-8 ===
            Digit1 => Self::use_quick_item(world, 0, network_tx),
            Digit2 => Self::use_quick_item(world, 1, network_tx),
            Digit3 => Self::use_quick_item(world, 2, network_tx),
            Digit4 => Self::use_quick_item(world, 3, network_tx),
            Digit5 => Self::use_quick_item(world, 4, network_tx),
            Digit6 => Self::use_quick_item(world, 5, network_tx),
            Digit7 => Self::use_quick_item(world, 6, network_tx),
            Digit8 => Self::use_quick_item(world, 7, network_tx),
            
            _ => {} // 其他按键不处理
        }
    }
    
    // ========================================================================
    // 辅助方法
    // ========================================================================
    
    /// 检查是否有文本输入焦点
    fn has_text_input_focus(world: &World) -> bool {
        // 检查聊天输入框是否激活
        for (_, chat) in world.query::<&ChatDialog>().iter() {
            if chat.is_input_active() {
                return true;
            }
        }
        false
    }
    
    /// 关闭文本输入
    fn close_text_input(world: &mut World) {
        for (_, chat) in world.query_mut::<&mut ChatDialog>() {
            chat.deactivate_input();
        }
    }
    
    /// 拾取物品
    fn pickup_item(world: &mut World, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        use crate::ecs::components::{Position, LocalPlayer};
        use crate::ecs::Coordinates;
        
        // 获取玩家位置
        let player_grid = {
            let mut query = world.query::<(&Position, &LocalPlayer)>();
            if let Some((_, (pos, _))) = query.into_iter().next() {
                Coordinates::world_to_grid(pos.x, pos.y)
            } else {
                tracing::warn!("⚠️ 未找到玩家");
                return;
            }
        };
        
        // 发送拾取命令
        if let Err(e) = network_tx.send(NetworkCommand::PickupItem {
            location: player_grid,
        }) {
            tracing::error!("❌ 发送拾取命令失败: {}", e);
        } else {
            tracing::info!("📦 拾取物品 at ({}, {})", player_grid.0, player_grid.1);
        }
    }
    
    /// 释放技能槽中的技能
    fn cast_spell_in_slot(
        world: &mut World,
        slot: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        use crate::ecs::components::{LocalPlayer, MagicList};
        
        // 从技能栏获取技能
        let spell = {
            let mut spell_opt = None;
            for (_, (_, magic_list)) in world.query::<(&LocalPlayer, &MagicList)>().iter() {
                if let Some(learned_magic) = magic_list.get_by_slot(slot as u8) {
                    spell_opt = Some(learned_magic.spell);
                }
                break;
            }
            spell_opt
        };
        
        if let Some(spell_type) = spell {
            MagicCastSystem::cast_spell(world, spell_type, network_tx);
        } else {
            tracing::debug!("⚠️ 技能栏 F{} 未绑定技能", slot + 1);
        }
    }
    
    /// 使用快捷栏物品
    fn use_quick_item(
        world: &mut World,
        slot: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        // TODO: 实现快捷栏物品使用逻辑
        tracing::info!("🎒 使用快捷栏物品 {}", slot + 1);
    }
}
