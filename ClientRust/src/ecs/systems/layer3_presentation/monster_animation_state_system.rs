// ============================================================================
// Monster Animation State System - 怪物动画状态系统
// ============================================================================
//
// 🎯 Layer 3 - Presentation Decision Layer（表现层决策）
//
// 职责：
// - 根据怪物AI状态决定应该播放什么动画
// - 根据怪物Velocity决定是否播放移动动画
// - 为怪物实体设置AnimationStateComponent
//
// 不负责：
// - 实际播放动画帧（由AnimationPlaybackSystem负责）
// - AI逻辑和移动（由MonsterSystem负责）
//
// ============================================================================

use hecs::World;
use crate::ecs::components::{MonsterData, Animation, AIAction, Velocity, MirAction};

/// 怪物动画状态系统（Layer 3）
/// 
/// # 设计原则
/// - 只负责"决定播放什么动画"
/// - 不负责实际播放（交给AnimationPlaybackSystem）
/// - 通过读取AIAction和Velocity决定动画状态
pub struct MonsterAnimationStateSystem;

impl MonsterAnimationStateSystem {
    /// 更新所有怪物的动画状态
    /// 
    /// # 参数
    /// - `world`: ECS世界
    pub fn update(world: &mut World) {
        for (_entity, (_monster, anim, ai_state, velocity)) in 
            world.query::<(&MonsterData, &mut Animation, &crate::ecs::components::AIState, Option<&Velocity>)>().iter() 
        {
            // 根据AI状态决定动画
            let target_action = match ai_state.current_action {
                AIAction::Chase | AIAction::Patrol | AIAction::Retreat => {
                    // 移动状态
                    // 进一步检查是否有实际速度
                    if let Some(vel) = velocity {
                        if vel.dx.abs() > 0.01 || vel.dy.abs() > 0.01 {
                            MirAction::Walking
                        } else {
                            MirAction::Standing
                        }
                    } else {
                        MirAction::Walking
                    }
                }
                AIAction::Attack => {
                    // 攻击状态
                    MirAction::Attack1
                }
                AIAction::Idle => {
                    // 站立状态
                    MirAction::Standing
                }
            };
            
            // 只有当动画真正需要改变时才更新
            if anim.action != target_action {
                anim.action = target_action;
                anim.frame_index = 0;
                anim.frame_timer = 0;
            }
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::{MonsterData, Animation, AIState, Velocity, Position};

    #[test]
    fn test_monster_animation_idle() {
        let mut world = World::new();
        
        let monster = world.spawn((
            MonsterData {
                name: "TestMonster".to_string(),
                ai_type: 0,
                spawn_x: 0.0,
                spawn_y: 0.0,
            },
            Animation {
                action: MirAction::Standing,
                direction: 0,
                frame_index: 0,
                frame_count: 4,
                frame_interval: 100,
                frame_timer: 0,
            },
            AIState {
                current_action: AIAction::Idle,
                target_pos: None,
                patrol_points: Vec::new(),
                current_patrol_index: 0,
            },
        ));
        
        MonsterAnimationStateSystem::update(&mut world);
        
        // 验证动画保持Standing
        let anim = world.get::<Animation>(monster).unwrap();
        assert_eq!(anim.action, MirAction::Standing);
    }
    
    #[test]
    fn test_monster_animation_moving() {
        let mut world = World::new();
        
        let monster = world.spawn((
            MonsterData {
                name: "TestMonster".to_string(),
                ai_type: 1,
                spawn_x: 0.0,
                spawn_y: 0.0,
            },
            Animation {
                action: MirAction::Standing,
                direction: 0,
                frame_index: 0,
                frame_count: 6,
                frame_interval: 100,
                frame_timer: 0,
            },
            AIState {
                current_action: AIAction::Chase,
                target_pos: Some((10.0, 10.0)),
                patrol_points: Vec::new(),
                current_patrol_index: 0,
            },
            Velocity {
                dx: 2.0,
                dy: 0.0,
            },
        ));
        
        MonsterAnimationStateSystem::update(&mut world);
        
        // 验证动画切换到Walking
        let anim = world.get::<Animation>(monster).unwrap();
        assert_eq!(anim.action, MirAction::Walking);
        assert_eq!(anim.frame_index, 0); // 重置帧索引
    }
    
    #[test]
    fn test_monster_animation_attacking() {
        let mut world = World::new();
        
        let monster = world.spawn((
            MonsterData {
                name: "TestMonster".to_string(),
                ai_type: 1,
                spawn_x: 0.0,
                spawn_y: 0.0,
            },
            Animation {
                action: MirAction::Walking,
                direction: 0,
                frame_index: 3,
                frame_count: 6,
                frame_interval: 100,
                frame_timer: 50,
            },
            AIState {
                current_action: AIAction::Attack,
                target_pos: Some((10.0, 10.0)),
                patrol_points: Vec::new(),
                current_patrol_index: 0,
            },
        ));
        
        MonsterAnimationStateSystem::update(&mut world);
        
        // 验证动画切换到Attack1
        let anim = world.get::<Animation>(monster).unwrap();
        assert_eq!(anim.action, MirAction::Attack1);
        assert_eq!(anim.frame_index, 0); // 重置帧索引
    }
}
