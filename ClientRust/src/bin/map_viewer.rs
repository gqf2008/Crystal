// Map Viewer - 独立地图绘制程序
// 功能:
// - 完整地图渲染 (Back/Middle/Front 三层)
// - 鼠标拖拽移动相机
// - 鼠标滚轮缩放
// - 显示坐标和FPS
// - M键选择地图文件
//
// 运行: cargo run --bin map_viewer --release

use ggez::winit::event::MouseButton;
use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{
        self, BlendComponent, BlendFactor, BlendMode, BlendOperation, Canvas, Color, DrawParam,
        FontData, Text,
    },
    Context, ContextBuilder, GameResult,
};
use mir2_client::graphics::libraries::{get_map_library, initialize_all_libraries};
use mir2_client::objects::{CellInfo, MapReader};
use rfd::FileDialog;
use std::time::Instant;

/// 相机系统
struct Camera {
    x: f32,    // 世界坐标 X
    y: f32,    // 世界坐标 Y
    zoom: f32, // 缩放级别 (1.0 = 正常)

    // 🖥️ 屏幕尺寸 (由相机维护，避免层层传递)
    screen_width: f32,
    screen_height: f32,

    // 拖拽状态
    dragging: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    drag_start_cam_x: f32,
    drag_start_cam_y: f32,
}

impl Camera {
    fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            screen_width,
            screen_height,
            dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_cam_x: 0.0,
            drag_start_cam_y: 0.0,
        }
    }

    /// 更新屏幕尺寸 (窗口大小改变时调用)
    fn update_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
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
    fn zoom_by(&mut self, delta: f32, mouse_x: f32, mouse_y: f32) {
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(0.1, 4.0);

        // 以鼠标位置为中心缩放
        let world_x = self.screen_to_world_x(mouse_x);
        let world_y = self.screen_to_world_y(mouse_y);

        self.x = world_x - (mouse_x - self.screen_width / 2.0) / self.zoom;
        self.y = world_y - (mouse_y - self.screen_height / 2.0) / self.zoom;
    }

    /// 屏幕坐标转世界坐标
    fn screen_to_world_x(&self, screen_x: f32) -> f32 {
        self.x + (screen_x - self.screen_width / 2.0) / self.zoom
    }

    fn screen_to_world_y(&self, screen_y: f32) -> f32 {
        self.y + (screen_y - self.screen_height / 2.0) / self.zoom
    }

    fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        (
            self.x + (screen_x - self.screen_width / 2.0) / self.zoom,
            self.y + (screen_y - self.screen_height / 2.0) / self.zoom,
        )
    }

    /// 世界坐标转屏幕坐标
    fn world_to_screen_x(&self, world_x: f32) -> f32 {
        (world_x - self.x) * self.zoom + self.screen_width / 2.0
    }

    fn world_to_screen_y(&self, world_y: f32) -> f32 {
        (world_y - self.y) * self.zoom + self.screen_height / 2.0
    }

    fn world_to_screen(&self, world_x: f32, world_y: f32) -> (f32, f32) {
        (
            (world_x - self.x) * self.zoom + self.screen_width / 2.0,
            (world_y - self.y) * self.zoom + self.screen_height / 2.0,
        )
    }
}

/// 门结构体 (对应 C# Door)
#[derive(Debug, Clone)]
struct Door {
    index: u8,                     // 门ID (DoorIndex)
    door_state: u8,                // 门状态：0=关闭, 1=正在开启, 2=已开启, 3=正在关闭
    image_index: i32,              // 当前动画帧 (0-8, 共9帧)
    last_tick: std::time::Instant, // 上次动画更新时间
}

impl Door {
    fn new(index: u8) -> Self {
        Self {
            index,
            door_state: 0,  // 初始为关闭状态
            image_index: 0, // 初始帧为0
            last_tick: std::time::Instant::now(),
        }
    }
}

/// 地图渲染器
struct MapRenderer {
    cells: Vec<Vec<CellInfo>>,
    width: i32,
    height: i32,
    animation_count: i32,
    doors: Vec<Door>, // 🚪 门列表
}

impl MapRenderer {
    // 传奇地图格子尺寸
    // 基础格子: 48x32 (逻辑坐标)
    // 实际瓦片: 96x64 (2x2格子，渲染在偶数坐标)
    const CELL_WIDTH: i32 = 48; // 单个格子宽度
    const CELL_HEIGHT: i32 = 32; // 单个格子高度

    /// 🔥 创建传奇特效混合模式
    ///
    /// 对应 C# 的混合设置:
    /// ```csharp
    /// Device.SetRenderState(RenderState.SourceBlend, Blend.SourceAlpha);
    /// Device.SetRenderState(RenderState.DestinationBlend, Blend.One);
    /// ```
    ///
    /// 混合公式:
    /// `最终颜色 = 源颜色 × 源Alpha + 背景颜色 × 1`
    ///
    /// 这种混合模式的特点:
    /// - 黑色(RGB=0)透明区域(Alpha=0) → 0×0 + 背景×1 = 背景 (完全透明，正确！)
    /// - 半透明火焰(RGB=亮色, Alpha=0.5) → 亮色×0.5 + 背景×1 = 发光效果
    /// - 不透明核心(RGB=亮色, Alpha=1.0) → 亮色×1 + 背景×1 = 明亮发光
    #[inline]
    fn create_blend_mode() -> BlendMode {
        BlendMode {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha, // 源颜色乘以源Alpha
                dst_factor: BlendFactor::One,      // 背景颜色乘以1（保持原样）
                operation: BlendOperation::Add,    // 相加
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,   // 源Alpha保持
                dst_factor: BlendFactor::One,   // 背景Alpha保持
                operation: BlendOperation::Add, // 相加
            },
        }
    }

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
            doors: Vec::new(), // 🚪 初始化空门列表
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

    /// 🗺️ 地图格子坐标 → 世界像素坐标
    ///
    /// 参数:
    /// - `grid_x`: 地图格子 X 坐标 (0 到 width-1)
    /// - `grid_y`: 地图格子 Y 坐标 (0 到 height-1)
    ///
    /// 返回: (world_x, world_y) 世界像素坐标
    #[inline]
    fn map_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        (
            (grid_x * Self::CELL_WIDTH) as f32,
            (grid_y * Self::CELL_HEIGHT) as f32,
        )
    }

    /// 🌍 世界像素坐标 → 地图格子坐标
    ///
    /// 参数:
    /// - `world_x`: 世界像素 X 坐标
    /// - `world_y`: 世界像素 Y 坐标
    ///
    /// 返回: (grid_x, grid_y) 地图格子坐标
    #[inline]
    fn world_to_map(world_x: f32, world_y: f32) -> (i32, i32) {
        (
            (world_x / Self::CELL_WIDTH as f32).floor() as i32,
            (world_y / Self::CELL_HEIGHT as f32).floor() as i32,
        )
    }

    /// � 获取瓦片尺寸 (宽度, 高度)
    /// 对应 C# 的 Libraries.MapLibs[fileIndex].GetSize(index)
    fn get_tile_size(&self, file_index: i32, image_index: usize) -> Option<(i32, i32)> {
        if file_index < 0 {
            return None;
        }

        if let Some(mlib) = get_map_library(file_index as i16) {
            if let Ok(mut mlib) = mlib.lock() {
                if let Ok((w, h)) = mlib.get_size(image_index) {
                    return Some((w as i32, h as i32));
                }
            }
        }
        None
    }

    /// �🚪 获取或创建门对象
    fn get_or_create_door(&mut self, door_index: u8) -> &mut Door {
        // 查找现有门
        if let Some(pos) = self.doors.iter().position(|d| d.index == door_index) {
            return &mut self.doors[pos];
        }

        // 首次遇到，创建新门
        self.doors.push(Door::new(door_index));
        self.doors.last_mut().unwrap()
    }

    /// 🚪 获取门的当前动画帧 (0-8)
    fn get_door_frame(&self, door_index: u8) -> i32 {
        self.doors
            .iter()
            .find(|d| d.index == door_index)
            .map(|d| d.image_index)
            .unwrap_or(0)
    }

    /// 🎨 绘制地板三层 (Back/Middle/Front) - 静态层
    ///
    /// 对应 C# 的 DrawFloor() 方法
    /// 职责: 只渲染静态瓦片，不处理动画
    /// 动画渲染在 draw_effects() 中统一处理
    fn draw_floor(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        show_back: bool,
        show_middle: bool,
        show_front: bool,
        show_borders: bool,
    ) -> GameResult<()> {
        // 动画计数器已在draw()中更新，这里不重复更新

        // 计算可见区域 (世界坐标转地图格子)
        let left = camera.screen_to_world_x(0.0);
        let right = camera.screen_to_world_x(camera.screen_width);
        let top = camera.screen_to_world_y(0.0);
        let bottom = camera.screen_to_world_y(camera.screen_height);

        // Back/Middle层：标准边距
        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32 - 2).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32 + 2).min(self.width - 1);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32 - 2).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32 + 2).min(self.height - 1);

        // 🎨 Front层特殊处理：向下扩展更多格子
        // 原因：长纹理(树木、建筑)底部可能在屏幕下方很远的格子
        // 假设最高纹理为640像素(20个格子高度)，那么需要向下看20格
        let front_extra_cells = 20; // 向下额外扩展的格子数
        let front_start_y = start_y; // 上边界不变
        let front_end_y = (end_y + front_extra_cells).min(self.height - 1); // 下边界扩展

        // 🚀 性能优化：计算可见格子数量并限制渲染范围
        let visible_width = end_x - start_x + 1;
        let visible_height = end_y - start_y + 1;
        let total_cells = visible_width * visible_height;

        // 根据可见格子数量动态调整图层绘制策略
        let (draw_middle, draw_front) = if total_cells > 50000 {
            (false, false) // 超大范围：只绘制Back层
        } else if total_cells > 20000 {
            (false, true) // 大范围：Back + Front
        } else {
            (true, true) // 正常范围：绘制所有层
        };

        // ========================================
        // BACK LAYER (大地砖)
        // ========================================
        if show_back {
            // 传奇地图特点：Back层只渲染偶数行列，通过大瓦片(96x64)覆盖4个格子
            // 🔧 关键修复：必须从偶数坐标开始，不能直接用 step_by(2)
            let back_start_y = if start_y % 2 == 0 {
                start_y
            } else {
                start_y + 1
            };
            let back_start_x = if start_x % 2 == 0 {
                start_x
            } else {
                start_x + 1
            };

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
                        let (world_x, world_y) = Self::map_to_world(x, y);

                        self.draw_normal(
                            ctx,
                            canvas,
                            camera,
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
        // MIDDLE LAYER (小地砖 - 静态层)
        // ========================================
        if show_middle && draw_middle {
            // 🚀 大范围时跳过Middle层
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
                        let (world_x, world_y) = Self::map_to_world(x, y);

                        self.draw_normal(
                            ctx,
                            canvas,
                            camera,
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
        // FRONT LAYER (前景层 - 静态层)
        // ========================================
        if show_front && draw_front {
            // 🚀 大范围时跳过Front层
            // 🎨 使用扩展后的Y范围，确保屏幕下方的长纹理能被绘制
            for y in front_start_y..=front_end_y {
                for x in start_x..=end_x {
                    if let Some(cell) = self.get_cell(x, y) {
                        let index = (cell.front_image & 0x7FFF) - 1;
                        if index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                            continue;
                        }
                        // if cell.front_animation_frame > 0 {
                        //     continue;
                        // }
                        let (tile_width, tile_height) = if let Some(mlib) =
                            get_map_library(cell.front_index)
                        {
                            if let Ok(mut mlib) = mlib.lock() {
                                mlib.get_size(index as usize)
                                    .unwrap_or((Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16))
                            } else {
                                (Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16)
                            }
                        } else {
                            (Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16)
                        };

                        let (mut world_x, world_y_base) = Self::map_to_world(x, y);
                        let mut world_y = if (tile_width as i32 != Self::CELL_WIDTH
                            || tile_height as i32 != Self::CELL_HEIGHT)
                            && (tile_width as i32 != Self::CELL_WIDTH * 2
                                || tile_height as i32 != Self::CELL_HEIGHT * 2)
                        {
                            // 🔑 非标准尺寸 = 大型物体 (树/建筑等)
                            // C#: drawY = (y - mapPoint.Y + 1)*(CellHeight) 然后 drawY - s.Height
                            // 简化为: (y + 1) * CellHeight - s.Height = y * CellHeight + CellHeight - s.Height
                            world_y_base + Self::CELL_HEIGHT as f32 - tile_height as f32
                        } else {
                            // 标准地板瓦片 (48×32 或 96×64)
                            // C#: drawY = (y - mapPoint.Y)*(CellHeight)

                            world_y_base
                        };

                        let use_blend = (cell.front_animation_frame & 0x80) != 0;
                        if use_blend {
                            world_x = world_x - 1.0 * Self::CELL_WIDTH as f32; // 混合模式的Front层纹理向左偏移4像素
                            world_y = world_y - 4.0 * Self::CELL_HEIGHT as f32; // 混合模式的Front层纹理向上偏移10像素
                        }
                        self.draw_blend(
                            ctx,
                            canvas,
                            camera,
                            cell.front_index as i32,
                            index as usize,
                            world_x,
                            world_y,
                            show_borders,
                            Color::from_rgb(0, 150, 255),
                            use_blend, // 根据动画标记决定混合模式
                            false,     // Front静态层不应用纹理偏移
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 🔥 绘制地图动画和特效
    ///
    /// 对应 C# 的 DrawObjects() 方法
    /// 职责: 渲染所有动态内容 (动画瓦片、门、对象、特效等)
    ///
    /// 包括:
    /// - Shanda瓦片动画 (TileAnimationImage - 库190)
    /// - Middle层动画 (MiddleAnimationFrame)
    /// - Front层动画 (FrontAnimationFrame + 门动画)
    fn draw_effects(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
    ) -> GameResult<()> {
        // 计算可见区域
        let left = camera.screen_to_world_x(0.0);
        let right = camera.screen_to_world_x(camera.screen_width);
        let top = camera.screen_to_world_y(0.0);
        let bottom = camera.screen_to_world_y(camera.screen_height);

        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32 - 2).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32 + 2).min(self.width - 1);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32 - 2).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32 + 2).min(self.height - 1);

        // Front层特殊处理：向下扩展更多格子
        let front_extra_cells = 20;
        let front_start_y = start_y;
        let front_end_y = (end_y + front_extra_cells).min(self.height - 1);

        // ========================================
        // 1️⃣ TileAnimationImage (库190 - Shanda动画)
        // ========================================
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    let tile_index = cell.tile_animation_image;
                    let tile_frames = cell.tile_animation_frames;

                    if tile_index > 0 && tile_frames > 0 {
                        let mut index = (tile_index - 1) as i32;
                        let animation_offset = (cell.tile_animation_offset ^ 0x2000) as i32;
                        index += animation_offset * (self.animation_count % tile_frames as i32);

                        if index >= 0 {
                            // 🔍 获取纹理高度用于DrawUp偏移
                            // C#: DrawUp() 会自动执行 y -= mi.Height
                            let tile_height = if let Some(mlib) = get_map_library(190) {
                                if let Ok(mut mlib) = mlib.lock() {
                                    mlib.get_size(index as usize)
                                        .map(|(_, h)| h as f32)
                                        .unwrap_or(Self::CELL_HEIGHT as f32)
                                } else {
                                    Self::CELL_HEIGHT as f32
                                }
                            } else {
                                Self::CELL_HEIGHT as f32
                            };

                            let (world_x, world_y_base) = Self::map_to_world(x, y);
                            // 🎯 DrawUp效果：向上偏移纹理高度 (对应 C# y -= mi.Height)
                            let world_y = world_y_base - tile_height;

                            self.draw_blend(
                                ctx,
                                canvas,
                                camera,
                                190,
                                index as usize,
                                world_x,
                                world_y,
                                false,
                                Color::WHITE,
                                true,  // TileAnimationImage使用自定义混合
                                false, // DrawUp不应用纹理偏移（C#: DrawUp无offSet参数）
                            )?;
                        }
                    }
                }
            }
        }

        // ========================================
        // 2️⃣ Middle层动画 (对应 C# DrawObjects 中的 mir3 middle layer)
        // ========================================
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    let mut index = cell.middle_image - 1;
                    if index < 0 || cell.middle_index == -1 {
                        continue;
                    }

                    let mut animation = cell.middle_animation_frame;
                    if animation > 0 && animation < 255 {
                        let use_blend = (animation & 0x0f) > 0;
                        animation &= 0x0f;

                        if animation > 0 {
                            let animation_tick = cell.middle_animation_tick;
                            let total_frames =
                                animation as i32 + (animation as i32 * animation_tick as i32);
                            let frame_offset =
                                (self.animation_count % total_frames) / (1 + animation_tick as i32);
                            index += frame_offset;

                            let (world_x, world_y) = Self::map_to_world(x, y);

                            // 绘制动画瓦片
                            let should_draw = if let Some(mlib) = get_map_library(cell.middle_index)
                            {
                                if let Ok(mut mlib) = mlib.lock() {
                                    if let Ok((w, h)) = mlib.get_size(index as usize) {
                                        // 只绘制非标准尺寸或需要blend的瓦片
                                        ((w as i32 != Self::CELL_WIDTH
                                            || h as i32 != Self::CELL_HEIGHT)
                                            && (w as i32 != Self::CELL_WIDTH * 2
                                                || h as i32 != Self::CELL_HEIGHT * 2))
                                            || use_blend
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if should_draw {
                                self.draw_blend(
                                    ctx,
                                    canvas,
                                    camera,
                                    cell.middle_index as i32,
                                    index as usize,
                                    world_x,
                                    world_y,
                                    false,
                                    Color::WHITE,
                                    use_blend && (animation == 10 || animation == 8),
                                    false, // Middle动画层不应用纹理偏移
                                )?;
                            }
                        }
                    }
                }
            }
        }

        // ========================================
        // 3️⃣ Front层动画 + 门动画 (对应 C# DrawObjects 中的 front layer)
        // ========================================
        for y in front_start_y..=front_end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    let mut index = (cell.front_image & 0x7FFF) - 1;
                    if index < 0 || cell.front_index == -1 || cell.front_index == 200 {
                        continue;
                    }

                    let mut animation = cell.front_animation_frame;
                    let use_blend = (animation & 0x80) != 0;
                    animation &= 0x7F;

                    // 只渲染有动画或门的瓦片
                    if animation == 0 && cell.door_index == 0 {
                        continue;
                    }

                    // 动画帧推进
                    if animation > 0 {
                        let animation_tick = cell.front_animation_tick;
                        let total_frames =
                            animation as i32 + (animation as i32 * animation_tick as i32);
                        let frame_offset =
                            (self.animation_count % total_frames) / (1 + animation_tick as i32);
                        index += frame_offset;
                    }

                    // 门动画处理
                    if cell.door_index > 0 {
                        let door_frame = self.get_door_frame(cell.door_index);
                        if door_frame > 0 {
                            index += (door_frame + 1) * cell.door_offset as i32;
                        }
                    }

                    // 尺寸检查：判断是否应该渲染
                    let should_draw = if let Some(mlib) = get_map_library(cell.front_index) {
                        if let Ok(mut mlib) = mlib.lock() {
                            if let Ok((w, h)) = mlib.get_size(index as usize) {
                                // 标准尺寸 (48x32 或 96x64) 且无动画，跳过
                                if ((w as i32 == Self::CELL_WIDTH && h as i32 == Self::CELL_HEIGHT)
                                    || (w as i32 == Self::CELL_WIDTH * 2
                                        && h as i32 == Self::CELL_HEIGHT * 2))
                                    && animation == 0
                                {
                                    false
                                } else {
                                    true
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if should_draw {
                        // 🔍 获取瓦片尺寸以判断是地板瓦片还是大型物体
                        // 与Front静态层保持一致的处理逻辑
                        let (tile_width, tile_height) = if let Some(mlib) =
                            get_map_library(cell.front_index)
                        {
                            if let Ok(mut mlib) = mlib.lock() {
                                mlib.get_size(index as usize)
                                    .unwrap_or((Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16))
                            } else {
                                (Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16)
                            }
                        } else {
                            (Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16)
                        };

                        let (mut world_x, world_y_base) = Self::map_to_world(x, y);
                        let mut world_y = if (tile_width as i32 != Self::CELL_WIDTH
                            || tile_height as i32 != Self::CELL_HEIGHT)
                            && (tile_width as i32 != Self::CELL_WIDTH * 2
                                || tile_height as i32 != Self::CELL_HEIGHT * 2)
                        {
                            // 🔑 非标准尺寸 = 大型物体 (树/建筑等)
                            // 底部对齐：(y + 1) * CELL_HEIGHT - tile_height
                            world_y_base + Self::CELL_HEIGHT as f32 - tile_height as f32
                        } else {
                            // 标准地板瓦片 (48×32 或 96×64)
                            world_y_base
                        };

                        // 🎯 Front层动画偏移规则 (对应 C# GameScene.cs:11970)
                        // 特殊库 (14, 27, 100-199) 使用纹理偏移
                        // C#: DrawBlend(index, new Point(drawX, drawY - 3*CellHeight), Color.White, true)
                        //                                                                             ^^^^
                        let apply_offset = cell.front_index == 14
                            || cell.front_index == 27
                            || (cell.front_index > 99 && cell.front_index < 199);
                        if use_blend {
                            world_x = world_x - 1.0 * Self::CELL_WIDTH as f32; // 混合模式的Front层纹理向左偏移4像素
                            world_y = world_y - 4.0 * Self::CELL_HEIGHT as f32; // 混合模式的Front层纹理向上偏移10像素
                        }
                        self.draw_blend(
                            ctx,
                            canvas,
                            camera,
                            cell.front_index as i32,
                            index as usize,
                            world_x,
                            world_y,
                            false,
                            Color::WHITE,
                            use_blend,
                            apply_offset, // 特殊库应用纹理偏移
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 绘制玩家/怪物/NPC
    fn draw_objects(
        &mut self,
        _ctx: &mut Context,
        _canvas: &mut Canvas,
        _camera: &Camera,
    ) -> GameResult<()> {
        // TODO: 实现对象渲染
        Ok(())
    }

    /// 绘制UI元素
    fn draw_ui(
        &mut self,
        _ctx: &mut Context,
        _canvas: &mut Canvas,
        _camera: &Camera,
    ) -> GameResult<()> {
        // TODO: 实现UI渲染
        Ok(())
    }

    /// 🎬 绘制所有屏幕元素 (完整渲染管线)
    ///
    /// 渲染顺序:
    /// 1. draw_floor() - 地板三层 (Back/Middle/Front + 门动画)
    /// 2. draw_effects() - 动画和特效
    /// 3. [未来扩展] draw_objects() - 玩家/怪物/NPC
    /// 4. [未来扩展] draw_ui() - UI元素(血条/名字等)
    fn draw(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        show_back: bool,
        show_middle: bool,
        show_front: bool,
        show_borders: bool,
        show_animations: bool,
    ) -> GameResult<()> {
        // 更新动画计数器 (全局动画时钟)
        self.animation_count = (self.animation_count + 1) % 1000;

        // 🎨 步骤1: 绘制地板三层
        self.draw_floor(
            ctx,
            canvas,
            camera,
            show_back,
            show_middle,
            show_front,
            show_borders,
        )?;

        // 🔥 步骤2: 绘制动画和特效 (仅当开关开启时)
        if show_animations {
            self.draw_effects(ctx, canvas, camera)?;
        }

        // 🎮 步骤3: [未来] 绘制对象 (玩家/怪物/NPC)
        // self.draw_objects(ctx, canvas, camera)?;

        // 📊 步骤4: [未来] 绘制UI (血条/名字/聊天/伤害数字)
        // self.draw_ui(ctx, canvas, camera)?;

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
    fn draw_normal(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
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
                        // 使用Camera的坐标转换方法
                        let screen_x = camera.world_to_screen_x(world_x);
                        let screen_y = camera.world_to_screen_y(world_y);

                        // 应用图像offset（如果需要的话）
                        let final_x = screen_x;
                        let final_y = screen_y;
                        canvas.set_blend_mode(graphics::BlendMode::REPLACE);
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

    fn draw_blend(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        lib_index: i32,
        image_index: usize,
        world_x: f32,
        world_y: f32,
        show_border: bool,
        border_color: Color,
        use_blend: bool,    // 🔥 是否使用自定义混合模式
        apply_offset: bool, // 🎯 是否应用纹理的 (X, Y) 偏移 (对应 C# 的 offSet 参数)
    ) -> GameResult<()> {
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();

            // 使用纹理缓存（先获取纹理再读取 info，避免重复 lock）
            match lib.get_or_create_texture(ctx, image_index) {
                Ok(info) => {
                    // 从ImageInfo获取实际的纹理
                    if let Some(ref texture) = info.image {
                        // 使用Camera的坐标转换方法
                        let screen_x = camera.world_to_screen_x(world_x);
                        let screen_y = camera.world_to_screen_y(world_y);

                        // 🎯 应用纹理偏移 (对应 C# if (offSet) point.Offset(mi.X, mi.Y))
                        let final_x = if apply_offset {
                            screen_x + info.x as f32 * camera.zoom
                        } else {
                            screen_x
                        };
                        let final_y = if apply_offset {
                            screen_y + info.y as f32 * camera.zoom
                        } else {
                            screen_y
                        };

                        // 🔥 使用自定义混合模式
                        // C#原版: SourceBlend=SourceAlpha, DestinationBlend=One
                        // 效果: 半透明发光，黑色区域完全透明
                        if use_blend {
                            canvas.set_blend_mode(Self::create_blend_mode());
                        } else {
                            canvas.set_blend_mode(graphics::BlendMode::ALPHA);
                        }

                        // �🔧 应用缩放到瓦片绘制
                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([final_x, final_y])
                                .scale([camera.zoom, camera.zoom]),
                        );

                        // if use_blend {
                        //     canvas.set_blend_mode(graphics::BlendMode::ALPHA);
                        // }

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
    fn draw_grid(&self, ctx: &mut Context, canvas: &mut Canvas, camera: &Camera) -> GameResult<()> {
        // 计算可见区域
        let left = camera.screen_to_world_x(0.0);
        let right = camera.screen_to_world_x(camera.screen_width);
        let top = camera.screen_to_world_y(0.0);
        let bottom = camera.screen_to_world_y(camera.screen_height);

        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32).max(0);
        let end_x = ((right / Self::CELL_WIDTH as f32).ceil() as i32).min(self.width);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32).max(0);
        let end_y = ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32).min(self.height);

        let grid_color = Color::from_rgba(0, 255, 0, 120);

        // 绘制垂直线
        for x in start_x..=end_x {
            let (world_x, _) = Self::map_to_world(x, 0);
            // 使用Camera的坐标转换
            let screen_x = camera.world_to_screen_x(world_x);

            if screen_x >= 0.0 && screen_x <= camera.screen_width {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[screen_x, 0.0], [screen_x, camera.screen_height]],
                    1.0,
                    grid_color,
                )?;
                canvas.draw(&line, DrawParam::default());
            }
        }

        // 绘制水平线
        for y in start_y..=end_y {
            let (_, world_y) = Self::map_to_world(0, y);
            // 使用Camera的坐标转换
            let screen_y = camera.world_to_screen_y(world_y);

            if screen_y >= 0.0 && screen_y <= camera.screen_height {
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[[0.0, screen_y], [camera.screen_width, screen_y]],
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
    ) -> GameResult<()> {
        // 计算可见区域
        let left = camera.screen_to_world_x(0.0);
        let right = camera.screen_to_world_x(camera.screen_width);
        let top = camera.screen_to_world_y(0.0);
        let bottom = camera.screen_to_world_y(camera.screen_height);

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
                    // 检查是否有障碍物
                    let has_obstacle = (cell.back_image & 0x20000000) != 0  // HighWall (山、水等不可行走地形)
                        || (cell.door_offset & 0x80) != 0                   // DoorClosed
                        || (cell.door_index & 0x80) != 0                    // Block
                        || (cell.front_image & 0x8000) != 0; // MiddleBlock (LowWall)

                    if has_obstacle {
                        let (world_x, world_y) = Self::map_to_world(x, y);

                        // 使用Camera的坐标转换
                        let screen_x = camera.world_to_screen_x(world_x);
                        let screen_y = camera.world_to_screen_y(world_y);

                        // 🔧 绘制半透明矩形表示障碍，尺寸也要缩放
                        let obstacle_rect = graphics::Mesh::new_rectangle(
                            ctx,
                            graphics::DrawMode::fill(),
                            graphics::Rect::new(
                                screen_x,
                                screen_y,
                                Self::CELL_WIDTH as f32 * camera.zoom,
                                Self::CELL_HEIGHT as f32 * camera.zoom,
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
    show_animations: bool,   // A键：显示动画
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

        // 🔧 相机初始位置：地图中心
        let mut camera = Camera::new(screen_width, screen_height);
        // 计算地图中心的世界坐标
        let map_center_x = (map_renderer.width / 2) as f32 * MapRenderer::CELL_WIDTH as f32;
        let map_center_y = (map_renderer.height / 2) as f32 * MapRenderer::CELL_HEIGHT as f32;
        camera.x = map_center_x;
        camera.y = map_center_y;

        println!(
            "📍 相机初始位置: 地图中心 世界坐标({:.1}, {:.1})",
            camera.x, camera.y
        );
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
            show_grid: false,
            show_borders: false,
            show_layer_back: true,
            show_layer_middle: true,
            show_layer_front: true,
            show_obstacles: false,
            show_animations: true, // 默认开启动画
        })
    }

    /// 重新加载地图
    fn reload_map(&mut self, map_path: &str) -> GameResult<()> {
        println!("\n🔄 重新加载地图: {}", map_path);

        let reader = MapReader::new(map_path).map_err(|e| {
            eprintln!("❌ 加载地图失败: {}", e);
            ggez::GameError::ResourceLoadError(format!("Failed to load map: {}", e))
        })?;

        println!("✅ 地图加载成功: {}x{}", reader.width, reader.height);

        self.map_renderer = MapRenderer::new(reader);

        // 重置相机到新地图中心
        let map_center_x = (self.map_renderer.width / 2) as f32 * MapRenderer::CELL_WIDTH as f32;
        let map_center_y = (self.map_renderer.height / 2) as f32 * MapRenderer::CELL_HEIGHT as f32;
        self.camera.x = map_center_x;
        self.camera.y = map_center_y;
        self.camera.zoom = 1.0;

        self.map_name = map_path.to_string();

        println!(
            "📍 相机重置到地图中心: 世界坐标({:.1}, {:.1})",
            self.camera.x, self.camera.y
        );
        println!(
            "🎯 地图像素尺寸: {}x{} 像素",
            self.map_renderer.width * MapRenderer::CELL_WIDTH,
            self.map_renderer.height * MapRenderer::CELL_HEIGHT
        );

        Ok(())
    }

    /// 打开文件选择对话框
    fn open_map_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("地图文件", &["map"])
            .set_title("选择地图文件")
            .pick_file()
        {
            if let Err(e) = self.reload_map(path.to_str().unwrap_or("")) {
                eprintln!("❌ 加载地图失败: {}", e);
            }
        }
    }
}

impl MapViewerState {
    fn draw_ui(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 计算当前鼠标悬停的地图格子坐标
        let mouse_pos = ctx.mouse.position();
        let mouse_x = mouse_pos.x;
        let mouse_y = mouse_pos.y;
        let world_x = self.camera.screen_to_world_x(mouse_x);
        let world_y = self.camera.screen_to_world_y(mouse_y);
        let (grid_x, grid_y) = MapRenderer::world_to_map(world_x, world_y);

        // 构建顶部状态栏
        let status_text = format!(
            "Map: {} | Size: {}x{} | FPS: {} | Zoom: {:.2}x | Camera: ({:.0}, {:.0}) | Grid: ({}, {})\nG-网格 | B-边框 | 1/2/3-图层 | O-障碍 | A-动画 | M-选择地图",
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
        self.camera.update_screen_size(w, h);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 使用黑色背景
        let mut canvas = Canvas::from_frame(ctx, Color::from_rgb(0, 0, 0));

        // 🎨 使用REPLACE混合模式 - 直接替换像素,不混合,避免发白
        // REPLACE: 完全不透明的纹理直接覆盖背景
        // 适合地图瓦片这种不需要半透明混合的场景

        // 绘制地图
        self.map_renderer.draw(
            ctx,
            &mut canvas,
            &self.camera,
            self.show_layer_back,
            self.show_layer_middle,
            self.show_layer_front,
            self.show_borders,
            self.show_animations,
        )?;

        // 绘制网格
        if self.show_grid {
            self.map_renderer
                .draw_grid(ctx, &mut canvas, &self.camera)?;
        }

        // 绘制障碍层
        if self.show_obstacles {
            self.map_renderer
                .draw_obstacles(ctx, &mut canvas, &self.camera)?;
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
        self.camera.zoom_by(y, mouse_pos.x, mouse_pos.y);
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
                KeyCode::KeyA => {
                    self.show_animations = !self.show_animations;
                    println!(
                        "🎬 动画效果: {}",
                        if self.show_animations {
                            "开启"
                        } else {
                            "关闭"
                        }
                    );
                }
                KeyCode::KeyM => {
                    println!("📂 打开地图选择对话框...");
                    self.open_map_dialog();
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
    println!("  - A键 - 切换动画效果");
    println!("  - 1/2/3键 - 切换Back/Middle/Front层");
    println!("  - M键 - 选择地图文件");
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
