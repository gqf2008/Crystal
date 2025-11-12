// 地图渲染器 - Macroquad 版本

use crate::backends::macroquad::graphics::{get_map_library, LibraryArray};
use crate::objects::MapReader;
use macroquad::prelude::*;
use std::time::Instant;

pub struct MapRenderer {
    pub tile_width: f32,
    pub tile_height: f32,
    
    /// 性能统计：draw_texture 总耗时(微秒)
    pub draw_texture_total_us: u128,
    /// 性能统计：draw_texture 调用次数
    pub draw_texture_calls: u32,
    
    /// 分层性能统计
    pub back_layer_time_us: u128,
    pub middle_layer_time_us: u128,
    pub front_layer_time_us: u128,
    pub back_layer_tiles: u32,
    pub middle_layer_tiles: u32,
    pub front_layer_tiles: u32,
}

impl MapRenderer {
    pub fn new(tile_width: f32, tile_height: f32) -> Self {
        Self {
            tile_width,
            tile_height,
            draw_texture_total_us: 0,
            draw_texture_calls: 0,
            back_layer_time_us: 0,
            middle_layer_time_us: 0,
            front_layer_time_us: 0,
            back_layer_tiles: 0,
            middle_layer_tiles: 0,
            front_layer_tiles: 0,
        }
    }

    pub fn update(&mut self, _dt: f32) {
        // 预留给动画更新
    }
    
    /// 重置性能统计
    pub fn reset_perf_stats(&mut self) {
        self.draw_texture_total_us = 0;
        self.draw_texture_calls = 0;
        self.back_layer_time_us = 0;
        self.middle_layer_time_us = 0;
        self.front_layer_time_us = 0;
        self.back_layer_tiles = 0;
        self.middle_layer_tiles = 0;
        self.front_layer_tiles = 0;
    }
    
    /// 获取平均每次 draw_texture 耗时(微秒)
    pub fn get_avg_draw_texture_us(&self) -> f64 {
        if self.draw_texture_calls == 0 {
            0.0
        } else {
            self.draw_texture_total_us as f64 / self.draw_texture_calls as f64
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        map_reader: &MapReader,
        camera_x: f32,
        camera_y: f32,
        camera_width: f32,
        camera_height: f32,
        zoom: f32,
        show_back_layer: bool,
        show_middle_layer: bool,
        show_front_layer: bool,
        show_texture_border: bool,
    ) -> u32 {
        let mut tiles_drawn = 0;

        let view_width = camera_width / zoom;
        let view_height = camera_height / zoom;
        
        let half_view_width = view_width / 2.0;
        let half_view_height = view_height / 2.0;
        
        let viewport_left = camera_x - half_view_width;
        let viewport_top = camera_y - half_view_height;
        let viewport_right = camera_x + half_view_width;
        let viewport_bottom = camera_y + half_view_height;
        
        let start_x = (viewport_left / self.tile_width).floor() as i32;
        let start_y = (viewport_top / self.tile_height).floor() as i32;
        let end_x = (viewport_right / self.tile_width).ceil() as i32 + 1;
        let end_y = (viewport_bottom / self.tile_height).ceil() as i32 + 1;

        let map_width = map_reader.width;
        let map_height = map_reader.height;

        // 测量 Back 层渲染耗时
        let back_layer_start = Instant::now();
        if show_back_layer {
            let back_start_x = if start_x % 2 == 0 { start_x } else { start_x - 1 };
            let back_start_y = if start_y % 2 == 0 { start_y } else { start_y - 1 };
            
            for y in (back_start_y..end_y).step_by(2) {
                if y < 0 || y >= map_height {
                    continue;
                }
                for x in (back_start_x..end_x).step_by(2) {
                    if x < 0 || x >= map_width {
                        continue;
                    }
                    
                    let Some(cell) = map_reader.get_cell(x, y) else {
                        continue;
                    };

                    if let Some((file_index, image_index)) = cell.back_tile() {
                        // 注意：back_tile() 返回的 image_index 已经是正确的值（已经做了 -1）
                        
                        // 获取纹理（需要持有锁直到纹理使用完毕）
                        let texture_opt = get_map_library(file_index)
                            .and_then(|lib| {
                                let mut lib_guard = lib.borrow_mut();
                                lib_guard.get_or_create_texture(image_index as usize).ok()
                                    .and_then(|info| info.image.as_ref().cloned())
                            });
                        
                        if let Some(texture) = texture_opt {
                            let world_x = x as f32 * self.tile_width;
                            let world_y = y as f32 * self.tile_height;
                            
                            // 测量 draw_texture 耗时
                            let start = Instant::now();
                            draw_texture_ex(
                                &texture,
                                world_x,
                                world_y,
                                WHITE,
                                DrawTextureParams {
                                    // 启用线性过滤,让阴影和纹理更细腻
                                    ..Default::default()
                                },
                            );
                            texture.set_filter(FilterMode::Linear);
                            self.draw_texture_total_us += start.elapsed().as_micros();
                            self.draw_texture_calls += 1;
                            
                            if show_texture_border {
                                draw_rectangle_lines(
                                    world_x,
                                    world_y,
                                    texture.width(),
                                    texture.height(),
                                    2.0,
                                    RED,
                                );
                            }
                            
                            tiles_drawn += 1;
                            self.back_layer_tiles += 1;
                        }
                    }
                }
            }
        }
        self.back_layer_time_us += back_layer_start.elapsed().as_micros();

        // 测量 Middle 层渲染耗时
        let middle_layer_start = Instant::now();
        if show_middle_layer {
            for y in start_y..end_y {
                if y < 0 || y >= map_height {
                    continue;
                }
                for x in start_x..end_x {
                    if x < 0 || x >= map_width {
                        continue;
                    }

                    let Some(cell) = map_reader.get_cell(x, y) else {
                        continue;
                    };

                    if let Some((file_index, image_index)) = cell.middle_tile() {
                        // 注意：middle_tile() 返回的 image_index 已经是正确的值（已经做了 -1）
                        
                        let texture_opt = get_map_library(file_index)
                            .and_then(|lib| {
                                let mut lib_guard = lib.borrow_mut();
                                lib_guard.get_or_create_texture(image_index as usize).ok()
                                    .and_then(|info| info.image.as_ref().cloned())
                            });
                        
                        if let Some(texture) = texture_opt {
                            let world_x = x as f32 * self.tile_width;
                            let world_y = y as f32 * self.tile_height;
                            let offset_y = world_y + self.tile_height - texture.height();

                            // 测量 draw_texture 耗时
                            let start = Instant::now();
                            draw_texture_ex(
                                &texture,
                                world_x,
                                offset_y,
                                WHITE,
                                DrawTextureParams::default(),
                            );
                            self.draw_texture_total_us += start.elapsed().as_micros();
                            self.draw_texture_calls += 1;
                            
                            if show_texture_border {
                                draw_rectangle_lines(
                                    world_x,
                                    offset_y,
                                    texture.width(),
                                    texture.height(),
                                    2.0,
                                    GREEN,
                                );
                            }
                            
                            tiles_drawn += 1;
                            self.middle_layer_tiles += 1;
                        }
                    }
                }
            }
        }
        self.middle_layer_time_us += middle_layer_start.elapsed().as_micros();

        // 测量 Front 层渲染耗时
        let front_layer_start = Instant::now();
        if show_front_layer {
            for y in start_y..end_y {
                if y < 0 || y >= map_height {
                    continue;
                }
                for x in start_x..end_x {
                    if x < 0 || x >= map_width {
                        continue;
                    }

                    let Some(cell) = map_reader.get_cell(x, y) else {
                        continue;
                    };

                    if let Some((file_index, image_index)) = cell.front_tile() {
                        // 注意：front_tile() 返回的 image_index 已经是正确的值（已经做了 -1）
                        
                        let texture_opt = get_map_library(file_index)
                            .and_then(|lib| {
                                let mut lib_guard = lib.borrow_mut();
                                lib_guard.get_or_create_texture(image_index as usize).ok()
                                    .and_then(|info| info.image.as_ref().cloned())
                            });
                        
                        if let Some(texture) = texture_opt {
                            let world_x = x as f32 * self.tile_width;
                            let world_y = y as f32 * self.tile_height;
                            let offset_y = world_y + self.tile_height - texture.height();
                            
                            // 测量 draw_texture 耗时
                            let start = Instant::now();
                            draw_texture_ex(
                                &texture,
                                world_x,
                                world_y,
                                WHITE,
                                DrawTextureParams {
                                    // 启用线性过滤,让阴影和纹理更细腻
                                    ..Default::default()
                                },
                            );
                            texture.set_filter(FilterMode::Linear);
                            self.draw_texture_total_us += start.elapsed().as_micros();
                            self.draw_texture_calls += 1;
                            
                            if show_texture_border {
                                draw_rectangle_lines(
                                    world_x,
                                    offset_y,
                                    texture.width(),
                                    texture.height(),
                                    2.0,
                                    BLUE,
                                );
                            }
                            
                            tiles_drawn += 1;
                            self.front_layer_tiles += 1;
                        } 
                    }
                }
            }
        }
        self.front_layer_time_us += front_layer_start.elapsed().as_micros();

        tiles_drawn
    }
}

impl Default for MapRenderer {
    fn default() -> Self {
        Self::new(48.0, 32.0)
    }
}
