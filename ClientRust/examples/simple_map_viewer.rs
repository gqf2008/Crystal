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

        // 预加载常用库。加载更多库以避免缺失地砖
        let mut libs: Vec<Option<Arc<Mutex<MLibrary>>>> = vec![None; 400];
        let lib_configs = [
            (0, "Data/Map/WemadeMir2/Tiles"),
            (1, "Data/Map/WemadeMir2/SmTiles"),
            (2, "Data/Map/WemadeMir2/Objects"),
            (3, "Data/Map/WemadeMir2/Objects2"),
            (4, "Data/Map/WemadeMir2/Objects3"),
            (5, "Data/Map/WemadeMir2/Objects4"),
            (6, "Data/Map/WemadeMir2/Objects5"),
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

        // 扩展绘制范围，避免边缘出现黑边
        let start_x = (self.offset_x - 2).max(0);
        let start_y = (self.offset_y - 2).max(0);
        let end_x = (self.offset_x + VIEW_RANGE + 2).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE + 2).min(self.height);

        // 参考代码：先绘制奇数坐标（填充缝隙）
        for map_y in start_y..end_y {
            for map_x in start_x..end_x {
                // 只绘制奇数行或奇数列
                if map_x % 2 == 0 && map_y % 2 == 0 {
                    continue;
                }
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

                // C# 中: Libraries.MapLibs[index].Draw(index, drawX, drawY)
                // Back层直接在格子坐标绘制
                let screen_x = ((map_x - self.offset_x) * TILE_WIDTH) as f32;
                let screen_y = ((map_y - self.offset_y) * TILE_HEIGHT) as f32;

                if let Ok(image_info) = lib.get_or_create_texture(ctx, image_index) {
                    if let Some(ref texture) = image_info.image {
                        canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                        draw_count += 1;
                    }
                }
            }
        }

        // 参考代码：第二次绘制偶数坐标，使用REPLACE模式（不混合，直接覆盖）
        canvas.set_blend_mode(graphics::BlendMode::REPLACE);
        
        for map_y in start_y..end_y {
            // 只绘制偶数行
            if map_y % 2 != 0 {
                continue;
            }
            
            for map_x in start_x..end_x {
                // 只绘制偶数列
                if map_x % 2 != 0 {
                    continue;
                }

                let Some(cell) = self.cell(map_x, map_y) else { continue };
                if cell.back_image <= 0 || cell.back_index < 0 {
                    continue;
                }

                let lib_index = cell.back_index as usize;
                let Some(lib_arc) = self.libs.get(lib_index).and_then(|o| o.as_ref()) else {
                    continue;
                };

                let image_index = ((cell.back_image & 0x1FFF_FFFF) as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                let screen_x = ((map_x - self.offset_x) * TILE_WIDTH) as f32;
                let screen_y = ((map_y - self.offset_y) * TILE_HEIGHT) as f32;

                if let Ok(image_info) = lib.get_or_create_texture(ctx, image_index) {
                    if let Some(ref texture) = image_info.image {
                        canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                        draw_count += 1;
                    }
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

        // 参考代码：Middle层使用REPLACE模式
        canvas.set_blend_mode(graphics::BlendMode::REPLACE);

        // 扩展绘制范围，避免边缘物体被裁剪
        let start_x = (self.offset_x - 2).max(0);
        let start_y = (self.offset_y - 2).max(0);
        let end_x = (self.offset_x + VIEW_RANGE + 2).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE + 2).min(self.height);

        for map_y in start_y..end_y {
            for map_x in start_x..end_x {
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

                // 🔧 关键修复：Middle层尺寸过滤（参考C#代码）
                // 只允许单格 (48x32) 或双格 (96x64) 尺寸
                // 防止绘制错误的瓦片条带
                let valid_size = (info.width == TILE_WIDTH && info.height == TILE_HEIGHT) ||
                                 (info.width == TILE_WIDTH * 2 && info.height == TILE_HEIGHT * 2);
                if !valid_size {
                    continue;
                }

                if !self.printed_debug_once && draw_count < 5 {
                    println!(
                        "  🟦 Middle ({map_x},{map_y}) idx={} size={}x{} offset=({}, {})",
                        image_index, info.width, info.height, info.x, info.y
                    );
                }

                // C# 中: Libraries.MapLibs[index].Draw(index, drawX, drawY)
                // Middle层直接在格子坐标绘制
                let screen_x = ((map_x - self.offset_x) * TILE_WIDTH) as f32;
                let screen_y = ((map_y - self.offset_y) * TILE_HEIGHT) as f32;

                if let Ok(image_info) = lib.get_or_create_texture(ctx, image_index) {
                    if let Some(ref texture) = image_info.image {
                        canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                        draw_count += 1;
                    }
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

        // 参考代码：Front层使用ALPHA模式（正常alpha混合）
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);

        // 扩展绘制范围，高大物体（树木、建筑）需要更大的范围
        let start_x = (self.offset_x - 5).max(0);
        let start_y = (self.offset_y - 10).max(0);  // 向上扩展更多，因为建筑物很高
        let end_x = (self.offset_x + VIEW_RANGE + 5).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE + 5).min(self.height);

        for map_y in start_y..end_y {
            for map_x in start_x..end_x {
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

                // C# 中: Libraries.MapLibs[fileIndex].Draw(index, drawX, drawY - s.Height)
                // Front层需要减去图像高度（从底部向上绘制）
                let draw_x = ((map_x - self.offset_x) * TILE_WIDTH) as f32;
                let draw_y = ((map_y - self.offset_y) * TILE_HEIGHT) as f32;
                let screen_x = draw_x;
                let screen_y = draw_y - info.height as f32;
                
                if let Ok(image_info) = lib.get_or_create_texture(ctx, image_index) {
                    if let Some(ref texture) = image_info.image {
                        canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                        draw_count += 1;
                    }
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

        if !self.printed_debug_once {
            println!(
                "\n🎨 绘制 offset=({}, {}), 视窗 {}x{}",
                self.offset_x, self.offset_y, VIEW_RANGE, VIEW_RANGE
            );
        }

        // 参考代码：Back层绘制两次，先奇数后偶数，使用REPLACE模式
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
