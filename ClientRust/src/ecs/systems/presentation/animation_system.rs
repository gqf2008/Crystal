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
// AnimationSystem (检测动画完成)
//     ↓ 移除 AttackState 组件 + 设置 Stand
// ```
//
// ============================================================================

use ggez::GameResult;
use std::time::Instant;
use crate::ecs::{
    GameContext,
    components::{
        AnimationFrame, AttackState, Player, PlayerAction, TimeTracker,
        WeaponAnimation, WeaponState,
    },
    systems::LogicSystem,
};

#[derive(ecs_macros::LogicSystem)]
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn new() -> Self {
        Self
    }
}

impl AnimationSystem {
    /// 更新所有动画帧
    /// 
    /// **职责**: 计算并更新实体的动画帧索引
    /// - 角色身体动画帧
    /// - 武器动画帧
    pub fn update_animation_frames(&mut self, ctx: &mut GameContext) -> GameResult {
        // 获取全局时间跟踪器
        let time_tracker = ctx
            .world
            .query::<&TimeTracker>()
            .iter()
            .next()
            .map(|(_, t)| t.clone())
            .unwrap_or_default();

        // 更新所有角色的动画帧
        for (_entity, (player, anim_frame)) in ctx
            .world
            .query_mut::<(&Player, &mut AnimationFrame)>()
            .into_iter()
        {
            // 计算角色动画帧索引
            anim_frame.character_frame =
                Self::calculate_character_frame(player, &time_tracker);
        }

        // 更新所有武器的动画帧
        for (_entity, (player, weapon_state, weapon_anim, anim_frame)) in ctx
            .world
            .query_mut::<(&Player, &WeaponState, &WeaponAnimation, &mut AnimationFrame)>()
            .into_iter()
        {
            // 计算武器动画帧索引
            anim_frame.weapon_frame =
                Self::calculate_weapon_frame(player, weapon_state, weapon_anim);
        }

        Ok(())
    }

    /// 计算角色动画帧索引
    ///
    /// C# 逻辑参考: PlayerObject.cs DrawBody()
    /// ```csharp
    /// int index = BaseIndex + (Direction * FrameCount) + CurrentFrame
    /// ```
    fn calculate_character_frame(player: &Player, time_tracker: &TimeTracker) -> i32 {
        let action_start = player.action.frame_start();
        let frame_count = player.action.frame_count();
        let frame_interval = player.action.frame_interval();

        // 基于全局动画计数器计算当前帧
        let animation_tick = (time_tracker.animation_count as i32) / frame_interval;
        let current_frame = animation_tick % frame_count;

        // 计算最终索引：基础索引 + 方向偏移 + 帧偏移
        action_start + (player.direction as u8 as i32 * frame_count) + current_frame
    }

    /// 计算武器动画帧索引
    ///
    /// 武器帧计算逻辑：
    /// - 站立/行走：显示武器默认帧 (方向 * 1)
    /// - 攻击：显示攻击动画帧 (BaseIndex + 方向 * 帧数 + 当前帧)
    fn calculate_weapon_frame(
        player: &Player,
        weapon_state: &WeaponState,
        weapon_anim: &WeaponAnimation,
    ) -> i32 {
        // 如果正在攻击，显示攻击动画
        if weapon_state.is_attacking {
            // 获取当前攻击类型的帧数
            let frame_count = weapon_anim.get_attack_frames(weapon_state.current_attack);

            // 攻击动画基础索引：Attack1=0, Attack2=200, Attack3=400
            let attack_base = match weapon_state.current_attack {
                1 => 0,
                2 => 200,
                3 => 400,
                _ => 0,
            };

            // 计算索引：基础 + 方向 * 帧数 + 当前帧
            attack_base
                + (player.direction as u8 as i32 * frame_count as i32)
                + weapon_state.current_frame as i32
        } else {
            // 站立/行走：显示默认姿势（每个方向1帧）
            // 基础索引1000 + 方向
            1000 + player.direction as u8 as i32
        }
    }

    pub fn update_attack_animation(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
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

impl LogicSystem for AnimationSystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // 1. 更新所有动画帧（逻辑层职责）
        self.update_animation_frames(ctx)?;
        
        // 2. 检测攻击动画完成
        self.update_attack_animation(ctx, dt)?;
        
        Ok(())
    }
}

