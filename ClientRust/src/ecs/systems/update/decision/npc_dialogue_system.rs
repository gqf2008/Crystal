// ============================================================================
// NPC Dialogue System - NPC对话系统
// ============================================================================
//
// 职责（Layer 2: 决策层）：
// - NPC对话逻辑
// - 对话树状态管理
//
// ============================================================================

use crate::ecs::systems::System;
use ggez::GameResult;

pub struct NpcDialogueSystem;

impl System for NpcDialogueSystem {
    fn name(&self) -> &'static str {
        "NpcDialogueSystem"
    }

    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::DIALOGUE
    }

    fn update(&mut self, _world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // TODO: 实现NPC对话逻辑
        Ok(())
    }
}
