// Loop Sound Provider - Play looping sounds (music, ambient)
// Mirrors Client.MirSounds.Libraries.LoopProvider

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, Sink, Source};

use super::sound_library::{SoundLibrary, scale_volume};

/// Loop provider - manages looping sound playback
pub struct LoopProvider {
    index: i32,
    expire_time: Instant,
    
    // Audio playback
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
    
    // Configuration
    filename: PathBuf,
    unscaled_volume: i32,
    should_loop: bool,
}

impl LoopProvider {
    /// Try to create a new loop provider
    /// 
    /// # Arguments
    /// * `index` - Sound index
    /// * `sound_path` - Base path to sound directory
    /// * `filename` - Sound filename (without extension)
    /// * `volume` - Initial volume (0-100)
    /// * `loop_sound` - Whether to loop the sound
    /// 
    /// # Returns
    /// * `Some(LoopProvider)` if file found
    /// * `None` if file not found
    pub fn try_create(index: i32, sound_path: &Path, filename: &str, volume: i32, loop_sound: bool) -> Option<Self> {
        let file_path = match Self::find_sound_file(sound_path, filename) {
            Ok(path) => path,
            Err(_) => return None,
        };
        
        Some(Self::new(index, file_path, volume, loop_sound))
    }
    
    /// Create a new loop provider (private - use try_create instead)
    fn new(index: i32, filename: PathBuf, volume: i32, should_loop: bool) -> Self {
        let mut provider = Self {
            index,
            expire_time: Instant::now(),
            _stream: None,
            sink: None,
            filename,
            unscaled_volume: volume,
            should_loop,
        };
        
        provider.play(volume);
        provider
    }
    
    /// Find sound file with supported extensions
    fn find_sound_file(base_path: &Path, filename: &str) -> Result<PathBuf> {
        let extensions = &[".wav", ".mp3", ".ogg", ".flac"];
        
        for ext in extensions {
            let mut path = base_path.join(filename);
            path.set_extension(&ext[1..]);
            
            if path.exists() {
                return Ok(path);
            }
        }
        
        let path = base_path.join(filename);
        if path.exists() {
            return Ok(path);
        }
        
        anyhow::bail!("Sound file not found: {}", filename)
    }
    
    /// Load and decode audio file
    fn load_audio(&self) -> Result<Decoder<BufReader<File>>> {
        let file = File::open(&self.filename)
            .with_context(|| format!("Failed to open sound file: {:?}", self.filename))?;
        
        let decoder = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode sound file: {:?}", self.filename))?;
        
        Ok(decoder)
    }
}

impl SoundLibrary for LoopProvider {
    fn index(&self) -> i32 {
        self.index
    }
    
    fn set_index(&mut self, index: i32) {
        self.index = index;
    }
    
    fn expire_time(&self) -> Instant {
        self.expire_time
    }
    
    fn set_expire_time(&mut self, time: Instant) {
        self.expire_time = time;
    }
    
    fn is_playing(&self) -> bool {
        self.sink.as_ref().map_or(false, |sink| !sink.empty())
    }
    
    fn play(&mut self, volume: i32) {
        // If already playing, don't restart
        if self.is_playing() {
            return;
        }
        
        self.unscaled_volume = volume;
        
        // Update expire time (e.g., 5 minutes from now)
        self.expire_time = Instant::now() + std::time::Duration::from_secs(300);
        
        // Create output stream if needed
        if self._stream.is_none() {
            if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
                // Create sink
                if let Ok(sink) = Sink::try_new(&stream_handle) {
                    // Load audio
                    if let Ok(source) = self.load_audio() {
                        let scaled_vol = scale_volume(volume);
                        sink.set_volume(scaled_vol);
                        
                        if self.should_loop {
                            sink.append(source.repeat_infinite());
                        } else {
                            sink.append(source);
                        }
                        
                        self.sink = Some(sink);
                        self._stream = Some(_stream);
                    }
                }
            }
        }
    }
    
    fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self._stream = None;
    }
    
    fn set_volume(&mut self, volume: i32) {
        self.unscaled_volume = volume;
        
        if let Some(ref sink) = self.sink {
            let scaled_vol = scale_volume(volume);
            sink.set_volume(scaled_vol);
        }
    }
}

impl Drop for LoopProvider {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_loop_provider_interface() {
        // Test basic interface (can't test actual playback without audio device)
        let path = PathBuf::from("test.wav");
        let mut provider = LoopProvider {
            index: 100,
            expire_time: Instant::now(),
            _stream: None,
            sink: None,
            filename: path,
            unscaled_volume: 50,
            should_loop: true,
        };
        
        assert_eq!(provider.index(), 100);
        
        provider.set_index(200);
        assert_eq!(provider.index(), 200);
        
        // Without audio device, is_playing should return false
        assert!(!provider.is_playing());
    }
    
    #[test]
    fn test_volume_scaling() {
        assert_eq!(scale_volume(0), 0.0);
        assert_eq!(scale_volume(50), 0.5);
        assert_eq!(scale_volume(100), 1.0);
        assert_eq!(scale_volume(150), 1.0); // Clamped
    }
}
