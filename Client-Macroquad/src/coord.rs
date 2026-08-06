/// 传奇2坐标系统 - 统一坐标转换模块
///
/// 🎯 设计理念: 所有坐标转换必须通过此模块,避免重复实现和计算不一致
///
/// 三大坐标系:
/// 1. 地图坐标 (Grid): 格子坐标 (i32, i32)
/// 2. 世界坐标 (World): 像素坐标 (f32, f32) - grid * cell_size
/// 3. 屏幕坐标 (Screen): 渲染坐标 (f32, f32) - 相对玩家 + 视野偏移
///
/// 🔑 关键概念:
/// - **格子左上角**: 用于渲染计算 `grid_to_world()`
/// - **格子中心点**: 用于物理位置 `grid_to_world_center()`
/// - **Floor vs Round**: 必须用 `floor()` 而非 `round()` 避免坐标跳变
///
/// 参考原版: Client/MirScenes/GameScene.cs MapControl
use rand::RngExt;

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
    pub offset_x: i32,     // 视野中心偏移X (格子数)
    pub offset_y: i32,     // 视野中心偏移Y (格子数)
    pub view_range_x: i32, // 视野范围X (格子数)
    pub view_range_y: i32, // 视野范围Y (格子数)
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
///
/// ⚠️ 注意: 这不是 ECS System，而是静态工具函数集合
/// 真正的 ECS Systems (如 PlayerSystem, RenderSystem) 在 `systems/` 目录下
pub struct Coord {
    pub viewport: ViewportConfig,
}

impl Coord {
    /// UI 设计分辨率宽度 (1024×768 固定设计尺寸)
    pub const DESIGN_WIDTH: f32 = 1024.0;

    /// UI 设计分辨率高度
    pub const DESIGN_HEIGHT: f32 = 768.0;

    pub fn new(viewport: ViewportConfig) -> Self {
        Self { viewport }
    }

    /// 地图坐标 → 世界坐标 (格子左上角)
    ///
    /// grid (286, 617) → world (13728.0, 19744.0)
    #[inline]
    pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        (
            grid_x as f32 * CELL_WIDTH as f32,
            grid_y as f32 * CELL_HEIGHT as f32,
        )
    }

    /// 🎯 地图坐标 → 世界坐标 (格子中心点)
    ///
    /// 用于玩家/NPC/怪物的物理位置
    /// grid (5, 10) → world (264.0, 336.0)
    ///   = (5*48+24, 10*32+16)
    #[inline]
    pub fn grid_to_world_center(grid_x: i32, grid_y: i32) -> (f32, f32) {
        (
            grid_x as f32 * CELL_WIDTH as f32 + CELL_WIDTH as f32 / 2.0,
            grid_y as f32 * CELL_HEIGHT as f32 + CELL_HEIGHT as f32 / 2.0,
        )
    }

    /// 世界坐标 → 地图坐标
    ///
    /// world (13728.0, 19744.0) → grid (286, 617)
    /// world (264.0, 336.0) → grid (5, 10)
    #[inline]
    pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        (
            (world_x / CELL_WIDTH as f32).floor() as i32,
            (world_y / CELL_HEIGHT as f32).floor() as i32,
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
        let mut screen_x =
            (obj_grid.0 - player_grid.0 + self.viewport.offset_x) as f32 * CELL_WIDTH as f32;
        let mut screen_y =
            (obj_grid.1 - player_grid.1 + self.viewport.offset_y) as f32 * CELL_HEIGHT as f32;

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
        let grid_y =
            (screen_y / CELL_HEIGHT as f32) as i32 - self.viewport.offset_y + player_grid.1;
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
    coord_system: Coord,
}

impl ObjectRenderer {
    pub fn new(coord_system: Coord) -> Self {
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
    pub fn calculate_draw_y(movement_grid: (i32, i32), current_grid: (i32, i32)) -> i32 {
        movement_grid.1.max(current_grid.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_to_world() {
        let (wx, wy) = Coord::grid_to_world(286, 617);
        assert_eq!(wx, 286.0 * 48.0);
        assert_eq!(wy, 617.0 * 32.0);
    }

    #[test]
    fn test_world_to_grid() {
        let (gx, gy) = Coord::world_to_grid(13728.0, 19744.0);
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
        let coord_sys = Coord::new(viewport);

        // 玩家在 (100, 100)
        let player_world = Coord::grid_to_world(100, 100);

        // 玩家的屏幕坐标应该在视野中心
        let (sx, sy) =
            coord_sys.to_screen_position(player_world, player_world, (0.0, 0.0), (0.0, 0.0), true);

        // 玩家在屏幕坐标 (10*48, 11*32) = (480, 352)
        assert_eq!(sx, 480.0);
        assert_eq!(sy, 352.0);
    }

    #[test]
    fn test_to_screen_position_other_object() {
        let viewport = ViewportConfig::new(1024.0, 768.0);
        let coord_sys = Coord::new(viewport);

        // 玩家在 (100, 100)
        let player_world = Coord::grid_to_world(100, 100);

        // 对象在玩家右侧 2 格 (102, 100)
        let obj_world = Coord::grid_to_world(102, 100);

        let (sx, sy) =
            coord_sys.to_screen_position(obj_world, player_world, (0.0, 0.0), (0.0, 0.0), false);

        // 对象应该在玩家右侧 2*48 = 96 像素
        assert_eq!(sx, 480.0 + 2.0 * 48.0);
        assert_eq!(sy, 352.0);
    }

    #[test]
    fn test_screen_to_grid() {
        let viewport = ViewportConfig::new(1024.0, 768.0);
        let coord_sys = Coord::new(viewport);

        // 玩家在 (100, 100)
        let player_grid = (100, 100);

        // 屏幕中心点击
        let (gx, gy) = coord_sys.screen_to_grid(480.0, 352.0, player_grid);

        // 应该点击到玩家脚下
        assert_eq!(gx, 100);
        assert_eq!(gy, 100);
    }
}

// ============================================================================
// 地图工具 - Map Utilities
// ============================================================================

use crate::components::MapData;

/// 地图工具函数
pub struct MapUtils;

impl MapUtils {
    /// 🎯 找到地图中心的可行走位置（用于角色出生点）
    ///
    /// 从地图中心开始螺旋搜索,直到找到可行走的格子
    pub fn find_center_walkable_position(map_data: &MapData) -> (i32, i32) {
        let center_x = map_data.width / 2;
        let center_y = map_data.height / 2;

        // 螺旋搜索：从中心向外扩散
        for radius in 0i32..50i32 {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    // 只检查当前半径的边界格子
                    if dx.abs() == radius || dy.abs() == radius {
                        let x = center_x + dx;
                        let y = center_y + dy;

                        if Self::is_walkable(map_data, x, y) {
                            return (x, y);
                        }
                    }
                }
            }
        }

        // 如果实在找不到，返回中心
        (center_x, center_y)
    }

    /// 🎯 检查格子是否可行走（没有障碍物）
    pub fn is_walkable(map_data: &MapData, x: i32, y: i32) -> bool {
        // 边界检查
        if x < 0 || x >= map_data.width || y < 0 || y >= map_data.height {
            return false;
        }

        // 传奇地图是按 cells[x][y] 存储的
        let cell = &map_data.cells[x as usize][y as usize];

        // back_image 的第 29 位 (0x20000000) 标记该格子是否有障碍物
        let has_obstacle = (cell.back_image & 0x20000000) != 0;

        // 有障碍物标记 = 不可行走
        !has_obstacle
    }
}

// ============================================================================
// 相机系统 - Camera System
// ============================================================================

/// 相机状态
#[derive(Debug, Clone, Copy)]
pub enum CameraState {
    /// 跟随玩家 (默认)
    Following,
    /// 自由移动
    Free,
    /// 平滑过渡到目标位置
    Transitioning {
        from: (f32, f32),
        to: (f32, f32),
        progress: f32,
        duration: f32,
    },
}

/// 相机系统 - 管理游戏视野
///
/// 功能:
/// - 跟随玩家
/// - 镜头平滑移动
/// - 平滑过渡动画
/// - 缩放 (预留)
/// - 边界限制
pub struct CameraController {
    /// 坐标系统
    coord_system: Coord,

    /// 相机状态
    state: CameraState,

    /// 相机世界坐标 (中心点)
    pub position: (f32, f32),

    /// 镜头震动偏移
    shake_offset: (f32, f32),

    /// 震动剩余时间
    shake_duration: f32,

    /// 震动强度
    shake_intensity: f32,

    /// 缩放级别 (1.0 = 正常, 预留功能)
    pub zoom: f32,

    /// 跟随平滑系数 (0.0-1.0, 值越大越快)
    follow_smoothness: f32,

    /// 地图边界 (用于限制相机移动)
    map_bounds: Option<(i32, i32, i32, i32)>, // (min_x, min_y, max_x, max_y)
}

impl CameraController {
    /// 创建相机
    pub fn new(coord_system: Coord) -> Self {
        Self {
            coord_system,
            state: CameraState::Following,
            position: (0.0, 0.0),
            shake_offset: (0.0, 0.0),
            shake_duration: 0.0,
            shake_intensity: 0.0,
            zoom: 1.0,
            follow_smoothness: 0.15,
            map_bounds: None,
        }
    }

    /// 设置地图边界
    pub fn set_map_bounds(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) {
        self.map_bounds = Some((min_x, min_y, max_x, max_y));
    }

    /// 设置跟随平滑度 (0.0-1.0)
    pub fn set_follow_smoothness(&mut self, smoothness: f32) {
        self.follow_smoothness = smoothness.clamp(0.0, 1.0);
    }

    /// 更新相机 (每帧调用)
    pub fn update(&mut self, delta_time: f32, player_position: (f32, f32)) {
        match self.state {
            CameraState::Following => {
                // 平滑跟随玩家
                self.position.0 += (player_position.0 - self.position.0) * self.follow_smoothness;
                self.position.1 += (player_position.1 - self.position.1) * self.follow_smoothness;
            }
            CameraState::Free => {
                // 自由模式不自动更新
            }
            CameraState::Transitioning {
                from,
                to,
                progress,
                duration,
            } => {
                let new_progress = progress + delta_time / duration;

                if new_progress >= 1.0 {
                    // 过渡完成
                    self.position = to;
                    self.state = CameraState::Following;
                } else {
                    // 平滑插值 (使用 ease-out)
                    let t = 1.0 - (1.0 - new_progress).powi(3);
                    self.position.0 = from.0 + (to.0 - from.0) * t;
                    self.position.1 = from.1 + (to.1 - from.1) * t;

                    self.state = CameraState::Transitioning {
                        from,
                        to,
                        progress: new_progress,
                        duration,
                    };
                }
            }
        }

        // 更新震动
        if self.shake_duration > 0.0 {
            self.shake_duration -= delta_time;

            if self.shake_duration <= 0.0 {
                self.shake_offset = (0.0, 0.0);
            } else {
                // 随机震动方向
                let mut rng = rand::rng();
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let intensity = self.shake_intensity * (self.shake_duration / 0.3).min(1.0);

                self.shake_offset.0 = angle.cos() * intensity;
                self.shake_offset.1 = angle.sin() * intensity;
            }
        }

        // 应用地图边界限制
        if let Some((min_x, min_y, max_x, max_y)) = self.map_bounds {
            let (min_world_x, min_world_y) = Coord::grid_to_world_center(min_x, min_y);
            let (max_world_x, max_world_y) = Coord::grid_to_world_center(max_x, max_y);

            self.position.0 = self.position.0.clamp(min_world_x, max_world_x);
            self.position.1 = self.position.1.clamp(min_world_y, max_world_y);
        }
    }

    /// 立即跳转到指定位置
    pub fn jump_to(&mut self, position: (f32, f32)) {
        self.position = position;
        self.state = CameraState::Following;
    }

    /// 平滑移动到指定位置
    pub fn transition_to(&mut self, target: (f32, f32), duration: f32) {
        self.state = CameraState::Transitioning {
            from: self.position,
            to: target,
            progress: 0.0,
            duration,
        };
    }

    /// 触发镜头震动
    ///
    /// # 参数
    /// - intensity: 震动强度 (像素)
    /// - duration: 震动持续时间 (秒)
    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity;
        self.shake_duration = duration;
    }

    /// 切换到自由模式
    pub fn set_free_mode(&mut self) {
        self.state = CameraState::Free;
    }

    /// 切换回跟随模式
    pub fn set_follow_mode(&mut self) {
        self.state = CameraState::Following;
    }

    /// 获取相机最终位置 (包含震动)
    pub fn get_final_position(&self) -> (f32, f32) {
        (
            self.position.0 + self.shake_offset.0,
            self.position.1 + self.shake_offset.1,
        )
    }

    /// 获取相机当前格子坐标
    pub fn get_grid_position(&self) -> (i32, i32) {
        let final_pos = self.get_final_position();
        Coord::world_to_grid(final_pos.0, final_pos.1)
    }

    /// 世界坐标 → 屏幕坐标 (通过相机)
    pub fn world_to_screen(&self, world_pos: (f32, f32)) -> (f32, f32) {
        let camera_pos = self.get_final_position();

        self.coord_system
            .to_screen_position(world_pos, camera_pos, (0.0, 0.0), (0.0, 0.0), false)
    }

    /// 屏幕坐标 → 世界坐标 (通过相机)
    pub fn screen_to_world(&self, screen_pos: (f32, f32)) -> (f32, f32) {
        let camera_grid = self.get_grid_position();
        let grid = self
            .coord_system
            .screen_to_grid(screen_pos.0, screen_pos.1, camera_grid);
        Coord::grid_to_world_center(grid.0, grid.1)
    }

    /// 检查世界坐标是否在相机视野内
    pub fn is_visible(&self, world_pos: (f32, f32)) -> bool {
        let camera_grid = self.get_grid_position();
        let obj_grid = Coord::world_to_grid(world_pos.0, world_pos.1);
        self.coord_system.is_in_viewport(obj_grid, camera_grid)
    }
}
