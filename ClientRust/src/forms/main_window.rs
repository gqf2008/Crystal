// Main Game Window - The main client window
// Corresponds to: Client/CMain.cs

use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use winit::window::Window;
use crate::settings::ClientSettings;
use crate::scenes::{SceneManager, SceneType};

/// Main game window state
pub struct MainWindow {
    /// Window handle
    window: Arc<Window>,
    
    /// Client settings
    settings: ClientSettings,
    
    /// Scene manager
    scene_manager: SceneManager,
    
    /// FPS counter
    fps: u32,
    fps_time: Instant,
    fps_count: u32,
    
    /// DPS counter (draws per second)
    dps: u32,
    dps_time: Instant,
    dps_count: u32,
    
    /// Network ping
    ping: i64,
    
    /// Mouse position
    mouse_x: i32,
    mouse_y: i32,
    
    /// Whether to show FPS
    show_fps: bool,
    
    /// Whether game is running
    running: bool,
}

impl MainWindow {
    /// Create a new main window
    pub fn new(window: Arc<Window>, settings: ClientSettings) -> Self {
        Self {
            window,
            settings,
            scene_manager: SceneManager::new(),
            fps: 0,
            fps_time: Instant::now(),
            fps_count: 0,
            dps: 0,
            dps_time: Instant::now(),
            dps_count: 0,
            ping: 0,
            mouse_x: 0,
            mouse_y: 0,
            show_fps: true,
            running: false,
        }
    }
    
    /// Initialize the game
    pub fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing main game window");
        
        // TODO: Initialize graphics
        // TODO: Initialize sounds
        // TODO: Load resources
        
        // Start with login scene
        self.scene_manager.switch_scene(SceneType::Login)?;
        
        self.running = true;
        Ok(())
    }
    
    /// Main game loop update
    pub fn update(&mut self, delta_time: f32) {
        if !self.running {
            return;
        }
        
        // Update FPS counter
        self.update_fps();
        
        // Process any queued scene transitions
        if let Err(e) = self.scene_manager.process_transitions() {
            tracing::error!("Failed to process scene transitions: {}", e);
        }
        
        // Update current scene
        self.scene_manager.update(delta_time);
        
        // TODO: Update network
        // TODO: Update sounds
    }
    
    /// Render the game
    pub fn render(&mut self) {
        if !self.running {
            return;
        }
        
        // Update DPS counter
        self.update_dps();
        
        // Render current scene
        self.scene_manager.draw();
        
        // TODO: Render FPS/ping if enabled
    }
    
    /// Update FPS counter
    fn update_fps(&mut self) {
        self.fps_count += 1;
        let elapsed = self.fps_time.elapsed();
        
        if elapsed.as_secs() >= 1 {
            self.fps = self.fps_count;
            self.fps_count = 0;
            self.fps_time = Instant::now();
        }
    }
    
    /// Update DPS counter
    fn update_dps(&mut self) {
        self.dps_count += 1;
        let elapsed = self.dps_time.elapsed();
        
        if elapsed.as_secs() >= 1 {
            self.dps = self.dps_count;
            self.dps_count = 0;
            self.dps_time = Instant::now();
        }
    }
    
    /// Handle window events
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::{WindowEvent, MouseButton, ElementState};
        
        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Main window close requested");
                self.running = false;
                return true;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as i32;
                self.mouse_y = position.y as i32;
                self.scene_manager.handle_mouse_move(self.mouse_x, self.mouse_y);
            }
            
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                self.scene_manager.handle_mouse_button(*button, pressed, self.mouse_x, self.mouse_y);
            }
            
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(key_code) = key_event.physical_key {
                        let modifiers = key_event.modifiers.state();
                        self.scene_manager.handle_key_press(key_code, modifiers);
                    }
                }
            }
            
            _ => {}
        }
        
        false
    }
    
    /// Get current scene type
    pub fn current_scene(&self) -> Option<SceneType> {
        self.scene_manager.current_scene_type()
    }
    
    /// Switch to a different scene
    pub fn switch_scene(&mut self, scene_type: SceneType) -> Result<()> {
        self.scene_manager.switch_scene(scene_type)
    }
    
    /// Get current FPS
    pub fn get_fps(&self) -> u32 {
        self.fps
    }
    
    /// Get current DPS
    pub fn get_dps(&self) -> u32 {
        self.dps
    }
    
    /// Get current ping
    pub fn get_ping(&self) -> i64 {
        self.ping
    }
    
    /// Set ping value
    pub fn set_ping(&mut self, ping: i64) {
        self.ping = ping;
    }
    
    /// Toggle FPS display
    pub fn toggle_fps(&mut self) {
        self.show_fps = !self.show_fps;
    }
    
    /// Clean up resources
    pub fn shutdown(&mut self) {
        tracing::info!("Shutting down main window");
        self.running = false;
        
        // TODO: Disconnect from server
        // TODO: Clean up graphics
        // TODO: Clean up sounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fps_counter() {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = Arc::new(
            event_loop.create_window(winit::window::Window::default_attributes())
                .unwrap()
        );
        let settings = ClientSettings::default();
        let window = MainWindow::new(window, settings);
        
        assert_eq!(window.get_fps(), 0);
        assert_eq!(window.get_dps(), 0);
    }
    
    #[test]
    fn test_ping() {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = Arc::new(
            event_loop.create_window(winit::window::Window::default_attributes())
                .unwrap()
        );
        let settings = ClientSettings::default();
        let mut window = MainWindow::new(window, settings);
        
        assert_eq!(window.get_ping(), 0);
        window.set_ping(50);
        assert_eq!(window.get_ping(), 50);
    }
}
