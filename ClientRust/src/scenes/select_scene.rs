// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs

use super::scene_trait::{Scene, SceneType, MouseButton, KeyCode};
use crate::network::game_client::GameEvent;
use mir2_shared::enums::{MirClass, MirGender};

/// Character selection data
#[derive(Debug, Clone)]
pub struct SelectCharacter {
    pub index: u32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub exists: bool,
}

/// Character selection scene
#[derive(Debug)]
pub struct SelectScene {
    // Character slots
    pub characters: Vec<Option<SelectCharacter>>,
    pub selected_index: usize,
    
    // UI state
    pub creating_character: bool,
    pub deleting_character: bool,
    
    // New character creation
    pub new_char_name: String,
    pub new_char_class: MirClass,
    pub new_char_gender: MirGender,
    
    // Dialogs (TODO: implement when dialog system ready)
    // new_character_dialog: Option<NewCharacterDialog>,
    // delete_confirm_dialog: Option<MessageBox>,
}

impl SelectScene {
    /// Create new select scene
    pub fn new() -> Self {
        Self {
            characters: vec![None; 4], // 4 character slots
            selected_index: 0,
            creating_character: false,
            deleting_character: false,
            new_char_name: String::new(),
            new_char_class: MirClass::Warrior,
            new_char_gender: MirGender::Male,
        }
    }
    
    /// Select character slot
    pub fn select_character(&mut self, index: usize) {
        if index < self.characters.len() {
            self.selected_index = index;
            println!("Selected character slot {}", index);
        }
    }
    
    /// Start game with selected character
    pub fn start_game(&mut self) {
        if let Some(Some(ref character)) = self.characters.get(self.selected_index) {
            println!("Starting game with character: {}", character.name);
            // TODO: Send StartGame packet
            // TODO: Switch to game scene
        } else {
            println!("No character in selected slot");
        }
    }
    
    /// Open new character creation dialog
    pub fn create_new_character(&mut self) {
        if matches!(self.characters.get(self.selected_index), Some(&None)) {
            println!("Opening character creation dialog");
            self.creating_character = true;
            // TODO: Show new character dialog
        } else {
            println!("Character slot already occupied");
        }
    }
    
    /// Submit new character creation
    pub fn submit_new_character(&mut self) {
        if self.new_char_name.is_empty() {
            println!("Character name required");
            return;
        }
        
        println!("Creating character: {}", self.new_char_name);
        // TODO: Send NewCharacter packet
        // TODO: Wait for server response
        
        self.creating_character = false;
    }
    
    /// Delete selected character
    pub fn delete_character(&mut self) {
        if let Some(Some(ref character)) = self.characters.get(self.selected_index) {
            println!("Deleting character: {}", character.name);
            self.deleting_character = true;
            // TODO: Show confirm dialog
            // TODO: Send DeleteCharacter packet
        } else {
            println!("No character to delete");
        }
    }
    
    /// Return to login screen
    pub fn return_to_login(&mut self) {
        println!("Returning to login screen");
        // TODO: Switch to login scene
    }
}

impl Default for SelectScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for SelectScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Select
    }
    
    fn initialize(&mut self) {
        println!("SelectScene::initialize");
        // TODO: Load character selection UI
        // TODO: Request character list from server
    }
    
    fn update(&mut self, _delta_time: f32) {
        // TODO: Update character previews
        // TODO: Update animations
    }
    
    fn draw(&self) {
        // TODO: Draw selection background
        // TODO: Draw character previews
        // TODO: Draw UI buttons
    }
    
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::SystemMessage { message } => {
                println!("System message: {}", message);
                // TODO: Display in UI
            }
            GameEvent::Disconnected { reason } => {
                println!("Disconnected: {}", reason);
                // TODO: Return to login
            }
            _ => {
                // TODO: Handle character creation/deletion events when added to GameEvent
                // For now, ignore other events
            }
        }
    }
    
    fn on_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
    }
    
    fn on_mouse_click(&mut self, x: i32, y: i32, button: MouseButton) {
        println!("SelectScene click at ({}, {}) with {:?}", x, y, button);
        // TODO: Handle character slot clicks
        // TODO: Handle button clicks
    }
    
    fn on_key_press(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if self.creating_character {
                    self.submit_new_character();
                } else {
                    self.start_game();
                }
            }
            KeyCode::Escape => {
                if self.creating_character {
                    self.creating_character = false;
                } else {
                    self.return_to_login();
                }
            }
            _ => {}
        }
    }
    
    fn show(&mut self) {
        println!("SelectScene::show");
        // TODO: Show selection UI
        // TODO: Play select music
    }
    
    fn hide(&mut self) {
        println!("SelectScene::hide");
        // TODO: Hide selection UI
    }
    
    fn dispose(&mut self) {
        println!("SelectScene::dispose");
        // TODO: Cleanup resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_scene_creation() {
        let scene = SelectScene::new();
        assert_eq!(scene.scene_type(), SceneType::Select);
        assert_eq!(scene.characters.len(), 4);
        assert_eq!(scene.selected_index, 0);
    }

    #[test]
    fn test_character_selection() {
        let mut scene = SelectScene::new();
        
        scene.select_character(2);
        assert_eq!(scene.selected_index, 2);
        
        // Out of bounds should be ignored
        scene.select_character(10);
        assert_eq!(scene.selected_index, 2);
    }
}
