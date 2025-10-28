// ============================================================================
// Layer 2: 核心逻辑层 - 插值系统
// ============================================================================
// 职责：对其他玩家/怪物/NPC 应用平滑插值移动
// 
// 工作流程：
// 1. 读取 Interpolation（由 ClientNetworkSystem 或 ReconciliationSystem 写入）
// 2. 计算插值进度：progress = elapsed_time / duration
// 3. 线性插值：position = lerp(from, to, progress)
// 4. 写入 Position 组件
//
// 设计原则：
// - 平滑性：网络更新是离散的（100ms一次），插值让移动连续
// - 预测性：渲染位置稍微落后于服务器（100ms buffer）
// - 通用性：适用于所有非本地实体
// ============================================================================

use hecs::World;
use crate::ecs::components::{
    Position,
    LocalPlayer,
    prediction::Interpolation,
};

pub struct InterpolationSystem;

impl InterpolationSystem {
    pub fn new() -> Self {
        Self
    }

    /// 🎯 Layer 2 核心：插值系统（平滑非本地实体移动）
    /// 
    /// 执行顺序：在 ClientNetworkSystem 接收移动数据之后
    /// 
    /// 数据流：
    /// - 读取：Interpolation（插值状态）
    /// - 写入：Position（平滑插值后的位置）
    pub fn update(world: &mut World, dt: f32) {
        // 遍历所有非本地玩家的实体（怪物、其他玩家、NPC等）
        for (_entity, (position, interpolation)) in world
            .query_mut::<(&mut Position, &mut Interpolation)>()
            .without::<&LocalPlayer>() // 排除本地玩家（本地玩家使用客户端预测）
        {
            // 1️⃣ 检查是否正在插值
            if !interpolation.is_active {
                continue;
            }

            // 2️⃣ 更新插值（Interpolation 有内置的 update 方法）
            if let Some(new_pos) = interpolation.update(dt) {
                *position = new_pos;
                
                // 检查插值是否完成
                if !interpolation.is_active {
                    tracing::debug!(
                        "[InterpolationSystem] ✅ 插值完成: 到达目标位置 ({:.1}, {:.1})",
                        interpolation.to_position.x,
                        interpolation.to_position.y
                    );
                }
            }
        }
    }

    /// 辅助方法：立即停止插值并设置到目标位置
    #[allow(dead_code)]
    pub fn snap_to_target(world: &mut World, entity: hecs::Entity) {
        if let Ok((mut position, mut interpolation)) = world.query_one_mut::<(
            &mut Position,
            &mut Interpolation,
        )>(entity) {
            if interpolation.is_active {
                *position = interpolation.to_position.clone();
                interpolation.stop();
                
                tracing::info!(
                    "[InterpolationSystem] ⚡ 强制跳到目标位置: ({:.1}, {:.1})",
                    position.x,
                    position.y
                );
            }
        }
    }
}
