// ============================================================================
// 2D 相机系统
// ============================================================================

use macroquad::prelude::*;

/// 2D 游戏相机
#[derive(Debug, Clone)]
pub struct GameCamera2D {
    /// 相机位置（世界坐标 - 相机看向的中心点）
    pub position: Vec2,
    
    /// 缩放级别
    pub zoom: f32,
    
    /// 渲染尺寸
    pub render_width: f32,
    pub render_height: f32,
    
    /// 是否正在拖拽
    dragging: bool,
    
    /// 上次鼠标位置
    last_mouse_pos: Vec2,
    
    /// 是否是第一次拖拽（用于忽略窗口激活时的异常 delta）
    first_drag: bool,
}

impl GameCamera2D {
    /// 创建新的相机
    pub fn new(render_width: f32, render_height: f32) -> Self {
        Self {
            position: vec2(0.0, 0.0),
            zoom: 1.0,
            render_width,
            render_height,
            dragging: false,
            last_mouse_pos: vec2(0.0, 0.0),
            first_drag: true,
        }
    }
    
    /// 设置相机位置
    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }
    
    /// 设置缩放级别
    pub fn set_zoom(&mut self, zoom: f32, min_zoom: f32, max_zoom: f32) {
        self.zoom = zoom.clamp(min_zoom, max_zoom);
    }
    
    /// 调整缩放（增量）
    pub fn adjust_zoom(&mut self, delta: f32, min_zoom: f32, max_zoom: f32) {
        self.zoom = (self.zoom + delta).clamp(min_zoom, max_zoom);
    }
    
    /// 平移相机
    pub fn translate(&mut self, delta: Vec2) {
        self.position += delta;
    }
    
    /// 开始拖拽
    pub fn start_drag(&mut self, mouse_pos: Vec2) {
        self.dragging = true;
        self.last_mouse_pos = mouse_pos;
        self.first_drag = true;
        println!("🖱️ 开始拖拽 at ({:.1}, {:.1})", mouse_pos.x, mouse_pos.y);
    }
    
    /// 停止拖拽
    pub fn stop_drag(&mut self) {
        self.dragging = false;
        println!("🖱️ 停止拖拽");
    }
    
    /// 更新拖拽
    pub fn update_drag(&mut self, current_mouse_pos: Vec2) {
        if !self.dragging {
            return;
        }
        
        let delta = current_mouse_pos - self.last_mouse_pos;
        let delta_magnitude = delta.length();
        
        // 首次拖拽保护：忽略过大的 delta
        if self.first_drag {
            if delta_magnitude > 100.0 {
                println!("⚠️ 忽略首次异常拖拽 delta: {:.1}", delta_magnitude);
                self.last_mouse_pos = current_mouse_pos;
                return;
            } else if delta_magnitude > 0.1 {
                println!("✅ 首次拖拽有效 delta: {:.1}", delta_magnitude);
                self.first_drag = false;
            }
        }
        
        // 应用拖拽偏移（考虑缩放）
        let camera_delta = delta / self.zoom;
        self.position -= camera_delta;
        
        self.last_mouse_pos = current_mouse_pos;
    }
    
    /// 是否正在拖拽
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }
    
    /// 屏幕坐标转世界坐标
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // 计算相对于屏幕中心的偏移
        let offset_x = screen_pos.x - self.render_width / 2.0;
        let offset_y = screen_pos.y - self.render_height / 2.0;
        
        // 应用缩放和相机位置
        vec2(
            self.position.x + offset_x / self.zoom,
            self.position.y + offset_y / self.zoom,
        )
    }
    
    /// 世界坐标转屏幕坐标
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        let offset_x = (world_pos.x - self.position.x) * self.zoom;
        let offset_y = (world_pos.y - self.position.y) * self.zoom;
        
        vec2(
            self.render_width / 2.0 + offset_x,
            self.render_height / 2.0 + offset_y,
        )
    }
    
    /// 转换为 macroquad 的 Camera2D
    pub fn to_macroquad_camera(&self) -> Camera2D {
        Camera2D {
            target: self.position,
            zoom: vec2(
                2.0 / self.render_width * self.zoom,
                2.0 / self.render_height * self.zoom,
            ),
            offset: vec2(0.0, 0.0),
            render_target: None,
            rotation: 0.0,
            viewport: None,
        }
    }
}
