use macroquad::prelude::get_time;

use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;

/// 远程玩家移动动作的“回站立”系统。
///
/// - NetworkApplySystem 收到 ObjectWalk/ObjectRun 后，会给远程玩家挂上 RemoteMoveAnim(end_time)
/// - 当插值结束（PositionInterpolation 被移除）且到达 end_time，且不在攻击/死亡状态时，将动作恢复为 Stand
#[derive(Default, ecs_macros::LogicSystem)]
pub struct RemoteMoveAnimSystem;

impl LogicSystem for RemoteMoveAnimSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        use crate::components::{AttackState, DeathState, Player, PlayerAction, PositionInterpolation, RemoteMoveAnim};

        let now = get_time();
        let mut done: Vec<hecs::Entity> = Vec::new();

        for eref in ctx.world.iter() {
            let (Some(mut player), Some(timer), interp, atk, dead) = (
                eref.get::<&mut Player>(),
                eref.get::<&RemoteMoveAnim>(),
                eref.get::<&PositionInterpolation>(),
                eref.get::<&AttackState>(),
                eref.get::<&DeathState>(),
            ) else {
                continue;
            };
            // 有插值说明仍在移动；攻击/死亡由其他系统驱动，不在这里抢写动作。
            if interp.is_some() || atk.is_some() || dead.is_some() {
                continue;
            }

            if now < timer.end_time {
                continue;
            }

            if matches!(player.action, PlayerAction::Walk | PlayerAction::Run) {
                player.action = PlayerAction::Stand;
            }
            done.push(eref.entity());
        }

        for e in done {
            let _ = ctx.world.remove_one::<RemoteMoveAnim>(e);
        }

        Ok(())
    }
}
