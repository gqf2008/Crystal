use crate::{
    components::{LocalPlayer, MirDirection, MovementMode, Player, PlayerAction, PlayerInput, Position},
    game::{GameContext, GameResult},
    systems::LogicSystem,
    ui::ui_state::UiState,
};

#[derive(ecs_macros::LogicSystem)]
pub struct MinimapSystem {
}

impl MinimapSystem {
    pub fn new() -> Self {
        Self {}
    }

    fn with_ui_state_mut<R>(
        ctx: &mut GameContext,
        f: impl FnOnce(&mut crate::ui::ui_state::UiStateData) -> R,
    ) -> Option<R> {
        let mut q = ctx.world.query::<&UiState>();
        let s = q.iter().next()?;
        let mut data = s.borrow_mut();
        Some(f(&mut data))
    }

    fn mir_direction_to_radians(dir: MirDirection) -> f32 {
        use std::f32::consts::{FRAC_PI_2, PI};
        match dir {
            MirDirection::Right => 0.0,
            MirDirection::DownRight => FRAC_PI_2 / 2.0,
            MirDirection::Down => FRAC_PI_2,
            MirDirection::DownLeft => FRAC_PI_2 + FRAC_PI_2 / 2.0,
            MirDirection::Left => PI,
            MirDirection::UpLeft => -PI + FRAC_PI_2 / 2.0,
            MirDirection::Up => -FRAC_PI_2,
            MirDirection::UpRight => -FRAC_PI_2 / 2.0,
        }
    }
}

impl LogicSystem for MinimapSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 1) UI -> ECS：点击小地图产生的自动寻路目标
        if let Some((wx, wy, run)) =
            Self::with_ui_state_mut(ctx, |ui| ui.pending_auto_path_target.take()).flatten()
        {
            // 模式互斥：挂机/AT/BT 控制开启时，忽略小地图的手动寻路命令。
            if ctx.session.local_player_ai_enabled {
                // 仍继续后续“ECS -> UI”同步，避免点击时小地图状态丢一帧。
            } else {
                // 小地图双击是 UI 事件，不一定会触发 PlayerControlSystem 的鼠标分支。
                // 若这里不设置 Player.action，可能出现“位置在动但动画不播放”的平移效果。
                // 这里直接把 walk/run 意图落地到 PlayerInput + Player.action（攻击中则不覆盖）。
                if let Some((entity, _local, mut input, mut player)) = ctx.world.iter().find_map(|e| {
                    let _local = e.get::<&LocalPlayer>()?;
                    let input = e.get::<&mut PlayerInput>()?;
                    let player = e.get::<&mut Player>()?;
                    Some((e.entity(), _local, input, player))
                }) {
                    input.move_to = Some((wx, wy));
                    input.movement_mode = MovementMode::Pathfinding;
                    input.run = run;

                    let is_attacking = ctx.world.get::<&crate::components::AttackState>(entity).is_ok();
                    if !is_attacking && !player.action.is_attack() {
                        player.action = if run { PlayerAction::Run } else { PlayerAction::Walk };
                    }
                }
            }
        }

        // 2) ECS -> UI：同步玩家点到小地图
        let player_snapshot = {
            let mut q = ctx.world.query::<(&LocalPlayer, &Position, &Player)>();
            q.iter()
                .next()
                .map(|(_local, pos, player)| (pos.x, pos.y, player.direction))
        };

        if let Some((x, y, dir)) = player_snapshot {
            let dir_rad = Self::mir_direction_to_radians(dir);
            let _ = Self::with_ui_state_mut(ctx, |ui| {
                ui.minimap_player_pos = Some(macroquad::prelude::vec2(x, y));
                ui.minimap_player_dir_radians = dir_rad;
            });
        }

        Ok(())
    }
}
