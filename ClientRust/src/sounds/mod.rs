use anyhow::{Context, Result};
use rodio::{OutputStream, Sink};

use crate::settings::SoundSettings;

pub struct AudioEngine {
    _stream: OutputStream,
    settings: SoundSettings,
    master_sink: Sink,
}

#[allow(dead_code)]
impl AudioEngine {
    pub fn new(sound: &SoundSettings) -> Result<Self> {
        let (stream, handle) =
            OutputStream::try_default().context("failed to open default audio output")?;
        let master_sink = Sink::try_new(&handle).context("failed to create master sink")?;

        let mut engine = Self {
            _stream: stream,
            settings: sound.clone(),
            master_sink,
        };
        engine.apply_volume();
        Ok(engine)
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
