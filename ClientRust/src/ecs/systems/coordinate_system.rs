// ============================================================================
// 坐标系统 - 处理各种坐标转换
// ============================================================================
//
// 功能：
// - 窗口坐标 → UI 设计坐标 (1024×768)
// - 窗口坐标 → 世界坐标
// - 世界坐标 → 格子坐标
// - 格子坐标 → 世界坐标
//
// ============================================================================

use ggez::Context;

/// 坐标系统 - 处理各种坐标转换
pub struct CoordinateSystem;

impl CoordinateSystem {
    /// UI 设计分辨率宽度
    pub const DESIGN_WIDTH: f32 = 1024.0;
    
    /// UI 设计分辨率高度
    pub const DESIGN_HEIGHT: f32 = 768.0;
    
    /// 将窗口逻辑坐标转换为 UI 设计坐标系（1024×768）
    /// 
    /// # 参数
    /// - `ctx`: ggez 上下文（用于获取窗口尺寸）
    /// - `window_x, window_y`: 窗口逻辑坐标
    /// 
    /// # 返回
    /// - `(design_x, design_y)`: UI 设计坐标 (1024×768)
    /// 
    /// # 说明
    /// ggez 会自动处理 DPI 缩放，我们只需要使用 drawable_size()
    /// 计算时会考虑 4:3 宽高比，添加黑边（letterbox）
    pub fn window_to_ui_coords(ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let (window_width, window_height) = ctx.gfx.drawable_size();
        
        // 计算 4:3 视口
        let aspect_ratio = 4.0 / 3.0;
        let current_ratio = window_width / window_height;
        
        let (viewport_width, viewport_height) = if current_ratio > aspect_ratio {
            // 窗口太宽，左右加黑边
            (window_height * aspect_ratio, window_height)
        } else {
            // 窗口太高，上下加黑边
            (window_width, window_width / aspect_ratio)
        };
        
        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;
        
        // 转换：窗口坐标 -> 视口坐标 -> 设计坐标
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;
        
        let design_x = (viewport_x / viewport_width) * Self::DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * Self::DESIGN_HEIGHT;
        
        (design_x, design_y)
    }
    
    /// 将窗口坐标转换为世界坐标
    /// 
    /// # 参数
    /// - `window_x, window_y`: 窗口逻辑坐标
    /// - `camera_x, camera_y`: 相机世界坐标
    /// - `screen_width, screen_height`: 屏幕尺寸
    /// 
    /// # 返回
    /// - `(world_x, world_y)`: 世界坐标
    pub fn window_to_world_coords(
        window_x: f32,
        window_y: f32,
        camera_x: f32,
        camera_y: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (f32, f32) {
        let world_x = camera_x + window_x - screen_width / 2.0;
        let world_y = camera_y + window_y - screen_height / 2.0;
        (world_x, world_y)
    }
    
    /// 将世界坐标转换为格子坐标
    /// 
    /// # 参数
    /// - `world_x, world_y`: 世界坐标
    /// - `cell_width, cell_height`: 格子尺寸（默认 48×32）
    /// 
    /// # 返回
    /// - `(grid_x, grid_y)`: 格子坐标
    pub fn world_to_grid_coords(
        world_x: f32,
        world_y: f32,
        cell_width: f32,
        cell_height: f32,
    ) -> (i32, i32) {
        let grid_x = (world_x / cell_width).floor() as i32;
        let grid_y = (world_y / cell_height).floor() as i32;
        (grid_x, grid_y)
    }
    
    /// 将格子坐标转换为世界坐标（格子中心点）
    /// 
    /// # 参数
    /// - `grid_x, grid_y`: 格子坐标
    /// - `cell_width, cell_height`: 格子尺寸（默认 48×32）
    /// 
    /// # 返回
    /// - `(world_x, world_y)`: 世界坐标（格子中心）
    pub fn grid_to_world_coords(
        grid_x: i32,
        grid_y: i32,
        cell_width: f32,
        cell_height: f32,
    ) -> (f32, f32) {
        let world_x = grid_x as f32 * cell_width + cell_width / 2.0;
        let world_y = grid_y as f32 * cell_height + cell_height / 2.0;
        (world_x, world_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_world_grid_conversion() {
        let cell_width = 48.0;
        let cell_height = 32.0;
        
        // 测试世界坐标转格子坐标
        let (grid_x, grid_y) = CoordinateSystem::world_to_grid_coords(100.0, 64.0, cell_width, cell_height);
        assert_eq!(grid_x, 2);
        assert_eq!(grid_y, 2);
        
        // 测试格子坐标转世界坐标
        let (world_x, world_y) = CoordinateSystem::grid_to_world_coords(2, 2, cell_width, cell_height);
        assert_eq!(world_x, 120.0);  // 2 * 48 + 24
        assert_eq!(world_y, 80.0);   // 2 * 32 + 16
    }
}
