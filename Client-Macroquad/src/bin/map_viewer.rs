// ============================================================================
// 传奇2地图查看器 - Macroquad 独立版本
// ============================================================================
//
// 说明：
// - 完全独立的 macroquad 项目，不依赖 ClientRust
// - 所有核心模块已复制到本项目
// - 过扫描渲染策略：底部扩展200px渲染区域避免高建筑物被裁切
// - Camera2D 相机控制（拖拽、缩放）
//
// 运行方式：
// cargo run --release
// ============================================================================

// Windows: Release 模式不弹控制台（Debug 仍保留控制台便于调试）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;
use macroquad::text::draw_text_ex;
use macroquad_profiler::ProfilerParams;

// 引用库模块
use client_macroquad::map_renderer::MeshMapRenderer;
use client_macroquad::resources::{init_map_libraries, MapReader};

// ============================================================================
// 常量配置
// ============================================================================

/// 窗口尺寸（实际显示窗口大小）
const WINDOW_WIDTH: i32 = 1600;
const WINDOW_HEIGHT: i32 = 1200;

/// 渲染尺寸 - 内部渲染分辨率，可以不同于窗口尺寸
const RENDER_WIDTH: f32 = 1600.0;
const RENDER_HEIGHT: f32 = 1200.0;

/// 传奇2 瓦片尺寸
const TILE_WIDTH: f32 = 48.0;
const TILE_HEIGHT: f32 = 32.0;

use std::sync::atomic::{AtomicBool, Ordering};

/// 调试：首次渲染标志
static FIRST_RENDER: AtomicBool = AtomicBool::new(true);

// ============================================================================
// 渲染配置
// ============================================================================

/// 渲染质量配置
#[derive(Debug, Clone)]
struct RenderConfig {
    /// Gamma校正值 (1.0 = 无校正, 2.2 = 标准sRGB)
    gamma: f32,

    /// 亮度增益 (1.0 = 原始亮度, >1.0 变亮, <1.0 变暗)
    brightness: f32,

    /// 对比度增益 (1.0 = 原始对比度, >1.0 增强, <1.0 降低)
    contrast: f32,

    /// 饱和度增益 (1.0 = 原始饱和度, 0.0 = 灰度, >1.0 过饱和)
    saturation: f32,

    /// 混合模式 Alpha (0.0-1.0, 控制纹理透明度)
    blend_alpha: f32,

    /// 是否启用色调映射 (HDR -> LDR)
    tone_mapping: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            gamma: 1.0,          // 默认无gamma校正
            brightness: 1.0,     // 默认亮度
            contrast: 1.0,       // 默认对比度
            saturation: 1.0,     // 默认饱和度
            blend_alpha: 1.0,    // 默认完全不透明
            tone_mapping: false, // 默认禁用色调映射
        }
    }
}

impl RenderConfig {
    /// 应用颜色调整到像素
    fn apply_color_adjustment(&self, color: Color) -> Color {
        let mut r = color.r;
        let mut g = color.g;
        let mut b = color.b;
        let a = color.a * self.blend_alpha;

        // 1. Gamma校正 (线性空间 -> sRGB空间)
        if self.gamma != 1.0 {
            r = r.powf(1.0 / self.gamma);
            g = g.powf(1.0 / self.gamma);
            b = b.powf(1.0 / self.gamma);
        }

        // 2. 亮度调整
        if self.brightness != 1.0 {
            r *= self.brightness;
            g *= self.brightness;
            b *= self.brightness;
        }

        // 3. 对比度调整 (以0.5为中心点拉伸)
        if self.contrast != 1.0 {
            r = (r - 0.5) * self.contrast + 0.5;
            g = (g - 0.5) * self.contrast + 0.5;
            b = (b - 0.5) * self.contrast + 0.5;
        }

        // 4. 饱和度调整 (转换到HSV空间)
        if self.saturation != 1.0 {
            let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
            r = luminance + (r - luminance) * self.saturation;
            g = luminance + (g - luminance) * self.saturation;
            b = luminance + (b - luminance) * self.saturation;
        }

        // 5. 色调映射 (简单Reinhard)
        if self.tone_mapping {
            r = r / (1.0 + r);
            g = g / (1.0 + g);
            b = b / (1.0 + b);
        }

        // 6. 裁剪到合法范围
        Color::new(
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
            a.clamp(0.0, 1.0),
        )
    }

    /// 高质量预设
    fn preset_high_quality() -> Self {
        Self {
            gamma: 2.2,       // sRGB标准gamma
            brightness: 1.05, // 稍微增亮
            contrast: 1.1,    // 轻微增强对比度
            saturation: 1.15, // 稍微增强饱和度
            blend_alpha: 1.0,
            tone_mapping: false,
        }
    }

    /// 低质量预设 (高性能)
    fn preset_low_quality() -> Self {
        Self::default() // 不做任何处理
    }

    /// 复古风格预设
    fn preset_retro() -> Self {
        Self {
            gamma: 1.8,
            brightness: 0.95,
            contrast: 1.2,
            saturation: 0.85,
            blend_alpha: 1.0,
            tone_mapping: false,
        }
    }
}

// ============================================================================
// 主程序
// ============================================================================

struct MapViewerState {
    /// 相机
    camera: Camera2D,

    /// 相机位置（世界坐标 - 相机看向的中心点）
    camera_position: Vec2,

    /// 缩放级别
    zoom: f32,

    /// 是否正在拖拽
    dragging: bool,

    /// 上次鼠标位置
    last_mouse_pos: Vec2,

    /// 是否是第一次拖拽（用于忽略窗口激活时的异常 delta）
    first_drag: bool,

    /// 是否显示网格
    show_grid: bool,

    /// 是否显示纹理边框
    show_texture_border: bool,

    /// 层级显示控制
    show_back_layer: bool,
    show_middle_layer: bool,
    show_front_layer: bool,

    /// FPS 计数
    frame_count: u32,
    fps_timer: f32,
    current_fps: u32,
    frame_time_ms: f32, // 帧时间（毫秒）

    /// 性能测试
    benchmark_mode: bool,
    benchmark_frames: u32,
    benchmark_total_time: f32,

    /// 渲染统计
    tiles_rendered: u32,

    /// 字体
    font: Option<Font>,

    /// 地图数据
    map_reader: Option<MapReader>,

    /// 地图渲染器（使用Mesh Batching优化）
    map_renderer: MeshMapRenderer,

    /// 鼠标世界坐标
    mouse_world_pos: Vec2,

    /// 鼠标对应的地图格子坐标
    mouse_tile_x: i32,
    mouse_tile_y: i32,

    /// 渲染配置
    render_config: RenderConfig,

    /// 当前配置预设索引 (0=低质量, 1=默认, 2=高质量, 3=复古)
    config_preset_index: usize,
}

impl MapViewerState {
    async fn new() -> Result<Self, String> {
        // 资源根目录：使用绝对路径，避免从不同工作目录启动时找不到 Data/
        let data_dir = format!("{}/Data", env!("CARGO_MANIFEST_DIR"));
        client_macroquad::resources::resource_manager::set_data_path(&data_dir);
        client_macroquad::resources::libraries::set_data_path(data_dir);

        // 创建 Camera2D (初始值,会在 update_camera 中更新)
        let camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(2.0 / RENDER_WIDTH, 2.0 / RENDER_HEIGHT),
            offset: vec2(0.0, 0.0),
            render_target: None,
            rotation: 0.0,
            viewport: None,
        };

        // 加载字体
        let font_data = include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let font =
            load_ttf_font_from_bytes(font_data).map_err(|e| format!("加载字体失败: {}", e))?;

        // 加载地图（使用 n0.map - 新手村，有漂亮的地图动画）
        println!("🗺️ 正在加载地图...");
        let map_path = client_macroquad::resources::map_reader::resolve_map_path("n0");
        let map_reader = match MapReader::new(&map_path) {
            Ok(reader) => {
                println!("✅ 地图加载成功: {}x{}", reader.width, reader.height);
                Some(reader)
            }
            Err(e) => {
                println!("⚠️ 地图加载失败: {} (将显示占位符)", e);
                None
            }
        };

        // 加载所有地图库 (MapLib_0 到 MapLib_399)
        println!("📦 正在加载图块库...");
        if let Err(e) = init_map_libraries() {
            println!("⚠️ 地图库加载失败: {}", e);
        }
        println!("✅ 图块库加载完成");

        // 如果地图加载成功，设置相机初始位置到地图中心
        let camera_position = if let Some(ref map) = map_reader {
            // 将相机设置到地图中心
            let center_x = (map.width as f32 / 2.0) * TILE_WIDTH;
            let center_y = (map.height as f32 / 2.0) * TILE_HEIGHT;
            println!("🎯 初始相机位置: ({:.0}, {:.0})", center_x, center_y);
            println!("📏 地图尺寸: {}x{} 格子", map.width, map.height);
            println!(
                "📏 世界尺寸: {:.0}x{:.0} 像素",
                map.width as f32 * TILE_WIDTH,
                map.height as f32 * TILE_HEIGHT
            );
            vec2(center_x, center_y)
        } else {
            vec2(0.0, 0.0)
        };

        // 创建地图渲染器 (使用Mesh Batching)
        let map_renderer = MeshMapRenderer::new(TILE_WIDTH, TILE_HEIGHT);

        Ok(Self {
            camera,
            camera_position,
            zoom: 0.5, // 初始缩放：0.5 = 缩小看更大范围
            dragging: false,
            last_mouse_pos: vec2(0.0, 0.0),
            first_drag: true, // 第一次拖拽
            show_grid: false,
            show_texture_border: false,
            show_back_layer: true,
            show_middle_layer: true,
            show_front_layer: true,
            frame_count: 0,
            fps_timer: 0.0,
            current_fps: 0,
            frame_time_ms: 0.0,
            benchmark_mode: false,
            benchmark_frames: 0,
            benchmark_total_time: 0.0,
            tiles_rendered: 0,
            font: Some(font),
            map_reader,
            map_renderer,
            mouse_world_pos: vec2(0.0, 0.0),
            mouse_tile_x: 0,
            mouse_tile_y: 0,
            render_config: RenderConfig::default(),
            config_preset_index: 1, // 默认配置
        })
    }

    fn update(&mut self) {
        let dt = get_frame_time();

        // 更新地图渲染器（动画计数器）
        self.map_renderer.update(dt);

        // 计算鼠标世界坐标
        let mouse_screen = mouse_position();
        // 屏幕坐标 -> 渲染目标坐标
        let mouse_render_x = (mouse_screen.0 / WINDOW_WIDTH as f32) * RENDER_WIDTH;
        let mouse_render_y = (mouse_screen.1 / WINDOW_HEIGHT as f32) * RENDER_HEIGHT;

        // 渲染目标坐标 -> 世界坐标
        // 相机位置是世界坐标中心点，鼠标相对于渲染中心的偏移除以缩放后加上相机位置
        let mouse_offset_x = (mouse_render_x - RENDER_WIDTH / 2.0) / self.zoom;
        let mouse_offset_y = (mouse_render_y - RENDER_HEIGHT / 2.0) / self.zoom;

        self.mouse_world_pos.x = self.camera_position.x + mouse_offset_x;
        self.mouse_world_pos.y = self.camera_position.y + mouse_offset_y;

        // 世界坐标 -> 地图格子坐标
        self.mouse_tile_x = (self.mouse_world_pos.x / TILE_WIDTH).floor() as i32;
        self.mouse_tile_y = (self.mouse_world_pos.y / TILE_HEIGHT).floor() as i32;

        // FPS 计算
        self.frame_count += 1;
        self.fps_timer += dt;
        self.frame_time_ms = dt * 1000.0; // 转换为毫秒
        if self.fps_timer >= 1.0 {
            self.current_fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer -= 1.0;
        }

        // 键盘输入处理

        // 键盘输入
        if is_key_pressed(KeyCode::Escape) {
            std::process::exit(0);
        }

        // P 键：性能测试模式
        if is_key_pressed(KeyCode::P) {
            self.benchmark_mode = !self.benchmark_mode;
            if self.benchmark_mode {
                self.benchmark_frames = 0;
                self.benchmark_total_time = 0.0;
                println!("🔥 性能测试模式：开启（测试300帧）");
            } else {
                println!("⏹️ 性能测试模式：关闭");
            }
        }

        if is_key_pressed(KeyCode::G) {
            self.show_grid = !self.show_grid;
            println!(
                "🔲 网格显示: {}",
                if self.show_grid { "开启" } else { "关闭" }
            );
        }

        if is_key_pressed(KeyCode::B) {
            self.show_texture_border = !self.show_texture_border;
            println!(
                "🖼️ 纹理边框: {}",
                if self.show_texture_border {
                    "开启"
                } else {
                    "关闭"
                }
            );
        }

        // 层级控制
        if is_key_pressed(KeyCode::Key1) {
            self.show_back_layer = !self.show_back_layer;
            println!(
                "🗺️ Back层(背景): {}",
                if self.show_back_layer {
                    "显示"
                } else {
                    "隐藏"
                }
            );
        }

        if is_key_pressed(KeyCode::Key2) {
            self.show_middle_layer = !self.show_middle_layer;
            println!(
                "🗺️ Middle层(中间): {}",
                if self.show_middle_layer {
                    "显示"
                } else {
                    "隐藏"
                }
            );
        }

        if is_key_pressed(KeyCode::Key3) {
            self.show_front_layer = !self.show_front_layer;
            println!(
                "🗺️ Front层(前景): {}",
                if self.show_front_layer {
                    "显示"
                } else {
                    "隐藏"
                }
            );
        }

        // Q/E键：切换渲染配置预设
        if is_key_pressed(KeyCode::Q) {
            // 上一个预设
            if self.config_preset_index > 0 {
                self.config_preset_index -= 1;
            } else {
                self.config_preset_index = 3; // 循环到最后
            }
            self.apply_config_preset();
        }

        if is_key_pressed(KeyCode::E) {
            // 下一个预设
            self.config_preset_index = (self.config_preset_index + 1) % 4;
            self.apply_config_preset();
        }

        // 微调控制 (Shift + 数字键)
        let shift_held = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

        if shift_held {
            // Shift+Up/Down: Gamma调整
            if is_key_pressed(KeyCode::Up) {
                self.render_config.gamma = (self.render_config.gamma + 0.1).min(3.0);
                println!("🎨 Gamma: {:.2}", self.render_config.gamma);
            }
            if is_key_pressed(KeyCode::Down) {
                self.render_config.gamma = (self.render_config.gamma - 0.1).max(0.5);
                println!("🎨 Gamma: {:.2}", self.render_config.gamma);
            }

            // Shift+Left/Right: 亮度调整
            if is_key_pressed(KeyCode::Right) {
                self.render_config.brightness = (self.render_config.brightness + 0.05).min(2.0);
                println!("💡 亮度: {:.2}", self.render_config.brightness);
            }
            if is_key_pressed(KeyCode::Left) {
                self.render_config.brightness = (self.render_config.brightness - 0.05).max(0.1);
                println!("💡 亮度: {:.2}", self.render_config.brightness);
            }

            // Shift+PageUp/PageDown: 对比度调整
            if is_key_pressed(KeyCode::PageUp) {
                self.render_config.contrast = (self.render_config.contrast + 0.05).min(2.0);
                println!("🔆 对比度: {:.2}", self.render_config.contrast);
            }
            if is_key_pressed(KeyCode::PageDown) {
                self.render_config.contrast = (self.render_config.contrast - 0.05).max(0.1);
                println!("🔆 对比度: {:.2}", self.render_config.contrast);
            }

            // Shift+Home/End: 饱和度调整
            if is_key_pressed(KeyCode::Home) {
                self.render_config.saturation = (self.render_config.saturation + 0.05).min(2.0);
                println!("🌈 饱和度: {:.2}", self.render_config.saturation);
            }
            if is_key_pressed(KeyCode::End) {
                self.render_config.saturation = (self.render_config.saturation - 0.05).max(0.0);
                println!("🌈 饱和度: {:.2}", self.render_config.saturation);
            }

            // Shift+T: 切换色调映射
            if is_key_pressed(KeyCode::T) {
                self.render_config.tone_mapping = !self.render_config.tone_mapping;
                println!(
                    "🎬 色调映射: {}",
                    if self.render_config.tone_mapping {
                        "开启"
                    } else {
                        "关闭"
                    }
                );
            }
        }

        // D键：输出鼠标所在格子的详细数据
        if is_key_pressed(KeyCode::D) {
            if let Some(map) = &self.map_reader {
                if let Some(cell) = map.get_cell(self.mouse_tile_x, self.mouse_tile_y) {
                    cell.debug_cell_data(self.mouse_tile_x, self.mouse_tile_y);
                } else {
                    println!(
                        "⚠️ 格子({},{}) 超出地图范围",
                        self.mouse_tile_x, self.mouse_tile_y
                    );
                }
            }
        }

        // R键：重置相机到地图中心
        if is_key_pressed(KeyCode::R) {
            if let Some(map) = &self.map_reader {
                let center_x = (map.width as f32 / 2.0) * TILE_WIDTH;
                let center_y = (map.height as f32 / 2.0) * TILE_HEIGHT;
                self.camera_position = vec2(center_x, center_y);
                self.zoom = 1.0;
                println!("🎯 相机重置到地图中心: ({:.0}, {:.0})", center_x, center_y);
            }
        }

        // H键：显示帮助和当前状态
        if is_key_pressed(KeyCode::H) {
            println!("\n📊 当前状态:");
            println!(
                "   相机位置: ({:.0}, {:.0})",
                self.camera_position.x, self.camera_position.y
            );
            println!("   缩放倍数: {:.2}x", self.zoom);
            if let Some(map) = &self.map_reader {
                println!("   地图大小: {}x{}", map.width, map.height);
                println!(
                    "   地图中心: ({:.0}, {:.0})",
                    (map.width as f32 / 2.0) * TILE_WIDTH,
                    (map.height as f32 / 2.0) * TILE_HEIGHT
                );
            }
            println!(
                "   鼠标格子: ({}, {})",
                self.mouse_tile_x, self.mouse_tile_y
            );
        }

        // 鼠标滚轮缩放
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            let zoom_factor = if wheel > 0.0 { 1.1 } else { 0.9 };
            self.zoom *= zoom_factor;
            // 限制缩放范围：最小0.3x（防止渲染过多格子导致卡顿），最大5.0x
            self.zoom = self.zoom.clamp(0.3, 5.0);
        }

        // 鼠标拖拽
        if is_mouse_button_pressed(MouseButton::Left) {
            self.dragging = true;
            self.last_mouse_pos = mouse_position().into();
            self.first_drag = true; // 每次开始拖拽时重置标志
            println!(
                "🖱️ 开始拖拽 at ({:.1}, {:.1})",
                self.last_mouse_pos.x, self.last_mouse_pos.y
            );
        }

        if is_mouse_button_released(MouseButton::Left) {
            self.dragging = false;
            println!("🖱️ 停止拖拽");
        }

        // 安全机制：如果鼠标左键没按下但dragging为true，强制重置
        // 这解决了窗口失去焦点导致释放事件丢失的问题
        if self.dragging && !is_mouse_button_down(MouseButton::Left) {
            self.dragging = false;
            println!("🖱️ 安全重置拖拽状态");
        }

        if self.dragging {
            let current_pos: Vec2 = mouse_position().into();
            let delta = current_pos - self.last_mouse_pos;

            // 第一次拖拽时，如果 delta 过大（超过 100 像素），忽略这次移动
            // 这解决了窗口启动时未激活，首次点击用于激活窗口的问题
            let delta_magnitude = (delta.x * delta.x + delta.y * delta.y).sqrt();

            if self.first_drag && delta_magnitude > 100.0 {
                // 忽略这次移动，只更新位置
                println!("⚠️ 首次拖拽 delta 过大 ({:.1}), 忽略", delta_magnitude);
                self.last_mouse_pos = current_pos;
                self.first_drag = false;
            } else if delta_magnitude > 0.1 {
                // 只在有实际移动时才处理
                // 正常拖动逻辑
                if self.first_drag {
                    println!("✅ 首次拖拽有效 delta: {:.1}", delta_magnitude);
                    self.first_drag = false;
                }

                // 拖动逻辑：拖动地图（而不是拖动摄像机）
                // 1. delta 是屏幕坐标的变化 (1024x768)
                // 2. 现在渲染坐标和窗口坐标相同 (1024x768)，所以直接转换为世界坐标
                // 3. 鼠标往右拖 -> delta.x > 0 -> 想看左边的地图 -> 相机往左移 -> camera.x -= delta
                let screen_to_render_x = RENDER_WIDTH / WINDOW_WIDTH as f32;
                let screen_to_render_y = RENDER_HEIGHT / WINDOW_HEIGHT as f32;

                let world_delta_x = delta.x * screen_to_render_x / self.zoom;
                let world_delta_y = delta.y * screen_to_render_y / self.zoom;

                // 鼠标往右拖，相机往左移，看到左边的地图（拖动地图的效果）
                self.camera_position.x -= world_delta_x;
                self.camera_position.y -= world_delta_y;

                // 更新鼠标位置，避免累积误差导致抖动
                self.last_mouse_pos = current_pos;
            }
        } else {
            // 不在拖拽时，也更新 last_mouse_pos，避免下次拖拽时产生巨大的 delta
            self.last_mouse_pos = mouse_position().into();
        }
    }

    /// 应用预设配置
    fn apply_config_preset(&mut self) {
        let (config, name) = match self.config_preset_index {
            0 => (RenderConfig::preset_low_quality(), "低质量(高性能)"),
            1 => (RenderConfig::default(), "默认"),
            2 => (RenderConfig::preset_high_quality(), "高质量"),
            3 => (RenderConfig::preset_retro(), "复古风格"),
            _ => (RenderConfig::default(), "默认"),
        };

        self.render_config = config.clone();
        println!("\n🎨 渲染预设: {}", name);
        println!("   Gamma: {:.2}", config.gamma);
        println!("   亮度: {:.2}", config.brightness);
        println!("   对比度: {:.2}", config.contrast);
        println!("   饱和度: {:.2}", config.saturation);
        println!(
            "   色调映射: {}",
            if config.tone_mapping {
                "开启"
            } else {
                "关闭"
            }
        );
    }

    /// 更新相机参数
    fn update_camera(&mut self) {
        self.camera.target = self.camera_position;
        self.camera.zoom = vec2(
            2.0 / RENDER_WIDTH * self.zoom,
            2.0 / RENDER_HEIGHT * self.zoom,
        );
    }

    fn draw(&mut self) {
        // 清空屏幕
        clear_background(Color::from_rgba(40, 40, 50, 255));
        // 更新并设置游戏相机
        self.update_camera();
        set_camera(&self.camera);

        // 绘制地图
        if self.map_reader.is_some() {
            self.draw_map();
        } else {
            self.draw_placeholder();
        }

        // 绘制网格
        if self.show_grid {
            self.draw_grid();
        }
        set_default_camera();
        // 不切换相机,继续在同一相机空间绘制UI (这会让UI也跟随相机移动)
        self.draw_ui();
    }

    /// 绘制占位符（当没有地图时）
    fn draw_placeholder(&self) {
        // 绘制一个简单的棋盘格作为占位符
        let grid_size = 100.0;
        let start_x =
            ((self.camera_position.x - RENDER_WIDTH / 2.0) / grid_size).floor() as i32 - 1;
        let start_y =
            ((self.camera_position.y - RENDER_HEIGHT / 2.0) / grid_size).floor() as i32 - 1;
        let end_x = start_x + (RENDER_WIDTH / grid_size).ceil() as i32 + 2;
        let end_y = start_y + (RENDER_HEIGHT / grid_size).ceil() as i32 + 2;

        for y in start_y..end_y {
            for x in start_x..end_x {
                let color = if (x + y) % 2 == 0 {
                    Color::from_rgba(60, 60, 70, 255)
                } else {
                    Color::from_rgba(50, 50, 60, 255)
                };

                draw_rectangle(
                    x as f32 * grid_size,
                    y as f32 * grid_size,
                    grid_size,
                    grid_size,
                    color,
                );
            }
        }

        // 中心标记
        draw_circle(0.0, 0.0, 20.0, RED);
        draw_line(-50.0, 0.0, 50.0, 0.0, 3.0, WHITE);
        draw_line(0.0, -50.0, 0.0, 50.0, 3.0, WHITE);
    }

    /// 绘制真实地图 (参考 ggez 版本的 MapRenderSystem)
    fn draw_map(&mut self) {
        let map = match &self.map_reader {
            Some(m) => m,
            None => return,
        };

        // 同步层级显示设置到渲染器
        self.map_renderer.show_back_layer = self.show_back_layer;
        self.map_renderer.show_middle_layer = self.show_middle_layer;
        self.map_renderer.show_front_layer = self.show_front_layer;
        self.map_renderer.show_texture_border = self.show_texture_border;

        // 应用颜色调整
        let tint_color = self.render_config.apply_color_adjustment(WHITE);

        // 使用 MapRenderer 渲染地图
        self.tiles_rendered = self.map_renderer.render(
            map,
            self.camera_position.x,
            self.camera_position.y,
            RENDER_WIDTH,
            RENDER_HEIGHT,
            self.zoom,
            tint_color,
        );

        // 首次渲染标记
        if FIRST_RENDER.load(Ordering::Relaxed) {
            FIRST_RENDER.store(false, Ordering::Relaxed);
        }
    }

    /// 绘制网格（地图格子网格，48x32）
    fn draw_grid(&self) {
        let map = match &self.map_reader {
            Some(m) => m,
            None => return,
        };

        // 计算视口范围
        let half_width = (RENDER_WIDTH / 2.0) / self.zoom;
        let half_height = (RENDER_HEIGHT / 2.0) / self.zoom;

        let view_left = self.camera_position.x - half_width;
        let view_right = self.camera_position.x + half_width;
        let view_top = self.camera_position.y - half_height;
        let view_bottom = self.camera_position.y + half_height;

        // 转换为格子坐标
        let start_x = ((view_left / TILE_WIDTH).floor() as i32).max(0);
        let start_y = ((view_top / TILE_HEIGHT).floor() as i32).max(0);
        let end_x = ((view_right / TILE_WIDTH).ceil() as i32 + 1).min(map.width);
        let end_y = ((view_bottom / TILE_HEIGHT).ceil() as i32 + 1).min(map.height);

        let grid_color = Color::from_rgba(255, 255, 0, 80); // 半透明黄色网格
        let text_color = Color::from_rgba(255, 255, 0, 255); // 不透明黄色文字

        // 绘制竖线和格子标注
        for grid_x in start_x..=end_x {
            let world_x = grid_x as f32 * TILE_WIDTH;

            let y1 = start_y as f32 * TILE_HEIGHT;
            let y2 = end_y as f32 * TILE_HEIGHT;

            draw_line(world_x, y1, world_x, y2, 1.0, grid_color);
        }

        // 绘制横线
        for grid_y in start_y..=end_y {
            let world_y = grid_y as f32 * TILE_HEIGHT;

            draw_line(
                start_x as f32 * TILE_WIDTH,
                world_y,
                end_x as f32 * TILE_WIDTH,
                world_y,
                1.0,
                grid_color,
            );
        }

        // 在每个格子中心绘制坐标
        // 只在缩放较大时显示,避免过于密集
        if self.zoom >= 0.8 {
            for grid_y in start_y..end_y {
                for grid_x in start_x..end_x {
                    // 每隔2个格子显示一次,避免太密集
                    if grid_x % 2 == 0 && grid_y % 2 == 0 {
                        let center_x = grid_x as f32 * TILE_WIDTH + TILE_WIDTH / 2.0;
                        let center_y = grid_y as f32 * TILE_HEIGHT + TILE_HEIGHT / 2.0;

                        let label = format!("{},{}", grid_x, grid_y);
                        draw_text_ex(
                            &label,
                            center_x - 15.0,
                            center_y + 3.0,
                            TextParams {
                                font: self.font.as_ref(),
                                font_size: 10,
                                color: text_color,
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }

    /// 绘制 UI（屏幕空间，不受相机影响）
    fn draw_ui(&self) {
        // 标题
        draw_text_ex(
            "传奇2地图查看器 - Macroquad V2",
            10.0,
            35.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 28,
                color: WHITE,
                ..Default::default()
            },
        );

        // 状态信息
        let map_info = if let Some(ref map) = self.map_reader {
            format!("地图: {}x{} | ", map.width, map.height)
        } else {
            "地图: 未加载 | ".to_string()
        };

        let info = format!(
            "{}FPS: {} ({:.2}ms) | 瓦片/帧: {} | 缩放: {:.1}x | 相机: ({:.0}, {:.0})",
            map_info,
            self.current_fps,
            self.frame_time_ms,
            self.tiles_rendered,
            self.zoom,
            self.camera_position.x,
            self.camera_position.y,
        );

        // 性能提示颜色
        let info_color = if self.current_fps >= 100 {
            Color::from_rgba(100, 255, 100, 255) // 绿色 - 优秀
        } else if self.current_fps >= 60 {
            WHITE // 白色 - 良好
        } else {
            Color::from_rgba(255, 200, 100, 255) // 橙色 - 需要优化
        };

        draw_text_ex(
            &info,
            10.0,
            75.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 24,
                color: info_color,
                ..Default::default()
            },
        );

        // 性能提示
        if self.current_fps < 100 && self.tiles_rendered > 1000 {
            let hint = "💡 提示: 放大地图可提升 FPS（瓦片数量减少）";
            draw_text_ex(
                hint,
                RENDER_WIDTH - 450.0,
                75.0,
                TextParams {
                    font: self.font.as_ref(),
                    font_size: 24,
                    color: Color::from_rgba(255, 255, 100, 255),
                    ..Default::default()
                },
            );
        }

        // 层级状态
        let layer_status = format!(
            "Layers: Back[{}] Mid[{}] Front[{}] | Grid[{}] Border[{}]",
            if self.show_back_layer { "ON" } else { "  " },
            if self.show_middle_layer { "ON" } else { "  " },
            if self.show_front_layer { "ON" } else { "  " },
            if self.show_grid { "ON" } else { "  " },
            if self.show_texture_border { "ON" } else { "  " },
        );

        draw_text_ex(
            &layer_status,
            10.0,
            105.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 24,
                color: WHITE,
                ..Default::default()
            },
        );

        // 渲染配置状态
        let preset_name = match self.config_preset_index {
            0 => "低质量",
            1 => "默认",
            2 => "高质量",
            3 => "复古",
            _ => "未知",
        };
        let render_status = format!(
            "渲染[{}] γ:{:.1} 亮:{:.2} 对:{:.2} 饱:{:.2} HDR:{}",
            preset_name,
            self.render_config.gamma,
            self.render_config.brightness,
            self.render_config.contrast,
            self.render_config.saturation,
            if self.render_config.tone_mapping {
                "ON"
            } else {
                "  "
            },
        );

        draw_text_ex(
            &render_status,
            10.0,
            130.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 24,
                color: Color::from_rgba(255, 200, 100, 255),
                ..Default::default()
            },
        );

        // 控制提示
        draw_text_ex(
            "控制: 拖拽移动 | 滚轮缩放 | 1/2/3 层级 | Q/E 渲染预设 | G 网格 | B 边框 | ESC 退出",
            10.0,
            155.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 24,
                color: WHITE,
                ..Default::default()
            },
        );

        draw_text_ex(
            "微调: Shift+↑↓ Gamma | Shift+←→ 亮度 | Shift+PgUp/Dn 对比度 | Shift+Home/End 饱和度 | Shift+T 色调映射",
            10.0,
            175.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 24,
                color: Color::from_rgba(200, 200, 200, 255),
                ..Default::default()
            },
        );

        // 鼠标位置信息
        self.draw_mouse_info();
    }

    /// 绘制鼠标位置和瓦片信息
    fn draw_mouse_info(&self) {
        let y_offset = 180.0;

        // 鼠标世界坐标
        let world_info = format!(
            "鼠标: 世界({:.1}, {:.1}) 格子({}, {})",
            self.mouse_world_pos.x, self.mouse_world_pos.y, self.mouse_tile_x, self.mouse_tile_y,
        );

        draw_text_ex(
            &world_info,
            10.0,
            y_offset,
            TextParams {
                font: self.font.as_ref(),
                font_size: 24,
                color: Color::from_rgba(255, 255, 100, 255),
                ..Default::default()
            },
        );

        // 获取瓦片信息
        if let Some(ref map) = self.map_reader {
            if let Some(cell) = map.get_cell(self.mouse_tile_x, self.mouse_tile_y) {
                let mut line_offset = 20.0;

                // Back 层信息 (总是显示,即使没有纹理)
                match cell.back_tile() {
                    Some((file_idx, img_idx)) => {
                        // 计算实际文件名: MapLib_100=Tiles, 101=Tiles2, 104=Tiles5
                        let actual_file = if file_idx == 100 {
                            "Tiles.Lib".to_string()
                        } else if (101..=109).contains(&file_idx) {
                            format!("Tiles{}.Lib", file_idx - 99)
                        } else if file_idx == 110 {
                            "SmTiles.Lib".to_string()
                        } else if (111..=119).contains(&file_idx) {
                            format!("SmTiles{}.Lib", file_idx - 109)
                        } else if file_idx == 190 {
                            "AniTiles1.Lib".to_string()
                        } else {
                            format!("Unknown_{}.Lib", file_idx)
                        };

                        let back_info = format!(
                            "  Back:   MapLib_{}={}, 图像={}, back_image=0x{:X}",
                            file_idx, actual_file, img_idx, cell.back_image
                        );
                        draw_text_ex(
                            &back_info,
                            10.0,
                            y_offset + line_offset,
                            TextParams {
                                font: self.font.as_ref(),
                                font_size: 24,
                                color: WHITE,
                                ..Default::default()
                            },
                        );
                    }
                    None => {
                        let back_info =
                            format!("  Back:   -1 (无纹理) back_image=0x{:X}", cell.back_image);
                        draw_text_ex(
                            &back_info,
                            10.0,
                            y_offset + line_offset,
                            TextParams {
                                font: self.font.as_ref(),
                                font_size: 24,
                                color: WHITE,
                                ..Default::default()
                            },
                        );
                    }
                }

                line_offset += 20.0;

                // Middle层信息 (总是显示)
                let middle_info = match cell.middle_tile() {
                    Some((file_idx, img_idx)) => {
                        format!(
                            "  Middle: MapLib_{} 图像={} (index={}, image=0x{:X})",
                            file_idx, img_idx, cell.middle_index, cell.middle_image
                        )
                    }
                    None => {
                        format!(
                            "  Middle: -1 (无纹理) (index={}, image=0x{:X})",
                            cell.middle_index, cell.middle_image
                        )
                    }
                };
                draw_text_ex(
                    &middle_info,
                    10.0,
                    y_offset + line_offset,
                    TextParams {
                        font: self.font.as_ref(),
                        font_size: 24,
                        color: WHITE,
                        ..Default::default()
                    },
                );

                line_offset += 20.0;

                // Front层信息 (总是显示)
                let front_info = match cell.front_tile() {
                    Some((file_idx, img_idx)) => {
                        format!(
                            "  Front:  MapLib_{} 图像={} (index={}, image=0x{:X})",
                            file_idx, img_idx, cell.front_index, cell.front_image
                        )
                    }
                    None => {
                        format!(
                            "  Front:  -1 (无纹理) (index={}, image=0x{:X})",
                            cell.front_index, cell.front_image
                        )
                    }
                };
                draw_text_ex(
                    &front_info,
                    10.0,
                    y_offset + line_offset,
                    TextParams {
                        font: self.font.as_ref(),
                        font_size: 24,
                        color: WHITE,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

// ============================================================================
// 入口函数
// ============================================================================

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2地图查看器 - Macroquad V2".to_string(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: true,
        high_dpi: true, // macOS Retina 支持
        fullscreen: false,
        platform: Platform {
            swap_interval: Some(1), // VSync
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // 初始化
    let mut state = MapViewerState::new().await.expect("初始化失败");

    println!("✅ 地图查看器启动成功");
    println!("📐 窗口尺寸: {}x{}", WINDOW_WIDTH, WINDOW_HEIGHT);
    println!(
        "🖼️ 渲染尺寸: {}x{}",
        RENDER_WIDTH as i32, RENDER_HEIGHT as i32
    );
    println!("🎮 控制: 拖拽移动 | 滚轮缩放 | 1/2/3 层级 | G 网格 | B 边框 | ESC 退出");

    // macOS 焦点修复：等待第一帧渲染完成
    // 这样可以确保窗口完全创建后再尝试获取焦点
    next_frame().await;

    // 主循环
    loop {
        state.update();
        state.draw();

        macroquad_profiler::profiler(ProfilerParams::default());
        next_frame().await;
    }
}
