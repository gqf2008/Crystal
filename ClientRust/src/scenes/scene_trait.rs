// Scene trait - base interface for all scenes
// Mirrors Client/MirScenes/MirScene.cs

use crate::network::protocol::ServerMessage;

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
    
    /// Process network packet
    fn process_packet(&mut self, packet: ServerMessage);
    
    /// Handle mouse move
    fn on_mouse_move(&mut self, x: i32, y: i32);
    
    /// Handle mouse click
    fn on_mouse_click(&mut self, x: i32, y: i32, button: MouseButton);
    
    /// Handle key press
    fn on_key_press(&mut self, key: KeyCode);
    
    /// Show scene (called when scene becomes active)
    fn show(&mut self);
    
    /// Hide scene (called when scene becomes inactive)
    fn hide(&mut self);
    
    /// Dispose scene (called when scene is destroyed)
    fn dispose(&mut self);
}

/// Mouse button enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Key code enumeration (simplified)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    // Movement
    Up,
    Down,
    Left,
    Right,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Control keys
    Escape,
    Enter,
    Tab,
    Space,
    Shift,
    Control,
    Alt,
    
    // Number keys
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    
    // Letter keys (A-Z)
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Other
    Unknown,
}
