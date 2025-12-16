// ============================================================================
// 测试最小 ECS 调度入口 - 验证 systems/ 能实际跑起来
//
// 目标：
// - 创建 GameContext + hecs::World
// - Spawn: Camera(+Position/Mode) + LocalPlayer(含 Position/PlayerInput/Path/Velocity)
// - SystemScheduler: PlayerControl -> Pathfinding -> Movement -> CameraFollow
// - 每帧执行 scheduler.update，并在屏幕上显示玩家坐标/速度
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
use client_macroquad::game::GameContext;
use client_macroquad::systems::{
    priority,
    CameraFollowSystem,
    MovementSystem,
    PathfindingSystem,
    PlayerControlSystem,
    SystemScheduler,
};
use mir2_shared::enums::MirDirection;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

fn window_conf() -> Conf {
    Conf {
        window_title: "Client-Macroquad - ECS 最小入口测试".to_string(),
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

#[macroquad::main(window_conf)]
async fn main() {
    println!("🧪 ECS 最小入口测试：PlayerControl -> Pathfinding -> Movement");
    println!("- 鼠标按住：DirectFollow（直线跟随）");
    println!("- 鼠标双击：Pathfinding（无 MapData 时会退化为直线目标格）");

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

    // 输入 -> 寻路/速度 -> 移动 -> 相机跟随
    scheduler
        .add_system(PlayerControlSystem::new(), priority::PLAYER_CONTROL)
        .add_system(PathfindingSystem::new(), priority::PATHFINDING)
        .add_system(MovementSystem, priority::MOVEMENT)
        .add_system(CameraFollowSystem, priority::CAMERA_FOLLOW);

    loop {
        let dt = get_frame_time();
        ctx.delta_time = dt;

        // 同步 camera 的屏幕尺寸（PlayerControlSystem 的 screen_to_world 依赖它）
        let (sw, sh) = (screen_width(), screen_height());
        for (_, cam) in ctx.world.query_mut::<&mut Camera>() {
            cam.screen_width = sw;
            cam.screen_height = sh;
        }

        if let Err(e) = scheduler.update(&mut ctx, dt) {
            // 没有 tracing-subscriber 时，println 至少能看到错误
            eprintln!("❌ scheduler.update 失败: {e}");
        }

        // ==== Debug Draw ====
        clear_background(BLACK);

        // 读取 camera
        let camera_snapshot = ctx
            .world
            .query::<(&Camera, &Position)>()
            .iter()
            .next()
            .map(|(_, (c, p))| (c.clone(), *p))
            .unwrap_or((Camera::new(sw, sh), Position::new(0.0, 0.0)));

        // 读取 player
        let mut player_info = None;
        for (_, (pos, vel, input, player, _local)) in ctx.world.query::<(
            &Position,
            &MovementVelocity,
            &PlayerInput,
            &Player,
            &LocalPlayer,
        )>()
        .iter()
        {
            player_info = Some((*pos, vel.clone(), input.clone(), player.clone()));
            break;
        }

        let (camera, camera_pos) = camera_snapshot;

        draw_text(
            "ECS MIN TEST  |  Hold mouse = follow  |  Double click = path (fallback)  |  ESC quit",
            16.0,
            24.0,
            20.0,
            WHITE,
        );

        if let Some((pos, vel, input, player)) = player_info {
            let (px, py) = world_to_screen(pos.x, pos.y, &camera_pos, &camera);

            // 画玩家点
            draw_circle(px, py, 6.0, YELLOW);
            draw_circle_lines(px, py, 10.0, 2.0, ORANGE);

            // 文本信息
            let info = format!(
                "pos=({:.1},{:.1})  vel=({:.1},{:.1})  mode={:?}  move_to={:?}  action={:?}  dir={:?}  cam=({:.1},{:.1})",
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
            );
            draw_text(&info, 16.0, 52.0, 18.0, WHITE);
        } else {
            draw_text("❌ 未找到 LocalPlayer 实体", 16.0, 52.0, 18.0, RED);
        }

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }
}
