//! 完整的 Type 100 地图查看器
//!
//! - 绘制 Back / Middle / Front 三层静态瓦片
//! - 无需纹理 Offset，直接使用 `mlibrary.rs` 的绘制接口
//! - 支持绘制地图网格、纹理边框、地表障碍高亮
//! - 鼠标悬停显示当前格子的详细资源信息

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Canvas, Color, DrawMode, DrawParam, FontData, Mesh, Rect, Text};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::input::mouse::MouseButton;
use ggez::{Context, ContextBuilder, GameResult};
use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};

/// Type 100 地图瓦片尺寸
const TILE_WIDTH: i32 = 48;
const TILE_HEIGHT: i32 = 32;

/// 窗口尺寸
const SCREEN_WIDTH: f32 = 1920.0;
const SCREEN_HEIGHT: f32 = 1080.0;

/// 视野参数（与 C# 客户端保持一致）
const OFFSET_X: i32 = ((SCREEN_WIDTH as i32 / 2) / TILE_WIDTH); // 20（偶数）
const OFFSET_Y: i32 = ((SCREEN_HEIGHT as i32 / 2) / TILE_HEIGHT - 1); // 16（偶数）
const VIEW_RANGE_X: i32 = OFFSET_X + 6; // 26，覆盖屏幕宽度并留缓冲
const VIEW_RANGE_Y: i32 = OFFSET_Y + 6; // 22，覆盖屏幕高度并留缓冲

/// 细调常量，用于需要时的像素级校准
const FINE_TUNE_X: i32 = 0;
const FINE_TUNE_Y: i32 = 0;

struct MapViewer {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,

    offset_x: i32,
    offset_y: i32,
    dragging: bool,
    last_mouse_pos: (f32, f32),
    accumulated_drag_x: f32,
    accumulated_drag_y: f32,

    show_tile_grid: bool,
    show_image_border: bool,
    render_back: bool,
    render_middle: bool,
    render_front: bool,
    show_cell_flags: bool,

    mouse_x: f32,
    mouse_y: f32,
}

impl MapViewer {
    fn new(map_path: &str) -> GameResult<Self> {
         println!("📚 正在初始化地图库...");
        initialize_all_libraries("Data")
            .map_err(|err| ggez::GameError::ResourceLoadError(format!("初始化地图库失败: {err}")))?;
        println!("✅ 地图库初始化完成");

        println!("📂 正在加载地图: {map_path}");
        let reader = MapReader::new(map_path)
            .map_err(|err| ggez::GameError::ResourceLoadError(err.to_string()))?;
        println!("✅ 地图尺寸: {} x {}", reader.width, reader.height);

       

        println!("🎮 控制说明:");
        println!("  ↑↓←→ / WASD     - 平移视口 (两格为单位)");
        println!("  🖱️  鼠标左键拖拽  - 平移视口");
        println!("  T              - 切换地图网格 [默认: 开]");
        println!("  B              - 切换纹理边框 [默认: 关]");
        println!("  F              - 切换地表障碍高亮 [默认: 关]");
        println!("  1/2/3          - 切换 Back/Middle/Front 层 [默认: 全开]");
        println!("  ESC            - 退出");
        println!("  鼠标悬停        - 显示格子详细信息");

        Ok(Self {
            cells: reader.map_cells,
            width: reader.width,
            height: reader.height,
            offset_x: OFFSET_X,
            offset_y: OFFSET_Y,
            dragging: false,
            last_mouse_pos: (0.0, 0.0),
            accumulated_drag_x: 0.0,
            accumulated_drag_y: 0.0,
            show_tile_grid: true,
            show_image_border: false,
            render_back: true,
            render_middle: true,
            render_front: true,
            show_cell_flags: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
        })
    }

    #[inline]
    fn cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        (x >= 0 && x < self.width && y >= 0 && y < self.height)
            .then_some(&self.cells[x as usize][y as usize])
    }

    #[inline]
    fn map_to_screen(&self, map_x: i32, map_y: i32) -> (f32, f32) {
        let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X + FINE_TUNE_X) as f32;
        let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT + FINE_TUNE_Y) as f32;
        (screen_x, screen_y)
    }

    fn max_offsets(&self) -> (i32, i32) {
        let max_x = (self.width - OFFSET_X).max(OFFSET_X);
        let max_y = (self.height - OFFSET_Y).max(OFFSET_Y);
        (max_x, max_y)
    }

    fn clamp_offsets(&mut self) {
        let (max_x, max_y) = self.max_offsets();
        self.offset_x = self.offset_x.clamp(OFFSET_X, max_x);
        self.offset_y = self.offset_y.clamp(OFFSET_Y, max_y);
    }

    fn draw_back_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.render_back {
            return Ok(());
        }

        canvas.set_blend_mode(graphics::BlendMode::REPLACE);

        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 1).min(self.height);

        for map_y in start_y..end_y {
            if map_y <= 0 || map_y % 2 != 0 {
                continue;
            }
            for map_x in start_x..end_x {
                if map_x <= 0 || map_x % 2 != 0 {
                    continue;
                }

                let Some(cell) = self.cell(map_x, map_y) else { continue; };
                if cell.back_image <= 0 || cell.back_index < 0 {
                    continue;
                }

                let Some(lib_arc) = get_map_library(cell.back_index) else { continue; };
                let image_index = ((cell.back_image & 0x1FFF_FFFF) as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                let (screen_x, screen_y) = self.map_to_screen(map_x, map_y);
                if lib.draw(ctx, canvas, image_index, screen_x, screen_y).is_ok() && self.show_image_border {
                    let border = Mesh::new_rectangle(
                        ctx,
                        DrawMode::stroke(1.0),
                        Rect::new(screen_x, screen_y, info.width as f32, info.height as f32),
                        Color::from_rgb(255, 0, 0),
                    )?;
                    canvas.draw(&border, DrawParam::default());
                }
            }
        }

        Ok(())
    }

    fn draw_middle_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.render_middle {
            return Ok(());
        }

        canvas.set_blend_mode(graphics::BlendMode::REPLACE);

        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 6).min(self.height);

        for map_y in start_y..end_y {
            if map_y <= 0 {
                continue;
            }

            for map_x in start_x..end_x {
                let Some(cell) = self.cell(map_x, map_y) else { continue; };
                if cell.middle_image <= 0 || cell.middle_index < 0 {
                    continue;
                }

                let Some(lib_arc) = get_map_library(cell.middle_index) else { continue; };
                let image_index = (cell.middle_image as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                let valid_size = (info.width == TILE_WIDTH as i16 && info.height == TILE_HEIGHT as i16)
                    || (info.width == (TILE_WIDTH * 2) as i16 && info.height == (TILE_HEIGHT * 2) as i16);
                if !valid_size {
                    continue;
                }

                let (screen_x, screen_y) = self.map_to_screen(map_x, map_y);
                if lib.draw(ctx, canvas, image_index, screen_x, screen_y).is_ok() && self.show_image_border {
                    let border = Mesh::new_rectangle(
                        ctx,
                        DrawMode::stroke(1.0),
                        Rect::new(screen_x, screen_y, info.width as f32, info.height as f32),
                        Color::from_rgb(255, 165, 0),
                    )?;
                    canvas.draw(&border, DrawParam::default());
                }
            }
        }

        Ok(())
    }

    fn draw_front_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.render_front {
            return Ok(());
        }

        canvas.set_blend_mode(graphics::BlendMode::ALPHA);

        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 6).min(self.height);

        for map_y in start_y..end_y {
            if map_y <= 0 {
                continue;
            }

            for map_x in start_x..end_x {
                let Some(cell) = self.cell(map_x, map_y) else { continue; };
                let front_image = cell.front_image & 0x7FFF;
                if front_image <= 0 || cell.front_index < 0 {
                    continue;
                }

                let Some(lib_arc) = get_map_library(cell.front_index) else { continue; };
                let image_index = (front_image as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                let (screen_x, screen_y) = self.map_to_screen(map_x, map_y);
                if lib
                    .draw_tinted(
                        ctx,
                        canvas,
                        image_index,
                        screen_x,
                        screen_y - info.height as f32,
                        Color::WHITE,
                        Color::WHITE,
                        false,
                    )
                    .is_ok()
                    && self.show_image_border
                {
                    let border = Mesh::new_rectangle(
                        ctx,
                        DrawMode::stroke(1.0),
                        Rect::new(screen_x, screen_y - info.height as f32, info.width as f32, info.height as f32),
                        Color::from_rgb(0, 150, 255),
                    )?;
                    canvas.draw(&border, DrawParam::default());
                }
            }
        }

        Ok(())
    }

    fn draw_tile_grid(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.show_tile_grid {
            return Ok(());
        }

        // 只绘制可视范围内实际存在的地图格子
        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X).min(self.width - 1);
        let end_y = (self.offset_y + VIEW_RANGE_Y).min(self.height - 1);

        let grid_color = Color::from_rgba(0, 255, 0, 120);

        // 绘制垂直线（每个格子左边缘）
        for map_x in start_x..=end_x {
            let (screen_x, screen_y_top) = self.map_to_screen(map_x, start_y);
            let (_, screen_y_bottom) = self.map_to_screen(map_x, end_y);
            
            // 只绘制在屏幕内的线段
            if screen_x >= 0.0 && screen_x <= SCREEN_WIDTH {
                let line = Mesh::new_line(
                    ctx,
                    &[[screen_x, screen_y_top.max(0.0)], [screen_x, screen_y_bottom.min(SCREEN_HEIGHT)]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 绘制水平线（每个格子上边缘）
        for map_y in start_y..=end_y {
            let (screen_x_left, screen_y) = self.map_to_screen(start_x, map_y);
            let (screen_x_right, _) = self.map_to_screen(end_x, map_y);
            
            // 只绘制在屏幕内的线段
            if screen_y >= 0.0 && screen_y <= SCREEN_HEIGHT {
                let line = Mesh::new_line(
                    ctx,
                    &[[screen_x_left.max(0.0), screen_y], [screen_x_right.min(SCREEN_WIDTH), screen_y]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 每隔10格标注一次坐标（避免太密集）
        let step = if self.width > 100 { 10 } else { 5 };
        for map_x in (start_x..=end_x).step_by(step as usize) {
            for map_y in (start_y..=end_y).step_by(step as usize) {
                let (screen_x, screen_y) = self.map_to_screen(map_x, map_y);
                
                // 只在屏幕内显示坐标
                if screen_x >= 0.0 && screen_x <= SCREEN_WIDTH - 100.0 
                   && screen_y >= 0.0 && screen_y <= SCREEN_HEIGHT - 20.0 {
                    let mut text = Text::new(format!("({},{})", map_x, map_y));
                    text.set_font("AlibabaPuHui");
                    text.set_scale(12.0);
                    canvas.draw(
                        &text,
                        DrawParam::default()
                            .dest([screen_x + 4.0, screen_y + 4.0])
                            .color(Color::from_rgb(0, 255, 0)),
                    );
                }
            }
        }

        Ok(())
    }

    fn draw_cell_flags(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.show_cell_flags {
            return Ok(());
        }

        let start_x = (self.offset_x - VIEW_RANGE_X - 1).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y - 1).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 2).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 2).min(self.height);

        for map_y in start_y..end_y {
            for map_x in start_x..end_x {
                let Some(cell) = self.cell(map_x, map_y) else { continue; };
                let (screen_x, screen_y) = self.map_to_screen(map_x, map_y);
                let has_obstacle = cell.front_image > 0;
                let color = if has_obstacle {
                    Color::from_rgba(255, 0, 0, 140)
                } else {
                    Color::from_rgba(0, 255, 0, 70)
                };

                let rect = Mesh::new_rectangle(
                    ctx,
                    DrawMode::fill(),
                    Rect::new(screen_x, screen_y, TILE_WIDTH as f32, TILE_HEIGHT as f32),
                    color,
                )?;
                canvas.draw(&rect, DrawParam::default());
            }
        }

        Ok(())
    }

    fn draw_hover_tooltip(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 计算鼠标所在的地图格子坐标
        let map_x = ((self.mouse_x / TILE_WIDTH as f32).floor() as i32 + self.offset_x - OFFSET_X)
            .clamp(0, self.width - 1);
        let map_y = ((self.mouse_y / TILE_HEIGHT as f32).floor() as i32 + self.offset_y - OFFSET_Y)
            .clamp(0, self.height - 1);

        let cell = &self.cells[map_x as usize][map_y as usize];

        let mut lines = Vec::with_capacity(5);
        lines.push(format!("📍 地图坐标: ({}, {})", map_x, map_y));

        // Back 层信息
        if cell.back_index >= 0 && cell.back_image > 0 {
            let masked_image = cell.back_image & 0x1FFF_FFFF;
            lines.push(format!("⬜ Back: 库[{}] 图像[{}]", cell.back_index, masked_image));
        } else {
            lines.push("⬜ Back: 无".to_string());
        }

        // Middle 层信息
        if cell.middle_index >= 0 && cell.middle_image > 0 {
            lines.push(format!("🟦 Middle: 库[{}] 图像[{}]", cell.middle_index, cell.middle_image));
        } else {
            lines.push("🟦 Middle: 无".to_string());
        }

        // Front 层信息（带障碍标记）
        if cell.front_index >= 0 && cell.front_image > 0 {
            let front_masked = cell.front_image & 0x7FFF;
            let is_obstacle = cell.front_image > 0;
            let obstacle_str = if is_obstacle { " 🚫" } else { "" };
            lines.push(format!("🟥 Front: 库[{}] 图像[{}]{}", cell.front_index, front_masked, obstacle_str));
        } else {
            lines.push("🟥 Front: 无".to_string());
        }

        let box_width = 280.0;
        let box_height = lines.len() as f32 * 22.0 + 10.0;

        let mut box_x = self.mouse_x + 15.0;
        let mut box_y = self.mouse_y + 15.0;

        if box_x + box_width > SCREEN_WIDTH {
            box_x = self.mouse_x - box_width - 15.0;
            if box_x < 5.0 {
                box_x = SCREEN_WIDTH - box_width - 5.0;
            }
        }

        if box_y + box_height > SCREEN_HEIGHT {
            box_y = self.mouse_y - box_height - 15.0;
            if box_y < 5.0 {
                box_y = SCREEN_HEIGHT - box_height - 5.0;
            }
        }

        box_x = box_x.max(5.0);
        box_y = box_y.max(5.0);

        let background = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(box_x, box_y, box_width, box_height),
            Color::from_rgba(0, 0, 0, 200),
        )?;
        canvas.draw(&background, DrawParam::default());

        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            Rect::new(box_x, box_y, box_width, box_height),
            Color::from_rgb(100, 200, 255),
        )?;
        canvas.draw(&border, DrawParam::default());

        for (i, line) in lines.iter().enumerate() {
            let mut text = Text::new(line);
            text.set_font("AlibabaPuHui");
            text.set_scale(16.0);
            canvas.draw(
                &text,
                DrawParam::default()
                    .dest([box_x + 6.0, box_y + 6.0 + i as f32 * 22.0])
                    .color(Color::WHITE),
            );
        }

        Ok(())
    }

    fn draw_ui_overlay(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 计算当前地图格子
        let map_x = ((self.mouse_x / TILE_WIDTH as f32).floor() as i32 + self.offset_x - OFFSET_X)
            .clamp(0, self.width - 1);
        let map_y = ((self.mouse_y / TILE_HEIGHT as f32).floor() as i32 + self.offset_y - OFFSET_Y)
            .clamp(0, self.height - 1);

        let summary = format!(
            "🗺️  地图: {}x{}  视口: ({}, {})\n📍 鼠标: 屏幕({:.0}, {:.0}) → 地图({}, {})\n🎨 图层: Back[{}] Middle[{}] Front[{}]\n🔍 显示: 网格[{}] 边框[{}] 障碍[{}]",
            self.width,
            self.height,
            self.offset_x,
            self.offset_y,
            self.mouse_x,
            self.mouse_y,
            map_x,
            map_y,
            if self.render_back { "✓" } else { "✗" },
            if self.render_middle { "✓" } else { "✗" },
            if self.render_front { "✓" } else { "✗" },
            if self.show_tile_grid { "✓" } else { "✗" },
            if self.show_image_border { "✓" } else { "✗" },
            if self.show_cell_flags { "✓" } else { "✗" },
        );

        let background = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(10.0, 10.0, 500.0, 100.0),
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&background, DrawParam::default());

        let border = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(1.5),
            Rect::new(10.0, 10.0, 500.0, 100.0),
            Color::from_rgb(80, 120, 200),
        )?;
        canvas.draw(&border, DrawParam::default());

        let mut text = Text::new(summary);
        text.set_font("AlibabaPuHui");
        text.set_scale(16.0);
        canvas.draw(
            &text,
            DrawParam::default()
                .dest([18.0, 18.0])
                .color(Color::from_rgb(240, 240, 240)),
        );

        Ok(())
    }
}

impl EventHandler for MapViewer {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(16, 16, 16));

        self.draw_back_layer(ctx, &mut canvas)?;
        self.draw_middle_layer(ctx, &mut canvas)?;
        self.draw_front_layer(ctx, &mut canvas)?;

        self.draw_cell_flags(ctx, &mut canvas)?;
        self.draw_tile_grid(ctx, &mut canvas)?;
        self.draw_hover_tooltip(ctx, &mut canvas)?;
       // self.draw_ui_overlay(ctx, &mut canvas)?;

        canvas.finish(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, _ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        use ggez::winit::keyboard::PhysicalKey;

        let step = 2; // 与 C# 客户端一致：两格为单位

        if let PhysicalKey::Code(code) = input.event.physical_key {
            match code {
                KeyCode::Escape => std::process::exit(0),
                // ⬅️ 向左看 - offset_x 减小,看到地图更左侧(更小x坐标)
                KeyCode::ArrowLeft | KeyCode::KeyA => {
                    self.offset_x -= step;
                    self.clamp_offsets();
                    println!("⬅️  视口左移: offset_x = {}", self.offset_x);
                }
                // ➡️ 向右看 - offset_x 增大,看到地图更右侧(更大x坐标)
                KeyCode::ArrowRight | KeyCode::KeyD => {
                    self.offset_x += step;
                    self.clamp_offsets();
                    println!("➡️  视口右移: offset_x = {}", self.offset_x);
                }
                // ⬆️ 向上看 - offset_y 减小,看到地图更上方(更小y坐标)
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.offset_y -= step;
                    self.clamp_offsets();
                    println!("⬆️  视口上移: offset_y = {}", self.offset_y);
                }
                // ⬇️ 向下看 - offset_y 增大,看到地图更下方(更大y坐标)
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.offset_y += step;
                    self.clamp_offsets();
                    println!("⬇️  视口下移: offset_y = {}", self.offset_y);
                }
                KeyCode::KeyT => {
                    self.show_tile_grid = !self.show_tile_grid;
                    println!("🔍 地图网格: {}", if self.show_tile_grid { "✅ 开启" } else { "❌ 关闭" });
                }
                KeyCode::KeyB => {
                    self.show_image_border = !self.show_image_border;
                    println!("🎨 纹理边框: {}", if self.show_image_border { "✅ 开启" } else { "❌ 关闭" });
                }
                KeyCode::KeyF => {
                    self.show_cell_flags = !self.show_cell_flags;
                    println!("�️  地表障碍: {}", if self.show_cell_flags { "✅ 高亮" } else { "❌ 隐藏" });
                }
                KeyCode::Digit1 => {
                    self.render_back = !self.render_back;
                    println!("🎨 Back层: {}", if self.render_back { "✅ 开启" } else { "❌ 关闭" });
                }
                KeyCode::Digit2 => {
                    self.render_middle = !self.render_middle;
                    println!("🎨 Middle层: {}", if self.render_middle { "✅ 开启" } else { "❌ 关闭" });
                }
                KeyCode::Digit3 => {
                    self.render_front = !self.render_front;
                    println!("🎨 Front层: {}", if self.render_front { "✅ 开启" } else { "❌ 关闭" });
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            self.dragging = true;
            self.last_mouse_pos = (x, y);
        }
        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) -> GameResult {
        if button == MouseButton::Left {
            self.dragging = false;
        }
        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.mouse_x = x;
        self.mouse_y = y;

        if self.dragging {
            // 🖱️ 鼠标向右拖 → delta_x > 0 → 应该看到地图更右侧 → offset_x 增大
            // 🖱️ 鼠标向左拖 → delta_x < 0 → 应该看到地图更左侧 → offset_x 减小
            let delta_x = x - self.last_mouse_pos.0;
            let delta_y = y - self.last_mouse_pos.1;
            self.last_mouse_pos = (x, y);

            // 累积拖拽距离(不需要取反了!)
            self.accumulated_drag_x += delta_x;
            self.accumulated_drag_y += delta_y;

            // 每拖拽 TILE_WIDTH 像素,offset 改变 1
            let step_x = (self.accumulated_drag_x / TILE_WIDTH as f32) as i32;
            let step_y = (self.accumulated_drag_y / TILE_HEIGHT as f32) as i32;

            if step_x != 0 || step_y != 0 {
                self.offset_x += step_x;
                self.offset_y += step_y;
                self.clamp_offsets();

                // 减去已处理的距离
                self.accumulated_drag_x -= step_x as f32 * TILE_WIDTH as f32;
                self.accumulated_drag_y -= step_y as f32 * TILE_HEIGHT as f32;
            }
        }

        Ok(())
    }
}

fn main() -> GameResult {
    let map_path = "Data/Map/0122.map";

    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer_complete", "MirX")
        .window_setup(WindowSetup::default().title("地图查看器 - 完整渲染"))
        .window_mode(WindowMode::default().dimensions(SCREEN_WIDTH, SCREEN_HEIGHT))
        .build()?;

    static FONT: &[u8] = include_bytes!("../resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");
    ctx.gfx
        .add_font("AlibabaPuHui", FontData::from_slice(FONT)?);
    ctx.gfx.window().set_ime_allowed(true);

    let viewer = MapViewer::new(map_path)?;
    event::run(ctx, event_loop, viewer)
}
