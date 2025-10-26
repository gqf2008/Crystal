use super::RenderSystem;
use crate::ecs::components::{Camera, Position};
use ggez::{
    graphics::{Canvas, Color, DrawParam},
    Context, GameResult,
};
use hecs::World;

impl RenderSystem {
    /// 绘制寻路路径 (调试用)
    pub fn draw_path(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        camera_pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::systems::CameraSystem;
        use crate::ecs::{Coordinates, Player};
        use ggez::graphics;

        // 查询玩家的路径信息
        for (_entity, (player, player_pos)) in world.query::<(&Player, &Position)>().iter() {
            if player.path.is_empty() {
                continue;
            }

            // 绘制从当前位置到第一个路径点的线段
            if let Some(&(first_x, first_y)) = player.path.get(player.path_index) {
                // 第一个路径点的世界坐标
                let (first_world_x, first_world_y) =
                    Coordinates::grid_to_world_center(first_x, first_y);

                // 转换到屏幕坐标
                let (player_screen_x, player_screen_y) =
                    CameraSystem::world_to_screen(camera_pos, camera, player_pos.x, player_pos.y);
                let (first_screen_x, first_screen_y) =
                    CameraSystem::world_to_screen(camera_pos, camera, first_world_x, first_world_y);

                // 🎯 绘制连接线前,检查坐标是否合理
                // 即使超出屏幕也绘制,但避免极端值导致的渲染问题
                if player_screen_x.is_finite()
                    && player_screen_y.is_finite()
                    && first_screen_x.is_finite()
                    && first_screen_y.is_finite()
                {
                    // 绘制连接线 (黄色)
                    let line = graphics::Mesh::new_line(
                        ctx,
                        &[
                            [player_screen_x, player_screen_y],
                            [first_screen_x, first_screen_y],
                        ],
                        2.0,
                        Color::from_rgb(255, 255, 0), // 黄色
                    )?;
                    canvas.draw(&line, DrawParam::default());
                }
            }

            // 绘制路径点之间的连接线
            for i in player.path_index..(player.path.len() - 1) {
                let (x1, y1) = player.path[i];
                let (x2, y2) = player.path[i + 1];

                // 转换到世界坐标
                let (world_x1, world_y1) = Coordinates::grid_to_world_center(x1, y1);
                let (world_x2, world_y2) = Coordinates::grid_to_world_center(x2, y2);

                // 转换到屏幕坐标
                let (screen_x1, screen_y1) =
                    CameraSystem::world_to_screen(camera_pos, camera, world_x1, world_y1);
                let (screen_x2, screen_y2) =
                    CameraSystem::world_to_screen(camera_pos, camera, world_x2, world_y2);

                // 🎯 检查坐标是否合理 (允许超出屏幕,但必须是有限值)
                if screen_x1.is_finite()
                    && screen_y1.is_finite()
                    && screen_x2.is_finite()
                    && screen_y2.is_finite()
                {
                    // 🎯 即使超出屏幕也绘制 (让GPU自己裁剪)
                    if let Ok(line) = graphics::Mesh::new_line(
                        ctx,
                        &[[screen_x1, screen_y1], [screen_x2, screen_y2]],
                        2.0,
                        Color::from_rgb(0, 255, 255), // 青色
                    ) {
                        canvas.draw(&line, DrawParam::default());
                    }
                }
            }

            // 绘制路径点标记 (小圆点)
            for (idx, &(x, y)) in player.path.iter().enumerate() {
                if idx < player.path_index {
                    continue; // 跳过已经走过的点
                }

                // 转换到世界坐标
                let (world_x, world_y) = Coordinates::grid_to_world_center(x, y);

                // 转换到屏幕坐标
                let (screen_x, screen_y) =
                    CameraSystem::world_to_screen(camera_pos, camera, world_x, world_y);

                // 🎯 只绘制有效坐标的路径点
                if screen_x.is_finite() && screen_y.is_finite() {
                    // 绘制圆点
                    // 当前目标点用更大的红色圆圈
                    let (radius, color) = if idx == player.path_index {
                        (6.0, Color::from_rgb(255, 0, 0)) // 红色,大圆
                    } else {
                        (3.0, Color::from_rgb(255, 255, 0)) // 黄色,小圆
                    };

                    // 使用圆形绘制路径点
                    if let Ok(circle) = graphics::Mesh::new_circle(
                        ctx,
                        graphics::DrawMode::fill(),
                        [screen_x, screen_y],
                        radius,
                        0.1,
                        color,
                    ) {
                        canvas.draw(&circle, DrawParam::default());
                    }
                }
            }
        }

        Ok(())
    }

    /// 绘制网格
    pub fn draw_grid(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::systems::CameraSystem;
        use crate::ecs::{MapData, CELL_HEIGHT, CELL_WIDTH};
        use ggez::graphics;

        // 获取地图尺寸
        let (map_width, map_height) = world
            .query::<&MapData>()
            .iter()
            .next()
            .map(|(_, data)| (data.width, data.height))
            .unwrap_or((100, 100));

        let left = pos.x + (0.0 - camera.screen_width / 2.0) / camera.zoom;
        let right = pos.x + (camera.screen_width - camera.screen_width / 2.0) / camera.zoom;
        let top = pos.y + (0.0 - camera.screen_height / 2.0) / camera.zoom;
        let bottom = pos.y + (camera.screen_height - camera.screen_height / 2.0) / camera.zoom;

        let start_x = ((left / CELL_WIDTH as f32).floor() as i32).max(0);
        let end_x = ((right / CELL_WIDTH as f32).ceil() as i32).min(map_width);
        let start_y = ((top / CELL_HEIGHT as f32).floor() as i32).max(0);
        let end_y = ((bottom / CELL_HEIGHT as f32).ceil() as i32).min(map_height);

        let grid_color = Color::from_rgba(0, 255, 0, 120);

        // 垂直线
        for x in start_x..=end_x {
            let world_x = (x * CELL_WIDTH) as f32;
            let (screen_x, _) = CameraSystem::world_to_screen(pos, camera, world_x, 0.0);

            if screen_x >= 0.0 && screen_x <= camera.screen_width {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[screen_x, 0.0], [screen_x, camera.screen_height]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 水平线
        for y in start_y..=end_y {
            let world_y = (y * CELL_HEIGHT) as f32;
            let (_, screen_y) = CameraSystem::world_to_screen(pos, camera, 0.0, world_y);

            if screen_y >= 0.0 && screen_y <= camera.screen_height {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[0.0, screen_y], [camera.screen_width, screen_y]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        Ok(())
    }

    /// 绘制障碍物
    pub fn draw_obstacles(
        ctx: &mut Context,
        canvas: &mut Canvas,
        world: &World,
        pos: &Position,
        camera: &Camera,
    ) -> GameResult<()> {
        use crate::ecs::systems::CameraSystem;
        use crate::ecs::{MapData, CELL_HEIGHT, CELL_WIDTH};
        use ggez::graphics::{self, Text, TextFragment};

        let map_data = world
            .query::<&MapData>()
            .iter()
            .next()
            .map(|(_, data)| data.clone());

        if let Some(map_data) = map_data {
            let left = pos.x + (0.0 - camera.screen_width / 2.0) / camera.zoom;
            let right = pos.x + (camera.screen_width - camera.screen_width / 2.0) / camera.zoom;
            let top = pos.y + (0.0 - camera.screen_height / 2.0) / camera.zoom;
            let bottom = pos.y + (camera.screen_height - camera.screen_height / 2.0) / camera.zoom;

            let start_x = ((left / CELL_WIDTH as f32).floor() as i32).max(0);
            let end_x = ((right / CELL_WIDTH as f32).ceil() as i32).min(map_data.width);
            let start_y = ((top / CELL_HEIGHT as f32).floor() as i32).max(0);
            let end_y = ((bottom / CELL_HEIGHT as f32).ceil() as i32).min(map_data.height);

            // 🔴 障碍物颜色（半透明红色，更明显）
            let obstacle_color = Color::from_rgba(255, 0, 0, 150);
            let text_color = Color::from_rgb(255, 255, 0);

            for y in start_y..end_y {
                for x in start_x..end_x {
                    if x >= 0 && x < map_data.width && y >= 0 && y < map_data.height {
                        let cell = &map_data.cells[x as usize][y as usize];

                        // 🎯 正确的障碍物判断：使用 back_image 的高位标记
                        let has_obstacle = (cell.back_image & 0x20000000) != 0;

                        if has_obstacle {
                            let world_x = (x * CELL_WIDTH) as f32;
                            let world_y = (y * CELL_HEIGHT) as f32;
                            let (screen_x, screen_y) =
                                CameraSystem::world_to_screen(pos, camera, world_x, world_y);

                            // 绘制障碍物方块
                            let rect = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::fill(),
                                graphics::Rect::new(
                                    screen_x,
                                    screen_y,
                                    CELL_WIDTH as f32 * camera.zoom,
                                    CELL_HEIGHT as f32 * camera.zoom,
                                ),
                                obstacle_color,
                            )?;
                            canvas.draw(&rect, DrawParam::default());

                            // 🔤 绘制障碍物标记文字（更大字体）
                            if camera.zoom > 0.5 {
                                // 只在放大时显示文字
                                let text = Text::new(
                                    TextFragment::new("X")
                                        .scale(24.0) // 加大字体
                                        .color(text_color),
                                );

                                let text_x =
                                    screen_x + (CELL_WIDTH as f32 * camera.zoom - 12.0) / 2.0;
                                let text_y =
                                    screen_y + (CELL_HEIGHT as f32 * camera.zoom - 24.0) / 2.0;

                                canvas.draw(&text, DrawParam::default().dest([text_x, text_y]));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
