// ============================================================================
// NPC AI System - NPC AI系统
// ============================================================================
//
// 职责（Layer 2: 决策层）：
// - NPC行为AI（闲逛、面向玩家等）
// - NPC对话触发判断
// - NPC交互检测
//
// 不负责：
// - ❌ 实际移动（由 Layer 4 MovementSystem 处理）
// - ❌ 对话内容处理（由 NpcDialogueSystem 处理）
// - ❌ 动画播放（由 Layer 5 AnimationSystem 处理）
//
// ============================================================================

use crate::game::GameContext;
use crate::systems::LogicSystem;
use crate::components::{Position, NPC, AIState, LocalPlayer};
use crate::game::GameResult;
use hecs::World;

pub struct NpcAISystem;

impl NpcAISystem {
    /// 更新NPC AI逻辑
    fn update_npc_ai(world: &mut World) {
        // 获取玩家位置
        let player_pos = Self::find_player_position(world);
        
        // 遍历所有NPC
        for (_entity, (npc, pos, mut ai_state)) in world.query_mut::<(&NPC, &Position, Option<&mut AIState>)>() {
            // 1. 检测玩家是否靠近（可以对话）
            if let Some((px, py)) = player_pos {
                let distance = Self::calculate_distance((pos.x, pos.y), (px, py));
                
                // 玩家在2格范围内，可以对话
                if distance < 2.0 {
                    tracing::debug!("💬 玩家靠近NPC: {}", npc.name);
                    // TODO: 触发对话提示UI
                }
                
                // 2. NPC面向玩家（如果距离很近）
                if distance < 3.0 {
                    if let Some(ai) = ai_state.as_deref_mut() {
                        // 计算朝向玩家的方向
                        let dx = px - pos.x;
                        let dy = py - pos.y;
                        let _direction = Self::calculate_direction(dx, dy);
                        
                        // 更新AI状态为面向玩家
                        ai.current_action = crate::ecs::components::AIAction::Idle;
                        // Note: AIState 没有 facing_direction 字段，方向由其他组件控制
                    }
                }
            }
            
            // 3. NPC闲逛逻辑（如果配置了）
            if !npc.can_interact {
                // 非交互NPC才可能闲逛
                if let Some(_ai) = ai_state.as_deref_mut() {
                    // TODO: 实现随机闲逛逻辑
                    // - 每隔一段时间随机选择一个方向
                    // - 行走几步后停下
                }
            }
        }
    }
    
    /// 查找玩家位置
    fn find_player_position(world: &World) -> Option<(f32, f32)> {
        world.query::<(&LocalPlayer, &Position)>()
            .iter()
            .next()
            .map(|(_, (_, pos))| (pos.x, pos.y))
    }
    
    /// 计算距离
    fn calculate_distance(pos1: (f32, f32), pos2: (f32, f32)) -> f32 {
        let dx = pos2.0 - pos1.0;
        let dy = pos2.1 - pos1.1;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// 计算方向（0-7，八方向）
    fn calculate_direction(dx: f32, dy: f32) -> u8 {
        let angle = dy.atan2(dx);
        let deg = angle.to_degrees();
        
        // 转换为0-7的八方向
        let direction = ((deg + 22.5) / 45.0).floor() as i32;
        ((direction + 8) % 8) as u8
    }
}

impl LogicSystem for NpcAISystem {

   

    fn update(&mut self,  ctx:&mut GameContext, _delay_time: f32) -> GameResult {
        Self::update_npc_ai(&mut ctx.world);
        Ok(())
    }
}

impl Default for NpcAISystem {
    fn default() -> Self {
        Self
    }
}
