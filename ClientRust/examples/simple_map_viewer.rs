//! 简化版 Type 100 地图查看器
//!
//! 尝试保持与 C# `GameScene.DrawFloor` 相同的绘制顺序与坐标体系，便于对比调试。

use std::sync::{Arc, Mutex};

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::input::mouse::MouseButton;
use ggez::graphics::{self, Canvas, Color, DrawParam};
use ggez::{Context, ContextBuilder, GameResult};

use mir2_client::graphics::mlibrary::MLibrary;
use mir2_client::objects::{CellInfo, MapReader};

/// 绘制视窗一次显示 50x50 个格子，便于与 C# 客户端截图对比。
const VIEW_RANGE: i32 = 50;

/// Type 100 地图的基础瓦片尺寸。
const TILE_WIDTH: i32 = 48;
const TILE_HEIGHT: i32 = 32;

struct SimpleMapViewer {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
    libs: Vec<Option<Arc<Mutex<MLibrary>>>>,
    offset_x: i32,
    offset_y: i32,
    printed_debug_once: bool,
    dragging: bool,
    last_mouse_pos: (f32, f32),
}

impl SimpleMapViewer {
    fn new(map_path: &str) -> GameResult<Self> {
        println!("📂 正在加载地图: {map_path}");

        let reader = MapReader::new(map_path)
            .map_err(|err| ggez::GameError::ResourceLoadError(err.to_string()))?;

        println!("✅ 地图尺寸: {} x {}", reader.width, reader.height);

        // 预加载常用库。为了避免缺库导致渲染失败，只加载最关键的 0、1、2。
        let mut libs: Vec<Option<Arc<Mutex<MLibrary>>>> = vec![None; 400];
        let lib_configs = [
            (0, "Data/Map/WemadeMir2/Tiles"),
            // SmTiles 太小，直接复用 Tiles 以便观察。
            (1, "Data/Map/WemadeMir2/Tiles"),
            (2, "Data/Map/WemadeMir2/Objects"),
        ];

        for (lib_index, path) in lib_configs {
            match MLibrary::open(path) {
                Ok(lib) => {
                    println!("  ✅ 库[{lib_index}] -> {path}");
                    libs[lib_index] = Some(Arc::new(Mutex::new(lib)));
                }
                Err(err) => {
                    println!("  ⚠️ 库[{lib_index}] -> {path} 加载失败: {err}");
                }
            }
        }

        Ok(Self {
            cells: reader.map_cells,
            width: reader.width,
            height: reader.height,
            libs,
            offset_x: 0,
            offset_y: 0,
            printed_debug_once: false,
            dragging: false,
            last_mouse_pos: (0.0, 0.0),
        })
    }

    #[inline]
    fn cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        (x >= 0 && x < self.width && y >= 0 && y < self.height)
            .then_some(&self.cells[x as usize][y as usize])
    }

    fn draw_back_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let mut draw_count = 0;

        let start_x = if self.offset_x % 2 == 0 {
            self.offset_x
        } else {
            self.offset_x - 1
        };
        let start_y = if self.offset_y % 2 == 0 {
            self.offset_y
        } else {
            self.offset_y - 1
        };
        let end_x = (self.offset_x + VIEW_RANGE).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE).min(self.height);

        for map_y in (start_y..end_y).step_by(2) {
            for map_x in (start_x..end_x).step_by(2) {
                let Some(cell) = self.cell(map_x, map_y) else { continue };
                if cell.back_image <= 0 || cell.back_index < 0 {
                    continue;
                }

                let lib_index = cell.back_index as usize;
                let Some(lib_arc) = self.libs.get(lib_index).and_then(|o| o.as_ref()) else {
                    continue;
                };

                // C# 中 `BackImage` 最高位保存标记，需要去掉。
                let image_index = ((cell.back_image & 0x1FFF_FFFF) as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                if !self.printed_debug_once && draw_count < 3 {
                    println!(
                        "  ⬜ Back ({map_x},{map_y}) idx={} size={}x{} offset=({}, {})",
                        image_index, info.width, info.height, info.x, info.y
                    );
                }

                let screen_x = ((map_x - self.offset_x) * TILE_WIDTH + info.x as i32) as f32;
                let screen_y = ((map_y - self.offset_y) * TILE_HEIGHT + info.y as i32) as f32;

                if let Ok(texture) = lib.get_or_create_texture(ctx, image_index) {
                    canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                    draw_count += 1;
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Back 层绘制数量: {draw_count}");
        }

        Ok(())
    }

    fn draw_middle_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let mut draw_count = 0;

        for vy in 0..VIEW_RANGE {
            for vx in 0..VIEW_RANGE {
                let map_x = self.offset_x + vx;
                let map_y = self.offset_y + vy;
                let Some(cell) = self.cell(map_x, map_y) else { continue };

                if cell.middle_image <= 0 || cell.middle_index < 0 {
                    continue;
                }

                let lib_index = cell.middle_index as usize;
                let Some(lib_arc) = self.libs.get(lib_index).and_then(|o| o.as_ref()) else {
                    continue;
                };

                // Type 100 middle image 不包含额外标记，直接减一即可。
                let image_index = (cell.middle_image as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                if !self.printed_debug_once && draw_count < 5 {
                    println!(
                        "  🟦 Middle ({map_x},{map_y}) idx={} size={}x{} offset=({}, {})",
                        image_index, info.width, info.height, info.x, info.y
                    );
                }

                let screen_x = ((vx - vy) * (TILE_WIDTH / 2) as i32 + info.x as i32) as f32;
                let screen_y = ((vx + vy) * (TILE_HEIGHT / 2) as i32 + info.y as i32) as f32;

                if let Ok(texture) = lib.get_or_create_texture(ctx, image_index) {
                    canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                    draw_count += 1;
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Middle 层绘制数量: {draw_count}");
        }

        Ok(())
    }

    fn draw_front_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let mut draw_count = 0;

        for vy in 0..VIEW_RANGE {
            for vx in 0..VIEW_RANGE {
                let map_x = self.offset_x + vx;
                let map_y = self.offset_y + vy;
                let Some(cell) = self.cell(map_x, map_y) else { continue };

                // 低位 15 bits 才是真正的图片索引。
                let front_image = cell.front_image & 0x7FFF;
                if front_image <= 0 || cell.front_index < 0 {
                    continue;
                }

                let lib_index = cell.front_index as usize;
                let Some(lib_arc) = self.libs.get(lib_index).and_then(|o| o.as_ref()) else {
                    continue;
                };

                let image_index = (front_image as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                if !self.printed_debug_once && draw_count < 5 {
                    println!(
                        "  🟥 Front ({map_x},{map_y}) idx={} size={}x{} offset=({}, {})",
                        image_index, info.width, info.height, info.x, info.y
                    );
                }

                let screen_x = ((map_x - map_y) * (TILE_WIDTH / 2) as i32 + info.x as i32 - self.offset_x) as f32;
                let screen_y = ((map_x + map_y) * (TILE_HEIGHT / 2) as i32 + info.y as i32 - self.offset_y) as f32;
                
                if let Ok(texture) = lib.get_or_create_texture(ctx, image_index) {
                    canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                    draw_count += 1;
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Front 层绘制数量: {draw_count}");
        }

        Ok(())
    }
}

impl EventHandler for SimpleMapViewer {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        Ok(())
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        use ggez::winit::keyboard::PhysicalKey;

        let step = 2; // 与 C# 客户端一致，两格为单位移动视口。

        if let PhysicalKey::Code(code) = input.event.physical_key {
            match code {
                KeyCode::ArrowLeft => self.offset_x = (self.offset_x - step).max(0),
                KeyCode::ArrowRight => {
                    self.offset_x = (self.offset_x + step).min(self.width - VIEW_RANGE)
                }
                KeyCode::ArrowUp => self.offset_y = (self.offset_y - step).max(0),
                KeyCode::ArrowDown => {
                    self.offset_y = (self.offset_y + step).min(self.height - VIEW_RANGE)
                }
                _ => {}
            }

            self.printed_debug_once = false;
            println!("🧭 视口偏移 -> ({}, {})", self.offset_x, self.offset_y);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);

        if !self.printed_debug_once {
            println!(
                "\n🎨 绘制 offset=({}, {}), 视窗 {}x{}",
                self.offset_x, self.offset_y, VIEW_RANGE, VIEW_RANGE
            );
        }

        self.draw_back_layer(ctx, &mut canvas)?;
        self.draw_middle_layer(ctx, &mut canvas)?;
        self.draw_front_layer(ctx, &mut canvas)?;

        self.printed_debug_once = true;
        canvas.finish(ctx)?;
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
        if self.dragging {
            let delta_x = x - self.last_mouse_pos.0;
            let delta_y = y - self.last_mouse_pos.1;

            let step_x = (-delta_x / TILE_WIDTH as f32) as i32;
            let step_y = (-delta_y / TILE_HEIGHT as f32) as i32;

            if step_x != 0 || step_y != 0 {
                self.offset_x = (self.offset_x + step_x).clamp(0, self.width - VIEW_RANGE);
                self.offset_y = (self.offset_y + step_y).clamp(0, self.height - VIEW_RANGE);
                self.printed_debug_once = false;
            }

            self.last_mouse_pos = (x, y);
        }

        Ok(())
    }
}

fn main() -> GameResult {
    println!("\n🔧 Simple Type 100 地图查看器，方向键或拖拽移动视角\n");

    let (ctx, event_loop) = ContextBuilder::new("simple_map_viewer", "Mir2")
        .window_setup(WindowSetup::default().title("Simple Map Viewer"))
        .window_mode(WindowMode::default().dimensions(1280.0, 960.0))
        .build()?;

    let state = SimpleMapViewer::new("Map/0.map")?;
    event::run(ctx, event_loop, state)
}
