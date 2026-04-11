use crate::components::{
    AttackState, LocalPlayer, MountState, Movement, MovementVelocity, Path, Player, PlayerData,
    Position, RenderConfig, RenderPass, RenderStage, Velocity, Camera, MapData,
};
use crate::systems::logic::physics::MapUpdateSystem;
use crate::systems::RenderSystem;
use macroquad::prelude::*;

/// DebugSystemV2 - 使用 GameContext 的调试系统
///
/// **改进点**:
/// - ✅ 使用 GameContext 零拷贝访问 ggez Context (用于渲染)
/// - ✅ 从 GameContext 读取按键事件 (使用边缘检测)
///
/// **职责**:
/// - 检测调试快捷键（图层切换、网格、边框等）
/// - 修改 RenderConfig 组件
/// - 渲染调试信息（FPS、实体数量等）
#[derive(ecs_macros::RenderSystem)]
pub struct DebugSystem;

impl DebugSystem {
    pub fn new() -> Self {
        Self
    }

    /// 处理调试相关的键盘按键
    #[allow(dead_code)]
    fn handle_keycode(world: &mut hecs::World, keycode: KeyCode) {
        // M键需要特殊处理，因为它会借用整个 world
        if keycode == KeyCode::M {
            tracing::info!("🗺️ 打开地图选择对话框");

            MapUpdateSystem::trigger_map_selection(world);
            return;
        }

        // 查找 RenderConfig 组件
        let mut config_query = world.query::<&mut RenderConfig>();
        let config = if let Some(cfg) = config_query.iter().next() {
            cfg
        } else {
            return;
        };

        match keycode {
            // 图层切换 (与 C# 版本一致: 1=Front, 2=Middle, 3=Back)
            KeyCode::Key1 => {
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
            KeyCode::Key2 => {
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
            KeyCode::Key3 => {
                config.show_back = !config.show_back;
                tracing::info!(
                    "🖼️ Back 层: {}",
                    if config.show_back { "显示" } else { "隐藏" }
                );
            }

            // 调试显示选项
            KeyCode::G => {
                config.show_grid = !config.show_grid;
                tracing::info!(
                    "🔲 网格: {}",
                    if config.show_grid { "显示" } else { "隐藏" }
                );
            }
            KeyCode::O => {
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
            KeyCode::B => {
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
            KeyCode::P => {
                config.show_path = !config.show_path;
                tracing::info!(
                    "🛤️ 路径: {}",
                    if config.show_path { "显示" } else { "隐藏" }
                );
            }
            KeyCode::A => {
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
            KeyCode::S => {
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
            KeyCode::D => {
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
            KeyCode::L => {
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

            // F键 - 调试显示
            KeyCode::F2 => {
                config.show_player_debug = !config.show_player_debug;
                tracing::info!(
                    "👤 玩家调试信息: {}",
                    if config.show_player_debug {
                        "显示"
                    } else {
                        "隐藏"
                    }
                );
            }
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
            KeyCode::Equal | KeyCode::KpAdd => {
                config.max_fps = (config.max_fps + 10).min(300);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }
            KeyCode::Minus | KeyCode::KpSubtract => {
                config.max_fps = (config.max_fps.saturating_sub(10)).max(30);
                tracing::info!("⚡ 最大FPS: {}", config.max_fps);
            }

            KeyCode::Escape => {
                tracing::info!("👋 用户按下 ESC，应该退出程序（需要在 App 层处理）");
            }

            _ => {}
        }
    }
}

impl RenderSystem for DebugSystem {
    fn update(&mut self, ctx: &mut crate::game::GameContext, _delay_time: f32) -> crate::game::GameResult {
        if !cfg!(debug_assertions) {
            return Ok(());
        }

        // macroquad 的 is_key_pressed 本身就是“边缘触发”（本帧刚按下），
        // 这里不需要依赖 InputState.prev_pressed_keys 做额外检测。
        const DEBUG_KEYS: &[KeyCode] = &[
            KeyCode::O,
            KeyCode::P,
            KeyCode::G,
            KeyCode::B,
            KeyCode::A,
            KeyCode::S,
            KeyCode::D,
            KeyCode::L,
            KeyCode::M,
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::F2,
            KeyCode::F9,
            KeyCode::F10,
            KeyCode::F11,
            KeyCode::Equal,
            KeyCode::KpAdd,
            KeyCode::Minus,
            KeyCode::KpSubtract,
            KeyCode::Escape,
        ];

        // 注意：Debug 热键应当不受 UI 的 world-input 阻塞影响。
        // 这里直接读 macroquad 的按键边缘事件。
        for &key in DEBUG_KEYS {
            if is_key_pressed(key) {
                Self::handle_keycode(&mut ctx.world, key);
            }
        }

        Ok(())
    }

    fn draw(
        &mut self,
        world: &hecs::World,
    ) -> crate::game::GameResult {
        let stage = world
            .query::<&RenderPass>()
            .iter()
            .next()
            .map(|p| p.stage)
            .unwrap_or(RenderStage::Normal);

        match stage {
            // 世界空间叠加层：依赖 GameScene 已经 set_camera(map_camera)
            RenderStage::PostFront => {
                self.draw_world_debug_overlays(world);
                self.draw_path_overlay(world);
            }
            // 屏幕空间叠加层：依赖 GameScene 已经 set_default_camera()
            RenderStage::Ui => {
                self.draw_debug_fps(world);
                self.draw_player_debug(world);
            }
            _ => {}
        }

        Ok(())
    }
}

impl DebugSystem {
    fn draw_world_debug_overlays(&self, world: &hecs::World) {
        if !cfg!(debug_assertions) {
            return;
        }

        let cfg = world
            .query::<&RenderConfig>()
            .iter()
            .next()
            .map(|c| c)
            .cloned()
            .unwrap_or_default();

        if !cfg.show_grid && !cfg.show_obstacles && !cfg.show_borders {
            return;
        }

        // 获取相机（Position 表示 camera center）
        let (cam_pos, cam) = {
            let mut q = world.query::<(&Position, &Camera)>();
            let Some((p, c)) = q.iter().next() else {
                return;
            };
            (*p, c.clone())
        };

        let zoom = cam.zoom.max(0.01);
        let view_left = cam_pos.x - cam.screen_width / (2.0 * zoom);
        let view_right = cam_pos.x + cam.screen_width / (2.0 * zoom);
        let view_top = cam_pos.y - cam.screen_height / (2.0 * zoom);
        let view_bottom = cam_pos.y + cam.screen_height / (2.0 * zoom);

        let tile_w = 48.0;
        let tile_h = 32.0;

        let mut start_x = ((view_left / tile_w).floor() as i32 - 1).max(0);
        let mut end_x = ((view_right / tile_w).floor() as i32 + 1).max(0);
        let mut start_y = ((view_top / tile_h).floor() as i32 - 1).max(0);
        let mut end_y = ((view_bottom / tile_h).floor() as i32 + 1).max(0);

        // 可选：根据 MapData clamp 范围，并用于障碍物查询
        // 注意：QueryBorrow 需要绑定到局部变量，避免临时值提前释放。
        let mut map_q = world.query::<&MapData>();
        let map_data = map_q.iter().next().map(|m| m);
        if let Some(m) = map_data {
            start_x = start_x.min(m.width.saturating_sub(1));
            end_x = end_x.min(m.width.saturating_sub(1));
            start_y = start_y.min(m.height.saturating_sub(1));
            end_y = end_y.min(m.height.saturating_sub(1));
        }

        // O：障碍物格子
        if cfg.show_obstacles {
            if let Some(m) = map_data {
                for gx in start_x..=end_x {
                    let ux = gx as usize;
                    if ux >= m.cells.len() {
                        continue;
                    }
                    for gy in start_y..=end_y {
                        let uy = gy as usize;
                        if uy >= m.cells[ux].len() {
                            continue;
                        }
                        if !m.cells[ux][uy].is_walkable() {
                            draw_rectangle(
                                gx as f32 * tile_w,
                                gy as f32 * tile_h,
                                tile_w,
                                tile_h,
                                Color::from_rgba(255, 0, 0, 70),
                            );
                        }
                    }
                }
            }
        }

        // G：网格线（每格）
        if cfg.show_grid {
            let top = start_y as f32 * tile_h;
            let bottom = (end_y as f32 + 1.0) * tile_h;
            for gx in start_x..=(end_x + 1) {
                let x = gx as f32 * tile_w;
                draw_line(x, top, x, bottom, 1.0, Color::from_rgba(255, 255, 255, 60));
            }

            let left = start_x as f32 * tile_w;
            let right = (end_x as f32 + 1.0) * tile_w;
            for gy in start_y..=(end_y + 1) {
                let y = gy as f32 * tile_h;
                draw_line(left, y, right, y, 1.0, Color::from_rgba(255, 255, 255, 60));
            }
        }

        // B：Chunk 边界（32x32 tiles，对齐 MeshMapRenderer 的 chunk cache）
        if cfg.show_borders {
            let chunk_tiles_x: i32 = 32;
            let chunk_tiles_y: i32 = 32;

            let chunk_start_x = (start_x / chunk_tiles_x) * chunk_tiles_x;
            let chunk_end_x = (end_x / chunk_tiles_x) * chunk_tiles_x;
            let chunk_start_y = (start_y / chunk_tiles_y) * chunk_tiles_y;
            let chunk_end_y = (end_y / chunk_tiles_y) * chunk_tiles_y;

            let mut cx = chunk_start_x;
            while cx <= chunk_end_x + chunk_tiles_x {
                let x = cx as f32 * tile_w;
                draw_line(
                    x,
                    chunk_start_y as f32 * tile_h,
                    x,
                    (chunk_end_y as f32 + chunk_tiles_y as f32) * tile_h,
                    2.0,
                    Color::from_rgba(0, 200, 255, 120),
                );
                cx += chunk_tiles_x;
            }

            let mut cy = chunk_start_y;
            while cy <= chunk_end_y + chunk_tiles_y {
                let y = cy as f32 * tile_h;
                draw_line(
                    chunk_start_x as f32 * tile_w,
                    y,
                    (chunk_end_x as f32 + chunk_tiles_x as f32) * tile_w,
                    y,
                    2.0,
                    Color::from_rgba(0, 200, 255, 120),
                );
                cy += chunk_tiles_y;
            }

            // 地图边界（如果有 MapData）
            if let Some(m) = map_data {
                draw_rectangle_lines(
                    0.0,
                    0.0,
                    m.width as f32 * tile_w,
                    m.height as f32 * tile_h,
                    3.0,
                    Color::from_rgba(255, 255, 0, 140),
                );
            }
        }
    }

    fn draw_debug_fps(&self, _world: &hecs::World) {
        if !cfg!(debug_assertions) {
            return;
        }

        let fps = get_fps();
        draw_text(
            &format!("FPS: {}", fps),
            12.0,
            22.0,
            20.0,
            Color::from_rgba(0, 255, 0, 220),
        );
    }

    fn draw_path_overlay(&self, world: &hecs::World) {
        // 保持与原先 GameScene 的表现一致：存在有效 Path 就绘制。
        // 由 DebugSystem 的按键切换（P）控制。
        let show_path = world
            .query::<&RenderConfig>()
            .iter()
            .next()
            .map(|c| c.show_path)
            .unwrap_or(false);

        if !show_path {
            return;
        }

        let mut q = world.query::<(&LocalPlayer, &Position, &Path)>();
        let Some((_lp, pos, path)) = q.iter().next() else {
            return;
        };

        if !path.is_valid || path.waypoints.is_empty() {
            return;
        }

        let mut last = (pos.x, pos.y);
        for (gx, gy) in path.waypoints.iter().copied() {
            let wx = gx as f32 * 48.0;
            let wy = gy as f32 * 32.0;
            draw_line(
                last.0,
                last.1,
                wx,
                wy,
                2.0,
                Color::from_rgba(255, 255, 0, 180),
            );
            last = (wx, wy);
        }
    }

    fn draw_player_debug(&self, world: &hecs::World) {
        if !cfg!(debug_assertions) {
            return;
        }

        let show_player_debug = world
            .query::<&RenderConfig>()
            .iter()
            .next()
            .map(|c| c.show_player_debug)
            .unwrap_or(false);

        if !show_player_debug {
            return;
        }

        let entity = world.iter().find_map(|e| {
            if e.get::<&LocalPlayer>().is_some() {
                return Some(e.entity());
            }
            None
        }).unwrap();
        let mut q = world.query::<(&LocalPlayer, &Position, &Player)>();
        let Some((_lp, pos, player)) = q.iter().next() else {
            draw_text(
                "LocalPlayer: <not found>",
                12.0,
                46.0,
                18.0,
                Color::from_rgba(255, 255, 255, 220),
            );
            return;
        };

        let player_data = world.get::<&PlayerData>(entity).ok();
        let movement = world.get::<&Movement>(entity).ok();
        let movement_velocity = world.get::<&MovementVelocity>(entity).ok();
        let velocity = world.get::<&Velocity>(entity).ok();
        let mount_state = world.get::<&MountState>(entity).ok();
        let attack_state = world.get::<&AttackState>(entity).ok();
        let path = world.get::<&Path>(entity).ok();

        let grid_x = (pos.x / 48.0).floor() as i32;
        let grid_y = (pos.y / 32.0).floor() as i32;

        let mut lines: Vec<String> = Vec::with_capacity(10);
        if let Some(pd) = player_data.as_deref() {
            lines.push(format!("Player: {} (real_id={} object_id={})", pd.name, pd.id, pd.object_id));
        } else {
            lines.push("Player: <no PlayerData>".to_string());
        }

        lines.push(format!(
            "Action: {:?} | Dir: {:?}",
            player.action, player.direction
        ));
        lines.push(format!("Pos(px): {:.1}, {:.1}", pos.x, pos.y));
        lines.push(format!("Pos(grid): {}, {}", grid_x, grid_y));

        if let Some(m) = movement.as_deref() {
            lines.push(format!("Move: {:?} | moving={}", m.state, m.is_moving()));
        }
        if let Some(v) = movement_velocity.as_deref() {
            lines.push(format!(
                "MovementVelocity(px/frame): {:.2}, {:.2} | mag={:.2}",
                v.x,
                v.y,
                v.magnitude()
            ));
        }
        if let Some(v) = velocity.as_deref() {
            lines.push(format!("Velocity(dx/dy): {:.2}, {:.2}", v.dx, v.dy));
        }
        if let Some(ms) = mount_state.as_deref() {
            lines.push(format!("Mount: {:?}", ms.mount_index));
        }

        if let Some(a) = attack_state.as_deref() {
            let elapsed_ms = a.start_time.elapsed().as_millis();
            lines.push(format!(
                "AttackState: {:?} | elapsed={}ms | server_type={}",
                a.attack_type, elapsed_ms, a.server_attack_type
            ));
        }

        if let Some(p) = path.as_deref() {
            lines.push(format!(
                "Path: valid={} | points={} | idx={}",
                p.is_valid,
                p.waypoints.len(),
                p.current_index
            ));
        }

        let font_size = 28.0;
        let line_height = 32.0;

        let x = 12.0;
        let mut y = 58.0;
        for line in lines {
            draw_text(
                &line,
                x,
                y,
                font_size,
                Color::from_rgba(255, 255, 255, 220),
            );
            y += line_height;
        }
    }
}
