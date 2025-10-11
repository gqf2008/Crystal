//! 简化版 Type 100 地图查看器
//!
//! 尝试保持与 C# `GameScene.DrawFloor` 相同的绘制顺序与坐标体系，便于对比调试。

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Canvas, Color, DrawParam};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::input::mouse::MouseButton;
use ggez::{Context, ContextBuilder, GameResult};

use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};

/// 绘制视窗一次显示 25x25 个格子，适合中小型地图测试拖拽。
const VIEW_RANGE: i32 = 25;

/// Type 100 地图的基础瓦片尺寸。
const TILE_WIDTH: i32 = 48;
const TILE_HEIGHT: i32 = 32;

struct SimpleMapViewer {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
    // 不再需要本地libraries字段，使用全局LIBRARIES
    offset_x: i32,
    offset_y: i32,
    printed_debug_once: bool,
    dragging: bool,
    last_mouse_pos: (f32, f32),
    // 累积的鼠标拖拽偏移（像素），用于处理小范围移动
    accumulated_drag_x: f32,
    accumulated_drag_y: f32,
}

impl SimpleMapViewer {
    fn new(map_path: &str) -> GameResult<Self> {
        println!("📂 正在加载地图: {map_path}");

        let reader = MapReader::new(map_path)
            .map_err(|err| ggez::GameError::ResourceLoadError(err.to_string()))?;

        println!("✅ 地图尺寸: {} x {}", reader.width, reader.height);

        // 计算可移动范围
        let max_offset_x = (reader.width - VIEW_RANGE).max(0);
        let max_offset_y = (reader.height - VIEW_RANGE).max(0);
        println!(
            "🎯 视窗: {}x{}, 可移动范围: {}x{}",
            VIEW_RANGE, VIEW_RANGE, max_offset_x, max_offset_y
        );

        // 🔧 使用全局 LIBRARIES 初始化所有地图库
        println!("📚 正在初始化地图库...");
        initialize_all_libraries("Data").map_err(|err| {
            ggez::GameError::ResourceLoadError(format!("初始化地图库失败: {}", err))
        })?;
        println!("✅ 地图库初始化完成");

        Ok(Self {
            cells: reader.map_cells,
            width: reader.width,
            height: reader.height,
            offset_x: 0,
            offset_y: 0,
            printed_debug_once: false,
            dragging: false,
            last_mouse_pos: (0.0, 0.0),
            accumulated_drag_x: 0.0,
            accumulated_drag_y: 0.0,
        })
    }

    #[inline]
    fn cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        (x >= 0 && x < self.width && y >= 0 && y < self.height)
            .then_some(&self.cells[x as usize][y as usize])
    }

    fn draw_back_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let mut draw_count = 0;
        let mut skip_count = 0; // 统计跳过的格子

        // ✅ 恢复C#原版逻辑：Back层只绘制偶数行列（96x64瓦片覆盖2x2格子）
        canvas.set_blend_mode(graphics::BlendMode::REPLACE);

        // 🔧 扩展绘制范围，避免边缘黑边（96x64瓦片需要额外空间）
        // 右下边缘需要更大扩展（+4），确保最右下角的瓦片完整渲染
        let start_x = (self.offset_x - 2).max(0);
        let start_y = (self.offset_y - 2).max(0);
        let end_x = (self.offset_x + VIEW_RANGE + 4).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE + 4).min(self.height);

        // ✅ C#原版：只绘制偶数行列（减少绘制量，96x64瓦片自动覆盖）
        for map_y in start_y..end_y {
            // 跳过奇数行（不检查<=0，允许绘制第0行）
            if map_y % 2 != 0 {
                continue;
            }

            for map_x in start_x..end_x {
                // 跳过奇数列（不检查<=0，允许绘制第0列）
                if map_x % 2 != 0 {
                    continue;
                }

                let Some(cell) = self.cell(map_x, map_y) else {
                    continue;
                };
                if cell.back_image <= 0 || cell.back_index < 0 {
                    skip_count += 1; // 计数空格子
                    continue;
                }

                let lib_index = cell.back_index;
                let Some(lib_arc) = get_map_library(lib_index) else {
                    skip_count += 1; // 计数缺失库
                    continue;
                };

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

                // C#中Draw函数直接使用坐标，不加图像偏移量
                let screen_x = ((map_x - self.offset_x) * TILE_WIDTH) as f32;
                let screen_y = ((map_y - self.offset_y) * TILE_HEIGHT) as f32;

                if let Ok(image_info) = lib.get_or_create_texture(ctx, image_index) {
                    if let Some(ref texture) = image_info.image {
                        canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                        draw_count += 1;

                        // 🔧 绘制遮罩层 (Mask)
                         if image_info.has_mask {
                            if let Some(ref mask_texture) = image_info.mask_image {
                                canvas.draw(
                                    mask_texture,
                                    DrawParam::default().dest([screen_x, screen_y]),
                                );
                            }
                        }
                    }
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Back 层绘制数量: {draw_count}, 跳过: {skip_count}");
        }

        Ok(())
    }

    fn draw_middle_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let mut draw_count = 0;

        // 参考代码：Middle层使用REPLACE模式
        canvas.set_blend_mode(graphics::BlendMode::REPLACE);

        // 扩展绘制范围，避免边缘物体被裁剪（右下边缘+4）
        let start_x = (self.offset_x - 2).max(0);
        let start_y = (self.offset_y - 2).max(0);
        let end_x = (self.offset_x + VIEW_RANGE + 4).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE + 4).min(self.height);

        for map_y in start_y..end_y {
            for map_x in start_x..end_x {
                let Some(cell) = self.cell(map_x, map_y) else {
                    continue;
                };

                if cell.middle_image <= 0 || cell.middle_index < 0 {
                    continue;
                }

                let lib_index = cell.middle_index;
                let Some(lib_arc) = get_map_library(lib_index) else {
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
                let valid_size = (info.width == TILE_WIDTH as i16
                    && info.height == TILE_HEIGHT as i16)
                    || (info.width == (TILE_WIDTH * 2) as i16
                        && info.height == (TILE_HEIGHT * 2) as i16);
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

                        //🔧 绘制遮罩层 (Mask)
                        if image_info.has_mask {
                            if let Some(ref mask_texture) = image_info.mask_image {
                                canvas.draw(
                                    mask_texture,
                                    DrawParam::default().dest([screen_x, screen_y]),
                                );
                            }
                        }
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
        let mut skip_no_lib = 0;
        let mut skip_no_info = 0;
        let mut skip_no_texture = 0;

        // 参考代码：Front层使用ALPHA模式（正常alpha混合）
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);

        // 扩展绘制范围，高大物体（树木、建筑）需要更大的范围
        let start_x = (self.offset_x - 5).max(0);
        let start_y = (self.offset_y - 10).max(0); // 向上扩展更多，因为建筑物很高
        let end_x = (self.offset_x + VIEW_RANGE + 8).min(self.width); // 右侧扩展更多
        let end_y = (self.offset_y + VIEW_RANGE + 8).min(self.height); // 下方扩展更多

        for map_y in start_y..end_y {
            for map_x in start_x..end_x {
                let Some(cell) = self.cell(map_x, map_y) else {
                    continue;
                };

                // 低位 15 bits 才是真正的图片索引。
                let front_image = cell.front_image & 0x7FFF;
                if front_image <= 0 || cell.front_index < 0 {
                    continue;
                }

                let lib_index = cell.front_index;
                let Some(lib_arc) = get_map_library(lib_index) else {
                    skip_no_lib += 1;
                    if !self.printed_debug_once && skip_no_lib <= 3 {
                        println!("  ⚠️ Front ({map_x},{map_y}) 库[{lib_index}]不存在");
                    }
                    continue;
                };

                let image_index = (front_image as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => {
                        skip_no_info += 1;
                        if !self.printed_debug_once && skip_no_info <= 3 {
                            println!("  ⚠️ Front ({map_x},{map_y}) 库[{lib_index}] 图像[{image_index}]不存在");
                        }
                        continue;
                    }
                };

                if !self.printed_debug_once && draw_count < 5 {
                    println!(
                        "  🟥 Front ({map_x},{map_y}) idx={} size={}x{} offset=({}, {})",
                        image_index, info.width, info.height, info.x, info.y
                    );
                }

                // 🔧 Front层：需要减去图像高度，让建筑物"站"在格子上
                // 原因：48x32格子只是地面位置，高大建筑需要向上延伸
                let draw_x = ((map_x - self.offset_x) * TILE_WIDTH) as f32;
                let draw_y = ((map_y - self.offset_y) * TILE_HEIGHT) as f32;
                let screen_x = draw_x;
                let screen_y = draw_y - info.height as f32 + TILE_HEIGHT as f32;

                if let Ok(image_info) = lib.get_or_create_texture(ctx, image_index) {
                    if let Some(ref texture) = image_info.image {
                        canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
                        draw_count += 1;

                        // 🔧 绘制遮罩层 (Mask)
                       // C#: if (mi.HasMask) { DXManager.Draw(mi.MaskImage, ..., Tint); }
                        if image_info.has_mask {
                            if let Some(ref mask_texture) = image_info.mask_image {
                                if !self.printed_debug_once && draw_count <= 5 {
                                    println!(
                                        "    🎭 绘制Mask层 ({map_x},{map_y}) idx={image_index}"
                                    );
                                }
                                canvas.draw(
                                    mask_texture,
                                    DrawParam::default().dest([screen_x, screen_y]),
                                );
                            }
                        }
                    } else {
                        skip_no_texture += 1;
                        if !self.printed_debug_once && skip_no_texture <= 3 {
                            println!("  ⚠️ Front ({map_x},{map_y}) 库[{lib_index}] 图像[{image_index}] 纹理创建失败");
                        }
                    }
                } else {
                    skip_no_texture += 1;
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Front 层绘制数量: {draw_count}");
            if skip_no_lib > 0 {
                println!("  ⚠️ 跳过（库不存在）: {skip_no_lib}");
            }
            if skip_no_info > 0 {
                println!("  ⚠️ 跳过（图像不存在）: {skip_no_info}");
            }
            if skip_no_texture > 0 {
                println!("  ⚠️ 跳过（纹理失败）: {skip_no_texture}");
            }
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
            // 🔧 计算实际可移动的最大值（如果地图比视窗小，则为0）
            let max_offset_x = (self.width - VIEW_RANGE).max(0);
            let max_offset_y = (self.height - VIEW_RANGE).max(0);

            match code {
                KeyCode::ArrowLeft => self.offset_x = (self.offset_x - step).max(0),
                KeyCode::ArrowRight => self.offset_x = (self.offset_x + step).min(max_offset_x),
                KeyCode::ArrowUp => self.offset_y = (self.offset_y - step).max(0),
                KeyCode::ArrowDown => self.offset_y = (self.offset_y + step).min(max_offset_y),
                _ => {}
            }

            self.printed_debug_once = false;
            println!("🧭 视口偏移 -> ({}, {})", self.offset_x, self.offset_y);
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
        if self.dragging {
            let delta_x = x - self.last_mouse_pos.0;
            let delta_y = y - self.last_mouse_pos.1;

            // 🔧 立即更新鼠标位置
            self.last_mouse_pos = (x, y);

            // ✅ 优化：降低移动阈值，提高灵敏度（从48像素降为24像素）
            self.accumulated_drag_x += -delta_x;
            self.accumulated_drag_y += -delta_y;

            // 当累积偏移超过半个格子时就移动（更流畅）
            let step_x = (self.accumulated_drag_x / 24.0) as i32;
            let step_y = (self.accumulated_drag_y / 16.0) as i32;

            if step_x != 0 || step_y != 0 {
                let max_offset_x = (self.width - VIEW_RANGE).max(0);
                let max_offset_y = (self.height - VIEW_RANGE).max(0);
                self.offset_x = (self.offset_x + step_x).clamp(0, max_offset_x);
                self.offset_y = (self.offset_y + step_y).clamp(0, max_offset_y);

                // 减去已经移动的格子数对应的像素
                self.accumulated_drag_x -= step_x as f32 * 24.0;
                self.accumulated_drag_y -= step_y as f32 * 16.0;

                self.printed_debug_once = false;
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // ✅ 使用深灰色背景，接近原版效果（比纯黑更柔和）
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(16, 16, 16));

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
}

fn main() -> GameResult {
    println!("\n🔧 Simple Type 100 地图查看器，方向键或拖拽移动视角\n");

    let (ctx, event_loop) = ContextBuilder::new("simple_map_viewer", "Mir2")
        .window_setup(WindowSetup::default().title("Simple Map Viewer"))
        .window_mode(WindowMode::default().dimensions(1280.0, 960.0))
        .build()?;

    let state = SimpleMapViewer::new("Map/0122.map")?;
    event::run(ctx, event_loop, state)
}
