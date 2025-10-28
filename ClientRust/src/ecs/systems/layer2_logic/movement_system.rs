// ============================================================================
// Layer 2: 核心逻辑层 - 移动系统（简化版）
// ============================================================================
// 职责：纯物理运动，应用速度到位置
// 
// 工作流程：
// 1. 读取 VelocityComponent（由 LocalPredictionSystem 或 InterpolationSystem 写入）
// 2. 计算新位置：position += velocity * dt
// 3. 写入 Position 组件
//
// 设计原则：
// - 无业务逻辑：不处理寻路、碰撞、网络
// - 纯数学运算：position = position + velocity * time
// - 通用性：适用于玩家、怪物、NPC、道具
// ============================================================================

use hecs::World;
use crate::ecs::components::{Position, movement::VelocityComponent};

pub struct MovementSystem;

impl MovementSystem {
    pub fn new() -> Self {
        Self
    }

    /// 🎯 Layer 2 核心：纯物理运动（应用速度到位置）
    /// 
    /// 执行顺序：在所有写入 VelocityComponent 的系统之后
    /// 
    /// 数据流：
    /// - 读取：VelocityComponent（速度）
    /// - 写入：Position（位置）
    pub fn update(world: &mut World, dt: f32) {
        for (_entity, (position, velocity)) in world
            .query_mut::<(&mut Position, &VelocityComponent)>()
        {
            // 应用速度到位置（简单的欧拉积分）
            position.x += velocity.x * dt;
            position.y += velocity.y * dt;

            // 可选：限制速度最大值（防止穿墙）
            let speed = velocity.magnitude();
            if speed > 10.0 {
                // 如果速度过快，记录警告（可能是bug）
                tracing::warn!(
                    "[MovementSystem] ⚠️ 速度过快: {:.2} px/frame, 位置: ({:.1}, {:.1})",
                    speed,
                    position.x,
                    position.y
                );
            }
        }
    }

    /// 辅助方法：更新指定实体的位置（用于特殊情况）
    #[allow(dead_code)]
    pub fn set_position(world: &mut World, entity: hecs::Entity, new_position: Position) {
        if let Ok(mut position) = world.get::<&mut Position>(entity) {
            *position = new_position;
        }
    }

    /// 辅助方法：停止指定实体的移动
    #[allow(dead_code)]
    pub fn stop_movement(world: &mut World, entity: hecs::Entity) {
        if let Ok(mut velocity) = world.get::<&mut VelocityComponent>(entity) {
            velocity.set(0.0, 0.0);
        }
    }
}
