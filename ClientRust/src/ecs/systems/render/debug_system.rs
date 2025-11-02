use crate::ecs::components::{Camera, InputEvent, Position, RenderConfig};
use crate::ecs::HybridSystem;
use crate::ecs::WorldExt;
use ggez::input::keyboard::KeyCode;
use ggez::GameResult;

/// DebugSystem 是唯一的混合系统，既实现了 System 又实现了 DrawSystem
/// 
/// 职责：
/// - 从 GlobalEvents 读取键盘事件，处理调试相关的快捷键
/// - 修改 RenderConfig 组件（图层切换、网格、边框等）
/// - 渲染调试信息（FPS、实体数量等）
pub struct DebugSystem;

impl HybridSystem for DebugSystem {
    fn priority(&self) -> u32 {
       u32::MAX - 1
    }
    
    fn update(&mut self, world: &mut hecs::World, _delay_time: f32) -> GameResult {
        // 从 GlobalEvents 读取输入事件
        let input_events = {
            let global_events = world.global_events();
            global_events.input_events.clone()
        };

        // 处理键盘事件
        for event in input_events {
            if let InputEvent::KeyDown { keycode, .. } = event {
                Self::handle_keycode(world, keycode);
            }
        }

        Ok(())
    }
    
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult {
        use crate::ecs::components::{Camera, Position, TimeTracker};
        use ggez::graphics::{Color, DrawParam, Text, TextFragment};

        // 1. 读取调试信息
        let mut fps = 0.0;
        let mut camera_pos = (0.0, 0.0);
        let mut camera_zoom = 1.0;
        let entity_count = world.len();

        // 读取 TimeTracker 获取 FPS
        for (_, time) in world.query::<&TimeTracker>().iter() {
            fps = time.fps;
            break;
        }

        // 读取 Camera 和 Position 获取相机信息
        for (_, (camera, pos)) in world.query::<(&Camera, &Position)>().iter() {
            camera_pos = (pos.x, pos.y);
            camera_zoom = camera.zoom;
            break;
        }

        // 2. 构建调试文本
        let debug_text = format!(
            "FPS: {:.1}\nCamera: ({:.0}, {:.0})\nZoom: {:.2}x\nEntities: {}",
            fps, camera_pos.0, camera_pos.1, camera_zoom, entity_count
        );

        // 3. 绘制调试文本（左上角，黑色背景，白色文字）
        let text = Text::new(TextFragment {
            text: debug_text,
            color: Some(Color::WHITE),
            font: None,
            scale: Some(ggez::graphics::PxScale::from(24.0)),  // 从 16.0 增大到 24.0
        });

        // 绘制半透明黑色背景
        let bg_rect = ggez::graphics::Rect::new(5.0, 5.0, 250.0, 100.0);  // 调整背景大小
        let bg_mesh = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            bg_rect,
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&bg_mesh, DrawParam::default());

        // 绘制文本
        canvas.draw(&text, DrawParam::default().dest([10.0, 10.0]));

        // 获取 RenderConfig
        let config = {
            let mut config_query = world.query::<&RenderConfig>();
            config_query.iter().next().map(|(_, cfg)| cfg.clone())
        };

        if let Some(config) = config {
            // 获取相机信息用于网格和边框绘制
            if let Some((camera, pos)) = world.query::<(&Camera, &Position)>().iter().next().map(|(_, (c, p))| (c.clone(), p.clone())) {
                // 绘制网格
                if config.show_grid {
                    Self::draw_grid(ctx, canvas, &camera, &pos)?;
                }

                // 绘制瓦片边框
                if config.show_borders {
                    Self::draw_tile_borders(ctx, canvas, world, &camera, &pos, &config)?;
                }
            }
        }

        Ok(())
    }
}

impl DebugSystem {
    /// 绘制网格线
    fn draw_grid(
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        camera: &Camera,
        camera_pos: &Position,
    ) -> GameResult {
        use crate::ecs::{CELL_HEIGHT, CELL_WIDTH};
        use ggez::graphics::{Color, Mesh, DrawMode, DrawParam};

        // 网格颜色：半透明白色
        let grid_color = Color::from_rgba(255, 255, 255, 80);

        // 计算视口范围
        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        let half_height = (camera.screen_height / 2.0) / camera.zoom;
        let view_left = camera_pos.x - half_width;
        let view_right = camera_pos.x + half_width;
        let view_top = camera_pos.y - half_height;
        let view_bottom = camera_pos.y + half_height;

        // 计算可见的网格范围
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
        use ggez::graphics::{Color, Mesh, DrawMode, DrawParam};

        // 计算视口范围
        let half_width = (camera.screen_width / 2.0) / camera.zoom;
        let half_height = (camera.screen_height / 2.0) / camera.zoom;
        let view_left = camera_pos.x - half_width;
        let view_right = camera_pos.x + half_width;
        let view_top = camera_pos.y - half_height;
        let view_bottom = camera_pos.y + half_height;

        // 遍历所有瓦片
        for (_, tile) in world.query::<&MapTile>().iter() {
            // 根据配置跳过某些层
            match tile.layer {
                TileLayer::Back if !config.show_back => continue,
                TileLayer::Middle if !config.show_middle => continue,
                TileLayer::Front if !config.show_front => continue,
                _ => {}
            }

            // 计算瓦片世界坐标
            let world_x = (tile.grid_x * CELL_WIDTH) as f32;
            let world_y = (tile.grid_y * CELL_HEIGHT) as f32;

            // 视口裁剪
            let bottom_extra = if matches!(tile.layer, TileLayer::Front) { 800.0 } else { 200.0 };
            if world_x > view_right + 200.0
                || world_x < view_left - 200.0
                || world_y > view_bottom + bottom_extra
                || world_y < view_top - 200.0
            {
                continue;
            }

            // 获取瓦片尺寸
            if let Some(lib) = get_map_library(tile.library_index) {
                if let Ok(mut lib_guard) = lib.lock() {
                    let (tile_w, tile_h) = lib_guard
                        .get_size(tile.image_index as usize)
                        .unwrap_or((CELL_WIDTH as i16, CELL_HEIGHT as i16));

                    // 计算屏幕坐标
                    let adjusted_y = world_y - tile_h as f32;
                    let screen_x = (world_x - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
                    let screen_y = (adjusted_y - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;

                    // 边框颜色
                    let border_color = match tile.layer {
                        TileLayer::Back => Color::from_rgb(255, 0, 0),    // 红色
                        TileLayer::Middle => Color::from_rgb(0, 255, 0),  // 绿色
                        TileLayer::Front => Color::from_rgb(0, 150, 255), // 蓝色
                    };

                    // 绘制边框
                    let border_rect = ggez::graphics::Rect::new(
                        screen_x,
                        screen_y,
                        tile_w as f32 * camera.zoom,
                        tile_h as f32 * camera.zoom,
                    );

                    let border_mesh = Mesh::new_rectangle(
                        ctx,
                        DrawMode::stroke(1.0),
                        border_rect,
                        border_color,
                    )?;

                    canvas.draw(&border_mesh, DrawParam::default());
                }
            }
        }

        Ok(())
    }

    /// 处理调试相关的键盘按键
    fn handle_keycode(world: &mut hecs::World, keycode: KeyCode) {
        // 查找 RenderConfig 组件
        let mut config_query = world.query::<&mut RenderConfig>();
        let config = if let Some((_, cfg)) = config_query.iter().next() {
            cfg
        } else {
            return;
        };

        match keycode {
            // 图层切换
            KeyCode::Digit1 => {
                config.show_back = !config.show_back;
                tracing::info!("🖼️ Back 层: {}", if config.show_back { "显示" } else { "隐藏" });
            }
            KeyCode::Digit2 => {
                config.show_middle = !config.show_middle;
                tracing::info!("🖼️ Middle 层: {}", if config.show_middle { "显示" } else { "隐藏" });
            }
            KeyCode::Digit3 => {
                config.show_front = !config.show_front;
                tracing::info!("🖼️ Front 层: {}", if config.show_front { "显示" } else { "隐藏" });
            }
            
            // 调试显示选项
            KeyCode::KeyG => {
                config.show_grid = !config.show_grid;
                tracing::info!("🔲 网格: {}", if config.show_grid { "显示" } else { "隐藏" });
            }
            KeyCode::KeyO => {
                config.show_obstacles = !config.show_obstacles;
                tracing::info!("🚧 障碍物: {}", if config.show_obstacles { "显示" } else { "隐藏" });
            }
            KeyCode::KeyB => {
                config.show_borders = !config.show_borders;
                tracing::info!("📦 边框: {}", if config.show_borders { "显示" } else { "隐藏" });
            }
            KeyCode::KeyP => {
                config.show_path = !config.show_path;
                tracing::info!("🛤️ 路径: {}", if config.show_path { "显示" } else { "隐藏" });
            }
            KeyCode::KeyA => {
                config.show_animations = !config.show_animations;
                tracing::info!("🎬 动画: {}", if config.show_animations { "播放" } else { "暂停" });
            }
            KeyCode::KeyL => {
                config.enable_lod = !config.enable_lod;
                tracing::info!("🔍 LOD: {}", if config.enable_lod { "启用" } else { "禁用" });
            }
            
            // F键 - 边框显示
            KeyCode::F9 => {
                config.show_monster_borders = !config.show_monster_borders;
                tracing::info!("👹 怪物边框: {}", if config.show_monster_borders { "显示" } else { "隐藏" });
            }
            KeyCode::F10 => {
                config.show_npc_borders = !config.show_npc_borders;
                tracing::info!("👤 NPC边框: {}", if config.show_npc_borders { "显示" } else { "隐藏" });
            }
            KeyCode::F11 => {
                config.show_effect_borders = !config.show_effect_borders;
                tracing::info!("✨ 特效边框: {}", if config.show_effect_borders { "显示" } else { "隐藏" });
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
                // 注意：退出程序需要调用 ctx.request_quit()，这需要在有 Context 的地方处理
            }
            
            _ => {}
        }
    }
}
