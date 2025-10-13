// Map Viewer - 独立地图绘制程序
// 功能:
// - 完整地图渲染 (Back/Middle/Front 三层)
// - 鼠标拖拽移动相机
// - 鼠标滚轮缩放
// - 显示坐标和FPS
//
// 运行: cargo run --bin map_viewer --release

use ggez::winit::event::MouseButton;
use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{self, Canvas, Color, DrawParam, FontData, Text},
    Context, ContextBuilder, GameResult,
};
use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};
use std::time::Instant;

/// 相机系统
struct Camera {
    x: f32,    // 世界坐标 X
    y: f32,    // 世界坐标 Y
    zoom: f32, // 缩放级别 (1.0 = 正常)

    // 拖拽状态
    dragging: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    drag_start_cam_x: f32,
    drag_start_cam_y: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_cam_x: 0.0,
            drag_start_cam_y: 0.0,
        }
    }

    /// 开始拖拽
    fn start_drag(&mut self, mouse_x: f32, mouse_y: f32) {
        self.dragging = true;
        self.drag_start_x = mouse_x;
        self.drag_start_y = mouse_y;
        self.drag_start_cam_x = self.x;
        self.drag_start_cam_y = self.y;
    }

    /// 更新拖拽
    fn update_drag(&mut self, mouse_x: f32, mouse_y: f32) {
        if self.dragging {
            let dx = mouse_x - self.drag_start_x;
            let dy = mouse_y - self.drag_start_y;
            self.x = self.drag_start_cam_x - dx / self.zoom;
            self.y = self.drag_start_cam_y - dy / self.zoom;
        }
    }

    /// 结束拖拽
    fn end_drag(&mut self) {
        self.dragging = false;
    }

    /// 缩放
    fn zoom_by(
        &mut self,
        delta: f32,
        mouse_x: f32,
        mouse_y: f32,
        screen_width: f32,
        screen_height: f32,
    ) {
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(0.1, 4.0);

        // 以鼠标位置为中心缩放
        let world_x = self.screen_to_world_x(mouse_x, screen_width);
        let world_y = self.screen_to_world_y(mouse_y, screen_height);

        self.x = world_x - (mouse_x - screen_width / 2.0) / self.zoom;
        self.y = world_y - (mouse_y - screen_height / 2.0) / self.zoom;
    }

    /// 屏幕坐标转世界坐标
    fn screen_to_world_x(&self, screen_x: f32, screen_width: f32) -> f32 {
        self.x + (screen_x - screen_width / 2.0) / self.zoom
    }

    fn screen_to_world_y(&self, screen_y: f32, screen_height: f32) -> f32 {
        self.y + (screen_y - screen_height / 2.0) / self.zoom
    }

    /// 世界坐标转屏幕坐标
    fn world_to_screen_x(&self, world_x: f32, screen_width: f32) -> f32 {
        (world_x - self.x) * self.zoom + screen_width / 2.0
    }

    fn world_to_screen_y(&self, world_y: f32, screen_height: f32) -> f32 {
        (world_y - self.y) * self.zoom + screen_height / 2.0
    }
}

/// 地图渲染器
struct MapRenderer {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
    animation_count: i32,
}

impl MapRenderer {
    // 传奇地图格子尺寸
    // 基础格子: 48x32 (逻辑坐标)
    // 实际瓦片: 96x64 (2x2格子，渲染在偶数坐标)
    const CELL_WIDTH: i32 = 48; // 单个格子宽度
    const CELL_HEIGHT: i32 = 32; // 单个格子高度

    fn new(reader: MapReader) -> Self {
        // 🔧 使用全局 LIBRARIES 初始化所有地图库
        println!("📚 正在初始化地图库...");
        initialize_all_libraries("Data").expect("初始化地图库失败");
        println!("✅ 地图库初始化完成");

        Self {
            cells: reader.map_cells,
            width: reader.width,
            height: reader.height,
            animation_count: 0,
        }
    }

    /// 获取单元格
    fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            Some(&self.cells[x as usize][y as usize])
        } else {
            None
        }
    }

    /// 渲染地图
    fn draw(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        screen_width: f32,
        screen_height: f32,
        show_back: bool,
        show_middle: bool,
        show_front: bool,
        show_borders: bool,
    ) -> GameResult<()> {
        // 更新动画计数器
        self.animation_count = (self.animation_count + 1) % 1000;

        // 计算可见区域 (世界坐标转地图格子)
        let left = camera.screen_to_world_x(0.0, screen_width);
        let right = camera.screen_to_world_x(screen_width, screen_width);
        let top = camera.screen_to_world_y(0.0, screen_height);
        let bottom = camera.screen_to_world_y(screen_height, screen_height);

        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32 - 2).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32 + 2).min(self.width - 1);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32 - 2).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32 + 2).min(self.height - 1);

        // ========================================
        // BACK LAYER (大地砖)
        // ========================================
        if show_back {
            // 传奇地图特点：Back层只渲染偶数行列，通过大瓦片(96x64)覆盖4个格子
            // 🔧 关键修复：必须从偶数坐标开始，不能直接用 step_by(2)
            let back_start_y = if start_y % 2 == 0 { start_y } else { start_y + 1 };
            let back_start_x = if start_x % 2 == 0 { start_x } else { start_x + 1 };
            
            for y in (back_start_y..=end_y).step_by(2) {
                // 只处理偶数行
                for x in (back_start_x..=end_x).step_by(2) {
                    // 只处理偶数列
                    if let Some(cell) = self.get_cell(x, y) {
                        //     if ((M2CellInfo[x, y].BackImage == 0) || (M2CellInfo[x, y].BackIndex == -1)) continue;
                        // // BackImage 高3位用于特殊标记，需要屏蔽
                        // index = (M2CellInfo[x, y].BackImage & 0x1FFFFFFF) - 1;
                        let index = (cell.back_image & 0x1FFFFFFF) - 1;
                        if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
                            continue;
                        }
                        let index = index as usize;

                        // C# 公式: drawX = (x - User.X + OffSetX) * CellWidth - OffSetX
                        // 简化版：直接使用格子坐标 * 格子宽度
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;

                        self.draw_tile(
                            ctx,
                            canvas,
                            camera,
                            screen_width,
                            screen_height,
                            cell.back_index as i32,
                            index,
                            world_x,
                            world_y,
                            show_borders,
                            Color::from_rgb(255, 0, 0),
                        )?;
                    }
                }
            }
        }

        // ========================================
        // MIDDLE LAYER (小地砖)
        // ========================================
        if show_middle {
            // 渲染所有格子，不限奇偶
            for y in start_y..=end_y {
                for x in start_x..=end_x {
                    if let Some(cell) = self.get_cell(x, y) {
                        let index = cell.middle_image - 1;
                        if index < 0 || cell.middle_index == -1 {
                            continue;
                        }
                        // Middle层尺寸过滤：只渲染 48x32 或 96x64 的瓦片
                        // 防止绘制条状错误瓦片 (tile strips)
                        if cell.middle_index >= 0 {
                            if let Some(mlib) = get_map_library(cell.middle_index) {
                                if let Ok(mut mlib) = mlib.lock() {
                                    if let Ok((w, h)) = mlib.get_size(index as usize) {
                                        // 只允许单格 (48x32) 或双格 (96x64) 尺寸
                                        if (w as i32 != Self::CELL_WIDTH
                                            || h as i32 != Self::CELL_HEIGHT)
                                            && (w as i32 != Self::CELL_WIDTH * 2
                                                || h as i32 != Self::CELL_HEIGHT * 2)
                                        {
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;

                        self.draw_tile(
                            ctx,
                            canvas,
                            camera,
                            screen_width,
                            screen_height,
                            cell.middle_index as i32,
                            index as usize,
                            world_x,
                            world_y,
                            show_borders,
                            Color::from_rgb(0, 255, 0),
                        )?;
                    }
                }
            }
        }

        // ========================================
        // FRONT LAYER (前景层)
        // ========================================
        if show_front {
            // 渲染建筑顶部、树冠等前景物体
            for y in start_y..=end_y {
                for x in start_x..=end_x {
                    if let Some(cell) = self.get_cell(x, y) {
                        // index = (M2CellInfo[x, y].FrontImage & 0x7FFF) - 1;
                        // if (index == -1) continue;
                        // int fileIndex = M2CellInfo[x, y].FrontIndex;
                        // if (fileIndex == -1) continue;
                        // Size s = Libraries.MapLibs[fileIndex].GetSize(index);
                        // if (fileIndex == 200) continue; // 修复旧版 4.map 的随机坏点
                        let index = (cell.front_image & 0x7FFF) - 1;
                        if index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                            continue;
                        }

                        //TODO 门动画处理
                        // if (M2CellInfo[x, y].DoorIndex > 0)
                        // {
                        //     // 查找或创建门对象
                        //     Door DoorInfo = GetDoor(M2CellInfo[x, y].DoorIndex);
                        //     if (DoorInfo == null)
                        //     {
                        //         // 首次遇到门，创建门对象
                        //         DoorInfo = new Door() { index = M2CellInfo[x, y].DoorIndex, DoorState = 0, ImageIndex = 0, LastTick = CMain.Time };
                        //         Doors.Add(DoorInfo);
                        //     }
                        //     else
                        //     {
                        //         // 门已开启，使用开门动画帧
                        //         if (DoorInfo.DoorState != 0)
                        //         {
                        //             // 门动画索引计算：基础索引 + (动画帧 + 1) * 偏移量
                        //             index += (DoorInfo.ImageIndex + 1) * M2CellInfo[x, y].DoorOffset;
                        //         }
                        //     }
                        // }

                        // if let Some(mlib) = get_map_library(cell.front_index) {
                        //     if let Ok(mut mlib) = mlib.lock() {
                        //         if let Ok((w, h)) = mlib.get_size(index as usize) {
                        //             // 只允许单格 (48x32) 或双格 (96x64) 尺寸
                        //             if (w as i32 != Self::CELL_WIDTH
                        //                 || h as i32 != Self::CELL_HEIGHT)
                        //                 && (w as i32 != Self::CELL_WIDTH * 2
                        //                     || h as i32 != Self::CELL_HEIGHT * 2)
                        //             {
                        //                 continue;
                        //             }
                        //         }
                        //     }
                        // }

                        // Front层不需要向上偏移，让图像的offset自然处理
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;

                        self.draw_front(
                            ctx,
                            canvas,
                            camera,
                            screen_width,
                            screen_height,
                            cell.front_index as i32,
                            index as usize,
                            world_x,
                            world_y,
                            show_borders,
                            Color::from_rgb(0, 150, 255),
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 绘制瓦片
    ///
    /// C# 绘制逻辑详解:
    /// ```csharp
    /// // GameScene.cs DrawFloor() - 计算格子的屏幕坐标
    /// drawX = (x - User.Movement.X + OffSetX) * CellWidth - OffSetX + User.OffSetMove.X;
    /// drawY = (y - User.Movement.Y + OffSetY) * CellHeight + User.OffSetMove.Y;
    ///
    /// // MLibrary.cs Draw(int index, int x, int y) - 直接绘制
    /// DXManager.Draw(mi.Image,
    ///     new Rectangle(0, 0, mi.Width, mi.Height),  // 源矩形：完整纹理
    ///     new Vector3((float)x, (float)y, 0.0F),     // 目标位置：不应用offset
    ///     Color.White);
    /// ```
    ///
    /// **关键发现**：
    /// 1. C# 的 Draw(index, x, y) 重载 **不应用 mi.X/mi.Y offset**
    /// 2. 只有 Draw(index, point, color, offSet=true) 才会应用 offset
    /// 3. 地图瓦片绘制使用前者，所以offset在格子坐标计算时已经体现
    /// 4. **但是**：对于 Middle/Front 层的大型物体（树、建筑），offset 用于定位
    fn draw_tile(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        screen_width: f32,
        screen_height: f32,
        lib_index: i32,
        image_index: usize,
        world_x: f32,
        world_y: f32,
        show_border: bool,
        border_color: Color,
    ) -> GameResult<()> {
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();

            // 使用纹理缓存（先获取纹理再读取 info，避免重复 lock）
            match lib.get_or_create_texture(ctx, image_index) {
                Ok(info) => {
                    // 从ImageInfo获取实际的纹理
                    if let Some(ref texture) = info.image {
                        // 🔧 修复坐标计算：正确的世界坐标转屏幕坐标
                        // 必须考虑相机缩放系数
                        let screen_x = (world_x - camera.x) * camera.zoom + screen_width / 2.0;
                        let screen_y = (world_y - camera.y) * camera.zoom + screen_height / 2.0;

                        // 应用图像offset（如果需要的话）
                        let final_x = screen_x;
                        let final_y = screen_y;

                        // 🔧 应用缩放到瓦片绘制
                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([final_x, final_y])
                                .scale([camera.zoom, camera.zoom]),
                        );

                        // 绘制纹理边框
                        if show_border {
                            let border_rect = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::stroke(1.0),
                                graphics::Rect::new(
                                    final_x,
                                    final_y,
                                    info.width as f32 * camera.zoom,
                                    info.height as f32 * camera.zoom,
                                ),
                                border_color,
                            )?;
                            canvas.draw(&border_rect, DrawParam::default());
                        }
                    }
                }
                Err(_e) => {
                    // 忽略加载错误（避免日志刷屏）
                }
            }
        }

        Ok(())
    }

    fn draw_front(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        screen_width: f32,
        screen_height: f32,
        lib_index: i32,
        image_index: usize,
        world_x: f32,
        world_y: f32,
        show_border: bool,
        border_color: Color,
    ) -> GameResult<()> {
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();


            // 使用纹理缓存（先获取纹理再读取 info，避免重复 lock）
            match lib.get_or_create_texture(ctx, image_index) {
                Ok(info) => {
                    // 从ImageInfo获取实际的纹理
                    if let Some(ref texture) = info.image {
                        // 🔧 修复坐标计算：正确的世界坐标转屏幕坐标
                        // 必须考虑相机缩放系数
                        let screen_x = (world_x - camera.x) * camera.zoom + screen_width / 2.0;
                        let screen_y = (world_y - camera.y) * camera.zoom + screen_height / 2.0;

                        // 应用图像offset和Front层特殊的Y偏移
                        let final_x = screen_x;
                        let final_y = screen_y - (info.height as f32 - Self::CELL_HEIGHT as f32) * camera.zoom;

                        // 🔧 应用缩放到瓦片绘制
                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([final_x, final_y])
                                .scale([camera.zoom, camera.zoom]),
                        );

                        // 绘制纹理边框
                        if show_border {
                            let border_rect = graphics::Mesh::new_rectangle(
                                ctx,
                                graphics::DrawMode::stroke(1.0),
                                graphics::Rect::new(
                                    final_x,
                                    final_y,
                                    info.width as f32 * camera.zoom,
                                    info.height as f32 * camera.zoom,
                                ),
                                border_color,
                            )?;
                            canvas.draw(&border_rect, DrawParam::default());
                        }
                    }
                }
                Err(_e) => {
                    // 忽略加载错误（避免日志刷屏）
                }
            }
        }

        Ok(())
    }

    /// 绘制地图网格
    fn draw_grid(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        screen_width: f32,
        screen_height: f32,
    ) -> GameResult<()> {
        // 计算可见区域
        let left = camera.screen_to_world_x(0.0, screen_width);
        let right = camera.screen_to_world_x(screen_width, screen_width);
        let top = camera.screen_to_world_y(0.0, screen_height);
        let bottom = camera.screen_to_world_y(screen_height, screen_height);

        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32).min(self.width);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32).min(self.height);

        let grid_color = Color::from_rgba(0, 255, 0, 120);

        // 绘制垂直线
        for x in start_x..=end_x {
            let world_x = (x * Self::CELL_WIDTH) as f32;
            // 应用缩放到网格线坐标
            let screen_x = (world_x - camera.x) * camera.zoom + screen_width / 2.0;

            if screen_x >= 0.0 && screen_x <= screen_width {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[screen_x, 0.0], [screen_x, screen_height]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 绘制水平线
        for y in start_y..=end_y {
            let world_y = (y * Self::CELL_HEIGHT) as f32;
            // 应用缩放到网格线坐标
            let screen_y = (world_y - camera.y) * camera.zoom + screen_height / 2.0;

            if screen_y >= 0.0 && screen_y <= screen_height {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[0.0, screen_y], [screen_width, screen_y]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        Ok(())
    }

    /// 绘制障碍层
    fn draw_obstacles(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        screen_width: f32,
        screen_height: f32,
    ) -> GameResult<()> {
        // 计算可见区域
        let left = camera.screen_to_world_x(0.0, screen_width);
        let right = camera.screen_to_world_x(screen_width, screen_width);
        let top = camera.screen_to_world_y(0.0, screen_height);
        let bottom = camera.screen_to_world_y(screen_height, screen_height);

        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32).min(self.width);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32).min(self.height);

        // 半透明红色表示障碍物
        let obstacle_color = Color::from_rgba(255, 0, 0, 100);

        // 遍历所有格子
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    // 检查是否有障碍物 (DoorClosed、Block 或 MiddleBlock 标记)
                    let has_obstacle = (cell.door_offset & 0x80) != 0  // DoorClosed
                        || (cell.door_index & 0x80) != 0               // Block
                        || (cell.front_image & 0x8000) != 0; // MiddleBlock (LowWall)

                    if has_obstacle {
                        let world_x = (x * Self::CELL_WIDTH) as f32;
                        let world_y = (y * Self::CELL_HEIGHT) as f32;
                        let screen_x = world_x - camera.x + screen_width / 2.0;
                        let screen_y = world_y - camera.y + screen_height / 2.0;

                        // 绘制半透明矩形表示障碍
                        let obstacle_rect = graphics::Mesh::new_rectangle(
                            ctx,
                            graphics::DrawMode::fill(),
                            graphics::Rect::new(
                                screen_x,
                                screen_y,
                                Self::CELL_WIDTH as f32,
                                Self::CELL_HEIGHT as f32,
                            ),
                            obstacle_color,
                        )?;
                        canvas.draw(&obstacle_rect, DrawParam::default());
                    }
                }
            }
        }

        Ok(())
    }
}

/// 主程序状态
struct MapViewerState {
    camera: Camera,
    map_renderer: MapRenderer,
    screen_width: f32,
    screen_height: f32,
    fps_timer: Instant,
    frame_count: u32,
    fps: u32,
    map_name: String,

    // 显示开关
    show_grid: bool,         // G键：显示地图网格
    show_borders: bool,      // B键：显示纹理边框
    show_layer_back: bool,   // 1键：显示Back层
    show_layer_middle: bool, // 2键：显示Middle层
    show_layer_front: bool,  // 3键：显示Front层
    show_obstacles: bool,    // O键：显示障碍层
}

impl MapViewerState {
    fn new(ctx: &mut Context, map_path: &str) -> GameResult<Self> {
        println!("\n🗺️  加载地图: {}", map_path);
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        println!("🔍 初始窗口尺寸: {}x{}", screen_width, screen_height);
        // 加载地图
        let reader = MapReader::new(map_path).map_err(|e| {
            eprintln!("❌ 加载地图失败: {}", e);
            ggez::GameError::ResourceLoadError(format!("Failed to load map: {}", e))
        })?;

        println!("✅ 地图加载成功: {}x{}", reader.width, reader.height);

        let map_renderer = MapRenderer::new(reader);

        // 🔧 相机初始位置：世界坐标原点(0,0)，这样屏幕中心显示世界(0,0)
        let mut camera = Camera::new();
        camera.x = 0.0; // 世界坐标X
        camera.y = 0.0; // 世界坐标Y
        camera.zoom = 1.0;

        println!("📍 相机初始位置: 世界坐标({}, {})", camera.x, camera.y);
        println!(
            "🎯 地图像素尺寸: {}x{} 像素",
            map_renderer.width * MapRenderer::CELL_WIDTH,
            map_renderer.height * MapRenderer::CELL_HEIGHT
        );

        Ok(Self {
            camera,
            map_renderer,
            screen_width,
            screen_height,
            fps_timer: Instant::now(),
            frame_count: 0,
            fps: 0,
            map_name: map_path.to_string(),
            show_grid: true,
            show_borders: false,
            show_layer_back: true,
            show_layer_middle: true,
            show_layer_front: true,
            show_obstacles: false,
        })
    }
}

impl EventHandler for MapViewerState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 更新FPS
        self.frame_count += 1;
        if self.fps_timer.elapsed().as_secs() >= 1 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer = Instant::now();
        }

        // 更新屏幕尺寸
        let (w, h) = ctx.gfx.drawable_size();
        self.screen_width = w;
        self.screen_height = h;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 使用完全透明背景，避免透明度处理不完全时露出背景色
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgba(0, 0, 0, 0));

        // 设置混合模式为标准 Alpha 混合 (修复黑块闪烁问题)
        canvas.set_blend_mode(graphics::BlendMode::ALPHA);

        // 绘制地图
        self.map_renderer.draw(
            ctx,
            &mut canvas,
            &self.camera,
            self.screen_width,
            self.screen_height,
            self.show_layer_back,
            self.show_layer_middle,
            self.show_layer_front,
            self.show_borders,
        )?;

        // 绘制网格
        if self.show_grid {
            self.map_renderer.draw_grid(
                ctx,
                &mut canvas,
                &self.camera,
                self.screen_width,
                self.screen_height,
            )?;
        }

        // 绘制障碍层
        if self.show_obstacles {
            self.map_renderer.draw_obstacles(
                ctx,
                &mut canvas,
                &self.camera,
                self.screen_width,
                self.screen_height,
            )?;
        }

        // 绘制UI信息
        self.draw_ui(ctx, &mut canvas)?;

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
            self.camera.start_drag(x, y);
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
            self.camera.end_drag();
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
        self.camera.update_drag(x, y);
        Ok(())
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, _x: f32, y: f32) -> GameResult {
        // 鼠标滚轮缩放：向上放大，向下缩小
        let mouse_pos = ctx.mouse.position();
        self.camera.zoom_by(
            y,
            mouse_pos.x,
            mouse_pos.y,
            self.screen_width,
            self.screen_height,
        );
        Ok(())
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult {
        use ggez::input::keyboard::KeyCode;
        use ggez::winit::keyboard::PhysicalKey;

        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            match keycode {
                KeyCode::KeyG => {
                    self.show_grid = !self.show_grid;
                    println!(
                        "🔍 地图网格: {}",
                        if self.show_grid { "开启" } else { "关闭" }
                    );
                }
                KeyCode::KeyB => {
                    self.show_borders = !self.show_borders;
                    println!(
                        "🔍 纹理边框: {}",
                        if self.show_borders {
                            "开启"
                        } else {
                            "关闭"
                        }
                    );
                }
                KeyCode::Digit1 => {
                    self.show_layer_back = !self.show_layer_back;
                    println!(
                        "🎨 Back层: {}",
                        if self.show_layer_back {
                            "开启"
                        } else {
                            "关闭"
                        }
                    );
                }
                KeyCode::Digit2 => {
                    self.show_layer_middle = !self.show_layer_middle;
                    println!(
                        "🎨 Middle层: {}",
                        if self.show_layer_middle {
                            "开启"
                        } else {
                            "关闭"
                        }
                    );
                }
                KeyCode::Digit3 => {
                    self.show_layer_front = !self.show_layer_front;
                    println!(
                        "🎨 Front层: {}",
                        if self.show_layer_front {
                            "开启"
                        } else {
                            "关闭"
                        }
                    );
                }
                KeyCode::KeyO => {
                    self.show_obstacles = !self.show_obstacles;
                    println!(
                        "🚧 障碍层: {}",
                        if self.show_obstacles {
                            "开启"
                        } else {
                            "关闭"
                        }
                    );
                }
                KeyCode::Escape => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl MapViewerState {
    fn draw_ui(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 计算当前鼠标悬停的地图格子坐标
        let mouse_pos = ctx.mouse.position();
        let mouse_x = mouse_pos.x;
        let mouse_y = mouse_pos.y;
        let world_x = self.camera.screen_to_world_x(mouse_x, self.screen_width);
        let world_y = self.camera.screen_to_world_y(mouse_y, self.screen_height);
        let grid_x = (world_x / MapRenderer::CELL_WIDTH as f32).floor() as i32;
        let grid_y = (world_y / MapRenderer::CELL_HEIGHT as f32).floor() as i32;

        // 构建顶部状态栏
        let status_text = format!(
            "Map: {} | Size: {}x{} | FPS: {} | Zoom: {:.2}x | Camera: ({:.0}, {:.0}) | Grid: ({}, {})\nG-网格 | B-边框 | 1/2/3-图层 | O-障碍",
            self.map_name,
            self.map_renderer.width, self.map_renderer.height,
            self.fps,
            self.camera.zoom,
            self.camera.x, self.camera.y,
            grid_x, grid_y
        );

        // 绘制状态栏背景
        let status_bg = graphics::Rect::new(10.0, 10.0, self.screen_width - 20.0, 60.0);
        let status_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            status_bg,
            Color::from_rgba(0, 0, 0, 180),
        )?;
        canvas.draw(&status_mesh, DrawParam::default());

        // 绘制状态栏文本
        let mut status = Text::new(status_text);
        status.set_font("AlibabaPuHui");
        status.set_scale(26.0); // 增大字体到26
        canvas.draw(
            &status,
            DrawParam::default().dest([20.0, 15.0]).color(Color::WHITE),
        );

        // 🖱️ 获取鼠标悬停单元格的详细信息（仿照图片格式）
        if let Some(cell) = self.map_renderer.get_cell(grid_x, grid_y) {
            // 获取库名
            let back_lib = if cell.back_index >= 0 {
                format!("Tiles")
            } else {
                "None".to_string()
            };

            let middle_lib = if cell.middle_index >= 0 {
                format!("Smtiles")
            } else {
                "None".to_string()
            };

            let front_lib = if cell.front_index >= 0 {
                format!("Objects")
            } else {
                "None".to_string()
            };

            // 构建详细信息（按图片格式）
            // 
            // 三层绘制逻辑中的索引计算：
            // Back:   index = (back_image & 0x1FFFFFFF) - 1
            // Middle: index = middle_image - 1  
            // Front:  index = (front_image & 0x7FFF) - 1
            //
            // 注意：显示的是原始值（去掉标志位），不是减1后的索引
            let back_image_value = cell.back_image & 0x1FFFFFFF;
            let middle_image_value = cell.middle_image;
            let front_image_value = cell.front_image & 0x7FFF;
            
            let cell_info = format!(
                "X: {}        Y: {}     Version        LibName    LibIndex\n\
                BackImage:   {}       WemadeMir2     {}      {}\n\
                MiddleImage: {}       WemadeMir2     {}    {}\n\
                FrontImage:  {}       WemadeMir2     {}    {}\n\n\
                Limit:       Back  {}           Front  {}\n\n\
                Animation:   F_Frame   F_Tick     F_Blend\n\
                Animation:   M_Frame   M_Tick     M_Blend\n\n\
                Door:        Offset {}  Index {}    Entity  {}\n\n\
                Light: {}     Fishing: {}",
                grid_x,
                grid_y,
                // BackImage: 显示去掉高位标志后的原始值
                back_image_value,
                back_lib,
                cell.back_index,
                // MiddleImage: 直接显示原始值（没有高位标志需要屏蔽）
                middle_image_value,
                middle_lib,
                cell.middle_index,
                // FrontImage: 显示去掉高位标志后的原始值
                front_image_value,
                front_lib,
                cell.front_index,
                // Limit: Back层的HighWall标记 (0x20000000)
                if (cell.back_image & 0x20000000) != 0 {
                    "True"
                } else {
                    "False"
                },
                // Limit: Front层的LowWall/MiddleBlock标记 (0x8000)
                if (cell.front_image & 0x8000) != 0 {
                    "True"
                } else {
                    "False"
                },
                // Door: DoorOffset的低7位
                cell.door_offset & 0x7F,
                // Door: DoorIndex的低7位
                cell.door_index & 0x7F,
                // Door: Entity - DoorClosed标记(door_offset.bit7) 或 Block标记(door_index.bit7)
                if (cell.door_offset & 0x80) != 0 || (cell.door_index & 0x80) != 0 {
                    "True"
                } else {
                    "False"
                },
                // Light: 光照强度
                cell.light,
                // Fishing: 地图文件中没有此字段
                "False"
            );

            // 🖱️ CellInfo 面板尺寸
            let panel_width = 650.0;
            let panel_height = 320.0;
            let offset_x = 20.0; // 鼠标偏移量，避免被鼠标遮挡
            let offset_y = 20.0;
            let margin = 10.0; // 屏幕边缘边距

            // 🧮 计算面板位置（跟随鼠标，边界自动翻转）
            let mut panel_x = mouse_x + offset_x;
            let mut panel_y = mouse_y + offset_y;

            // 检查右边界，超出则翻转到鼠标左侧
            if panel_x + panel_width + margin > self.screen_width {
                panel_x = mouse_x - panel_width - offset_x;
            }

            // 检查下边界，超出则翻转到鼠标上方
            if panel_y + panel_height + margin > self.screen_height {
                panel_y = mouse_y - panel_height - offset_y;
            }

            // 检查左边界，确保不超出屏幕左侧
            if panel_x < margin {
                panel_x = margin;
            }

            // 检查上边界，确保不超出屏幕顶部（避开状态栏）
            let status_bar_bottom = 80.0; // 状态栏底部位置
            if panel_y < status_bar_bottom {
                panel_y = status_bar_bottom;
            }

            // 绘制CellInfo背景（跟随鼠标）
            let info_bg = graphics::Rect::new(panel_x, panel_y, panel_width, panel_height);
            let info_mesh = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                info_bg,
                Color::from_rgba(40, 40, 40, 220),
            )?;
            canvas.draw(&info_mesh, DrawParam::default());

            // 绘制边框
            let border_mesh = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::stroke(2.0),
                info_bg,
                Color::from_rgb(100, 100, 100),
            )?;
            canvas.draw(&border_mesh, DrawParam::default());

            // 绘制详细信息文本
            let mut info_text = Text::new(cell_info);
            info_text.set_font("AlibabaPuHui");
            info_text.set_scale(26.0); // 增大字体到26
            canvas.draw(
                &info_text,
                DrawParam::default()
                    .dest([panel_x + 10.0, panel_y + 10.0])
                    .color(Color::WHITE),
            );
        }

        Ok(())
    }
}

fn main() -> GameResult {
    // 从命令行参数获取地图路径
    let args: Vec<String> = std::env::args().collect();
    let map_path = if args.len() > 1 {
        args[1].clone()
    } else {
        // 默认地图
        "Map/0.map".to_string()
    };

    println!("\n╔══════════════════════════════════════════╗");
    println!("║     地图查看器 - Map Viewer v1.0        ║");
    println!("╚══════════════════════════════════════════╝");
    println!("\n📖 使用说明:");
    println!("  - 按住左键拖拽移动视角");
    println!("  - G键 - 切换地图网格");
    println!("  - B键 - 切换纹理边框");
    println!("  - O键 - 切换障碍层");
    println!("  - 1/2/3键 - 切换Back/Middle/Front层");
    println!("  - ESC 退出");
    println!("\n🎮 启动参数: cargo run --bin map_viewer [地图路径]");
    println!("   示例: cargo run --bin map_viewer Map/0.map\n");

    // 创建窗口
    let (mut ctx, event_loop) = ContextBuilder::new("map_viewer", "Mir2")
        .window_setup(WindowSetup::default().title("Map Viewer - 传奇地图查看器"))
        .window_mode(
            WindowMode::default()
                .dimensions(1280.0, 960.0)
                .resizable(true),
        )
        .build()?;
    static FONT: &[u8] = include_bytes!("../../resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");
    ctx.gfx
        .add_font("AlibabaPuHui", FontData::from_slice(FONT)?);
    ctx.gfx.window().set_ime_allowed(true);
    // 创建状态
    let state = MapViewerState::new(&mut ctx, &map_path)?;

    // 运行
    event::run(ctx, event_loop, state)
}
