use crate::{
    components::{LocalPlayer, MirDirection, MovementMode, Player, PlayerInput, Position},
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
        let (_e, s) = q.iter().next()?;
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
            // 说明：PlayerAction 由 PlayerControlSystem 统一写入。
            // 小地图双击产生的“run”意图通过 PlayerInput.run 传递。
            let mut q = ctx.world.query::<(&LocalPlayer, &mut PlayerInput)>();
            if let Some((_entity, (_local, input))) = q.iter().next() {
                input.move_to = Some((wx, wy));
                input.movement_mode = MovementMode::Pathfinding;
                input.run = run;
            }
        }

        // 2) ECS -> UI：同步玩家点到小地图
        let player_snapshot = {
            let mut q = ctx.world.query::<(&LocalPlayer, &Position, &Player)>();
            q.iter()
                .next()
                .map(|(_entity, (_local, pos, player))| (pos.x, pos.y, player.direction))
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
