// LoginScene - Login scene implementation
// Mirrors Client/MirScenes/LoginScene.cs

use super::scene_trait::{Scene, SceneType, MouseButton, KeyCode};
use crate::network::game_client::GameEvent;

/// Login scene state
#[derive(Debug)]
pub struct LoginScene {
    // Network connection
    pub connecting: bool,
    pub connect_attempts: u32,
    
    // UI state
    pub username: String,
    pub password: String,
    pub remember_account: bool,
    
    // Dialogs (TODO: implement when dialog system ready)
    // login_dialog: LoginDialog,
    // new_account_dialog: Option<NewAccountDialog>,
    // change_password_dialog: Option<ChangePasswordDialog>,
}

impl LoginScene {
    /// Create new login scene
    pub fn new() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
            username: String::new(),
            password: String::new(),
            remember_account: false,
        }
    }
    
    /// Attempt to connect to server
    pub fn connect_to_server(&mut self) {
        self.connecting = true;
        self.connect_attempts += 1;
        
        // TODO: Actual network connection
        println!("Attempting to connect to server (attempt {})", self.connect_attempts);
    }
    
    /// Submit login credentials
    pub fn submit_login(&mut self) {
        if self.username.is_empty() || self.password.is_empty() {
            println!("Username and password required");
            return;
        }
        
        // TODO: Send login packet
        println!("Logging in: {}", self.username);
    }
    
    /// Open new account dialog
    pub fn open_new_account_dialog(&mut self) {
        println!("Opening new account dialog");
        // TODO: Show new account dialog
    }
    
    /// Open change password dialog
    pub fn open_change_password_dialog(&mut self) {
        println!("Opening change password dialog");
        // TODO: Show change password dialog
    }
}

impl Default for LoginScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for LoginScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Login
    }
    
    fn initialize(&mut self) {
        println!("LoginScene::initialize");
        // TODO: Load login UI
        // TODO: Play intro music
        self.connect_to_server();
    }
    
    fn update(&mut self, _delta_time: f32) {
        // TODO: Update connection status
        // TODO: Update animations
    }
    
    fn draw(&self) {
        // TODO: Draw login background
        // TODO: Draw login dialogs
        // TODO: Draw version info
    }
    
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::Connected => {
                println!("Connected to server!");
                self.connecting = false;
            }
            GameEvent::Disconnected { reason } => {
                println!("Disconnected: {}", reason);
                self.connecting = false;
            }
            GameEvent::SystemMessage { message } => {
                println!("System: {}", message);
                // TODO: Display in UI
            }
            _ => {
                // Ignore other events in login scene
            }
        }
    }
    
    fn on_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
    }
    
    fn on_mouse_click(&mut self, x: i32, y: i32, button: MouseButton) {
        println!("LoginScene click at ({}, {}) with {:?}", x, y, button);
        // TODO: Handle dialog clicks
    }
    
    fn on_key_press(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.submit_login();
            }
            KeyCode::Escape => {
                // TODO: Close dialog or exit
            }
            _ => {}
        }
    }
    
    fn show(&mut self) {
        println!("LoginScene::show");
        // TODO: Show login UI
        // TODO: Start music
    }
    
    fn hide(&mut self) {
        println!("LoginScene::hide");
        // TODO: Hide login UI
    }
    
    fn dispose(&mut self) {
        println!("LoginScene::dispose");
        // TODO: Cleanup resources
        // TODO: Stop music
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_scene_creation() {
        let scene = LoginScene::new();
        assert_eq!(scene.scene_type(), SceneType::Login);
        assert!(!scene.connecting);
        assert_eq!(scene.connect_attempts, 0);
    }

    #[test]
    fn test_login_validation() {
        let mut scene = LoginScene::new();
        
        // Empty credentials should fail
        scene.submit_login(); // Should print error
        
        // Set credentials
        scene.username = "testuser".to_string();
        scene.password = "testpass".to_string();
        scene.submit_login(); // Should proceed
    }
}
