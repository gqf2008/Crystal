use anyhow::Result;
use rodio::{OutputStream, Sink};

use crate::settings::SoundSettings;

pub mod sound_loader;
pub use sound_loader::{SoundManager, SoundType, SoundInfo};

pub struct AudioEngine {
    _stream: OutputStream,
    settings: SoundSettings,
    master_sink: Sink,
}

#[allow(dead_code)]
impl AudioEngine {
    pub fn new(sound: &SoundSettings) -> Result<Self> {
        // 暂时禁用音频初始化以修复编译问题
        // TODO: 修复rodio API兼容性问题
        Err(anyhow::anyhow!("Audio system temporarily disabled"))
    }

    fn apply_volume(&mut self) {
        let volume = self.settings.master_volume_scalar();
        self.master_sink.set_volume(volume);
    }

    pub fn play_effect(&self, _id: &str) -> Result<()> {
        // TODO: Load and play effect from resource library.
        Ok(())
    }

    pub fn play_music(&self, _id: &str) -> Result<()> {
        // TODO: Differentiate between music and effect playback.
        Ok(())
    }

    pub fn stop_music(&self) {
        self.master_sink.stop();
    }
}
