// ============================================================================
// Map Viewer - Macroquad 版本
// ============================================================================
//
// 功能：
// - 演示如何使用 macroquad 后端
// - 加载和显示传奇2地图
// - 支持鼠标拖拽、缩放
// - 跨平台支持 (Web/Mobile/Desktop)
//
// 用法：
//   cargo run --bin map_viewer_macroquad --features backend-macroquad --no-default-features
//
// 控制：
//   鼠标拖拽 - 移动地图
//   滚轮     - 缩放
//   ESC      - 退出
//   G        - 切换网格显示
//
// ============================================================================

use macroquad::prelude::*;
use mir2_client::backends::macroquad::MacroquadRenderer;
use mir2_client::backends::{
    Color as RenderColor, DrawParams, Rect, Renderer, TextParams,
    TextureId, TextureManager, Vec2 as RenderVec2,
};
use std::collections::HashMap;

/// 地图查看器状态
struct MapViewerState {
    /// 渲染器
    renderer: MacroquadRenderer,

    /// 相机偏移
    camera_offset: (f32, f32),

    /// 缩放级别
    zoom: f32,

    /// 是否显示网格
    show_grid: bool,

    /// 地图纹理库（暂时为空，后续实现）
    map_library: Option<()>,

    /// 纹理缓存 (MLibrary index -> TextureId)
    texture_cache: HashMap<usize, TextureId>,

    /// 鼠标拖拽状态
    dragging: bool,
    last_mouse_pos: (f32, f32),

    /// FPS 计数
    frame_count: u32,
    fps_timer: f64,
    current_fps: u32,
    
    /// 固定的渲染尺寸（与 ggez 保持一致）
    render_width: f32,
    render_height: f32,
    
    /// RenderTarget 用于离屏渲染
    render_target: RenderTarget,
    
    /// Camera2D 用于处理 RenderTarget 的 Y 轴翻转
    camera: Camera2D,
}

impl MapViewerState {
    async fn new() -> Self {
        let mut renderer = MacroquadRenderer::new();

        // 加载中文字体 - 使用绝对路径
        let font_path = std::env::current_dir()
            .unwrap()
            .join("ClientRust/resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");
        
        if let Err(e) = renderer
            .load_font(
                "AlibabaPuHuiTi",
                font_path.to_str().unwrap(),
            )
            .await
        {
            tracing::warn!("⚠️ 加载字体失败: {}", e);
        } else {
            tracing::info!("✅ 加载字体成功");
        }

        // 使用固定的渲染尺寸，与 ggez 保持一致
        let render_width = 1600.0;
        let render_height = 1200.0;
        
        // 创建 RenderTarget 用于离屏渲染
        let render_target = render_target(render_width as u32, render_height as u32);
        render_target.texture.set_filter(FilterMode::Linear);
        
        // 创建 Camera2D，使用正常的坐标系（不翻转）
        let camera = Camera2D {
            zoom: vec2(1.0 / (render_width / 2.0), 1.0 / (render_height / 2.0)), // 正常缩放
            target: vec2(render_width / 2.0, render_height / 2.0),
            offset: vec2(0.0, 0.0),
            rotation: 0.0,
            render_target: Some(render_target.clone()),
            viewport: None,
        };
        
        // 输出实际屏幕尺寸信息
        let (screen_w, screen_h) = renderer.screen_size();
        tracing::info!("📐 窗口实际尺寸: {}x{}", screen_w, screen_h);
        tracing::info!("📐 使用固定渲染尺寸: {}x{}", render_width, render_height);
        
        Self {
            renderer,
            camera_offset: (0.0, 0.0),
            zoom: 1.0,
            show_grid: false,
            map_library: None,
            texture_cache: HashMap::new(),
            dragging: false,
            last_mouse_pos: (0.0, 0.0),
            frame_count: 0,
            fps_timer: 0.0,
            current_fps: 0,
            render_width,
            render_height,
            render_target,
            camera,
        }
    }

    /// 加载地图资源
    fn load_map_resources(&mut self, _data_path: &str) -> anyhow::Result<()> {
        tracing::info!("📦 加载地图资源库...");
        // TODO: 实现 MLibrary 加载逻辑
        tracing::warn!("⚠️ 地图资源加载功能尚未实现");
        Ok(())
    }

    /// 获取或创建纹理
    fn get_texture(&mut self, _index: usize) -> Option<TextureId> {
        // TODO: 实现纹理加载
        None
    }

    /// 更新逻辑
    fn update(&mut self) {
        let dt = get_frame_time() as f64;

        // FPS 计算
        self.frame_count += 1;
        self.fps_timer += dt;
        if self.fps_timer >= 1.0 {
            self.current_fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer -= 1.0;
        }

        // 键盘输入
        if is_key_pressed(KeyCode::Escape) {
            tracing::info!("👋 用户按下 ESC，退出程序");
            std::process::exit(0);
        }

        if is_key_pressed(KeyCode::G) {
            self.show_grid = !self.show_grid;
            tracing::info!("🔲 网格显示: {}", self.show_grid);
        }

        // 鼠标滚轮缩放
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            self.zoom *= 1.0 + wheel * 0.1;
            self.zoom = self.zoom.clamp(0.1, 5.0);
        }

        // 鼠标拖拽
        let (mouse_x, mouse_y) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) {
            self.dragging = true;
            self.last_mouse_pos = (mouse_x, mouse_y);
        } else if is_mouse_button_released(MouseButton::Left) {
            self.dragging = false;
        }

        if self.dragging {
            let dx = mouse_x - self.last_mouse_pos.0;
            let dy = mouse_y - self.last_mouse_pos.1;
            self.camera_offset.0 += dx;
            self.camera_offset.1 += dy;
            self.last_mouse_pos = (mouse_x, mouse_y);
        }
    }

    /// 渲染
    fn draw(&mut self) {
        // 第一步：渲染游戏世界到 RenderTarget（1600x1200）
        set_camera(&self.camera);
        
        // 清空 RenderTarget
        self.renderer
            .clear(RenderColor::from_rgba_u8(30, 30, 40, 255));

        // 绘制演示内容到 RenderTarget
        self.draw_demo_tiles();

        // 绘制网格
        if self.show_grid {
            self.draw_grid();
        }

        // 第二步：切换回默认相机，将 RenderTarget 绘制到屏幕
        set_default_camera();
        
        // 清空屏幕
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 将 RenderTarget 绘制到屏幕，翻转 Y 轴（因为 RenderTarget 坐标系是上下颠倒的）
        let (screen_w, screen_h) = (screen_width(), screen_height());
        draw_texture_ex(
            &self.render_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_w, screen_h)),
                flip_y: true,  // 翻转 Y 轴，将 RenderTarget 的内容正确显示
                ..Default::default()
            },
        );

        // 第三步：在屏幕空间绘制 UI（避免文字被翻转）
        self.draw_ui();

        // 提交渲染
        let _ = self.renderer.present();
    }

    /// 绘制演示图块
    fn draw_demo_tiles(&mut self) {
        let screen_w = self.render_width;
        let screen_h = self.render_height;
        let tile_size = 48.0 * self.zoom; // 传奇2 标准图块大小

        let start_x = -self.camera_offset.0 / tile_size;
        let start_y = -self.camera_offset.1 / tile_size;

        let tiles_x = (screen_w / tile_size).ceil() as i32 + 2;
        let tiles_y = (screen_h / tile_size).ceil() as i32 + 2;

        for y in 0..tiles_y {
            for x in 0..tiles_x {
                let tile_x = start_x as i32 + x;
                let tile_y = start_y as i32 + y;

                if tile_x < 0 || tile_y < 0 {
                    continue;
                }

                let screen_x = self.camera_offset.0 + tile_x as f32 * tile_size;
                let screen_y = self.camera_offset.1 + tile_y as f32 * tile_size;

                // 绘制棋盘格演示图块
                let is_dark = (tile_x + tile_y) % 2 == 0;
                let color = if is_dark {
                    RenderColor::from_rgba_u8(60, 80, 100, 255)
                } else {
                    RenderColor::from_rgba_u8(80, 100, 120, 255)
                };

                self.renderer.draw_rect(
                    Rect {
                        x: screen_x,
                        y: screen_y,
                        w: tile_size,
                        h: tile_size,
                    },
                    color,
                );

                // 添加边框
                self.renderer.draw_rect(
                    Rect {
                        x: screen_x,
                        y: screen_y,
                        w: tile_size,
                        h: 1.0,
                    },
                    RenderColor::from_rgba_u8(100, 120, 140, 100),
                );
                self.renderer.draw_rect(
                    Rect {
                        x: screen_x,
                        y: screen_y,
                        w: 1.0,
                        h: tile_size,
                    },
                    RenderColor::from_rgba_u8(100, 120, 140, 100),
                );
            }
        }
    }

    /// 绘制网格
    fn draw_grid(&mut self) {
        let screen_w = self.render_width;
        let screen_h = self.render_height;
        let tile_size = 48.0 * self.zoom;

        let grid_color = RenderColor::from_rgba_u8(255, 255, 255, 50);

        // 垂直线
        let mut x = self.camera_offset.0 % tile_size;
        while x < screen_w {
            self.renderer.draw_line(
                RenderVec2::new(x, 0.0),
                RenderVec2::new(x, screen_h),
                1.0,
                grid_color,
            );
            x += tile_size;
        }

        // 水平线
        let mut y = self.camera_offset.1 % tile_size;
        while y < screen_h {
            self.renderer.draw_line(
                RenderVec2::new(0.0, y),
                RenderVec2::new(screen_w, y),
                1.0,
                grid_color,
            );
            y += tile_size;
        }
    }

    /// 绘制 UI
    fn draw_ui(&mut self) {
        let text_color = RenderColor::WHITE;

        // UI 在屏幕空间绘制，使用正常的屏幕坐标（左上角原点）
        // 标题（距离顶部 30 像素）
        self.renderer.draw_text(
            "传奇2地图查看器 - Macroquad版本",
            RenderVec2::new(10.0, 30.0),
            TextParams {
                font_size: 24.0,
                color: text_color,
                font_name: Some("AlibabaPuHuiTi".to_string()),
                ..Default::default()
            },
        );

        // 状态信息（距离顶部 60 像素）
        let info = format!(
            "FPS: {} | 缩放: {:.1}x | 相机: ({:.0}, {:.0}) | 网格: {}",
            self.current_fps,
            self.zoom,
            -self.camera_offset.0,
            -self.camera_offset.1,
            if self.show_grid { "开" } else { "关" }
        );

        self.renderer.draw_text(
            &info,
            RenderVec2::new(10.0, 60.0),
            TextParams {
                font_size: 16.0,
                color: text_color,
                font_name: Some("AlibabaPuHuiTi".to_string()),
                ..Default::default()
            },
        );

        // 控制提示（距离顶部 90 像素）
        self.renderer.draw_text(
            "控制: 鼠标拖拽移动 | 滚轮缩放 | G 切换网格 | ESC 退出",
            RenderVec2::new(10.0, 90.0),
            TextParams {
                font_size: 14.0,
                color: RenderColor::from_rgba_u8(200, 200, 200, 255),
                font_name: Some("AlibabaPuHuiTi".to_string()),
                ..Default::default()
            },
        );
    }
}

/// macroquad 窗口配置
fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2地图查看器 - Macroquad".to_owned(),
        window_width: 1024,   // 窗口实际大小
        window_height: 768,   // 窗口实际大小
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🚀 Macroquad Map Viewer 启动中...");

    // 创建状态
    let mut state = MapViewerState::new().await;

    // 暂时跳过资源加载，先展示界面
    tracing::info!("💡 资源加载功能尚未实现，将显示演示界面");

    tracing::info!("✅ 初始化完成，进入主循环");

    // 主循环
    loop {
        state.update();
        state.draw();
        next_frame().await
    }
}
