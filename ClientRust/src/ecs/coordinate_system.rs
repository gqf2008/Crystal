/// 传奇2坐标系统 - 统一坐标转换模块
/// 
/// 三大坐标系:
/// 1. 地图坐标 (Grid): 格子坐标 (i32, i32)
/// 2. 世界坐标 (World): 像素坐标 (f32, f32) - grid * cell_size
/// 3. 屏幕坐标 (Screen): 渲染坐标 (f32, f32) - 相对玩家 + 视野偏移
/// 
/// 参考原版: Client/MirScenes/GameScene.cs MapControl

use hecs::World;

/// 地图格子宽度 (像素)
pub const CELL_WIDTH: i32 = 48;

/// 地图格子高度 (像素) - 等距视角
pub const CELL_HEIGHT: i32 = 32;

/// 视野配置
/// 
/// 对应原版:
/// - OffSetX = ScreenWidth / 2 / CellWidth
/// - OffSetY = ScreenHeight / 2 / CellHeight - 1
#[derive(Debug, Clone, Copy)]
pub struct ViewportConfig {
    pub screen_width: f32,
    pub screen_height: f32,
    pub offset_x: i32,  // 视野中心偏移X (格子数)
    pub offset_y: i32,  // 视野中心偏移Y (格子数)
    pub view_range_x: i32,  // 视野范围X (格子数)
    pub view_range_y: i32,  // 视野范围Y (格子数)
}

impl ViewportConfig {
    /// 创建视野配置
    /// 
    /// 示例 (1024x768 窗口):
    /// - offset_x = 1024/2/48 = 10
    /// - offset_y = 768/2/32 - 1 = 11
    /// - view_range_x = 10 + 6 = 16
    /// - view_range_y = 11 + 6 = 17
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let offset_x = (screen_width / 2.0 / CELL_WIDTH as f32) as i32;
        let offset_y = (screen_height / 2.0 / CELL_HEIGHT as f32) as i32 - 1;
        
        Self {
            screen_width,
            screen_height,
            offset_x,
            offset_y,
            view_range_x: offset_x + 6,
            view_range_y: offset_y + 6,
        }
    }
}

/// 坐标转换工具
pub struct CoordinateSystem {
    pub viewport: ViewportConfig,
}

impl CoordinateSystem {
    pub fn new(viewport: ViewportConfig) -> Self {
        Self { viewport }
    }
    
    /// 地图坐标 → 世界坐标
    /// 
    /// grid (286, 617) → world (13728.0, 19744.0)
    #[inline]
    pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        (
            grid_x as f32 * CELL_WIDTH as f32,
            grid_y as f32 * CELL_HEIGHT as f32
        )
    }
    
    /// 世界坐标 → 地图坐标
    /// 
    /// world (13728.0, 19744.0) → grid (286, 617)
    #[inline]
    pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        (
            (world_x / CELL_WIDTH as f32).floor() as i32,
            (world_y / CELL_HEIGHT as f32).floor() as i32
        )
    }
    
    /// 计算对象的屏幕坐标 (DrawLocation)
    /// 
    /// 对应原版 PlayerObject.cs:971
    /// ```csharp
    /// DrawLocation = new Point(
    ///     (Movement.X - User.Movement.X + OffSetX) * CellWidth,
    ///     (Movement.Y - User.Movement.Y + OffSetY) * CellHeight
    /// );
    /// if (this != User) {
    ///     DrawLocation.Offset(User.OffSetMove);
    ///     DrawLocation.Offset(-OffSetMove.X, -OffSetMove.Y);
    /// }
    /// ```
    /// 
    /// # 参数
    /// - `obj_world`: 对象世界坐标 (像素)
    /// - `player_world`: 玩家世界坐标 (像素)
    /// - `player_pixel_offset`: 玩家像素偏移 (OffSetMove) - 移动中的亚像素偏移
    /// - `obj_pixel_offset`: 对象像素偏移
    /// - `is_player`: 是否是玩家本身
    /// 
    /// # 返回
    /// 屏幕坐标 (像素)
    pub fn to_screen_position(
        &self,
        obj_world: (f32, f32),
        player_world: (f32, f32),
        player_pixel_offset: (f32, f32),
        obj_pixel_offset: (f32, f32),
        is_player: bool,
    ) -> (f32, f32) {
        // 转换为格子坐标 (Movement)
        let obj_grid = Self::world_to_grid(obj_world.0, obj_world.1);
        let player_grid = Self::world_to_grid(player_world.0, player_world.1);
        
        // 计算基础屏幕坐标: (Movement.X - User.Movement.X + OffSetX) * CellWidth
        let mut screen_x = (obj_grid.0 - player_grid.0 + self.viewport.offset_x) as f32 * CELL_WIDTH as f32;
        let mut screen_y = (obj_grid.1 - player_grid.1 + self.viewport.offset_y) as f32 * CELL_HEIGHT as f32;
        
        // 非玩家对象需要修正偏移
        if !is_player {
            // DrawLocation.Offset(User.OffSetMove)
            screen_x += player_pixel_offset.0;
            screen_y += player_pixel_offset.1;
            
            // DrawLocation.Offset(-OffSetMove.X, -OffSetMove.Y)
            screen_x -= obj_pixel_offset.0;
            screen_y -= obj_pixel_offset.1;
        }
        
        (screen_x, screen_y)
    }
    
    /// 屏幕坐标 → 地图坐标 (鼠标点击)
    /// 
    /// 对应原版 GameScene.cs MapControl.MapLocation:
    /// ```csharp
    /// mapX = (screenX / CellWidth) - OffSetX + User.CurrentLocation.X
    /// mapY = (screenY / CellHeight) - OffSetY + User.CurrentLocation.Y
    /// ```
    pub fn screen_to_grid(
        &self,
        screen_x: f32,
        screen_y: f32,
        player_grid: (i32, i32),
    ) -> (i32, i32) {
        let grid_x = (screen_x / CELL_WIDTH as f32) as i32 - self.viewport.offset_x + player_grid.0;
        let grid_y = (screen_y / CELL_HEIGHT as f32) as i32 - self.viewport.offset_y + player_grid.1;
        (grid_x, grid_y)
    }
    
    /// 检查格子是否在视野范围内
    /// 
    /// 用于渲染优化：只渲染可见区域的对象
    pub fn is_in_viewport(&self, grid: (i32, i32), player_grid: (i32, i32)) -> bool {
        let dx = (grid.0 - player_grid.0).abs();
        let dy = (grid.1 - player_grid.1).abs();
        
        dx <= self.viewport.view_range_x && dy <= self.viewport.view_range_y
    }
}

/// 地图对象渲染位置计算器
/// 
/// 封装了完整的坐标转换逻辑，对应原版 MapObject 的坐标计算
pub struct ObjectRenderer {
    coord_system: CoordinateSystem,
}

impl ObjectRenderer {
    pub fn new(coord_system: CoordinateSystem) -> Self {
        Self { coord_system }
    }
    
    /// 计算对象的 DrawLocation
    /// 
    /// 返回: (draw_x, draw_y) - 对象脚底中心点的屏幕坐标
    pub fn calculate_draw_location(
        &self,
        obj_world: (f32, f32),
        player_world: (f32, f32),
        player_pixel_offset: (f32, f32),
        obj_pixel_offset: (f32, f32),
        is_player: bool,
    ) -> (f32, f32) {
        self.coord_system.to_screen_position(
            obj_world,
            player_world,
            player_pixel_offset,
            obj_pixel_offset,
            is_player,
        )
    }
    
    /// 计算对象的 FinalDrawLocation (加上纹理偏移)
    /// 
    /// 对应原版: FinalDrawLocation = DrawLocation.Add(BodyLibrary.GetOffSet(DrawFrame))
    /// 
    /// 返回: (final_x, final_y) - 纹理左上角的屏幕坐标
    pub fn calculate_final_draw_location(
        &self,
        draw_location: (f32, f32),
        texture_offset: (i32, i32),
    ) -> (f32, f32) {
        (
            draw_location.0 + texture_offset.0 as f32,
            draw_location.1 + texture_offset.1 as f32,
        )
    }
    
    /// 计算对象的 DisplayRectangle
    /// 
    /// 对应原版: DisplayRectangle = new Rectangle(DrawLocation, BodyLibrary.GetTrueSize(DrawFrame))
    /// 
    /// 返回: (x, y, width, height) - 屏幕空间矩形
    pub fn calculate_display_rect(
        &self,
        draw_location: (f32, f32),
        texture_size: (u32, u32),
    ) -> (f32, f32, f32, f32) {
        (
            draw_location.0,
            draw_location.1,
            texture_size.0 as f32,
            texture_size.1 as f32,
        )
    }
    
    /// 计算对象的 DrawY (用于深度排序)
    /// 
    /// 对应原版: DrawY = Movement.Y > CurrentLocation.Y ? Movement.Y : CurrentLocation.Y
    pub fn calculate_draw_y(
        movement_grid: (i32, i32),
        current_grid: (i32, i32),
    ) -> i32 {
        movement_grid.1.max(current_grid.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_to_world() {
        let (wx, wy) = CoordinateSystem::grid_to_world(286, 617);
        assert_eq!(wx, 286.0 * 48.0);
        assert_eq!(wy, 617.0 * 32.0);
    }

    #[test]
    fn test_world_to_grid() {
        let (gx, gy) = CoordinateSystem::world_to_grid(13728.0, 19744.0);
        assert_eq!(gx, 286);
        assert_eq!(gy, 617);
    }

    #[test]
    fn test_viewport_config() {
        let viewport = ViewportConfig::new(1024.0, 768.0);
        assert_eq!(viewport.offset_x, 10);
        assert_eq!(viewport.offset_y, 11);
        assert_eq!(viewport.view_range_x, 16);
        assert_eq!(viewport.view_range_y, 17);
    }

    #[test]
    fn test_to_screen_position_player() {
        let viewport = ViewportConfig::new(1024.0, 768.0);
        let coord_sys = CoordinateSystem::new(viewport);
        
        // 玩家在 (100, 100)
        let player_world = CoordinateSystem::grid_to_world(100, 100);
        
        // 玩家的屏幕坐标应该在视野中心
        let (sx, sy) = coord_sys.to_screen_position(
            player_world,
            player_world,
            (0.0, 0.0),
            (0.0, 0.0),
            true,
        );
        
        // 玩家在屏幕坐标 (10*48, 11*32) = (480, 352)
        assert_eq!(sx, 480.0);
        assert_eq!(sy, 352.0);
    }

    #[test]
    fn test_to_screen_position_other_object() {
        let viewport = ViewportConfig::new(1024.0, 768.0);
        let coord_sys = CoordinateSystem::new(viewport);
        
        // 玩家在 (100, 100)
        let player_world = CoordinateSystem::grid_to_world(100, 100);
        
        // 对象在玩家右侧 2 格 (102, 100)
        let obj_world = CoordinateSystem::grid_to_world(102, 100);
        
        let (sx, sy) = coord_sys.to_screen_position(
            obj_world,
            player_world,
            (0.0, 0.0),
            (0.0, 0.0),
            false,
        );
        
        // 对象应该在玩家右侧 2*48 = 96 像素
        assert_eq!(sx, 480.0 + 2.0 * 48.0);
        assert_eq!(sy, 352.0);
    }

    #[test]
    fn test_screen_to_grid() {
        let viewport = ViewportConfig::new(1024.0, 768.0);
        let coord_sys = CoordinateSystem::new(viewport);
        
        // 玩家在 (100, 100)
        let player_grid = (100, 100);
        
        // 屏幕中心点击
        let (gx, gy) = coord_sys.screen_to_grid(480.0, 352.0, player_grid);
        
        // 应该点击到玩家脚下
        assert_eq!(gx, 100);
        assert_eq!(gy, 11); // 注意: 屏幕坐标 352 对应格子 11，不是 100
    }
}
