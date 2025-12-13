// GameScene - 游戏主场景
// 
// 快捷键:
//   I = 背包, C = 角色, B = 快捷栏, S = 商城
//   F1-F6 = 快捷栏技能
//   ESC = 返回角色选择

use crate::game::GameResult;
use crate::scenes::dialogs::game::{
    MainDialog,
};
use crate::scenes::{Scene, SceneTransition};
use crate::ui::text_renderer::draw_text_cn;
use crate::{map_renderer::MeshMapRenderer, resources::{init_map_libraries, MapReader}};
use macroquad::prelude::*;

/// 游戏主场景 - 集成所有混合对话框
pub struct GameScene {
    // 地图渲染
    map_reader: Option<MapReader>,
    map_renderer: MeshMapRenderer,
    map_camera: Camera2D,
    map_camera_position: Vec2,
    map_zoom: f32,
    map_dragging: bool,
    map_first_drag: bool,
    map_last_mouse_pos: Vec2,

    // 完整 UI（底部主界面 + 全部子对话框）
    main_dialog: MainDialog,
    
    // 初始化状态
    initialized: bool,
}

impl GameScene {
    pub fn new() -> Self {
        // Camera2D 初始值（真实参数会在地图加载后更新）
        let map_camera = Camera2D {
            target: vec2(0.0, 0.0),
            zoom: vec2(2.0 / screen_width().max(1.0), 2.0 / screen_height().max(1.0)),
            offset: vec2(0.0, 0.0),
            render_target: None,
            rotation: 0.0,
            viewport: None,
        };

        Self {
            map_reader: None,
            map_renderer: MeshMapRenderer::new(48.0, 32.0),
            map_camera,
            map_camera_position: vec2(0.0, 0.0),
            map_zoom: 1.0,
            map_dragging: false,
            map_first_drag: true,
            map_last_mouse_pos: mouse_position().into(),

            main_dialog: MainDialog::new(),
            initialized: false,
        }
    }

    fn update_map_camera(&mut self) {
        self.map_camera.target = self.map_camera_position;
        let sw = screen_width().max(1.0);
        let sh = screen_height().max(1.0);
        self.map_camera.zoom = vec2(2.0 / sw * self.map_zoom, 2.0 / sh * self.map_zoom);
    }

    fn clamp_map_camera_position(&mut self) {
        let Some(map) = self.map_reader.as_ref() else {
            return;
        };

        // 视口半宽/半高（世界坐标，单位：像素）
        let half_w = (screen_width().max(1.0) / 2.0) / self.map_zoom;
        let half_h = (screen_height().max(1.0) / 2.0) / self.map_zoom;

        // 地图总大小（世界坐标，单位：像素）
        let map_w = map.width as f32 * 48.0;
        let map_h = map.height as f32 * 32.0;

        // 若视口比地图大，直接居中
        if map_w <= half_w * 2.0 {
            self.map_camera_position.x = map_w / 2.0;
        } else {
            self.map_camera_position.x = self.map_camera_position.x.clamp(half_w, map_w - half_w);
        }

        if map_h <= half_h * 2.0 {
            self.map_camera_position.y = map_h / 2.0;
        } else {
            self.map_camera_position.y = self.map_camera_position.y.clamp(half_h, map_h - half_h);
        }
    }

    fn handle_map_input(&mut self) {
        if self.map_reader.is_none() {
            return;
        }

        // 为了避免与 UI 点击/拖拽冲突：只有按住 Space 才启用地图交互
        let map_input_enabled = is_key_down(KeyCode::Space);

        if map_input_enabled {
            // 鼠标滚轮缩放
            let wheel_y = mouse_wheel().1;
            if wheel_y != 0.0 {
                let zoom_factor = if wheel_y > 0.0 { 1.1 } else { 0.9 };
                self.map_zoom = (self.map_zoom * zoom_factor).clamp(0.3, 5.0);
            }

            // 鼠标拖拽平移（Space + 左键）
            if is_mouse_button_pressed(MouseButton::Left) {
                self.map_dragging = true;
                self.map_last_mouse_pos = mouse_position().into();
                self.map_first_drag = true;
            }

            if is_mouse_button_released(MouseButton::Left) {
                self.map_dragging = false;
            }
        } else {
            // 没按 Space 时，永远不保持拖拽状态
            self.map_dragging = false;
        }

        // 安全机制：避免窗口失焦导致释放事件丢失
        if self.map_dragging && !is_mouse_button_down(MouseButton::Left) {
            self.map_dragging = false;
        }

        if self.map_dragging {
            let current_pos: Vec2 = mouse_position().into();
            let delta = current_pos - self.map_last_mouse_pos;

            // 第一次拖拽时，如果 delta 过大（例如点击用于激活窗口），忽略
            let delta_magnitude = (delta.x * delta.x + delta.y * delta.y).sqrt();
            if self.map_first_drag && delta_magnitude > 100.0 {
                self.map_last_mouse_pos = current_pos;
                self.map_first_drag = false;
            } else if delta_magnitude > 0.1 {
                self.map_first_drag = false;

                // 公式参考 map_viewer：屏幕像素 delta -> 世界坐标 delta，再除以 zoom
                let world_delta_x = delta.x / self.map_zoom;
                let world_delta_y = delta.y / self.map_zoom;
                self.map_camera_position.x -= world_delta_x;
                self.map_camera_position.y -= world_delta_y;

                // 防止拖出地图边界
                self.clamp_map_camera_position();

                self.map_last_mouse_pos = current_pos;
            }
        } else {
            // 不拖拽时也更新，避免下次拖拽出现巨大 delta
            self.map_last_mouse_pos = mouse_position().into();
        }
    }
    
    /// 异步加载所有对话框纹理
    pub async fn load_textures(&mut self) {
        println!("🎮 GameScene: 加载对话框纹理...");

        self.main_dialog.load_native_textures().await;

        // 加载地图（用于主场景背景渲染）
        // 说明：这里先用固定地图文件，后续接入网络/地图切换系统再动态更新。
        println!("🗺️ GameScene: 初始化地图库...");
        if let Err(e) = init_map_libraries() {
            println!("⚠️ GameScene: 地图库初始化失败: {}", e);
        }

        let map_path = "Map/n0.map";
        match MapReader::new(map_path) {
            Ok(reader) => {
                println!("✅ GameScene: 地图加载成功 {} ({}x{})", map_path, reader.width, reader.height);
                // 初始相机位置：地图中心
                self.map_camera_position = vec2(reader.width as f32 * 48.0 / 2.0, reader.height as f32 * 32.0 / 2.0);
                self.map_zoom = 1.0;
                self.map_reader = Some(reader);
                self.update_map_camera();
            }
            Err(e) => {
                println!("⚠️ GameScene: 地图加载失败 {}: {} (将使用占位网格背景)", map_path, e);
                self.map_reader = None;
            }
        }
        
        self.initialized = true;
        println!("✅ GameScene: 对话框纹理加载完成");
    }
    
    /// 处理快捷键
    fn handle_hotkeys(&mut self) {
        // 如果聊天输入框激活，不处理其他快捷键
        if self.main_dialog.is_any_input_active() {
            return;
        }

        // Enter = 激活聊天输入框
        if is_key_pressed(KeyCode::Enter) {
            self.main_dialog.activate_chat_input();
        }

        // M = 切换小地图显示
        if is_key_pressed(KeyCode::M) {
            self.main_dialog.toggle_minimap();
        }

        // Tab = 切换小地图大小
        if is_key_pressed(KeyCode::Tab) {
            self.main_dialog.toggle_minimap_size();
        }

        // ESC = 先关闭弹窗；若没弹窗则返回角色选择（在 update 中处理返回）
        if is_key_pressed(KeyCode::Escape) {
            if self.main_dialog.any_popup_open() {
                self.main_dialog.close_all_popups();
            }
        }
    }
    
    /// 绘制快捷键提示
    fn draw_help_text(&self) {
        let y = screen_height() - 25.0;
        draw_text_cn(
            "快捷键: Space+拖拽/滚轮=地图 | Enter=聊天 M=小地图 Tab=小地图大小 | ESC=返回角色选择",
            10.0, y, 14.0, Color::from_rgba(200, 200, 200, 180)
        );
    }
}

impl Default for GameScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for GameScene {
    fn name(&self) -> &str { "游戏场景" }
    
    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入游戏场景");
        // 注意: 纹理需要异步加载，这里无法调用 async 函数
        // 应该在进入场景前或通过 Loading 场景预加载
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开游戏场景");
        self.main_dialog.close_all_popups();
        self.main_dialog.deactivate_chat_input();
        Ok(())
    }
    
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        // 地图动画更新
        self.map_renderer.update(_dt);

        // 地图输入（空格模式）
        self.handle_map_input();

        // 处理快捷键
        self.handle_hotkeys();
        
        // ESC 且没有打开的对话框 = 返回角色选择
        if is_key_pressed(KeyCode::Escape) {
            if !self.main_dialog.any_popup_open() {
                return Ok(SceneTransition::CharacterSelect);
            }
        }
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        // 绘制地图背景（若地图未加载则使用占位网格）
        clear_background(Color::from_rgba(30, 45, 30, 255));

        self.update_map_camera();
        if let Some(map) = self.map_reader.as_ref() {
            set_camera(&self.map_camera);
            // 颜色先保持原样，后续若需要再引入 RenderConfig
            let _tiles = self.map_renderer.render(
                map,
                self.map_camera_position.x,
                self.map_camera_position.y,
                screen_width(),
                screen_height(),
                self.map_zoom,
                WHITE,
            );
            set_default_camera();
        } else {
            // 占位网格背景（模拟地图）
            let grid_color = Color::from_rgba(50, 65, 50, 255);
            for i in 0..=((screen_width() / 48.0) as i32 + 1) {
                let x = i as f32 * 48.0;
                draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
            }
            for i in 0..=((screen_height() / 32.0) as i32 + 1) {
                let y = i as f32 * 32.0;
                draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
            }
        }
        
        // 提示文字
        if !self.initialized {
            draw_text_cn(
                "⏳ 正在加载游戏资源...",
                screen_width() / 2.0 - 100.0,
                screen_height() / 2.0,
                24.0, WHITE
            );
        } else {
            // 绘制完整 UI
            self.main_dialog.update_and_draw();
            let _ui_consumed = self.main_dialog.show_dialogs();
            
            // 绘制帮助提示
            self.draw_help_text();
        }
        
        Ok(())
    }
    
    fn handle_input(&mut self) -> GameResult { 
        Ok(()) 
    }
}
