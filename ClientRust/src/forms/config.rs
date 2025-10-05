// Configuration Window - Settings editor
// Corresponds to: Client/CConfig.cs

use anyhow::Result;
use std::sync::Arc;
use winit::window::Window;
use crate::settings::{ClientSettings, SupportedResolution};

/// Resolution option for display
#[derive(Debug, Clone)]
pub struct ResolutionOption {
    pub width: u32,
    pub height: u32,
    pub resolution: SupportedResolution,
}

/// Configuration window state
pub struct ConfigWindow {
    /// Window handle
    window: Arc<Window>,
    
    /// Current settings
    settings: ClientSettings,
    
    /// Available resolutions
    resolutions: Vec<ResolutionOption>,
    
    /// Selected resolution index
    selected_resolution: usize,
    
    /// Whether settings have changed
    dirty: bool,
}

impl ConfigWindow {
    /// Create a new config window
    pub fn new(window: Arc<Window>, settings: ClientSettings) -> Self {
        // Create resolution options from SupportedResolution enum
        let resolutions = vec![
            ResolutionOption { width: 1024, height: 768, resolution: SupportedResolution::W1024H768 },
            ResolutionOption { width: 1280, height: 720, resolution: SupportedResolution::W1280H720 },
            ResolutionOption { width: 1366, height: 768, resolution: SupportedResolution::W1366H768 },
            ResolutionOption { width: 1920, height: 1080, resolution: SupportedResolution::W1920H1080 },
        ];
        
        let dims = settings.graphics.dimensions();
        
        // Find current resolution index
        let selected_resolution = resolutions
            .iter()
            .position(|r| r.width == dims.width as u32 && r.height == dims.height as u32)
            .unwrap_or(0); // Default to first (1024x768)
        
        Self {
            window,
            settings,
            resolutions,
            selected_resolution,
            dirty: false,
        }
    }
    
    /// Get current settings
    pub fn settings(&self) -> &ClientSettings {
        &self.settings
    }
    
    /// Check if settings have changed
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Set resolution
    pub fn set_resolution(&mut self, index: usize) {
        if index < self.resolutions.len() {
            let res = &self.resolutions[index];
            // Update resolution in settings
            self.settings.graphics.resolution = res.resolution;
            self.selected_resolution = index;
            self.dirty = true;
        }
    }
    
    /// Get available resolutions
    pub fn get_resolutions(&self) -> &[ResolutionOption] {
        &self.resolutions
    }
    
    /// Get selected resolution index
    pub fn get_selected_resolution(&self) -> usize {
        self.selected_resolution
    }
    
    /// Set fullscreen mode
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if self.settings.graphics.full_screen != fullscreen {
            self.settings.graphics.full_screen = fullscreen;
            self.dirty = true;
        }
    }
    
    /// Set sound volume (0-100)
    pub fn set_sound_volume(&mut self, volume: u8) {
        let volume = volume.min(100);
        if self.settings.sound.volume != volume {
            self.settings.sound.volume = volume;
            self.dirty = true;
        }
    }
    
    /// Set music volume (0-100)
    pub fn set_music_volume(&mut self, volume: u8) {
        let volume = volume.min(100);
        if self.settings.sound.music != volume {
            self.settings.sound.music = volume;
            self.dirty = true;
        }
    }
    
    /// Toggle FPS cap
    pub fn set_fps_cap(&mut self, enabled: bool) {
        if self.settings.graphics.fps_cap != enabled {
            self.settings.graphics.fps_cap = enabled;
            self.dirty = true;
        }
    }
    
    /// Save settings
    pub fn save(&mut self) -> Result<()> {
        tracing::info!("Saving settings");
        self.settings.save()?;
        self.dirty = false;
        Ok(())
    }
    
    /// Reset to defaults
    pub fn reset_to_defaults(&mut self) {
        tracing::info!("Resetting to default settings");
        self.settings = ClientSettings::default();
        self.dirty = true;
    }
    
    /// Render the config UI
    pub fn render(&self) {
        // TODO: Implement rendering using wgpu
        // Should display:
        // 1. Resolution dropdown
        // 2. Fullscreen checkbox
        // 3. Sound volume slider
        // 4. Music volume slider
        // 5. FPS cap checkbox
        // 6. Save button
        // 7. Cancel button
        // 8. Reset button
    }
    
    /// Handle window events
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::WindowEvent;
        
        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Config window close requested");
                return true;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // TODO: Handle button clicks
                tracing::debug!("Mouse input: {:?} {:?}", state, button);
            }
            _ => {}
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolution_selection() {
        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .build(&winit::event_loop::EventLoop::new().unwrap())
                .unwrap()
        );
        let settings = ClientSettings::default();
        let mut config = ConfigWindow::new(window, settings);
        
        assert!(!config.is_dirty());
        
        config.set_resolution(0);
        assert!(config.is_dirty());
    }
    
    #[test]
    fn test_volume_settings() {
        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .build(&winit::event_loop::EventLoop::new().unwrap())
                .unwrap()
        );
        let mut settings = ClientSettings::default();
        settings.volume = 50;
        settings.music_volume = 50;
        
        let mut config = ConfigWindow::new(window, settings);
        
        config.set_sound_volume(75);
        assert_eq!(config.settings().volume, 75);
        assert!(config.is_dirty());
        
        config.dirty = false;
        config.set_music_volume(80);
        assert_eq!(config.settings().music_volume, 80);
        assert!(config.is_dirty());
        
        // Test clamping
        config.set_sound_volume(150);
        assert_eq!(config.settings().volume, 100);
    }
    
    #[test]
    fn test_reset_to_defaults() {
        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .build(&winit::event_loop::EventLoop::new().unwrap())
                .unwrap()
        );
        let mut settings = ClientSettings::default();
        settings.volume = 75;
        settings.fullscreen = true;
        
        let mut config = ConfigWindow::new(window, settings);
        config.reset_to_defaults();
        
        let defaults = ClientSettings::default();
        assert_eq!(config.settings().volume, defaults.volume);
        assert_eq!(config.settings().fullscreen, defaults.fullscreen);
        assert!(config.is_dirty());
    }
}
