// ============================================================================
// 音效系统（M11）
// 数据：Client-Macroquad/Sound/SoundList.lst（sound_id → 文件名）+ *.wav
// 用法：play_sound(commands, assets, bank, id)
// 挂接：攻击 Swing(10050)、受击 Struck(10060)
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings};
use bevy::prelude::*;

/// 音效库（SoundList.lst 映射）
#[derive(Resource, Default)]
pub struct SoundBank {
    pub map: HashMap<u32, String>,
    pub root: PathBuf,
}

impl SoundBank {
    /// 加载 SoundList.lst（参考 Client-Macroquad/src/systems/presentation/sound_system.rs）
    pub fn load(&mut self) {
        // 共享主仓库的 Sound 目录（独立 worktree 场景：Crystal-bevy → ../Crystal）
        let candidate = [
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Client-Macroquad/Sound"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Crystal/Client-Macroquad/Sound"),
        ];
        self.root = candidate
            .into_iter()
            .find(|p| p.join("SoundList.lst").exists())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Crystal/Client-Macroquad/Sound"));
        let lst = self.root.join("SoundList.lst");
        let Ok(text) = std::fs::read_to_string(&lst) else {
            tracing::warn!("🔊 SoundList.lst 未找到: {}", lst.display());
            return;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if let Some((id, file)) = line.split_once(':') {
                if let Ok(id) = id.trim().parse::<u32>() {
                    self.map.insert(id, file.trim().to_string());
                }
            }
        }
        tracing::info!("🔊 SoundList 加载: {} 条音效", self.map.len());
    }

    pub fn file_for(&self, id: u32) -> Option<PathBuf> {
        self.map.get(&id).map(|f| self.root.join(f))
    }
}

/// 全局音效音量（0-100 百分比，C# Settings.Volume；option_view_system 同步）
pub static SOUND_VOLUME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(80);

/// 百分比 → 线性音量（0.0-1.0）
pub fn volume_from_percent(percent: u32) -> f32 {
    (percent as f32 / 100.0).clamp(0.0, 1.0)
}

fn volume_settings() -> PlaybackSettings {
    let v = SOUND_VOLUME.load(std::sync::atomic::Ordering::Relaxed);
    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(volume_from_percent(v)))
}

/// 播放音效（读取 wav → AudioSource → 一次性播放）
pub fn play_sound(
    commands: &mut Commands,
    assets: &mut Assets<AudioSource>,
    bank: &SoundBank,
    id: u32,
) {
    let Some(path) = bank.file_for(id) else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let source = AudioSource {
        bytes: Arc::from(bytes),
    };
    let handle = assets.add(source);
    commands.spawn((
        AudioPlayer(handle),
        volume_settings(),
    ));
}

/// 音效缓存（#91：UI 高频点击音效复用 AudioSource，避免每次读盘）
#[derive(Resource, Default)]
pub struct SoundCache {
    pub map: HashMap<u32, Handle<AudioSource>>,
}

/// 播放音效（带缓存；未命中时读盘一次并缓存）
pub fn play_sound_cached(
    commands: &mut Commands,
    assets: &mut Assets<AudioSource>,
    bank: &SoundBank,
    cache: &mut SoundCache,
    id: u32,
) {
    let handle = if let Some(h) = cache.map.get(&id) {
        h.clone()
    } else {
        let Some(path) = bank.file_for(id) else {
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let source = AudioSource {
            bytes: Arc::from(bytes),
        };
        let h = assets.add(source);
        cache.map.insert(id, h.clone());
        h
    };
    commands.spawn((
        AudioPlayer(handle),
        volume_settings(),
    ));
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundBank>();
        app.init_resource::<SoundCache>();
        app.add_systems(Startup, load_sound_bank);
        // #230：S.PlaySound → 播放服务端指定音效
        app.add_systems(
            Update,
            play_server_sounds
                .after(crate::network::network_system)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
    }
}

fn load_sound_bank(mut bank: ResMut<SoundBank>) {
    bank.load();
}

/// #230：消费 ServerEvent::PlaySound 并播放（带缓存）
fn play_server_sounds(
    mut commands: Commands,
    mut assets: ResMut<Assets<AudioSource>>,
    bank: Res<SoundBank>,
    mut cache: ResMut<SoundCache>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::PlaySound { sound_id } = ev {
            play_sound_cached(&mut commands, &mut assets, &bank, &mut cache, *sound_id);
            tracing::debug!("🔊 [SOUND] 播放服务端音效 #{}", sound_id);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_from_percent_clamps() {
        assert_eq!(volume_from_percent(0), 0.0);
        assert_eq!(volume_from_percent(80), 0.8);
        assert_eq!(volume_from_percent(100), 1.0);
        assert_eq!(volume_from_percent(200), 1.0);
    }
}
