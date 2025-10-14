/// 游戏场景摄像机系统
/// 
/// 功能：
/// - 坐标转换（世界坐标 ↔ 屏幕坐标）
/// - 跟随玩家移动
/// - 可选的缩放功能
/// 
/// 与 map_viewer 的区别：
/// - 自动跟随玩家位置
/// - 不需要手动拖拽（玩家始终在屏幕中心）
/// - 缩放功能可选

use ggez::graphics::{DrawParam, Rect};

/// 游戏摄像机
#[derive(Debug, Clone)]
pub struct Camera {
    /// 摄像机中心的世界坐标 X（像素）
    pub x: f32,
    
    /// 摄像机中心的世界坐标 Y（像素）
    pub y: f32,
    
    /// 缩放级别 (1.0 = 正常, >1.0 = 放大, <1.0 = 缩小)
    pub zoom: f32,
    
    /// 屏幕宽度（像素）
    pub screen_width: f32,
    
    /// 屏幕高度（像素）
    pub
    screen_height: f32,
}

impl Camera {
    /// 创建新的摄像机
    /// 
    /// # 参数
    /// - `screen_width`: 屏幕宽度（像素）
    /// - `screen_height`: 屏幕高度（像素）
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            screen_width,
            screen_height,
        }
    }
    
    /// 更新屏幕尺寸
    pub fn update_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }
    
    /// 设置摄像机跟随目标（通常是玩家）
    /// 
    /// # 参数
    /// - `world_x`: 目标世界坐标 X（像素）
    /// - `world_y`: 目标世界坐标 Y（像素）
    pub fn follow_target(&mut self, world_x: f32, world_y: f32) {
        self.x = world_x;
        self.y = world_y;
    }
    
    /// 设置摄像机跟随目标（带地图边界限制）
    /// 
    /// # 参数
    /// - `world_x`: 目标世界坐标 X（像素）
    /// - `world_y`: 目标世界坐标 Y（像素）
    /// - `map_width_px`: 地图宽度（像素）
    /// - `map_height_px`: 地图高度（像素）
    pub fn follow_target_clamped(&mut self, world_x: f32, world_y: f32, map_width_px: f32, map_height_px: f32) {
        // 计算可视区域的半宽和半高
        let half_width = self.screen_width / (2.0 * self.zoom);
        let half_height = self.screen_height / (2.0 * self.zoom);
        
        // 限制摄像机位置，确保不超出地图边界
        // 摄像机中心不能小于半屏幕（否则会显示地图外）
        // 摄像机中心不能大于 地图尺寸 - 半屏幕
        let min_x = half_width.max(0.0);
        let max_x = (map_width_px - half_width).max(min_x);
        let min_y = half_height.max(0.0);
        let max_y = (map_height_px - half_height).max(min_y);
        
        self.x = world_x.clamp(min_x, max_x);
        self.y = world_y.clamp(min_y, max_y);
        
        tracing::trace!("📷 Camera clamped: target=({:.1}, {:.1}) → actual=({:.1}, {:.1}), bounds=[{:.1}-{:.1}, {:.1}-{:.1}]",
            world_x, world_y, self.x, self.y, min_x, max_x, min_y, max_y);
    }
    
    /// 世界坐标转屏幕坐标（X）
    /// 
    /// # 参数
    /// - `world_x`: 世界坐标 X（像素）
    /// 
    /// # 返回
    /// 屏幕坐标 X（像素）
    pub fn world_to_screen_x(&self, world_x: f32) -> f32 {
        (world_x - self.x) * self.zoom + self.screen_width / 2.0
    }
    
    /// 世界坐标转屏幕坐标（Y）
    /// 
    /// # 参数
    /// - `world_y`: 世界坐标 Y（像素）
    /// 
    /// # 返回
    /// 屏幕坐标 Y（像素）
    pub fn world_to_screen_y(&self, world_y: f32) -> f32 {
        (world_y - self.y) * self.zoom + self.screen_height / 2.0
    }
    
    /// 世界坐标转屏幕坐标
    /// 
    /// # 参数
    /// - `world_x`: 世界坐标 X（像素）
    /// - `world_y`: 世界坐标 Y（像素）
    /// 
    /// # 返回
    /// `(screen_x, screen_y)`: 屏幕坐标（像素）
    pub fn world_to_screen(&self, world_x: f32, world_y: f32) -> (f32, f32) {
        (
            self.world_to_screen_x(world_x),
            self.world_to_screen_y(world_y),
        )
    }
    
    /// 屏幕坐标转世界坐标（X）
    /// 
    /// # 参数
    /// - `screen_x`: 屏幕坐标 X（像素）
    /// 
    /// # 返回
    /// 世界坐标 X（像素）
    pub fn screen_to_world_x(&self, screen_x: f32) -> f32 {
        self.x + (screen_x - self.screen_width / 2.0) / self.zoom
    }
    
    /// 屏幕坐标转世界坐标（Y）
    /// 
    /// # 参数
    /// - `screen_y`: 屏幕坐标 Y（像素）
    /// 
    /// # 返回
    /// 世界坐标 Y（像素）
    pub fn screen_to_world_y(&self, screen_y: f32) -> f32 {
        self.y + (screen_y - self.screen_height / 2.0) / self.zoom
    }
    
    /// 屏幕坐标转世界坐标
    /// 
    /// # 参数
    /// - `screen_x`: 屏幕坐标 X（像素）
    /// - `screen_y`: 屏幕坐标 Y（像素）
    /// 
    /// # 返回
    /// `(world_x, world_y)`: 世界坐标（像素）
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        (
            self.screen_to_world_x(screen_x),
            self.screen_to_world_y(screen_y),
        )
    }
    
    /// 获取可见区域（世界坐标）
    /// 
    /// # 返回
    /// `Rect { x, y, w, h }`: 可见区域的世界坐标和尺寸
    pub fn get_visible_rect(&self) -> Rect {
        let half_width = self.screen_width / (2.0 * self.zoom);
        let half_height = self.screen_height / (2.0 * self.zoom);
        
        Rect::new(
            self.x - half_width,
            self.y - half_height,
            half_width * 2.0,
            half_height * 2.0,
        )
    }
    
    /// 创建变换参数（用于绘制）
    /// 
    /// 将整个场景按摄像机视角变换
    pub fn get_draw_param(&self) -> DrawParam {
        DrawParam::default()
            .dest([self.screen_width / 2.0, self.screen_height / 2.0])
            .offset([self.x, self.y])
            .scale([self.zoom, self.zoom])
    }
    
    /// 设置缩放级别
    /// 
    /// # 参数
    /// - `zoom`: 缩放级别（0.1 ~ 4.0）
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 4.0);
    }
    
    /// 调整缩放（相对）
    /// 
    /// # 参数
    /// - `delta`: 缩放增量（正数放大，负数缩小）
    pub fn zoom_by(&mut self, delta: f32) {
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(0.1, 4.0);
    }
    
    /// 获取屏幕中心的世界坐标
    pub fn get_center_world_pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }
    
    /// 获取屏幕尺寸
    pub fn get_screen_size(&self) -> (f32, f32) {
        (self.screen_width, self.screen_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_camera_world_to_screen() {
        let camera = Camera::new(1024.0, 768.0);
        
        // 摄像机在 (0, 0)，屏幕中心应该对应世界 (0, 0)
        let (sx, sy) = camera.world_to_screen(0.0, 0.0);
        assert_eq!(sx, 512.0); // 1024 / 2
        assert_eq!(sy, 384.0); // 768 / 2
    }
    
    #[test]
    fn test_camera_screen_to_world() {
        let mut camera = Camera::new(1024.0, 768.0);
        camera.follow_target(100.0, 100.0);
        
        // 屏幕中心应该对应摄像机位置
        let (wx, wy) = camera.screen_to_world(512.0, 384.0);
        assert_eq!(wx, 100.0);
        assert_eq!(wy, 100.0);
    }
    
    #[test]
    fn test_camera_zoom() {
        let mut camera = Camera::new(1024.0, 768.0);
        camera.set_zoom(2.0);
        
        // 2倍缩放，世界坐标 (100, 100) 应该在屏幕上更远
        let (sx, sy) = camera.world_to_screen(100.0, 100.0);
        assert_eq!(sx, 512.0 + 100.0 * 2.0);
        assert_eq!(sy, 384.0 + 100.0 * 2.0);
    }
}
