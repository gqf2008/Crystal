// ============================================================================
// Animation Playback System - 动画帧播放系统 (Layer 4)
// ============================================================================
//
// 职责：
// - 更新所有实体的动画帧索引 (frame_index)
// - 纯播放逻辑，不决定播放什么动画
// - 根据 Animation 组件的配置自动推进帧
//
// 与其他系统的关系：
// - Layer 3 (AnimationStateSystem) 决定动画状态 → 写入 Animation.action
// - Layer 4 (本系统) 播放动画 → 更新 Animation.frame_index
// - Layer 4 (RenderSystem) 渲染动画 → 读取 Animation.frame_index
//
// 替代：
// - deprecated/AnimationSystem::update_entities()
//
// ============================================================================

use hecs::World;
use crate::ecs::components::Animation;

/// 动画帧播放系统
pub struct AnimationPlaybackSystem;

impl AnimationPlaybackSystem {
    /// 更新所有实体的动画帧
    /// 
    /// # 参数
    /// - world: ECS世界
    /// - delta_ms: 距上一帧的时间差(毫秒)
    /// 
    /// # 功能
    /// - 遍历所有带 Animation 组件的实体
    /// - 累积帧计时器
    /// - 超过帧间隔时推进到下一帧
    /// - 自动循环播放
    pub fn update(world: &mut World, delta_ms: u32) {
        for (_entity, anim) in world.query_mut::<&mut Animation>() {
            anim.update(delta_ms);
        }
    }
}
