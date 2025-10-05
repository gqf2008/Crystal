// Cached Sound - Preloaded audio data in memory
// Mirrors Client.MirSounds.Libraries.CachedSound

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use rodio::{Decoder, Source};

/// Cached sound data stored in memory
pub struct CachedSound {
    /// Sound index
    pub index: i32,
    
    /// Expiration time for cache cleanup
    pub expire_time: Instant,
    
    /// Audio sample data (interleaved, normalized to f32)
    pub audio_data: Vec<f32>,
    
    /// Sample rate (e.g., 44100 Hz)
    pub sample_rate: u32,
    
    /// Number of channels (1=mono, 2=stereo)
    pub channels: u16,
}

impl CachedSound {
    /// Create a new cached sound from file
    /// 
    /// # Arguments
    /// * `index` - Sound index
    /// * `sound_path` - Base path to sound directory
    /// * `filename` - Base filename (without extension)
    /// 
    /// # Returns
    /// * `Ok(CachedSound)` if file found and loaded
    /// * `Err` if file not found or decode failed
    pub fn new(index: i32, sound_path: &Path, filename: &str) -> Result<Self> {
        let file_path = Self::find_sound_file(sound_path, filename)?;
        
        // Open and decode audio file
        let file = File::open(&file_path)
            .with_context(|| format!("Failed to open sound file: {:?}", file_path))?;
        
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode sound file: {:?}", file_path))?;
        
        let sample_rate = source.sample_rate();
        let channels = source.channels();
        
        // Read all samples into memory
        let audio_data: Vec<f32> = source.convert_samples().collect();
        
        Ok(Self {
            index,
            expire_time: Instant::now(),
            audio_data,
            sample_rate,
            channels,
        })
    }
    
    /// Find sound file with supported extensions
    fn find_sound_file(base_path: &Path, filename: &str) -> Result<PathBuf> {
        let extensions = &[".wav", ".mp3", ".ogg", ".flac"];
        
        for ext in extensions {
            let mut path = base_path.join(filename);
            path.set_extension(&ext[1..]); // Remove leading dot
            
            if path.exists() {
                return Ok(path);
            }
        }
        
        // Try with original filename if it has an extension
        let path = base_path.join(filename);
        if path.exists() {
            return Ok(path);
        }
        
        anyhow::bail!(
            "Sound file not found: {} (tried extensions: {:?})",
            filename,
            extensions
        )
    }
    
    /// Get duration of the sound
    pub fn duration(&self) -> std::time::Duration {
        let total_samples = self.audio_data.len() / self.channels as usize;
        let seconds = total_samples as f64 / self.sample_rate as f64;
        std::time::Duration::from_secs_f64(seconds)
    }
    
    /// Check if cache has expired
    pub fn is_expired(&self, now: Instant, expiry_duration: std::time::Duration) -> bool {
        now.duration_since(self.expire_time) > expiry_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_duration_calculation() {
        let sound = CachedSound {
            index: 1,
            expire_time: Instant::now(),
            audio_data: vec![0.0; 88200], // 2 seconds at 44.1kHz stereo
            sample_rate: 44100,
            channels: 2,
        };
        
        let duration = sound.duration();
        assert_eq!(duration.as_secs(), 1); // Should be ~1 second
    }
    
    #[test]
    fn test_expiry_check() {
        let mut sound = CachedSound {
            index: 1,
            expire_time: Instant::now(),
            audio_data: vec![],
            sample_rate: 44100,
            channels: 2,
        };
        
        // Just created, should not be expired
        assert!(!sound.is_expired(Instant::now(), std::time::Duration::from_secs(60)));
        
        // Set old expire time
        sound.expire_time = Instant::now() - std::time::Duration::from_secs(120);
        assert!(sound.is_expired(Instant::now(), std::time::Duration::from_secs(60)));
    }
}
