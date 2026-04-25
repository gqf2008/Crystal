// ============================================================================
// BigMapDialogHybrid - 大地图对话框
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/BigMapDialog.cs
// - 全屏显示，点击小地图按钮打开
// - 显示当前地图名称和尺寸
// - 支持拖拽平移和滚轮缩放
// - 显示玩家当前位置标记
//
// ============================================================================

use macroquad::prelude::*;
use crate::coord::{CELL_WIDTH, CELL_HEIGHT};
use crate::resources::map_reader::CellInfo;
use crate::resources::get_map_texture;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::{ButtonState, ButtonTextures};

pub struct BigMapDialogHybrid {
    visible: bool,
    position: Vec2,
    size: Vec2,

    // 地图数据
    map_name: String,
    world_size: Vec2,
    player_pos: Vec2,
    map_cells: Option<Vec<Vec<CellInfo>>>,
    map_width: i32,
    map_height: i32,

    // 拖拽状态
    is_dragging: bool,
    last_mouse_pos: Vec2,

    // 视图控制
    view_offset: Vec2,
    zoom_level: f32,

    // 纹理
    close_btn: ButtonTextures,
}

impl Default for BigMapDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl BigMapDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: Vec2::ZERO,
            size: Vec2::ZERO,
            map_name: String::new(),
            world_size: Vec2::ZERO,
            player_pos: Vec2::ZERO,
            map_cells: None,
            map_width: 0,
            map_height: 0,
            is_dragging: false,
            last_mouse_pos: Vec2::ZERO,
            view_offset: Vec2::ZERO,
            zoom_level: 1.0,
            close_btn: ButtonTextures::new(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.size = vec2(screen_width(), screen_height());
        self.position = Vec2::ZERO;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_map_info(&mut self, name: String, width: f32, height: f32) {
        self.map_name = name;
        self.world_size = vec2(width, height);
    }

    pub fn set_player_position(&mut self, x: f32, y: f32) {
        // player_pos 是世界像素坐标，转为格子坐标以便与瓦片渲染对齐
        self.player_pos = vec2(x / CELL_WIDTH as f32, y / CELL_HEIGHT as f32);
    }

    pub fn set_map_data(&mut self, cells: Vec<Vec<CellInfo>>, width: i32, height: i32) {
        self.map_cells = Some(cells);
        self.map_width = width;
        self.map_height = height;
        self.world_size = vec2(width as f32, height as f32);
    }

    pub fn map_width(&self) -> i32 { self.map_width }
    pub fn map_height(&self) -> i32 { self.map_height }

    /// 更新并绘制，返回是否消耗了输入
    pub fn update_and_draw(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        let (mx, my) = mouse_position();
        let mouse_pos = vec2(mx, my);
        let mut consumed = false;

        // 半透明背景
        draw_rectangle(0.0, 0.0, self.size.x, self.size.y, Color::from_rgba(0, 0, 0, 180));

        // 地图绘制区域（留边距）
        let margin = 40.0;
        let map_area = Rect::new(margin, margin, self.size.x - margin * 2.0, self.size.y - margin * 2.0 - 50.0);

        // 地图背景
        draw_rectangle(map_area.x, map_area.y, map_area.w, map_area.h, Color::from_rgba(20, 20, 30, 255));
        draw_rectangle_lines(map_area.x, map_area.y, map_area.w, map_area.h, 2.0, Color::from_rgba(80, 80, 120, 255));

        // 处理拖拽和缩放
        self.handle_input(map_area, mouse_pos);

        // 绘制地图瓦片
        self.draw_map_tiles(map_area);

        // 绘制玩家位置
        self.draw_player_marker(map_area);

        // 地图名称
        draw_text_cn(&self.map_name, map_area.x + 10.0, map_area.y - 20.0, 16.0, WHITE);

        // 坐标信息
        if self.world_size.x > 0.0 {
            let coord_text = format!("{:.0} x {:.0}", self.player_pos.x, self.player_pos.y);
            draw_text_cn(&coord_text, map_area.x + map_area.w - 100.0, map_area.y - 20.0, 14.0, Color::from_rgba(180, 180, 180, 255));
        }

        // 关闭按钮
        let close_size = vec2(32.0, 32.0);
        let close_pos = vec2(self.size.x - margin - close_size.x, margin - close_size.y - 5.0);
        let close_rect = Rect::new(close_pos.x, close_pos.y, close_size.x, close_size.y);
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn.draw(close_pos, close_state);

        if ButtonState::is_clicked(close_rect, mouse_pos) {
            self.hide();
            consumed = true;
        }

        consumed
    }

    fn handle_input(&mut self, map_area: Rect, mouse_pos: Vec2) {
        if !map_area.contains(mouse_pos) {
            self.is_dragging = false;
            return;
        }

        // 拖拽平移
        if is_mouse_button_down(MouseButton::Left) {
            if self.is_dragging {
                let delta = mouse_pos - self.last_mouse_pos;
                self.view_offset += delta;
            } else {
                self.is_dragging = true;
            }
            self.last_mouse_pos = mouse_pos;
        } else {
            self.is_dragging = false;
        }

        // 滚轮缩放
        let wheel = mouse_wheel();
        if wheel.1.abs() > 0.1 {
            let delta = if wheel.1 > 0.0 { 0.1 } else { -0.1 };
            self.zoom_level = (self.zoom_level + delta).clamp(0.5, 5.0);
        }
    }

    fn draw_map_tiles(&mut self, map_area: Rect) {
        let tile_w = CELL_WIDTH as f32 * self.zoom_level;
        let tile_h = CELL_HEIGHT as f32 * self.zoom_level;

        if let Some(ref cells) = self.map_cells {
            let start_x = ((-self.view_offset.x / tile_w).floor() as i32).max(0).min(self.map_width);
            let start_y = ((-self.view_offset.y / tile_h).floor() as i32).max(0).min(self.map_height);
            let end_x = ((-self.view_offset.x + map_area.w) / tile_w).ceil() as i32 + 1;
            let end_y = ((-self.view_offset.y + map_area.h) / tile_h).ceil() as i32 + 1;
            let end_x = end_x.min(self.map_width);
            let end_y = end_y.min(self.map_height);

            // 绘制 Back 层（2x2 格子共用纹理，只绘制偶数坐标）
            for y in (start_y..end_y).step_by(2) {
                for x in (start_x..end_x).step_by(2) {
                    let cell = &cells[x as usize][y as usize];
                    if let Some((file_idx, img_idx)) = cell.back_tile() {
                        self.draw_tile(file_idx, img_idx, x, y, map_area, tile_w, tile_h, true);
                    }
                }
            }

            // 绘制 Middle 层（所有格子）
            for y in start_y..end_y {
                for x in start_x..end_x {
                    let cell = &cells[x as usize][y as usize];
                    if let Some((file_idx, img_idx)) = cell.middle_tile() {
                        self.draw_tile(file_idx, img_idx, x, y, map_area, tile_w, tile_h, false);
                    }
                }
            }

            // 绘制 Front 层（所有格子）
            for y in start_y..end_y {
                for x in start_x..end_x {
                    let cell = &cells[x as usize][y as usize];
                    if let Some((file_idx, img_idx)) = cell.front_tile() {
                        self.draw_tile(file_idx, img_idx, x, y, map_area, tile_w, tile_h, false);
                    }
                }
            }
        } else if self.world_size.x <= 0.0 || self.world_size.y <= 0.0 {
            // 无地图数据时显示提示
            let text = "暂无地图数据";
            let dims = measure_text(text, None, 24, 1.0);
            let cx = map_area.x + map_area.w / 2.0 - dims.width / 2.0;
            let cy = map_area.y + map_area.h / 2.0;
            draw_text_cn(text, cx, cy, 24.0, Color::from_rgba(120, 120, 120, 255));
        }
    }

    fn draw_tile(
        &self,
        file_idx: i16,
        img_idx: i32,
        cell_x: i32,
        cell_y: i32,
        map_area: Rect,
        tile_w: f32,
        tile_h: f32,
        _is_back_layer: bool,
    ) {
        let screen_x = map_area.x + cell_x as f32 * tile_w + self.view_offset.x;
        let screen_y = map_area.y + cell_y as f32 * tile_h + self.view_offset.y;

        // 粗略的屏幕外剔除
        if screen_x + tile_w < 0.0 || screen_x > map_area.x + map_area.w
            || screen_y + tile_h < 0.0 || screen_y > map_area.y + map_area.h
        {
            return;
        }

        if let Some(info) = get_map_texture(file_idx, img_idx) {
            if let Some(tex) = info.image {
                draw_texture_ex(
                    &tex,
                    screen_x,
                    screen_y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(tex.width(), tex.height())),
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn draw_player_marker(&self, map_area: Rect) {
        if self.world_size.x <= 0.0 {
            return;
        }

        let step_x = map_area.w / self.world_size.x * self.zoom_level;
        let step_y = map_area.h / self.world_size.y * self.zoom_level;

        let px = map_area.x + self.player_pos.x * step_x + self.view_offset.x;
        let py = map_area.y + self.player_pos.y * step_y + self.view_offset.y;

        // 玩家标记（红色圆点）
        draw_circle(px, py, 5.0, RED);
        draw_circle_lines(px, py, 5.0, 2.0, WHITE);

        // 坐标标签
        let label = format!("({:.0},{:.0})", self.player_pos.x, self.player_pos.y);
        draw_text_cn(&label, px + 8.0, py - 5.0, 12.0, YELLOW);
    }
}
