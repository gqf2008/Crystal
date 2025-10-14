// 地图渲染器 - 拥有地图数据并负责渲染
//
// 职责:
// - 拥有地图数据（cells, doors, width, height）
// - 负责所有图层的绘制逻辑
// - 提供地图数据访问接口（给寻路、碰撞检测等系统使用）
//
// 从 map_viewer.rs 移植的 MapRenderer 完整实现

use super::camera::Camera;
use ggez::{
    graphics::{
        BlendComponent, BlendFactor, BlendMode, BlendOperation, Canvas, Color, DrawParam,
    },
    Context, GameResult,
};
use crate::graphics::libraries::get_map_library;
use crate::objects::{CellInfo, MapReader};
use std::time::Instant;

/// 门结构体（带动画状态）
#[derive(Debug, Clone)]
pub struct Door {
    pub index: u8,                 // 门ID (DoorIndex)
    pub door_state: u8,            // 门状态：0=关闭, 1=开启中, 2=已开启, 3=关闭中
    pub image_index: i32,          // 当前动画帧 (0-8, 共9帧)
    pub last_tick: Instant,        // 上次动画更新时间
}

impl Door {
    fn new(index: u8) -> Self {
        Self {
            index,
            door_state: 0,
            image_index: 0,
            last_tick: Instant::now(),
        }
    }
}

/// 地图渲染器 - 拥有地图数据
pub struct MapRenderer {
    // 🗺️ 地图网格数据
    cells: Vec<Vec<CellInfo>>,
    pub width: i32,
    pub height: i32,
    
    // 🚪 门列表（动态管理）
    doors: Vec<Door>,
    
    // 🎬 动画计数器
    animation_count: i32,
}

impl Default for MapRenderer {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            width: 0,
            height: 0,
            doors: Vec::new(),
            animation_count: 0,
        }
    }
}

impl MapRenderer {
    // 传奇地图格子尺寸（公开常量，供 GameScene 等外部模块使用）
    pub const CELL_WIDTH: i32 = 48;
    pub const CELL_HEIGHT: i32 = 32;

    /// 从 MapReader 构造 MapRenderer（拥有数据所有权）
    pub fn from_reader(reader: MapReader) -> Self {
        let width = reader.width;
        let height = reader.height;
        let cells = reader.map_cells; // 转移所有权（MapReader 字段名是 map_cells）
        
        Self {
            cells,
            width,
            height,
            doors: Vec::new(),        // 门列表初始化为空（运行时动态创建）
            animation_count: 0,
        }
    }

    /// 获取指定坐标的格子信息（边界安全）
    pub fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        self.cells.get(y as usize)?.get(x as usize)
    }

    /// 判断指定坐标是否可行走
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.get_cell(x, y)
            .map(|cell| cell.is_walkable()) // 调用 CellInfo 的 is_walkable() 方法
            .unwrap_or(false)
    }

    /// 获取或创建门对象（运行时管理）
    pub fn get_or_create_door(&mut self, index: u8) -> &mut Door {
        // 先查找是否已存在
        if let Some(pos) = self.doors.iter().position(|d| d.index == index) {
            return &mut self.doors[pos];
        }
        
        // 不存在则创建新门
        self.doors.push(Door::new(index));
        self.doors.last_mut().unwrap()
    }

    /// 获取门的当前动画帧
    pub fn get_door_frame(&self, index: u8) -> i32 {
        self.doors
            .iter()
            .find(|d| d.index == index)
            .map(|d| d.image_index)
            .unwrap_or(0)
    }

    /// 🔥 创建传奇特效混合模式
    ///
    /// 对应 C# 的混合设置:
    /// ```csharp
    /// Device.SetRenderState(RenderState.SourceBlend, Blend.SourceAlpha);
    /// Device.SetRenderState(RenderState.DestinationBlend, Blend.One);
    /// ```
    ///
    /// 混合公式: 最终颜色 = 源颜色 × 源Alpha + 背景颜色 × 1
    #[inline]
    fn create_blend_mode() -> BlendMode {
        BlendMode {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        }
    }

    /// 🗺️ 地图格子坐标 → 世界像素坐标
    #[inline]
    pub fn map_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        (
            (grid_x * Self::CELL_WIDTH) as f32,
            (grid_y * Self::CELL_HEIGHT) as f32,
        )
    }

    /// 🎨 Back层绘制 (大地砖层 - 仅静态)
    fn draw_back(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
    ) -> GameResult<()> {
        // 传奇地图特点：Back层只渲染偶数行列
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
            for x in (back_start_x..=end_x).step_by(2) {
                if let Some(cell) = self.get_cell(x, y) {
                    let index = (cell.back_image & 0x1FFFFFFF) - 1;
                    if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
                        continue;
                    }

                    let (world_x, world_y) = Self::map_to_world(x, y);
                    self.draw_tile_normal(
                        ctx,
                        canvas,
                        camera,
                        cell.back_index as i32,
                        index as usize,
                        world_x,
                        world_y,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// 🎨 Middle层绘制 (小地砖层 - 静态 + 动画)
    fn draw_middle(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
    ) -> GameResult<()> {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    let mut index = cell.middle_image - 1;
                    if index < 0 || cell.middle_index == -1 {
                        continue;
                    }

                    let mut animation = cell.middle_animation_frame;
                    let has_animation = animation > 0 && animation < 255;

                    // 静态瓦片：无动画的标准尺寸瓦片
                    if !has_animation {
                        // 尺寸过滤：只渲染 48x32 或 96x64 的瓦片
                        if let Some(mlib) = get_map_library(cell.middle_index) {
                            if let Ok(mut mlib) = mlib.lock() {
                                if let Ok((w, h)) = mlib.get_size(index as usize) {
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

                        let (world_x, world_y) = Self::map_to_world(x, y);
                        self.draw_tile_normal(
                            ctx,
                            canvas,
                            camera,
                            cell.middle_index as i32,
                            index as usize,
                            world_x,
                            world_y,
                        )?;
                    } else {
                        // 动画瓦片：有动画的格子
                        let use_blend = (animation & 0x0f) > 0;
                        animation &= 0x0f;

                        if animation > 0 {
                            let animation_tick = cell.middle_animation_tick;
                            let total_frames =
                                animation as i32 + (animation as i32 * animation_tick as i32);
                            let frame_offset =
                                (self.animation_count % total_frames) / (1 + animation_tick as i32);
                            index += frame_offset;

                            // 尺寸检查：只绘制非标准尺寸或需要blend的瓦片
                            let should_draw = if let Some(mlib) = get_map_library(cell.middle_index)
                            {
                                if let Ok(mut mlib) = mlib.lock() {
                                    if let Ok((w, h)) = mlib.get_size(index as usize) {
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
                                let (world_x, world_y) = Self::map_to_world(x, y);
                                self.draw_tile_blend(
                                    ctx,
                                    canvas,
                                    camera,
                                    cell.middle_index as i32,
                                    index as usize,
                                    world_x,
                                    world_y,
                                    use_blend && (animation == 10 || animation == 8),
                                    1.0,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 🎨 Front层绘制 (前景层 - 静态 + 动画 + 门)
    fn draw_front(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
    ) -> GameResult<()> {
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if let Some(cell) = self.get_cell(x, y) {
                    let mut index = (cell.front_image & 0x7FFF) - 1;
                    if index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                        continue;
                    }

                    let mut animation = cell.front_animation_frame;
                    let use_blend = (animation & 0x80) != 0;
                    animation &= 0x7F;

                    let has_animation = animation > 0;
                    let has_door = cell.door_index > 0;

                    // 动画帧推进（如果有动画）
                    if has_animation {
                        let animation_tick = cell.front_animation_tick;
                        let total_frames =
                            animation as i32 + (animation as i32 * animation_tick as i32);
                        let frame_offset =
                            (self.animation_count % total_frames) / (1 + animation_tick as i32);
                        index += frame_offset;
                    }

                    // 门动画处理
                    if has_door {
                        let door_frame = self.get_door_frame(cell.door_index);
                        if door_frame > 0 {
                            index += (door_frame + 1) * cell.door_offset as i32;
                        }
                    }

                    // 获取瓦片尺寸
                    let (tile_width, tile_height) =
                        if let Some(mlib) = get_map_library(cell.front_index) {
                            if let Ok(mut mlib) = mlib.lock() {
                                mlib.get_size(index as usize)
                                    .unwrap_or((Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16))
                            } else {
                                (Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16)
                            }
                        } else {
                            (Self::CELL_WIDTH as i16, Self::CELL_HEIGHT as i16)
                        };

                    // 计算世界坐标
                    let (mut world_x, world_y_base) = Self::map_to_world(x, y);
                    let mut world_y = if (tile_width as i32 != Self::CELL_WIDTH
                        || tile_height as i32 != Self::CELL_HEIGHT)
                        && (tile_width as i32 != Self::CELL_WIDTH * 2
                            || tile_height as i32 != Self::CELL_HEIGHT * 2)
                    {
                        // 非标准尺寸 = 大型物体 (树/建筑等)
                        world_y_base + Self::CELL_HEIGHT as f32 - tile_height as f32
                    } else {
                        // 标准地板瓦片
                        world_y_base
                    };

                    // 混合模式偏移
                    if use_blend {
                        world_x = world_x - 1.0 * Self::CELL_WIDTH as f32;
                        world_y = world_y - 4.0 * Self::CELL_HEIGHT as f32;
                    }

                    // 亮度控制：有火焰效果时变亮
                    let brightness = if use_blend && !has_animation {
                        1.5
                    } else {
                        1.0
                    };

                    self.draw_tile_blend(
                        ctx,
                        canvas,
                        camera,
                        cell.front_index as i32,
                        index as usize,
                        world_x,
                        world_y,
                        use_blend,
                        brightness,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// 🎬 主渲染入口 - 绘制所有图层
    ///
    /// 参数:
    /// - `camera`: 相机引用
    pub fn draw(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
    ) -> GameResult<()> {
        // 更新动画计数器
        self.animation_count = (self.animation_count + 1) % 1000;

        // 计算可见区域 (世界坐标转地图格子)
        let left = camera.screen_to_world_x(0.0);
        let right = camera.screen_to_world_x(camera.screen_width);
        let top = camera.screen_to_world_y(0.0);
        let bottom = camera.screen_to_world_y(camera.screen_height);

        // 标准边距
        let start_x = ((left / Self::CELL_WIDTH as f32).floor() as i32 - 2).max(0);
        let end_x =
            ((right / Self::CELL_WIDTH as f32).ceil() as i32 + 2).min(self.width - 1);
        let start_y = ((top / Self::CELL_HEIGHT as f32).floor() as i32 - 2).max(0);
        let end_y =
            ((bottom / Self::CELL_HEIGHT as f32).ceil() as i32 + 2).min(self.height - 1);

        // Front层特殊处理：向下扩展更多格子
        let front_extra_cells = 20;
        let front_start_y = start_y;
        let front_end_y = (end_y + front_extra_cells).min(self.height - 1);

        // 性能优化：根据可见格子数量动态调整
        let visible_width = end_x - start_x + 1;
        let visible_height = end_y - start_y + 1;
        let total_cells = visible_width * visible_height;

        let (draw_middle, draw_front) = if total_cells > 50000 {
            (false, false)
        } else if total_cells > 20000 {
            (false, true)
        } else {
            (true, true)
        };

        // 🎨 分层绘制
        // Back层 - 仅静态
        self.draw_back(ctx, canvas, camera, start_x, end_x, start_y, end_y)?;

        // Middle层 - 静态+动画
        if draw_middle {
            self.draw_middle(ctx, canvas, camera, start_x, end_x, start_y, end_y)?;
        }

        // Front层 - 静态+动画+门
        if draw_front {
            self.draw_front(
                ctx,
                canvas,
                camera,
                start_x,
                end_x,
                front_start_y,
                front_end_y,
            )?;
        }

        Ok(())
    }

    /// 绘制普通瓦片 (不使用混合模式)
    fn draw_tile_normal(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        lib_index: i32,
        image_index: usize,
        world_x: f32,
        world_y: f32,
    ) -> GameResult<()> {
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();

            match lib.get_or_create_texture(ctx, image_index) {
                Ok(info) => {
                    if let Some(ref texture) = info.image {
                        let screen_x = camera.world_to_screen_x(world_x);
                        let screen_y = camera.world_to_screen_y(world_y);

                        canvas.set_blend_mode(ggez::graphics::BlendMode::REPLACE);
                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([screen_x, screen_y])
                                .scale([camera.zoom, camera.zoom])
                                .color(Color::WHITE),
                        );
                    }
                }
                Err(_) => {
                    // 忽略加载错误
                }
            }
        }

        Ok(())
    }

    /// 绘制混合瓦片 (使用特效混合模式)
    fn draw_tile_blend(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera,
        lib_index: i32,
        image_index: usize,
        world_x: f32,
        world_y: f32,
        use_blend: bool,
        brightness: f32,
    ) -> GameResult<()> {
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();

            match lib.get_or_create_texture(ctx, image_index) {
                Ok(info) => {
                    if let Some(ref texture) = info.image {
                        let screen_x = camera.world_to_screen_x(world_x);
                        let screen_y = camera.world_to_screen_y(world_y);

                        // 设置混合模式
                        if use_blend {
                            canvas.set_blend_mode(Self::create_blend_mode());
                        } else {
                            canvas.set_blend_mode(ggez::graphics::BlendMode::ALPHA);
                        }

                        // 亮度控制
                        let draw_color = if brightness > 1.0 {
                            Color::new(brightness, brightness, brightness, 1.0)
                        } else {
                            Color::WHITE
                        };

                        canvas.draw(
                            texture,
                            DrawParam::default()
                                .dest([screen_x, screen_y])
                                .scale([camera.zoom, camera.zoom])
                                .color(draw_color),
                        );
                    }
                }
                Err(_) => {
                    // 忽略加载错误
                }
            }
        }

        Ok(())
    }
}
