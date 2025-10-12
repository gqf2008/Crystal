//! 简化版 Type 100 地图查看器
//!
//! 尝试保持与 C# `GameScene.DrawFloor` 相同的绘制顺序与坐标体系，便于对比调试。

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Canvas, Color, FontData};
use ggez::input::keyboard::{KeyCode, KeyInput};
use ggez::input::mouse::MouseButton;
use ggez::{Context, ContextBuilder, GameResult};

use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};

// 📚 MapLibs 索引说明 (与C#原版一致)
// MapLibs[400] 数组虽然定义了400个元素,但只有部分索引被初始化:
//   0-29, 90              - WeMade Mir2
//   100-119, 190          - Shanda Mir2  
//   200-213, 215-228, ... - WeMade Mir3 (每15个为一组,5组)
//   300-313, 315-328, ... - Shanda Mir3 (每15个为一组,5组)
// 
// ⚠️ 索引范围 214-299, 314-399 中有很多**未初始化的空位**
// 地图文件可能引用这些空索引(如257),这是**正常现象**,会被跳过绘制
// C#中 MapLibs[257] == null,不会崩溃,Rust返回None同样安全

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
const VIEW_RANGE_Y: i32 = 24;  // 16+24=40,增加2格以完全消除底部黑边

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
    // 🔍 调试显示开关
    show_tile_grid: bool,      // T键：地图网格（绿色坐标网格）
    show_image_border: bool,   // B键：图片边框（彩色瓦片边框）
    // 🎨 图层渲染开关
    render_back: bool,         // 1键：Back层
    render_middle: bool,       // 2键：Middle层
    render_front: bool,        // 3键：Front层
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

        // 🔧 计算可移动范围：让offset能移动到使屏幕完全覆盖地图
        // 绘制范围: [offset-VIEW_RANGE, offset+VIEW_RANGE+1)
        // 要让地图最右下角可见，max_offset应该是地图尺寸减去OFFSET
        // 这样当offset=max时，绘制end = max + VIEW_RANGE + 1 能覆盖到地图边界
        let max_offset_x = (reader.width - OFFSET_X).max(OFFSET_X);
        let max_offset_y = (reader.height - OFFSET_Y).max(OFFSET_Y);
        println!(
            "🎯 视野范围: 绘制区域 offset±[{},{}] 格 [总宽度≈{}格]",
            VIEW_RANGE_X, VIEW_RANGE_Y,
            VIEW_RANGE_X * 2 + 1
        );
        println!(
            "🎯 可移动范围: offset_x={}~{}, offset_y={}~{} (地图:{}x{})",
            OFFSET_X, max_offset_x, OFFSET_Y, max_offset_y,
            reader.width, reader.height
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
            show_tile_grid: true,      // 🔍 默认开启地图网格
            show_image_border: true,   // 🔍 默认开启图片边框
            render_back: true,         // 🎨 默认渲染Back层
            render_middle: true,       // 🎨 默认渲染Middle层
            render_front: true,        // 🎨 默认渲染Front层
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
        // 🎨 如果Back层被禁用，直接返回
        if !self.render_back {
            return Ok(());
        }
        
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
        // C#: if (y <= 0 || y % 2 == 1) continue; - 跳过 y<=0 和奇数坐标
        // C#: if (x <= 0 || x % 2 == 1) continue; - 跳过 x<=0 和奇数坐标
        // ⚠️ C#从1开始计数,所以y<=0表示跳过0和负数,即从y=2开始绘制
        for map_y in start_y..end_y {
            // 跳过 y<=0 (即 0, -1, -2...)
            if map_y <= 0 {
                continue;
            }
            // 只跳过奇数行
            if map_y % 2 != 0 {
                continue;
            }

            for map_x in start_x..end_x {
                // 跳过 x<=0 (即 0, -1, -2...)
                if map_x <= 0 {
                    skip_count += 1;
                    continue;
                }
                // 只跳过奇数列
                if map_x % 2 != 0 {
                    skip_count += 1;
                    continue;
                }

                let Some(cell) = self.cell(map_x, map_y) else {
                    continue;
                };
                if cell.back_image <= 0 || cell.back_index < 0 {
                    skip_count += 1; // 计数空格子
                    // 🔍 调试输出：检查黑色区域 (10-20, 38-46)
                    if !self.printed_debug_once && map_x >= 10 && map_x <= 20 && map_y >= 38 && map_y <= 46 {
                        println!("  ⚫ Back ({map_x},{map_y}) SKIPPED: back_image={}, back_index={}", 
                            cell.back_image, cell.back_index);
                    }
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

                // 🔧 Back层坐标计算:
                // C#从(2,2)开始绘制96x64瓦片,但应该覆盖(0,0)~(1,1)
                // 所以需要减去2格的偏移: -2 * TILE_WIDTH, -2 * TILE_HEIGHT
                let base_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH) as f32;
                let base_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;
                let screen_x = base_x - (TILE_WIDTH * 2) as f32 - info.x as f32;
                let screen_y = base_y - (TILE_HEIGHT * 2) as f32 - info.y as f32;

                // C#: Libraries.MapLibs[lib_index].Draw(image_index, screen_x, screen_y);
                if lib.draw(ctx, canvas, image_index, screen_x, screen_y).is_ok() {
                    draw_count += 1;
                    
                    // 🔴 调试：绘制红色边框显示Back层瓦片范围
                    if self.show_image_border {
                        use ggez::graphics::{Mesh, DrawMode, Rect};
                        let border = Mesh::new_rectangle(
                            ctx,
                            DrawMode::stroke(1.0),
                            Rect::new(screen_x, screen_y, info.width as f32, info.height as f32),
                            Color::from_rgb(255, 0, 0), // 红色
                        );
                        if let Ok(rect) = border {
                            canvas.draw(&rect, graphics::DrawParam::default());
                        }
                    }
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Back 层绘制数量: {draw_count}, 跳过: {skip_count}");
            println!("  🔍 视野范围: start=({},{}) end=({},{})", start_x, start_y, end_x, end_y);
        }

        Ok(())
    }

    fn draw_middle_layer(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 🎨 如果Middle层被禁用，直接返回
        if !self.render_middle {
            return Ok(());
        }
        
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

                // 🔧 简化坐标计算,与网格对齐(与Back层相同)
                let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH) as f32;
                let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;

                // C#: Libraries.MapLibs[lib_index].Draw(image_index, screen_x, screen_y);
                if lib.draw(ctx, canvas, image_index, screen_x, screen_y).is_ok() {
                    draw_count += 1;
                    
                    // 🟧 调试：绘制橙色边框显示Middle层瓦片范围
                    if self.show_image_border {
                        use ggez::graphics::{Mesh, DrawMode, Rect};
                        let border = Mesh::new_rectangle(
                            ctx,
                            DrawMode::stroke(1.0),
                            Rect::new(screen_x, screen_y, info.width as f32, info.height as f32),
                            Color::from_rgb(255, 165, 0), // 橙色
                        );
                        if let Ok(rect) = border {
                            canvas.draw(&rect, graphics::DrawParam::default());
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
        // 🎨 如果Front层被禁用，直接返回
        if !self.render_front {
            return Ok(());
        }
        
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
                    // 🔇 只在首次绘制时输出3条示例,避免刷屏
                    // 这是正常现象:地图文件可能引用未初始化的库索引(如257在214-299空隙中)
                    // C#中MapLibs[257]为null也会跳过,不会报错
                    continue;
                };

                let image_index = (front_image as usize).saturating_sub(1);

                let mut lib = lib_arc.lock().unwrap();
                let info = match lib.get_image_info(image_index) {
                    Ok(info) => info,
                    Err(_) => {
                        skip_no_info += 1;
                        continue;
                    }
                };

                if !self.printed_debug_once && draw_count < 5 {
                    println!(
                        "  🟥 Front ({map_x},{map_y}) idx={} size={}x{} offset=({}, {})",
                        image_index, info.width, info.height, info.x, info.y
                    );
                }

                // 🔧 简化坐标计算,与网格对齐(与Back/Middle层相同)
                // Front层需要额外减去图像高度，让建筑物"站"在格子上
                let draw_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH) as f32;
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
                    
                    // 🔵 调试：绘制蓝色边框显示Front层瓦片范围
                    if self.show_image_border {
                        use ggez::graphics::{Mesh, DrawMode, Rect};
                        let border = Mesh::new_rectangle(
                            ctx,
                            DrawMode::stroke(1.0),
                            Rect::new(screen_x, screen_y, info.width as f32, info.height as f32),
                            Color::from_rgb(0, 150, 255), // 蓝色
                        );
                        if let Ok(rect) = border {
                            canvas.draw(&rect, graphics::DrawParam::default());
                        }
                    }
                    
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
                }
            }
        }

        if !self.printed_debug_once {
            println!("  Front 层绘制数量: {draw_count}");
            if skip_no_lib > 0 {
                println!("  ℹ️ 跳过（库未初始化）: {skip_no_lib} - 正常现象,地图引用了MapLibs空索引");
            }
            if skip_no_info > 0 {
                println!("  ℹ️ 跳过（图像索引超出范围）: {skip_no_info}");
            }
            if skip_no_texture > 0 {
                println!("  ⚠️ 跳过（纹理解码失败）: {skip_no_texture} - 需检查.lib文件完整性");
            }
        }

        Ok(())
    }

    /// 🔍 绘制调试网格：显示瓦片坐标和边界
    fn draw_debug_grid(&self, ctx: &Context, canvas: &mut Canvas) -> GameResult {
        use ggez::graphics::{Mesh, DrawMode, Color};
        
        // 绘制范围：屏幕可见区域（从0开始，不跳过任何坐标）
        let start_x = (self.offset_x - VIEW_RANGE_X).max(0);
        let start_y = (self.offset_y - VIEW_RANGE_Y).max(0);
        let end_x = (self.offset_x + VIEW_RANGE_X + 1).min(self.width);
        let end_y = (self.offset_y + VIEW_RANGE_Y + 1).min(self.height);

        // 网格颜色：半透明绿色
        let grid_color = Color::from_rgba(0, 255, 0, 100);
        
        // 绘制垂直线和水平线
        // 🔧 网格线标记格子边界,使用简化坐标公式(不含 -OFFSET_X)
        // 而瓦片图像会根据纹理offset调整位置对齐网格
        for map_x in start_x..=end_x {
            // 简化公式：直接计算格子边界位置
            let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH) as f32;
            
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
            // Y方向不需要额外偏移（与瓦片Y坐标基准一致）
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
                // 使用与网格线一致的简化坐标公式
                let screen_x = ((map_x - self.offset_x + OFFSET_X) * TILE_WIDTH) as f32;
                let screen_y = ((map_y - self.offset_y + OFFSET_Y) * TILE_HEIGHT) as f32;
                let mut text = graphics::Text::new(format!("({},{})", map_x, map_y));
                text.set_font("AlibabaPuHui"); // 🔧 设置中文字体
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
        
        // 🔧 计算鼠标对应的地图坐标（使用与网格线一致的公式）
        // 网格线绘制公式: screen_x = (map_x - offset_x + OFFSET_X) * TILE_WIDTH
        // 逆向推导:
        //   screen_x / TILE_WIDTH = map_x - offset_x + OFFSET_X
        //   map_x = screen_x / TILE_WIDTH + offset_x - OFFSET_X
        let map_x = ((self.mouse_x / TILE_WIDTH as f32).floor() as i32 + self.offset_x - OFFSET_X)
            .clamp(0, self.width - 1);
        
        // Y方向：使用网格线公式，并限制在有效范围内
        // screen_y = (map_y - offset_y + OFFSET_Y) * TILE_HEIGHT
        // map_y = screen_y / TILE_HEIGHT + offset_y - OFFSET_Y
        let map_y = ((self.mouse_y / TILE_HEIGHT as f32).floor() as i32 + self.offset_y - OFFSET_Y)
            .clamp(0, self.height - 1);

        // 🔧 坐标已经通过 clamp 限制在有效范围内，无需额外检查
        // 这样可以确保鼠标在最后一格的边缘也能显示信息

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
        
        // 🔧 计算信息框位置，确保不超出屏幕边界
        let mut box_x = self.mouse_x + 15.0;
        let mut box_y = self.mouse_y + 15.0;
        
        // 检查右边界：如果信息框右边超出屏幕，则显示在鼠标左侧
        if box_x + box_width > SCREEN_WIDTH {
            box_x = self.mouse_x - box_width - 15.0;
            // 如果左侧也放不下，则贴着右边界
            if box_x < 0.0 {
                box_x = SCREEN_WIDTH - box_width - 5.0;
            }
        }
        
        // 检查下边界：如果信息框底部超出屏幕，则显示在鼠标上方
        if box_y + box_height > SCREEN_HEIGHT {
            box_y = self.mouse_y - box_height - 15.0;
            // 如果上方也放不下，则贴着下边界
            if box_y < 0.0 {
                box_y = SCREEN_HEIGHT - box_height - 5.0;
            }
        }
        
        // 确保不会超出左边界和上边界
        box_x = box_x.max(5.0);
        box_y = box_y.max(5.0);

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
            text.set_font("AlibabaPuHui"); // 🔧 设置中文字体
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
            // 🔧 边界计算：与初始化保持一致
            let max_offset_x = (self.width - OFFSET_X).max(OFFSET_X);
            let max_offset_y = (self.height - OFFSET_Y).max(OFFSET_Y);

            match code {
                KeyCode::ArrowLeft => {
                    self.offset_x = (self.offset_x - step).max(OFFSET_X);
                }
                KeyCode::ArrowRight => {
                    self.offset_x = (self.offset_x + step).min(max_offset_x);
                }
                KeyCode::ArrowUp => {
                    self.offset_y = (self.offset_y - step).max(OFFSET_Y);
                }
                KeyCode::ArrowDown => {
                    self.offset_y = (self.offset_y + step).min(max_offset_y);
                }
                KeyCode::KeyT => {
                    // 🔍 T键：切换地图网格显示（绿色坐标网格）
                    self.show_tile_grid = !self.show_tile_grid;
                    println!("🔍 地图网格: {}", if self.show_tile_grid { "✅ 开启" } else { "❌ 关闭" });
                }
                KeyCode::KeyB => {
                    // 🔍 B键：切换图片边框显示（彩色瓦片边框）
                    self.show_image_border = !self.show_image_border;
                    println!("🔍 图片边框: {}", if self.show_image_border { "✅ 开启" } else { "❌ 关闭" });
                }
                KeyCode::Digit1 => {
                    // 🎨 1键：切换Back层渲染
                    self.render_back = !self.render_back;
                    println!("🎨 Back层: {}", if self.render_back { "✅ 显示" } else { "❌ 隐藏" });
                    self.printed_debug_once = false;
                }
                KeyCode::Digit2 => {
                    // 🎨 2键：切换Middle层渲染
                    self.render_middle = !self.render_middle;
                    println!("🎨 Middle层: {}", if self.render_middle { "✅ 显示" } else { "❌ 隐藏" });
                    self.printed_debug_once = false;
                }
                KeyCode::Digit3 => {
                    // 🎨 3键：切换Front层渲染
                    self.render_front = !self.render_front;
                    println!("🎨 Front层: {}", if self.render_front { "✅ 显示" } else { "❌ 隐藏" });
                    self.printed_debug_once = false;
                }
                _ => {}
            }

            // 只有移动视口时才输出offset信息
            match code {
                KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                    self.printed_debug_once = false;
                    println!("🧭 视口偏移 -> ({}, {})", self.offset_x, self.offset_y);
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
        // 🖱️ 更新鼠标位置(用于悬停显示)
        self.mouse_x = x;
        self.mouse_y = y;
        
        if self.dragging {
            let delta_x = x - self.last_mouse_pos.0;
            let delta_y = y - self.last_mouse_pos.1;

            // 🔧 立即更新鼠标位置
            self.last_mouse_pos = (x, y);

            // 🎮 鼠标拖动：使用与瓦片宽度匹配的像素比例
            // 每移动 TILE_WIDTH(48) 像素 = 地图移动 1 格
            // 为了更流畅的体验，使用 TILE_WIDTH/2 = 24 像素移动1格
            self.accumulated_drag_x += -delta_x;
            self.accumulated_drag_y += -delta_y;

            // 🔧 使用更合理的像素比例：24像素 = 1格地图移动
            let step_x = (self.accumulated_drag_x / 24.0) as i32;
            let step_y = (self.accumulated_drag_y / 24.0) as i32;

            if step_x != 0 || step_y != 0 {
                // 🔧 使用与键盘一致的边界计算
                let max_offset_x = (self.width - OFFSET_X).max(OFFSET_X);
                let max_offset_y = (self.height - OFFSET_Y).max(OFFSET_Y);
                
                // 计算新位置并clamp到边界
                self.offset_x = (self.offset_x + step_x).clamp(OFFSET_X, max_offset_x);
                self.offset_y = (self.offset_y + step_y).clamp(OFFSET_Y, max_offset_y);

                // 减去已经移动的格子数对应的像素
                self.accumulated_drag_x -= step_x as f32 * 24.0;
                self.accumulated_drag_y -= step_y as f32 * 24.0;

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

        // 🔍 绘制调试网格(按T键切换)
        if self.show_tile_grid {
            self.draw_debug_grid(ctx, &mut canvas)?;
        }

        // 🖱️ 绘制鼠标悬停信息
        self.draw_hover_info(ctx, &mut canvas)?;

        self.printed_debug_once = true;
        canvas.finish(ctx)?;
        Ok(())
    }
}
static FONT: &[u8] = include_bytes!("../resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");

fn main() -> GameResult {
    println!("\n🔧 Simple Type 100 地图查看器");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📍 视角控制:");
    println!("   方向键 / 鼠标拖拽 - 移动视角");
    println!("\n🔍 显示控制:");
    println!("   T键 - 切换地图网格（绿色坐标网格）");
    println!("   B键 - 切换图片边框（彩色瓦片边框）");
    println!("\n🎨 图层控制:");
    println!("   1键 - 切换Back层显示（地表砖 - 红色边框）");
    println!("   2键 - 切换Middle层显示（装饰 - 橙色边框）");
    println!("   3键 - 切换Front层显示（建筑物 - 蓝色边框）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let (mut ctx, event_loop) = ContextBuilder::new("simple_map_viewer", "Mir2")
        .window_setup(WindowSetup::default().title("Simple Map Viewer"))
        .window_mode(WindowMode::default().dimensions(SCREEN_WIDTH, SCREEN_HEIGHT))
        .build()?;
    
    ctx.gfx.add_font("AlibabaPuHui", FontData::from_slice(FONT)?);
    ctx.gfx.window().set_ime_allowed(true);
    let state = SimpleMapViewer::new("Map/0122.map")?;
    event::run(ctx, event_loop, state)
}
