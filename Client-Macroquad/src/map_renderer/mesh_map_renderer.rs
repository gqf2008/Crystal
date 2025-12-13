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
// ============================================================================

use crate::resources;
use crate::resources::MapReader;
use macroquad::prelude::*;
use macroquad::miniquad::{BlendState, BlendFactor, BlendValue, Equation, UniformDesc, UniformType};
use std::collections::HashMap;

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

    /// Front 遮挡半透明遮罩材质（按屏幕矩形区域按像素降低 alpha）
    focus_mask_material: Material,
    /// Front 遮挡半透明遮罩材质（ADD 混合版本）
    focus_mask_add_material: Material,

    // ------------------------------------------------------------------------
    // Chunk 缓存（静态层）
    // ------------------------------------------------------------------------
    /// 是否启用静态层 chunk 缓存（Back + 可缓存的 Middle）。
    pub enable_static_chunk_cache: bool,
    static_chunk_cache: StaticChunkCache,
    render_counter: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ChunkKey {
    cx: i32,
    cy: i32,
}

struct ChunkEntry {
    rt: RenderTarget,
    world_left: f32,
    world_top: f32,
    last_used: u64,
}

struct StaticChunkCache {
    chunk_tiles_x: i32,
    chunk_tiles_y: i32,
    /// 渲染 chunk 时额外采样的瓦片边界（用于覆盖跨 chunk 的高贴图，避免边缘被裁切）。
    margin_tiles_x: i32,
    margin_tiles_y: i32,
    max_chunks: usize,
    map_w: i32,
    map_h: i32,
    include_back: bool,
    include_middle: bool,
    hits_since_report: u64,
    misses_since_report: u64,
    evictions_since_report: u64,
    entries: HashMap<ChunkKey, ChunkEntry>,
}

impl StaticChunkCache {
    fn new() -> Self {
        Self {
            chunk_tiles_x: 32,
            chunk_tiles_y: 32,
            margin_tiles_x: 2,
            margin_tiles_y: 6,
            max_chunks: 128,
            map_w: 0,
            map_h: 0,
            include_back: true,
            include_middle: true,
            hits_since_report: 0,
            misses_since_report: 0,
            evictions_since_report: 0,
            entries: HashMap::new(),
        }
    }

    fn reset_stats(&mut self) {
        self.hits_since_report = 0;
        self.misses_since_report = 0;
        self.evictions_since_report = 0;
    }

    fn ensure_map(&mut self, map_reader: &MapReader) {
        if self.map_w != map_reader.width || self.map_h != map_reader.height {
            self.map_w = map_reader.width;
            self.map_h = map_reader.height;
            self.entries.clear();
            self.reset_stats();
        }
    }

    fn ensure_mode(&mut self, include_back: bool, include_middle: bool) {
        if self.include_back != include_back || self.include_middle != include_middle {
            self.include_back = include_back;
            self.include_middle = include_middle;
            self.entries.clear();
            self.reset_stats();
        }
    }

    fn maybe_report_stats(&mut self, render_counter: u64, visible_chunks: u64) {
        // 轻量统计：避免刷屏，默认每 120 帧输出一次。
        const REPORT_INTERVAL_FRAMES: u64 = 120;
        if render_counter % REPORT_INTERVAL_FRAMES != 0 {
            return;
        }

        let total = self.hits_since_report + self.misses_since_report;
        let hit_rate = if total > 0 {
            self.hits_since_report as f64 / total as f64
        } else {
            0.0
        };

        tracing::info!(
            "[chunk-cache] visible={} hits={} misses={} hit_rate={:.1}% evictions={} entries={}/{} mode=back:{} middle:{} chunk={}x{} margin={}x{}",
            visible_chunks,
            self.hits_since_report,
            self.misses_since_report,
            hit_rate * 100.0,
            self.evictions_since_report,
            self.entries.len(),
            self.max_chunks,
            self.include_back,
            self.include_middle,
            self.chunk_tiles_x,
            self.chunk_tiles_y,
            self.margin_tiles_x,
            self.margin_tiles_y,
        );

        self.reset_stats();
    }

    fn chunk_world_rect(&self, tile_w: f32, tile_h: f32, cx: i32, cy: i32) -> (f32, f32, f32, f32) {
        let start_x = cx * self.chunk_tiles_x;
        let start_y = cy * self.chunk_tiles_y;
        let world_left = start_x as f32 * tile_w;
        let world_top = start_y as f32 * tile_h;
        let world_w = self.chunk_tiles_x as f32 * tile_w;
        let world_h = self.chunk_tiles_y as f32 * tile_h;
        (world_left, world_top, world_w, world_h)
    }

    fn evict_if_needed(&mut self) {
        if self.entries.len() <= self.max_chunks {
            return;
        }

        // 简单 LRU：线性扫描找 last_used 最小。
        let mut oldest: Option<(ChunkKey, u64)> = None;
        for (k, v) in self.entries.iter() {
            let candidate = (*k, v.last_used);
            if oldest.is_none() || candidate.1 < oldest.unwrap().1 {
                oldest = Some(candidate);
            }
        }
        if let Some((k, _)) = oldest {
            self.entries.remove(&k);
            self.evictions_since_report = self.evictions_since_report.saturating_add(1);
        }
    }
}

impl MeshMapRenderer {
    pub fn new(tile_width: f32, tile_height: f32) -> Self {
        // 创建ADD混合材质 (dst + src)
        // 使用默认的vertex/fragment shader，只改变混合模式
        let add_blend_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../shaders/default.vert"),
                fragment: include_str!("../../shaders/default.frag"),
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

        // Front 遮挡半透明：使用自定义 frag，在 FocusRect 内按像素降低 alpha。
        let focus_mask_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../shaders/default.vert"),
                fragment: include_str!("../../shaders/focus_mask.frag"),
            },
            MaterialParams {
                // 关键：front 贴图大量使用透明像素。
                // 如果不启用 alpha blending，透明区域会以黑色显示（看起来像“黑色块”）。
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                uniforms: vec![
                    UniformDesc::new("FocusRect", UniformType::Float4),
                    UniformDesc::new("FocusAlpha", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .unwrap();

        let focus_mask_add_material = load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../shaders/default.vert"),
                fragment: include_str!("../../shaders/focus_mask.frag"),
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
                uniforms: vec![
                    UniformDesc::new("FocusRect", UniformType::Float4),
                    UniformDesc::new("FocusAlpha", UniformType::Float1),
                ],
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
            focus_mask_material,
            focus_mask_add_material,

            enable_static_chunk_cache: true,
            static_chunk_cache: StaticChunkCache::new(),
            render_counter: 0,
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
        camera_x: f32,
        camera_y: f32,
        viewport_width: f32,
        viewport_height: f32,
        zoom: f32,
        tint_color: Color, // 新增: 颜色调整(gamma/亮度/对比度等)
    ) -> u32 {
        self.render_counter = self.render_counter.wrapping_add(1);

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

        // 优先渲染静态 chunk（Back + Middle 可缓存部分）
        if self.enable_static_chunk_cache && (self.show_back_layer || self.show_middle_layer) {
            self.static_chunk_cache.ensure_map(map_reader);
            self.static_chunk_cache
                .ensure_mode(self.show_back_layer, self.show_middle_layer);

            // chunk 可见范围（按 tile 坐标）
            let cx0 = (start_x / self.static_chunk_cache.chunk_tiles_x).max(0);
            let cy0 = (start_y / self.static_chunk_cache.chunk_tiles_y).max(0);
            let cx1 = ((end_x - 1) / self.static_chunk_cache.chunk_tiles_x).max(0);
            let cy1 = ((end_y - 1) / self.static_chunk_cache.chunk_tiles_y).max(0);

            let visible_chunks = (cx1 - cx0 + 1).max(0) as u64 * (cy1 - cy0 + 1).max(0) as u64;
            let mut frame_hits: u64 = 0;
            let mut frame_misses: u64 = 0;

            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    let key = ChunkKey { cx, cy };
                    let used = self.render_counter;

                    // 命中缓存
                    if let Some(entry) = self.static_chunk_cache.entries.get_mut(&key) {
                        frame_hits = frame_hits.saturating_add(1);
                        entry.last_used = used;
                        draw_texture_ex(
                            &entry.rt.texture,
                            entry.world_left,
                            entry.world_top,
                            tint_color,
                            DrawTextureParams {
                                dest_size: Some(vec2(
                                    self.static_chunk_cache.chunk_tiles_x as f32 * self.tile_width,
                                    self.static_chunk_cache.chunk_tiles_y as f32 * self.tile_height,
                                )),
                                ..Default::default()
                            },
                        );
                        continue;
                    }

                    frame_misses = frame_misses.saturating_add(1);

                    // 缓存未命中：构建 chunk
                    let (world_left, world_top, world_w, world_h) = self
                        .static_chunk_cache
                        .chunk_world_rect(self.tile_width, self.tile_height, cx, cy);

                    let rt = render_target(world_w as u32, world_h as u32);
                    rt.texture.set_filter(FilterMode::Nearest);

                    // 切到 RenderTarget 相机
                    let mut rt_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, world_w, world_h));
                    // 当前项目的世界相机使用正的 zoom.y（不翻转 Y），这里保持一致，避免 chunk 纹理上下颠倒。
                    rt_cam.zoom.y = rt_cam.zoom.y.abs();
                    rt_cam.render_target = Some(rt.clone());
                    set_camera(&rt_cam);
                    clear_background(Color::new(0.0, 0.0, 0.0, 0.0));

                    self.render_static_chunk_contents(
                        map_reader,
                        cx,
                        cy,
                        world_left,
                        world_top,
                        tint_color,
                        self.show_back_layer,
                        self.show_middle_layer,
                    );

                    // 恢复世界相机（GameScene 的 map_camera 也使用同一套参数）
                    let mut world_cam = Camera2D::default();
                    world_cam.target = vec2(camera_x, camera_y);
                    world_cam.zoom = vec2(2.0 / viewport_width.max(1.0) * zoom, 2.0 / viewport_height.max(1.0) * zoom);
                    set_camera(&world_cam);

                    // 写入缓存并绘制
                    self.static_chunk_cache.entries.insert(
                        key,
                        ChunkEntry {
                            rt,
                            world_left,
                            world_top,
                            last_used: used,
                        },
                    );
                    self.static_chunk_cache.evict_if_needed();

                    let entry = self.static_chunk_cache.entries.get(&key).expect("inserted");
                    draw_texture_ex(
                        &entry.rt.texture,
                        entry.world_left,
                        entry.world_top,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(world_w, world_h)),
                            ..Default::default()
                        },
                    );
                }
            }

            self.static_chunk_cache.hits_since_report =
                self.static_chunk_cache.hits_since_report.saturating_add(frame_hits);
            self.static_chunk_cache.misses_since_report =
                self.static_chunk_cache.misses_since_report.saturating_add(frame_misses);
            self.static_chunk_cache
                .maybe_report_stats(self.render_counter, visible_chunks);
        }

        // Back层 - 特殊处理: 2x2格子共享,只在偶数坐标绘制
        if self.show_back_layer {
            let tiles = self.render_back_layer(
                map_reader,
                start_x,
                start_y,
                end_x,
                end_y,
                tint_color,
                if self.enable_static_chunk_cache { Some(1) } else { None },
            );
            total_tiles += tiles;
        }

        // Middle层
        if self.show_middle_layer {
            let tiles = self.render_middle_animated(
                map_reader,
                start_x,
                start_y,
                end_x,
                end_y,
                |cell| cell.middle_tile(),
                tint_color,
                self.enable_static_chunk_cache,
            );
            total_tiles += tiles;
        }

        // Front层
        if self.show_front_layer {
            let tiles = self.render_front_animated(
                map_reader,
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

    /// 构建静态 chunk 内容：Back + Middle（仅 priority=0、无动画、非 blend）
    fn render_static_chunk_contents(
        &mut self,
        map_reader: &MapReader,
        cx: i32,
        cy: i32,
        chunk_world_left: f32,
        chunk_world_top: f32,
        tint_color: Color,
        include_back: bool,
        include_middle: bool,
    ) {
        let sx = cx * self.static_chunk_cache.chunk_tiles_x;
        let sy = cy * self.static_chunk_cache.chunk_tiles_y;
        let ex = (sx + self.static_chunk_cache.chunk_tiles_x).min(map_reader.width);
        let ey = (sy + self.static_chunk_cache.chunk_tiles_y).min(map_reader.height);

        let mx = self.static_chunk_cache.margin_tiles_x;
        let my = self.static_chunk_cache.margin_tiles_y;

        let start_x = (sx - mx).max(0);
        let start_y = (sy - my).max(0);
        let end_x = (ex + mx).min(map_reader.width);
        let end_y = (ey + my).min(map_reader.height);

        if include_back {
            // Back（2x2 only even coords），仅 priority=0
            let start_x_even = if start_x % 2 == 0 { start_x } else { start_x - 1 };
            let start_y_even = if start_y % 2 == 0 { start_y } else { start_y - 1 };
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

                    // priority=0 only
                    if (cell.back_image & 0x20000000) != 0 {
                        continue;
                    }

                    let Some((file_index, image_index)) = cell.back_tile() else {
                        continue;
                    };
                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let offset_y = world_y + self.tile_height - texture.height();

                    let pixel_x = (world_x - chunk_world_left).floor();
                    let pixel_y = (offset_y - chunk_world_top).floor();
                    let width = texture.width();
                    let height = texture.height();

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        if include_middle {
            // Middle：仅 priority=0、无动画、非 blend
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

                    if (cell.back_image & 0x20000000) != 0 {
                        continue;
                    }
                    if cell.middle_use_blend() || cell.middle_has_animation() {
                        continue;
                    }

                    let Some((file_index, image_index)) = cell.middle_tile() else {
                        continue;
                    };
                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let base_y = world_y + self.tile_height - texture.height();

                    let pixel_x = (world_x - chunk_world_left).floor();
                    let pixel_y = (base_y - chunk_world_top).floor();
                    let width = texture.width();
                    let height = texture.height();

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    /// 只渲染 Front 层（用于“人物先画，前景后画”的遮挡逻辑）。
    ///
    /// 当 `focus_world` 存在时，会把靠近该点上方的前景瓦片按 `focus_alpha` 降低透明度，
    /// 用于实现“人物被树/屋檐遮挡时，前景半透明”。
    pub fn render_front_layer_with_focus(
        &mut self,
        map_reader: &MapReader,
        camera_x: f32,
        camera_y: f32,
        viewport_width: f32,
        viewport_height: f32,
        zoom: f32,
        tint_color: Color,
        focus_world: Option<Vec2>,
        focus_radius_tiles_x: i32,
        focus_radius_tiles_y: i32,
        focus_alpha: f32,
    ) -> u32 {
        let half_width = (viewport_width / 2.0) / zoom;
        let half_height = (viewport_height / 2.0) / zoom;

        let view_left = camera_x - half_width - self.left_margin;
        let view_right = camera_x + half_width + self.right_margin;
        let view_top = camera_y - half_height - self.top_margin;
        let view_bottom = camera_y + half_height + self.bottom_margin;

        let start_x = ((view_left / self.tile_width).floor() as i32 - 1).max(0);
        let start_y = ((view_top / self.tile_height).floor() as i32 - 1).max(0);
        let end_x = ((view_right / self.tile_width).ceil() as i32 + 1).min(map_reader.width);
        let end_y = ((view_bottom / self.tile_height).ceil() as i32 + 1).min(map_reader.height);

        // focus 遮罩参数（按屏幕矩形区域按像素降低 alpha）
        if let Some(focus) = focus_world {
            // focus 区域（世界坐标）：以玩家脚下为基准，向上覆盖一段，向下给 1 个 tile 缓冲
            let half_w_world = focus_radius_tiles_x.max(1) as f32 * self.tile_width;
            let up_h_world = focus_radius_tiles_y.max(1) as f32 * self.tile_height;

            let left_w = focus.x - half_w_world;
            let right_w = focus.x + half_w_world;
            let top_w = focus.y - up_h_world;
            let bottom_w = focus.y + self.tile_height;

            // world -> screen（以屏幕中心为原点，应用相机位置与 zoom）
            let world_to_screen = |wx: f32, wy: f32| -> (f32, f32) {
                (
                    viewport_width / 2.0 + (wx - camera_x) * zoom,
                    viewport_height / 2.0 + (wy - camera_y) * zoom,
                )
            };

            let (left_s, top_s) = world_to_screen(left_w, top_w);
            let (right_s, bottom_s) = world_to_screen(right_w, bottom_w);

            let x1 = left_s.min(right_s).clamp(0.0, viewport_width);
            let x2 = left_s.max(right_s).clamp(0.0, viewport_width);

            // gl_FragCoord：左下为原点；screen：左上为原点
            let gl_top = viewport_height - top_s;
            let gl_bottom = viewport_height - bottom_s;
            let y1 = gl_bottom.min(gl_top).clamp(0.0, viewport_height);
            let y2 = gl_bottom.max(gl_top).clamp(0.0, viewport_height);

            let rect = [x1, y1, x2, y2];
            self.focus_mask_material.set_uniform("FocusRect", rect);
            self.focus_mask_add_material.set_uniform("FocusRect", rect);
            self.focus_mask_material
                .set_uniform("FocusAlpha", focus_alpha.clamp(0.0, 1.0));
            self.focus_mask_add_material
                .set_uniform("FocusAlpha", focus_alpha.clamp(0.0, 1.0));
        }

        self.render_front_animated_with_focus(
            map_reader,
            start_x,
            start_y,
            end_x,
            end_y,
            tint_color,
            focus_world,
            focus_radius_tiles_x,
            focus_radius_tiles_y,
            focus_alpha,
        )
    }

    /// 更真实的遮挡检测：使用与 Front 渲染同样的贴图矩形计算逻辑，判断是否有任何 Front 贴图的
    /// 实际绘制矩形覆盖/相交 `probe_world`。
    ///
    /// 备注：这里只做几何矩形检测，不做逐像素 alpha 测试。
    pub fn front_layer_occludes_probe(
        &self,
        map_reader: &MapReader,
        probe_world: Rect,
        search_radius_tiles_x: i32,
        search_radius_tiles_y: i32,
    ) -> bool {
        let center_x = (probe_world.x + probe_world.w * 0.5) / self.tile_width;
        let center_y = (probe_world.y + probe_world.h * 0.5) / self.tile_height;
        let fx = center_x.floor() as i32;
        let fy = center_y.floor() as i32;

        let start_x = (fx - search_radius_tiles_x).max(0);
        let end_x = (fx + search_radius_tiles_x).min(map_reader.width - 1);
        let start_y = (fy - search_radius_tiles_y).max(0);
        let end_y = (fy + search_radius_tiles_y).min(map_reader.height - 1);

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let Some(cell) = map_reader.get_cell(x, y) else {
                    continue;
                };

                let Some((file_index, base_image_index)) = cell.front_tile() else {
                    continue;
                };

                // 计算动画帧（与 render_front_animated_with_focus 一致）
                let mut image_index = base_image_index;
                let mut animation = cell.front_animation_frame;
                let use_blend = if (animation & 0x80) > 0 {
                    animation &= 0x7F;
                    true
                } else {
                    false
                };

                if animation > 0 {
                    let tick_count = cell.front_animation_tick;
                    let adjusted_frame_count =
                        animation as u32 + (animation as u32 * tick_count as u32);
                    let current_frame =
                        (self.animation_frame % adjusted_frame_count) / (1 + tick_count as u32);
                    image_index = base_image_index + current_frame as i32;
                }

                let Some(info) = resources::get_map_texture(file_index, image_index) else {
                    continue;
                };
                let Some(texture) = info.image.clone() else {
                    continue;
                };

                let offset_x = info.offset_x;
                let offset_y = info.offset_y;

                let world_x = x as f32 * self.tile_width;
                let world_y = y as f32 * self.tile_height;
                let width = texture.width();
                let height = texture.height();
                let base_y = world_y + self.tile_height;

                let should_apply_offset = if use_blend {
                    file_index == 14
                        || file_index == 27
                        || (file_index > 99 && file_index < 199)
                        || (image_index >= 2723 && image_index <= 2732)
                } else if file_index == 28 {
                    offset_x != 0 || offset_y != 0
                } else {
                    false
                };

                let (pixel_x, pixel_y) = if use_blend {
                    if file_index == 14 || file_index == 27 || (file_index > 99 && file_index < 199)
                    {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - 3.0 * self.tile_height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    } else {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    }
                } else if file_index == 28 && (offset_x != 0 || offset_y != 0) {
                    (
                        (world_x + offset_x as f32).floor(),
                        (base_y - self.tile_height + offset_y as f32).floor(),
                    )
                } else {
                    let mut base_x = world_x;
                    let mut base_y_pos = base_y - height;
                    if should_apply_offset {
                        base_x += offset_x as f32;
                        base_y_pos += offset_y as f32;
                    }
                    (base_x.floor(), base_y_pos.floor())
                };

                let front_rect = Rect::new(pixel_x, pixel_y, width, height);
                if front_rect.overlaps(&probe_world) {
                    return true;
                }
            }
        }

        false
    }

    /// 渲染Back层 (2x2格子共享,只在偶数坐标绘制)
    fn render_back_layer(
        &mut self,
        map_reader: &MapReader,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        tint_color: Color,
        only_priority: Option<i32>,
    ) -> u32 {
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

        // 性能优化：避免每帧 push+sort 大量 Vec。
        // 直接按 (priority, y, x) 双层遍历，保证与 sort_unstable_by_key 一致的绘制顺序。
        let pri_start = only_priority.unwrap_or(0);
        let pri_end = only_priority.unwrap_or(1);
        for priority in pri_start..=pri_end {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    let Some((file_index, image_index)) = cell.back_tile() else {
                        continue;
                    };

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let offset_y = world_y + self.tile_height - texture.height();
                    let width = texture.width();
                    let height = texture.height();

                    let pixel_x = world_x.floor();
                    let pixel_y = offset_y.floor();

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }

        tiles_count
    }

    /// 渲染动画层 (Middle/Front)
    /// 
    /// 动画帧计算公式 (来自 C# 客户端):
    /// ```csharp
    /// index += (AnimationCount % (animation + (animation * animationTick))) / (1 + animationTick);
    /// ```
    fn render_middle_animated(
        &mut self,
        map_reader: &MapReader,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        get_tile: fn(&crate::resources::map_reader::CellInfo) -> Option<(i16, i32)>,
        tint_color: Color,
        skip_cached_static: bool,
    ) -> u32 {
        let mut tiles_count = 0;

        // 性能优化：避免每帧构建 Vec 并排序。
        // 保持原有“普通先画、混合后画”的语义，同时保证 (priority, y, x) 顺序一致。

        // 1) 普通瓦片（正常混合）
        for priority in 0..=1 {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    if skip_cached_static {
                        let priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                        if priority == 0 && !cell.middle_use_blend() && !cell.middle_has_animation() {
                            // 这部分已由静态 chunk 缓存绘制
                            continue;
                        }
                    }

                    if cell.middle_use_blend() {
                        continue;
                    }

                    let Some((file_index, mut image_index)) = get_tile(cell) else {
                        continue;
                    };

                    if cell.middle_has_animation() {
                        let frame_count = cell.middle_animation_frame;
                        let frame_interval = cell.middle_animation_tick;
                        let total_ticks =
                            frame_count as u32 + frame_count as u32 * frame_interval as u32;
                        let divisor = 1 + frame_interval as u32;
                        let frame_offset = ((self.animation_frame % total_ticks) / divisor) as i32;
                        image_index += frame_offset;
                    }

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let base_y = world_y + self.tile_height - texture.height();
                    let width = texture.width();
                    let height = texture.height();

                    let pixel_x = world_x.floor();
                    let pixel_y = base_y.floor();

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }

        // 2) 混合瓦片（ADD 混合）
        gl_use_material(&self.add_blend_material);
        for priority in 0..=1 {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    if !cell.middle_use_blend() {
                        continue;
                    }

                    let Some((file_index, mut image_index)) = get_tile(cell) else {
                        continue;
                    };

                    if cell.middle_has_animation() {
                        let frame_count = cell.middle_animation_frame;
                        let frame_interval = cell.middle_animation_tick;
                        let total_ticks =
                            frame_count as u32 + frame_count as u32 * frame_interval as u32;
                        let divisor = 1 + frame_interval as u32;
                        let frame_offset = ((self.animation_frame % total_ticks) / divisor) as i32;
                        image_index += frame_offset;
                    }

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let base_y = world_y + self.tile_height - texture.height();
                    let width = texture.width();
                    let height = texture.height();

                    let pixel_x = world_x.floor();
                    let pixel_y = base_y.floor();

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }
        gl_use_default_material();

        tiles_count
    }
    
    /// 渲染Front层（支持动画和混合）
    fn render_front_animated(
        &mut self,
        map_reader: &MapReader,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        tint_color: Color,
    ) -> u32 {
        let mut tiles_count = 0;

        // 性能优化：去掉 Vec 收集与排序，直接按 (priority, y, x) 顺序绘制。

        // 1) 普通瓦片
        for priority in 0..=1 {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    let Some((file_index, base_image_index)) = cell.front_tile() else {
                        continue;
                    };

                    let mut image_index = base_image_index;
                    let mut animation = cell.front_animation_frame;
                    let use_blend = if (animation & 0x80) > 0 {
                        animation &= 0x7F;
                        true
                    } else {
                        false
                    };
                    if use_blend {
                        continue;
                    }

                    if animation > 0 {
                        let tick_count = cell.front_animation_tick;
                        let adjusted_frame_count =
                            animation as u32 + (animation as u32 * tick_count as u32);
                        let current_frame =
                            (self.animation_frame % adjusted_frame_count) / (1 + tick_count as u32);
                        image_index = base_image_index + current_frame as i32;
                    }

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };
                    let offset_x = info.offset_x;
                    let offset_y = info.offset_y;

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let width = texture.width();
                    let height = texture.height();
                    let base_y = world_y + self.tile_height;

                    let should_apply_offset = if file_index == 28 {
                        offset_x != 0 || offset_y != 0
                    } else {
                        false
                    };

                    let (pixel_x, pixel_y) = if file_index == 28 && (offset_x != 0 || offset_y != 0) {
                        (
                            (world_x + offset_x as f32).floor(),
                            (base_y - self.tile_height + offset_y as f32).floor(),
                        )
                    } else {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    };

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }

        // 2) 混合瓦片（ADD）
        gl_use_material(&self.add_blend_material);
        for priority in 0..=1 {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    let Some((file_index, base_image_index)) = cell.front_tile() else {
                        continue;
                    };

                    let mut image_index = base_image_index;
                    let mut animation = cell.front_animation_frame;
                    let use_blend = if (animation & 0x80) > 0 {
                        animation &= 0x7F;
                        true
                    } else {
                        false
                    };
                    if !use_blend {
                        continue;
                    }

                    if animation > 0 {
                        let tick_count = cell.front_animation_tick;
                        let adjusted_frame_count =
                            animation as u32 + (animation as u32 * tick_count as u32);
                        let current_frame =
                            (self.animation_frame % adjusted_frame_count) / (1 + tick_count as u32);
                        image_index = base_image_index + current_frame as i32;
                    }

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };
                    let offset_x = info.offset_x;
                    let offset_y = info.offset_y;

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let width = texture.width();
                    let height = texture.height();
                    let base_y = world_y + self.tile_height;

                    let should_apply_offset = file_index == 14
                        || file_index == 27
                        || (file_index > 99 && file_index < 199)
                        || (image_index >= 2723 && image_index <= 2732);

                    let (pixel_x, pixel_y) = if file_index == 14
                        || file_index == 27
                        || (file_index > 99 && file_index < 199)
                    {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - 3.0 * self.tile_height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    } else {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    };

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }
        gl_use_default_material();

        tiles_count
    }

    fn render_front_animated_with_focus(
        &mut self,
        map_reader: &MapReader,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        tint_color: Color,
        focus_world: Option<Vec2>,
        _focus_radius_tiles_x: i32,
        _focus_radius_tiles_y: i32,
        _focus_alpha: f32,
    ) -> u32 {
        let mut tiles_count = 0;

        // 性能优化：保持 focus shader 语义不变，去掉 Vec+排序。

        // 1) 普通瓦片（可能需要 focus shader）
        if focus_world.is_some() {
            gl_use_material(&self.focus_mask_material);
        }
        for priority in 0..=1 {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    let Some((file_index, base_image_index)) = cell.front_tile() else {
                        continue;
                    };

                    let mut image_index = base_image_index;
                    let mut animation = cell.front_animation_frame;
                    let use_blend = if (animation & 0x80) > 0 {
                        animation &= 0x7F;
                        true
                    } else {
                        false
                    };
                    if use_blend {
                        continue;
                    }

                    if animation > 0 {
                        let tick_count = cell.front_animation_tick;
                        let adjusted_frame_count =
                            animation as u32 + (animation as u32 * tick_count as u32);
                        let current_frame =
                            (self.animation_frame % adjusted_frame_count) / (1 + tick_count as u32);
                        image_index = base_image_index + current_frame as i32;
                    }

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };
                    let offset_x = info.offset_x;
                    let offset_y = info.offset_y;

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let width = texture.width();
                    let height = texture.height();
                    let base_y = world_y + self.tile_height;

                    let should_apply_offset = if file_index == 28 {
                        offset_x != 0 || offset_y != 0
                    } else {
                        false
                    };

                    let (pixel_x, pixel_y) = if file_index == 28 && (offset_x != 0 || offset_y != 0) {
                        (
                            (world_x + offset_x as f32).floor(),
                            (base_y - self.tile_height + offset_y as f32).floor(),
                        )
                    } else {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    };

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }
        if focus_world.is_some() {
            gl_use_default_material();
        }

        // 2) 混合瓦片（focus 时用 focus_mask_add，否则用 add_blend）
        if focus_world.is_some() {
            gl_use_material(&self.focus_mask_add_material);
        } else {
            gl_use_material(&self.add_blend_material);
        }
        for priority in 0..=1 {
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

                    let cell_priority = if (cell.back_image & 0x20000000) != 0 { 1 } else { 0 };
                    if cell_priority != priority {
                        continue;
                    }

                    let Some((file_index, base_image_index)) = cell.front_tile() else {
                        continue;
                    };

                    let mut image_index = base_image_index;
                    let mut animation = cell.front_animation_frame;
                    let use_blend = if (animation & 0x80) > 0 {
                        animation &= 0x7F;
                        true
                    } else {
                        false
                    };
                    if !use_blend {
                        continue;
                    }

                    if animation > 0 {
                        let tick_count = cell.front_animation_tick;
                        let adjusted_frame_count =
                            animation as u32 + (animation as u32 * tick_count as u32);
                        let current_frame =
                            (self.animation_frame % adjusted_frame_count) / (1 + tick_count as u32);
                        image_index = base_image_index + current_frame as i32;
                    }

                    let Some(info) = resources::get_map_texture(file_index, image_index) else {
                        continue;
                    };
                    let Some(texture) = info.image.clone() else {
                        continue;
                    };
                    let offset_x = info.offset_x;
                    let offset_y = info.offset_y;

                    let world_x = x as f32 * self.tile_width;
                    let world_y = y as f32 * self.tile_height;
                    let width = texture.width();
                    let height = texture.height();
                    let base_y = world_y + self.tile_height;

                    let should_apply_offset = file_index == 14
                        || file_index == 27
                        || (file_index > 99 && file_index < 199)
                        || (image_index >= 2723 && image_index <= 2732);

                    let (pixel_x, pixel_y) = if file_index == 14
                        || file_index == 27
                        || (file_index > 99 && file_index < 199)
                    {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - 3.0 * self.tile_height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    } else {
                        let mut base_x = world_x;
                        let mut base_y_pos = base_y - height;
                        if should_apply_offset {
                            base_x += offset_x as f32;
                            base_y_pos += offset_y as f32;
                        }
                        (base_x.floor(), base_y_pos.floor())
                    };

                    draw_texture_ex(
                        &texture,
                        pixel_x,
                        pixel_y,
                        tint_color,
                        DrawTextureParams {
                            dest_size: Some(vec2(width, height)),
                            ..Default::default()
                        },
                    );
                    tiles_count += 1;
                }
            }
        }
        gl_use_default_material();

        tiles_count
    }
}

