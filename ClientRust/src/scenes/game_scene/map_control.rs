// MapControl - Map rendering and interaction control
// Mirrors Client/MirScenes/GameScene.cs::MapControl (lines 10062-12294)

use ggez::{GameResult, Context};
use ggez::graphics::Canvas;
use mir2_shared::enums::{LightSetting, WeatherSetting};
use crate::objects::{MapReader, CellInfo}; // 使用 objects::map_code 中的定义
use crate::graphics::get_map_library;

/// Door state
#[derive(Debug, Clone)]
pub struct Door {
    pub index: usize,
    pub location: (i32, i32),
    pub opened: bool,
    pub image_index: i32,
}

/// Map control - handles map rendering, pathfinding, and interaction
#[derive(Debug)]
pub struct MapControl {
    // Map dimensions
    pub width: i32,
    pub height: i32,
    
    // Map metadata
    pub index: i32,
    pub filename: String,
    pub title: String,
    pub minimap: u16,
    pub bigmap: u16,
    pub music: u16,
    pub set_music: u16,
    
    // Lighting and weather
    pub lights: LightSetting,
    pub weather: WeatherSetting,
    pub map_dark_light: u8,
    pub lightning: bool,
    pub fire: bool,
    pub lightning_time: i64,
    pub fire_time: i64,
    
    // Map cells (2D grid)
    pub cells: Vec<Vec<CellInfo>>,
    
    // Doors
    pub doors: Vec<Door>,
    
    // View settings
    pub offset_x: i32,
    pub offset_y: i32,
    pub view_range_x: i32,
    pub view_range_y: i32,
    
    // Pathfinding
    pub auto_path: bool,
    pub auto_run: bool,
    pub auto_hit: bool,
    pub awakening_action: bool,
    
    // Input state
    pub mouse_location: (i32, i32),
    pub next_action: i64,
    pub input_delay: i64,
    pub output_delay: i64,
    
    // Animation
    pub animation_count: i32,
    
    // Rendering cache
    floor_valid: bool,           // C#: FloorValid
    // floor_texture: Option<Image>, // C#: FloorTexture (cached floor rendering)
}

/// User position for rendering (temporary until we have proper User object)
#[derive(Debug, Clone, Copy)]
pub struct UserPosition {
    pub x: i32,
    pub y: i32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl MapControl {
    /// Cell dimensions in pixels
    pub const CELL_WIDTH: i32 = 48;
    pub const CELL_HEIGHT: i32 = 32;
    
    /// Create new map control from MapReader
    pub fn from_map_reader(reader: MapReader) -> Self {
        Self {
            width: reader.width,
            height: reader.height,
            index: 0,
            filename: reader.file_name.clone(),
            title: String::new(),
            minimap: 0,
            bigmap: 0,
            music: 0,
            set_music: 0,
            lights: LightSetting::Normal,
            weather: WeatherSetting::NONE,
            map_dark_light: 0,
            lightning: false,
            fire: false,
            lightning_time: 0,
            fire_time: 0,
            cells: reader.map_cells,
            doors: Vec::new(),
            offset_x: 0,
            offset_y: 0,
            view_range_x: 20,
            view_range_y: 15,
            auto_path: false,
            auto_run: false,
            auto_hit: false,
            awakening_action: false,
            mouse_location: (0, 0),
            next_action: 0,
            input_delay: 0,
            output_delay: 0,
            animation_count: 0,
            floor_valid: false,
        }
    }
    
    /// Create new map control (legacy method for compatibility)
    pub fn new(width: i32, height: i32) -> Self {
        let cells = vec![vec![CellInfo::new(); height as usize]; width as usize];
        
        Self {
            width,
            height,
            index: 0,
            filename: String::new(),
            title: String::new(),
            minimap: 0,
            bigmap: 0,
            music: 0,
            set_music: 0,
            lights: LightSetting::Normal,
            weather: WeatherSetting::NONE,
            map_dark_light: 0,
            lightning: false,
            fire: false,
            lightning_time: 0,
            fire_time: 0,
            cells,
            doors: Vec::new(),
            offset_x: 0,
            offset_y: 0,
            view_range_x: 20,
            view_range_y: 15,
            auto_path: false,
            auto_run: false,
            auto_hit: false,
            awakening_action: false,
            mouse_location: (0, 0),
            next_action: 0,
            input_delay: 0,
            output_delay: 0,
            animation_count: 0,
            floor_valid: false,
        }
    }
    
    /// Get map location from screen coordinates
    pub fn screen_to_map(&self, screen_x: i32, screen_y: i32) -> (i32, i32) {
        let map_x = (screen_x - self.offset_x) / Self::CELL_WIDTH;
        let map_y = (screen_y - self.offset_y) / Self::CELL_HEIGHT;
        (map_x, map_y)
    }
    
    /// Get screen coordinates from map location
    pub fn map_to_screen(&self, map_x: i32, map_y: i32) -> (i32, i32) {
        let screen_x = map_x * Self::CELL_WIDTH + self.offset_x;
        let screen_y = map_y * Self::CELL_HEIGHT + self.offset_y;
        (screen_x, screen_y)
    }
    
    /// Check if location is valid
    pub fn is_valid_location(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }
    
    /// Check if location is walkable
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if !self.is_valid_location(x, y) {
            return false;
        }
        self.cells[x as usize][y as usize].is_walkable()
    }
    
    /// Get cell at location
    pub fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if !self.is_valid_location(x, y) {
            return None;
        }
        Some(&self.cells[x as usize][y as usize])
    }
    
    /// Get mutable cell at location
    pub fn get_cell_mut(&mut self, x: i32, y: i32) -> Option<&mut CellInfo> {
        if !self.is_valid_location(x, y) {
            return None;
        }
        Some(&mut self.cells[x as usize][y as usize])
    }
    
    /// Update view offset to center on location
    pub fn center_on(&mut self, x: i32, y: i32, screen_width: i32, screen_height: i32) {
        let center_x = screen_width / 2;
        let center_y = screen_height / 2;
        
        self.offset_x = center_x - (x * Self::CELL_WIDTH);
        self.offset_y = center_y - (y * Self::CELL_HEIGHT);
    }
    
    /// Draw map
    /// 
    /// 对应 C# DrawControl (line 10420-10440) 和 CreateTexture (line 10333-10418)
    /// 
    /// 渲染流程:
    /// 1. 绘制静态地表(Floor) - 缓存优化
    /// 2. 绘制远景背景(Background)
    /// 3. 绘制动态层和对象(Objects)
    /// 4. 绘制天气效果(Weather)
    /// 5. 应用光照(Lighting)
    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, user_pos: &UserPosition) -> GameResult<()> {
        // 更新动画计数器
        self.animation_count = (self.animation_count + 1) % 1000;
        
        // 1) 绘制静态地表
        if !self.floor_valid {
            self.draw_floor(ctx, canvas, user_pos)?;
        }
        
        // 2) 绘制远景背景
        self.draw_background(ctx, canvas)?;
        
        // 3) 绘制动态层和对象
        self.draw_objects(ctx, canvas, user_pos)?;
        
        // TODO: 4) 绘制天气效果
        // TODO: 5) 应用光照遮罩
        
        Ok(())
    }
    
    /// 绘制静态地表到缓存
    /// 
    /// 对应 C# DrawFloor (line 10442-10544)
    /// 
    /// 优化策略:
    /// - 仅渲染偶数坐标的格子(y % 2 == 0, x % 2 == 0)
    /// - 渲染 Back 层(不可见的基础层)
    /// - 渲染 Middle 层的静态部分
    /// - 渲染 Front 层的静态部分
    fn draw_floor(&mut self, ctx: &mut Context, canvas: &mut Canvas, user_pos: &UserPosition) -> GameResult<()> {
        // 计算视野范围
        let start_y = (user_pos.y - self.view_range_y).max(0);
        let end_y = (user_pos.y + self.view_range_y).min(self.height - 1);
        let start_x = (user_pos.x - self.view_range_x).max(0);
        let end_x = (user_pos.x + self.view_range_x).min(self.width - 1);
        
        // 遍历视野内的格子
        for y in start_y..=end_y {
            // C#: if (y <= 0 || y % 2 == 1) continue;
            if y <= 0 || y % 2 == 1 {
                continue;
            }
            
            // 计算屏幕Y坐标
            let draw_y = ((y - user_pos.y + self.offset_y) * Self::CELL_HEIGHT + user_pos.offset_y) as f32;
            
            for x in start_x..=end_x {
                // C#: if (x <= 0 || x % 2 == 1) continue;
                if x <= 0 || x % 2 == 1 {
                    continue;
                }
                
                // 计算屏幕X坐标
                let draw_x = ((x - user_pos.x + self.offset_x) * Self::CELL_WIDTH - self.offset_x + user_pos.offset_x) as f32;
                
                // 获取格子信息
                if let Some(cell) = self.get_cell(x, y) {
                    // 绘制 Back 层
                    if cell.back_image > 0 && cell.back_index >= 0 {
                        self.draw_tile(ctx, canvas, cell.back_index as i32, cell.back_image as usize - 1, draw_x, draw_y)?;
                    }
                    
                    // TODO: 绘制 Middle 层静态部分
                    // TODO: 绘制 Front 层静态部分
                }
            }
        }
        
        self.floor_valid = true;
        Ok(())
    }
    
    /// 绘制远景背景
    /// 
    /// 对应 C# DrawBackground (line 10546-10566)
    /// 
    /// 根据地图文件名选择背景图:
    /// - ID1/ID2 → 山脉背景(index 10)
    /// - ID3_013 → 沙漠背景(index 22)
    /// - ID3_015 → 长城背景(index 23)
    /// - ID3_023/025 → 村庄入口(index 21)
    fn draw_background(&mut self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult<()> {
        let background_index = if self.filename.starts_with("ID1") || self.filename.starts_with("ID2") {
            Some(10) // mountains
        } else if self.filename.starts_with("ID3_013") {
            Some(22) // desert
        } else if self.filename.starts_with("ID3_015") {
            Some(23) // greatwall
        } else if self.filename.starts_with("ID3_023") || self.filename.starts_with("ID3_025") {
            Some(21) // village entrance
        } else {
            None
        };
        
        if let Some(_idx) = background_index {
            // TODO: 从 Libraries.Background 加载并绘制背景图
            // Libraries.Background.Draw(idx, 0, 0);
        }
        
        Ok(())
    }
    
    /// 绘制动态层和对象
    /// 
    /// 对应 C# DrawObjects (line 10568-10803)
    /// 
    /// 渲染顺序(9步):
    /// 1. 背景特效 (Effects DrawBehind)
    /// 2. 尸体对象 (DeadObjects)
    /// 3. Shanda 瓦片动画 (TileAnimation)
    /// 4. Middle 动态层 (MiddleLayer with animation)
    /// 5. Front 动态层 (FrontLayer with animation/doors)
    /// 6. 对象本体 (M2CellInfo[x,y].DrawObjects)
    /// 7. User 高亮
    /// 8. 前景特效 (Effects !DrawBehind)
    /// 9. 名字/血条/聊天/伤害文字
    fn draw_objects(&mut self, ctx: &mut Context, canvas: &mut Canvas, user_pos: &UserPosition) -> GameResult<()> {
        // TODO: 1) 绘制背景特效
        
        // 2) 绘制尸体对象
        let start_y = (user_pos.y - self.view_range_y).max(0);
        let end_y = (user_pos.y + self.view_range_y + 25).min(self.height - 1);
        let start_x = (user_pos.x - self.view_range_x).max(0);
        let end_x = (user_pos.x + self.view_range_x).min(self.width - 1);
        
        // TODO: Draw dead objects
        
        // 3-6) 按行遍历绘制瓦片和对象
        for y in start_y..=end_y {
            if y <= 0 {
                continue;
            }
            
            let draw_y = ((y - user_pos.y + self.offset_y + 1) * Self::CELL_HEIGHT + user_pos.offset_y) as f32;
            
            for x in start_x..=end_x {
                if x < 0 {
                    continue;
                }
                
                let draw_x = ((x - user_pos.x + self.offset_x) * Self::CELL_WIDTH - self.offset_x + user_pos.offset_x) as f32;
                
                if let Some(cell) = self.get_cell(x, y) {
                    // 3) Shanda 瓦片动画
                    if cell.tile_animation_image > 0 && cell.tile_animation_frames > 0 {
                        let mut index = cell.tile_animation_image as i32 - 1;
                        let animation_offset = cell.tile_animation_offset ^ 0x2000;
                        index += (animation_offset as i32) * (self.animation_count % cell.tile_animation_frames as i32);
                        
                        // Libraries.MapLibs[190].DrawUp(index, drawX, drawY);
                        self.draw_tile(ctx, canvas, 190, index as usize, draw_x, draw_y)?;
                    }
                    
                    // 4) Middle 动态层
                    if cell.middle_index >= 0 && cell.middle_image > 0 {
                        let mut index = cell.middle_image as i32 - 1;
                        let animation = cell.middle_animation_frame;
                        
                        if animation > 0 && animation < 255 {
                            let animation_tick = cell.middle_animation_tick;
                            let anim_frames = animation & 0x0f;
                            if anim_frames > 0 {
                                index += (self.animation_count % (anim_frames as i32 + (anim_frames as i32 * animation_tick as i32))) 
                                    / (1 + animation_tick as i32);
                            }
                        }
                        
                        self.draw_tile(ctx, canvas, cell.middle_index as i32, index as usize, draw_x, draw_y)?;
                    }
                    
                    // 5) Front 动态层
                    let front_image = cell.front_image & 0x7FFF;
                    if front_image > 0 && cell.front_index >= 0 {
                        let mut index = front_image as i32 - 1;
                        let animation = cell.front_animation_frame & 0x7F;
                        
                        if animation > 0 {
                            let animation_tick = cell.front_animation_tick;
                            index += (self.animation_count % (animation as i32 + (animation as i32 * animation_tick as i32))) 
                                / (1 + animation_tick as i32);
                        }
                        
                        // 处理门动画
                        if cell.door_index > 0 {
                            if let Some(door) = self.doors.iter().find(|d| d.index == cell.door_index as usize) {
                                if door.opened {
                                    index += (door.image_index + 1) * cell.door_offset as i32;
                                }
                            }
                        }
                        
                        self.draw_tile(ctx, canvas, cell.front_index as i32, index as usize, draw_x, draw_y)?;
                    }
                    
                    // TODO: 6) 绘制对象本体 (M2CellInfo[x,y].DrawObjects)
                }
            }
        }
        
        // TODO: 7) User 高亮
        // TODO: 8) 前景特效
        // TODO: 9) 名字/血条/聊天/伤害文字
        
        Ok(())
    }
    
    /// 从 MapLibs 绘制瓦片 (使用纹理缓存)
    /// 
    /// 对应 C#: Libraries.MapLibs[index].Draw(...)
    /// 
    /// **性能优化**: 使用 MLibrary.get_or_create_texture() 避免每帧创建纹理
    /// 
    /// C# 实现:
    /// ```csharp
    /// Libraries.MapLibs[M2CellInfo[x, y].BackIndex].Draw(index, drawX, drawY);
    /// // Draw() 内部使用 MImage 缓存纹理 (DXManager.TextureList)
    /// ```
    /// 
    /// 参数:
    /// - ctx: ggez Context (用于创建 Image)
    /// - lib_index: MapLibs 数组索引 (0-399)
    /// - image_index: 图像索引
    /// - x, y: 屏幕坐标
    fn draw_tile(&self, ctx: &mut Context, canvas: &mut Canvas, lib_index: i32, image_index: usize, x: f32, y: f32) -> GameResult<()> {
        use ggez::graphics::DrawParam;
        
        // 获取 MapLibrary
        if let Some(map_lib) = get_map_library(lib_index) {
            let mut lib = map_lib.lock().unwrap();
            
            // 🚀 使用纹理缓存 (关键性能优化)
            // 先获取图像信息以计算偏移
            match lib.get_image_info(image_index) {
                Ok(info) => {
                    let draw_x = x + info.x as f32;
                    let draw_y = y + info.y as f32;
                    
                    // 然后获取或创建纹理并绘制
                    match lib.get_or_create_texture(ctx, image_index) {
                        Ok(texture) => {
                            // 绘制缓存的纹理
                            canvas.draw(texture, DrawParam::default().dest([draw_x, draw_y]));
                        }
                        Err(e) => {
                            // C#的CheckImage在Width/Height==0时静默返回false
                            // 只在非InvalidData错误时打印警告
                            if e.kind() != std::io::ErrorKind::InvalidData {
                                tracing::warn!("⚠️  Failed to get/create texture (lib={}, img={}): {}", lib_index, image_index, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    // 同样,对InvalidData错误静默处理
                    if e.kind() != std::io::ErrorKind::InvalidData {
                        tracing::warn!("⚠️  Failed to get image info (lib={}, img={}): {}", lib_index, image_index, e);
                    }
                }
            }
        } else {
            tracing::warn!("⚠️  Map library {} not loaded!", lib_index);
        }
        
        Ok(())
    }
    
    /// Open door at location
    pub fn open_door(&mut self, x: i32, y: i32) {
        if let Some(cell) = self.get_cell(x, y) {
            if cell.door_index > 0 {
                let door_idx = cell.door_index as usize;
                if let Some(door) = self.doors.get_mut(door_idx) {
                    door.opened = true;
                }
            }
        }
    }
    
    /// Close door at location
    pub fn close_door(&mut self, x: i32, y: i32) {
        if let Some(cell) = self.get_cell(x, y) {
            if cell.door_index > 0 {
                let door_idx = cell.door_index as usize;
                if let Some(door) = self.doors.get_mut(door_idx) {
                    door.opened = false;
                }
            }
        }
    }
    
    /// Update animation
    pub fn update_animation(&mut self) {
        self.animation_count = (self.animation_count + 1) % 100;
    }
    
    /// Clear map data
    pub fn clear(&mut self) {
        self.cells.clear();
        self.doors.clear();
        self.filename.clear();
        self.title.clear();
        self.index = 0;
    }
}

impl Default for MapControl {
    fn default() -> Self {
        Self::new(100, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_map_control_creation() {
        let map = MapControl::new(100, 100);
        assert_eq!(map.width, 100);
        assert_eq!(map.height, 100);
        assert_eq!(map.cells.len(), 100);
        assert_eq!(map.cells[0].len(), 100);
    }
    
    #[test]
    fn test_coordinate_conversion() {
        let map = MapControl::new(100, 100);
        
        let (map_x, map_y) = map.screen_to_map(480, 320);
        assert_eq!(map_x, 10);
        assert_eq!(map_y, 10);
        
        let (screen_x, screen_y) = map.map_to_screen(10, 10);
        assert_eq!(screen_x, 480);
        assert_eq!(screen_y, 320);
    }
    
    #[test]
    fn test_is_valid_location() {
        let map = MapControl::new(100, 100);
        
        assert!(map.is_valid_location(0, 0));
        assert!(map.is_valid_location(99, 99));
        assert!(!map.is_valid_location(-1, 0));
        assert!(!map.is_valid_location(0, -1));
        assert!(!map.is_valid_location(100, 0));
        assert!(!map.is_valid_location(0, 100));
    }
    
    #[test]
    fn test_walkable() {
        let map = MapControl::new(10, 10);
        
        // Use CellInfo's is_walkable method
        assert!(map.is_walkable(5, 5));
    }
    
    #[test]
    fn test_door_operations() {
        let mut map = MapControl::new(10, 10);
        
        // Add a door
        map.doors.push(Door {
            index: 0,
            location: (5, 5),
            opened: false,
            image_index: 100,
        });
        
        // Link cell to door
        if let Some(cell) = map.get_cell_mut(5, 5) {
            cell.door_index = 0; // u8 type
        }
        
        // Open door
        map.open_door(5, 5);
        assert!(map.doors[0].opened);
        
        // Close door
        map.close_door(5, 5);
        assert!(!map.doors[0].opened);
    }
}
