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
        PlayerInput, Player, PlayerAction, Path, LocalPlayer,
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
        for (_entity, (state_machine, player_input, path, player, _local)) in ctx.world
            .query_mut::<(
                &mut PlayerStateMachine,
                &PlayerInput,
                &Path,
                &mut Player,
                &LocalPlayer,
            )>()
            .into_iter()
        {
            // 根据输入和路径状态决定目标状态
            let target_event = if path.is_valid && player_input.move_to.is_some() {
                // 有移动指令
                if player_input.is_running {
                    Some(PlayerInputEvent::StartRunning)
                } else {
                    Some(PlayerInputEvent::StartWalking)
                }
            } else if player_input.move_to.is_none() && state_machine.current_state.is_moving() {
                // 没有移动指令但还在移动状态 -> 停止
                Some(PlayerInputEvent::StopMoving)
            } else {
                None
            };

            // 执行状态转换
            if let Some(event) = target_event {
                state_machine.handle_event(event);
            }

            // 同步状态到 Player 组件的 action 字段 (用于动画系统)
            player.action = match state_machine.current_state {
                PlayerState::Idle => PlayerAction::Stand,
                PlayerState::Walking => PlayerAction::Walk,
                PlayerState::Running => PlayerAction::Run,
                // TODO: 添加更多动作类型
                _ => PlayerAction::Stand,
            };

            // 同步 is_moving 标志
            player.is_moving = state_machine.current_state.is_moving();
        }

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
