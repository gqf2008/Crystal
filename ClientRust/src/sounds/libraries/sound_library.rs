// Sound Library Trait - Interface for sound playback
// Mirrors Client.MirSounds.ISoundLibrary

use std::time::{Duration, Instant};

/// Sound library trait - common interface for all sound providers
pub trait SoundLibrary {
    /// Get sound index
    fn index(&self) -> i32;
    
    /// Set sound index
    fn set_index(&mut self, index: i32);
    
    /// Get expiration time
    fn expire_time(&self) -> Instant;
    
    /// Set expiration time
    fn set_expire_time(&mut self, time: Instant);
    
    /// Check if sound is currently playing
    fn is_playing(&self) -> bool;
    
    /// Play sound with given volume (0-100)
    fn play(&mut self, volume: i32);
    
    /// Stop sound playback
    fn stop(&mut self);
    
    /// Set volume (0-100)
    fn set_volume(&mut self, volume: i32);
}

/// Helper to convert volume from 0-100 to 0.0-1.0
pub fn scale_volume(volume: i32) -> f32 {
    let clamped = volume.clamp(0, 100);
    clamped as f32 / 100.0
}
