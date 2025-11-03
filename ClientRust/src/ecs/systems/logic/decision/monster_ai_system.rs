// ============================================================================
// Monster AI System - 怪物AI系统
// ============================================================================
//
// 职责（Layer 2: 决策层）：
// - 怪物AI更新（追击、攻击、巡逻等）
// - AI状态决策（根据玩家位置、距离等）
// - 巡逻路径规划
//
// 不负责：
// - ❌ 实际移动（由 Layer 4 MovementSystem 处理）
// - ❌ 动画更新（由 Layer 5 AnimationSystem 处理）
// - ❌ 战斗伤害计算（由 Layer 3 CombatSystem 处理）
//
// 对应 C# 版本：
// - MonsterObject.ProcessAI()
//
// ============================================================================

use hecs::World;
use crate::ecs::GameContext;
use crate::ecs::components::{
    Position, MonsterData, Health, AIState,
    Player,
};
use crate::ecs::systems::System;
use ggez::GameResult;

/// 怪物AI系统
pub struct MonsterAISystem;

impl MonsterAISystem {
   
    /// 更新怪物AI逻辑
    fn update_ai(world: &mut World) {
        // 首先查找玩家位置
        let player_pos = Self::find_player_position(world);
        
        // 遍历所有怪物
        for (_entity, (monster, pos, ai_state, health)) in 
            world.query::<(&MonsterData, &mut Position, &mut AIState, &Health)>().iter() 
        {
            // 跳过死亡怪物
            if health.current <= 0 {
                continue;
            }
            
            // 根据AI类型执行不同逻辑
            match monster.ai_type {
                0 => {
                    // AI = 0: 无AI，静止不动
                    ai_state.current_action = crate::ecs::components::AIAction::Idle;
                }
                1 => {
                    // AI = 1: 近战攻击型（最常见）
                    Self::ai_melee_attack(pos, ai_state, player_pos);
                }
                2 => {
                    // AI = 2: 远程攻击型
                    Self::ai_ranged_attack(pos, ai_state, player_pos);
                }
                3 => {
                    // AI = 3: 巡逻型
                    Self::ai_patrol(pos, ai_state, monster);
                }
                _ => {
                    // 其他AI类型暂未实现
                }
            }
        }
    }
    
    /// 近战攻击AI
    fn ai_melee_attack(
        pos: &mut Position,
        ai_state: &mut AIState,
        player_pos: Option<(f32, f32)>
    ) {
        if let Some((px, py)) = player_pos {
            let distance = Self::distance((pos.x, pos.y), (px, py));
            
            if distance < 1.5 {
                // 在攻击范围内 - 攻击
                ai_state.current_action = crate::ecs::components::AIAction::Attack;
                ai_state.target_pos = Some((px, py));
            } else if distance < 10.0 {
                // 在视野范围内 - 追击
                ai_state.current_action = crate::ecs::components::AIAction::Chase;
                ai_state.target_pos = Some((px, py));
            } else {
                // 超出视野 - 闲置
                ai_state.current_action = crate::ecs::components::AIAction::Idle;
                ai_state.target_pos = None;
            }
        } else {
            // 找不到玩家 - 闲置
            ai_state.current_action = crate::ecs::components::AIAction::Idle;
            ai_state.target_pos = None;
        }
    }
    
    /// 远程攻击AI
    fn ai_ranged_attack(
        pos: &mut Position,
        ai_state: &mut AIState,
        player_pos: Option<(f32, f32)>
    ) {
        if let Some((px, py)) = player_pos {
            let distance = Self::distance((pos.x, pos.y), (px, py));
            
            if distance >= 3.0 && distance < 8.0 {
                // 在最佳攻击范围内 - 攻击
                ai_state.current_action = crate::ecs::components::AIAction::Attack;
                ai_state.target_pos = Some((px, py));
            } else if distance < 3.0 {
                // 太近了 - 后退
                ai_state.current_action = crate::ecs::components::AIAction::Retreat;
                ai_state.target_pos = Some((px, py));
            } else if distance < 12.0 {
                // 太远了 - 追击
                ai_state.current_action = crate::ecs::components::AIAction::Chase;
                ai_state.target_pos = Some((px, py));
            } else {
                // 超出视野 - 闲置
                ai_state.current_action = crate::ecs::components::AIAction::Idle;
                ai_state.target_pos = None;
            }
        } else {
            ai_state.current_action = crate::ecs::components::AIAction::Idle;
            ai_state.target_pos = None;
        }
    }
    
    /// 巡逻AI
    fn ai_patrol(
        pos: &mut Position,
        ai_state: &mut AIState,
        monster: &MonsterData,
    ) {
        // 如果没有巡逻点，随机选择一个
        if ai_state.patrol_points.is_empty() {
            // 在出生点周围生成4个巡逻点
            let spawn_x = monster.spawn_x;
            let spawn_y = monster.spawn_y;
            
            ai_state.patrol_points = vec![
                (spawn_x + 5.0, spawn_y),
                (spawn_x, spawn_y + 5.0),
                (spawn_x - 5.0, spawn_y),
                (spawn_x, spawn_y - 5.0),
            ];
            ai_state.current_patrol_index = 0;
        }
        
        // 获取当前目标巡逻点
        if let Some(&target) = ai_state.patrol_points.get(ai_state.current_patrol_index) {
            let distance = Self::distance((pos.x, pos.y), target);
            
            if distance < 0.5 {
                // 到达巡逻点，切换到下一个
                ai_state.current_patrol_index = (ai_state.current_patrol_index + 1) % ai_state.patrol_points.len();
                ai_state.current_action = crate::ecs::components::AIAction::Idle;
            } else {
                // 继续前往巡逻点
                ai_state.current_action = crate::ecs::components::AIAction::Patrol;
                ai_state.target_pos = Some(target);
            }
        }
    }
    
    /// 查找玩家位置
    fn find_player_position(world: &World) -> Option<(f32, f32)> {
        for (_entity, (_, pos)) in world.query::<(&Player, &Position)>().iter() {
            return Some((pos.x, pos.y));
        }
        None
    }
    
    /// 计算两点之间的距离
    fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        (dx * dx + dy * dy).sqrt()
    }
}

impl System for MonsterAISystem {

    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::MONSTER_AI
    }

    fn update(&mut self,  ctx:&mut GameContext, _delay_time: f32) -> GameResult {
        Self::update_ai(&mut ctx.world);
        Ok(())
    }
}
