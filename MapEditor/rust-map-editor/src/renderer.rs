use crate::map_code::{CellInfo, MapReader};
use crate::mlibrary::MLibrary;
use ggez::graphics::{Canvas, Color, DrawParam, Mesh, Rect};
use ggez::{Context, GameResult};
use glam::Vec2;

const CELL_WIDTH: f32 = 48.0;
const CELL_HEIGHT: f32 = 32.0;

pub struct Renderer {
    pub zoom: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub map_point_x: i32,
    pub map_point_y: i32,
    pub animation_count: u32,
    
    // 显示选项
    pub show_back: bool,
    pub show_middle: bool,
    pub show_front: bool,
    pub show_grid: bool,
    pub show_front_tag: bool,
    pub show_middle_tag: bool,
    pub show_door_tag: bool,
    pub show_light_tag: bool,
    pub show_back_mask: bool,
    pub show_front_mask: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            zoom: 1.0,
            offset_x: 0,
            offset_y: 0,
            map_point_x: 0,
            map_point_y: 0,
            animation_count: 0,
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: true,
            show_front_tag: false,
            show_middle_tag: false,
            show_door_tag: false,
            show_light_tag: false,
            show_back_mask: false,
            show_front_mask: false,
        }
    }

    pub fn update(&mut self, _ctx: &mut Context) {
        self.animation_count = self.animation_count.wrapping_add(1);
    }

    /// 渲染整个场景
    pub fn render(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        map: &MapReader,
        _libraries: &Vec<MLibrary>,
    ) -> GameResult {
        // 计算可视区域
        let screen_width = ctx.gfx.drawable_size().0;
        let screen_height = ctx.gfx.drawable_size().1;
        let cells_x = (screen_width / (CELL_WIDTH * self.zoom)) as i32 + 2;
        let cells_y = (screen_height / (CELL_HEIGHT * self.zoom)) as i32 + 2;

        // 绘制背景层
        if self.show_back {
            self.draw_back_layer(ctx, canvas, map, cells_x, cells_y)?;
        }

        // 绘制中间层
        if self.show_middle {
            self.draw_middle_layer(ctx, canvas, map, cells_x, cells_y)?;
        }

        // 绘制前景层
        if self.show_front {
            self.draw_front_layer(ctx, canvas, map, cells_x, cells_y)?;
        }

        // 绘制网格
        if self.show_grid {
            self.draw_grid(ctx, canvas, cells_x, cells_y)?;
        }

        // 绘制标记
        if self.show_front_tag {
            self.draw_front_tags(ctx, canvas, map, cells_x, cells_y)?;
        }

        Ok(())
    }

    fn draw_back_layer(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        map: &MapReader,
        cells_x: i32,
        cells_y: i32,
    ) -> GameResult {
        for y in (self.map_point_y - 1)..(self.map_point_y + cells_y) {
            if y % 2 != 0 || y < 0 || y >= map.height as i32 {
                continue;
            }
            
            for x in (self.map_point_x - 1)..(self.map_point_x + cells_x) {
                if x % 2 != 0 || x < 0 || x >= map.width as i32 {
                    continue;
                }

                if x >= 0 && y >= 0 && (x as usize) < map.width && (y as usize) < map.height {
                    let cell = &map.map_cells[x as usize][y as usize];
                    let back_index = (cell.back_image & 0x1FFFFFFF) - 1;
                    if back_index >= 0 {
                        self.draw_cell(ctx, canvas, x, y, Color::from_rgb(200, 200, 255))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_middle_layer(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        map: &MapReader,
        cells_x: i32,
        cells_y: i32,
    ) -> GameResult {
        for y in (self.map_point_y - 1)..(self.map_point_y + cells_y + 35) {
            if y < 0 || y >= map.height as i32 {
                continue;
            }
            
            for x in (self.map_point_x - 1)..(self.map_point_x + cells_x + 35) {
                if x < 0 || x >= map.width as i32 {
                    continue;
                }

                if x >= 0 && y >= 0 && (x as usize) < map.width && (y as usize) < map.height {
                    let cell = &map.map_cells[x as usize][y as usize];
                    let middle_index = cell.middle_image - 1;
                    if middle_index >= 0 {
                        self.draw_cell(ctx, canvas, x, y, Color::from_rgb(200, 255, 200))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_front_layer(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        map: &MapReader,
        cells_x: i32,
        cells_y: i32,
    ) -> GameResult {
        for y in (self.map_point_y - 1)..(self.map_point_y + cells_y + 35) {
            if y < 0 || y >= map.height as i32 {
                continue;
            }
            
            for x in (self.map_point_x)..(self.map_point_x + cells_x + 35) {
                if x < 0 || x >= map.width as i32 {
                    continue;
                }

                if x >= 0 && y >= 0 && (x as usize) < map.width && (y as usize) < map.height {
                    let cell = &map.map_cells[x as usize][y as usize];
                    let front_index = (cell.front_image & 0x7FFF) - 1;
                    if front_index >= 0 {
                        self.draw_cell(ctx, canvas, x, y, Color::from_rgb(255, 200, 200))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_grid(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        cells_x: i32,
        cells_y: i32,
    ) -> GameResult {
        for y in self.map_point_y..(self.map_point_y + cells_y + 2) {
            for x in self.map_point_x..(self.map_point_x + cells_x + 2) {
                self.draw_cell_border(ctx, canvas, x, y)?;
            }
        }
        Ok(())
    }

    fn draw_front_tags(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        map: &MapReader,
        cells_x: i32,
        cells_y: i32,
    ) -> GameResult {
        for y in (self.map_point_y - 1)..(self.map_point_y + cells_y + 35) {
            if y < 0 || y >= map.height as i32 {
                continue;
            }
            
            for x in self.map_point_x..(self.map_point_x + cells_x + 35) {
                if x < 0 || x >= map.width as i32 {
                    continue;
                }

                if x >= 0 && y >= 0 && (x as usize) < map.width && (y as usize) < map.height {
                    let cell = &map.map_cells[x as usize][y as usize];
                    let front_index = (cell.front_image & 0x7FFF) - 1;
                    if front_index >= 0 {
                        self.draw_cell(ctx, canvas, x, y, Color::from_rgba(255, 255, 0, 128))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_cell(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        grid_x: i32,
        grid_y: i32,
        color: Color,
    ) -> GameResult {
        let x = (grid_x - self.map_point_x) as f32 * CELL_WIDTH * self.zoom;
        let y = (grid_y - self.map_point_y) as f32 * CELL_HEIGHT * self.zoom;

        let rect = Rect::new(
            x,
            y,
            CELL_WIDTH * self.zoom,
            CELL_HEIGHT * self.zoom,
        );

        let mesh = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(), rect, color)?;
        canvas.draw(&mesh, DrawParam::default());
        Ok(())
    }

    fn draw_cell_border(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        grid_x: i32,
        grid_y: i32,
    ) -> GameResult {
        let x = (grid_x - self.map_point_x) as f32 * CELL_WIDTH * self.zoom;
        let y = (grid_y - self.map_point_y) as f32 * CELL_HEIGHT * self.zoom;

        let rect = Rect::new(
            x,
            y,
            CELL_WIDTH * self.zoom,
            CELL_HEIGHT * self.zoom,
        );

        let mesh = Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::stroke(1.0),
            rect,
            Color::from_rgb(255, 0, 255),
        )?;
        canvas.draw(&mesh, DrawParam::default());
        Ok(())
    }

    pub fn screen_to_cell(&self, screen_x: f32, screen_y: f32) -> (i32, i32) {
        let cell_x = (screen_x / (CELL_WIDTH * self.zoom)) as i32 + self.map_point_x;
        let cell_y = (screen_y / (CELL_HEIGHT * self.zoom)) as i32 + self.map_point_y;
        (cell_x, cell_y)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
