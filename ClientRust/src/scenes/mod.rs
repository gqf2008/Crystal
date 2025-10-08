// MirScenes - Game scene system
// Mirrors the structure of Client/MirScenes/

pub mod state;
pub mod scene_manager;
pub mod login_scene;
pub mod select_scene;
pub mod game_scene;
pub mod dialogs;


// Re-export scene types
pub use state::ClientState;
pub use scene_manager::{SceneManager, SharedSceneManager, create_shared_scene_manager};
pub use login_scene::{LoginScene, BanInfo};
pub use select_scene::SelectScene;
pub use game_scene::GameScene;
pub use game_scene::map_control::{MapControl, Door};
pub use crate::objects::CellInfo; // 从 objects::map_code 导出

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

/// Mouse button (ggez compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// Key code (ggez compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ,
    KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT,
    KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    Digit1, Digit2, Digit3, Digit4, Digit5,
    Digit6, Digit7, Digit8, Digit9, Digit0,
    Enter, Escape, Backspace, Tab, Space,
    ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // 可根据需要添加更多
}

/// Modifier state
#[derive(Debug, Clone, Copy, Default)]
pub struct ModifiersState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl ModifiersState {
    pub fn control_key(&self) -> bool {
        self.ctrl
    }
}

/// Scene trait - all scenes must implement this
pub trait Scene {
    /// Get scene type
    fn scene_type(&self) -> SceneType;
    
    /// Initialize scene (called once when scene is created)
    fn initialize(&mut self);
    
    /// Downcast to concrete type (for accessing scene-specific methods)
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    
    /// Downcast to concrete type (immutable)
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Process per-frame logic
    fn update(&mut self, delta_time: f32);
    
    /// Render scene (ggez版本)
    fn draw(&mut self, _ctx: &mut ggez::Context, _canvas: &mut crate::graphics::Canvas, _ggez_manager: &mut crate::graphics::GgezManager) {
        // Default: do nothing
    }
    
    /// Process game event from GameClient
    fn process_event(&mut self, event: &GameEvent);
    
    /// Handle mouse move
    fn handle_mouse_move(&mut self, _x: i32, _y: i32) {
        // Default implementation: do nothing
    }
    
    /// Handle mouse button
    fn handle_mouse_button(&mut self, _button: MouseButton, _pressed: bool, _x: i32, _y: i32) {
        // Default implementation: do nothing
    }
    
    /// Handle key press (returns true if handled)
    fn handle_key_press(&mut self, _key: KeyCode, _modifiers: ModifiersState) -> bool {
        // Default implementation: not handled
        false
    }
    
    /// Handle text input (for typing in text fields)
    fn handle_text_input(&mut self, _character: char) {
        // Default implementation: do nothing
    }
    
    /// Handle IME preedit (拼音编辑中)
    fn handle_ime_preedit(&mut self, _text: String) {
        // Default implementation: do nothing
    }
    
    /// Handle IME commit (中文确认输入)
    fn handle_ime_commit(&mut self, _text: String) {
        // Default implementation: do nothing
    }
}
