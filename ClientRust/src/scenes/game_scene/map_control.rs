// MapControl - Map rendering and interaction control
// Mirrors Client/MirScenes/GameScene.cs::MapControl (lines 10062-12294)

use mir2_shared::enums::{LightSetting, WeatherSetting};
use crate::objects::{MapReader, CellInfo}; // 使用 objects::map_code 中的定义

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
