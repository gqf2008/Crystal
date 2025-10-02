// BigMapDialog - World Map / Big Map System
// Rust implementation of Client/MirScenes/Dialogs/BigMapDialog.cs

use crate::scenes::dialogs::Dialog;

/// Map information for big map display
#[derive(Debug, Clone)]
pub struct MapRecord {
    pub index: u16,
    pub title: String,
    pub width: u16,
    pub height: u16,
    pub mini_map: u16,
    pub big_map: u16,
    pub can_teleport: bool,
    pub can_fly: bool,
}

/// NPC information displayed on map
#[derive(Debug, Clone)]
pub struct MapNPC {
    pub index: u32,
    pub name: String,
    pub icon: u16,
    pub map_index: u16,
    pub x: i32,
    pub y: i32,
    pub can_teleport_to: bool,
    pub description: String,
}

/// Map image for display
#[derive(Debug, Clone)]
pub struct MapImage {
    pub index: u16,
    pub x: i32,
    pub y: i32,
    pub destination_map: Option<u16>,
}

/// Big map viewport state
#[derive(Debug, Clone)]
pub struct MapViewPort {
    pub offset_x: i32,
    pub offset_y: i32,
    pub scale: f32,
    pub show_user_dot: bool,
    pub show_selected_npc: bool,
}

impl MapViewPort {
    pub fn new() -> Self {
        Self {
            offset_x: 0,
            offset_y: 0,
            scale: 1.0,
            show_user_dot: true,
            show_selected_npc: false,
        }
    }

    pub fn set_center(&mut self, x: i32, y: i32) {
        self.offset_x = x;
        self.offset_y = y;
    }

    pub fn zoom_in(&mut self) {
        self.scale = (self.scale * 1.2).min(3.0);
    }

    pub fn zoom_out(&mut self) {
        self.scale = (self.scale * 0.8).max(0.5);
    }
}

/// Big Map Dialog - World map with NPC locations
pub struct BigMapDialog {
    visible: bool,
    pub current_map: Option<MapRecord>,
    pub target_map_index: u16,
    pub npcs: Vec<MapNPC>,
    pub images: Vec<MapImage>,
    pub viewport: MapViewPort,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub selected_npc: Option<usize>,
    pub scroll_offset: usize,
    pub max_visible_npcs: usize, // 18 in C#
    pub search_text: String,
    pub world_map_mode: bool, // true = world map, false = detailed map
}

impl BigMapDialog {
    const MAX_NPC_ROWS: usize = 18;

    pub fn new() -> Self {
        Self {
            visible: false,
            current_map: None,
            target_map_index: 0,
            npcs: Vec::new(),
            images: Vec::new(),
            viewport: MapViewPort::new(),
            mouse_x: 0,
            mouse_y: 0,
            selected_npc: None,
            scroll_offset: 0,
            max_visible_npcs: Self::MAX_NPC_ROWS,
            search_text: String::new(),
            world_map_mode: false,
        }
    }

    pub fn set_current_map(&mut self, map: MapRecord) {
        self.target_map_index = map.index;
        self.current_map = Some(map);
        self.scroll_offset = 0;
        self.selected_npc = None;
    }

    pub fn add_npc(&mut self, npc: MapNPC) {
        self.npcs.push(npc);
    }

    pub fn clear_npcs(&mut self) {
        self.npcs.clear();
        self.selected_npc = None;
    }

    pub fn add_map_image(&mut self, image: MapImage) {
        self.images.push(image);
    }

    pub fn clear_images(&mut self) {
        self.images.clear();
    }

    pub fn set_mouse_location(&mut self, x: i32, y: i32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    pub fn get_coordinate_text(&self) -> String {
        format!("X: {}, Y: {}", self.mouse_x, self.mouse_y)
    }

    pub fn select_npc(&mut self, index: usize) -> bool {
        if index < self.npcs.len() {
            self.selected_npc = Some(index);
            true
        } else {
            false
        }
    }

    pub fn get_selected_npc(&self) -> Option<&MapNPC> {
        self.selected_npc.and_then(|idx| self.npcs.get(idx))
    }

    pub fn can_teleport_to_selected(&self) -> bool {
        if let Some(npc) = self.get_selected_npc() {
            npc.can_teleport_to
        } else {
            false
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let total_npcs = self.get_filtered_npcs().len();
        if self.scroll_offset + self.max_visible_npcs < total_npcs {
            self.scroll_offset += 1;
        }
    }

    pub fn get_visible_npcs(&self) -> Vec<&MapNPC> {
        let filtered = self.get_filtered_npcs();
        filtered
            .iter()
            .skip(self.scroll_offset)
            .take(self.max_visible_npcs)
            .copied()
            .collect()
    }

    pub fn get_filtered_npcs(&self) -> Vec<&MapNPC> {
        if self.search_text.is_empty() {
            self.npcs.iter().collect()
        } else {
            let search_lower = self.search_text.to_lowercase();
            self.npcs
                .iter()
                .filter(|npc| {
                    npc.name.to_lowercase().contains(&search_lower)
                        || npc.description.to_lowercase().contains(&search_lower)
                })
                .collect()
        }
    }

    pub fn search(&mut self, text: String) {
        self.search_text = text;
        self.scroll_offset = 0;
        self.selected_npc = None;
    }

    pub fn clear_search(&mut self) {
        self.search_text.clear();
        self.scroll_offset = 0;
    }

    pub fn toggle_world_map(&mut self) {
        self.world_map_mode = !self.world_map_mode;
    }

    pub fn open_world_map(&mut self) {
        self.world_map_mode = true;
    }

    pub fn open_detailed_map(&mut self) {
        self.world_map_mode = false;
    }

    pub fn target_my_location(&mut self, user_x: i32, user_y: i32) {
        self.viewport.set_center(user_x, user_y);
        self.world_map_mode = false;
    }

    pub fn set_target_map(&mut self, map_index: u16) {
        self.target_map_index = map_index;
    }

    pub fn set_target_npc(&mut self, npc_index: u32) {
        if let Some(idx) = self.npcs.iter().position(|n| n.index == npc_index) {
            self.select_npc(idx);
        }
    }

    pub fn find_npc_by_name(&self, name: &str) -> Option<usize> {
        self.npcs
            .iter()
            .position(|n| n.name.eq_ignore_ascii_case(name))
    }

    pub fn total_npc_count(&self) -> usize {
        self.npcs.len()
    }

    pub fn filtered_npc_count(&self) -> usize {
        self.get_filtered_npcs().len()
    }

    pub fn can_scroll_up(&self) -> bool {
        self.scroll_offset > 0
    }

    pub fn can_scroll_down(&self) -> bool {
        self.scroll_offset + self.max_visible_npcs < self.filtered_npc_count()
    }
}

impl Dialog for BigMapDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic would go here
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic would go here
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigmap_dialog_creation() {
        let dialog = BigMapDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.npcs.len(), 0);
        assert_eq!(dialog.max_visible_npcs, 18);
    }

    #[test]
    fn test_map_record() {
        let map = MapRecord {
            index: 1,
            title: "Test Map".to_string(),
            width: 100,
            height: 100,
            mini_map: 1,
            big_map: 1,
            can_teleport: true,
            can_fly: false,
        };
        assert_eq!(map.title, "Test Map");
    }

    #[test]
    fn test_set_current_map() {
        let mut dialog = BigMapDialog::new();
        let map = MapRecord {
            index: 5,
            title: "Forest".to_string(),
            width: 200,
            height: 200,
            mini_map: 5,
            big_map: 5,
            can_teleport: true,
            can_fly: true,
        };
        dialog.set_current_map(map);
        assert!(dialog.current_map.is_some());
        assert_eq!(dialog.target_map_index, 5);
    }

    #[test]
    fn test_add_npc() {
        let mut dialog = BigMapDialog::new();
        let npc = MapNPC {
            index: 1,
            name: "Blacksmith".to_string(),
            icon: 10,
            map_index: 1,
            x: 50,
            y: 50,
            can_teleport_to: true,
            description: "Repairs equipment".to_string(),
        };
        dialog.add_npc(npc);
        assert_eq!(dialog.total_npc_count(), 1);
    }

    #[test]
    fn test_select_npc() {
        let mut dialog = BigMapDialog::new();
        dialog.add_npc(MapNPC {
            index: 1,
            name: "Guard".to_string(),
            icon: 5,
            map_index: 1,
            x: 10,
            y: 10,
            can_teleport_to: false,
            description: "Town guard".to_string(),
        });
        
        assert!(dialog.select_npc(0));
        assert!(dialog.get_selected_npc().is_some());
        assert_eq!(dialog.get_selected_npc().unwrap().name, "Guard");
    }

    #[test]
    fn test_scroll_npcs() {
        let mut dialog = BigMapDialog::new();
        for i in 0..25 {
            dialog.add_npc(MapNPC {
                index: i,
                name: format!("NPC {}", i),
                icon: 1,
                map_index: 1,
                x: 0,
                y: 0,
                can_teleport_to: false,
                description: String::new(),
            });
        }

        assert_eq!(dialog.scroll_offset, 0);
        dialog.scroll_down();
        assert_eq!(dialog.scroll_offset, 1);
        dialog.scroll_up();
        assert_eq!(dialog.scroll_offset, 0);
    }

    #[test]
    fn test_search_npcs() {
        let mut dialog = BigMapDialog::new();
        dialog.add_npc(MapNPC {
            index: 1,
            name: "Blacksmith".to_string(),
            icon: 1,
            map_index: 1,
            x: 0,
            y: 0,
            can_teleport_to: true,
            description: "Repairs".to_string(),
        });
        dialog.add_npc(MapNPC {
            index: 2,
            name: "Guard".to_string(),
            icon: 2,
            map_index: 1,
            x: 0,
            y: 0,
            can_teleport_to: false,
            description: "Protects".to_string(),
        });

        dialog.search("smith".to_string());
        assert_eq!(dialog.filtered_npc_count(), 1);
        
        dialog.clear_search();
        assert_eq!(dialog.filtered_npc_count(), 2);
    }

    #[test]
    fn test_viewport() {
        let mut viewport = MapViewPort::new();
        assert_eq!(viewport.scale, 1.0);
        
        viewport.zoom_in();
        assert!(viewport.scale > 1.0);
        
        viewport.zoom_out();
        viewport.zoom_out();
        assert!(viewport.scale < 1.0);
    }

    #[test]
    fn test_mouse_location() {
        let mut dialog = BigMapDialog::new();
        dialog.set_mouse_location(123, 456);
        assert_eq!(dialog.mouse_x, 123);
        assert_eq!(dialog.mouse_y, 456);
        assert_eq!(dialog.get_coordinate_text(), "X: 123, Y: 456");
    }

    #[test]
    fn test_world_map_toggle() {
        let mut dialog = BigMapDialog::new();
        assert!(!dialog.world_map_mode);
        
        dialog.toggle_world_map();
        assert!(dialog.world_map_mode);
        
        dialog.open_detailed_map();
        assert!(!dialog.world_map_mode);
    }

    #[test]
    fn test_find_npc_by_name() {
        let mut dialog = BigMapDialog::new();
        dialog.add_npc(MapNPC {
            index: 1,
            name: "Merchant".to_string(),
            icon: 1,
            map_index: 1,
            x: 0,
            y: 0,
            can_teleport_to: false,
            description: String::new(),
        });

        assert!(dialog.find_npc_by_name("merchant").is_some());
        assert!(dialog.find_npc_by_name("Guard").is_none());
    }

    #[test]
    fn test_can_teleport() {
        let mut dialog = BigMapDialog::new();
        dialog.add_npc(MapNPC {
            index: 1,
            name: "Teleporter".to_string(),
            icon: 1,
            map_index: 1,
            x: 0,
            y: 0,
            can_teleport_to: true,
            description: String::new(),
        });

        assert!(!dialog.can_teleport_to_selected());
        dialog.select_npc(0);
        assert!(dialog.can_teleport_to_selected());
    }

    #[test]
    fn test_visible_npcs() {
        let mut dialog = BigMapDialog::new();
        for i in 0..25 {
            dialog.add_npc(MapNPC {
                index: i,
                name: format!("NPC {}", i),
                icon: 1,
                map_index: 1,
                x: 0,
                y: 0,
                can_teleport_to: false,
                description: String::new(),
            });
        }

        let visible = dialog.get_visible_npcs();
        assert_eq!(visible.len(), 18); // MAX_NPC_ROWS
        
        dialog.scroll_down();
        let visible_after_scroll = dialog.get_visible_npcs();
        assert_eq!(visible_after_scroll[0].name, "NPC 1");
    }
}
