// One-Shot Sound Provider - Play sound once without looping
// Mirrors Client.MirSounds.Libraries.OneShotProvider

use std::sync::Arc;
use std::time::Instant;

use rodio::{Source};

use super::cached_sound::CachedSound;

/// One-shot sound source - plays cached sound once
pub struct OneShotSource {
    cached_sound: Arc<CachedSound>,
    position: usize,
}

impl OneShotSource {
    /// Create a new one-shot sound source
    pub fn new(cached_sound: Arc<CachedSound>) -> Self {
        Self {
            cached_sound,
            position: 0,
        }
    }
}

impl Iterator for OneShotSource {
    type Item = f32;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.cached_sound.audio_data.len() {
            let sample = self.cached_sound.audio_data[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for OneShotSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.cached_sound.audio_data.len() - self.position)
    }
    
    fn channels(&self) -> u16 {
        self.cached_sound.channels
    }
    
    fn sample_rate(&self) -> u32 {
        self.cached_sound.sample_rate
    }
    
    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(self.cached_sound.duration())
    }
}

/// One-shot provider - manages one-time sound playback
pub struct OneShotProvider {
    index: i32,
    expire_time: Instant,
    cached_sound: Arc<CachedSound>,
}

impl OneShotProvider {
    /// Create a new one-shot provider
    pub fn new(cached_sound: Arc<CachedSound>) -> Self {
        Self {
            index: cached_sound.index,
            expire_time: cached_sound.expire_time,
            cached_sound,
        }
    }
    
    /// Create a sound source for playback
    pub fn create_source(&self) -> OneShotSource {
        OneShotSource::new(Arc::clone(&self.cached_sound))
    }
    
    /// Get sound index
    pub fn index(&self) -> i32 {
        self.index
    }
    
    /// Get expiration time
    pub fn expire_time(&self) -> Instant {
        self.expire_time
    }
    
    /// Set expiration time
    pub fn set_expire_time(&mut self, time: Instant) {
        self.expire_time = time;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_oneshot_source() {
        let cached = Arc::new(CachedSound {
            index: 1,
            expire_time: Instant::now(),
            audio_data: vec![0.0, 0.5, 1.0, 0.5, 0.0],
            sample_rate: 44100,
            channels: 1,
        });
        
        let mut source = OneShotSource::new(cached);
        
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(0.5));
        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(0.5));
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), None);
    }
    
    #[test]
    fn test_oneshot_provider() {
        let cached = Arc::new(CachedSound {
            index: 123,
            expire_time: Instant::now(),
            audio_data: vec![0.0; 100],
            sample_rate: 44100,
            channels: 2,
        });
        
        let provider = OneShotProvider::new(cached);
        
        assert_eq!(provider.index(), 123);
        
        let source = provider.create_source();
        assert_eq!(source.channels(), 2);
        assert_eq!(source.sample_rate(), 44100);
    }
}
