// ============================================================================
// Player State System - 玩家状态系统
// Priority: 380 (在 PathfindingSystem 之后, MovementSystem 之前)
// ============================================================================
//
// **职责**:
// - 根据 PlayerInput 和 Path 自动管理状态转换
// - 确保状态转换的合法性
// - 同步状态到 Player 组件的 action 字段
//
// **数据流**:
// ```
// PlayerInput + Path
//     ↓
// PlayerStateMachine (状态转换)
//     ↓
// Player.action (同步动画状态)
//     ↓
// AnimationSystem 播放动画
// ```
//
// ============================================================================

use ggez::GameResult;
use crate::ecs::{
    GameContext,
    components::{
        PlayerStateMachine, PlayerState, PlayerInputEvent,
        PlayerInput, PlayerAction, Path, LocalPlayer, Player,
    },
    systems::System,
};

pub struct PlayerStateSystem;

impl PlayerStateSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for PlayerStateSystem {
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::PLAYER_STATE
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 处理本地玩家的状态转换
        for (_entity, (state_machine, player_input, path, player, _local, velocity)) in ctx.world
            .query_mut::<(
                &mut PlayerStateMachine,
                &PlayerInput,
                &Path,
                &mut Player,
                &LocalPlayer,
                &crate::ecs::components::MovementVelocity,
            )>()
            .into_iter()
        {
            // 🎯 关键修复：动画播放应该由 mouse_pressed 决定，而不是 move_to 或 velocity
            // 需求：
            // 1. 鼠标按下 → 动画播放（即使碰到障碍物velocity=0也继续）
            // 2. 鼠标松开 → 立即停止动画
            // 3. 碰撞时 → velocity=0停止位移，但mouse_pressed=true保持动画
            let should_play_animation = player_input.mouse_pressed;
            
            // 根据鼠标按下状态决定目标状态
            let target_event = if should_play_animation {
                // 鼠标按下：播放走/跑动画
                if player_input.is_running {
                    Some(PlayerInputEvent::StartRunning)
                } else {
                    Some(PlayerInputEvent::StartWalking)
                }
            } else if !player_input.mouse_pressed && state_machine.current_state.is_moving() {
                // 鼠标松开但还在移动状态 → 立即停止
                Some(PlayerInputEvent::StopMoving)
            } else {
                None
            };

            // 执行状态转换
            if let Some(event) = target_event {
                state_machine.handle_event(event);
            }

            // 同步状态到 Player 组件的 action 字段 (用于动画系统)
            // ⚔️ 不覆盖攻击动作 (由 AttackSystem 管理)
            if !player.action.is_attack() {
                let new_action = match state_machine.current_state {
                    PlayerState::Idle => PlayerAction::Stand,
                    PlayerState::Walking => PlayerAction::Walk,
                    PlayerState::Running => PlayerAction::Run,
                    // TODO: 添加更多动作类型
                    _ => PlayerAction::Stand,
                };
                
                // 调试日志：状态变化
                if player.action != new_action {
                    tracing::info!(
                        "🎬 Player动作切换: {:?} -> {:?} (mouse_pressed={}, is_running={})",
                        player.action, new_action, player_input.mouse_pressed, player_input.is_running
                    );
                }
                player.action = new_action;
            }

            // 同步 is_moving 标志
            player.is_moving = state_machine.current_state.is_moving();
        }
        
        // 注意: AnimationControl 同步代码已移除（未使用）
        // 渲染系统直接使用 Player.action 和 PlayerAction.frame_interval()

        Ok(())
    }
}

impl Default for PlayerStateSystem {
    fn default() -> Self {
        Self::new()
    }
}

// 声明为逻辑系统
crate::logic_system!(PlayerStateSystem);
