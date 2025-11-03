use crate::ecs::components::{Camera, InputEvent, Position, RenderConfig};
use crate::ecs::{GameContext, HybridSystem, System};
use ggez::input::keyboard::KeyCode;
use ggez::GameResult;

/// DebugSystemV2 - 使用 GameContext 的调试系统
///
/// **改进点**:
/// - ✅ 使用 GameContext 零拷贝访问 ggez Context (用于渲染)
/// - ✅ 从 GlobalEvents 读取 KeyDown 事件 (仍需边缘检测)
///
/// **职责**:
/// - 检测调试快捷键（图层切换、网格、边框等）
/// - 修改 RenderConfig 组件
/// - 渲染调试信息（FPS、实体数量等）
pub struct DebugSystem;

impl DebugSystem {
    pub fn new() -> Self {
        Self
    }

    /// 处理调试相关的键盘按键
    fn handle_keycode(world: &mut hecs::World, keycode: KeyCode) {
        // M键需要特殊处理，因为它会借用整个 world
        if keycode == KeyCode::KeyM {
            tracing::info!("🗺️ 打开地图选择对话框");
            use crate::ecs::systems::logic::update::MapUpdateSystem;
            MapUpdateSystem::trigger_map_selection(world);
            return;
        }

        // 查找 RenderConfig 组件
        let mut config_query = world.query::<&mut RenderConfig>();
        let config = if let Some((_, cfg)) = config_query.iter().next() {
            cfg
        } else {
            return;
        };

        match keycode {
            // 图层切换 (与 C# 版本一致: 1=Front, 2=Middle, 3=Back)
            KeyCode::Digit1 => {
                config.show_front = !config.show_front;
                tracing::info!(
                    "🖼️ Front 层: {}",
                    if config.show_front {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::Digit2 => {
                config.show_middle = !config.show_middle;
                tracing::info!(
                    "🖼️ Middle 层: {}",
                    if config.show_middle {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::Digit3 => {
                config.show_back = !config.show_back;
                tracing::info!(
                    "🖼️ Back 层: {}",
                    if config.show_back { "显示" } else { "隐藏" }
                );
            }

            // 调试显示选项
            KeyCode::KeyG => {
                config.show_grid = !config.show_grid;
                tracing::info!(
                    "🔲 网格: {}",
                    if config.show_grid { "显示" } else { "隐藏" }
                );
            }
            KeyCode::KeyO => {
                config.show_obstacles = !config.show_obstacles;
                tracing::info!(
                    "🚧 障碍物: {}",
                    if config.show_obstacles {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::KeyB => {
                config.show_borders = !config.show_borders;
                tracing::info!(
                    "📦 边框: {}",
                    if config.show_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::KeyP => {
                config.show_path = !config.show_path;
                tracing::info!(
                    "🛤️ 路径: {}",
                    if config.show_path { "显示" } else { "隐藏" }
                );
            }
            KeyCode::KeyA => {
                config.show_animations = !config.show_animations;
                tracing::info!(
                    "🎬 动画: {}",
                    if config.show_animations {
                        "播放"
                    } else {
                        "暂停"
                    }
                );
            }
            KeyCode::KeyS => {
                config.show_static_tiles = !config.show_static_tiles;
                tracing::info!(
                    "🗿 静态瓦片: {}",
                    if config.show_static_tiles {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::KeyD => {
                config.show_animated_tiles = !config.show_animated_tiles;
                tracing::info!(
                    "🎞️ 动画瓦片: {}",
                    if config.show_animated_tiles {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::KeyL => {
                config.enable_lod = !config.enable_lod;
                tracing::info!(
                    "🔍 LOD: {}",
                    if config.enable_lod {
                        "启用"
                    } else {
                        "禁用"
                    }
                );
            }

            // F键 - 边框显示
            KeyCode::F9 => {
                config.show_monster_borders = !config.show_monster_borders;
                tracing::info!(
                    "👹 怪物边框: {}",
                    if config.show_monster_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::F10 => {
                config.show_npc_borders = !config.show_npc_borders;
                tracing::info!(
                    "👤 NPC边框: {}",
                    if config.show_npc_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
            KeyCode::F11 => {
                config.show_effect_borders = !config.show_effect_borders;
                tracing::info!(
                    "✨ 特效边框: {}",
                    if config.show_effect_borders {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }

            // FPS 控制
            KeyCode::Equal | KeyCode::NumpadAdd => {
                config.max_fps = (config.max_fps + 10).min(300);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }

            KeyCode::Escape => {
                tracing::info!("👋 用户按下 ESC，应该退出程序（需要在 App 层处理）");
            }

            _ => {}
        }
    }

    /// 绘制网格线
    fn draw_grid(
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        camera: &Camera,
        camera_pos: &Position,
    ) -> GameResult {
        use crate::ecs::{CELL_HEIGHT, CELL_WIDTH};
        use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};

        let grid_color = Color::from_rgba(255, 255, 255, 80);

        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        let half_height = (camera.screen_height / 2.0) / camera.zoom;
        let view_left = camera_pos.x - half_width;
        let view_right = camera_pos.x + half_width;
        let view_top = camera_pos.y - half_height;
        let view_bottom = camera_pos.y + half_height;

        let start_x = (view_left / CELL_WIDTH as f32).floor() as i32;
        let end_x = (view_right / CELL_WIDTH as f32).ceil() as i32;
        let start_y = (view_top / CELL_HEIGHT as f32).floor() as i32;
        let end_y = (view_bottom / CELL_HEIGHT as f32).ceil() as i32;

        // 绘制垂直线
        for x in start_x..=end_x {
            let world_x = (x * CELL_WIDTH) as f32;
            let screen_x = (world_x - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
            let top_y = (view_top - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;
            let bottom_y = (view_bottom - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;

            let line = Mesh::new_line(
                ctx,
                &[[screen_x, top_y], [screen_x, bottom_y]],
                1.0,
                grid_color,
            )?;
            canvas.draw(&line, DrawParam::default());
        }

        // 绘制水平线
        for y in start_y..=end_y {
            let world_y = (y * CELL_HEIGHT) as f32;
            let screen_y = (world_y - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;
            let left_x = (view_left - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
            let right_x = (view_right - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;

            let line = Mesh::new_line(
                ctx,
                &[[left_x, screen_y], [right_x, screen_y]],
                1.0,
                grid_color,
            )?;
            canvas.draw(&line, DrawParam::default());
        }

        Ok(())
    }

    /// 绘制瓦片边框
    fn draw_tile_borders(
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
        camera: &Camera,
        camera_pos: &Position,
        config: &RenderConfig,
    ) -> GameResult {
        use crate::ecs::components::{MapTile, TileLayer};
        use crate::ecs::{CELL_HEIGHT, CELL_WIDTH};
        use crate::graphics::get_map_library;
        use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};

        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        let half_height = (camera.screen_height / 2.0) / camera.zoom;
        let view_left = camera_pos.x - half_width;
        let view_right = camera_pos.x + half_width;
        let view_top = camera_pos.y - half_height;
        let view_bottom = camera_pos.y + half_height;

        for (_, tile) in world.query::<&MapTile>().iter() {
            match tile.layer {
                TileLayer::Back if !config.show_back => continue,
                TileLayer::Middle if !config.show_middle => continue,
                TileLayer::Front if !config.show_front => continue,
                _ => {}
            }

            let world_x = (tile.grid_x * CELL_WIDTH) as f32;
            let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

            let bottom_extra = if matches!(tile.layer, TileLayer::Front) {
                800.0
            } else {
                200.0
            };
            if world_x > view_right + 200.0
                || world_x < view_left - 200.0
                || world_y > view_bottom + bottom_extra
                || world_y < view_top - 200.0
            {
                continue;
            }

            if let Some(lib) = get_map_library(tile.library_index) {
                if let Ok(mut lib_guard) = lib.lock() {
                    let (tile_w, tile_h) = lib_guard
                        .get_size(tile.image_index as usize)
                        .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));

                    let adjusted_y = world_y - tile_h as f32;
                    let screen_x =
                        (world_x - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
                    let screen_y =
                        (adjusted_y - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;

                    let border_color = match tile.layer {
                        TileLayer::Back => Color::from_rgb(255, 0, 0),
                        TileLayer::Middle => Color::from_rgb(0, 255, 0),
                        TileLayer::Front => Color::from_rgb(0, 150, 255),
                    };

                    let border_rect = ggez::graphics::Rect::new(
                        screen_x,
                        screen_y,
                        tile_w as f32 * camera.zoom,
                        tile_h as f32 * camera.zoom,
                    );

                    let border_mesh =
                        Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), border_rect, border_color)?;
                    canvas.draw(&border_mesh, DrawParam::default());
                }
            }
        }

        Ok(())
    }

    /// 绘制障碍物格子
    fn draw_obstacles(
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
        camera: &Camera,
        camera_pos: &Position,
    ) -> GameResult {
        use crate::ecs::components::MapData;
        use crate::ecs::{CELL_HEIGHT, CELL_WIDTH};
        use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};

        let obstacle_color = Color::from_rgba(180, 0, 0, 120);

        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        let half_height = (camera.screen_height / 2.0) / camera.zoom;
        let view_left = camera_pos.x - half_width;
        let view_right = camera_pos.x + half_width;
        let view_top = camera_pos.y - half_height;
        let view_bottom = camera_pos.y + half_height;

        let start_x = (view_left / CELL_WIDTH as f32).floor() as i32;
        let end_x = (view_right / CELL_WIDTH as f32).ceil() as i32;
        let start_y = (view_top / CELL_HEIGHT as f32).floor() as i32;
        let end_y = (view_bottom / CELL_HEIGHT as f32).ceil() as i32;

        for (_, map) in world.query::<&MapData>().iter() {
            for grid_y in start_y..=end_y {
                for grid_x in start_x..=end_x {
                    if grid_x < 0 || grid_y < 0 || grid_x >= map.width || grid_y >= map.height {
                        continue;
                    }

                    if grid_x as usize >= map.cells.len() {
                        continue;
                    }
                    if grid_y as usize >= map.cells[grid_x as usize].len() {
                        continue;
                    }

                    let cell = &map.cells[grid_x as usize][grid_y as usize];
                    let has_obstacle = (cell.back_image & 0x20000000) != 0;

                    if has_obstacle {
                        let world_x = (grid_x * CELL_WIDTH) as f32;
                        let world_y = (grid_y * CELL_HEIGHT) as f32;
                        let screen_x =
                            (world_x - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
                        let screen_y =
                            (world_y - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;

                        let rect = ggez::graphics::Rect::new(
                            screen_x,
                            screen_y,
                            CELL_WIDTH as f32 * camera.zoom,
                            CELL_HEIGHT as f32 * camera.zoom,
                        );

                        let mesh =
                            Mesh::new_rectangle(ctx, DrawMode::fill(), rect, obstacle_color)?;
                        canvas.draw(&mesh, DrawParam::default());
                    }
                }
            }
        }

        Ok(())
    }

    /// 绘制路径
    fn draw_paths(
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
        camera: &Camera,
        camera_pos: &Position,
    ) -> GameResult {
        use crate::ecs::components::{LocalPlayer, Path, Position as EntityPosition};
        use crate::ecs::{CELL_HEIGHT, CELL_WIDTH};
        use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};

        for (_, (_, path, pos)) in world
            .query::<(&LocalPlayer, &Path, &EntityPosition)>()
            .iter()
        {
            if !path.is_valid || path.waypoints.is_empty() {
                continue;
            }

            let world_to_screen = |wx: f32, wy: f32| -> [f32; 2] {
                let screen_x = (wx - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
                let screen_y = (wy - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;
                [screen_x, screen_y]
            };

            let grid_to_world = |gx: i32, gy: i32| -> (f32, f32) {
                let wx = (gx * CELL_WIDTH) as f32 + CELL_WIDTH as f32 / 2.0;
                let wy = (gy * CELL_HEIGHT) as f32 + CELL_HEIGHT as f32 / 2.0;
                (wx, wy)
            };

            // 从当前位置到第一个路径点
            if let Some(&first_waypoint) = path.waypoints.first() {
                let (target_wx, target_wy) = grid_to_world(first_waypoint.0, first_waypoint.1);
                let start_screen = world_to_screen(pos.x, pos.y);
                let end_screen = world_to_screen(target_wx, target_wy);

                let line = Mesh::new_line(
                    ctx,
                    &[start_screen, end_screen],
                    2.0 * camera.zoom,
                    Color::from_rgb(255, 255, 0),
                )?;
                canvas.draw(&line, DrawParam::default());
            }

            // 路径点之间的线段
            for i in 0..path.waypoints.len().saturating_sub(1) {
                let (wx1, wy1) = grid_to_world(path.waypoints[i].0, path.waypoints[i].1);
                let (wx2, wy2) = grid_to_world(path.waypoints[i + 1].0, path.waypoints[i + 1].1);

                let start_screen = world_to_screen(wx1, wy1);
                let end_screen = world_to_screen(wx2, wy2);

                let line = Mesh::new_line(
                    ctx,
                    &[start_screen, end_screen],
                    2.0 * camera.zoom,
                    Color::from_rgb(0, 255, 255),
                )?;
                canvas.draw(&line, DrawParam::default());
            }

            // 路径点标记
            for (i, &(gx, gy)) in path.waypoints.iter().enumerate() {
                let (wx, wy) = grid_to_world(gx, gy);
                let screen_pos = world_to_screen(wx, wy);

                let (color, radius) = if i == 0 {
                    (Color::from_rgb(255, 0, 0), 6.0 * camera.zoom)
                } else {
                    (Color::from_rgb(255, 255, 0), 3.0 * camera.zoom)
                };

                let circle =
                    Mesh::new_circle(ctx, DrawMode::fill(), [0.0, 0.0], radius, 0.5, color)?;
                canvas.draw(&circle, DrawParam::default().dest(screen_pos));
            }
        }

        Ok(())
    }
}

impl HybridSystem for DebugSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // ✅ 使用 InputContext API 检查按键状态
        // 注意：由于键盘状态检查是即时的，这里只处理当前帧按下的键
        // 如果需要边缘检测（按下瞬间），需要在 InputContext 中维护上一帧状态

        // 检查调试快捷键
        let keys_to_check = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::KeyG,
            KeyCode::KeyO,
            KeyCode::KeyB,
            KeyCode::KeyP,
            KeyCode::KeyA,
            KeyCode::KeyS,
            KeyCode::KeyD,
            KeyCode::KeyL,
            KeyCode::KeyM,
            KeyCode::F9,
            KeyCode::F10,
            KeyCode::F11,
            KeyCode::Equal,
            KeyCode::NumpadAdd,
            KeyCode::Minus,
            KeyCode::NumpadSubtract,
            KeyCode::Escape,
        ];

        for &key in &keys_to_check {
            if ctx.input().key_pressed(key) {
                Self::handle_keycode(ctx.world, key);
            }
        }

        Ok(())
    }

    fn priority(&self) -> u32 {
        u32::MAX - 1
    }

    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult {
        use crate::ecs::components::TimeTracker;
        use ggez::graphics::{Color, DrawParam, Text, TextFragment};

        // 读取调试信息
        let mut fps = 0.0;
        let mut camera_pos = (0.0, 0.0);
        let mut camera_zoom = 1.0;
        let entity_count = world.len();

        for (_, time) in world.query::<&TimeTracker>().iter() {
            fps = time.fps;
            break;
        }

        for (_, (camera, pos)) in world.query::<(&Camera, &Position)>().iter() {
            camera_pos = (pos.x, pos.y);
            camera_zoom = camera.zoom;
            break;
        }

        // 读取图层显示状态
        let mut layer_status = String::from("Layers: ");
        let mut show_back = true;
        let mut show_middle = true;
        let mut show_front = true;
        let mut show_obstacles = false;
        let mut show_grid = false;
        let mut show_borders = false;

        for (_, config) in world.query::<&RenderConfig>().iter() {
            show_back = config.show_back;
            show_middle = config.show_middle;
            show_front = config.show_front;
            show_obstacles = config.show_obstacles;
            show_grid = config.show_grid;
            show_borders = config.show_borders;

            let mut layers = Vec::new();
            if show_front {
                layers.push("F");
            } else {
                layers.push("f");
            }
            if show_middle {
                layers.push("M");
            } else {
                layers.push("m");
            }
            if show_back {
                layers.push("B");
            } else {
                layers.push("b");
            }
            layer_status = format!("Layers: {} (1/2/3)", layers.join(" "));
            break;
        }

        // 构建调试选项状态
        let mut debug_options = Vec::new();
        if show_grid {
            debug_options.push("Grid");
        }
        if show_borders {
            debug_options.push("Border");
        }
        if show_obstacles {
            debug_options.push("Obstacle");
        }
        let debug_status = if debug_options.is_empty() {
            String::from("Debug: None")
        } else {
            format!("Debug: {}", debug_options.join(", "))
        };

        // 构建调试文本
        let debug_text = format!(
            "FPS: {:.1}\nCamera: ({:.0}, {:.0})\nZoom: {:.2}x\nEntities: {}\n{}\n{}",
            fps, camera_pos.0, camera_pos.1, camera_zoom, entity_count, layer_status, debug_status
        );

        let text = Text::new(TextFragment {
            text: debug_text,
            color: Some(Color::WHITE),
            font: None,
            scale: Some(ggez::graphics::PxScale::from(20.0)),
        });

        // 绘制背景
        let bg_rect = ggez::graphics::Rect::new(5.0, 5.0, 350.0, 160.0);
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());

        // 绘制文本
        canvas.draw(&text, DrawParam::default().dest([10.0, 10.0]));

        // 获取 RenderConfig 和 Camera
        let config = world
            .query::<&RenderConfig>()
            .iter()
            .next()
            .map(|(_, cfg)| cfg.clone());
        if let Some(config) = config {
            if let Some((camera, pos)) = world
                .query::<(&Camera, &Position)>()
                .iter()
                .next()
                .map(|(_, (c, p))| (c.clone(), p.clone()))
            {
                if config.show_grid {
                    Self::draw_grid(ctx, canvas, &camera, &pos)?;
                }

                if config.show_borders {
                    Self::draw_tile_borders(ctx, canvas, world, &camera, &pos, &config)?;
                }

                if config.show_obstacles {
                    Self::draw_obstacles(ctx, canvas, world, &camera, &pos)?;
                }

                if config.show_path {
                    Self::draw_paths(ctx, canvas, world, &camera, &pos)?;
                }
            }
        }

        Ok(())
    }
}
