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

use crate::game::{GameResult, GameContext};
use std::time::Instant;
use crate::{
    components::{
        AnimationFrame, AttackState, MountState, Player, PlayerAction, TimeTracker,
    },
    systems::LogicSystem,
};
use crate::objects::frames::get_player_frame;

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
        for (_entity, (player, mount_state, anim_frame)) in ctx
            .world
            .query_mut::<(&Player, Option<&MountState>, &mut AnimationFrame)>()
            .into_iter()
        {
            // C# 原版核心：
            // - DrawFrame = Frame.Start + Frame.OffSet * Direction + FrameIndex
            // - DrawWingFrame = Frame.EffectStart + Frame.EffectOffSet * Direction + EffectFrameIndex
            // - 角色骑乘时 CurrentAction 会切到 Mount*，从而同时影响 DrawFrame/DrawWingFrame

            let mounted = mount_state.and_then(|m| m.mount_index).is_some();
            let (draw_frame, effect_frame) = Self::calculate_frames(player, &time_tracker, mounted);

            anim_frame.character_frame = draw_frame;
            // 武器/武器特效：C# 用同一套 DrawFrame，只是在取纹理时叠加 WeaponOffSet
            anim_frame.weapon_frame = draw_frame;
            anim_frame.effect_frame = effect_frame;
        }

        Ok(())
    }

    /// 计算角色动画帧索引
    ///
    /// **重构说明**: 现在从 `objects/frames.rs` 的 `PLAYER_FRAMES` 读取配置
    /// 
    /// C# 逻辑参考: PlayerObject.cs DrawBody()
    /// ```csharp
    /// int index = BaseIndex + (Direction * FrameCount) + CurrentFrame
    /// ```
    fn calculate_frames(player: &Player, time_tracker: &TimeTracker, mounted: bool) -> (i32, i32) {
        // 选择 MirAction（对齐 C#：骑乘时使用 Mount*）
        let mir_action = if mounted {
            match player.action {
                PlayerAction::Walk => mir2_shared::enums::MirAction::MountWalking,
                PlayerAction::Run => mir2_shared::enums::MirAction::MountRunning,
                // 先覆盖最常见的 3 个；攻击/受击等后续再扩展
                _ => mir2_shared::enums::MirAction::MountStanding,
            }
        } else {
            player.action.to_mir_action()
        };

        let Some(frame) = get_player_frame(mir_action) else {
            tracing::warn!("⚠️ 未找到动画配置: {:?}, 使用默认值", mir_action);
            let fallback = player.direction as u8 as i32 * 4;
            return (fallback, 0);
        };

        let dir = player.direction as u8 as i32;

        // body: DrawFrame
        let interval = frame.interval.max(1);
        let animation_tick = (time_tracker.animation_count as i32) * 100 / interval;
        let mut frame_index = animation_tick.rem_euclid(frame.count.max(1));
        if frame.reverse {
            frame_index = (frame.count.max(1) - 1) - frame_index;
        }
        let draw_frame = frame.start + (dir * frame.offset()) + frame_index;

        // effect: DrawWingFrame
        let effect_frame = if frame.effect_count > 0 {
            let effect_interval = frame.effect_interval.max(1);
            let effect_tick = (time_tracker.animation_count as i32) * 100 / effect_interval;
            let mut effect_index = effect_tick.rem_euclid(frame.effect_count.max(1));
            if frame.reverse {
                effect_index = (frame.effect_count.max(1) - 1) - effect_index;
            }
            frame.effect_start + (dir * frame.effect_offset()) + effect_index
        } else {
            // 没有 effect_* 配置时，退回到身体帧（比固定 0 更接近“跟随动作帧”）
            draw_frame
        };

        (draw_frame, effect_frame)
    }

    // 注意：武器帧独立逻辑暂时保留在 WeaponState/WeaponAnimation（用于未来的“挥砍特效触发帧”等）。
    // 但渲染使用的 weapon_frame 目前直接跟随 character_frame，避免与资源布局不一致导致“取不到纹理”。

    pub fn update_attack_animation(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
let now = Instant::now();
        
        // 收集需要移除 AttackState 的实体
        let mut finished_attacks = Vec::new();
        
        for (entity, attack_state) in ctx.world
            .query_mut::<&AttackState>()
            .into_iter()
        {
            // 从 PLAYER_FRAMES 获取攻击动画时长
            let duration_ms = if let Some(frame) = get_player_frame(attack_state.attack_type.to_mir_action()) {
                (frame.count * frame.interval) as u64
            } else {
                // 后备：默认600ms (6帧 * 100ms)
                tracing::warn!("⚠️ 未找到攻击动画配置: {:?}, 使用默认时长", attack_state.attack_type);
                600
            };
            
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{MirDirection, Player};

    #[test]
    fn effect_frame_matches_csharp_drawwingframe_formula_for_mount_standing() {
        let player = Player {
            direction: MirDirection::Down,
            action: PlayerAction::Stand,
        };
        let time_tracker = TimeTracker::default();

        // mounted=true 会将 Stand 映射为 MountStanding
        let (draw_frame, effect_frame) = AnimationSystem::calculate_frames(&player, &time_tracker, true);
        let frame = get_player_frame(mir2_shared::enums::MirAction::MountStanding).expect("mount standing frame");

        let dir = player.direction as u8 as i32;
        let expected_draw = frame.start + dir * frame.offset();
        let expected_effect = frame.effect_start + dir * frame.effect_offset();

        assert_eq!(draw_frame, expected_draw);
        assert_eq!(effect_frame, expected_effect);
        assert!(frame.effect_count > 0);
    }
}

