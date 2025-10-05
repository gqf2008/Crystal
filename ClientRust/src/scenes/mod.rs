// MirScenes - Game scene system
// Mirrors the structure of Client/MirScenes/

pub mod state;
pub mod scene_manager;
pub mod login_scene;
pub mod select_scene;
pub mod game_scene;
pub mod map_control;
pub mod dialogs;

// Re-export scene types
pub use state::ClientState;
pub use scene_manager::{SceneManager, SharedSceneManager, create_shared_scene_manager};
pub use login_scene::{LoginScene, BanInfo};
pub use select_scene::SelectScene;
pub use game_scene::GameScene;
pub use map_control::{MapControl, CellInfo, Door};

// Re-export enums from SharedRust
pub use mir2_shared::enums::{LightSetting, WeatherSetting};

// Scene trait - base interface for all scenes
// Mirrors Client/MirScenes/MirScene.cs

use crate::network::game_client::GameEvent;

/// Scene type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneType {
    Login,
    Select,
    Game,
}

/// Scene trait - all scenes must implement this
pub trait Scene {
    /// Get scene type
    fn scene_type(&self) -> SceneType;
    
    /// Initialize scene (called once when scene is created)
    fn initialize(&mut self);
    
    /// Process per-frame logic
    fn update(&mut self, delta_time: f32);
    
    /// Render scene
    fn draw(&self);
    
    /// Process game event from GameClient
    fn process_event(&mut self, event: &GameEvent);
    
    /// Handle mouse move
    fn handle_mouse_move(&mut self, _x: i32, _y: i32) {
        // Default implementation: do nothing
    }
    
    /// Handle mouse button
    fn handle_mouse_button(&mut self, _button: winit::event::MouseButton, _pressed: bool, _x: i32, _y: i32) {
        // Default implementation: do nothing
    }
    
    /// Handle key press (returns true if handled)
    fn handle_key_press(&mut self, _key: winit::keyboard::KeyCode, _modifiers: winit::keyboard::ModifiersState) -> bool {
        // Default implementation: not handled
        false
    }
}
