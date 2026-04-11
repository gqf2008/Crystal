// ============================================================================
// 测试 ECS 调度器 draw 阶段 - 最小可运行验证
//
// 目标：
// - 复用最小 ECS 世界（相机 + 本地玩家）
// - 使用 SystemScheduler 同时执行 update + draw
// - 通过一个最小 RenderSystem 在 draw 阶段绘制文本/点位，验证 draw 确实被调用
//
// 不影响现有 GameScene 主链路。
// ============================================================================

// Windows: Release 模式不弹控制台（Debug 仍保留控制台便于调试）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;

use client_macroquad::components::{
    movement::MovementVelocity,
    Camera,
    CameraMode,
    LocalPlayer,
    Path,
    Player,
    PlayerAction,
    PlayerInput,
    Position,
};
use client_macroquad::game::{GameContext, GameResult};
use client_macroquad::systems::{
    priority,
    CameraFollowSystem,
    IntoSystemKind,
    MovementSystem,
    PathfindingSystem,
    PlayerControlSystem,
    RenderSystem,
    SystemKind,
    SystemScheduler,
};
use mir2_shared::enums::MirDirection;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

fn window_conf() -> Conf {
    Conf {
        window_title: "Client-Macroquad - ECS Render(draw) 最小入口测试".to_string(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: true,
        high_dpi: false,
        fullscreen: false,
        platform: Platform {
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn world_to_screen(world_x: f32, world_y: f32, camera_pos: &Position, camera: &Camera) -> (f32, f32) {
    let sx = (world_x - camera_pos.x) / camera.zoom + camera.screen_width / 2.0;
    let sy = (world_y - camera_pos.y) / camera.zoom + camera.screen_height / 2.0;
    (sx, sy)
}

struct DrawProbe {
    frames: u64,
    last_dt: f32,
}

impl DrawProbe {
    fn new() -> Self {
        Self {
            frames: 0,
            last_dt: 0.0,
        }
    }
}

impl RenderSystem for DrawProbe {
    fn update(&mut self, _ctx: &mut GameContext, dt: f32) -> GameResult {
        self.frames = self.frames.wrapping_add(1);
        self.last_dt = dt;
        Ok(())
    }

    fn draw(&mut self, world: &hecs::World) -> GameResult {
        // 读取 camera
        let (camera, camera_pos) = world
            .query::<(&Camera, &Position)>()
            .iter()
            .next()
            .map(|(c, p)| (c.clone(), *p))
            .unwrap_or((Camera::new(screen_width(), screen_height()), Position::new(0.0, 0.0)));

        // 读取 player
        let player_snapshot = world
            .query::<(&LocalPlayer, &Position, &MovementVelocity, &PlayerInput, &Player)>()
            .iter()
            .next()
            .map(|(_lp, pos, vel, input, player)| (*pos, vel.clone(), input.clone(), player.clone()));

        draw_text(
            "ECS Render(draw) MIN TEST | Scheduler.draw is running | ESC quit",
            16.0,
            24.0,
            20.0,
            WHITE,
        );

        draw_text(
            &format!("frames={} dt={:.3} zoom={:.2}", self.frames, self.last_dt, camera.zoom),
            16.0,
            48.0,
            18.0,
            GRAY,
        );

        if let Some((pos, vel, input, player)) = player_snapshot {
            let (sx, sy) = world_to_screen(pos.x, pos.y, &camera_pos, &camera);

            draw_circle(sx, sy, 6.0, YELLOW);
            draw_circle_lines(sx, sy, 10.0, 2.0, ORANGE);

            draw_text(
                &format!(
                    "pos=({:.1},{:.1}) vel=({:.1},{:.1}) mode={:?} move_to={:?} action={:?} dir={:?} cam=({:.1},{:.1})",
                    pos.x,
                    pos.y,
                    vel.x,
                    vel.y,
                    input.movement_mode,
                    input.move_to,
                    player.action,
                    player.direction,
                    camera_pos.x,
                    camera_pos.y
                ),
                16.0,
                74.0,
                16.0,
                WHITE,
            );
        } else {
            draw_text("❌ 未找到 LocalPlayer", 16.0, 74.0, 16.0, RED);
        }

        Ok(())
    }
}

impl IntoSystemKind for DrawProbe {
    fn into_kind(self: Box<Self>) -> SystemKind {
        SystemKind::Render(self)
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🧪 ECS Render(draw) 最小入口测试");

    let mut ctx = GameContext::new();

    // ==== Spawn Camera ====
    let (sw, sh) = (screen_width(), screen_height());
    ctx.world.spawn((
        Camera::new(sw, sh),
        Position::new(0.0, 0.0),
        CameraMode::FollowPlayer,
    ));

    // ==== Spawn Local Player ====
    ctx.world.spawn((
        LocalPlayer,
        Position::new(0.0, 0.0),
        Player {
            direction: MirDirection::Down,
            action: PlayerAction::Stand,
        },
        PlayerInput::default(),
        Path::new(),
        MovementVelocity::new(300.0),
    ));

    // ==== Scheduler ====
    let mut scheduler = SystemScheduler::new();
    scheduler
        .add_system(PlayerControlSystem::new(), priority::PLAYER_CONTROL)
        .add_system(PathfindingSystem::new(), priority::PATHFINDING)
        .add_system(MovementSystem, priority::MOVEMENT)
        .add_system(CameraFollowSystem, priority::CAMERA_FOLLOW)
        .add_system(DrawProbe::new(), priority::DEBUG);

    loop {
        let dt = get_frame_time();
        ctx.delta_time = dt;

        // 同步 camera 的屏幕尺寸（PlayerControlSystem/渲染换算都依赖它）
        let (sw, sh) = (screen_width(), screen_height());
        for cam in ctx.world.query_mut::<&mut Camera>() {
            cam.screen_width = sw;
            cam.screen_height = sh;
        }

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        if let Err(e) = scheduler.update(&mut ctx, dt) {
            eprintln!("❌ scheduler.update 失败: {e}");
        }

        clear_background(BLACK);
        if let Err(e) = scheduler.draw(&ctx.world) {
            eprintln!("❌ scheduler.draw 失败: {e}");
        }

        next_frame().await;
    }
}
