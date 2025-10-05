// CompassDialog - 指南针对话框
// Mirrors Client/MirScenes/Dialogs/CompassDialog.cs (52 lines)

/// 指南针对话框 - 显示目标方向
#[derive(Debug, Clone)]
pub struct CompassDialog {
    /// 是否可见
    pub visible: bool,
    
    /// 目标位置 (地图坐标)
    pub destination: Option<(i32, i32)>,
    
    /// 显示位置 (屏幕坐标)
    pub location: (i32, i32),
    
    /// 当前指向角度 (弧度)
    pub angle: f32,
}

impl CompassDialog {
    /// 创建新的指南针对话框
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self {
            visible: false,
            destination: None,
            location: (screen_width / 2 - 25, screen_height / 2 - 120),
            angle: 0.0,
        }
    }
    
    /// 清除目标点
    pub fn clear_point(&mut self) {
        self.destination = None;
        self.visible = false;
    }
    
    /// 设置目标点
    pub fn set_point(&mut self, x: i32, y: i32) {
        self.destination = Some((x, y));
        self.visible = true;
    }
    
    /// 更新指南针（计算角度）
    pub fn process(&mut self, current_location: (i32, i32)) {
        if let Some((dest_x, dest_y)) = self.destination {
            // 检查是否已到达目标
            if dest_x == current_location.0 && dest_y == current_location.1 {
                self.clear_point();
                return;
            }
            
            self.visible = true;
            
            // 计算方向角度
            let x_diff = (current_location.0 - dest_x) as f32;
            let y_diff = (current_location.1 - dest_y) as f32;
            
            // 计算角度（弧度）
            self.angle = y_diff.atan2(x_diff);
            
            // C# 代码中使用的图像索引计算
            // _image.Index = (int)Math.Round(Math.Atan2(yDiff, xDiff) * (32f / Math.PI)) + 40;
            // 这里保存角度，渲染时再计算图像索引
        } else {
            self.visible = false;
        }
    }
    
    /// 获取指南针图像索引（用于渲染）
    pub fn get_image_index(&self) -> i32 {
        if !self.visible || self.destination.is_none() {
            return 0;
        }
        
        // 根据角度计算图像索引
        // C# 公式: (int)Math.Round(angle * (32f / Math.PI)) + 40
        let index = (self.angle * (32.0 / std::f32::consts::PI)).round() as i32 + 40;
        index
    }
    
    /// 是否有目标
    pub fn has_destination(&self) -> bool {
        self.destination.is_some()
    }
}

impl Default for CompassDialog {
    fn default() -> Self {
        Self::new(800, 600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compass_creation() {
        let compass = CompassDialog::new(800, 600);
        assert!(!compass.visible);
        assert!(compass.destination.is_none());
    }
    
    #[test]
    fn test_set_destination() {
        let mut compass = CompassDialog::new(800, 600);
        
        compass.set_point(100, 200);
        assert!(compass.visible);
        assert_eq!(compass.destination, Some((100, 200)));
    }
    
    #[test]
    fn test_clear_destination() {
        let mut compass = CompassDialog::new(800, 600);
        
        compass.set_point(100, 200);
        compass.clear_point();
        
        assert!(!compass.visible);
        assert!(compass.destination.is_none());
    }
    
    #[test]
    fn test_arrived_at_destination() {
        let mut compass = CompassDialog::new(800, 600);
        
        compass.set_point(100, 100);
        compass.process((100, 100)); // 当前位置等于目标
        
        assert!(!compass.visible);
        assert!(compass.destination.is_none());
    }
    
    #[test]
    fn test_angle_calculation() {
        let mut compass = CompassDialog::new(800, 600);
        
        // 目标在东边
        compass.set_point(150, 100);
        compass.process((100, 100));
        
        assert!(compass.visible);
        assert!(compass.angle.abs() < 0.01); // 应该接近0（正东）
        
        // 目标在南边
        compass.set_point(100, 150);
        compass.process((100, 100));
        
        assert!(compass.angle.abs() - std::f32::consts::FRAC_PI_2 < 0.01); // 应该接近 π/2（正南）
    }
    
    #[test]
    fn test_image_index_calculation() {
        let mut compass = CompassDialog::new(800, 600);
        
        compass.set_point(150, 100);
        compass.process((100, 100));
        
        let index = compass.get_image_index();
        assert!(index >= 0 && index < 100); // 合理范围
    }
}
