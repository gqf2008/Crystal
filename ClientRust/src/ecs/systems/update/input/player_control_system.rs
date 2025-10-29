// ============================================================================
// Player Control System - 玩家控制系统
// ============================================================================
//
// 职责（Layer 1: 输入与网络层）：
// - 将 PlayerInput 转换为具体的游戏指令
// - 处理玩家输入的验证和限制
// - 状态转换逻辑（站立→行走→跑步→攻击等）
//
// 不负责：
// - ❌ 实际移动（由 Layer 4 MovementSystem 处理）
// - ❌ 寻路计算（由 PathfindingService 处理）
// - ❌ 网络发送（由 NetworkSystem 处理）
//
// ============================================================================

use crate::ecs::systems::System;
use ggez::GameResult;
use hecs::World;
use crate::ecs::components::{PlayerInput, LocalPlayer, Player, Position, PlayerAction, MoveMode};

/// 玩家控制系统
pub struct PlayerControlSystem;

impl PlayerControlSystem {
    

    /// 处理玩家命令
    fn process_player_commands(world: &mut World) {
        // 查找本地玩家
        for (_entity, (_, player_input, player, _pos)) in world.query_mut::<(
            &LocalPlayer,
            &PlayerInput,
            &mut Player,
            &Position,
        )>() {
            // 1. 处理移动指令
            if let Some(move_to) = player_input.move_to {
                tracing::debug!("🎮 玩家控制: 移动到 ({}, {}), 跑步={}", 
                    move_to.0, move_to.1, player_input.is_running);
                
                // 更新目标位置
                player.target_x = move_to.0;
                player.target_y = move_to.1;
                player.is_moving = true;
                
                // 根据输入设置行走/跑步动作
                if player_input.is_running && player.can_run {
                    player.action = PlayerAction::Run;
                } else {
                    player.action = PlayerAction::Walk;
                }
                
                // 设置移动模式
                if player_input.use_pathfinding {
                    player.move_mode = MoveMode::AutoPathfinding;
                } else {
                    player.move_mode = MoveMode::DirectFollow;
                }
            } else if !player.is_moving {
                // 没有移动输入且不在移动中，切换到站立
                player.action = PlayerAction::Stand;
                player.move_mode = MoveMode::Idle;
            }
            
            // 2. 处理攻击指令
            if let Some(_target_id) = player_input.attack_target {
                tracing::debug!("⚔️ 玩家控制: 攻击目标 {:?}", _target_id);
                // TODO: 设置攻击状态（需要等待 CombatSystem 实现）
                // 攻击时停止移动
                player.is_moving = false;
            }
            
            // 3. 处理施法指令
            if let Some(_spell_input) = &player_input.cast_spell {
                tracing::debug!("✨ 玩家控制: 施法");
                // TODO: 设置施法状态（需要等待 SkillSystem 实现）
                // 施法时停止移动
                player.is_moving = false;
            }
        }
    }
}

impl System for PlayerControlSystem {
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::PLAYER_CONTROL
    }

    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        Self::process_player_commands(world);
        Ok(())
    }
}