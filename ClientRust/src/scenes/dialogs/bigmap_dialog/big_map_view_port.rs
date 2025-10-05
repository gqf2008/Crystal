// BigMapViewPort - Viewport control for big map display
// Rust implementation of Client/MirScenes/Dialogs/BigMapDialog.BigMapViewPort

use crate::scenes::dialogs::Dialog;

/// Big map viewport control - handles map display and interaction
pub struct BigMapViewPort {
    visible: bool,
    pub scale_x: f32,
    pub scale_y: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub selected_npc_icon_visible: bool,
    pub selected_npc_icon_index: u16,
    pub user_radar_dot_visible: bool,
    pub user_radar_dot_x: i32,
    pub user_radar_dot_y: i32,
    pub mouse_coords_offset_x: i32,
    pub mouse_coords_offset_y: i32,
    pub players: Vec<PlayerDot>,
    pub player_locations: std::collections::HashMap<String, (i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct PlayerDot {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
}

impl BigMapViewPort {
    pub fn new() -> Self {
        let mut players = Vec::new();
        // Initialize player dots (max group size)
        for _ in 0..10 {
            players.push(PlayerDot {
                visible: false,
                x: 0,
                y: 0,
            });
        }

        Self {
            visible: true,
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0,
            offset_y: 0,
            selected_npc_icon_visible: false,
            selected_npc_icon_index: 0,
            user_radar_dot_visible: false,
            user_radar_dot_x: 0,
            user_radar_dot_y: 0,
            mouse_coords_offset_x: 0,
            mouse_coords_offset_y: 0,
            players,
            player_locations: std::collections::HashMap::new(),
        }
    }

    pub fn set_scale(&mut self, scale_x: f32, scale_y: f32) {
        self.scale_x = scale_x;
        self.scale_y = scale_y;
    }

    pub fn set_offset(&mut self, offset_x: i32, offset_y: i32) {
        self.offset_x = offset_x;
        self.offset_y = offset_y;
    }

    pub fn show_selected_npc_icon(&mut self, icon_index: u16) {
        self.selected_npc_icon_index = icon_index;
        self.selected_npc_icon_visible = true;
    }

    pub fn hide_selected_npc_icon(&mut self) {
        self.selected_npc_icon_visible = false;
    }

    pub fn show_user_radar_dot(&mut self, x: i32, y: i32) {
        self.user_radar_dot_x = x;
        self.user_radar_dot_y = y;
        self.user_radar_dot_visible = true;
    }

    pub fn hide_user_radar_dot(&mut self) {
        self.user_radar_dot_visible = false;
    }

    pub fn update_mouse_coordinates(&mut self, mouse_x: i32, mouse_y: i32) -> (i32, i32) {
        let map_x = ((mouse_x - self.mouse_coords_offset_x) as f32 / self.scale_x) as i32;
        let map_y = ((mouse_y - self.mouse_coords_offset_y) as f32 / self.scale_y) as i32;
        (map_x, map_y)
    }

    pub fn set_mouse_coords_offset(&mut self, offset_x: i32, offset_y: i32) {
        self.mouse_coords_offset_x = offset_x;
        self.mouse_coords_offset_y = offset_y;
    }

    pub fn update_player_location(&mut self, player_name: String, x: i32, y: i32) {
        self.player_locations.insert(player_name, (x, y));
    }

    pub fn remove_player_location(&mut self, player_name: &str) {
        self.player_locations.remove(player_name);
    }

    pub fn get_player_location(&self, player_name: &str) -> Option<(i32, i32)> {
        self.player_locations.get(player_name).copied()
    }

    pub fn update_player_dots(&mut self) {
        let mut index = 0;
        for (name, (x, y)) in &self.player_locations {
            if index < self.players.len() {
                self.players[index].visible = true;
                self.players[index].x = *x;
                self.players[index].y = *y;
                index += 1;
            }
        }

        // Hide remaining player dots
        for i in index..self.players.len() {
            self.players[i].visible = false;
        }
    }

    pub fn zoom_in(&mut self) {
        self.scale_x = (self.scale_x * 1.2).min(3.0);
        self.scale_y = (self.scale_y * 1.2).min(3.0);
    }

    pub fn zoom_out(&mut self) {
        self.scale_x = (self.scale_x * 0.8).max(0.5);
        self.scale_y = (self.scale_y * 0.8).max(0.5);
    }

    pub fn reset_zoom(&mut self) {
        self.scale_x = 1.0;
        self.scale_y = 1.0;
    }

    pub fn center_on_location(&mut self, x: i32, y: i32, viewport_width: i32, viewport_height: i32) {
        self.offset_x = x - viewport_width / 2;
        self.offset_y = y - viewport_height / 2;
    }
}

impl Dialog for BigMapViewPort {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update player dots based on current locations
        self.update_player_dots();
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic would go here
        // - Draw map background
        // - Draw selected NPC icon
        // - Draw user radar dot
        // - Draw player dots
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "BigMapViewPort" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 568 && y >= 0 && y < 380 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (568, 380) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigmap_viewport_creation() {
        let viewport = BigMapViewPort::new();
        assert!(viewport.is_visible());
        assert_eq!(viewport.scale_x, 1.0);
        assert_eq!(viewport.scale_y, 1.0);
        assert_eq!(viewport.players.len(), 10);
    }

    #[test]
    fn test_selected_npc_icon() {
        let mut viewport = BigMapViewPort::new();
        assert!(!viewport.selected_npc_icon_visible);

        viewport.show_selected_npc_icon(123);
        assert!(viewport.selected_npc_icon_visible);
        assert_eq!(viewport.selected_npc_icon_index, 123);

        viewport.hide_selected_npc_icon();
        assert!(!viewport.selected_npc_icon_visible);
    }

    #[test]
    fn test_user_radar_dot() {
        let mut viewport = BigMapViewPort::new();
        assert!(!viewport.user_radar_dot_visible);

        viewport.show_user_radar_dot(100, 200);
        assert!(viewport.user_radar_dot_visible);
        assert_eq!(viewport.user_radar_dot_x, 100);
        assert_eq!(viewport.user_radar_dot_y, 200);

        viewport.hide_user_radar_dot();
        assert!(!viewport.user_radar_dot_visible);
    }

    #[test]
    fn test_mouse_coordinates() {
        let mut viewport = BigMapViewPort::new();
        viewport.set_mouse_coords_offset(10, 20);
        viewport.set_scale(2.0, 2.0);

        let (map_x, map_y) = viewport.update_mouse_coordinates(50, 60);
        // (50 - 10) / 2.0 = 20, (60 - 20) / 2.0 = 20
        assert_eq!(map_x, 20);
        assert_eq!(map_y, 20);
    }

    #[test]
    fn test_player_locations() {
        let mut viewport = BigMapViewPort::new();

        viewport.update_player_location("Player1".to_string(), 10, 20);
        viewport.update_player_location("Player2".to_string(), 30, 40);

        assert_eq!(viewport.get_player_location("Player1"), Some((10, 20)));
        assert_eq!(viewport.get_player_location("Player2"), Some((30, 40)));
        assert_eq!(viewport.get_player_location("Player3"), None);

        viewport.remove_player_location("Player1");
        assert_eq!(viewport.get_player_location("Player1"), None);
    }

    #[test]
    fn test_zoom() {
        let mut viewport = BigMapViewPort::new();

        viewport.zoom_in();
        assert_eq!(viewport.scale_x, 1.2);
        assert_eq!(viewport.scale_y, 1.2);

        viewport.zoom_out();
        assert_eq!(viewport.scale_x, 1.2 * 0.8);
        assert_eq!(viewport.scale_y, 1.2 * 0.8);

        viewport.reset_zoom();
        assert_eq!(viewport.scale_x, 1.0);
        assert_eq!(viewport.scale_y, 1.0);
    }

    #[test]
    fn test_center_on_location() {
        let mut viewport = BigMapViewPort::new();
        viewport.center_on_location(100, 200, 50, 60);

        assert_eq!(viewport.offset_x, 100 - 25);
        assert_eq!(viewport.offset_y, 200 - 30);
    }
}