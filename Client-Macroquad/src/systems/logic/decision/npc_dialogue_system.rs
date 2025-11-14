// ============================================================================
// NPC Dialogue System - NPC对话系统
// ============================================================================
//
// 职责（Layer 2: 决策层）：
// - NPC对话逻辑处理
// - 对话树状态管理
// - 对话选项处理
// - 任务对话触发
//
// 不负责：
// - ❌ UI渲染（由 Layer 7 UIRenderSystem 处理）
// - ❌ 网络通信（由 Layer 1 NetworkSystem 处理）
//
// ============================================================================

use crate::game::GameContext;
use crate::game::GameResult;
use crate::systems::LogicSystem;
use hecs::World;
use std::collections::HashMap;

/// NPC对话选项
#[derive(Debug, Clone)]
pub struct DialogOption {
    pub text: String,
    pub index: u8,
    pub next_page: Option<u32>,
}

/// NPC对话页面
#[derive(Debug, Clone)]
pub struct DialogPage {
    pub text: String,
    pub options: Vec<DialogOption>,
    pub npc_script: Option<String>, // 触发的脚本
}

/// NPC对话状态
#[derive(Debug, Clone)]
pub struct DialogState {
    pub npc_id: u32,
    pub current_page: u32,
    pub is_open: bool,
}

#[derive(ecs_macros::LogicSystem)]
pub struct NpcDialogueSystem {
    /// 当前激活的对话
    active_dialogs: HashMap<hecs::Entity, DialogState>,
    
    /// 对话内容缓存（NPC ID → 对话页面）
    dialog_cache: HashMap<u32, HashMap<u32, DialogPage>>,
}

impl NpcDialogueSystem {
    pub fn new() -> Self {
        Self {
            active_dialogs: HashMap::new(),
            dialog_cache: HashMap::new(),
        }
    }
    
    /// 开始对话
    pub fn start_dialog(&mut self, player_entity: hecs::Entity, npc_id: u32) {
        tracing::info!("💬 开始与NPC对话: {}", npc_id);
        
        let state = DialogState {
            npc_id,
            current_page: 0,
            is_open: true,
        };
        
        self.active_dialogs.insert(player_entity, state);
    }
    
    /// 选择对话选项
    pub fn select_option(&mut self, player_entity: hecs::Entity, option_index: u8) {
        if let Some(state) = self.active_dialogs.get_mut(&player_entity) {
            tracing::info!("📝 选择对话选项: {} (NPC: {})", option_index, state.npc_id);
            
            // 查找对话内容
            if let Some(pages) = self.dialog_cache.get(&state.npc_id) {
                if let Some(current_page) = pages.get(&state.current_page) {
                    // 查找选项
                    if let Some(option) = current_page.options.iter().find(|o| o.index == option_index) {
                        if let Some(next_page) = option.next_page {
                            // 跳转到下一页
                            state.current_page = next_page;
                        } else {
                            // 没有下一页，关闭对话
                            state.is_open = false;
                        }
                    }
                }
            }
        }
    }
    
    /// 关闭对话
    pub fn close_dialog(&mut self, player_entity: hecs::Entity) {
        if let Some(state) = self.active_dialogs.remove(&player_entity) {
            tracing::info!("❌ 关闭对话: NPC {}", state.npc_id);
        }
    }
    
    /// 处理对话更新
    fn process_dialogs(&mut self, _world: &mut World) {
        // 清理已关闭的对话
        self.active_dialogs.retain(|_, state| state.is_open);
        
        // TODO: 处理对话超时
        // TODO: 处理对话脚本触发
    }
    
    /// 向后兼容的静态方法
    pub fn update(_world: &mut World, _delta_time: f32) {
        // 静态方法为空，实际逻辑在实例方法中
    }
}

impl LogicSystem for NpcDialogueSystem {
    
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        self.process_dialogs(&mut ctx.world);
        Ok(())
    }
}

impl Default for NpcDialogueSystem {
    fn default() -> Self {
        Self::new()
    }
}
