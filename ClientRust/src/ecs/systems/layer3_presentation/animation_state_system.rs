// ============================================================================
// Layer 3: 表现状态层 - 动画状态系统
// ============================================================================
// 职责：根据游戏逻辑状态决定应该播放的动画状态
// 
// 工作流程：
// 1. 读取 MovementState（由 LocalPredictionSystem 写入）
// 2. 读取 Player 组件（获取方向、武器、装备等信息）
// 3. 决定动画状态（Idle/Walk/Run/Attack等）
// 4. 写入 AnimationState（由渲染系统读取）
//
// 设计原则：
// - 决策逻辑：只负责"应该播放什么动画"
// - 不播放动画：实际播放由 AnimationSystem 负责
// - 状态转换：处理动画状态切换的逻辑
// ============================================================================

use hecs::World;
use crate::ecs::components::{
    Player,
    movement::{MovementState, Movement},
    animation_state::{AnimationState, AnimationControl},
};

pub struct AnimationStateSystem;

impl AnimationStateSystem {
    pub fn new() -> Self {
        Self
    }

    /// 🎯 Layer 3 核心：动画状态决策系统
    /// 
    /// 执行顺序：在 MovementSystemV2 之后，RenderSystem 之前
    /// 
    /// 数据流：
    /// - 读取：Movement（移动状态）、Player（角色信息）
    /// - 写入：AnimationControl（动画控制）
    pub fn update(world: &mut World, _dt: f32) {
        for (_entity, (player, movement, animation_ctrl)) in world
            .query_mut::<(
                &Player,
                &Movement,
                &mut AnimationControl,
            )>()
        {
            // 1️⃣ 根据移动状态决定基础动画
            let desired_state = match movement.state {
                MovementState::Idle => AnimationState::Idle,
                MovementState::Walking => AnimationState::Walk,
                MovementState::Running => AnimationState::Run,
                MovementState::Knocked => AnimationState::Hit,
            };

            // 2️⃣ 检查是否需要切换动画状态
            if animation_ctrl.current_state != desired_state {
                // 特殊情况：某些动画需要播放完毕才能切换
                if Self::can_interrupt(animation_ctrl.current_state) {
                    tracing::debug!(
                        "[AnimationStateSystem] 动画状态切换: {:?} -> {:?}",
                        animation_ctrl.current_state,
                        desired_state
                    );
                    animation_ctrl.set_state(desired_state);
                } else {
                    // 某些动画（如攻击、施法）需要播放完才能切换
                    if animation_ctrl.is_finished() {
                        animation_ctrl.set_state(desired_state);
                    }
                }
            }

            // 3️⃣ 更新动画方向（角色转向时更新）
            if animation_ctrl.direction != player.direction {
                animation_ctrl.direction = player.direction;
            }

            // 4️⃣ 处理特殊动画（死亡动画只播放一次）
            if animation_ctrl.current_state == AnimationState::Die {
                if animation_ctrl.is_finished() {
                    animation_ctrl.loop_animation = false;
                }
            }
        }
    }

    /// 辅助方法：判断当前动画是否可以被中断
    /// 
    /// 规则：
    /// - Idle/Walk/Run 可以随时中断
    /// - Attack/Spell 需要播放完毕
    /// - Die 不可中断
    fn can_interrupt(state: AnimationState) -> bool {
        match state {
            AnimationState::Idle 
            | AnimationState::Walk 
            | AnimationState::Run 
            | AnimationState::Hit 
            | AnimationState::Harvest => true,
            
            AnimationState::Attack 
            | AnimationState::Spell => false, // 需要播放完攻击/施法动画
            
            AnimationState::Die => false, // 死亡动画不可中断
        }
    }

    /// 辅助方法：处理攻击动画
    /// 
    /// 这个方法会在战斗系统中调用
    #[allow(dead_code)]
    pub fn trigger_attack(world: &mut World, entity: hecs::Entity) {
        if let Ok(mut animation_ctrl) = world.get::<&mut AnimationControl>(entity) {
            animation_ctrl.set_state(AnimationState::Attack);
            tracing::debug!("[AnimationStateSystem] 触发攻击动画");
        }
    }

    /// 辅助方法：处理施法动画
    #[allow(dead_code)]
    pub fn trigger_spell(world: &mut World, entity: hecs::Entity) {
        if let Ok(mut animation_ctrl) = world.get::<&mut AnimationControl>(entity) {
            animation_ctrl.set_state(AnimationState::Spell);
            tracing::debug!("[AnimationStateSystem] 触发施法动画");
        }
    }

    /// 辅助方法：处理死亡动画
    #[allow(dead_code)]
    pub fn trigger_death(world: &mut World, entity: hecs::Entity) {
        if let Ok(mut animation_ctrl) = world.get::<&mut AnimationControl>(entity) {
            animation_ctrl.set_state(AnimationState::Die);
            animation_ctrl.loop_animation = false; // 死亡动画只播放一次
            tracing::info!("[AnimationStateSystem] 触发死亡动画");
        }
    }

    /// 辅助方法：处理受击动画
    #[allow(dead_code)]
    pub fn trigger_hit(world: &mut World, entity: hecs::Entity) {
        if let Ok(mut animation_ctrl) = world.get::<&mut AnimationControl>(entity) {
            // 只有在非攻击/施法状态下才触发受击动画
            if Self::can_interrupt(animation_ctrl.current_state) {
                animation_ctrl.set_state(AnimationState::Hit);
                tracing::debug!("[AnimationStateSystem] 触发受击动画");
            }
        }
    }
}
