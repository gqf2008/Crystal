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
use crate::game::GameContext;
use crate::components::{
    AIAction, AIState, Health, LocalPlayer, Monster, Position,
};
use crate::systems::LogicSystem;
use crate::game::GameResult;

/// 怪物AI系统
#[derive(ecs_macros::LogicSystem)]
pub struct MonsterAISystem;

impl MonsterAISystem {
    /// 更新怪物AI逻辑
    fn update_ai(world: &mut World) {
        // 首先查找玩家位置
        let player_pos = Self::find_player_position(world);

        // Use world.iter() to get entities for AIState check
        let missing_ai: Vec<hecs::Entity> = world.iter().filter_map(|eref| {
            if eref.get::<&Monster>().is_some() && eref.get::<&AIState>().is_none() {
                Some(eref.entity())
            } else {
                None
            }
        }).collect();

        for e in missing_ai {
            let _ = world.insert_one(e, AIState::default());
        }
        
        // 遍历所有怪物
        for (_monster, pos, ai_state, health) in
            world.query::<(&Monster, &mut Position, &mut AIState, &Health)>().iter() 
        {
            // 跳过死亡怪物
            if health.current <= 0 {
                continue;
            }

            // 最小可用：先按“近战追击/攻击”行为驱动，避免出现 AI 系统完全不生效。
            Self::ai_melee_attack(pos, ai_state, player_pos);
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
                ai_state.current_action = AIAction::Attack;
                ai_state.target_pos = Some((px, py));
            } else if distance < 10.0 {
                // 在视野范围内 - 追击
                ai_state.current_action = AIAction::Chase;
                ai_state.target_pos = Some((px, py));
            } else {
                // 超出视野 - 闲置
                ai_state.current_action = AIAction::Idle;
                ai_state.target_pos = None;
            }
        } else {
            // 找不到玩家 - 闲置
            ai_state.current_action = AIAction::Idle;
            ai_state.target_pos = None;
        }
    }
    
    /// 查找玩家位置
    fn find_player_position(world: &World) -> Option<(f32, f32)> {
        world
            .query::<(&LocalPlayer, &Position)>()
            .iter()
            .next()
            .map(|(_local, pos)| (pos.x, pos.y))
    }
    
    /// 计算两点之间的距离
    fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        (dx * dx + dy * dy).sqrt()
    }
}

impl LogicSystem for MonsterAISystem {

    

    fn update(&mut self,  ctx:&mut GameContext, _delay_time: f32) -> GameResult {
        Self::update_ai(&mut ctx.world);
        Ok(())
    }
}
