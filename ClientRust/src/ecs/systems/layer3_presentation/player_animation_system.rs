// ============================================================================
// Layer 3: 表现层 - 玩家动画系统
// ============================================================================
// 职责：更新 Player 组件的动画帧（frame_index, frame_time）
// 
// 工作流程：
// 1. 读取 MovementVelocity（判断是否移动）
// 2. 读取 PlayerInput（判断是否跑步）
// 3. 更新 Player.action（Stand/Walk/Run）
// 4. 更新 Player.frame_index（动画帧）
//
// 执行顺序：在 MovementSystemV2 之后，RenderSystem 之前
// ============================================================================

use hecs::World;
use crate::ecs::components::{
    Player,
    PlayerAction,
    MovementVelocity,
    PlayerInput,
};

pub struct PlayerAnimationSystem;

impl PlayerAnimationSystem {
    /// 更新玩家动画状态
    pub fn update(world: &mut World) {
        for (_, (player, velocity, input)) in world
            .query_mut::<(&mut Player, &MovementVelocity, &PlayerInput)>()
        {
            let speed = velocity.magnitude();
            
            if speed > 1.0 {  // 移动中
                player.is_moving = true;
                
                // 根据 PlayerInput.is_running 决定走/跑
                if input.is_running {
                    player.action = PlayerAction::Run;
                } else {
                    player.action = PlayerAction::Walk;
                }
                
                // 更新动画帧
                player.frame_time += 1;
                let frame_interval = player.action.frame_interval();
                if player.frame_time >= frame_interval {
                    player.frame_time = 0;
                    player.frame_index += 1;
                    let frame_count = player.action.frame_count();
                    if player.frame_index >= frame_count {
                        player.frame_index = 0;
                    }
                }
            } else {  // 静止
                player.is_moving = false;
                player.action = PlayerAction::Stand;
                player.frame_index = 0;
                player.frame_time = 0;
            }
        }
    }
}
