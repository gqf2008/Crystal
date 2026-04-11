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
use crate::components::{Position, NPC, AIState, LocalPlayer, InteractionHint};
use crate::game::GameResult;
use hecs::World;

/// 可交互距离阈值（格子）
const INTERACT_RANGE: f32 = 2.0;

#[derive(ecs_macros::LogicSystem)]
pub struct NpcAISystem;

impl NpcAISystem {
    /// 更新NPC AI逻辑
    fn update_npc_ai(world: &mut World) {
        // 获取玩家位置
        let player_pos = Self::find_player_position(world);

        // 第一遍：更新 InteractionHint 标记
        // 收集需要添加/移除标记的 NPC 实体（避免在迭代时修改组件）
        let mut to_add_hint: Vec<hecs::Entity> = Vec::new();
        let mut to_remove_hint: Vec<hecs::Entity> = Vec::new();

        // 第一遍：收集 NPC 数据（分两步避免借用冲突）
        let npc_data: Vec<(hecs::Entity, bool, f32, f32)> = world
            .query_mut::<(hecs::Entity, &NPC, &Position)>()
            .into_iter()
            .map(|(e, npc, pos)| (e, npc.can_interact, pos.x, pos.y))
            .collect();

        // 第二步：检查 hint 标记并分类
        for (entity, can_interact, nx, ny) in &npc_data {
            let has_hint = world.get::<&InteractionHint>(*entity).is_ok();
            let in_range = player_pos
                .map(|(px, py)| Self::calculate_distance((*nx, *ny), (px, py)) < INTERACT_RANGE)
                .unwrap_or(false);

            if *can_interact && in_range && !has_hint {
                to_add_hint.push(*entity);
            } else if (!*can_interact || !in_range) && has_hint {
                to_remove_hint.push(*entity);
            }
        }

        for entity in to_add_hint {
            let _ = world.insert_one(entity, InteractionHint);
        }
        for entity in to_remove_hint {
            let _ = world.remove_one::<InteractionHint>(entity);
        }

        // 第二遍：更新 NPC AI 状态
        for (npc, pos, mut ai_state) in world.query_mut::<(&NPC, &Position, Option<&mut AIState>)>() {
            // 1. 检测玩家是否靠近（可以对话）
            if let Some((px, py)) = player_pos {
                let distance = Self::calculate_distance((pos.x, pos.y), (px, py));

                // 玩家在可交互范围内
                if distance < INTERACT_RANGE && npc.can_interact {
                    tracing::debug!("💬 玩家靠近可交互NPC: {}", npc.name);
                }

                // 2. NPC面向玩家（如果距离很近）
                if distance < 3.0 {
                    if let Some(ai) = ai_state.as_deref_mut() {
                        // 计算朝向玩家的方向
                        let dx = px - pos.x;
                        let dy = py - pos.y;
                        let _direction = Self::calculate_direction(dx, dy);

                        // 更新AI状态为面向玩家
                        ai.current_action = crate::components::AIAction::Idle;
                        // Note: AIState 没有 facing_direction 字段，方向由其他组件控制
                    }
                }
            }

            // 3. NPC闲逛逻辑（如果配置了）
            if !npc.can_interact {
                // 非交互NPC才可能闲逛
                if let Some(_ai) = ai_state {
                    // 注：NPC 位置由服务器同步驱动，本地闲逛仅在 mock 模式下有意义
                }
            }
        }
    }
    
    /// 查找玩家位置
    fn find_player_position(world: &World) -> Option<(f32, f32)> {
        world.query::<(&LocalPlayer, &Position)>()
            .iter()
            .next()
            .map(|(_local, pos)| (pos.x, pos.y))
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
