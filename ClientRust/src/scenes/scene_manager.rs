// Scene Manager - Manages scene transitions and lifecycle
// Corresponds to: Client/Program.cs scene management

use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::network::game_client::GameEvent;
use super::{Scene, SceneType, LoginScene, SelectScene, GameScene};

/// Scene manager - handles scene lifecycle and transitions
pub struct SceneManager {
    /// Current active scene
    current_scene: Option<Box<dyn Scene>>,
    
    /// Pending scene transition (if any)
    pending_scene: Option<SceneType>,
}

impl SceneManager {
    /// Create a new scene manager
    pub fn new() -> Self {
        Self {
            current_scene: None,
            pending_scene: None,
        }
    }
    
    /// Get current scene type
    pub fn current_scene_type(&self) -> Option<SceneType> {
        self.current_scene.as_ref().map(|s| s.scene_type())
    }
    
    /// Check if a scene is active
    pub fn has_scene(&self) -> bool {
        self.current_scene.is_some()
    }
    
    /// Switch to a new scene
    pub fn switch_scene(&mut self, scene_type: SceneType) -> Result<()> {
        tracing::info!("Switching to scene: {:?}", scene_type);
        
        // Clean up old scene
        if let Some(old_scene) = self.current_scene.take() {
            tracing::debug!("Cleaning up old scene: {:?}", old_scene.scene_type());
            // Scene cleanup happens automatically via Drop
        }
        
        // Create and initialize new scene
        let mut new_scene: Box<dyn Scene> = match scene_type {
            SceneType::Login => Box::new(LoginScene::new()),
            SceneType::Select => Box::new(SelectScene::new(Vec::new())), // Empty character list initially
            SceneType::Game => Box::new(GameScene::new()),
        };
        
        new_scene.initialize();
        self.current_scene = Some(new_scene);
        
        tracing::info!("Scene switch completed: {:?}", scene_type);
        Ok(())
    }
    
    /// Queue a scene transition (will happen at end of frame)
    pub fn queue_scene_transition(&mut self, scene_type: SceneType) {
        tracing::debug!("Queuing scene transition to: {:?}", scene_type);
        self.pending_scene = Some(scene_type);
    }
    
    /// Process any pending scene transitions
    pub fn process_transitions(&mut self) -> Result<()> {
        if let Some(scene_type) = self.pending_scene.take() {
            self.switch_scene(scene_type)?;
        }
        Ok(())
    }
    
    /// Update current scene
    pub fn update(&mut self, delta_time: f32) {
        if let Some(scene) = &mut self.current_scene {
            scene.update(delta_time);
        }
    }
    
    /// Render current scene (ggez版本)
    pub fn draw(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager) {
        if let Some(scene) = &self.current_scene {
            scene.draw(ctx, canvas, ggez_manager);
        }
    }
    
    /// Process game event in current scene
    pub fn process_event(&mut self, event: &GameEvent) {
        if let Some(scene) = &mut self.current_scene {
            scene.process_event(event);
        }
    }
    
    /// Handle keyboard input (使用Scene自定义KeyCode)
    pub fn handle_key_press(&mut self, key: super::KeyCode, modifiers: super::ModifiersState) -> bool {
        if let Some(scene) = &mut self.current_scene {
            return scene.handle_key_press(key, modifiers);
        }
        false
    }
    
    /// Handle mouse input
    pub fn handle_mouse_move(&mut self, x: i32, y: i32) {
        if let Some(scene) = &mut self.current_scene {
            scene.handle_mouse_move(x, y);
        }
    }
    
    /// Handle mouse button (使用Scene自定义MouseButton)
    pub fn handle_mouse_button(&mut self, button: super::MouseButton, pressed: bool, x: i32, y: i32) {
        if let Some(scene) = &mut self.current_scene {
            scene.handle_mouse_button(button, pressed, x, y);
        }
    }
    
    /// Handle text input
    pub fn handle_text_input(&mut self, character: char) {
        if let Some(scene) = &mut self.current_scene {
            scene.handle_text_input(character);
        }
    }
    
    /// Handle IME preedit (拼音编辑中)
    pub fn handle_ime_preedit(&mut self, text: String) {
        if let Some(scene) = &mut self.current_scene {
            scene.handle_ime_preedit(text);
        }
    }
    
    /// Handle IME commit (中文确认输入)
    pub fn handle_ime_commit(&mut self, text: String) {
        if let Some(scene) = &mut self.current_scene {
            scene.handle_ime_commit(text);
        }
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe scene manager
pub type SharedSceneManager = Arc<RwLock<SceneManager>>;

/// Create a shared scene manager
pub fn create_shared_scene_manager() -> SharedSceneManager {
    Arc::new(RwLock::new(SceneManager::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scene_manager_creation() {
        let manager = SceneManager::new();
        assert!(!manager.has_scene());
        assert_eq!(manager.current_scene_type(), None);
    }
    
    #[test]
    fn test_scene_transitions() {
        let mut manager = SceneManager::new();
        
        // Switch to login scene
        manager.switch_scene(SceneType::Login).unwrap();
        assert!(manager.has_scene());
        assert_eq!(manager.current_scene_type(), Some(SceneType::Login));
        
        // Switch to select scene
        manager.switch_scene(SceneType::Select).unwrap();
        assert_eq!(manager.current_scene_type(), Some(SceneType::Select));
        
        // Switch to game scene
        manager.switch_scene(SceneType::Game).unwrap();
        assert_eq!(manager.current_scene_type(), Some(SceneType::Game));
    }
    
    #[test]
    fn test_queued_transitions() {
        let mut manager = SceneManager::new();
        
        manager.switch_scene(SceneType::Login).unwrap();
        assert_eq!(manager.current_scene_type(), Some(SceneType::Login));
        
        // Queue a transition
        manager.queue_scene_transition(SceneType::Select);
        assert_eq!(manager.current_scene_type(), Some(SceneType::Login)); // Still login
        
        // Process transitions
        manager.process_transitions().unwrap();
        assert_eq!(manager.current_scene_type(), Some(SceneType::Select)); // Now select
    }
}
