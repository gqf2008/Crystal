// ============================================================================
// Attack System - 攻击动画管理系统
// Priority: 400 (在 PlayerStateSystem 之后, MovementSystem 之前)
// ============================================================================
//
// **职责**:
// - 检测攻击动画完成
// - 自动移除 AttackState 组件
// - 恢复角色到 Stand 状态
//
// **ECS 设计原则**:
// - ✅ 无状态 System (所有状态存储在 AttackState Component)
// - ✅ 单一职责 (只负责攻击动画生命周期管理)
// - ✅ 组件驱动 (通过 AttackState 组件查询攻击中的实体)
//
// **数据流**:
// ```
// PlayerControlSystem (右键点击)
//     ↓ 添加 AttackState 组件
// AttackSystem (检测动画完成)
//     ↓ 移除 AttackState 组件 + 设置 Stand
// PlayerStateSystem (同步状态)
// ```
//
// ============================================================================

use ggez::GameResult;
use std::time::Instant;
use crate::ecs::{
    GameContext,
    components::{AttackState, Player, PlayerAction},
    systems::System,
};

pub struct AttackSystem;

impl AttackSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for AttackSystem {
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::ATTACK
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let now = Instant::now();
        
        // 收集需要移除 AttackState 的实体
        let mut finished_attacks = Vec::new();
        
        for (entity, attack_state) in ctx.world
            .query_mut::<&AttackState>()
            .into_iter()
        {
            // 计算攻击动画是否完成
            let duration_ms = attack_state.attack_type.duration_ms();
            let elapsed = now.duration_since(attack_state.start_time).as_millis() as u64;
            
            if elapsed >= duration_ms {
                finished_attacks.push(entity);
                tracing::debug!(
                    "⚔️ 攻击动画完成: {:?} (耗时 {}ms)",
                    attack_state.attack_type,
                    elapsed
                );
            }
        }
        
        // 移除完成的攻击状态并恢复 Stand
        for entity in finished_attacks {
            // 移除 AttackState 组件
            let _ = ctx.world.remove_one::<AttackState>(entity);
            
            // 恢复到站立状态
            if let Ok(player) = ctx.world.query_one_mut::<&mut Player>(entity) {
                player.action = PlayerAction::Stand;
                tracing::info!("✅ 攻击完成，返回站立状态");
            }
        }
        
        Ok(())
    }
}

impl Default for AttackSystem {
    fn default() -> Self {
        Self::new()
    }
}
