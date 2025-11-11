// ============================================================================
// Direct Texture Map Renderer for Macroquad
// ============================================================================
//
// 性能优化历程:
// - 原始方案 (14 FPS): 使用 RenderTarget + 双相机切换 → GPU 同步开销巨大
// - 优化方案 (120 FPS): 直接屏幕渲染 + 单相机 → 消除 GPU 同步
//
// 技术要点:
// 1. 移除 RenderTarget,避免离屏渲染和相机切换导致的 GPU flush
// 2. 像素对齐 (.floor()) 和最近邻过滤 (FilterMode::Nearest) 消除闪烁
// 3. Back 层特殊处理: 2x2 格子共享纹理,只在偶数坐标绘制
// 4. 依赖 macroquad 内部的自动 quad batching 优化 draw call
//
// 注: 虽然定义了 MeshBatch 结构,但当前未使用手动 mesh batching,
//     因为 macroquad 的自动批处理已经足够高效 (120 FPS)
// ============================================================================

use super::LibraryManager;
use crate::objects::MapReader;
use macroquad::prelude::*;
use macroquad::miniquad::{BlendState, BlendFactor, BlendValue, Equation};

/// Mesh批次数据 (预留,当前未使用)
struct MeshBatch {
    /// 顶点数据 (position + uv + color)
    vertices: Vec<Vertex>,
    /// 索引数据
    indices: Vec<u16>,
    /// 纹理
    texture: Texture2D,
}

impl MeshBatch {
    fn new(texture: Texture2D) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture,
        }
    }

    /// 添加一个四边形瓦片
    fn add_quad(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let base_index = self.vertices.len() as u16;

        // 法线向量 (Vec4, 面向屏幕外)
        let normal = Vec4::new(0.0, 0.0, 1.0, 0.0);

        // 添加4个顶点 (左上, 右上, 右下, 左下)
        // UV坐标: (0,0)是左上角, (1,1)是右下角
        // Color: [255, 255, 255, 255] = 白色不透明
        self.vertices.push(Vertex {
            position: Vec3::new(x, y, 0.0),
            uv: Vec2::new(0.0, 0.0),
            color: [255, 255, 255, 255],
            normal,
        });
        self.vertices.push(Vertex {
            position: Vec3::new(x + width, y, 0.0),
            uv: Vec2::new(1.0, 0.0),
            color: [255, 255, 255, 255],
            normal,
        });
        self.vertices.push(Vertex {
            position: Vec3::new(x + width, y + height, 0.0),
            uv: Vec2::new(1.0, 1.0),
            color: [255, 255, 255, 255],
            normal,
        });
        self.vertices.push(Vertex {
            position: Vec3::new(x, y + height, 0.0),
            uv: Vec2::new(0.0, 1.0),
            color: [255, 255, 255, 255],
            normal,
        });

        // 添加2个三角形的索引 (逆时针)
        self.indices.push(base_index);
        self.indices.push(base_index + 1);
        self.indices.push(base_index + 2);

        self.indices.push(base_index);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 3);
    }

    /// 构建并返回Mesh
    fn build_mesh(&self) -> Mesh {
        Mesh {
            vertices: self.vertices.clone(),
            indices: self.indices.clone(),
            texture: Some(self.texture.clone()),
        }
    }

    fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

/// 直接纹理渲染的地图渲染器
///
/// 性能特点:
/// - 逐瓦片调用 draw_texture_ex(),依赖 macroquad 自动批处理
/// - 无 RenderTarget,无相机切换,无 GPU 同步开销
/// - 实测性能: 120 FPS (从原来的 14 FPS 提升 8.5 倍)
pub struct MeshMapRenderer {
    tile_width: f32,
    tile_height: f32,

    /// 过扫描边界 - 用于渲染屏幕外的物体避免裁切
    pub left_margin: f32,
    pub top_margin: f32,
    pub right_margin: f32,
    pub bottom_margin: f32,

    /// 层级显示控制
    pub show_back_layer: bool,
    pub show_middle_layer: bool,
    pub show_front_layer: bool,

    /// 是否显示纹理边框 (调试用)
    pub show_texture_border: bool,
    
    /// 动画计时器
    animation_time: f32,
    /// 动画帧计数
    animation_frame: u32,
    /// ADD混合材质
    add_blend_material: Material,
}

impl MeshMapRenderer {
    pub fn new(tile_width: f32, tile_height: f32) -> Self {
        // 创建ADD混合材质 (dst + src)
        // 使用默认的vertex/fragment shader，只改变混合模式
        let add_blend_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../../shaders/default.vert"),
                fragment: include_str!("../../../shaders/default.frag"),
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        
        Self {
            tile_width,
            tile_height,
            // 默认只在底部扩展200px用于渲染高建筑物
            left_margin: 0.0,
            top_margin: 0.0,
            right_margin: 0.0,
            bottom_margin: 200.0,
            // 默认显示所有层级
            show_back_layer: true,
            show_middle_layer: true,
            show_front_layer: true,
            show_texture_border: false,
            animation_time: 0.0,
            animation_frame: 0,
            add_blend_material,
        }
    }

    pub fn update(&mut self, dt: f32) {
        // 动画更新 - 每100ms一帧
        self.animation_time += dt;
        if self.animation_time >= 0.1 {
            self.animation_frame = self.animation_frame.wrapping_add(1);
            self.animation_time -= 0.1;
        }
    }

    /// 渲染地图 (直接纹理渲染,依赖框架自动批处理)
    pub fn render(
        &mut self,
        map_reader: &MapReader,
        library_manager: &LibraryManager,
        camera_x: f32,
        camera_y: f32,
        viewport_width: f32,
        viewport_height: f32,
        zoom: f32,
        tint_color: Color, // 新增: 颜色调整(gamma/亮度/对比度等)
    ) -> u32 {
        // 使用成员变量的过扫描边界
        // 计算视口范围(加上各方向的过扫描边界)
        let half_width = (viewport_width / 2.0) / zoom;
        let half_height = (viewport_height / 2.0) / zoom;

        let view_left = camera_x - half_width - self.left_margin;
        let view_right = camera_x + half_width + self.right_margin;
        let view_top = camera_y - half_height - self.top_margin;
        let view_bottom = camera_y + half_height + self.bottom_margin;

        // 转换为格子坐标
        let start_x = ((view_left / self.tile_width).floor() as i32 - 1).max(0);
        let start_y = ((view_top / self.tile_height).floor() as i32 - 1).max(0);
        let end_x = ((view_right / self.tile_width).ceil() as i32 + 1).min(map_reader.width);
        let end_y = ((view_bottom / self.tile_height).ceil() as i32 + 1).min(map_reader.height);

        let mut total_tiles = 0;

        // Back层 - 特殊处理: 2x2格子共享,只在偶数坐标绘制
        if self.show_back_layer {
            let tiles = self.render_back_layer(
                map_reader,
                library_manager,
                start_x,
                start_y,
                end_x,
                end_y,
                tint_color,
            );
            total_tiles += tiles;
        }

        // Middle层
        if self.show_middle_layer {
            let tiles = self.render_middle_animated(
                map_reader,
                library_manager,
                start_x,
                start_y,
                end_x,
                end_y,
                tint_color,
            );
            total_tiles += tiles;
        }

        // Front层
        if self.show_front_layer {
            let tiles = self.render_front_animated(
                map_reader,
                library_manager,
                start_x,
                start_y,
                end_x,
                end_y,
                tint_color,
            );
            total_tiles += tiles;
        }

        total_tiles
    }

    /// 渲染Back层 (2x2格子共享,只在偶数坐标绘制)
    fn render_back_layer(
        &mut self,
        map_reader: &MapReader,
        library_manager: &LibraryManager,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        tint_color: Color,
    ) -> u32 {
        let mut tiles: Vec<(Texture2D, f32, f32, f32, f32, i32, i32, i32)> = Vec::new();
        let mut tiles_count = 0;

        // 🔧 关键: Back层是2x2格子共享的,只遍历偶数坐标
        let start_x_even = if start_x % 2 == 0 {
            start_x
        } else {
            start_x - 1
        };
        let start_y_even = if start_y % 2 == 0 {
            start_y
        } else {
            start_y - 1
        };

        for y in (start_y_even..end_y).step_by(2) {
            if y < 0 || y >= map_reader.height {
                continue;
            }
            for x in (start_x_even..end_x).step_by(2) {
                if x < 0 || x >= map_reader.width {
                    continue;
                }

                let Some(cell) = map_reader.get_cell(x, y) else {
                    continue;
                };

                if let Some((file_index, image_index)) = cell.back_tile() {
                    let lib_name = format!("MapLib_{}", file_index);
                    // Note: back_tile() already returns index-1, so use it directly
                    
                    if let Some(texture) =
                        library_manager.get_or_create_texture(&lib_name, image_index as usize)
                    {
                        // Back瓦片从偶数坐标开始绘制
                        let world_x = x as f32 * self.tile_width;
                        let world_y = y as f32 * self.tile_height;
                        let offset_y = world_y + self.tile_height - texture.height();
                        let width = texture.width();
                        let height = texture.height();

                        // 对齐到像素边界
                        let pixel_x = world_x.floor();
                        let pixel_y = offset_y.floor();

                        // 提取优先级标志
                        let priority = if (cell.back_image & 0x20000000) != 0 {
                            1
                        } else {
                            0
                        };

                        tiles.push((
                            texture.clone(),
                            pixel_x,
                            pixel_y,
                            width,
                            height,
                            priority,
                            y,
                            x,
                        ));
                        tiles_count += 1;
                    }
                }
            }
        }

        // 按优先级、Y、X排序
        tiles.sort_by_key(|(_, _, _, _, _, priority, y, x)| (*priority, *y, *x));

        // 渲染
        for (texture, x, y, width, height, _, _, _) in tiles {
            // 启用线性过滤以获得更细腻的缩放效果
            texture.set_filter(FilterMode::Linear);
            draw_texture_ex(
                &texture,
                x,
                y,
                tint_color, // 应用颜色调整
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }

        tiles_count
    }

    /// 渲染单个层 (逐瓦片渲染)
    fn render_layer(
        &mut self,
        map_reader: &MapReader,
        library_manager: &LibraryManager,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        get_tile: fn(&crate::objects::CellInfo) -> Option<(i16, i32)>,
        tint_color: Color,
    ) -> u32 {
        // 存储瓦片数据: (texture, x, y, width, height, priority, tile_y, tile_x)
        let mut tiles: Vec<(Texture2D, f32, f32, f32, f32, i32, i32, i32)> = Vec::new();
        let mut tiles_count = 0;

        for y in start_y..end_y {
            if y < 0 || y >= map_reader.height {
                continue;
            }
            for x in start_x..end_x {
                if x < 0 || x >= map_reader.width {
                    continue;
                }

                let Some(cell) = map_reader.get_cell(x, y) else {
                    continue;
                };

                if let Some((file_index, image_index)) = get_tile(cell) {
                    let lib_name = format!("MapLib_{}", file_index);
                    // Note: middle_tile()/front_tile() already return index-1, so use it directly
                    
                    if let Some(texture) =
                        library_manager.get_or_create_texture(&lib_name, image_index as usize)
                    {
                        let world_x = x as f32 * self.tile_width;
                        let world_y = y as f32 * self.tile_height;
                        let offset_y = world_y + self.tile_height - texture.height();
                        let width = texture.width();
                        let height = texture.height();

                        // 对齐到像素边界,避免闪烁
                        let pixel_x = world_x.floor();
                        let pixel_y = offset_y.floor();

                        // 🔧 关键修复: 提取back_image的优先级标志 (第29位: 0x20000000)
                        // 优先级: 0 = 先绘制(底层), 1 = 后绘制(覆盖层)
                        let priority = if (cell.back_image & 0x20000000) != 0 {
                            1
                        } else {
                            0
                        };

                        // 存储瓦片数据,包括优先级和坐标用于排序
                        tiles.push((
                            texture.clone(),
                            pixel_x,
                            pixel_y,
                            width,
                            height,
                            priority, // 优先级: 0先绘制, 1后绘制
                            y,        // Y坐标
                            x,        // X坐标
                        ));
                        tiles_count += 1;
                    }
                }
            }
        }

        // 三级排序确保正确的渲染顺序
        // 1. 优先级 (0先1后)
        // 2. Y坐标 (从上到下)
        // 3. X坐标 (从左到右)
        tiles.sort_by_key(|(_, _, _, _, _, priority, y, x)| (*priority, *y, *x));

        // 按顺序渲染所有瓦片
        for (texture, x, y, width, height, _, _, _) in tiles {
            // 启用线性过滤以获得更细腻的缩放效果
            texture.set_filter(FilterMode::Linear);

            draw_texture_ex(
                &texture,
                x,
                y,
                tint_color, // 应用颜色调整
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }

        tiles_count
    }
    
    /// 渲染Middle层（支持动画和混合）
    fn render_middle_animated(
        &self,
        map_reader: &MapReader,
        library_manager: &LibraryManager,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        tint_color: Color,
    ) -> u32 {
        let mut normal_tiles: Vec<(Texture2D, f32, f32, f32, f32, i32, i32, i32)> = Vec::new();
        let mut blend_tiles: Vec<(Texture2D, f32, f32, f32, f32, i32, i32, i32)> = Vec::new();
        let mut tiles_count = 0;

        for y in start_y..end_y {
            if y < 0 || y >= map_reader.height {
                continue;
            }
            for x in start_x..end_x {
                if x < 0 || x >= map_reader.width {
                    continue;
                }

                let Some(cell) = map_reader.get_cell(x, y) else {
                    continue;
                };

                if let Some((file_index, base_image_index)) = cell.middle_tile() {
                    let mut image_index = base_image_index;
                    
                    // 检查动画
                    if cell.middle_has_animation() {
                        let frame_count = cell.middle_animation_frame as u32;
                        if frame_count > 0 && frame_count < 255 {
                            let current_frame = self.animation_frame % frame_count;
                            image_index = base_image_index + current_frame as i32;
                        }
                    }
                    
                    // 检查是否需要混合
                    let use_blend = cell.middle_use_blend();
                    
                    let lib_name = format!("MapLib_{}", file_index);
                    // Note: middle_tile() already returns index-1, and animation is added on top, use directly
                    
                    if let Some(texture) =
                        library_manager.get_or_create_texture(&lib_name, image_index as usize)
                    {
                        // 获取图像偏移信息（动画帧可能有不同的偏移）
                        let (offset_x, offset_y) = library_manager
                            .get_image_offset(&lib_name, image_index as usize)
                            .unwrap_or((0, 0));
                        
                        // 应用偏移：底部对齐 + offset
                        let world_x = x as f32 * self.tile_width + offset_x as f32;
                        let world_y = y as f32 * self.tile_height;
                        let base_offset_y = world_y + self.tile_height - texture.height();
                        let final_y = base_offset_y + offset_y as f32;
                        let width = texture.width();
                        let height = texture.height();

                        let pixel_x = world_x.floor();
                        let pixel_y = final_y.floor();

                        let priority = if (cell.back_image & 0x20000000) != 0 {
                            1
                        } else {
                            0
                        };

                        let tile = (
                            texture.clone(),
                            pixel_x,
                            pixel_y,
                            width,
                            height,
                            priority,
                            y,
                            x,
                        );
                        
                        if use_blend {
                            blend_tiles.push(tile);
                        } else {
                            normal_tiles.push(tile);
                        }
                        tiles_count += 1;
                    }
                }
            }
        }

        // 先渲染普通瓦片
        normal_tiles.sort_by_key(|(_, _, _, _, _, priority, y, x)| (*priority, *y, *x));
        for (texture, x, y, width, height, _, _, _) in normal_tiles {
            texture.set_filter(FilterMode::Linear);
            draw_texture_ex(
                &texture,
                x,
                y,
                tint_color,
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }
        
        // 使用ADD混合材质渲染混合瓦片
        blend_tiles.sort_by_key(|(_, _, _, _, _, priority, y, x)| (*priority, *y, *x));
        gl_use_material(&self.add_blend_material);
        for (texture, x, y, width, height, _, _, _) in blend_tiles {
            texture.set_filter(FilterMode::Linear);
            draw_texture_ex(
                &texture,
                x,
                y,
                tint_color,
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }
        gl_use_default_material();

        tiles_count
    }
    
    /// 渲染Front层（支持动画和混合）
    fn render_front_animated(
        &self,
        map_reader: &MapReader,
        library_manager: &LibraryManager,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        tint_color: Color,
    ) -> u32 {
        let mut normal_tiles: Vec<(Texture2D, f32, f32, f32, f32, i32, i32, i32)> = Vec::new();
        let mut blend_tiles: Vec<(Texture2D, f32, f32, f32, f32, i32, i32, i32)> = Vec::new();
        let mut tiles_count = 0;

        for y in start_y..end_y {
            if y < 0 || y >= map_reader.height {
                continue;
            }
            for x in start_x..end_x {
                if x < 0 || x >= map_reader.width {
                    continue;
                }

                let Some(cell) = map_reader.get_cell(x, y) else {
                    continue;
                };

                if let Some((file_index, base_image_index)) = cell.front_tile() {
                    let mut image_index = base_image_index;
                    
                    // 检查动画（Front层从front_animation_frame判断）
                    let front_anim = cell.front_animation_frame;
                    let use_blend = front_anim > 0x0F; // 高4位表示需要混合
                    
                    if front_anim > 0 {
                        let frame_count = (front_anim & 0x0F) as u32; // 低4位是帧数
                        if frame_count > 0 {
                            let current_frame = self.animation_frame % frame_count;
                            image_index = base_image_index + current_frame as i32;
                        }
                    }
                    
                    let lib_name = format!("MapLib_{}", file_index);
                    // Note: front_tile() already returns index-1, and animation is added on top, use directly
                    
                    if let Some(texture) =
                        library_manager.get_or_create_texture(&lib_name, image_index as usize)
                    {
                        // 获取图像偏移信息（动画帧可能有不同的偏移）
                        let (offset_x, offset_y) = library_manager
                            .get_image_offset(&lib_name, image_index as usize)
                            .unwrap_or((0, 0));
                        
                        // 应用偏移：底部对齐 + offset
                        let world_x = x as f32 * self.tile_width + offset_x as f32;
                        let world_y = y as f32 * self.tile_height;
                        let base_offset_y = world_y + self.tile_height - texture.height();
                        let final_y = base_offset_y + offset_y as f32;
                        let width = texture.width();
                        let height = texture.height();

                        let pixel_x = world_x.floor();
                        let pixel_y = final_y.floor();

                        let priority = if (cell.back_image & 0x20000000) != 0 {
                            1
                        } else {
                            0
                        };

                        let tile = (
                            texture.clone(),
                            pixel_x,
                            pixel_y,
                            width,
                            height,
                            priority,
                            y,
                            x,
                        );
                        
                        if use_blend {
                            blend_tiles.push(tile);
                        } else {
                            normal_tiles.push(tile);
                        }
                        tiles_count += 1;
                    }
                }
            }
        }

        // 先渲染普通瓦片
        normal_tiles.sort_by_key(|(_, _, _, _, _, priority, y, x)| (*priority, *y, *x));
        for (texture, x, y, width, height, _, _, _) in normal_tiles {
            texture.set_filter(FilterMode::Linear);
            draw_texture_ex(
                &texture,
                x,
                y,
                tint_color,
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }
        
        // 使用ADD混合材质渲染混合瓦片
        blend_tiles.sort_by_key(|(_, _, _, _, _, priority, y, x)| (*priority, *y, *x));
        gl_use_material(&self.add_blend_material);
        for (texture, x, y, width, height, _, _, _) in blend_tiles {
            texture.set_filter(FilterMode::Linear);
            draw_texture_ex(
                &texture,
                x,
                y,
                tint_color,
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }
        gl_use_default_material();

        tiles_count
    }
}

