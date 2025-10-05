// SelectScene - Character selection scene
// Mirrors Client/MirScenes/SelectScene.cs

use super::{Scene, SceneType};
use super::dialogs::NewCharacterDialog;
use crate::network::game_client::GameEvent;
use mir2_shared::SelectInfo;

/// Character selection scene
/// 
/// Mirrors C# SelectScene:
/// ```csharp
/// public class SelectScene : MirScene
/// {
///     public List<SelectInfo> Characters = new List<SelectInfo>();
///     private int _selected;
///     private NewCharacterDialog _character;
///     // ... UI controls
/// }
/// ```
pub struct SelectScene {
    // Character list (mirrors C# Characters)
    /// Mirrors C# `public List<SelectInfo> Characters`
    pub characters: Vec<SelectInfo>,
    
    /// Mirrors C# `private int _selected`
    pub selected_index: i32,
    
    // Dialogs
    /// Mirrors C# `private NewCharacterDialog _character`
    pub new_character_dialog: Option<NewCharacterDialog>,
    
    // TODO Phase 3: Add UI controls
    // - Background: MirImageControl
    // - Title: MirImageControl
    // - CharacterButtons: [CharacterButton; 4]
    // - StartGameButton: MirButton
    // - NewCharacterButton: MirButton
    // - DeleteCharacterButton: MirButton
    // - CreditsButton: MirButton
    // - ExitGame: MirButton
    // - CharacterDisplay: MirAnimatedControl
    // - LastAccessLabel: MirLabel
}

impl SelectScene {
    /// Create new select scene with character list
    /// 
    /// Mirrors C# constructor:
    /// ```csharp
    /// public SelectScene(List<SelectInfo> characters)
    /// {
    ///     Characters = characters;
    ///     SortList();
    ///     // ... initialize UI
    /// }
    /// ```
    pub fn new(characters: Vec<SelectInfo>) -> Self {
        let mut scene = Self {
            characters,
            selected_index: 0,
            new_character_dialog: None,
        };
        scene.sort_list();
        scene
    }
    
    /// Sort character list by last access time
    /// 
    /// Mirrors C# SortList():
    /// ```csharp
    /// public void SortList()
    /// {
    ///     if (Characters != null)
    ///         Characters.Sort((c1, c2) => c2.LastAccess.CompareTo(c1.LastAccess));
    /// }
    /// ```
    fn sort_list(&mut self) {
        self.characters.sort_by(|a, b| b.last_access.cmp(&a.last_access));
    }
    
    /// Select character by index
    /// 
    /// Mirrors C# CharacterButton click handler:
    /// ```csharp
    /// _selected = index;
    /// UpdateInterface();
    /// ```
    pub fn select_character(&mut self, index: i32) {
        if index >= 0 && (index as usize) < self.characters.len() {
            self.selected_index = index;
            println!("Selected character: {}", self.characters[index as usize].name);
            // TODO: UpdateInterface() - update UI display
        }
    }
    
    /// Start game with selected character
    /// 
    /// Mirrors C# StartGame():
    /// ```csharp
    /// private void StartGame()
    /// {
    ///     // Send StartGame packet
    ///     Network.Enqueue(new C.StartGame { CharacterIndex = Characters[_selected].Index });
    /// }
    /// ```
    pub fn start_game(&mut self) {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            let character = &self.characters[self.selected_index as usize];
            println!("Starting game with character: {}", character.name);
            // TODO: Send StartGame packet
            // TODO: Switch to game scene
        }
    }
    
    /// Open new character creation dialog
    /// 
    /// Mirrors C# OpenNewCharacterDialog():
    /// ```csharp
    /// private void OpenNewCharacterDialog()
    /// {
    ///     if (_character == null || _character.IsDisposed)
    ///     {
    ///         _character = new NewCharacterDialog { Parent = this };
    ///         // ...
    ///     }
    /// }
    /// ```
    pub fn open_new_character_dialog(&mut self) {
        if self.new_character_dialog.is_none() {
            println!("Opening character creation dialog");
            self.new_character_dialog = Some(NewCharacterDialog::new());
            // TODO: Show dialog UI
        }
    }
    
    /// Delete selected character
    /// 
    /// Mirrors C# DeleteCharacter():
    /// ```csharp
    /// private void DeleteCharacter()
    /// {
    ///     // Show confirm dialog
    ///     // Send DeleteCharacter packet
    /// }
    /// ```
    pub fn delete_character(&mut self) {
        if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
            let character = &self.characters[self.selected_index as usize];
            println!("Deleting character: {}", character.name);
            // TODO: Show confirm dialog
            // TODO: Send DeleteCharacter packet
        }
    }
}

impl Default for SelectScene {
    fn default() -> Self {
        Self::new(Vec::new())
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
    
    fn draw(&self, _canvas: &mut crate::graphics::Canvas, _ggez_manager: &crate::graphics::GgezManager) {
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
    
    fn handle_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
    }
    
    fn handle_mouse_button(&mut self, button: super::MouseButton, pressed: bool, x: i32, y: i32) {
        if pressed {
            tracing::debug!("SelectScene click at ({}, {}) with {:?}", x, y, button);
            // TODO: Handle character slot clicks
            // TODO: Handle button clicks
        }
    }
    
    fn handle_key_press(&mut self, key: super::KeyCode, _modifiers: super::ModifiersState) -> bool {
        use super::KeyCode;
        
        match key {
            KeyCode::Enter => {
                self.start_game();
                true
            }
            KeyCode::Escape => {
                // TODO: Return to login scene
                tracing::info!("Escape pressed - would return to login");
                true
            }
            _ => false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::{MirClass, MirGender};

    fn create_test_character(index: i32, name: &str, timestamp: i64) -> SelectInfo {
        use std::io::Cursor;
        use byteorder::{LittleEndian, WriteBytesExt};
        
        // Create a minimal binary representation
        let mut buffer = Vec::new();
        buffer.write_i32::<LittleEndian>(index).unwrap();
        // Write string length + string
        let name_bytes = name.as_bytes();
        buffer.push(name_bytes.len() as u8);
        buffer.extend_from_slice(name_bytes);
        buffer.write_u16::<LittleEndian>(1).unwrap(); // level
        buffer.write_u8(MirClass::Warrior as u8).unwrap();
        buffer.write_u8(MirGender::Male as u8).unwrap();
        buffer.write_i64::<LittleEndian>(timestamp).unwrap();
        
        let mut cursor = Cursor::new(buffer);
        SelectInfo::read_from(&mut cursor).unwrap()
    }

    #[test]
    fn test_select_scene_creation() {
        let now = 638000000000000000i64; // Some .NET DateTime ticks
        let characters = vec![
            create_test_character(0, "TestChar1", now),
            create_test_character(1, "TestChar2", now),
        ];
        let scene = SelectScene::new(characters);
        assert_eq!(scene.scene_type(), SceneType::Select);
        assert_eq!(scene.characters.len(), 2);
        assert_eq!(scene.selected_index, 0);
    }

    #[test]
    fn test_character_selection() {
        let now = 638000000000000000i64;
        let characters = vec![
            create_test_character(0, "TestChar1", now),
            create_test_character(1, "TestChar2", now),
            create_test_character(2, "TestChar3", now),
        ];
        let mut scene = SelectScene::new(characters);
        
        scene.select_character(1);
        assert_eq!(scene.selected_index, 1);
        
        // Out of bounds should be ignored
        scene.select_character(10);
        assert_eq!(scene.selected_index, 1);
        
        // Negative index should be ignored
        scene.select_character(-1);
        assert_eq!(scene.selected_index, 1);
    }
    
    #[test]
    fn test_sort_characters_by_last_access() {
        let old_time = 638000000000000000i64;
        let new_time = 638000100000000000i64; // Later time
        
        let characters = vec![
            create_test_character(0, "Old", old_time),
            create_test_character(1, "Recent", new_time),
        ];
        
        let scene = SelectScene::new(characters);
        // Should be sorted by last_access descending (most recent first)
        assert_eq!(scene.characters[0].name, "Recent");
        assert_eq!(scene.characters[1].name, "Old");
    }
}
