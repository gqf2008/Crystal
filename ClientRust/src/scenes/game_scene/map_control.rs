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
        // C#: OffSetX = Settings.ScreenWidth / 2 / CellWidth;
        // C#: OffSetY = Settings.ScreenHeight / 2 / CellHeight - 1;
        // 窗口大小: 1024x768
        let offset_x = 1024 / 2 / Self::CELL_WIDTH;  // 512 / 48 = 10
        let offset_y = 768 / 2 / Self::CELL_HEIGHT - 1;  // 384 / 32 - 1 = 11
        
        // C#: ViewRangeX = OffSetX + 6;
        // C#: ViewRangeY = OffSetY + 6;
        let view_range_x = offset_x + 6;  // 10 + 6 = 16
        let view_range_y = offset_y + 6;  // 11 + 6 = 17
        
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
            offset_x,
            offset_y,
            view_range_x,
            view_range_y,
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
    
    /// Draw map - 简化版本,只渲染基础瓦片
    /// 
    /// 🔧 临时简化模式: 禁用所有高级渲染,只显示最基础的地表瓦片
    /// 用于调试瓦片错位和黑块问题
    /// 
    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, user_pos: &UserPosition) -> GameResult<()> {
        // 更新动画计数器
        self.animation_count = (self.animation_count + 1) % 1000;
        
        // 1. 绘制背景图（远景山脉/沙漠等）
        self.draw_background(ctx, canvas)?;
        
        // 2. 渲染地板（Back/Middle/Front三层）
        self.draw_floor(ctx, canvas, user_pos)?;
        
        // 3. 🚧 临时：绘制角色位置标记（红色方块）
        self.draw_player_marker(ctx, canvas, user_pos)?;
        
        Ok(())
    }
    
    /// 🚧 临时方法：绘制角色位置标记
    fn draw_player_marker(&self, _ctx: &mut Context, canvas: &mut Canvas, user_pos: &UserPosition) -> GameResult<()> {
        use ggez::graphics::{Mesh, Rect, Color, DrawMode};
        
        // 角色应该在屏幕中央
        let screen_center_x = (self.offset_x * Self::CELL_WIDTH) as f32;
        let screen_center_y = (self.offset_y * Self::CELL_HEIGHT) as f32;
        
        // 绘制一个红色方块标记角色位置
        let rect = Rect::new(screen_center_x - 24.0, screen_center_y - 16.0, 48.0, 32.0);
        let mesh = Mesh::new_rectangle(
            _ctx,
            DrawMode::stroke(2.0),
            rect,
            Color::from_rgb(255, 0, 0),
        )?;
        canvas.draw(&mesh, ggez::graphics::DrawParam::default());
        
        static mut FIRST_MARKER: bool = true;
        unsafe {
            if FIRST_MARKER {
                println!("🎯 角色位置标记: 屏幕中心=({:.1},{:.1}), 地图位置=({},{})", 
                    screen_center_x, screen_center_y, user_pos.x, user_pos.y);
                FIRST_MARKER = false;
            }
        }
        
        Ok(())
    }
    
    /// 渲染地板（整合 Back/Middle/Front 三层）
    /// 
    /// C# 参考: GameScene.cs DrawFloor() 第10442-10544行
    /// 完全对应C#的渲染流程，将三层整合在一个方法中
    fn draw_floor(&mut self, ctx: &mut Context, canvas: &mut Canvas, user_pos: &UserPosition) -> GameResult<()> {
        let start_y = (user_pos.y - self.view_range_y).max(0);
        let end_y_back = (user_pos.y + self.view_range_y).min(self.height - 1);
        let end_y_extended = (user_pos.y + self.view_range_y + 5).min(self.height - 1);
        let start_x = (user_pos.x - self.view_range_x).max(0);
        let end_x = (user_pos.x + self.view_range_x).min(self.width - 1);
        
        static mut FIRST_CALL: bool = true;
        unsafe {
            if FIRST_CALL {
                println!("\n🎨 === draw_floor 调试信息 ===");
                println!("用户位置: ({}, {})", user_pos.x, user_pos.y);
                println!("用户偏移: ({}, {})", user_pos.offset_x, user_pos.offset_y);
                println!("视野范围: x[{}, {}], y[{}, {}]", start_x, end_x, start_y, end_y_back);
                println!("offset_x={}, offset_y={}", self.offset_x, self.offset_y);
                println!("============================\n");
                FIRST_CALL = false;
            }
        }
        
        // ========== Back Layer (地表瓦片，只渲染偶数行列) ==========
        // C# 第10459-10475行
        static mut FIRST_BACK_TILE: bool = true;
        for y in start_y..=end_y_back {
            if y <= 0 || y % 2 == 1 { continue; }
            
            let draw_y = ((y - user_pos.y + self.offset_y) * Self::CELL_HEIGHT + user_pos.offset_y) as f32;
            
            for x in start_x..=end_x {
                if x <= 0 || x % 2 == 1 { continue; }
                
                let draw_x = ((x - user_pos.x + self.offset_x) * Self::CELL_WIDTH - self.offset_x + user_pos.offset_x) as f32;
                
                if let Some(cell) = self.get_cell(x, y) {
                    if cell.back_image > 0 && cell.back_index >= 0 {
                        let masked_back = cell.back_image & 0x1FFFFFFF;
                        let index = (masked_back as usize) - 1;
                        
                        unsafe {
                            if FIRST_BACK_TILE {
                                println!("🟢 Back层第一个瓦片: 地图({},{}) 屏幕({:.1},{:.1})", x, y, draw_x, draw_y);
                                FIRST_BACK_TILE = false;
                            }
                        }
                        
                        let _ = self.draw_tile(ctx, canvas, cell.back_index as i32, index, draw_x, draw_y);
                    }
                }
            }
        }
        
        // ========== Middle Layer (建筑层，渲染所有格子) ==========
        // C# 第10477-10496行
        static mut FIRST_MIDDLE_TILE: bool = true;
        for y in start_y..=end_y_extended {
            if y <= 0 { continue; }
            
            let draw_y = ((y - user_pos.y + self.offset_y) * Self::CELL_HEIGHT + user_pos.offset_y) as f32;
            
            for x in start_x..=end_x {
                if x < 0 { continue; }
                
                let draw_x = ((x - user_pos.x + self.offset_x) * Self::CELL_WIDTH - self.offset_x + user_pos.offset_x) as f32;
                
                if let Some(cell) = self.get_cell(x, y) {
                    // 🔧 CRITICAL FIX: 屏蔽 HighWall 标志 (0x20000000)
                    // Map Type 100 使用高位标记墙体属性，必须屏蔽后才能获取正确的图像索引
                    let middle_image_masked = cell.middle_image & 0x1FFFFFFF;
                    
                    if middle_image_masked > 0 && cell.middle_index >= 0 {
                        let index = (middle_image_masked as usize) - 1;
                        
                        // 🔧 关键修复：C# 第10495-10496行的尺寸检查
                        // Middle层瓦片必须是 48x32 或 96x64，否则跳过
                        if let Some(size_valid) = self.check_tile_size(cell.middle_index as i32, index) {
                            if !size_valid {
                                static mut SKIP_COUNT: u32 = 0;
                                unsafe {
                                    if SKIP_COUNT < 3 {
                                        println!("⚠️  跳过尺寸不符的Middle瓦片: 地图({},{}) lib={} idx={}", 
                                            x, y, cell.middle_index, index);
                                        SKIP_COUNT += 1;
                                    }
                                }
                                continue;
                            }
                        }
                        
                        unsafe {
                            if FIRST_MIDDLE_TILE {
                                println!("🔵 Middle层第一个瓦片: 地图({},{}) 屏幕({:.1},{:.1})", x, y, draw_x, draw_y);
                                FIRST_MIDDLE_TILE = false;
                            }
                        }
                        
                        let _ = self.draw_tile(ctx, canvas, cell.middle_index as i32, index, draw_x, draw_y);
                    }
                }
            }
        }
        
        // ========== Front Layer (前景层，带Y偏移) ==========
        // C# 第10497-10542行
        static mut FIRST_FRONT_TILE: bool = true;
        for y in start_y..=end_y_extended {
            if y <= 0 { continue; }
            
            let base_y = ((y - user_pos.y + self.offset_y) * Self::CELL_HEIGHT + user_pos.offset_y) as f32;
            let draw_y = base_y - 32.0; // Front层向上偏移32像素
            
            for x in start_x..=end_x {
                if x < 0 { continue; }
                
                let draw_x = ((x - user_pos.x + self.offset_x) * Self::CELL_WIDTH - self.offset_x + user_pos.offset_x) as f32;
                
                if let Some(cell) = self.get_cell(x, y) {
                    let masked_front = cell.front_image & 0x7FFF;
                    if masked_front > 0 && cell.front_index >= 0 {
                        let index = (masked_front as usize) - 1;
                        
                        unsafe {
                            if FIRST_FRONT_TILE {
                                println!("🟡 Front层第一个瓦片: 地图({},{}) 屏幕({:.1},{:.1})", x, y, draw_x, draw_y);
                                FIRST_FRONT_TILE = false;
                            }
                        }
                        
                        // TODO: Door动画处理（如果需要）
                        // if cell.door_index > 0 { ... }
                        
                        let _ = self.draw_tile(ctx, canvas, cell.front_index as i32, index, draw_x, draw_y);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    
    
    // /// 绘制远景背景
    // /// 
    // /// 对应 C# DrawBackground (line 10546-10566)
    // /// 
    // /// 根据地图文件名选择背景图:
    // /// - ID1/ID2 → 山脉背景(index 10)
    /// 绘制背景图（远景）
    /// 
    /// C# 参考: GameScene.cs 第10545-10565行
    /// 根据地图文件名绘制对应的背景图:
    /// - ID1/ID2 → 山脉背景(index 10)
    /// - ID3_013 → 沙漠背景(index 22)
    /// - ID3_015 → 长城背景(index 23)
    /// - ID3_023/025 → 村庄入口(index 21)
    fn draw_background(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult<()> {
        use crate::graphics::libraries::LibraryName;
        use ggez::graphics::{Image, ImageFormat, DrawParam};
        
        static mut FIRST_CALL: bool = true;
        
        // 提取干净的文件名（去除路径）
        let normalized = self.filename.replace("\\", "/");
        let clean_filename = normalized
            .split('/')
            .last()
            .unwrap_or(&self.filename);
        
        unsafe {
            if FIRST_CALL {
                println!("🗺️  当前地图文件: {}", self.filename);
                println!("🗺️  清理后文件名: {}", clean_filename);
                FIRST_CALL = false;
            }
        }
        
        let background_index = if clean_filename.starts_with("ID1") || clean_filename.starts_with("ID2") {
            Some(10) // mountains
        } else if clean_filename.starts_with("ID3_013") {
            Some(22) // desert
        } else if clean_filename.starts_with("ID3_015") {
            Some(23) // greatwall
        } else if clean_filename.starts_with("ID3_023") || clean_filename.starts_with("ID3_025") {
            Some(21) // village entrance
        } else if clean_filename.starts_with("r") || clean_filename.starts_with("R") {
            // 🔧 r001, r002 等地图使用山脉背景
            Some(10) // mountains (default for 'r' prefix maps)
        } else {
            None
        };
        
        if let Some(idx) = background_index {
            // 从Background库加载背景图
            use crate::graphics::libraries::get_library;
            if let Some(bg_lib_arc) = get_library(LibraryName::Background) {
                if let Ok(mut bg_lib) = bg_lib_arc.lock() {
                    match bg_lib.load_rgba_data(idx) {
                        Ok((info, rgba_data)) => {
                            let texture = Image::from_pixels(
                                ctx,
                                &rgba_data,
                                ImageFormat::Rgba8UnormSrgb,
                                info.width as u32,
                                info.height as u32,
                            );
                            
                            // 🔧 CRITICAL FIX: wgpu 坐标系统 (0,0) 在左下角，C# DirectX (0,0) 在左上角
                            // 背景图应该从屏幕顶部开始，所以 y = screen_height - image_height
                            // C#: Draw(index, 0, 0) 表示从屏幕左上角开始
                            // wgpu: dest([0, screen_height - img_height]) 才能从屏幕左上角开始
                            let screen_height = 768.0; // 窗口高度
                            let draw_y = screen_height - info.height as f32;
                            
                            canvas.draw(&texture, DrawParam::default().dest([0.0, draw_y]));
                            
                            unsafe {
                                if FIRST_CALL {
                                    println!("✅ 背景图已绘制: idx={}, 尺寸={}x{}, offset=({},{}), 屏幕位置=(0, {:.1})", 
                                        idx, info.width, info.height, info.x, info.y, draw_y);
                                }
                            }
                        }
                        Err(e) => {
                            unsafe {
                                if FIRST_CALL {
                                    println!("❌ 加载背景图失败: idx={}, error={}", idx, e);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            unsafe {
                if FIRST_CALL {
                    println!("⚠️  当前地图无背景图（文件名: {}）", clean_filename);
                }
            }
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
    
    /// 检查瓦片尺寸是否有效
    /// 
    /// C# 参考: GameScene.cs 第10495-10496行
    /// Middle/Front层瓦片必须是 48x32 或 96x64
    /// 
    /// 返回:
    /// - Some(true): 尺寸有效 (48x32 或 96x64)
    /// - Some(false): 尺寸无效
    /// - None: 无法获取尺寸信息
    fn check_tile_size(&self, lib_index: i32, image_index: usize) -> Option<bool> {
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            if let Ok(mut lib) = map_lib.lock() {
                if let Ok((info, _)) = lib.load_rgba_data(image_index) {
                    let w = info.width as i32;
                    let h = info.height as i32;
                    
                    // 检查是否为 48x32 或 96x64
                    let is_single = w == Self::CELL_WIDTH && h == Self::CELL_HEIGHT;
                    let is_double = w == Self::CELL_WIDTH * 2 && h == Self::CELL_HEIGHT * 2;
                    
                    return Some(is_single || is_double);
                }
            }
        }
        None
    }
    
    /// 从 MapLibs 绘制瓦片 (简化版本,禁用offset)
    /// 
    /// 🔧 临时简化: 不使用图像内部偏移,直接使用屏幕坐标
    /// 
    /// 参数:
    /// - ctx: ggez Context (用于创建 Image)
    /// - lib_index: MapLibs 数组索引 (0-399)
    /// - image_index: 图像索引
    /// - x, y: 屏幕坐标
    /// 绘制地图瓦片 (使用纹理缓存)
    /// 
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.Draw() - 自动缓存纹理
    /// if (!CheckImage(index)) return;  // 加载并缓存纹理
    /// MImage mi = _images[index];
    /// DXManager.Draw(mi.Image, ...);   // 使用缓存的纹理
    /// ```
    /// 
    /// 关键优化:
    /// - ✅ 使用 `get_or_create_texture()` 缓存机制
    /// - ✅ 纹理只创建一次,后续帧直接复用
    /// - ✅ 避免每帧重新加载和解压图像数据
    fn draw_tile(&self, ctx: &mut Context, canvas: &mut Canvas, lib_index: i32, image_index: usize, x: f32, y: f32) -> GameResult<()> {
        use ggez::graphics::DrawParam;
        
        // 获取 MapLibrary (转换 i32 -> i16)
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();
            
            // 🔧 先获取图像偏移信息,避免借用冲突
            let (offset_x, offset_y) = if let Ok(info) = lib.get_image_info(image_index) {
                (info.x as f32, info.y as f32)
            } else {
                (0.0, 0.0)
            };
            
            // ✅ 使用纹理缓存机制 (对应 C# MLibrary.CheckImage + CreateTexture)
            match lib.get_or_create_texture(ctx, image_index) {
                Ok(texture) => {
                    let draw_x = x + offset_x;
                    let draw_y = y + offset_y;
                    
                    // 绘制缓存的纹理
                    canvas.draw(texture, DrawParam::default().dest([draw_x, draw_y]));
                }
                Err(e) => {
                    // C#的CheckImage在Width/Height==0时静默返回false
                    // 只在非InvalidData错误时打印警告
                    if e.kind() != std::io::ErrorKind::InvalidData {
                        tracing::warn!("⚠️  Failed to load tile (lib={}, img={}): {}", lib_index, image_index, e);
                    }
                }
            }
        } else {
            tracing::warn!("⚠️  Map library {} not loaded!", lib_index);
        }
        
        Ok(())
    }
    
    /// 简化版绘制函数 - 不使用图像内部偏移 (使用纹理缓存)
    fn draw_tile_simple(&self, ctx: &mut Context, canvas: &mut Canvas, lib_index: i32, image_index: usize, x: f32, y: f32) -> GameResult<()> {
        use ggez::graphics::DrawParam;
        
        if let Some(map_lib) = get_map_library(lib_index as i16) {
            let mut lib = map_lib.lock().unwrap();
            
            // ✅ 使用纹理缓存机制
            match lib.get_or_create_texture(ctx, image_index) {
                Ok(texture) => {
                    // 🔧 不使用图像偏移,直接使用传入的坐标
                    canvas.draw(texture, DrawParam::default().dest([x, y]));
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::InvalidData {
                        tracing::warn!("⚠️  Failed to load tile (lib={}, img={}): {}", lib_index, image_index, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 垂直翻转纹理数据 (修复DirectX vs wgpu坐标系差异)
    /// 
    /// # 背景
    /// - DirectX (C#): Top-Left 坐标系，(0,0)在左上角
    /// - wgpu (Rust): Bottom-Left 坐标系，(0,0)在左下角
    /// 
    /// # 参数
    /// - `data`: RGBA格式的像素数据 (每像素4字节)
    /// - `width`: 图像宽度
    /// - `height`: 图像高度
    fn flip_texture_vertically(data: &mut [u8], width: usize, height: usize) {
        let row_size = width * 4; // RGBA = 4 bytes per pixel
        let mut temp_row = vec![0u8; row_size];
        
        for y in 0..(height / 2) {
            let top_offset = y * row_size;
            let bottom_offset = (height - 1 - y) * row_size;
            
            // 交换上下两行
            temp_row.copy_from_slice(&data[top_offset..top_offset + row_size]);
            data.copy_within(bottom_offset..bottom_offset + row_size, top_offset);
            data[bottom_offset..bottom_offset + row_size].copy_from_slice(&temp_row);
        }
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
