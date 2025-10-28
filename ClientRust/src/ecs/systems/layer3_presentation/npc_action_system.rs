// ============================================================================
// NPC Action System - NPC动作决策系统 (Layer 3)
// ============================================================================
//
// 职责：
// - 决定NPC应该播放什么动作（Standing/Harvest）
// - 实现智能动作切换，带随机延迟
// - 属于表现决策层，不涉及游戏逻辑
//
// 特性：
// - 动作权重: Standing 70%, Harvest 30%
// - 完整播放: 等待当前动画循环完成后再切换
// - 随机延迟: 3-8秒随机间隔,避免所有NPC同步
//
// 迁移自：
// - deprecated/AnimationSystem::NPCActionSystem
//
// ============================================================================

use hecs::World;
use crate::ecs::components::{NPCData, Animation};
use crate::objects::frames::{DEFAULT_NPC_FRAMES, get_frame};
use mir2_shared::MirAction;
use rand::Rng;

/// NPC动作决策系统
pub struct NPCActionSystem;

impl NPCActionSystem {
    /// 更新NPC动作，实现智能切换Standing/Harvest
    /// 
    /// # 参数
    /// - world: ECS世界
    /// - delta_ms: 距上一帧的时间差(毫秒)
    /// 
    /// # 逻辑流程
    /// 1. 累积动作计时器
    /// 2. 到达切换时间时，根据权重选择新动作
    /// 3. 等待当前动画播放到最后一帧才切换
    /// 4. 从FrameSet读取新动作的配置
    /// 5. 重置计时器，设置新的随机延迟
    pub fn update(world: &mut World, delta_ms: u32) {
        for (_entity, (npc, anim)) in world.query_mut::<(&mut NPCData, &mut Animation)>() {
            // 只在Standing和Harvest之间切换
            if anim.action != MirAction::Standing && anim.action != MirAction::Harvest {
                continue;
            }
            
            // 累积计时器
            npc.action_timer += delta_ms;
            
            // 检查是否到达切换时间
            if npc.action_timer >= npc.next_action_delay {
                // 根据权重选择新动作: Standing 70%, Harvest 30%
                let roll = rand::rng().random_range(0..100);
                let new_action = if roll < 70 {
                    MirAction::Standing
                } else {
                    MirAction::Harvest
                };
                
                // 只有在真正需要切换动作时才处理
                if new_action != anim.action {
                    // 检查当前动画是否接近循环点(最后一帧) - 只有切换时才需要检查
                    let is_near_loop = anim.frame_index >= anim.frame_count.saturating_sub(1);
                    
                    if is_near_loop || anim.frame_count == 0 {
                        // 从FrameSet读取新动作的配置
                        if let Some(frame) = get_frame(&DEFAULT_NPC_FRAMES, new_action) {
                            tracing::debug!(
                                "🏪 NPC {} 切换动作: {:?} -> {:?}", 
                                npc.name, 
                                anim.action, 
                                new_action
                            );
                            
                            // 更新动画状态（决策层职责）
                            anim.action = new_action;
                            anim.frame_count = frame.count as u8;
                            anim.frame_interval = frame.interval as u32;
                            anim.frame_index = 0;
                            anim.frame_timer = 0;
                        }
                    }
                }
                
                // 无论是否切换，都重置计时器以避免重复触发
                npc.action_timer = 0;
                npc.next_action_delay = rand::rng().random_range(3000..8000);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_npc_action_timer() {
        let mut world = World::new();
        
        let entity = world.spawn((
            NPCData {
                name: "测试NPC".to_string(),
                action_timer: 0,
                next_action_delay: 5000,
                ..Default::default()
            },
            Animation {
                action: MirAction::Standing,
                frame_count: 4,
                frame_index: 3, // 最后一帧
                ..Default::default()
            },
        ));
        
        // 更新3秒，不应该切换
        NPCActionSystem::update(&mut world, 3000);
        let npc = world.get::<&NPCData>(entity).unwrap();
        assert_eq!(npc.action_timer, 3000);
        
        // 再更新2秒，应该触发切换检查
        NPCActionSystem::update(&mut world, 2000);
        let npc = world.get::<&NPCData>(entity).unwrap();
        assert_eq!(npc.action_timer, 0); // 计时器已重置
        assert!(npc.next_action_delay >= 3000 && npc.next_action_delay <= 8000);
    }
}
