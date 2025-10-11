//! 简化版 Type 100 地图查看器
//!
//! 尝试保持与 C# `GameScene.DrawFloor` 相同的绘制顺序与坐标体系，便于对比调试。

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Canvas, Color};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::input::mouse::MouseButton;
use ggez::{Context, ContextBuilder, GameResult};

use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};

/// Type 100 地图的基础瓦片尺寸。
const TILE_WIDTH: i32 = 48;
const TILE_HEIGHT: i32 = 32;

/// 窗口尺寸 (放大到1920x1080以便更清晰地观察)
const SCREEN_WIDTH: f32 = 1920.0;
const SCREEN_HEIGHT: f32 = 1080.0;

/// 🔧 根据C#原版计算视野参数,适配1920x1080窗口
/// C# GameScene.MapControl:
/// OffSetX = ScreenWidth / 2 / CellWidth = 1920 / 2 / 48 = 20
/// OffSetY = ScreenHeight / 2 / CellHeight - 1 = 1080 / 2 / 32 - 1 = 15.875 ≈ 16
/// ViewRangeX = OffSetX + 6 = 20 + 6 = 26
/// ViewRangeY = OffSetY + 6 = 16 + 6 = 22
// ⚠️ 由于Back层只绘制偶数坐标,offset也必须是偶数以保持对齐
const OFFSET_X: i32 = ((SCREEN_WIDTH as i32 / 2) / TILE_WIDTH) & !1;        // 1920/2/48 = 20 (偶数)
const OFFSET_Y: i32 = ((SCREEN_HEIGHT as i32 / 2) / TILE_HEIGHT - 1) & !1; // 1080/2/32-1 = 16 (偶数)
// 🔧 VIEW_RANGE需要足够大以覆盖整个屏幕+额外缓冲
// 屏幕宽度1920需要至少40格(1920/48),高度1080需要至少34格(1080/32)
// 从中心offset=20向左右扩展,需要至少20+缓冲格才能覆盖到屏幕边缘
const VIEW_RANGE_X: i32 = 26;  // 20+26=46,足够覆盖屏幕宽度并有余量
const VIEW_RANGE_Y: i32 = 22;  // 16+22=38,足够覆盖屏幕高度并有余量

/// 绘制视窗 (横向)
const VIEW_RANGE: i32 = VIEW_RANGE_X;

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
    // 🔍 调试网格开关(按G键切换)
    show_grid: bool,
    // 🖱️ 鼠标当前位置(用于显示悬停信息)
    mouse_x: f32,
    mouse_y: f32,
}

impl SimpleMapViewer {
    fn new(map_path: &str) -> GameResult<Self> {
        println!("📂 正在加载地图: {map_path}");

        let reader = MapReader::new(map_path)
            .map_err(|err| ggez::GameError::ResourceLoadError(err.to_string()))?;

        println!("✅ 地图尺寸: {} x {}", reader.width, reader.height);

        // 🔧 计算可移动范围：确保视窗不超出地图边界
        // 最大偏移 = 地图大小 - 视野范围（不能让视窗右下角超出地图）
        let max_offset_x = (reader.width - VIEW_RANGE_X).max(0);
        let max_offset_y = (reader.height - VIEW_RANGE_Y).max(0);
        println!(
            "🎯 视野范围: X={}(左右各{}格) Y={}(上下{}格), 最大偏移: offset_x≤{}, offset_y≤{}",
            VIEW_RANGE_X, OFFSET_X, VIEW_RANGE_Y, OFFSET_Y, max_offset_x, max_offset_y
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
            // 🔧 初始offset设置为(OFFSET_X, OFFSET_Y),使视窗左上角对齐地图(0,0)
            // 这样地图(0,0)就会出现在屏幕左上角,而不是中心
            offset_x: OFFSET_X,
            offset_y: OFFSET_Y,
            printed_debug_once: false,
            dragging: false,
            last_mouse_pos: (0.0, 0.0),
            accumulated_drag_x: 0.0,
            accumulated_drag_y: 0.0,
            show_grid: true, // 🔍 默认开启网格
            mouse_x: 0.0,
            mouse_y: 0.0,
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

        // 🔧 使用C#原版的视野范围计算
        // C#: for (y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY; y++)
        // C#: if (x <= 0 || x % 2 == 1) continue; - C#跳过x≤0是因为从1开始的地图边界
        // offset是摄像机中心,需要向左上和右下各扩展VIEW_RANGE
        let start_x = (self.offset_x - VIEW_RANGE_X).max(0); // 从0开始,0是偶数可以绘制
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0); // 从0开始,0是偶数可以绘制
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 1).min(self.height);

        // ✅ C#原版：只绘制偶数行列（减少绘制量,96x64瓦片自动覆盖）
        // C#: if (y <= 0 || y % 2 == 1) continue; - 跳过负数和奇数坐标
        // C#: if (x <= 0 || x % 2 == 1) continue;
        // 注意:0是偶数,应该绘制!
        for map_y in start_y..end_y {
            // 只跳过奇数行(0是偶数,不跳过)
            if map_y % 2 != 0 {
                continue;
            }

            for map_x in start_x..end_x {
                // 只跳过奇数列(0是偶数,不跳过)
                if map_x % 2 != 0 {
                    skip_count += 1;
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

                // 🔧 C#原版屏幕坐标计算公式:
                // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
                // drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
                // 注意X坐标有额外的 -OffSetX 调整！
                let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X) as f32;
                let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;

                // C#: Libraries.MapLibs[lib_index].Draw(image_index, screen_x, screen_y);
                if lib.draw(ctx, canvas, image_index, screen_x, screen_y).is_ok() {
                    draw_count += 1;
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

        // 🔧 C#原版:Middle层向下多扩展5格
        // C#: for (y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY + 5; y++)
        // C#: if (y <= 0) continue; if (x < 0) continue;
        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 6).min(self.height); // +5 向下扩展 (+6是因为包含边界)

        // C#原版逻辑: 跳过 y<=0 (但x可以从0开始)
        for map_y in start_y..end_y {
            if map_y == 0 {
                continue; // C#: if (y <= 0) continue;
            }
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

                // 🔧 C#原版屏幕坐标计算公式（与Back层相同）
                // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
                let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X) as f32;
                let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;

                // C#: Libraries.MapLibs[lib_index].Draw(image_index, screen_x, screen_y);
                if lib.draw(ctx, canvas, image_index, screen_x, screen_y).is_ok() {
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
        let mut skip_no_lib = 0;
        let mut skip_no_info = 0;
        let mut skip_no_texture = 0;

        // 参考代码：Front层使用ALPHA模式（正常alpha混合）
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);

        // 🔧 C#原版：Front层与Middle层相同，向下多扩展5格
        // C#: for (y = User.Movement.Y - ViewRangeY; y <= User.Movement.Y + ViewRangeY + 5; y++)
        // C#: if (y <= 0) continue; if (x < 0) continue;
        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 6).min(self.height); // +5 向下扩展 (+6是因为包含边界)

        // C#原版逻辑: 跳过 y<=0 (但x可以从0开始)
        for map_y in start_y..end_y {
            if map_y == 0 {
                continue; // C#: if (y <= 0) continue;
            }
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

                // 🔧 C#原版屏幕坐标计算（与Back/Middle层相同）
                // drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
                // Front层需要额外减去图像高度，让建筑物"站"在格子上
                let draw_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X) as f32;
                let draw_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;
                let screen_x = draw_x;
                let screen_y = draw_y - info.height as f32 + TILE_HEIGHT as f32;

                // 🔧 使用 MLibrary 的 draw_tinted 方法绘制（支持 Mask 层）
                // C#: Libraries.MapLibs[lib_index].DrawTinted(image_index, point, Color.White, Tint, false);
                // 这里使用 WHITE 作为主色，WHITE 作为 Tint（可以根据光照系统调整）
                if lib
                    .draw_tinted(
                        ctx,
                        canvas,
                        image_index,
                        screen_x,
                        screen_y,
                        Color::WHITE,
                        Color::WHITE, // Tint颜色（光照效果）
                        false,        // offset = false
                    )
                    .is_ok()
                {
                    draw_count += 1;
                    if !self.printed_debug_once && draw_count <= 5 {
                        // 检查是否有Mask层
                        if let Ok(img_info) = lib.get_or_create_texture(ctx, image_index) {
                            if img_info.has_mask {
                                println!("    🎭 已绘制Mask层 ({map_x},{map_y}) idx={image_index}");
                            }
                        }
                    }
                } else {
                    skip_no_texture += 1;
                    if !self.printed_debug_once && skip_no_texture <= 3 {
                        println!("  ⚠️ Front ({map_x},{map_y}) 库[{lib_index}] 图像[{image_index}] 绘制失败");
                    }
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

    /// 🔍 绘制调试网格：显示瓦片坐标和边界
    fn draw_debug_grid(&self, ctx: &Context, canvas: &mut Canvas) -> GameResult {
        use ggez::graphics::{Mesh, DrawMode, Color};
        
        // 绘制范围：屏幕可见区域
        let start_x = (self.offset_x - VIEW_RANGE_X).max(2);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(2);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 1).min(self.height);

        // 网格颜色：半透明绿色
        let grid_color = Color::from_rgba(0, 255, 0, 100);
        
        // 绘制垂直线和水平线
        for map_x in start_x..=end_x {
            let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X) as f32;
            
            // 垂直线
            let line = Mesh::new_line(
                ctx,
                &[
                    [screen_x, 0.0],
                    [screen_x, SCREEN_HEIGHT as f32],
                ],
                1.0,
                grid_color,
            )?;
            canvas.draw(&line, graphics::DrawParam::default());
        }

        for map_y in start_y..=end_y {
            let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;
            
            // 水平线
            let line = Mesh::new_line(
                ctx,
                &[
                    [0.0, screen_y],
                    [SCREEN_WIDTH as f32, screen_y],
                ],
                1.0,
                grid_color,
            )?;
            canvas.draw(&line, graphics::DrawParam::default());
        }
        
        // 🔍 在每个格子中心绘制坐标文本（每4格显示一次，避免太密集）
        for map_x in (start_x..=end_x).step_by(4) {
            for map_y in (start_y..=end_y).step_by(4) {
                let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X) as f32;
                let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;
                let mut text = graphics::Text::new(format!("({},{})", map_x, map_y));
                text.set_scale(12.0);
                canvas.draw(
                    &text,
                    graphics::DrawParam::default()
                        .dest([screen_x + 2.0, screen_y + 2.0])
                        .color(Color::YELLOW),
                );
            }
        }

        // 🎯 注释掉红色方框标记,避免视觉干扰
        // 如需显示offset位置,应该在屏幕中心绘制,而不是地图坐标
        // let center_x = (SCREEN_WIDTH / 2.0 - TILE_WIDTH as f32 / 2.0);
        // let center_y = (SCREEN_HEIGHT / 2.0 - TILE_HEIGHT as f32 / 2.0);
        // let center_rect = Mesh::new_rectangle(
        //     ctx,
        //     DrawMode::stroke(2.0),
        //     graphics::Rect::new(center_x, center_y, TILE_WIDTH as f32, TILE_HEIGHT as f32),
        //     Color::RED,
        // )?;
        // canvas.draw(&center_rect, graphics::DrawParam::default());

        Ok(())
    }

    /// 🖱️ 绘制鼠标悬停信息:显示当前坐标和资源编号
    fn draw_hover_info(&self, ctx: &Context, canvas: &mut Canvas) -> GameResult {
        use ggez::graphics::{Text, DrawParam, PxScale};
        
        // 计算鼠标对应的地图坐标
        // 屏幕坐标 → 地图坐标的逆向计算
        // screen_x = (map_x - offset_x + OFFSET_X) * TILE_WIDTH - OFFSET_X
        // → map_x = (screen_x + OFFSET_X) / TILE_WIDTH + offset_x - OFFSET_X
        let map_x = ((self.mouse_x + OFFSET_X as f32) / TILE_WIDTH as f32).floor() as i32 + self.offset_x - OFFSET_X;
        let map_y = (self.mouse_y / TILE_HEIGHT as f32).floor() as i32 + self.offset_y - OFFSET_Y;

        // 检查坐标是否在地图范围内
        if map_x < 0 || map_x >= self.width || map_y < 0 || map_y >= self.height {
            return Ok(());
        }

        // 获取该位置的单元格信息
        let cell = &self.cells[map_x as usize][map_y as usize];

        // 构建信息文本
        let mut info_lines = vec![
            format!("📍 地图坐标: ({}, {})", map_x, map_y),
        ];

        // Back层信息
        if cell.back_index > 0 && cell.back_image > 0 {
            info_lines.push(format!("⬜ Back: 库[{}] 图像[{}]", cell.back_index, cell.back_image));
        } else {
            info_lines.push("⬜ Back: 无".to_string());
        }

        // Middle层信息
        if cell.middle_index > 0 && cell.middle_image > 0 {
            info_lines.push(format!("🟦 Middle: 库[{}] 图像[{}]", cell.middle_index, cell.middle_image));
        } else {
            info_lines.push("🟦 Middle: 无".to_string());
        }

        // Front层信息
        if cell.front_index > 0 && cell.front_image > 0 {
            info_lines.push(format!("🟥 Front: 库[{}] 图像[{}]", cell.front_index, cell.front_image));
        } else {
            info_lines.push("🟥 Front: 无".to_string());
        }

        // 绘制半透明背景框
        let box_width = 280.0;
        let box_height = (info_lines.len() as f32 * 22.0) + 10.0;
        let box_x = self.mouse_x + 15.0;
        let box_y = self.mouse_y + 15.0;

        use ggez::graphics::{Mesh, DrawMode, Rect};
        let bg_rect = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(box_x, box_y, box_width, box_height),
            Color::from_rgba(0, 0, 0, 200),
        )?;
        canvas.draw(&bg_rect, DrawParam::default());

        // 绘制边框
        let border_rect = Mesh::new_rectangle(
            ctx,
            DrawMode::stroke(2.0),
            Rect::new(box_x, box_y, box_width, box_height),
            Color::from_rgb(100, 200, 255),
        )?;
        canvas.draw(&border_rect, DrawParam::default());

        // 绘制文本
        for (i, line) in info_lines.iter().enumerate() {
            let mut text = Text::new(line);
            text.set_scale(PxScale::from(16.0));
            canvas.draw(
                &text,
                DrawParam::default()
                    .dest([box_x + 5.0, box_y + 5.0 + (i as f32 * 22.0)])
                    .color(Color::WHITE),
            );
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

        let step = 2; // 与 C# 客户端一致,两格为单位移动视口。

        if let PhysicalKey::Code(code) = input.event.physical_key {
            // 🔧 使用正确的视野范围计算边界
            // 最小offset = OFFSET_X/Y (防止地图(0,0)移到屏幕中间导致左上黑边)
            // 最大offset = 地图大小 - VIEW_RANGE (防止右下超出地图)
            // ⚠️ max_offset也必须是偶数,否则边界处会变成奇数!
            let max_offset_x = ((self.width - VIEW_RANGE_X).max(OFFSET_X)) & !1;
            let max_offset_y = ((self.height - VIEW_RANGE_Y).max(OFFSET_Y)) & !1;

            match code {
                KeyCode::ArrowLeft => {
                    self.offset_x = ((self.offset_x - step).max(OFFSET_X)) & !1;
                }
                KeyCode::ArrowRight => {
                    self.offset_x = ((self.offset_x + step).min(max_offset_x)) & !1;
                }
                KeyCode::ArrowUp => {
                    self.offset_y = ((self.offset_y - step).max(OFFSET_Y)) & !1;
                }
                KeyCode::ArrowDown => {
                    self.offset_y = ((self.offset_y + step).min(max_offset_y)) & !1;
                }
                KeyCode::KeyG => {
                    // 🔍 按G键切换网格显示
                    self.show_grid = !self.show_grid;
                    println!("🔍 调试网格: {}", if self.show_grid { "开启" } else { "关闭" });
                }
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
        // 🖱️ 更新鼠标位置(用于悬停显示)
        self.mouse_x = x;
        self.mouse_y = y;
        
        if self.dragging {
            let delta_x = x - self.last_mouse_pos.0;
            let delta_y = y - self.last_mouse_pos.1;

            // 🔧 立即更新鼠标位置
            self.last_mouse_pos = (x, y);

            // 🎮 更丝滑的拖动:每移动8像素就响应(之前24像素)
            self.accumulated_drag_x += -delta_x;
            self.accumulated_drag_y += -delta_y;

            // 降低阈值提升响应速度,同时保持2格对齐(偶数)
            let step_x = ((self.accumulated_drag_x / 8.0) as i32) & !1;  // 强制偶数步进
            let step_y = ((self.accumulated_drag_y / 8.0) as i32) & !1;

            if step_x != 0 || step_y != 0 {
                // 🔧 边界也必须是偶数
                let max_offset_x = ((self.width - VIEW_RANGE_X).max(OFFSET_X)) & !1;
                let max_offset_y = ((self.height - VIEW_RANGE_Y).max(OFFSET_Y)) & !1;
                
                // 先计算新位置,再偶数对齐,最后clamp到边界
                self.offset_x = ((self.offset_x + step_x).clamp(OFFSET_X, max_offset_x)) & !1;
                self.offset_y = ((self.offset_y + step_y).clamp(OFFSET_Y, max_offset_y)) & !1;

                // 减去已经移动的格子数对应的像素
                self.accumulated_drag_x -= step_x as f32 * 8.0;
                self.accumulated_drag_y -= step_y as f32 * 8.0;

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

        // 参考代码:Back层绘制两次,先奇数后偶数,使用REPLACE模式
        self.draw_back_layer(ctx, &mut canvas)?;
        self.draw_middle_layer(ctx, &mut canvas)?;
        self.draw_front_layer(ctx, &mut canvas)?;

        // 🔍 绘制调试网格(按G键切换)
        if self.show_grid {
            self.draw_debug_grid(ctx, &mut canvas)?;
        }

        // 🖱️ 绘制鼠标悬停信息
        self.draw_hover_info(ctx, &mut canvas)?;

        self.printed_debug_once = true;
        canvas.finish(ctx)?;
        Ok(())
    }
}

fn main() -> GameResult {
    println!("\n🔧 Simple Type 100 地图查看器，方向键或拖拽移动视角\n");

    let (ctx, event_loop) = ContextBuilder::new("simple_map_viewer", "Mir2")
        .window_setup(WindowSetup::default().title("Simple Map Viewer"))
        .window_mode(WindowMode::default().dimensions(SCREEN_WIDTH, SCREEN_HEIGHT))
        .build()?;

    let state = SimpleMapViewer::new("Map/0.map")?;
    event::run(ctx, event_loop, state)
}
