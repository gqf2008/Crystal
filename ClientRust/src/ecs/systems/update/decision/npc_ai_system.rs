// ============================================================================
// NPC AI System - NPC AI系统
// ============================================================================
//
// 职责（Layer 2: 决策层）：
// - NPC行为AI
// - NPC对话触发判断
//
// ============================================================================

use crate::ecs::systems::System;
use ggez::GameResult;

pub struct NpcAISystem;

impl System for NpcAISystem {
    fn name(&self) -> &'static str {
        "NpcAISystem"
    }

    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::NPC_AI
    }

    fn update(&mut self, _world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // TODO: 实现NPC AI逻辑
        Ok(())
    }
}
