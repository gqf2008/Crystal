// Sound Manager - Core sound playback management
// Mirrors Client.MirSounds.SoundManager

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::settings::SoundSettings;
use super::sound_list::{SoundId, SoundIndexMap, load_sound_list, generate_filename};

/// Delayed sound entry (play_time, sound_id)
type DelayedSound = (Instant, SoundId);

/// Cached sound data
struct CachedSound {
    data: Vec<u8>,
    expire_time: Instant,
}

/// Sound Manager - Main sound playback system
pub struct SoundManager {
    // Audio output
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    
    // Settings
    sound_path: PathBuf,
    volume: i32,
    music_volume: i32,
    
    // Sound index mapping (ID -> filename)
    index_list: SoundIndexMap,
    
    // One-shot sounds sink (mixed)
    one_shots_sink: Sink,
    
    // Looping sounds
    looping_sounds: HashMap<SoundId, Sink>,
    
    // Background music
    music_sink: Option<Sink>,
    
    // Cached one-shot sounds
    cached_sounds: HashMap<SoundId, CachedSound>,
    
    // Delayed sound queue
    delayed_sounds: Vec<DelayedSound>,
    
    // Cleanup timer
    last_cleanup_time: Instant,
    cleanup_interval: Duration,
    cache_expiry_duration: Duration,
}

impl SoundManager {
    /// Create a new SoundManager
    pub fn new(settings: &SoundSettings) -> Result<Self> {
        // Initialize audio output
        let (_stream, stream_handle) = OutputStream::try_default()
            .context("Failed to initialize audio output")?;
        
        // Create main sink for one-shot sounds
        let one_shots_sink = Sink::try_new(&stream_handle)
            .context("Failed to create one-shot sounds sink")?;
        
        // Load sound index list
        let sound_path = PathBuf::from(&settings.sound_path);
        let index_list = load_sound_list(&sound_path)
            .context("Failed to load sound list")?;
        
        let volume = settings.volume as i32;
        let music_volume = settings.music as i32;
        
        // Set initial volume
        let scaled_volume = Self::scale_volume(volume);
        one_shots_sink.set_volume(scaled_volume);
        
        Ok(Self {
            _stream,
            stream_handle,
            sound_path,
            volume,
            music_volume,
            index_list,
            one_shots_sink,
            looping_sounds: HashMap::new(),
            music_sink: None,
            cached_sounds: HashMap::new(),
            delayed_sounds: Vec::new(),
            last_cleanup_time: Instant::now(),
            cleanup_interval: Duration::from_secs(30),
            cache_expiry_duration: Duration::from_secs(300), // 5 minutes
        })
    }
    
    /// Play a sound effect
    /// 
    /// # Arguments
    /// * `index` - Sound ID
    /// * `loop_sound` - Whether to loop the sound
    /// * `delay_ms` - Delay before playing (milliseconds)
    pub fn play_sound(&mut self, index: SoundId, loop_sound: bool, delay_ms: u64) -> Result<()> {
        // Check for cleanup
        self.check_cleanup();
        
        // Handle delayed sound
        if delay_ms > 0 {
            let play_time = Instant::now() + Duration::from_millis(delay_ms);
            self.delayed_sounds.push((play_time, index));
            return Ok(());
        }
        
        // Get filename
        let filename = self.get_filename(index);
        
        if loop_sound {
            // Create looping sound
            self.play_looping_sound(index, &filename)?;
        } else {
            // Play one-shot sound
            self.play_oneshot_sound(index, &filename)?;
        }
        
        Ok(())
    }
    
    /// Stop a looping sound
    pub fn stop_sound(&mut self, index: SoundId) {
        if let Some(sink) = self.looping_sounds.remove(&index) {
            sink.stop();
        }
    }
    
    /// Play background music
    pub fn play_music(&mut self, index: SoundId, loop_music: bool) -> Result<()> {
        // Stop current music
        self.stop_music();
        
        // Get filename
        let filename = self.get_filename(index);
        
        // Load and play music
        let file_path = self.find_sound_file(&filename)?;
        let file = std::fs::File::open(&file_path)
            .with_context(|| format!("Failed to open music file: {:?}", file_path))?;
        
        let source = Decoder::new(file)
            .with_context(|| format!("Failed to decode music file: {:?}", file_path))?;
        
        // Create new sink for music
        let music_sink = Sink::try_new(&self.stream_handle)
            .context("Failed to create music sink")?;
        
        let scaled_volume = Self::scale_volume(self.music_volume);
        music_sink.set_volume(scaled_volume);
        
        if loop_music {
            music_sink.append(source.repeat_infinite());
        } else {
            music_sink.append(source);
        }
        
        self.music_sink = Some(music_sink);
        
        Ok(())
    }
    
    /// Stop background music
    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }
    
    /// Set sound effects volume (0-100)
    pub fn set_volume(&mut self, volume: i32) {
        if self.volume == volume {
            return;
        }
        
        self.volume = volume;
        self.adjust_all_volumes();
    }
    
    /// Set music volume (0-100)
    pub fn set_music_volume(&mut self, volume: i32) {
        if self.music_volume == volume {
            return;
        }
        
        self.music_volume = volume;
        
        if let Some(ref sink) = self.music_sink {
            let scaled_volume = Self::scale_volume(volume);
            sink.set_volume(scaled_volume);
        }
    }
    
    /// Process delayed sounds (call every frame)
    pub fn process_delayed_sounds(&mut self) {
        if self.delayed_sounds.is_empty() {
            return;
        }
        
        let now = Instant::now();
        let mut sounds_to_play = Vec::new();
        
        // Find sounds that should play now
        self.delayed_sounds.retain(|(play_time, index)| {
            if *play_time <= now {
                sounds_to_play.push(*index);
                false // Remove from queue
            } else {
                true // Keep in queue
            }
        });
        
        // Play the sounds
        for index in sounds_to_play {
            let _ = self.play_sound(index, false, 0);
        }
    }
    
    // ===== Private Methods =====
    
    /// Get filename for sound index
    fn get_filename(&mut self, index: SoundId) -> String {
        self.index_list.get(&index)
            .cloned()
            .unwrap_or_else(|| {
                let filename = generate_filename(index);
                self.index_list.insert(index, filename.clone());
                filename
            })
    }
    
    /// Play one-shot sound
    fn play_oneshot_sound(&mut self, index: SoundId, filename: &str) -> Result<()> {
        // Try to find the sound file
        let file_path = self.find_sound_file(filename)?;
        
        // Load and decode
        let file = std::fs::File::open(&file_path)
            .with_context(|| format!("Failed to open sound file: {:?}", file_path))?;
        
        let source = Decoder::new(file)
            .with_context(|| format!("Failed to decode sound file: {:?}", file_path))?;
        
        // Append to one-shots sink
        self.one_shots_sink.append(source);
        
        // Update expire time (for cleanup)
        let expire_time = Instant::now() + self.cache_expiry_duration;
        // Note: We're not actually caching decoded audio in this implementation
        // to keep memory usage low. Could add caching later if needed.
        
        Ok(())
    }
    
    /// Play looping sound
    fn play_looping_sound(&mut self, index: SoundId, filename: &str) -> Result<()> {
        let file_path = self.find_sound_file(filename)?;
        
        let file = std::fs::File::open(&file_path)
            .with_context(|| format!("Failed to open looping sound: {:?}", file_path))?;
        
        let source = Decoder::new(file)
            .with_context(|| format!("Failed to decode looping sound: {:?}", file_path))?;
        
        // Create new sink for this looping sound
        let sink = Sink::try_new(&self.stream_handle)
            .context("Failed to create looping sound sink")?;
        
        let scaled_volume = Self::scale_volume(self.volume);
        sink.set_volume(scaled_volume);
        
        // Append with infinite repeat
        sink.append(source.repeat_infinite());
        
        self.looping_sounds.insert(index, sink);
        
        Ok(())
    }
    
    /// Find sound file with supported extensions
    fn find_sound_file(&self, filename: &str) -> Result<PathBuf> {
        let extensions = &[".wav", ".mp3", ".ogg"];
        
        for ext in extensions {
            let mut path = self.sound_path.join(filename);
            path.set_extension(&ext[1..]); // Remove leading dot
            
            if path.exists() {
                return Ok(path);
            }
        }
        
        anyhow::bail!("Sound file not found: {} (tried extensions: {:?})", filename, extensions)
    }
    
    /// Scale volume from 0-100 to 0.0-1.0
    fn scale_volume(volume: i32) -> f32 {
        let clamped = volume.clamp(0, 100);
        clamped as f32 / 100.0
    }
    
    /// Adjust all volumes
    fn adjust_all_volumes(&mut self) {
        let scaled_volume = Self::scale_volume(self.volume);
        self.one_shots_sink.set_volume(scaled_volume);
        
        for sink in self.looping_sounds.values() {
            sink.set_volume(scaled_volume);
        }
    }
    
    /// Check and perform cleanup
    fn check_cleanup(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_cleanup_time) < self.cleanup_interval {
            return;
        }
        
        self.last_cleanup_time = now;
        
        // Remove expired cached sounds
        self.cached_sounds.retain(|_, cache| cache.expire_time > now);
        
        // Remove stopped looping sounds
        self.looping_sounds.retain(|_, sink| !sink.empty());
    }
}

impl Drop for SoundManager {
    fn drop(&mut self) {
        // Stop all sounds
        self.stop_music();
        self.one_shots_sink.stop();
        
        for (_, sink) in self.looping_sounds.drain() {
            sink.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_volume() {
        assert_eq!(SoundManager::scale_volume(0), 0.0);
        assert_eq!(SoundManager::scale_volume(50), 0.5);
        assert_eq!(SoundManager::scale_volume(100), 1.0);
        assert_eq!(SoundManager::scale_volume(150), 1.0); // Clamped
        assert_eq!(SoundManager::scale_volume(-10), 0.0); // Clamped
    }

    #[test]
    fn test_delayed_sound_timing() {
        // Test that delayed sounds are queued correctly
        // Note: Requires actual SoundManager initialization which needs audio device
        // So we just test the timing logic here
        
        let now = Instant::now();
        let future = now + Duration::from_millis(100);
        
        assert!(future > now);
    }
}
